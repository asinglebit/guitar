use super::*;
use crate::git::{
    queries::commits::get_current_branch,
    test_support::{TestDir, commit_file, create_branch, linked_worktree_fixture},
};
use git2::BranchType;
use im::HashSet;
use std::{collections::HashMap, fs};

#[test]
fn checkout_head_detaches_then_checkout_branch_switches_linked_worktree() {
    let dir = TestDir::new("checkout-linked-head");
    let fixture = linked_worktree_fixture(&dir, "feature");
    create_branch(&fixture.repo, "release", fixture.base);
    let feature_commit = commit_file(&fixture.linked_repo, "file.txt", "feature\n", "feature");

    checkout_head(&fixture.linked_repo, fixture.base).unwrap();

    assert_eq!(get_current_branch(&fixture.linked_repo), None);
    assert!(!fixture.linked_repo.head().unwrap().is_branch());
    assert_eq!(fs::read_to_string(fixture.linked_path.join("file.txt")).unwrap(), "base\n");
    assert_eq!(fixture.repo.find_branch("feature", BranchType::Local).unwrap().get().target(), Some(feature_commit));

    let mut hidden_branch_names = HashSet::new();
    let mut local = HashMap::new();

    checkout_branch(&fixture.linked_repo, &mut hidden_branch_names, &mut local, 7, "release").unwrap();

    assert_eq!(get_current_branch(&fixture.linked_repo).as_deref(), Some("release"));
    assert_eq!(fs::read_to_string(fixture.linked_path.join("file.txt")).unwrap(), "base\n");
    assert!(fixture.repo.find_branch("release", BranchType::Local).is_ok());
    assert!(local.is_empty());
    assert!(!hidden_branch_names.contains("release"));
    assert_eq!(fixture.repo.find_branch("feature", BranchType::Local).unwrap().get().target(), Some(feature_commit));
}

#[test]
fn checkout_branch_bootstraps_a_local_branch_from_a_remote_tracking_ref() {
    let dir = TestDir::new("checkout-linked-remote-branch");
    let fixture = linked_worktree_fixture(&dir, "feature");
    fixture.repo.remote("origin", "https://example.com/origin.git").unwrap();
    fixture.repo.reference("refs/remotes/origin/release", fixture.base, true, "seed remote tracking ref").unwrap();

    let mut hidden_branch_names = HashSet::new();
    hidden_branch_names.insert("release".to_string());
    let mut local = HashMap::new();

    checkout_branch(&fixture.linked_repo, &mut hidden_branch_names, &mut local, 7, "origin/release").unwrap();

    let release = fixture.repo.find_branch("release", BranchType::Local).unwrap();
    let expected_local = vec!["release".to_string()];
    assert_eq!(get_current_branch(&fixture.linked_repo).as_deref(), Some("release"));
    assert_eq!(fs::read_to_string(fixture.linked_path.join("file.txt")).unwrap(), "base\n");
    assert_eq!(release.upstream().unwrap().get().name(), Some("refs/remotes/origin/release"));
    assert_eq!(local.get(&7), Some(&expected_local));
    assert!(!hidden_branch_names.contains("release"));

    assert!(checkout_branch(&fixture.linked_repo, &mut hidden_branch_names, &mut local, 7, "origin/missing").is_err());
}
