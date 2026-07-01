use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorktreeKind {
    Main,
    Linked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeEntry {
    pub name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head: Option<gix::ObjectId>,
    pub alias: Option<u32>,
    pub kind: WorktreeKind,
    pub is_current: bool,
    pub is_valid: bool,
    pub is_prunable: bool,
    pub locked_reason: Option<String>,
    pub is_dirty: bool,
}

impl WorktreeEntry {
    pub fn is_main(&self) -> bool {
        self.kind == WorktreeKind::Main
    }

    pub fn is_linked(&self) -> bool {
        self.kind == WorktreeKind::Linked
    }

    pub fn can_lock(&self) -> bool {
        self.is_linked() && self.is_valid
    }

    pub fn can_remove(&self) -> bool {
        self.is_linked() && !self.is_current && self.locked_reason.is_none()
    }
}

#[derive(Default)]
pub struct Worktrees {
    pub entries: Vec<WorktreeEntry>,
}

impl Worktrees {
    pub fn from_entries(entries: Vec<WorktreeEntry>) -> Self {
        Self { entries }
    }

    pub fn current_name(&self) -> Option<&str> {
        self.entries.iter().find(|entry| entry.is_current).map(|entry| entry.name.as_str())
    }
}

#[cfg(test)]
#[path = "../tests/core/worktrees.rs"]
mod tests;
