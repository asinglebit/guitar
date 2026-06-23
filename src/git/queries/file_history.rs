use crate::{core::oids::git2_to_gix_oid, git::queries::helpers::FileStatus};
use git2::{Oid, Repository};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use gix::object::tree::diff::ChangeDetached;

pub fn changed_file_status_at_commit(repo: &Repository, oid: Oid, path: &str) -> Result<Option<FileStatus>, git2::Error> {
    let repo_path = repo.workdir().unwrap_or(repo.path()).to_path_buf();
    let normalized_path = normalize_path(path);
    if let Some(status) = FILE_STATUS_CACHE.with(|cache| cache.borrow().get(&(repo_path.clone(), oid, normalized_path.clone())).copied()) {
        return Ok(status);
    }

    let repo = open_repo(repo)?;
    let status = changed_file_status_at_commit_from_repo(&repo, git2_to_gix_oid(oid), &normalized_path)?;
    FILE_STATUS_CACHE.with(|cache| {
        cache.borrow_mut().insert((repo_path, oid, normalized_path), status);
    });
    Ok(status)
}

thread_local! {
    static GIX_REPO_CACHE: RefCell<HashMap<PathBuf, gix::Repository>> = RefCell::new(HashMap::new());
    static COMMIT_TREE_CACHE: RefCell<HashMap<gix::ObjectId, (gix::ObjectId, Option<gix::ObjectId>)>> = RefCell::new(HashMap::new());
    static FILE_STATUS_CACHE: RefCell<HashMap<(PathBuf, Oid, String), Option<FileStatus>>> = RefCell::new(HashMap::new());
}

#[doc(hidden)]
pub fn clear_changed_file_status_cache() {
    FILE_STATUS_CACHE.with(|cache| cache.borrow_mut().clear());
}

fn open_repo(repo: &Repository) -> Result<gix::Repository, git2::Error> {
    let path = repo.workdir().unwrap_or(repo.path());
    GIX_REPO_CACHE.with(|cache| {
        if let Some(repo) = cache.borrow().get(path).cloned() {
            return Ok(repo);
        }

        let repo = gix::open(path).map_err(|error| git2::Error::from_str(&error.to_string()))?;
        cache.borrow_mut().insert(path.to_path_buf(), repo.clone());
        Ok(repo)
    })
}

pub(crate) fn changed_file_status_at_commit_from_repo(repo: &gix::Repository, oid: gix::ObjectId, path: &str) -> Result<Option<FileStatus>, git2::Error> {
    let path = normalize_path(path);
    if path.is_empty() {
        return Ok(None);
    }

    let (tree_id, parent_tree_id) = commit_tree_ids(repo, oid)?;
    let tree = repo.find_tree(tree_id).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let parent_tree = parent_tree_id.map(|tree_id| repo.find_tree(tree_id).map_err(|error| git2::Error::from_str(&error.to_string()))).transpose()?;

    match status_from_path_lookup(repo, &tree, parent_tree.as_ref(), &path)? {
        PathLookupStatus::Changed(status) => return Ok(Some(status)),
        PathLookupStatus::Unchanged => return Ok(None),
        PathLookupStatus::NeedsRenameDetection => {},
    }

    let mut options = gix::diff::Options::default();
    options.track_path().track_rewrites(Some(gix::diff::Rewrites::default()));

    let changes = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(options)).map_err(|error| git2::Error::from_str(&error.to_string()))?;

    for change in changes.iter() {
        if let Some(status) = file_status_from_change(change, &path) {
            return Ok(Some(status));
        }
    }

    Ok(None)
}

fn commit_tree_ids(repo: &gix::Repository, oid: gix::ObjectId) -> Result<(gix::ObjectId, Option<gix::ObjectId>), git2::Error> {
    if let Some(ids) = COMMIT_TREE_CACHE.with(|cache| cache.borrow().get(&oid).copied()) {
        return Ok(ids);
    }

    let commit = repo.find_commit(oid).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let tree_id = commit.tree_id().map_err(|error| git2::Error::from_str(&error.to_string()))?.detach();
    let parent_tree_id = commit
        .parent_ids()
        .next()
        .map(|parent_oid| {
            repo.find_commit(parent_oid.detach())
                .map_err(|error| git2::Error::from_str(&error.to_string()))
                .and_then(|parent| parent.tree_id().map(|id| id.detach()).map_err(|error| git2::Error::from_str(&error.to_string())))
        })
        .transpose()?;

    let ids = (tree_id, parent_tree_id);
    COMMIT_TREE_CACHE.with(|cache| {
        cache.borrow_mut().insert(oid, ids);
    });
    Ok(ids)
}

enum PathLookupStatus {
    Changed(FileStatus),
    Unchanged,
    NeedsRenameDetection,
}

fn status_from_path_lookup(repo: &gix::Repository, tree: &gix::Tree<'_>, parent_tree: Option<&gix::Tree<'_>>, path: &str) -> Result<PathLookupStatus, git2::Error> {
    let current = tree_entry_identity(tree, path)?;
    let parent = parent_tree.map(|parent_tree| tree_entry_identity(parent_tree, path)).transpose()?.flatten();

    Ok(match (parent, current) {
        (Some(parent), Some(current)) if parent == current => PathLookupStatus::Unchanged,
        (Some(parent), Some(current)) if typechange(&parent.1, &current.1) => PathLookupStatus::Changed(FileStatus::Deleted),
        (Some(_), Some(_)) => PathLookupStatus::Changed(FileStatus::Modified),
        (None, Some(_)) if parent_tree.is_none() => PathLookupStatus::Changed(FileStatus::Added),
        (Some(parent), None) => exact_missing_path_status(repo, tree, parent_tree, path, &parent.0, parent.1)?,
        (None, Some(current)) if parent_tree.is_some() => exact_new_path_status(repo, tree, parent_tree, path, &current.0, current.1)?,
        _ => PathLookupStatus::NeedsRenameDetection,
    })
}

fn tree_entry_identity(tree: &gix::Tree<'_>, path: &str) -> Result<Option<(gix::ObjectId, gix::object::tree::EntryMode)>, git2::Error> {
    let selected = Path::new(path);
    if selected.parent().is_none_or(|parent| parent.as_os_str().is_empty()) {
        let Some(filename) = selected.file_name().and_then(|name| name.to_str()) else {
            return Ok(None);
        };
        return Ok(tree.find_entry(filename.as_bytes()).map(|entry| (entry.object_id(), entry.mode())));
    }

    Ok(tree.lookup_entry_by_path(selected).map_err(|error| git2::Error::from_str(&error.to_string()))?.map(|entry| (entry.object_id(), entry.mode())))
}

fn exact_missing_path_status(
    repo: &gix::Repository, tree: &gix::Tree<'_>, parent_tree: Option<&gix::Tree<'_>>, selected_path: &str, oid: &gix::oid, mode: gix::object::tree::EntryMode,
) -> Result<PathLookupStatus, git2::Error> {
    let Some(parent_tree) = parent_tree else {
        return Ok(PathLookupStatus::NeedsRenameDetection);
    };

    if same_directory_exact_entry_path(tree, selected_path, oid, mode)?.is_some() {
        return Ok(PathLookupStatus::Changed(FileStatus::Renamed));
    }

    if raw_tree_changes(repo, parent_tree, tree)?
        .into_iter()
        .any(|change| matches!(change, ChangeDetached::Addition { location, entry_mode, id, .. } if location.as_slice() != selected_path.as_bytes() && id == oid && entry_mode == mode))
    {
        Ok(PathLookupStatus::Changed(FileStatus::Renamed))
    } else {
        Ok(PathLookupStatus::Changed(FileStatus::Deleted))
    }
}

fn exact_new_path_status(
    repo: &gix::Repository, tree: &gix::Tree<'_>, parent_tree: Option<&gix::Tree<'_>>, selected_path: &str, oid: &gix::oid, mode: gix::object::tree::EntryMode,
) -> Result<PathLookupStatus, git2::Error> {
    let Some(parent_tree) = parent_tree else {
        return Ok(PathLookupStatus::Changed(FileStatus::Added));
    };

    if let Some(source_path) = same_directory_exact_entry_path(parent_tree, selected_path, oid, mode)? {
        let source_still_exists =
            tree.lookup_entry_by_path(Path::new(&source_path)).map_err(|error| git2::Error::from_str(&error.to_string()))?.is_some_and(|entry| entry.oid() == oid && entry.mode() == mode);
        return Ok(PathLookupStatus::Changed(if source_still_exists { FileStatus::Added } else { FileStatus::Renamed }));
    }

    let Some(source_location) = raw_tree_changes(repo, parent_tree, tree)?.into_iter().find_map(|change| match change {
        ChangeDetached::Deletion { location, entry_mode, id, .. } if location.as_slice() != selected_path.as_bytes() && id == oid && entry_mode == mode => Some(location),
        _ => None,
    }) else {
        return Ok(PathLookupStatus::Changed(FileStatus::Added));
    };

    let source_path = Path::new(std::str::from_utf8(source_location.as_slice()).map_err(|error| git2::Error::from_str(&error.to_string()))?);
    let source_still_exists = tree.lookup_entry_by_path(source_path).map_err(|error| git2::Error::from_str(&error.to_string()))?.is_some_and(|entry| entry.oid() == oid && entry.mode() == mode);
    Ok(PathLookupStatus::Changed(if source_still_exists { FileStatus::Added } else { FileStatus::Renamed }))
}

fn same_directory_exact_entry_path(tree: &gix::Tree<'_>, selected_path: &str, oid: &gix::oid, mode: gix::object::tree::EntryMode) -> Result<Option<String>, git2::Error> {
    let selected = Path::new(selected_path);
    if selected.parent().is_some_and(|parent| !parent.as_os_str().is_empty()) {
        return Ok(None);
    }

    let Some(filename) = selected.file_name().and_then(|name| name.to_str()) else {
        return Ok(None);
    };
    let filename = filename.as_bytes();

    for entry in tree.iter() {
        let entry = entry.map_err(|error| git2::Error::from_str(&error.to_string()))?;
        if entry.filename() != filename && entry.oid() == oid && entry.mode() == mode {
            let path = std::str::from_utf8(entry.filename()).map_err(|error| git2::Error::from_str(&error.to_string()))?;
            return Ok(Some(path.to_string()));
        }
    }

    Ok(None)
}

fn raw_tree_changes(repo: &gix::Repository, parent_tree: &gix::Tree<'_>, tree: &gix::Tree<'_>) -> Result<Vec<ChangeDetached>, git2::Error> {
    let mut options = gix::diff::Options::default();
    options.track_path();

    repo.diff_tree_to_tree(Some(parent_tree), Some(tree), Some(options)).map_err(|error| git2::Error::from_str(&error.to_string())).map(|changes| changes.to_vec())
}

fn file_status_from_change(change: &ChangeDetached, path: &str) -> Option<FileStatus> {
    match change {
        ChangeDetached::Addition { location, .. } if path_matches(location, path) => Some(FileStatus::Added),
        ChangeDetached::Deletion { location, .. } if path_matches(location, path) => Some(FileStatus::Deleted),
        ChangeDetached::Modification { location, previous_entry_mode, entry_mode, .. } if path_matches(location, path) => {
            Some(if typechange(previous_entry_mode, entry_mode) { FileStatus::Deleted } else { FileStatus::Modified })
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
