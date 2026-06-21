use super::*;
use crate::git::{
    actions::worktrees::create_worktree,
    queries::commits::get_current_branch,
    repository::open,
    test_support::{TestDir, commit_file, create_branch, init_repo_at},
};
use git2::BranchType;
use im::HashSet;
use std::{collections::HashMap, fs};

#[test]
fn checkout_head_detaches_a_linked_worktree_and_leaves_the_branch_alone() {
    let dir = TestDir::new("checkout-linked-head");
    let repo = init_repo_at(&dir.join("repo"));
    let base = commit_file(&repo, "file.txt", "base\n", "base");

    let linked_path = dir.join("feature");
    create_worktree(&repo, "feature", &linked_path, base).unwrap();
    let linked_repo = open(&linked_path).unwrap();
    let feature_commit = commit_file(&linked_repo, "file.txt", "feature\n", "feature");

    checkout_head(&linked_repo, base).unwrap();

    assert_eq!(get_current_branch(&linked_repo), None);
    assert!(!linked_repo.head().unwrap().is_branch());
    assert_eq!(fs::read_to_string(linked_path.join("file.txt")).unwrap(), "base\n");
    assert_eq!(repo.find_branch("feature", BranchType::Local).unwrap().get().target(), Some(feature_commit));
}

#[test]
fn checkout_branch_switches_a_linked_worktree_to_an_existing_branch() {
    let dir = TestDir::new("checkout-linked-branch");
    let repo = init_repo_at(&dir.join("repo"));
    let base = commit_file(&repo, "file.txt", "base\n", "base");
    create_branch(&repo, "release", base);

    let linked_path = dir.join("feature");
    create_worktree(&repo, "feature", &linked_path, base).unwrap();
    let linked_repo = open(&linked_path).unwrap();
    let feature_commit = commit_file(&linked_repo, "file.txt", "feature\n", "feature");

    let mut hidden_branch_names = HashSet::new();
    let mut local = HashMap::new();

    checkout_branch(&linked_repo, &mut hidden_branch_names, &mut local, 7, "release").unwrap();

    assert_eq!(get_current_branch(&linked_repo).as_deref(), Some("release"));
    assert_eq!(fs::read_to_string(linked_path.join("file.txt")).unwrap(), "base\n");
    assert!(repo.find_branch("release", BranchType::Local).is_ok());
    assert!(local.is_empty());
    assert!(!hidden_branch_names.contains("release"));
    assert_eq!(repo.find_branch("feature", BranchType::Local).unwrap().get().target(), Some(feature_commit));
}
