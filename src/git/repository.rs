use git2::{Error, Repository};
use std::path::Path;

/// Backend-open helpers kept in one place so repository loading can stay a narrow seam.
pub fn open(path: impl AsRef<Path>) -> Result<Repository, Error> {
    Repository::open(path)
}

/// Worktree-aware owner lookup for linked repositories that share a common directory.
pub fn open_worktree_owner(repo: &Repository) -> Result<Repository, Error> {
    Repository::open(repo.commondir())
}

#[cfg(test)]
#[path = "../tests/git/repository.rs"]
mod tests;
