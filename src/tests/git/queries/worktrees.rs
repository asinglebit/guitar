use super::*;
use crate::git::actions::worktrees::create_worktree;
#[path = "../support.rs"]
mod support;
use git2::Repository;
use std::{fs, path::Path};
use support::{TestDir, init_repo, stage, write};

fn worktree_repo(name: &str) -> (TestDir, std::path::PathBuf, Repository, git2::Oid) {
    let dir = TestDir::new("worktree", name);
    let repo_path = dir.path.join("repo");
    let repo = init_repo(&repo_path);
    let oid = repo.head().unwrap().target().unwrap();
    (dir, repo_path, repo, oid)
}

fn create_linked(repo: &Repository, dir: &TestDir, name: &str, oid: git2::Oid) -> std::path::PathBuf {
    let path = dir.path.join(format!("repo-{name}"));
    create_worktree(repo, name, &path, oid).unwrap();
    path
}

fn linked_entry<'a>(repo: &Repository, repo_path: &Path, name: &str) -> crate::core::worktrees::WorktreeEntry {
    list_worktrees(repo, Some(repo_path)).unwrap().into_iter().find(|entry| entry.name == name).unwrap()
}

#[test]
fn lists_main_and_linked_worktrees() {
    let (dir, repo_path, repo, oid) = worktree_repo("list");
    create_linked(&repo, &dir, "feature", oid);

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|entry| entry.is_main() && entry.is_current));
    assert!(entries.iter().any(|entry| entry.name == "feature" && entry.is_linked() && entry.is_valid));
}

#[test]
fn marks_current_linked_worktree() {
    let (dir, _repo_path, repo, oid) = worktree_repo("current");
    let worktree_path = create_linked(&repo, &dir, "feature", oid);
    let linked_repo = Repository::open(&worktree_path).unwrap();

    let entries = list_worktrees(&linked_repo, Some(&worktree_path)).unwrap();
    assert!(entries.iter().any(|entry| entry.name == "feature" && entry.is_current));
    assert!(entries.iter().any(|entry| entry.is_main() && !entry.is_current));
}

#[test]
fn marks_untracked_directory_worktree_dirty() {
    let (dir, repo_path, repo, oid) = worktree_repo("dir-dirty");
    let worktree_path = create_linked(&repo, &dir, "feature", oid);
    write(&worktree_path, "target/very/deep/scratch.txt", "scratch\n");

    assert!(linked_entry(&repo, &repo_path, "feature").is_dirty);
}

#[test]
fn marks_modified_tracked_worktree_dirty() {
    let (dir, repo_path, repo, oid) = worktree_repo("tracked-dirty");
    let worktree_path = create_linked(&repo, &dir, "feature", oid);
    write(&worktree_path, "file.txt", "changed\n");

    assert!(linked_entry(&repo, &repo_path, "feature").is_dirty);
}

#[test]
fn marks_staged_worktree_dirty() {
    let (dir, repo_path, repo, oid) = worktree_repo("staged-dirty");
    let worktree_path = create_linked(&repo, &dir, "feature", oid);
    write(&worktree_path, "file.txt", "staged\n");
    let linked = Repository::open(&worktree_path).unwrap();
    stage(&linked, "file.txt");

    assert!(linked_entry(&repo, &repo_path, "feature").is_dirty);
}

#[test]
fn lists_multiple_linked_worktrees_sorted_with_dirty_flags() {
    let (dir, repo_path, repo, oid) = worktree_repo("many-linked");

    create_linked(&repo, &dir, "gamma", oid);
    create_linked(&repo, &dir, "alpha", oid);
    let beta = create_linked(&repo, &dir, "beta", oid);
    write(&beta, "scratch.txt", "dirty\n");

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let linked = entries.iter().filter(|entry| entry.is_linked()).collect::<Vec<_>>();

    assert_eq!(linked.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>(), vec!["alpha", "beta", "gamma"]);
    assert!(!linked.iter().find(|entry| entry.name == "alpha").unwrap().is_dirty);
    assert!(linked.iter().find(|entry| entry.name == "beta").unwrap().is_dirty);
    assert!(!linked.iter().find(|entry| entry.name == "gamma").unwrap().is_dirty);
}

#[test]
fn marks_removed_linked_worktree_invalid_and_prunable() {
    let (dir, repo_path, repo, oid) = worktree_repo("invalid");
    let worktree_path = create_linked(&repo, &dir, "feature", oid);
    fs::remove_dir_all(&worktree_path).unwrap();

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let feature = entries.iter().find(|entry| entry.name == "feature").unwrap();
    assert!(!feature.is_valid);
    assert!(feature.is_prunable);
}
