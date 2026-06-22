use crate::{
    git::queries::{
        helpers::{ConflictFile, FileChange, FileStatus, Hunk, UncommittedChanges, deduplicate, diff_to_hunks, walk_tree},
        submodules::submodules_if_present,
    },
    helpers::text::{decode, sanitize},
};
use git2::{Delta, DiffOptions, Error, Oid, Repository, Submodule, SubmoduleIgnore, SubmoduleStatus};
use gix::bstr::{BStr, ByteSlice};
use std::path::{Path, PathBuf};

// Collect staged and unstaged changes separately so the status panes can act on each side.
pub fn get_filenames_diff_at_workdir(repo: &Repository) -> Result<UncommittedChanges, Error> {
    get_filenames_diff_at_workdir_gix(repo)
}

pub fn get_filenames_diff_at_workdir_gix(repo: &Repository) -> Result<UncommittedChanges, Error> {
    let workdir = repo.workdir().ok_or_else(|| Error::from_str("bare repositories are not supported"))?;
    let gix_repo = gix::open(workdir).map_err(gix_error)?;
    let submodules = submodules_if_present(repo).unwrap_or_default();
    let submodule_paths = submodules.iter().map(|entry| entry.path().to_path_buf()).collect::<Vec<_>>();
    let mut changes = UncommittedChanges::default();
    let status = gix_repo
        .status(gix::progress::Discard)
        .map_err(gix_error)?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
        .index_worktree_rewrites(None)
        .index_worktree_submodules(gix::status::Submodule::AsConfigured { check_dirty: true });

    for item in status.into_iter(Vec::new()).map_err(gix_error)? {
        match item.map_err(gix_error)? {
            gix::status::Item::TreeIndex(change) => {
                let path = gix_path(change.location());
                if is_submodule_status_path(&path, &submodule_paths) {
                    continue;
                }
                match change {
                    gix::diff::index::Change::Addition { .. } => push_unique(&mut changes.staged.added, path),
                    gix::diff::index::Change::Deletion { .. } => push_unique(&mut changes.staged.deleted, path),
                    gix::diff::index::Change::Modification { .. } | gix::diff::index::Change::Rewrite { .. } => {
                        push_unique(&mut changes.staged.modified, path);
                    },
                }
            },
            gix::status::Item::IndexWorktree(item) => {
                let path = gix_path(item.rela_path());
                if is_submodule_status_path(&path, &submodule_paths) {
                    continue;
                }
                match item.summary() {
                    Some(gix::status::index_worktree::iter::Summary::Conflict) => push_unique(&mut changes.conflicts, path),
                    Some(gix::status::index_worktree::iter::Summary::Removed) => push_unique(&mut changes.unstaged.deleted, path),
                    Some(gix::status::index_worktree::iter::Summary::Added) | Some(gix::status::index_worktree::iter::Summary::IntentToAdd) => {
                        push_unique(&mut changes.unstaged.added, path);
                    },
                    Some(
                        gix::status::index_worktree::iter::Summary::Modified
                        | gix::status::index_worktree::iter::Summary::TypeChange
                        | gix::status::index_worktree::iter::Summary::Renamed
                        | gix::status::index_worktree::iter::Summary::Copied,
                    ) => push_unique(&mut changes.unstaged.modified, path),
                    None => {},
                }
            },
        }
    }

    add_submodule_pointer_changes(repo, &submodules, &mut changes);
    finalize_uncommitted_changes(&mut changes);

    Ok(changes)
}

fn finalize_uncommitted_changes(changes: &mut UncommittedChanges) {
    changes.modified_count = deduplicate(&changes.staged.modified, &changes.unstaged.modified);
    changes.added_count = deduplicate(&changes.staged.added, &changes.unstaged.added);
    changes.deleted_count = deduplicate(&changes.staged.deleted, &changes.unstaged.deleted);
    changes.conflict_count = changes.conflicts.len();
    changes.has_conflicts = changes.conflict_count > 0;
    changes.is_staged = changes.has_conflicts || !changes.staged.modified.is_empty() || !changes.staged.added.is_empty() || !changes.staged.deleted.is_empty();
    changes.is_unstaged = changes.has_conflicts || !changes.unstaged.modified.is_empty() || !changes.unstaged.added.is_empty() || !changes.unstaged.deleted.is_empty();
    changes.is_clean = !changes.is_staged && !changes.is_unstaged && !changes.has_conflicts;
}

fn add_submodule_pointer_changes(repo: &Repository, submodules: &[Submodule<'_>], changes: &mut UncommittedChanges) {
    let mut index = repo.index().ok();
    if let Some(index) = index.as_mut() {
        let _ = index.read(true);
    }
    let head_tree = repo.head().ok().and_then(|head| head.peel_to_tree().ok());

    for submodule in submodules {
        let path = submodule.path();
        let path_text = path.to_string_lossy().to_string();
        let name = submodule.name().unwrap_or(path_text.as_str());
        let status = submodule_status_for(repo, name, path);
        let index_entry = index.as_ref().and_then(|index| index.get_path(path, 0));
        let head_entry = head_tree.as_ref().and_then(|tree| tree.get_path(path).ok());
        let submodule_head = repo.workdir().and_then(|workdir| Repository::open(workdir.join(path)).ok()).and_then(|submodule_repo| submodule_repo.head().ok().and_then(|head| head.target()));

        if status.is_index_added() || (head_entry.is_none() && index_entry.is_some()) {
            push_unique(&mut changes.staged.added, path_text.clone());
        }
        if status.is_index_deleted() || (head_entry.is_some() && index_entry.is_none()) {
            push_unique(&mut changes.staged.deleted, path_text.clone());
        }
        if status.is_index_modified() || head_entry.zip(index_entry.as_ref()).is_some_and(|(head, index)| head.id() != index.id) {
            push_unique(&mut changes.staged.modified, path_text.clone());
        }

        if status.is_wd_added() {
            push_unique(&mut changes.unstaged.added, path_text.clone());
        } else if status.is_wd_deleted() {
            push_unique(&mut changes.unstaged.deleted, path_text.clone());
        } else if submodule_head.zip(index_entry.as_ref()).is_some_and(|(workdir, index)| workdir != index.id) {
            push_unique(&mut changes.unstaged.modified, path_text.clone());
        }
    }
}

fn submodule_status_for(repo: &Repository, name: &str, path: &Path) -> SubmoduleStatus {
    repo.submodule_status(name, SubmoduleIgnore::None)
        .or_else(|_| path.to_str().map(|path| repo.submodule_status(path, SubmoduleIgnore::None)).unwrap_or_else(|| Err(Error::from_str("invalid submodule path"))))
        .unwrap_or_else(|_| SubmoduleStatus::empty())
}

fn is_submodule_status_path(path: &str, submodule_paths: &[PathBuf]) -> bool {
    if path.is_empty() {
        return false;
    }

    let normalized = path.trim_end_matches('/');
    let path = Path::new(normalized);
    submodule_paths.iter().any(|submodule_path| path == submodule_path || path.starts_with(submodule_path))
}

fn gix_error(error: impl std::fmt::Display) -> Error {
    Error::from_str(&error.to_string())
}

fn gix_path(path: &BStr) -> String {
    path.to_str_lossy().into_owned()
}

fn push_unique(paths: &mut Vec<String>, path: String) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

// List files changed by a commit compared with its first parent.
pub fn get_filenames_diff_at_oid(repo: &Repository, oid: Oid) -> Vec<FileChange> {
    let commit = repo.find_commit(oid).unwrap();
    let tree = commit.tree().unwrap();
    let mut changes = Vec::new();

    // The root commit has no parent, so every tree entry appears as added.
    if commit.parent_count() == 0 {
        walk_tree(repo, &tree, "", &mut changes);
        return changes;
    }

    // Compare against the first parent, matching the normal `git show` view of merges.
    let parent_tree = commit.parent(0).unwrap().tree().unwrap();
    let mut opts = DiffOptions::new();
    opts.include_untracked(false).recurse_untracked_dirs(false).include_typechange(true).ignore_submodules(false).show_binary(false).minimal(false).skip_binary_check(true);

    let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opts)).unwrap();

    for delta in diff.deltas() {
        let path = delta.new_file().path().or_else(|| delta.old_file().path()).unwrap().display().to_string();

        // Tree deltas can represent directories; expand them so the list stays file-oriented.
        let is_folder = !path.contains('.');

        if is_folder && let Ok(tree_obj) = repo.find_tree(delta.new_file().id()) {
            walk_tree(repo, &tree_obj, &path, &mut changes);
            continue;
        }

        changes.push(FileChange {
            filename: path,
            status: match delta.status() {
                Delta::Added => FileStatus::Added,
                Delta::Modified => FileStatus::Modified,
                Delta::Deleted => FileStatus::Deleted,
                Delta::Renamed => FileStatus::Renamed,
                _ => FileStatus::Other,
            },
        });
    }

    // libgit2 does not consistently emit gitlink pointer changes here, so compare known
    // submodule entries directly and upsert the expected row when the recorded commit moved.
    add_committed_submodule_pointer_changes(repo, &parent_tree, &tree, &mut changes);

    changes
}

fn add_committed_submodule_pointer_changes(repo: &Repository, parent_tree: &git2::Tree, tree: &git2::Tree, changes: &mut Vec<FileChange>) {
    let Ok(submodules) = submodules_if_present(repo) else {
        return;
    };

    for submodule in submodules {
        let Some(path) = submodule.path().to_str().map(str::to_string) else {
            continue;
        };

        let parent_entry = parent_tree.get_path(Path::new(&path)).ok();
        let new_entry = tree.get_path(Path::new(&path)).ok();

        let Some(status) = committed_submodule_change_status(parent_entry.as_ref(), new_entry.as_ref()) else {
            continue;
        };

        upsert_change(changes, path, status);
    }
}

fn committed_submodule_change_status(parent_entry: Option<&git2::TreeEntry<'_>>, new_entry: Option<&git2::TreeEntry<'_>>) -> Option<FileStatus> {
    match (parent_entry, new_entry) {
        (Some(old), Some(new)) if old.id() == new.id() => None,
        (Some(_), Some(_)) => Some(FileStatus::Modified),
        (Some(_), None) => Some(FileStatus::Deleted),
        (None, Some(_)) => Some(FileStatus::Added),
        (None, None) => None,
    }
}

fn upsert_change(changes: &mut Vec<FileChange>, filename: String, status: FileStatus) {
    if let Some(existing) = changes.iter_mut().find(|change| change.filename == filename) {
        existing.status = status;
    } else {
        changes.push(FileChange { filename, status });
    }
}

// Build structured hunks for a working tree file against HEAD and the index.
pub fn get_file_diff_at_workdir(repo: &Repository, filename: &str) -> Result<Vec<Hunk>, git2::Error> {
    // HEAD can be absent in a fresh repository, so the diff may be against an empty tree.
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

    // Limit the diff early; libgit2 still reports hunks through the callback below.
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(filename).show_untracked_content(true);

    diff_to_hunks(repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut diff_options))?)
}

// Build structured hunks for one file in a commit against its first parent.
pub fn get_file_diff_at_oid(repo: &Repository, commit_oid: Oid, filename: &str) -> std::result::Result<Vec<Hunk>, git2::Error> {
    let commit = repo.find_commit(commit_oid)?;
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 { Some(commit.parent(0)?.tree()?) } else { None };

    // For root commits, libgit2 treats None as the empty parent side.
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(filename);

    diff_to_hunks(repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_options))?)
}

// Read file contents from a commit, returning sanitized display lines.
pub fn get_file_at_oid(repo: &Repository, commit_oid: Oid, filename: &str) -> Vec<String> {
    let commit = repo.find_commit(commit_oid).unwrap();
    let tree = commit.tree().unwrap();
    tree.get_path(Path::new(filename)).ok().and_then(|entry| repo.find_blob(entry.id()).ok()).map(|blob| sanitize(decode(blob.content())).lines().map(|s| s.to_string()).collect()).unwrap_or_default()
}

// Read file contents from disk, falling back to an empty viewer on IO errors.
pub fn get_file_at_workdir(repo: &Repository, filename: &str) -> Vec<String> {
    let full_path = repo.workdir().map(|root| root.join(filename)).unwrap_or_else(|| Path::new(filename).to_path_buf());
    std::fs::read_to_string(full_path).map(|s| s.lines().map(|l| l.to_string()).collect()).unwrap_or_default()
}

pub fn get_conflict_file(repo: &Repository, filename: &str) -> Result<Option<ConflictFile>, git2::Error> {
    let index = repo.index()?;
    let conflict = match index.conflict_get(Path::new(filename)) {
        Ok(conflict) => conflict,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    Ok(Some(ConflictFile {
        ancestor: conflict.ancestor.as_ref().map(|entry| read_index_entry_lines(repo, entry)).transpose()?.unwrap_or_default(),
        ours: conflict.our.as_ref().map(|entry| read_index_entry_lines(repo, entry)).transpose()?.unwrap_or_default(),
        theirs: conflict.their.as_ref().map(|entry| read_index_entry_lines(repo, entry)).transpose()?.unwrap_or_default(),
        workdir: get_file_at_workdir(repo, filename),
    }))
}

fn read_index_entry_lines(repo: &Repository, entry: &git2::IndexEntry) -> Result<Vec<String>, git2::Error> {
    let blob = repo.find_blob(entry.id)?;
    Ok(sanitize(decode(blob.content())).lines().map(|s| s.to_string()).collect())
}

#[cfg(test)]
#[path = "../../tests/git/queries/diffs.rs"]
mod tests;
