use crate::core::chunk::{Chunk, LaneRef, NONE};
use crate::core::graph_service::{GraphHistory, LaneSnapshot};
use smallvec::SmallVec;

const DELTA_OP_CHUNK_SIZE: usize = 131_072;
const CHECKPOINT_INTERVAL: usize = 16_384;

#[derive(Default, Clone)]
pub struct Delta {
    pub ops: DeltaOps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeltaOp {
    Insert { index: u32, item: Chunk },
    Remove { index: u32 },
    Replace { index: u32, new: Chunk },
    Truncate { len: u32 },
    ReplaceAndTruncate { index: u32, new: Chunk, len: u32 },
}

impl DeltaOp {
    fn apply(self, curr: &mut LaneSnapshot) {
        match self {
            DeltaOp::Insert { index, item } => {
                curr.insert(index as usize, item);
            },
            DeltaOp::Remove { index } => {
                curr.remove(index as usize);
            },
            DeltaOp::Replace { index, new } => {
                curr[index as usize] = new;
            },
            DeltaOp::Truncate { len } => {
                curr.truncate(len as usize);
            },
            DeltaOp::ReplaceAndTruncate { index, new, len } => {
                curr[index as usize] = new;
                curr.truncate(len as usize);
            },
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct DeltaOps(SmallVec<[DeltaOp; 8]>);

impl DeltaOps {
    fn push(&mut self, op: DeltaOp) {
        self.0.push(op);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, DeltaOp> {
        self.0.iter()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    #[cfg(test)]
    fn spilled(&self) -> bool {
        self.0.spilled()
    }
}

#[derive(Clone, Default)]
pub struct DeltaLog {
    spans: Vec<DeltaSpan>,
    op_chunks: Vec<Vec<DeltaOp>>,
}

#[derive(Clone, Copy, Default)]
struct DeltaSpan {
    chunk: u16,
    start: u32,
    len: u16,
}

impl DeltaLog {
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.spans.capacity()
    }

    #[cfg(test)]
    pub fn op_chunk_capacities(&self) -> Vec<usize> {
        self.op_chunks.iter().map(Vec::capacity).collect()
    }

    fn push(&mut self, delta: Delta) {
        let span = self.store_ops(&delta.ops);
        self.spans.push(span);
    }

    fn store_ops(&mut self, ops: &DeltaOps) -> DeltaSpan {
        let len = ops.len();
        if len == 0 {
            return DeltaSpan::default();
        }

        if self.op_chunks.last().is_none_or(|chunk| chunk.len() + len > DELTA_OP_CHUNK_SIZE) {
            self.op_chunks.push(Vec::with_capacity(DELTA_OP_CHUNK_SIZE.max(len)));
        }

        let chunk_idx = self.op_chunks.len() - 1;
        let chunk = &mut self.op_chunks[chunk_idx];
        let start = chunk.len();
        chunk.extend(ops.iter().copied());

        DeltaSpan {
            chunk: u16::try_from(chunk_idx).expect("delta op chunk index exceeded u16::MAX"),
            start: u32::try_from(start).expect("delta op arena chunk exceeded u32::MAX entries"),
            len: u16::try_from(len).expect("delta entry exceeded u16::MAX ops"),
        }
    }

    fn iter_range(&self, start: usize, end: usize) -> DeltaLogRangeIter<'_> {
        DeltaLogRangeIter { log: self, next: start.min(self.len()), end: end.min(self.len()) }
    }

    fn shrink_to_fit(&mut self) {
        self.spans.shrink_to_fit();
        for chunk in &mut self.op_chunks {
            chunk.shrink_to_fit();
        }
        self.op_chunks.shrink_to_fit();
    }

    fn reserve_entries(&mut self, entries: usize) {
        self.spans.reserve(entries.saturating_sub(self.spans.len()));
    }
}

struct DeltaView<'a> {
    ops: &'a [DeltaOp],
}

struct DeltaLogRangeIter<'a> {
    log: &'a DeltaLog,
    next: usize,
    end: usize,
}

impl<'a> Iterator for DeltaLogRangeIter<'a> {
    type Item = (usize, DeltaView<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }

        let idx = self.next;
        self.next += 1;

        let span = self.log.spans.get(idx)?;
        if span.len == 0 {
            return Some((idx, DeltaView { ops: &[] }));
        }

        let ops = self.log.op_chunks.get(span.chunk as usize)?;
        let start = span.start as usize;
        let end = start + span.len as usize;
        Some((idx, DeltaView { ops: &ops[start..end] }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateOutcome {
    pub lane: LaneRef,
    pub started_lane: bool,
}

#[derive(Default, Clone)]
pub struct Buffer {
    pub curr: LaneSnapshot,
    // Deltas are append-only; chunking avoids giant realloc/copy spikes while loading huge repos.
    pub deltas: DeltaLog,
    pub checkpoints: Vec<Checkpoint>,
    pub delta: Delta,
    mergers: Vec<u32>,
    transient_lanes: Vec<usize>,
    lane_limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub idx: usize,
    pub curr: LaneSnapshot,
}

impl Buffer {
    pub fn with_lane_limit(limit: usize) -> Self {
        Self { lane_limit: Some(limit.max(1)), ..Self::default() }
    }

    pub fn merger(&mut self, alias: u32) {
        self.mergers.push(alias);
    }

    pub fn expire_lane_after_snapshot(&mut self, lane_idx: usize) {
        if let Some(limit) = self.lane_limit {
            if lane_idx >= limit {
                return;
            }
            if lane_idx + 1 == limit && self.curr.get(lane_idx).is_some_and(|chunk| chunk.is_flattened) {
                return;
            }
        }

        if !self.transient_lanes.contains(&lane_idx) {
            self.transient_lanes.push(lane_idx);
        }
    }

    pub fn update(&mut self, chunk: Chunk) -> UpdateOutcome {
        self.backup();
        self.expire_transient_lanes();
        self.trim_trailing_dummies();
        self.split_planned_merger();

        if let Some(first_idx) = self.curr.iter().position(|inner| inner.parent_a == chunk.alias) {
            self.replace_parent_lane(first_idx, chunk);
            self.enforce_lane_limit(Some(first_idx));
            UpdateOutcome { lane: self.lane_ref_for_original_index(first_idx), started_lane: false }
        } else {
            self.curr.push(chunk);
            self.delta.ops.push(DeltaOp::Insert { index: delta_index(self.curr.len() - 1), item: chunk });
            let lane_idx = self.curr.len() - 1;
            self.enforce_lane_limit(Some(lane_idx));
            UpdateOutcome { lane: self.lane_ref_for_original_index(lane_idx), started_lane: true }
        }
    }

    fn expire_transient_lanes(&mut self) {
        let mut lanes = std::mem::take(&mut self.transient_lanes);
        for lane_idx in lanes.drain(..) {
            if self.curr.get(lane_idx).is_some_and(|chunk| !chunk.is_dummy()) {
                self.curr[lane_idx] = Chunk::dummy();
                self.delta.ops.push(DeltaOp::Replace { index: delta_index(lane_idx), new: self.curr[lane_idx] });
            }
        }
        self.transient_lanes = lanes;
    }

    fn trim_trailing_dummies(&mut self) {
        let old_len = self.curr.len();
        let new_len = self.curr.iter().rposition(|chunk| !chunk.is_dummy()).map_or(0, |idx| idx + 1);

        self.curr.truncate(new_len);
        (new_len..old_len).rev().for_each(|idx| self.delta.ops.push(DeltaOp::Remove { index: delta_index(idx) }));
    }

    fn split_planned_merger(&mut self) {
        let Some(merger_idx) = self.curr.iter().position(|inner| self.mergers.contains(&inner.alias)) else {
            return;
        };

        let merger_alias = self.curr[merger_idx].alias;
        self.mergers.retain(|alias| *alias != merger_alias);

        let mut split = self.curr[merger_idx];
        split.parent_a = split.parent_b;
        split.parent_b = NONE;
        self.curr[merger_idx].parent_b = NONE;
        self.curr.push(split);

        self.delta.ops.push(DeltaOp::Replace { index: delta_index(merger_idx), new: self.curr[merger_idx] });
        self.delta.ops.push(DeltaOp::Insert { index: delta_index(self.curr.len() - 1), item: split });
    }

    fn replace_parent_lane(&mut self, lane_idx: usize, chunk: Chunk) {
        let old_alias = chunk.alias;

        self.curr[lane_idx] = chunk;
        self.delta.ops.push(DeltaOp::Replace { index: delta_index(lane_idx), new: chunk });
        self.clear_consumed_parent_lanes(old_alias);
    }

    fn clear_consumed_parent_lanes(&mut self, old_alias: u32) {
        self.curr.iter_mut().enumerate().filter(|(_, inner)| inner.alias != old_alias && (inner.parent_a == old_alias || inner.parent_b == old_alias)).for_each(|(idx, inner)| {
            if inner.parent_a == old_alias {
                inner.parent_a = NONE;
            }
            if inner.parent_b == old_alias {
                inner.parent_b = NONE;
            }
            if inner.parent_a == NONE && inner.parent_b == NONE {
                *inner = Chunk::dummy();
            }
            self.delta.ops.push(DeltaOp::Replace { index: delta_index(idx), new: *inner });
        });
    }

    fn enforce_lane_limit(&mut self, preferred_idx: Option<usize>) {
        let Some(limit) = self.lane_limit else {
            return;
        };

        if self.curr.len() <= limit {
            self.purge_unstored_mergers();
            self.transient_lanes.retain(|lane_idx| *lane_idx < limit);
            return;
        }

        let cap_idx = limit - 1;
        let replacement = self.flattened_representative(cap_idx, preferred_idx);
        if self.curr[cap_idx] != replacement {
            self.curr[cap_idx] = replacement;
            self.delta.ops.push(DeltaOp::ReplaceAndTruncate { index: delta_index(cap_idx), new: replacement, len: delta_index(limit) });
        } else {
            self.delta.ops.push(DeltaOp::Truncate { len: delta_index(limit) });
        }

        self.curr.truncate(limit);

        self.purge_unstored_mergers();
        self.transient_lanes.retain(|lane_idx| *lane_idx < limit && (*lane_idx + 1 != limit || !self.curr.get(*lane_idx).is_some_and(|chunk| chunk.is_flattened)));
    }

    fn flattened_representative(&self, cap_idx: usize, preferred_idx: Option<usize>) -> Chunk {
        let preferred = preferred_idx.and_then(|idx| (idx >= cap_idx).then(|| self.curr.get(idx)).flatten()).copied().filter(|chunk| !chunk.is_dummy());

        let fallback = self.curr.iter().skip(cap_idx).find(|chunk| !chunk.is_dummy()).copied().unwrap_or_else(Chunk::dummy);
        preferred.unwrap_or(fallback).with_flattened(true)
    }

    fn lane_ref_for_original_index(&self, lane_idx: usize) -> LaneRef {
        if let Some(limit) = self.lane_limit
            && lane_idx >= limit
        {
            return LaneRef::new(limit - 1, true);
        }

        LaneRef::new(lane_idx, self.curr.get(lane_idx).is_some_and(|chunk| chunk.is_flattened))
    }

    fn purge_unstored_mergers(&mut self) {
        self.mergers.retain(|alias| self.curr.iter().any(|chunk| !chunk.is_dummy() && chunk.alias == *alias));
    }

    pub fn backup(&mut self) {
        let old = std::mem::take(&mut self.delta);
        self.deltas.push(old);
        let idx = self.deltas.len().saturating_sub(1);
        if idx.is_multiple_of(CHECKPOINT_INTERVAL) {
            self.checkpoints.push(Checkpoint { idx, curr: self.curr.clone() });
        }
    }

    pub fn reserve_history(&mut self, commits: usize) {
        self.deltas.reserve_entries(commits);
        let checkpoint_count = commits.div_ceil(CHECKPOINT_INTERVAL);
        if checkpoint_count > self.checkpoints.capacity() {
            self.checkpoints.reserve(checkpoint_count - self.checkpoints.capacity());
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.deltas.shrink_to_fit();
        self.delta.ops.shrink_to_fit();
        self.mergers.shrink_to_fit();
        self.transient_lanes.shrink_to_fit();
    }

    pub fn window(&self, start: usize, end: usize) -> GraphHistory {
        let checkpoint = self.checkpoint_before_or_at(start);
        let mut curr = checkpoint.map(|checkpoint| checkpoint.curr.clone()).unwrap_or_default();
        GraphHistory::from_rows(self.replay_window_rows(start, end, checkpoint, &mut curr))
    }

    fn checkpoint_before_or_at(&self, index: usize) -> Option<&Checkpoint> {
        self.checkpoints.get(self.checkpoints.partition_point(|checkpoint| checkpoint.idx <= index).saturating_sub(1))
    }

    fn replay_window_rows<'a>(&'a self, start: usize, end: usize, checkpoint: Option<&Checkpoint>, curr: &'a mut LaneSnapshot) -> impl Iterator<Item = LaneSnapshot> + 'a {
        let replay_start = checkpoint.map_or(0, |checkpoint| checkpoint.idx + 1);
        let replay_end = end.min(self.deltas.len());

        self.deltas.iter_range(replay_start, replay_end).filter_map(move |(idx, delta)| {
            apply_delta(curr, &delta);
            (idx >= start).then(|| curr.clone())
        })
    }
}

fn delta_index(index: usize) -> u32 {
    debug_assert!(index <= u32::MAX as usize);
    index as u32
}

fn apply_delta(curr: &mut LaneSnapshot, delta: &DeltaView<'_>) {
    for op in delta.ops.iter().copied() {
        op.apply(curr);
    }
}

#[cfg(test)]
#[path = "../tests/core/buffer.rs"]
mod tests;
