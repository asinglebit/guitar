use crate::git::queries::helpers::FileStatus;
use git2::{Delta, DiffFindOptions, DiffOptions, Oid, Repository};
use std::path::Path;

use gix::object::tree::diff::ChangeDetached;

pub fn changed_file_status_at_commit(repo: &Repository, oid: Oid, path: &str) -> Result<Option<FileStatus>, git2::Error> {
    let path = normalize_path(path);
    if path.is_empty() {
        return Ok(None);
    }

    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 { Some(commit.parent(0)?.tree()?) } else { None };

    let mut opts = DiffOptions::new();
    opts.include_untracked(false).recurse_untracked_dirs(false).include_typechange(false).ignore_submodules(true).show_binary(false).minimal(false).skip_binary_check(true);

    let mut diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;
    let mut find_options = DiffFindOptions::new();
    find_options.renames(true);
    diff.find_similar(Some(&mut find_options))?;
    for delta in diff.deltas() {
        if !delta_matches_path(&delta, &path) {
            continue;
        }

        return Ok(Some(file_status(delta.status())));
    }

    Ok(None)
}

pub(crate) fn changed_file_status_at_commit_gix(repo: &gix::Repository, oid: gix::ObjectId, path: &str) -> Result<Option<FileStatus>, git2::Error> {
    let path = normalize_path(path);
    if path.is_empty() {
        return Ok(None);
    }

    let commit = repo.find_commit(oid).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let tree = commit.tree().map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let parent_tree = commit
        .parent_ids()
        .next()
        .map(|parent_oid| {
            repo.find_commit(parent_oid.detach()).map_err(|error| git2::Error::from_str(&error.to_string())).and_then(|parent| parent.tree().map_err(|error| git2::Error::from_str(&error.to_string())))
        })
        .transpose()?;

    let mut options = gix::diff::Options::default();
    options.track_path().track_rewrites(Some(gix::diff::Rewrites::default()));

    let changes = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(options)).map_err(|error| git2::Error::from_str(&error.to_string()))?;

    for change in changes.iter() {
        if let Some(status) = file_status_from_gix_change(change, &path) {
            return Ok(Some(status));
        }
    }

    Ok(None)
}

fn file_status_from_gix_change(change: &ChangeDetached, path: &str) -> Option<FileStatus> {
    match change {
        ChangeDetached::Addition { location, .. } if path_matches(location, path) => Some(FileStatus::Added),
        ChangeDetached::Deletion { location, .. } if path_matches(location, path) => Some(FileStatus::Deleted),
        ChangeDetached::Modification { location, previous_entry_mode, entry_mode, .. } if path_matches(location, path) => {
            if typechange(previous_entry_mode, entry_mode) {
                Some(FileStatus::Deleted)
            } else {
                Some(FileStatus::Modified)
            }
        },
        ChangeDetached::Rewrite { source_location, location, copy, .. } => {
            if path_matches(location, path) {
                Some(if *copy { FileStatus::Added } else { FileStatus::Renamed })
            } else if !copy && path_matches(source_location, path) {
                Some(FileStatus::Renamed)
            } else {
                None
            }
        },
        _ => None,
    }
}

fn path_matches(location: &[u8], path: &str) -> bool {
    location == path.as_bytes()
}

fn typechange(previous_entry_mode: &gix::object::tree::EntryMode, entry_mode: &gix::object::tree::EntryMode) -> bool {
    previous_entry_mode.is_tree() != entry_mode.is_tree() || previous_entry_mode.is_link() != entry_mode.is_link() || previous_entry_mode.is_commit() != entry_mode.is_commit()
}

fn delta_matches_path(delta: &git2::DiffDelta<'_>, path: &str) -> bool {
    let selected = Path::new(path);
    delta.old_file().path().is_some_and(|old_path| old_path == selected) || delta.new_file().path().is_some_and(|new_path| new_path == selected)
}

fn file_status(delta: Delta) -> FileStatus {
    match delta {
        Delta::Added => FileStatus::Added,
        Delta::Modified => FileStatus::Modified,
        Delta::Deleted => FileStatus::Deleted,
        Delta::Renamed => FileStatus::Renamed,
        _ => FileStatus::Other,
    }
}

fn normalize_path(path: &str) -> String {
    let normalized = path.trim().replace('\\', "/");
    strip_leading_dot_slashes(&normalized).to_string()
}

fn strip_leading_dot_slashes(mut path: &str) -> &str {
    while let Some(stripped) = path.strip_prefix("./") {
        path = stripped;
    }
    path
}

#[cfg(test)]
#[path = "../../tests/git/queries/file_history.rs"]
mod tests;
