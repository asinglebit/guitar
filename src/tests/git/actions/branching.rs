use super::*;
use crate::git::queries::commits::get_current_branch;
use git2::{Repository, Signature};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-branch-actions-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = crate::git::test_support::init_repo_at(&path);
    (path, repo)
}

fn commit(repo: &Repository, file: &str, message: &str) -> git2::Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), "content\n").unwrap();

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

#[test]
fn renames_local_branch_and_preserves_target() {
    let (_path, repo) = temp_repo("rename-preserves-target");
    let oid = commit(&repo, "file.txt", "initial");
    let target = repo.find_commit(oid).unwrap();
    repo.branch("feature", &target, false).unwrap();

    rename_branch(&repo, "feature", "topic").unwrap();

    assert!(repo.find_branch("feature", BranchType::Local).is_err());
    let branch = repo.find_branch("topic", BranchType::Local).unwrap();
    assert_eq!(branch.get().target(), Some(oid));
}

#[test]
fn renames_current_branch() {
    let (_path, repo) = temp_repo("rename-current");
    commit(&repo, "file.txt", "initial");

    rename_branch(&repo, "master", "main").unwrap();

    assert_eq!(get_current_branch(&repo).as_deref(), Some("main"));
    assert!(repo.find_branch("master", BranchType::Local).is_err());
    assert!(repo.find_branch("main", BranchType::Local).is_ok());
}

#[test]
fn rejects_empty_invalid_unchanged_and_existing_names() {
    let (_path, repo) = temp_repo("rename-invalid");
    let oid = commit(&repo, "file.txt", "initial");
    let target = repo.find_commit(oid).unwrap();
    repo.branch("feature", &target, false).unwrap();
    repo.branch("existing", &target, false).unwrap();

    assert!(rename_branch(&repo, "feature", "").is_err());
    assert!(rename_branch(&repo, "feature", "bad..name").is_err());
    assert!(rename_branch(&repo, "feature", "feature").is_err());
    assert!(rename_branch(&repo, "feature", "existing").is_err());
    assert!(repo.find_branch("feature", BranchType::Local).is_ok());
    assert!(repo.find_branch("existing", BranchType::Local).is_ok());
}
