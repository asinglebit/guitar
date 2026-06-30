use crate::{
    core::worktrees::{WorktreeEntry, WorktreeKind},
    git::queries::commits::get_current_branch,
};
use git2::{Repository, Worktree, WorktreeLockStatus, WorktreePruneOptions};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn canonical_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    canonical_path(a) == canonical_path(b)
}

fn repo_dirty(repo: &Repository) -> bool {
    repo.statuses(None).map(|statuses| !statuses.is_empty()).unwrap_or(false)
}

fn repo_head(repo: &Repository) -> Option<git2::Oid> {
    repo.head().ok().and_then(|head| head.target())
}

fn main_worktree_path(repo: &Repository) -> Option<PathBuf> {
    if repo.is_bare() {
        return None;
    }

    repo.commondir().parent().map(Path::to_path_buf)
}

fn entry_from_repository(name: String, path: PathBuf, kind: WorktreeKind, current_path: &Path) -> WorktreeEntry {
    let repo = Repository::open(&path).ok();
    let branch = repo.as_ref().and_then(get_current_branch);
    let head = repo.as_ref().and_then(repo_head);
    let is_dirty = repo.as_ref().is_some_and(repo_dirty);

    WorktreeEntry {
        name,
        path: path.clone(),
        branch,
        head,
        alias: None,
        kind,
        is_current: paths_equal(&path, current_path),
        is_valid: repo.is_some(),
        is_prunable: false,
        locked_reason: None,
        is_dirty,
    }
}

fn linked_entry(repo: &Repository, worktree_name: &str, current_path: &Path) -> Option<WorktreeEntry> {
    let worktree = repo.find_worktree(worktree_name).ok()?;
    let path = worktree.path().to_path_buf();
    let is_valid = worktree.validate().is_ok();
    let locked_reason = match worktree.is_locked() {
        Ok(WorktreeLockStatus::Unlocked) => None,
        Ok(WorktreeLockStatus::Locked(reason)) => Some(reason.unwrap_or_default()),
        Err(_) => None,
    };
    let is_prunable = is_prunable(&worktree);

    let mut entry = entry_from_repository(worktree_name.to_string(), path, WorktreeKind::Linked, current_path);
    entry.is_valid = is_valid;
    entry.is_prunable = is_prunable;
    entry.locked_reason = locked_reason;

    Some(entry)
}

fn is_prunable(worktree: &Worktree) -> bool {
    let mut opts = WorktreePruneOptions::new();
    worktree.is_prunable(Some(&mut opts)).unwrap_or(false)
}

pub fn list_worktrees(repo: &Repository, current_path: Option<&Path>) -> Result<Vec<WorktreeEntry>, git2::Error> {
    let owner = Repository::open(repo.commondir()).ok();
    let worktree_repo = owner.as_ref().unwrap_or(repo);
    let current = current_path.map(Path::to_path_buf).or_else(|| repo.workdir().map(Path::to_path_buf)).or_else(|| main_worktree_path(repo)).unwrap_or_else(|| PathBuf::from("."));

    let mut entries = Vec::new();

    if let Some(main_path) = main_worktree_path(worktree_repo) {
        let main_name = main_path.file_name().and_then(|name| name.to_str()).unwrap_or("main").to_string();
        entries.push(entry_from_repository(main_name, main_path, WorktreeKind::Main, &current));
    }

    let names = worktree_repo.worktrees()?;
    let mut linked: Vec<WorktreeEntry> = names.iter().flatten().filter_map(|name| linked_entry(worktree_repo, name, &current)).collect();
    linked.sort_by(|a, b| a.name.cmp(&b.name));
    entries.extend(linked);

    Ok(entries)
}

#[cfg(test)]
#[path = "../../tests/git/queries/worktrees.rs"]
mod tests;
