use crate::core::chunk::{Chunk, LaneRef, NONE};
use crate::core::graph_service::{GraphHistory, LaneSnapshot};
use smallvec::{SmallVec, smallvec};

const DELTA_CHUNK_SIZE: usize = 8_192;
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

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum DeltaOps {
    #[default]
    Empty,
    One(DeltaOp),
    Two(DeltaOp, DeltaOp),
    Many(SmallVec<[DeltaOp; 8]>),
}

impl DeltaOps {
    fn push(&mut self, op: DeltaOp) {
        match std::mem::take(self) {
            DeltaOps::Empty => *self = DeltaOps::One(op),
            DeltaOps::One(first) => *self = DeltaOps::Two(first, op),
            DeltaOps::Two(first, second) => *self = DeltaOps::Many(smallvec![first, second, op]),
            DeltaOps::Many(mut ops) => {
                ops.push(op);
                *self = DeltaOps::Many(ops);
            },
        }
    }

    pub fn iter(&self) -> DeltaOpsIter<'_> {
        match self {
            DeltaOps::Empty => DeltaOpsIter::Empty,
            DeltaOps::One(op) => DeltaOpsIter::One(Some(op)),
            DeltaOps::Two(first, second) => DeltaOpsIter::Two([first, second].into_iter()),
            DeltaOps::Many(ops) => DeltaOpsIter::Many(ops.iter()),
        }
    }

    fn len(&self) -> usize {
        match self {
            DeltaOps::Empty => 0,
            DeltaOps::One(_) => 1,
            DeltaOps::Two(_, _) => 2,
            DeltaOps::Many(ops) => ops.len(),
        }
    }

    fn shrink_to_fit(&mut self) {
        if let DeltaOps::Many(ops) = self {
            ops.shrink_to_fit();
        }
    }
}

pub enum DeltaOpsIter<'a> {
    Empty,
    One(Option<&'a DeltaOp>),
    Two(std::array::IntoIter<&'a DeltaOp, 2>),
    Many(std::slice::Iter<'a, DeltaOp>),
}

impl<'a> Iterator for DeltaOpsIter<'a> {
    type Item = &'a DeltaOp;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            DeltaOpsIter::Empty => None,
            DeltaOpsIter::One(op) => op.take(),
            DeltaOpsIter::Two(iter) => iter.next(),
            DeltaOpsIter::Many(iter) => iter.next(),
        }
    }
}

#[derive(Clone, Default)]
pub struct DeltaLog {
    chunks: Vec<Vec<DeltaSpan>>,
    op_chunks: Vec<Vec<DeltaOp>>,
    len: usize,
}

#[derive(Clone, Copy, Default)]
struct DeltaSpan {
    chunk: u16,
    start: u32,
    len: u16,
}

impl DeltaLog {
    pub fn len(&self) -> usize {
        self.len
    }

    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.chunks.iter().map(Vec::capacity).sum()
    }

    #[cfg(test)]
    pub fn op_chunk_capacities(&self) -> Vec<usize> {
        self.op_chunks.iter().map(Vec::capacity).collect()
    }

    fn push(&mut self, delta: Delta) {
        if self.chunks.last().is_none_or(|chunk| chunk.len() == DELTA_CHUNK_SIZE) {
            self.chunks.push(Vec::with_capacity(DELTA_CHUNK_SIZE));
        }

        let op_len = delta.ops.len();
        if op_len > 0 && self.op_chunks.last().is_none_or(|chunk| chunk.len() + op_len > DELTA_OP_CHUNK_SIZE) {
            self.op_chunks.push(Vec::with_capacity(DELTA_OP_CHUNK_SIZE.max(op_len)));
        }

        let chunk_idx = self.op_chunks.len().saturating_sub(1);
        let start = self.op_chunks.last().map_or(0, Vec::len);
        for op in delta.ops.iter() {
            self.op_chunks.last_mut().expect("delta op log has a writable chunk").push(*op);
        }
        let len = u16::try_from(op_len).expect("delta entry exceeded u16::MAX ops");

        self.chunks.last_mut().expect("delta log has a writable chunk").push(DeltaSpan {
            chunk: u16::try_from(chunk_idx).expect("delta op chunk index exceeded u16::MAX"),
            start: u32::try_from(start).expect("delta op arena chunk exceeded u32::MAX entries"),
            len,
        });
        self.len += 1;
    }

    fn iter_range(&self, start: usize, end: usize) -> DeltaLogRangeIter<'_> {
        DeltaLogRangeIter { log: self, next: start.min(self.len), end: end.min(self.len) }
    }

    fn shrink_to_fit(&mut self) {
        for chunk in &mut self.chunks {
            chunk.shrink_to_fit();
        }
        self.chunks.shrink_to_fit();
        for chunk in &mut self.op_chunks {
            chunk.shrink_to_fit();
        }
        self.op_chunks.shrink_to_fit();
    }
}

struct DeltaView<'a> {
    ops: DeltaOpsView<'a>,
}

struct DeltaOpsView<'a> {
    ops: &'a [DeltaOp],
}

impl<'a> DeltaOpsView<'a> {
    fn iter(&self) -> std::slice::Iter<'a, DeltaOp> {
        self.ops.iter()
    }
}

struct DeltaLogRangeIter<'a> {
    log: &'a DeltaLog,
    next: usize,
    end: usize,
}

impl<'a> Iterator for DeltaLogRangeIter<'a> {
    type Item = DeltaView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }

        let chunk_idx = self.next / DELTA_CHUNK_SIZE;
        let delta_idx = self.next % DELTA_CHUNK_SIZE;
        self.next += 1;

        self.log.chunks.get(chunk_idx).and_then(|chunk| chunk.get(delta_idx)).map(|span| {
            if span.len == 0 {
                return DeltaView { ops: DeltaOpsView { ops: &[] } };
            }

            let ops = self.log.op_chunks.get(span.chunk as usize).expect("delta span references an existing op chunk");
            let start = span.start as usize;
            let end = start + span.len as usize;
            DeltaView { ops: DeltaOpsView { ops: &ops[start..end] } }
        })
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

        if !self.transient_lanes.iter().any(|idx| *idx == lane_idx) {
            self.transient_lanes.push(lane_idx);
        }
    }

    pub fn update(&mut self, chunk: Chunk) -> UpdateOutcome {
        self.backup();

        let transient_lanes = std::mem::take(&mut self.transient_lanes);
        for lane_idx in transient_lanes {
            if lane_idx < self.curr.len() && !self.curr[lane_idx].is_dummy() {
                self.curr[lane_idx] = Chunk::dummy();
                self.delta.ops.push(DeltaOp::Replace { index: delta_index(lane_idx), new: self.curr[lane_idx] });
            }
        }

        // Trailing dummy lanes carry no future topology and can be removed immediately.
        while let Some(last_idx) = self.curr.len().checked_sub(1) {
            if !self.curr[last_idx].is_dummy() {
                break;
            }
            self.curr.pop();
            self.delta.ops.push(DeltaOp::Remove { index: delta_index(last_idx) });
        }

        // Planned mergers split a lane so the second parent can draw toward its target later.
        if !self.mergers.is_empty()
            && let Some(merger_idx) = self.curr.iter().position(|inner| self.mergers.iter().any(|alias| *alias == inner.alias))
        {
            let merger_alias = self.curr[merger_idx].alias;
            self.mergers.retain(|alias| *alias != merger_alias);

            let mut clone = self.curr[merger_idx].clone();
            clone.parent_a = clone.parent_b;
            clone.parent_b = NONE;
            self.curr[merger_idx].parent_b = NONE;
            self.curr.push(clone);

            self.delta.ops.push(DeltaOp::Replace { index: delta_index(merger_idx), new: self.curr[merger_idx] });

            self.delta.ops.push(DeltaOp::Insert { index: delta_index(self.curr.len() - 1), item: clone });
        }

        // Prefer replacing the parent lane; append only when the commit starts a new lane.
        if let Some(first_idx) = self.curr.iter().position(|inner| inner.parent_a == chunk.alias) {
            let old_alias = chunk.alias;

            self.curr[first_idx] = chunk;
            self.delta.ops.push(DeltaOp::Replace { index: delta_index(first_idx), new: chunk });

            // Clear consumed parent pointers so inactive branch lanes collapse into dummies.
            for (i, inner) in self.curr.iter_mut().enumerate() {
                if inner.alias == old_alias {
                    continue;
                }

                let mut parents_changed = false;

                if inner.parent_a == old_alias {
                    inner.parent_a = NONE;
                    parents_changed = true;
                }

                if inner.parent_b == old_alias {
                    inner.parent_b = NONE;
                    parents_changed = true;
                }

                if parents_changed {
                    if inner.parent_a == NONE && inner.parent_b == NONE {
                        *inner = Chunk::dummy();
                    }

                    self.delta.ops.push(DeltaOp::Replace { index: delta_index(i), new: *inner });
                }
            }

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

        while self.curr.len() > limit {
            self.curr.pop();
        }

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
        if self.mergers.is_empty() {
            return;
        }
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

    pub fn shrink_to_fit(&mut self) {
        self.deltas.shrink_to_fit();
        self.delta.ops.shrink_to_fit();
        self.mergers.shrink_to_fit();
        self.transient_lanes.shrink_to_fit();
    }

    pub fn window(&self, start: usize, end: usize) -> GraphHistory {
        let mut history = GraphHistory::new();

        // Start from the nearest checkpoint before the requested range.
        let checkpoint = self.checkpoints.get(self.checkpoints.partition_point(|checkpoint| checkpoint.idx <= start).saturating_sub(1));

        let mut curr = checkpoint.map(|checkpoint| checkpoint.curr.clone()).unwrap_or_default();

        // Replay only the deltas needed to produce the requested visible range.
        let begin = checkpoint.map_or(0, |checkpoint| checkpoint.idx + 1);
        let end = end.min(self.deltas.len());

        for (idx, delta) in (begin..end).zip(self.deltas.iter_range(begin, end)) {
            for op in delta.ops.iter() {
                match op {
                    DeltaOp::Insert { index, item } => {
                        curr.insert(*index as usize, *item);
                    },
                    DeltaOp::Remove { index } => {
                        curr.remove(*index as usize);
                    },
                    DeltaOp::Replace { index, new } => {
                        curr[*index as usize] = *new;
                    },
                    DeltaOp::Truncate { len } => {
                        while curr.len() > *len as usize {
                            curr.pop();
                        }
                    },
                    DeltaOp::ReplaceAndTruncate { index, new, len } => {
                        curr[*index as usize] = *new;
                        while curr.len() > *len as usize {
                            curr.pop();
                        }
                    },
                }
            }
            if idx >= start {
                history.push(curr.clone());
            }
        }

        history
    }
}

fn delta_index(index: usize) -> u32 {
    u32::try_from(index).expect("graph delta index exceeded u32::MAX")
}

#[cfg(test)]
#[path = "../tests/core/buffer.rs"]
mod tests;
