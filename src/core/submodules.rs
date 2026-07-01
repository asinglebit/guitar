use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmoduleStackEntry {
    pub parent_path: PathBuf,
    pub submodule_path: PathBuf,
    pub submodule_name: String,
}

impl SubmoduleStackEntry {
    pub fn new(parent_path: PathBuf, submodule_path: PathBuf, submodule_name: String) -> Self {
        Self { parent_path, submodule_path, submodule_name }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmoduleEntry {
    pub name: String,
    pub path: PathBuf,
    pub absolute_path: PathBuf,
    pub url: Option<String>,
    pub branch: Option<String>,
    pub head: Option<gix::ObjectId>,
    pub index: Option<gix::ObjectId>,
    pub workdir: Option<gix::ObjectId>,
    pub is_open: bool,
    pub is_uninitialized: bool,
    pub is_in_head: bool,
    pub is_in_index: bool,
    pub is_in_config: bool,
    pub is_in_workdir: bool,
    pub is_index_modified: bool,
    pub is_workdir_modified: bool,
    pub has_new_commits: bool,
    pub has_modified_content: bool,
    pub has_untracked_content: bool,
}

impl SubmoduleEntry {
    pub fn can_open(&self) -> bool {
        self.is_open
    }

    pub fn is_dirty(&self) -> bool {
        self.is_index_modified || self.is_workdir_modified || self.has_new_commits || self.has_modified_content || self.has_untracked_content
    }
}

#[derive(Default)]
pub struct Submodules {
    pub entries: Vec<SubmoduleEntry>,
}

impl Submodules {
    pub fn from_entries(entries: Vec<SubmoduleEntry>) -> Self {
        Self { entries }
    }
}

#[cfg(test)]
#[path = "../tests/core/submodules.rs"]
mod tests;
