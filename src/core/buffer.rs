use crate::core::chunk::{Chunk, LaneRef, NONE};
use im::{OrdMap, Vector};

const DELTA_CHUNK_SIZE: usize = 8_192;
const CHECKPOINT_INTERVAL: usize = 512;

#[derive(Default, Clone)]
pub struct Delta {
    pub ops: DeltaOps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeltaOp {
    Insert { index: usize, item: Chunk },
    Remove { index: usize },
    Replace { index: usize, new: Chunk },
    Truncate { len: usize },
    ReplaceAndTruncate { index: usize, new: Chunk, len: usize },
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum DeltaOps {
    #[default]
    Empty,
    One(DeltaOp),
    Two(DeltaOp, DeltaOp),
    Many(Vec<DeltaOp>),
}

impl DeltaOps {
    fn push(&mut self, op: DeltaOp) {
        match std::mem::take(self) {
            DeltaOps::Empty => *self = DeltaOps::One(op),
            DeltaOps::One(first) => *self = DeltaOps::Two(first, op),
            DeltaOps::Two(first, second) => *self = DeltaOps::Many(vec![first, second, op]),
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
    chunks: Vec<Vec<Delta>>,
    len: usize,
}

impl DeltaLog {
    pub fn len(&self) -> usize {
        self.len
    }

    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.chunks.iter().map(Vec::capacity).sum()
    }

    fn push(&mut self, delta: Delta) {
        if self.chunks.last().is_none_or(|chunk| chunk.len() == DELTA_CHUNK_SIZE) {
            self.chunks.push(Vec::with_capacity(DELTA_CHUNK_SIZE));
        }

        self.chunks.last_mut().expect("delta log has a writable chunk").push(delta);
        self.len += 1;
    }

    fn iter_range(&self, start: usize, end: usize) -> DeltaLogRangeIter<'_> {
        DeltaLogRangeIter { log: self, next: start.min(self.len), end: end.min(self.len) }
    }

    fn shrink_to_fit(&mut self) {
        for chunk in &mut self.chunks {
            for delta in chunk.iter_mut() {
                delta.ops.shrink_to_fit();
            }
            chunk.shrink_to_fit();
        }
        self.chunks.shrink_to_fit();
    }
}

struct DeltaLogRangeIter<'a> {
    log: &'a DeltaLog,
    next: usize,
    end: usize,
}

impl<'a> Iterator for DeltaLogRangeIter<'a> {
    type Item = &'a Delta;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }

        let chunk_idx = self.next / DELTA_CHUNK_SIZE;
        let delta_idx = self.next % DELTA_CHUNK_SIZE;
        self.next += 1;

        self.log.chunks.get(chunk_idx).and_then(|chunk| chunk.get(delta_idx))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateOutcome {
    pub lane: LaneRef,
    pub started_lane: bool,
}

#[derive(Default, Clone)]
pub struct Buffer {
    pub curr: Vector<Chunk>,
    // Deltas are append-only; chunking avoids giant realloc/copy spikes while loading huge repos.
    pub deltas: DeltaLog,
    pub checkpoints: OrdMap<usize, Vector<Chunk>>,
    pub delta: Delta,
    mergers: Vec<u32>,
    transient_lanes: Vec<usize>,
    lane_limit: Option<usize>,
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
                self.delta.ops.push(DeltaOp::Replace { index: lane_idx, new: self.curr[lane_idx] });
            }
        }

        // Trailing dummy lanes carry no future topology and can be removed immediately.
        while let Some(last_idx) = self.curr.len().checked_sub(1) {
            if !self.curr[last_idx].is_dummy() {
                break;
            }
            self.curr.pop_back();
            self.delta.ops.push(DeltaOp::Remove { index: last_idx });
        }

        // Planned mergers split a lane so the second parent can draw toward its target later.
        if let Some(merger_idx) = self.curr.iter().position(|inner| self.mergers.iter().any(|alias| *alias == inner.alias)) {
            if let Some(merger_pos) = self.mergers.iter().position(|alias| *alias == self.curr[merger_idx].alias) {
                self.mergers.remove(merger_pos);
            }

            let mut clone = self.curr[merger_idx].clone();
            clone.parent_a = clone.parent_b;
            clone.parent_b = NONE;
            self.curr[merger_idx].parent_b = NONE;
            self.curr.push_back(clone);

            self.delta.ops.push(DeltaOp::Replace { index: merger_idx, new: self.curr[merger_idx] });

            self.delta.ops.push(DeltaOp::Insert { index: self.curr.len() - 1, item: clone });
        }

        // Prefer replacing the parent lane; append only when the commit starts a new lane.
        if let Some(first_idx) = self.curr.iter().position(|inner| inner.parent_a == chunk.alias) {
            let old_alias = chunk.alias;

            self.curr[first_idx] = chunk;
            self.delta.ops.push(DeltaOp::Replace { index: first_idx, new: chunk });

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

                    self.delta.ops.push(DeltaOp::Replace { index: i, new: *inner });
                }
            }

            self.enforce_lane_limit(Some(first_idx));
            UpdateOutcome { lane: self.lane_ref_for_original_index(first_idx), started_lane: false }
        } else {
            self.curr.push_back(chunk);
            self.delta.ops.push(DeltaOp::Insert { index: self.curr.len() - 1, item: chunk });
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
            self.delta.ops.push(DeltaOp::ReplaceAndTruncate { index: cap_idx, new: replacement, len: limit });
        } else {
            self.delta.ops.push(DeltaOp::Truncate { len: limit });
        }

        while self.curr.len() > limit {
            self.curr.pop_back();
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
        self.mergers.retain(|alias| self.curr.iter().any(|chunk| !chunk.is_dummy() && chunk.alias == *alias));
    }

    pub fn backup(&mut self) {
        let old = std::mem::take(&mut self.delta);
        self.deltas.push(old);
        let idx = self.deltas.len().saturating_sub(1);
        if idx.is_multiple_of(CHECKPOINT_INTERVAL) {
            self.checkpoints.insert(idx, self.curr.clone());
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.deltas.shrink_to_fit();
        self.delta.ops.shrink_to_fit();
        self.mergers.shrink_to_fit();
        self.transient_lanes.shrink_to_fit();
    }

    pub fn window(&self, start: usize, end: usize) -> Vector<Vector<Chunk>> {
        let mut history = Vector::new();

        // Start from the nearest checkpoint before the requested range.
        let checkpoint_idx = self.checkpoints.keys().rev().find(|&&idx| idx <= start).copied();

        let mut curr = checkpoint_idx.and_then(|idx| self.checkpoints.get(&idx)).cloned().unwrap_or_default();

        // Replay only the deltas needed to produce the requested visible range.
        let begin = checkpoint_idx.map_or(0, |idx| idx + 1);
        let end = end.min(self.deltas.len());

        for (idx, delta) in (begin..end).zip(self.deltas.iter_range(begin, end)) {
            for op in delta.ops.iter() {
                match op {
                    DeltaOp::Insert { index, item } => {
                        curr.insert(*index, *item);
                    },
                    DeltaOp::Remove { index } => {
                        curr.remove(*index);
                    },
                    DeltaOp::Replace { index, new } => {
                        curr[*index] = *new;
                    },
                    DeltaOp::Truncate { len } => {
                        while curr.len() > *len {
                            curr.pop_back();
                        }
                    },
                    DeltaOp::ReplaceAndTruncate { index, new, len } => {
                        curr[*index] = *new;
                        while curr.len() > *len {
                            curr.pop_back();
                        }
                    },
                }
            }
            if idx >= start {
                history.push_back(curr.clone());
            }
        }

        history
    }
}

#[cfg(test)]
#[path = "../tests/core/buffer.rs"]
mod tests;
