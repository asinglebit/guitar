use crate::{
    core::oids::gix_to_git2_oid,
    core::worktrees::{WorktreeEntry, WorktreeKind},
    git::queries::helpers::UncommittedChanges,
};
use git2::{Error, Oid, Repository};
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

fn open_gix_repo(repo: &Repository) -> Result<gix::Repository, Error> {
    let path = gix_path(repo);
    let options = if repo.workdir().is_some() { gix::open::Options::default() } else { gix::open::Options::default().open_path_as_is(true) };

    gix::open_opts(path, options).map_err(|err| Error::from_str(&err.to_string()))
}

fn head_branch(repo: &gix::Repository) -> Option<String> {
    let head = repo.head_ref().ok().flatten()?;
    head.name().to_string().strip_prefix("refs/heads/").map(str::to_string)
}

fn head_oid(repo: &gix::Repository) -> Option<Oid> {
    Some(gix_to_git2_oid(repo.head_id().ok()?.detach()))
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

fn entry_from_gix_repo(repo: &gix::Repository, name: String, path: PathBuf, kind: WorktreeKind, current_path: &Path, dirty_check: DirtyCheck) -> WorktreeEntry {
    let canonical_entry_path = canonical_path(&path);
    WorktreeEntry {
        name,
        path: path.clone(),
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
        path: path.clone(),
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
    let current = canonical_path(&current);

    let current_repo = open_gix_repo(repo)?;
    let owner_repo = current_repo.main_repo().map_err(|err| Error::from_str(&err.to_string()))?;

    let mut entries = Vec::new();

    if let Some(main_path) = owner_repo.worktree().map(|worktree| worktree.base().to_path_buf()) {
        let main_name = main_path.file_name().and_then(|name| name.to_str()).unwrap_or("main").to_string();
        entries.push(entry_from_gix_repo(&owner_repo, main_name, main_path, WorktreeKind::Main, &current, dirty_check));
    }

    let mut linked: Vec<WorktreeEntry> = owner_repo.worktrees().map_err(|err| Error::from_str(&err.to_string()))?.into_iter().filter_map(|proxy| linked_entry(proxy, &current, dirty_check)).collect();
    linked.sort_by(|a, b| a.name.cmp(&b.name));
    entries.extend(linked);

    Ok(entries)
}

pub fn list_worktrees(repo: &Repository, current_path: Option<&Path>) -> Result<Vec<WorktreeEntry>, Error> {
    list_worktrees_with_dirty_check(repo, current_path, DirtyCheck::Compute)
}

pub fn list_worktrees_metadata(repo: &Repository, current_path: Option<&Path>) -> Result<Vec<WorktreeEntry>, Error> {
    list_worktrees_with_dirty_check(repo, current_path, DirtyCheck::Skip)
}

pub fn list_worktrees_metadata_with_current_dirty(repo: &Repository, current_path: Option<&Path>, uncommitted: &UncommittedChanges) -> Result<Vec<WorktreeEntry>, Error> {
    let mut entries = list_worktrees_metadata(repo, current_path)?;
    let is_dirty = !uncommitted.is_clean;
    for entry in &mut entries {
        if entry.is_current {
            entry.is_dirty = is_dirty;
        }
    }
    Ok(entries)
}

#[cfg(test)]
#[path = "../../tests/git/queries/worktrees.rs"]
mod tests;
