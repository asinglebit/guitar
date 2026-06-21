use super::*;
use crate::git::{
    actions::worktrees::create_worktree,
    queries::diffs::get_filenames_diff_at_workdir,
    repository::open,
    test_support::{TestDir, commit_file, init_repo_at, stage_path, write_workdir_file},
};
use git2::ResetType;
use std::{fs, path::Path};

#[test]
fn reset_to_commit_moves_a_linked_worktree_branch_and_workdir() {
    let dir = TestDir::new("reset-linked-hard");
    let repo = init_repo_at(&dir.join("repo"));
    let base = commit_file(&repo, "file.txt", "base\n", "base");

    let linked_path = dir.join("feature");
    create_worktree(&repo, "feature", &linked_path, base).unwrap();
    let linked_repo = open(&linked_path).unwrap();
    let feature_commit = commit_file(&linked_repo, "file.txt", "feature\n", "feature");

    assert_eq!(repo.find_branch("feature", git2::BranchType::Local).unwrap().get().target(), Some(feature_commit));
    reset_to_commit(&linked_repo, base, ResetType::Hard).unwrap();

    assert_eq!(linked_repo.head().unwrap().target(), Some(base));
    assert_eq!(repo.find_branch("feature", git2::BranchType::Local).unwrap().get().target(), Some(base));
    assert_eq!(fs::read_to_string(linked_path.join("file.txt")).unwrap(), "base\n");
    assert!(get_filenames_diff_at_workdir(&linked_repo).unwrap().is_clean);
}

#[test]
fn mixed_reset_keeps_linked_worktree_changes() {
    let dir = TestDir::new("reset-linked-mixed");
    let repo = init_repo_at(&dir.join("repo"));
    let base = commit_file(&repo, "file.txt", "base\n", "base");

    let linked_path = dir.join("feature");
    create_worktree(&repo, "feature", &linked_path, base).unwrap();
    let linked_repo = open(&linked_path).unwrap();
    commit_file(&linked_repo, "file.txt", "feature\n", "feature");
    write_workdir_file(&linked_repo, "file.txt", "dirty\n");

    reset_to_commit(&linked_repo, base, ResetType::Mixed).unwrap();

    assert_eq!(linked_repo.head().unwrap().target(), Some(base));
    assert_eq!(repo.find_branch("feature", git2::BranchType::Local).unwrap().get().target(), Some(base));
    assert_eq!(fs::read_to_string(linked_path.join("file.txt")).unwrap(), "dirty\n");

    let diff = get_filenames_diff_at_workdir(&linked_repo).unwrap();
    assert!(diff.is_unstaged);
    assert!(diff.unstaged.modified.contains(&"file.txt".to_string()));
}

#[test]
fn reset_file_clears_staged_and_unstaged_changes_in_a_linked_worktree() {
    let dir = TestDir::new("reset-linked-file");
    let repo = init_repo_at(&dir.join("repo"));
    let base = commit_file(&repo, "file.txt", "base\n", "base");

    let linked_path = dir.join("feature");
    create_worktree(&repo, "feature", &linked_path, base).unwrap();
    let linked_repo = open(&linked_path).unwrap();
    commit_file(&linked_repo, "file.txt", "feature\n", "feature");

    write_workdir_file(&linked_repo, "file.txt", "changed\n");
    stage_path(&linked_repo, "file.txt");
    write_workdir_file(&linked_repo, "file.txt", "dirty\n");

    let before = get_filenames_diff_at_workdir(&linked_repo).unwrap();
    assert!(before.is_staged);
    assert!(before.is_unstaged);

    reset_file(&linked_repo, Path::new("file.txt")).unwrap();

    let after = get_filenames_diff_at_workdir(&linked_repo).unwrap();
    assert!(after.is_clean);
    assert_eq!(fs::read_to_string(linked_path.join("file.txt")).unwrap(), "feature\n");
}
