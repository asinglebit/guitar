use super::*;
use crate::git::{
    queries::diffs::get_filenames_diff_at_workdir,
    test_support::{TestDir, commit_file, linked_worktree_fixture, stage_path, write_workdir_file},
};
use git2::ResetType;
use std::{fs, path::Path};

#[test]
fn reset_to_commit_updates_linked_worktree_branch_and_workdir() {
    let dir = TestDir::new("reset-linked-hard");
    let fixture = linked_worktree_fixture(&dir, "feature");
    commit_file(&fixture.linked_repo, "file.txt", "feature\n", "feature");

    reset_to_commit(&fixture.linked_repo, fixture.base, ResetType::Hard).unwrap();

    assert_eq!(fixture.linked_repo.head().unwrap().target(), Some(fixture.base));
    assert_eq!(fixture.repo.find_branch("feature", git2::BranchType::Local).unwrap().get().target(), Some(fixture.base));
    assert_eq!(fs::read_to_string(fixture.linked_path.join("file.txt")).unwrap(), "base\n");
    assert!(get_filenames_diff_at_workdir(&fixture.linked_repo).unwrap().is_clean);

    let dir = TestDir::new("reset-linked-mixed");
    let fixture = linked_worktree_fixture(&dir, "feature");
    commit_file(&fixture.linked_repo, "file.txt", "feature\n", "feature");
    write_workdir_file(&fixture.linked_repo, "file.txt", "dirty\n");

    reset_to_commit(&fixture.linked_repo, fixture.base, ResetType::Mixed).unwrap();

    assert_eq!(fixture.linked_repo.head().unwrap().target(), Some(fixture.base));
    assert_eq!(fixture.repo.find_branch("feature", git2::BranchType::Local).unwrap().get().target(), Some(fixture.base));
    assert_eq!(fs::read_to_string(fixture.linked_path.join("file.txt")).unwrap(), "dirty\n");

    let diff = get_filenames_diff_at_workdir(&fixture.linked_repo).unwrap();
    assert!(diff.is_unstaged);
    assert!(diff.unstaged.modified.contains(&"file.txt".to_string()));
}

#[test]
fn reset_file_clears_staged_and_unstaged_changes_in_a_linked_worktree() {
    let dir = TestDir::new("reset-linked-file");
    let fixture = linked_worktree_fixture(&dir, "feature");
    commit_file(&fixture.linked_repo, "file.txt", "feature\n", "feature");

    write_workdir_file(&fixture.linked_repo, "file.txt", "changed\n");
    stage_path(&fixture.linked_repo, "file.txt");
    write_workdir_file(&fixture.linked_repo, "file.txt", "dirty\n");

    let before = get_filenames_diff_at_workdir(&fixture.linked_repo).unwrap();
    assert!(before.is_staged);
    assert!(before.is_unstaged);

    reset_file(&fixture.linked_repo, Path::new("file.txt")).unwrap();

    let after = get_filenames_diff_at_workdir(&fixture.linked_repo).unwrap();
    assert!(after.is_clean);
    assert_eq!(fs::read_to_string(fixture.linked_path.join("file.txt")).unwrap(), "feature\n");
}
