use crate::{
    core::worktrees::{WorktreeEntry, WorktreeKind},
    git::gix::gix_error,
    git::queries::helpers::UncommittedChanges,
};
use git2::{Error, Repository};
use gix::bstr::ByteSlice;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn canonical_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn gix_path(repo: &Repository) -> PathBuf {
    repo.workdir().map(Path::to_path_buf).unwrap_or_else(|| repo.path().to_path_buf())
}

fn open_repo_path(path: PathBuf, options: gix::open::Options) -> Result<gix::Repository, Error> {
    gix::open_opts(path, options).map_err(gix_error)
}

fn open_repo(repo: &Repository) -> Result<gix::Repository, Error> {
    open_repo_path(gix_path(repo), if repo.workdir().is_some() { gix::open::Options::default() } else { gix::open::Options::default().open_path_as_is(true) })
}

fn head_branch(repo: &gix::Repository) -> Option<String> {
    let head = repo.head_ref().ok().flatten()?;
    head.name().as_bstr().strip_prefix(b"refs/heads/")?.to_str().ok().map(str::to_string)
}

fn head_oid(repo: &gix::Repository) -> Option<gix::ObjectId> {
    Some(repo.head_id().ok()?.detach())
}

fn repo_dirty(repo: &gix::Repository) -> bool {
    repo.status(gix::progress::Discard)
        .ok()
        .and_then(|status| {
            status
                .untracked_files(gix::status::UntrackedFiles::Files)
                .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
                .index_worktree_rewrites(None)
                .index_worktree_submodules(gix::status::Submodule::AsConfigured { check_dirty: true })
                .index_worktree_options_mut(|opts| {
                    opts.dirwalk_options = None;
                })
                .into_iter(Vec::new())
                .ok()
                .map(|mut iter| iter.any(|item| item.is_ok()))
        })
        .unwrap_or(false)
}

fn main_worktree_path(repo: &Repository) -> Option<PathBuf> {
    repo.commondir().parent().map(Path::to_path_buf)
}

#[derive(Clone, Copy)]
enum DirtyCheck {
    Skip,
    Compute,
}

fn worktree_entry_from_repo(repo: &gix::Repository, name: String, path: PathBuf, kind: WorktreeKind, current_path: &Path, dirty_check: DirtyCheck) -> WorktreeEntry {
    let canonical_entry_path = canonical_path(&path);
    WorktreeEntry {
        name,
        path,
        branch: head_branch(repo),
        head: head_oid(repo),
        alias: None,
        kind,
        is_current: canonical_entry_path == current_path,
        is_valid: true,
        is_prunable: false,
        locked_reason: None,
        is_dirty: matches!(dirty_check, DirtyCheck::Compute) && repo_dirty(repo),
    }
}

fn linked_entry(proxy: gix::worktree::Proxy<'_>, current_path: &Path, dirty_check: DirtyCheck) -> Option<WorktreeEntry> {
    let path = proxy.base().ok()?;
    let canonical_entry_path = canonical_path(&path);
    let name = proxy.id().to_string();
    let is_locked = proxy.is_locked();
    let locked_reason = proxy.lock_reason().map(|reason| reason.to_string());
    let repo = proxy.into_repo().ok();

    let mut entry = WorktreeEntry {
        name,
        path,
        branch: repo.as_ref().and_then(head_branch),
        head: repo.as_ref().and_then(head_oid),
        alias: None,
        kind: WorktreeKind::Linked,
        is_current: canonical_entry_path == current_path,
        is_valid: repo.is_some(),
        is_prunable: !is_locked && repo.is_none(),
        locked_reason,
        is_dirty: matches!(dirty_check, DirtyCheck::Compute) && repo.as_ref().is_some_and(repo_dirty),
    };

    if entry.is_valid {
        entry.is_prunable = false;
    }

    Some(entry)
}

fn list_worktrees_with_dirty_check(repo: &Repository, current_path: Option<&Path>, dirty_check: DirtyCheck) -> Result<Vec<WorktreeEntry>, Error> {
    let current = current_path.map(Path::to_path_buf).or_else(|| repo.workdir().map(Path::to_path_buf)).or_else(|| main_worktree_path(repo)).unwrap_or_else(|| PathBuf::from("."));
    list_worktrees_with_dirty_check_from_path(open_repo(repo)?, current, dirty_check)
}

fn list_worktrees_with_dirty_check_from_path(current_repo: gix::Repository, current: PathBuf, dirty_check: DirtyCheck) -> Result<Vec<WorktreeEntry>, Error> {
    let current = canonical_path(&current);
    let owner_repo = current_repo.main_repo().map_err(gix_error)?;

    let main_entry = owner_repo.worktree().map(|worktree| {
        let main_path = worktree.base().to_path_buf();
        let main_name = main_path.file_name().and_then(|name| name.to_str()).unwrap_or("main").to_string();
        worktree_entry_from_repo(&owner_repo, main_name, main_path, WorktreeKind::Main, &current, dirty_check)
    });

    let first_linked = usize::from(main_entry.is_some());
    let linked_entries = owner_repo.worktrees().map_err(gix_error)?.into_iter().filter_map(|proxy| linked_entry(proxy, &current, dirty_check));
    let mut entries: Vec<_> = main_entry.into_iter().chain(linked_entries).collect();
    entries[first_linked..].sort_by(|a, b| a.name.cmp(&b.name));

    Ok(entries)
}

pub fn list_worktrees(repo: &Repository, current_path: Option<&Path>) -> Result<Vec<WorktreeEntry>, Error> {
    list_worktrees_with_dirty_check(repo, current_path, DirtyCheck::Compute)
}

pub fn list_worktrees_metadata(repo: &Repository, current_path: Option<&Path>) -> Result<Vec<WorktreeEntry>, Error> {
    list_worktrees_with_dirty_check(repo, current_path, DirtyCheck::Skip)
}

pub fn list_worktrees_metadata_from_path(path: impl AsRef<Path>, current_path: Option<&Path>) -> Result<Vec<WorktreeEntry>, Error> {
    let current_path = current_path.map(Path::to_path_buf).unwrap_or_else(|| path.as_ref().to_path_buf());
    list_worktrees_with_dirty_check_from_path(open_repo_path(path.as_ref().to_path_buf(), gix::open::Options::default())?, current_path, DirtyCheck::Skip)
}

pub fn list_worktrees_metadata_with_current_dirty(repo: &Repository, current_path: Option<&Path>, uncommitted: &UncommittedChanges) -> Result<Vec<WorktreeEntry>, Error> {
    let mut entries = list_worktrees_metadata(repo, current_path)?;
    mark_current_dirty(&mut entries, uncommitted);
    Ok(entries)
}

pub fn list_worktrees_metadata_with_current_dirty_from_path(path: impl AsRef<Path>, current_path: Option<&Path>, uncommitted: &UncommittedChanges) -> Result<Vec<WorktreeEntry>, Error> {
    let mut entries = list_worktrees_metadata_from_path(path.as_ref(), current_path)?;
    mark_current_dirty(&mut entries, uncommitted);
    Ok(entries)
}

fn mark_current_dirty(entries: &mut [WorktreeEntry], uncommitted: &UncommittedChanges) {
    entries.iter_mut().filter(|entry| entry.is_current).for_each(|entry| {
        entry.is_dirty = !uncommitted.is_clean;
    });
}

#[cfg(test)]
#[path = "../../tests/git/queries/worktrees.rs"]
mod tests;
