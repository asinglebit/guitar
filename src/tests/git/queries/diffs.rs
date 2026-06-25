use super::*;
use crate::git::{
    actions::{
        rebasing::{RebaseOutcome, start_rebase},
        submodules::{stage_submodule_head, unstage_submodule},
    },
    test_support::{
        TestDir, checkout_branch, commit_file, commit_index, commit_staged_file as commit, diverge_file, parent_with_submodule, stage_path as stage, temp_repo, write_path_file as write,
        write_workdir_file,
    },
};
use git2::Repository;
use std::{fs, path::PathBuf};

fn assert_no_file_status_rows(changes: &UncommittedChanges) {
    assert!(changes.staged.modified.is_empty());
    assert!(changes.staged.added.is_empty());
    assert!(changes.staged.deleted.is_empty());
    assert!(changes.unstaged.modified.is_empty());
    assert!(changes.unstaged.added.is_empty());
    assert!(changes.unstaged.deleted.is_empty());
    assert!(changes.conflicts.is_empty());
    assert!(changes.is_clean);
}

fn assert_contains_path(paths: &[String], expected: &str) {
    assert!(paths.iter().any(|path| path == expected), "expected {expected} in {paths:?}");
}

#[test]
fn workdir_diff_marks_conflicted_paths() {
    let (_dir, repo) = temp_repo("conflict");
    let (_, main) = diverge_file(&repo, "file.txt");
    checkout_branch(&repo, "feature");

    assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);

    let changes = get_filenames_diff_at_workdir(&repo).unwrap();
    assert!(changes.has_conflicts);
    assert!(changes.is_staged);
    assert!(changes.is_unstaged);
    assert_eq!(changes.conflict_count, 1);
    assert_eq!(changes.conflicts, vec!["file.txt".to_string()]);

    let conflict = get_conflict_file(&repo, "file.txt").unwrap().unwrap();
    assert!(!conflict.ours.is_empty());
    assert!(!conflict.theirs.is_empty());
    assert!(conflict.workdir.iter().any(|line| line.starts_with("<<<<<<<")));
    assert!(conflict.workdir.iter().any(|line| line.starts_with("=======")));
    assert!(conflict.workdir.iter().any(|line| line.starts_with(">>>>>>>")));
}

#[test]
fn workdir_file_diff_emits_untracked_file_contents_as_added_lines() {
    let (_dir, repo) = temp_repo("untracked-added-lines");
    commit_file(&repo, "tracked.txt", "base\n", "initial");
    write_workdir_file(&repo, "new.txt", "alpha\nbeta\n");

    let hunks = get_file_diff_at_workdir(&repo, "new.txt").unwrap();
    let content_lines = hunks.iter().flat_map(|hunk| hunk.lines.iter()).filter(|line| line.origin != 'H').collect::<Vec<_>>();

    assert!(!content_lines.is_empty());
    assert_eq!(content_lines.iter().map(|line| line.origin).collect::<Vec<_>>(), vec!['+', '+']);
    assert_eq!(content_lines.iter().map(|line| line.content.as_str()).collect::<Vec<_>>(), vec!["alpha\n", "beta\n"]);
}

fn status_matrix_repo(name: &str) -> (TestDir, PathBuf, Repository) {
    let (dir, repo) = temp_repo(name);
    let path = repo.workdir().unwrap().to_path_buf();
    commit_file(&repo, "staged.txt", "base\n", "staged base");
    commit_file(&repo, "unstaged.txt", "base\n", "unstaged base");
    commit_file(&repo, "deleted.txt", "base\n", "deleted base");

    write_workdir_file(&repo, "staged.txt", "staged\n");
    stage(&repo, "staged.txt");
    write_workdir_file(&repo, "unstaged.txt", "unstaged\n");
    fs::remove_file(path.join("deleted.txt")).unwrap();
    write_workdir_file(&repo, "new.txt", "new\n");

    (dir, path, repo)
}

#[test]
fn workdir_and_staged_diffs_share_the_status_matrix_without_requerying_paths() {
    let (_dir, path, repo) = status_matrix_repo("ordinary-statuses");

    let workdir = get_filenames_diff_at_workdir(&repo).unwrap();
    let staged = get_staged_filenames_diff(&repo).unwrap();
    let staged_from_path = get_staged_filenames_diff_from_path(&path).unwrap();

    assert_contains_path(&workdir.staged.modified, "staged.txt");
    assert_contains_path(&workdir.unstaged.modified, "unstaged.txt");
    assert_contains_path(&workdir.unstaged.deleted, "deleted.txt");
    assert_contains_path(&workdir.unstaged.added, "new.txt");
    assert_eq!(workdir.modified_count, 2);
    assert_eq!(workdir.added_count, 1);
    assert_eq!(workdir.deleted_count, 1);
    assert!(workdir.is_staged);
    assert!(workdir.is_unstaged);

    for changes in [&staged, &staged_from_path] {
        assert_contains_path(&changes.staged.modified, "staged.txt");
        assert!(changes.unstaged.modified.is_empty());
        assert!(changes.unstaged.deleted.is_empty());
        assert!(changes.unstaged.added.is_empty());
        assert_eq!(changes.modified_count, 1);
        assert_eq!(changes.added_count, 0);
        assert_eq!(changes.deleted_count, 0);
        assert!(changes.is_staged);
        assert!(!changes.is_unstaged);
    }
}

#[test]
fn workdir_diff_expands_untracked_directories_without_ignored_files() {
    let (_dir, repo) = temp_repo("untracked-directory-ignore");
    let path = repo.workdir().unwrap();
    write(&path, ".gitignore", "*.ignored\n");
    commit(&repo, ".gitignore", "ignore generated files");
    write(&path, "scratch/one.txt", "one\n");
    write(&path, "scratch/nested/two.txt", "two\n");
    write(&path, "scratch/nested/skip.ignored", "ignored\n");

    let changes = get_filenames_diff_at_workdir(&repo).unwrap();

    assert_contains_path(&changes.unstaged.added, "scratch/one.txt");
    assert_contains_path(&changes.unstaged.added, "scratch/nested/two.txt");
    assert!(!changes.unstaged.added.iter().any(|path| path == "scratch"));
    assert!(!changes.unstaged.added.iter().any(|path| path.ends_with("skip.ignored")));
}

#[test]
fn workdir_diff_ignores_clean_initialized_submodule() {
    let dir = TestDir::new("submodule-clean");
    let (parent, _) = parent_with_submodule(&dir);

    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert_no_file_status_rows(&changes);
}

#[test]
fn workdir_diff_ignores_dirty_tracked_submodule_content() {
    let dir = TestDir::new("submodule-dirty");
    let (parent, _) = parent_with_submodule(&dir);
    fs::write(parent.workdir().unwrap().join("deps/child/file.txt"), "dirty\n").unwrap();

    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert_no_file_status_rows(&changes);
}

#[test]
fn workdir_diff_ignores_untracked_submodule_content() {
    let dir = TestDir::new("submodule-untracked");
    let (parent, _) = parent_with_submodule(&dir);
    fs::write(parent.workdir().unwrap().join("deps/child/extra.txt"), "extra\n").unwrap();

    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert_no_file_status_rows(&changes);
}

#[test]
fn workdir_diff_ignores_uninitialized_submodule() {
    let dir = TestDir::new("submodule-uninitialized");
    let (parent, _) = parent_with_submodule(&dir);
    let clone_path = dir.path().join("clone");
    let clone = Repository::clone(parent.workdir().unwrap().to_str().unwrap(), &clone_path).unwrap();

    let changes = get_filenames_diff_at_workdir(&clone).unwrap();

    assert_no_file_status_rows(&changes);
}

#[test]
fn workdir_diff_lists_changed_submodule_pointer_as_unstaged_modified() {
    let dir = TestDir::new("submodule-pointer");
    let (parent, _) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    write(sub_repo.workdir().unwrap(), "file.txt", "advanced\n");
    commit(&sub_repo, "file.txt", "advance child");

    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert!(changes.staged.modified.is_empty());
    assert_eq!(changes.unstaged.modified, vec!["deps/child".to_string()]);
    assert_eq!(changes.modified_count, 1);
    assert!(changes.is_unstaged);
    assert!(!changes.is_staged);
    assert!(!changes.is_clean);
}

#[test]
fn workdir_diff_lists_staged_submodule_pointer_as_staged_modified() {
    let dir = TestDir::new("submodule-pointer-staged");
    let (parent, _) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    write(sub_repo.workdir().unwrap(), "file.txt", "advanced\n");
    commit(&sub_repo, "file.txt", "advance child");

    stage_submodule_head(&parent, "deps/child").unwrap();
    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert_eq!(changes.staged.modified, vec!["deps/child".to_string()]);
    assert!(changes.unstaged.modified.is_empty());
    assert_eq!(changes.modified_count, 1);
    assert!(changes.is_staged);
    assert!(!changes.is_unstaged);
    assert!(!changes.is_clean);

    unstage_submodule(&parent, "deps/child").unwrap();
    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert!(changes.staged.modified.is_empty());
    assert_eq!(changes.unstaged.modified, vec!["deps/child".to_string()]);
}

#[test]
fn commit_diff_lists_committed_submodule_pointer_change() {
    let dir = TestDir::new("submodule-pointer-commit");
    let (parent, _) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    write(sub_repo.workdir().unwrap(), "file.txt", "advanced\n");
    commit(&sub_repo, "file.txt", "advance child");
    stage_submodule_head(&parent, "deps/child").unwrap();

    let commit_oid = commit_index(&parent, "update submodule pointer");
    let changes = get_filenames_diff_at_oid(&parent, commit_oid);

    assert!(changes.iter().any(|change| change.filename == "deps/child" && change.status == FileStatus::Modified), "{changes:?}");
}

#[test]
fn submodule_status_path_guard_matches_exact_paths_and_children() {
    let submodule_paths = vec![PathBuf::from("deps/child")];

    assert!(is_submodule_status_path("deps/child", &submodule_paths));
    assert!(is_submodule_status_path("deps/child/", &submodule_paths));
    assert!(is_submodule_status_path("deps/child/file.txt", &submodule_paths));
    assert!(!is_submodule_status_path("deps/childish", &submodule_paths));
    assert!(!is_submodule_status_path("deps", &submodule_paths));
}
