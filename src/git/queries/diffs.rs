use crate::{
    git::queries::helpers::{FileChange, FileStatus, Hunk, UncommittedChanges, deduplicate, diff_to_hunks, walk_tree},
    helpers::text::{decode, sanitize},
};
use git2::{Delta, DiffOptions, Error, Oid, Repository, StatusOptions};
use std::path::Path;

// Collect staged and unstaged changes separately so the status panes can act on each side.
pub fn get_filenames_diff_at_workdir(repo: &Repository) -> Result<UncommittedChanges, Error> {
    let mut options = StatusOptions::new();
    options.include_untracked(true).show(git2::StatusShow::IndexAndWorkdir).renames_head_to_index(false).renames_index_to_workdir(false);

    let statuses = repo.statuses(Some(&mut options))?;
    let mut changes = UncommittedChanges::default();
    let workdir = repo.workdir().expect("Bare repo not supported");

    for entry in statuses.iter() {
        let rel_path = entry.path().unwrap_or("");
        let full_path = workdir.join(rel_path);

        // Directory statuses are expanded so the UI can show actionable file rows.
        let files = if full_path.is_dir() { collect_files_for_status(repo, workdir, rel_path) } else { vec![rel_path.to_string()] };

        for file in files {
            // Query each file after expansion to avoid applying directory status to children.
            let file_status = repo.status_file(Path::new(&file))?;

            if file_status.is_conflicted() {
                push_unique(&mut changes.conflicts, file.clone());
                continue;
            }

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

fn conflict_path(entry: &git2::IndexEntry) -> Option<String> {
    std::str::from_utf8(&entry.path).ok().map(|path| path.to_string())
}

fn push_unique(paths: &mut Vec<String>, path: String) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
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
    opts.include_untracked(false).recurse_untracked_dirs(false).include_typechange(false).ignore_submodules(true).show_binary(false).minimal(false).skip_binary_check(true);

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
    diff_options.pathspec(filename);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::actions::rebasing::{RebaseOutcome, start_rebase};
    use git2::{Repository, Signature, build::CheckoutBuilder};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_repo(name: &str) -> (PathBuf, Repository) {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-diff-{name}-{id}"));
        fs::create_dir_all(&path).unwrap();
        let repo = Repository::init(&path).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        (path, repo)
    }

    fn write(path: &Path, file: &str, content: &str) {
        fs::write(path.join(file), content).unwrap();
    }

    fn commit(repo: &Repository, file: &str, message: &str) -> Oid {
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
    }

    fn checkout_new_branch(repo: &Repository, name: &str) {
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch(name, &head, false).unwrap();
        repo.set_head(&format!("refs/heads/{name}")).unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
    }

    fn checkout_branch(repo: &Repository, name: &str) {
        repo.set_head(&format!("refs/heads/{name}")).unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
    }

    #[test]
    fn workdir_diff_marks_conflicted_paths() {
        let (path, repo) = temp_repo("conflict");
        write(&path, "file.txt", "base\n");
        commit(&repo, "file.txt", "base");
        checkout_new_branch(&repo, "feature");
        write(&path, "file.txt", "feature\n");
        commit(&repo, "file.txt", "feature");
        checkout_branch(&repo, "master");
        write(&path, "file.txt", "main\n");
        let main = commit(&repo, "file.txt", "main");
        checkout_branch(&repo, "feature");

        assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);

        let changes = get_filenames_diff_at_workdir(&repo).unwrap();
        assert!(changes.has_conflicts);
        assert!(changes.is_staged);
        assert!(changes.is_unstaged);
        assert_eq!(changes.conflict_count, 1);
        assert_eq!(changes.conflicts, vec!["file.txt".to_string()]);

        let _ = fs::remove_dir_all(path);
    }
}
