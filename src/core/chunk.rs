#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Markers {
    Uncommitted,
    Commit,
    Dummy,
}

pub const NONE: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneRef {
    pub index: usize,
    pub is_flattened: bool,
}

impl LaneRef {
    pub const fn new(index: usize, is_flattened: bool) -> Self {
        Self { index, is_flattened }
    }
}

// A lane entry points at the commit alias currently occupying that graph lane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub alias: u32,
    pub parent_a: u32,
    pub parent_b: u32,
    pub marker: Markers,
    pub is_flattened: bool,
}

impl Default for Chunk {
    fn default() -> Self {
        Chunk { alias: NONE, parent_a: NONE, parent_b: NONE, marker: Markers::Dummy, is_flattened: false }
    }
}

impl Chunk {
    pub fn uncommitted(parent_a: u32, parent_b: u32) -> Self {
        Chunk { alias: NONE, parent_a, parent_b, marker: Markers::Uncommitted, is_flattened: false }
    }

    pub fn commit(alias: u32, parent_a: u32, parent_b: u32) -> Self {
        Chunk { alias, parent_a, parent_b, marker: Markers::Commit, is_flattened: false }
    }

    pub fn dummy() -> Self {
        Chunk { alias: NONE, parent_a: NONE, parent_b: NONE, marker: Markers::Dummy, is_flattened: false }
    }

    pub fn with_flattened(mut self, is_flattened: bool) -> Self {
        self.is_flattened = is_flattened && !self.is_dummy();
        self
    }

    pub fn remove_parent(&mut self, parent: u32) -> bool {
        let mut changed = false;

        if self.parent_a == parent {
            self.parent_a = NONE;
            changed = true;
        }

        if self.parent_b == parent {
            self.parent_b = NONE;
            changed = true;
        }

        changed
    }

    pub fn has_parent(&self, parent: u32) -> bool {
        parent != NONE && (self.parent_a == parent || self.parent_b == parent)
    }

    pub fn has_any_parent(&self) -> bool {
        self.parent_a != NONE || self.parent_b != NONE
    }

    pub fn parent_aliases(&self) -> Vec<u32> {
        let mut parents = Vec::new();
        for parent in [self.parent_a, self.parent_b] {
            if parent != NONE && !parents.contains(&parent) {
                parents.push(parent);
            }
        }
        parents
    }

    pub fn is_dummy(&self) -> bool {
        self.marker == Markers::Dummy
    }
}
