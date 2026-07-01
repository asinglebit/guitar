use super::*;
use crate::git::queries::worktrees::list_worktrees;
use crate::git::test_support::TestDir;
use git2::{Repository, Signature};
use std::{fs, path::Path};

fn init_repo(path: &Path) -> Repository {
    let repo = Repository::init(path).unwrap();
    fs::write(path.join("file.txt"), "hello\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Tester", "tester@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
    drop(tree);
    repo
}

#[test]
fn validates_v1_worktree_names() {
    assert!(is_valid_worktree_name("feature"));
    assert!(!is_valid_worktree_name(""));
    assert!(!is_valid_worktree_name("../feature"));
    assert!(!is_valid_worktree_name("topic/feature"));
    assert!(!is_valid_worktree_name("topic\\feature"));
}

#[test]
fn creates_locks_unlocks_and_removes_worktree() {
    let dir = TestDir::new("worktree-actions");
    let repo_path = dir.join("repo");
    let worktree_path = dir.join("repo-feature");
    fs::create_dir_all(&repo_path).unwrap();
    let repo = init_repo(&repo_path);
    let oid = repo.head().unwrap().target().unwrap();

    create_worktree(&repo, "feature", &worktree_path, oid).unwrap();
    assert!(worktree_path.join(".git").is_file());
    assert!(repo.find_branch("feature", BranchType::Local).is_ok());

    lock_worktree(&repo, "feature", Some("keep it")).unwrap();
    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let feature = entries.iter().find(|entry| entry.name == "feature").unwrap();
    assert_eq!(feature.locked_reason.as_deref(), Some("keep it"));
    assert!(remove_worktree(&repo, "feature").is_err());

    unlock_worktree(&repo, "feature").unwrap();
    remove_worktree(&repo, "feature").unwrap();
    assert!(!worktree_path.exists());
    assert!(repo.find_worktree("feature").is_err());
}

#[test]
fn prunes_invalid_worktree_metadata() {
    let dir = TestDir::new("worktree-prune");
    let repo_path = dir.join("repo");
    let worktree_path = dir.join("repo-feature");
    fs::create_dir_all(&repo_path).unwrap();
    let repo = init_repo(&repo_path);
    let oid = repo.head().unwrap().target().unwrap();

    create_worktree(&repo, "feature", &worktree_path, oid).unwrap();
    fs::remove_dir_all(&worktree_path).unwrap();

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let feature = entries.iter().find(|entry| entry.name == "feature").unwrap();
    assert!(!feature.is_valid);

    remove_worktree(&repo, "feature").unwrap();
    assert!(repo.find_worktree("feature").is_err());
}

#[test]
fn creates_worktree_from_linked_worktree_repository() {
    let dir = TestDir::new("worktree-from-linked");
    let repo_path = dir.join("repo");
    let first_path = dir.join("repo-feature");
    let second_path = dir.join("repo-second");
    fs::create_dir_all(&repo_path).unwrap();
    let repo = init_repo(&repo_path);
    let oid = repo.head().unwrap().target().unwrap();

    create_worktree(&repo, "feature", &first_path, oid).unwrap();
    let linked_repo = Repository::open(&first_path).unwrap();

    create_worktree(&linked_repo, "second", &second_path, oid).unwrap();
    assert!(second_path.join(".git").is_file());
    assert!(repo.find_worktree("second").is_ok());
}
