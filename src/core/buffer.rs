use crate::core::chunk::{Chunk, NONE};
use im::{OrdMap, Vector};

#[derive(Default, Clone)]
pub struct Delta {
    pub ops: Vec<DeltaOp>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeltaOp {
    Insert { index: usize, item: Chunk },
    Remove { index: usize },
    Replace { index: usize, new: Chunk },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateOutcome {
    pub lane_idx: usize,
    pub started_lane: bool,
}

#[derive(Default, Clone)]
pub struct Buffer {
    pub curr: Vector<Chunk>,
    // Deltas keep memory bounded while still allowing visible ranges to be reconstructed.
    pub deltas: Vector<Delta>,
    pub checkpoints: OrdMap<usize, Vector<Chunk>>,
    pub delta: Delta,
    mergers: Vec<u32>,
    transient_lanes: Vec<usize>,
}

impl Buffer {
    pub fn merger(&mut self, alias: u32) {
        self.mergers.push(alias);
    }

    pub fn expire_lane_after_snapshot(&mut self, lane_idx: usize) {
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

            UpdateOutcome { lane_idx: first_idx, started_lane: false }
        } else {
            self.curr.push_back(chunk);
            self.delta.ops.push(DeltaOp::Insert { index: self.curr.len() - 1, item: chunk });
            UpdateOutcome { lane_idx: self.curr.len() - 1, started_lane: true }
        }
    }

    pub fn backup(&mut self) {
        let old = std::mem::take(&mut self.delta);
        self.deltas.push_back(old);
        let idx = self.deltas.len().saturating_sub(1);
        if idx.is_multiple_of(100) {
            self.checkpoints.insert(idx, self.curr.clone());
        }
    }

    pub fn window(&self, start: usize, end: usize) -> Vector<Vector<Chunk>> {
        let mut history = Vector::new();

        // Start from the nearest checkpoint before the requested range.
        let checkpoint_idx = self.checkpoints.keys().rev().find(|&&idx| idx <= start).copied();

        let mut curr = checkpoint_idx.and_then(|idx| self.checkpoints.get(&idx)).cloned().unwrap_or_default();

        // Replay only the deltas needed to produce the requested visible range.
        let begin = checkpoint_idx.map_or(0, |idx| idx + 1);
        let end = end.min(self.deltas.len());

        for delta in self.deltas.iter().skip(begin).take(end - begin) {
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
                }
            }
            history.push_back(curr.clone());
        }

        history
    }
}

#[cfg(test)]
#[path = "../tests/core/buffer.rs"]
mod tests;
