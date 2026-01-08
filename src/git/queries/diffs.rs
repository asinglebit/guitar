use crate::{
    git::queries::helpers::{
        FileChange, FileStatus, Hunk, UncommittedChanges, deduplicate, diff_to_hunks, walk_tree,
    },
    helpers::text::{decode, sanitize},
};
use git2::{Delta, DiffOptions, Error, Oid, Repository, StatusOptions};
use std::path::Path;

// Collects and categorizes uncommitted changes in the working directory and index
pub fn get_filenames_diff_at_workdir(repo: &Repository) -> Result<UncommittedChanges, Error> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .show(git2::StatusShow::IndexAndWorkdir)
        .renames_head_to_index(false)
        .renames_index_to_workdir(false);

    let statuses = repo.statuses(Some(&mut options))?;
    let mut changes = UncommittedChanges::default();
    let workdir = repo.workdir().expect("Bare repo not supported");

    for entry in statuses.iter() {
        let rel_path = entry.path().unwrap_or("");
        let full_path = workdir.join(rel_path);

        // Expand directories
        let files = if full_path.is_dir() {
            collect_files_for_status(repo, workdir, rel_path)
        } else {
            vec![rel_path.to_string()]
        };

        for file in files {
            // Ask git for this file’s individual status
            let file_status = repo.status_file(Path::new(&file))?;

            // Now you can safely check staged vs unstaged per file
            if file_status.is_index_modified() {
                changes.staged.modified.push(file.clone());
            }
            if file_status.is_index_new() {
                changes.staged.added.push(file.clone());
            }
            if file_status.is_index_deleted() {
                changes.staged.deleted.push(file.clone());
            }

            if file_status.is_wt_modified() {
                changes.unstaged.modified.push(file.clone());
            }
            if file_status.is_wt_new() {
                changes.unstaged.added.push(file.clone());
            }
            if file_status.is_wt_deleted() {
                changes.unstaged.deleted.push(file.clone());
            }
        }
    }

    // Counts
    changes.modified_count = deduplicate(&changes.staged.modified, &changes.unstaged.modified);
    changes.added_count = deduplicate(&changes.staged.added, &changes.unstaged.added);
    changes.deleted_count = deduplicate(&changes.staged.deleted, &changes.unstaged.deleted);

    changes.is_staged = !changes.staged.modified.is_empty()
        || !changes.staged.added.is_empty()
        || !changes.staged.deleted.is_empty();

    changes.is_unstaged = !changes.unstaged.modified.is_empty()
        || !changes.unstaged.added.is_empty()
        || !changes.unstaged.deleted.is_empty();

    changes.is_clean = !changes.is_staged && !changes.is_unstaged;

    Ok(changes)
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

                    // Skip ignored files
                    if repo
                        .status_should_ignore(Path::new(&child_rel))
                        .unwrap_or(false)
                    {
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

    // If path does not exist (deleted file), just return the rel_path itself
    vec![rel_path.to_string()]
}

// Lists all files changed in a given commit compared to its parent
pub fn get_filenames_diff_at_oid(repo: &Repository, oid: Oid) -> Vec<FileChange> {
    let commit = repo.find_commit(oid).unwrap();
    let tree = commit.tree().unwrap();
    let mut changes = Vec::new();

    // Handle the initial commit (no parent)
    if commit.parent_count() == 0 {
        walk_tree(repo, &tree, "", &mut changes);
        return changes;
    }

    // Diff current commit tree against its parent tree
    let parent_tree = commit.parent(0).unwrap().tree().unwrap();
    let mut opts = DiffOptions::new();
    opts.include_untracked(false)
        .recurse_untracked_dirs(false)
        .include_typechange(false)
        .ignore_submodules(true)
        .show_binary(false)
        .minimal(false)
        .skip_binary_check(true);

    let diff = repo
        .diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opts))
        .unwrap();

    // Iterate through all deltas (changed files)
    for delta in diff.deltas() {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .unwrap()
            .display()
            .to_string();

        // Rough check for folders (no '.' in name)
        let is_folder = !path.contains('.');

        // Recursively collect folder contents if applicable
        if is_folder && let Ok(tree_obj) = repo.find_tree(delta.new_file().id()) {
            walk_tree(repo, &tree_obj, &path, &mut changes);
            continue;
        }

        // Record file and its change status
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

// Generate a line-by-line diff for a file in the working directory
pub fn get_file_diff_at_workdir(
    repo: &Repository,
    filename: &str,
) -> Result<Vec<Hunk>, git2::Error> {
    // Get the current HEAD tree (if available)
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

    // Set diff options to include only the target file
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(filename);

    // Compare HEAD tree with workdir + index
    diff_to_hunks(
        repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut diff_options))?,
    )
}

// Generate a line-by-line diff for a file between a commit and its parent
pub fn get_file_diff_at_oid(
    repo: &Repository,
    commit_oid: Oid,
    filename: &str,
) -> std::result::Result<Vec<Hunk>, git2::Error> {
    let commit = repo.find_commit(commit_oid)?;
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    // Diff options limited to the specific file
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(filename);

    // Compare parent tree with current commit tree
    diff_to_hunks(repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&tree),
        Some(&mut diff_options),
    )?)
}

// Retrieve the contents of a file at a specific commit
pub fn get_file_at_oid(repo: &Repository, commit_oid: Oid, filename: &str) -> Vec<String> {
    let commit = repo.find_commit(commit_oid).unwrap();
    let tree = commit.tree().unwrap();
    tree.get_path(Path::new(filename))
        .ok()
        .and_then(|entry| repo.find_blob(entry.id()).ok())
        .map(|blob| {
            sanitize(decode(blob.content()))
                .lines()
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

// Retrieve the contents of a file from the working directory
pub fn get_file_at_workdir(repo: &Repository, filename: &str) -> Vec<String> {
    let full_path = repo
        .workdir()
        .map(|root| root.join(filename))
        .unwrap_or_else(|| Path::new(filename).to_path_buf());
    std::fs::read_to_string(full_path)
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default()
}
