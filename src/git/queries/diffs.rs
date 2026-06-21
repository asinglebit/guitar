use crate::{
    git::queries::{
        helpers::{ConflictFile, FileChange, FileStatus, Hunk, UncommittedChanges, deduplicate, diff_to_hunks, walk_tree},
        submodules::submodules_if_present,
    },
    helpers::text::{decode, sanitize},
};
use git2::{Delta, DiffOptions, Error, Oid, Repository, Status, StatusOptions, Submodule, SubmoduleIgnore, SubmoduleStatus};
use std::path::{Path, PathBuf};

// Collect staged and unstaged changes separately so the status panes can act on each side.
pub fn get_filenames_diff_at_workdir(repo: &Repository) -> Result<UncommittedChanges, Error> {
    let mut options = StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true).exclude_submodules(true).show(git2::StatusShow::IndexAndWorkdir).renames_head_to_index(false).renames_index_to_workdir(false);

    let statuses = repo.statuses(Some(&mut options))?;
    let mut changes = UncommittedChanges::default();
    let workdir = repo.workdir().expect("Bare repo not supported");
    let submodules = submodules_if_present(repo).unwrap_or_default();
    let submodule_paths = submodules.iter().map(|entry| entry.path().to_path_buf()).collect::<Vec<_>>();

    for entry in statuses.iter() {
        let rel_path = entry.path().unwrap_or("");
        if is_submodule_status_path(rel_path, &submodule_paths) {
            continue;
        }

        let full_path = workdir.join(rel_path);

        if full_path.is_dir() {
            // Directory statuses are expanded so the UI can show actionable file rows.
            for file in collect_files_for_status(repo, workdir, rel_path) {
                if is_submodule_status_path(&file, &submodule_paths) {
                    continue;
                }

                // Query expanded children because the directory status is not specific enough.
                push_status_changes(&mut changes, file.clone(), repo.status_file(Path::new(&file))?);
            }
        } else {
            push_status_changes(&mut changes, rel_path.to_string(), entry.status());
        }
    }

    if let Ok(index) = repo.index()
        && let Ok(conflicts) = index.conflicts()
    {
        for conflict in conflicts.flatten() {
            let path = conflict.our.as_ref().and_then(conflict_path).or_else(|| conflict.their.as_ref().and_then(conflict_path)).or_else(|| conflict.ancestor.as_ref().and_then(conflict_path));
            if let Some(path) = path {
                push_unique(&mut changes.conflicts, path);
            }
        }
    }

    add_submodule_pointer_changes(repo, &submodules, &mut changes);

    // Counts are deduplicated because the same path can be both staged and unstaged.
    changes.modified_count = deduplicate(&changes.staged.modified, &changes.unstaged.modified);
    changes.added_count = deduplicate(&changes.staged.added, &changes.unstaged.added);
    changes.deleted_count = deduplicate(&changes.staged.deleted, &changes.unstaged.deleted);
    changes.conflict_count = changes.conflicts.len();
    changes.has_conflicts = changes.conflict_count > 0;
    changes.is_staged = changes.has_conflicts || !changes.staged.modified.is_empty() || !changes.staged.added.is_empty() || !changes.staged.deleted.is_empty();
    changes.is_unstaged = changes.has_conflicts || !changes.unstaged.modified.is_empty() || !changes.unstaged.added.is_empty() || !changes.unstaged.deleted.is_empty();
    changes.is_clean = !changes.is_staged && !changes.is_unstaged && !changes.has_conflicts;

    Ok(changes)
}

fn add_submodule_pointer_changes(repo: &Repository, submodules: &[Submodule<'_>], changes: &mut UncommittedChanges) {
    for submodule in submodules {
        let path = submodule.path();
        let path_text = path.to_string_lossy().to_string();
        let name = submodule.name().unwrap_or(path_text.as_str());
        let status = submodule_status_for(repo, name, path);

        if status.is_index_added() {
            push_unique(&mut changes.staged.added, path_text.clone());
        }
        if status.is_index_deleted() {
            push_unique(&mut changes.staged.deleted, path_text.clone());
        }
        if status.is_index_modified() {
            push_unique(&mut changes.staged.modified, path_text.clone());
        }

        if status.is_wd_added() {
            push_unique(&mut changes.unstaged.added, path_text.clone());
        } else if status.is_wd_deleted() {
            push_unique(&mut changes.unstaged.deleted, path_text.clone());
        } else if status.is_wd_modified() || submodule.workdir_id().zip(submodule.index_id()).is_some_and(|(workdir, index)| workdir != index) {
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

fn conflict_path(entry: &git2::IndexEntry) -> Option<String> {
    std::str::from_utf8(&entry.path).ok().map(|path| path.to_string())
}

fn push_unique(paths: &mut Vec<String>, path: String) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn push_status_changes(changes: &mut UncommittedChanges, file: String, status: Status) {
    if status.is_conflicted() {
        push_unique(&mut changes.conflicts, file);
        return;
    }

    if status.is_index_modified() {
        changes.staged.modified.push(file.clone());
    }
    if status.is_index_new() {
        changes.staged.added.push(file.clone());
    }
    if status.is_index_deleted() {
        changes.staged.deleted.push(file.clone());
    }

    if status.is_wt_modified() {
        changes.unstaged.modified.push(file.clone());
    }
    if status.is_wt_new() {
        changes.unstaged.added.push(file.clone());
    }
    if status.is_wt_deleted() {
        changes.unstaged.deleted.push(file);
    }
}

fn collect_files_for_status(repo: &Repository, workdir: &Path, rel_path: &str) -> Vec<String> {
    let full_path = workdir.join(rel_path);

    if full_path.exists() {
        if full_path.is_file() {
            return vec![rel_path.to_string()];
        } else if full_path.is_dir() {
            let mut result = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&full_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let child_rel = match path.strip_prefix(workdir) {
                        Ok(p) => p.to_string_lossy().to_string(),
                        Err(_) => continue,
                    };

                    // Respect gitignore while recursively expanding untracked directories.
                    if repo.status_should_ignore(Path::new(&child_rel)).unwrap_or(false) {
                        continue;
                    }

                    if path.is_file() {
                        result.push(child_rel);
                    } else if path.is_dir() {
                        result.extend(collect_files_for_status(repo, workdir, &child_rel));
                    }
                }
            }
            return result;
        }
    }

    // Deleted paths no longer exist on disk, but git still reports them by relative path.
    vec![rel_path.to_string()]
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
    opts.include_untracked(false).recurse_untracked_dirs(false).include_typechange(false).ignore_submodules(false).show_binary(false).minimal(false).skip_binary_check(true);

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

    changes
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
