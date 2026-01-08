#[derive(Clone, PartialEq)]
pub enum Markers {
    Uncommitted,
    Commit,
    Dummy,
}

pub const NONE: u32 = u32::MAX;

#[derive(Clone)]
pub struct Chunk {
    pub alias: u32,
    pub parent_a: u32,
    pub parent_b: u32,
    pub marker: Markers,
}

impl Default for Chunk {
    fn default() -> Self {
        Chunk {
            alias: NONE,
            parent_a: NONE,
            parent_b: NONE,
            marker: Markers::Dummy,
        }
    }
}

impl Chunk {
    pub fn uncommitted(parent_a: u32, parent_b: u32) -> Self {
        Chunk {
            alias: NONE,
            parent_a,
            parent_b,
            marker: Markers::Uncommitted,
        }
    }

    pub fn commit(alias: u32, parent_a: u32, parent_b: u32) -> Self {
        Chunk {
            alias,
            parent_a,
            parent_b,
            marker: Markers::Commit,
        }
    }

    pub fn dummy() -> Self {
        Chunk {
            alias: NONE,
            parent_a: NONE,
            parent_b: NONE,
            marker: Markers::Dummy,
        }
    }

    pub fn is_dummy(&self) -> bool {
        self.marker == Markers::Dummy
    }
}
