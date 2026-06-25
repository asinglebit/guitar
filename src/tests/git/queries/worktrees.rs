use super::*;
use crate::{
    core::{oids::git2_to_gix_oid, worktrees::WorktreeEntry},
    git::{
        actions::worktrees::{create_worktree, lock_worktree, remove_worktree, unlock_worktree},
        queries::commits::get_current_branch,
        queries::helpers::UncommittedChanges,
        test_support::{TestDir, commit_file, init_repo_at, linked_worktree_fixture, stage_path, write_workdir_file},
    },
};
use std::fs;

fn entry<'a>(entries: &'a [WorktreeEntry], name: &str) -> &'a WorktreeEntry {
    entries.iter().find(|entry| entry.name == name).unwrap()
}

fn main_entry(entries: &[WorktreeEntry]) -> &WorktreeEntry {
    entries.iter().find(|entry| entry.is_main()).unwrap()
}

#[test]
fn lists_main_and_linked_worktrees_with_stable_metadata() {
    let dir = TestDir::new("worktree-list");
    let repo_path = dir.join("repo");
    let repo = init_repo_at(&repo_path);
    let oid = commit_file(&repo, "file.txt", "hello\n", "initial");

    create_worktree(&repo, "zeta", &dir.join("repo-zeta"), oid).unwrap();
    create_worktree(&repo, "alpha", &dir.join("repo-alpha"), oid).unwrap();

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].name, "repo");
    assert!(entries[0].is_main());
    assert!(entries[0].is_current);
    assert_eq!(entries[0].branch.as_deref(), get_current_branch(&repo).as_deref());
    assert_eq!(entries[0].head, Some(git2_to_gix_oid(oid)));

    let linked_names: Vec<_> = entries.iter().skip(1).map(|entry| entry.name.as_str()).collect();
    assert_eq!(linked_names, vec!["alpha", "zeta"]);

    for entry in entries.iter().skip(1) {
        assert!(entry.is_linked());
        assert!(entry.is_valid);
        assert!(!entry.is_current);
        assert_eq!(entry.branch.as_deref(), Some(entry.name.as_str()));
        assert_eq!(entry.head, Some(git2_to_gix_oid(oid)));
        assert!(!entry.is_dirty);
        assert!(entry.locked_reason.is_none());
        assert!(!entry.is_prunable);
    }
}

#[test]
fn linked_worktree_current_dirty_and_metadata_policy() {
    let dir = TestDir::new("worktree-current");
    let fixture = linked_worktree_fixture(&dir, "feature");

    let entries = list_worktrees(&fixture.linked_repo, Some(&fixture.linked_path)).unwrap();
    let linked = entry(&entries, "feature");
    let main = main_entry(&entries);

    assert!(linked.is_current);
    assert_eq!(linked.branch.as_deref(), Some("feature"));
    assert_eq!(linked.head, Some(git2_to_gix_oid(fixture.base)));
    assert!(main.is_main());
    assert!(!main.is_current);
    assert_eq!(main.head, Some(git2_to_gix_oid(fixture.base)));

    let uncommitted = UncommittedChanges { is_clean: false, ..Default::default() };
    let metadata_entries = list_worktrees_metadata_with_current_dirty(&fixture.linked_repo, Some(&fixture.linked_path), &uncommitted).unwrap();
    let path_entries = list_worktrees_metadata_with_current_dirty_from_path(&fixture.linked_path, Some(&fixture.linked_path), &uncommitted).unwrap();
    let metadata_linked = entry(&metadata_entries, "feature");
    let metadata_main = main_entry(&metadata_entries);
    let path_linked = entry(&path_entries, "feature");

    assert!(metadata_linked.is_current);
    assert!(metadata_linked.is_dirty);
    assert!(path_linked.is_current);
    assert!(path_linked.is_dirty);
    assert!(!metadata_main.is_current);
    assert!(!metadata_main.is_dirty);

    write_workdir_file(&fixture.linked_repo, "staged.txt", "staged\n");
    stage_path(&fixture.linked_repo, "staged.txt");
    let entries = list_worktrees(&fixture.repo, Some(&fixture.repo_path)).unwrap();
    let linked = entry(&entries, "feature");
    assert!(linked.is_dirty);
}

#[test]
fn main_worktree_metadata_skips_dirty_scan_but_full_listing_marks_dirty() {
    let dir = TestDir::new("worktree-dirty");
    let repo_path = dir.join("repo");
    let repo = init_repo_at(&repo_path);
    let oid = commit_file(&repo, "file.txt", "hello\n", "initial");

    fs::write(repo_path.join("untracked.txt"), "extra\n").unwrap();
    let entries = list_worktrees_metadata(&repo, Some(&repo_path)).unwrap();
    let main = main_entry(&entries);

    assert!(main.is_current);
    assert_eq!(main.head, Some(git2_to_gix_oid(oid)));
    assert_eq!(main.branch.as_deref(), get_current_branch(&repo).as_deref());
    assert!(!main.is_dirty);

    write_workdir_file(&repo, "staged.txt", "staged\n");
    stage_path(&repo, "staged.txt");

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let main = main_entry(&entries);

    assert!(main.is_current);
    assert!(main.is_dirty);
}

#[test]
fn reports_lock_reason_and_prunability_for_stale_worktrees() {
    let dir = TestDir::new("worktree-stale");
    let repo_path = dir.join("repo");
    let repo = init_repo_at(&repo_path);
    let oid = commit_file(&repo, "file.txt", "hello\n", "initial");

    create_worktree(&repo, "feature", &dir.join("repo-feature"), oid).unwrap();
    lock_worktree(&repo, "feature", Some("keep it")).unwrap();

    let locked_entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let locked = entry(&locked_entries, "feature");
    assert_eq!(locked.locked_reason.as_deref(), Some("keep it"));
    assert!(!locked.can_remove());

    unlock_worktree(&repo, "feature").unwrap();
    remove_worktree(&repo, "feature").unwrap();
    assert!(repo.find_worktree("feature").is_err());

    let stale_path = dir.join("repo-stale");
    create_worktree(&repo, "stale", &stale_path, oid).unwrap();
    fs::remove_dir_all(&stale_path).unwrap();

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let stale = entry(&entries, "stale");
    assert!(!stale.is_valid);
    assert!(stale.is_prunable);
}
