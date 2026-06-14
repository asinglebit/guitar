use crate::core::oids::Oids;
use git2::Oid;
use std::collections::HashMap;
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
    pub head: Option<Oid>,
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
    pub by_alias: HashMap<u32, Vec<usize>>,
}

impl Worktrees {
    pub fn from_entries(entries: Vec<WorktreeEntry>) -> Self {
        Self { entries, by_alias: HashMap::new() }
    }

    pub fn current_name(&self) -> Option<&str> {
        self.entries.iter().find(|entry| entry.is_current).map(|entry| entry.name.as_str())
    }

    pub fn refresh_aliases(&mut self, oids: &Oids) {
        self.by_alias.clear();

        for (idx, entry) in self.entries.iter_mut().enumerate() {
            entry.alias = entry.head.and_then(|oid| oids.aliases.get(&oid).copied());

            if let Some(alias) = entry.alias {
                self.by_alias.entry(alias).or_default().push(idx);
            }
        }
    }

    pub fn get_by_alias(&self, alias: &u32) -> Vec<&WorktreeEntry> {
        self.by_alias.get(alias).into_iter().flat_map(|indices| indices.iter()).filter_map(|idx| self.entries.get(*idx)).collect()
    }

    pub fn has_detached_or_unlabeled_at(&self, alias: &u32, branch_labels: bool) -> bool {
        self.get_by_alias(alias).iter().any(|entry| entry.branch.is_none() || !branch_labels)
    }
}

#[cfg(test)]
#[path = "../tests/core/worktrees.rs"]
mod tests;
