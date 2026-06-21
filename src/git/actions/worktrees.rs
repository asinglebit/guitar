use crate::git::repository::open_worktree_owner;
use git2::{BranchType, Error, Oid, Repository, WorktreeAddOptions, WorktreeLockStatus, WorktreePruneOptions};
use std::path::Path;

fn worktree_owner(repo: &Repository) -> Result<Repository, Error> {
    open_worktree_owner(repo)
}

pub fn is_valid_worktree_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name != "." && name != ".." && !name.contains('/') && !name.contains('\\')
}

pub fn create_worktree(repo: &Repository, name: &str, path: &Path, target_oid: Oid) -> Result<(), Error> {
    if !is_valid_worktree_name(name) {
        return Err(Error::from_str("Worktree names cannot be empty or contain path separators"));
    }

    let repo = worktree_owner(repo)?;
    let target_commit = repo.find_commit(target_oid)?;

    let result = {
        let branch = repo.branch(name, &target_commit, false)?;
        let reference = branch.into_reference();
        let mut opts = WorktreeAddOptions::new();
        opts.reference(Some(&reference));
        repo.worktree(name, path, Some(&opts)).map(|_| ())
    };

    if let Err(error) = result {
        if let Ok(mut branch) = repo.find_branch(name, BranchType::Local) {
            let _ = branch.delete();
        }
        return Err(error);
    }

    Ok(())
}

pub fn remove_worktree(repo: &Repository, name: &str) -> Result<(), Error> {
    let repo = worktree_owner(repo)?;
    let worktree = repo.find_worktree(name)?;

    match worktree.is_locked()? {
        WorktreeLockStatus::Unlocked => {},
        WorktreeLockStatus::Locked(_) => return Err(Error::from_str("Cannot remove a locked worktree")),
    }

    let mut opts = WorktreePruneOptions::new();
    if worktree.validate().is_ok() {
        opts.valid(true).working_tree(true);
    }

    worktree.prune(Some(&mut opts))
}

pub fn lock_worktree(repo: &Repository, name: &str, reason: Option<&str>) -> Result<(), Error> {
    let repo = worktree_owner(repo)?;
    let worktree = repo.find_worktree(name)?;
    let reason = reason.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    });
    worktree.lock(reason)
}

pub fn unlock_worktree(repo: &Repository, name: &str) -> Result<(), Error> {
    let repo = worktree_owner(repo)?;
    let worktree = repo.find_worktree(name)?;
    worktree.unlock()
}

#[cfg(test)]
#[path = "../../tests/git/actions/worktrees.rs"]
mod tests;
