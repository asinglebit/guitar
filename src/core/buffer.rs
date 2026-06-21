use crate::core::chunk::{Chunk, LaneRef, NONE};
use im::{OrdMap, Vector};
use std::ops::Deref;

#[derive(Default, Clone)]
pub struct Delta {
    pub ops: Vec<DeltaOp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeltaOp {
    Insert { index: usize, item: Chunk },
    Remove { index: usize },
    Replace { index: usize, new: Chunk },
    CompressedParentInsert { parent: u32 },
    CompressedParentRemove { parent: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateOutcome {
    pub lane: LaneRef,
    pub started_lane: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphSnapshot {
    pub lanes: Vector<Chunk>,
    pub compressed_parents: Vector<u32>,
}

impl GraphSnapshot {
    pub fn new(lanes: Vector<Chunk>, compressed_parents: Vector<u32>) -> Self {
        Self { lanes, compressed_parents }
    }

    pub fn from_lanes(lanes: Vector<Chunk>) -> Self {
        Self { lanes, compressed_parents: Vector::new() }
    }
}

impl From<Vector<Chunk>> for GraphSnapshot {
    fn from(lanes: Vector<Chunk>) -> Self {
        Self::from_lanes(lanes)
    }
}

impl Deref for GraphSnapshot {
    type Target = Vector<Chunk>;

    fn deref(&self) -> &Self::Target {
        &self.lanes
    }
}

pub type GraphHistory = Vector<GraphSnapshot>;

#[derive(Default, Clone)]
pub struct Buffer {
    pub curr: Vector<Chunk>,
    pub compressed_parents: Vector<u32>,
    // Deltas keep memory bounded while still allowing visible ranges to be reconstructed.
    pub deltas: Vector<Delta>,
    pub checkpoints: OrdMap<usize, GraphSnapshot>,
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
                self.delta.ops.push(DeltaOp::Replace { index: lane_idx, new: self.curr[lane_idx].clone() });
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
            self.curr.push_back(clone.clone());

            self.delta.ops.push(DeltaOp::Replace { index: merger_idx, new: self.curr[merger_idx].clone() });

            self.delta.ops.push(DeltaOp::Insert { index: self.curr.len() - 1, item: clone });
        }

        // Prefer replacing the parent lane; append only when the commit starts a new lane.
        if let Some(first_idx) = self.lane_for_parent(chunk.alias) {
            let old_alias = chunk.alias;
            let replacement = self.replacement_chunk_for_lane(first_idx, chunk);

            self.curr[first_idx] = replacement.clone();
            self.delta.ops.push(DeltaOp::Replace { index: first_idx, new: replacement });
            let compressed_parent_consumed = self.remove_compressed_parent(old_alias);

            // Clear consumed parent pointers so inactive branch lanes collapse into dummies.
            let mut changed_lanes = Vec::new();
            for (i, inner) in self.curr.iter_mut().enumerate() {
                if inner.alias == old_alias {
                    continue;
                }

                let parents_changed = inner.remove_parent(old_alias);

                if parents_changed {
                    if !inner.has_any_parent() && !(inner.is_flattened && !self.compressed_parents.is_empty()) {
                        *inner = Chunk::dummy();
                    }

                    changed_lanes.push(i);
                }
            }
            self.refresh_flattened_end_marker(compressed_parent_consumed, &mut changed_lanes);
            changed_lanes.sort_unstable();
            changed_lanes.dedup();
            for i in changed_lanes {
                self.delta.ops.push(DeltaOp::Replace { index: i, new: self.curr[i].clone() });
            }

            self.enforce_lane_limit(Some(first_idx));
            UpdateOutcome { lane: self.lane_ref_for_original_index(first_idx), started_lane: false }
        } else {
            self.curr.push_back(chunk.clone());
            self.delta.ops.push(DeltaOp::Insert { index: self.curr.len() - 1, item: chunk });
            let lane_idx = self.curr.len() - 1;
            self.enforce_lane_limit(Some(lane_idx));
            UpdateOutcome { lane: self.lane_ref_for_original_index(lane_idx), started_lane: true }
        }
    }

    fn lane_for_parent(&self, alias: u32) -> Option<usize> {
        self.curr.iter().position(|inner| inner.has_parent(alias)).or_else(|| self.has_compressed_parent(alias).then(|| self.flattened_lane_idx()).flatten())
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
            self.curr[cap_idx] = replacement.clone();
            self.delta.ops.push(DeltaOp::Replace { index: cap_idx, new: replacement });
        }

        while self.curr.len() > limit {
            let idx = self.curr.len() - 1;
            self.curr.pop_back();
            self.delta.ops.push(DeltaOp::Remove { index: idx });
        }

        self.purge_unstored_mergers();
        self.transient_lanes.retain(|lane_idx| *lane_idx < limit && (*lane_idx + 1 != limit || !self.curr.get(*lane_idx).is_some_and(|chunk| chunk.is_flattened)));
    }

    fn flattened_representative(&mut self, cap_idx: usize, preferred_idx: Option<usize>) -> Chunk {
        let preferred = preferred_idx
            .and_then(|idx| (idx >= cap_idx).then(|| self.curr.get(idx).map(|chunk| (idx, chunk))).flatten())
            .filter(|(_, chunk)| !chunk.is_dummy())
            .map(|(idx, chunk)| (idx, chunk.clone()));

        let fallback = self.curr.iter().enumerate().skip(cap_idx).find(|(_, chunk)| !chunk.is_dummy()).map(|(idx, chunk)| (idx, chunk.clone())).unwrap_or_else(|| (cap_idx, Chunk::dummy()));
        let (representative_idx, source) = preferred.unwrap_or(fallback);
        let mut representative = source.clone().with_flattened(true);
        let new_parents = self.compressed_parent_candidates(cap_idx, representative_idx);
        let start_marker = source.parent_a;
        let mut end_marker = NONE;
        for parent in new_parents {
            if self.add_compressed_parent(parent) {
                end_marker = parent;
            }
        }

        if start_marker != NONE {
            self.add_compressed_parent(start_marker);
            representative.parent_a = start_marker;
        }

        if end_marker == NONE {
            end_marker = self.compressed_parents.back().copied().unwrap_or(NONE);
        }
        representative.parent_b = end_marker;
        representative
    }

    fn compressed_parent_candidates(&self, cap_idx: usize, representative_idx: usize) -> Vec<u32> {
        let mut parents = Vec::new();
        if let Some(representative) = self.curr.get(representative_idx)
            && !representative.is_dummy()
            && !representative.is_flattened
        {
            append_unique_parents(&mut parents, representative);
        }

        for (idx, chunk) in self.curr.iter().enumerate().skip(cap_idx) {
            if idx == representative_idx || chunk.is_dummy() || chunk.is_flattened {
                continue;
            }
            append_unique_parents(&mut parents, chunk);
        }
        parents
    }

    fn replacement_chunk_for_lane(&mut self, lane_idx: usize, mut replacement: Chunk) -> Chunk {
        let old = &self.curr[lane_idx];
        if !old.is_flattened {
            return replacement;
        }

        if replacement.parent_b != NONE {
            self.add_compressed_parent(replacement.parent_b);
        }

        replacement = replacement.with_flattened(true);
        let end_marker = self.compressed_parents.back().copied().unwrap_or(NONE);
        replacement.parent_b = end_marker;
        replacement
    }

    fn add_compressed_parent(&mut self, parent: u32) -> bool {
        if parent == NONE || self.has_compressed_parent(parent) {
            return false;
        }

        self.compressed_parents.push_back(parent);
        self.delta.ops.push(DeltaOp::CompressedParentInsert { parent });
        true
    }

    fn remove_compressed_parent(&mut self, parent: u32) -> bool {
        let Some(index) = self.compressed_parents.iter().position(|candidate| *candidate == parent) else {
            return false;
        };

        self.compressed_parents.remove(index);
        self.delta.ops.push(DeltaOp::CompressedParentRemove { parent });
        true
    }

    fn has_compressed_parent(&self, parent: u32) -> bool {
        parent != NONE && self.compressed_parents.iter().any(|candidate| *candidate == parent)
    }

    fn flattened_lane_idx(&self) -> Option<usize> {
        self.curr.iter().position(|chunk| chunk.is_flattened)
    }

    fn refresh_flattened_end_marker(&mut self, compressed_parent_consumed: bool, changed_lanes: &mut Vec<usize>) {
        if !compressed_parent_consumed {
            return;
        }

        let Some(lane_idx) = self.flattened_lane_idx() else {
            return;
        };
        if !self.curr[lane_idx].is_flattened {
            return;
        }

        let end_marker = self.compressed_parents.back().copied().unwrap_or(NONE);
        if self.curr[lane_idx].parent_b != end_marker {
            self.curr[lane_idx].parent_b = end_marker;
            changed_lanes.push(lane_idx);
        }
        if !self.curr[lane_idx].has_any_parent() && self.compressed_parents.is_empty() {
            self.curr[lane_idx] = Chunk::dummy();
            changed_lanes.push(lane_idx);
        }
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
        self.deltas.push_back(old);
        let idx = self.deltas.len().saturating_sub(1);
        if idx.is_multiple_of(100) {
            self.checkpoints.insert(idx, self.snapshot());
        }
    }

    fn snapshot(&self) -> GraphSnapshot {
        GraphSnapshot::new(self.curr.clone(), self.compressed_parents.clone())
    }

    pub fn window(&self, start: usize, end: usize) -> GraphHistory {
        let mut history = Vector::new();

        // Start from the nearest checkpoint before the requested range.
        let checkpoint_idx = self.checkpoints.keys().rev().find(|&&idx| idx <= start).copied();

        let mut snapshot = checkpoint_idx.and_then(|idx| self.checkpoints.get(&idx)).cloned().unwrap_or_default();

        // Replay only the deltas needed to produce the requested visible range.
        let begin = checkpoint_idx.map_or(0, |idx| idx + 1);
        let end = end.min(self.deltas.len());

        for delta in self.deltas.iter().skip(begin).take(end - begin) {
            for op in delta.ops.iter() {
                match op {
                    DeltaOp::Insert { index, item } => {
                        snapshot.lanes.insert(*index, item.clone());
                    },
                    DeltaOp::Remove { index } => {
                        snapshot.lanes.remove(*index);
                    },
                    DeltaOp::Replace { index, new } => {
                        snapshot.lanes[*index] = new.clone();
                    },
                    DeltaOp::CompressedParentInsert { parent } => {
                        if *parent != NONE && !snapshot.compressed_parents.iter().any(|candidate| candidate == parent) {
                            snapshot.compressed_parents.push_back(*parent);
                        }
                    },
                    DeltaOp::CompressedParentRemove { parent } => {
                        if let Some(index) = snapshot.compressed_parents.iter().position(|candidate| candidate == parent) {
                            snapshot.compressed_parents.remove(index);
                        }
                    },
                }
            }
            history.push_back(snapshot.clone());
        }

        history
    }
}

fn append_unique_parents(parents: &mut Vec<u32>, chunk: &Chunk) {
    for parent in chunk.parent_aliases() {
        if parent != NONE && !parents.contains(&parent) {
            parents.push(parent);
        }
    }
}

#[cfg(test)]
#[path = "../tests/core/buffer.rs"]
mod tests;
