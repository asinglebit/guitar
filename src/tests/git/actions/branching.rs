use super::*;
use crate::git::queries::commits::get_current_branch;
use crate::git::test_support::{commit_file, temp_repo};
use git2::BranchType;

fn set_branch_upstream(repo: &Repository, branch: &str, remote: &str, target: git2::Oid) {
    repo.remote(remote, "https://example.com/origin.git").unwrap();
    repo.reference(&format!("refs/remotes/{remote}/{branch}"), target, true, "seed remote tracking ref").unwrap();

    let mut config = repo.config().unwrap();
    config.set_str(&format!("branch.{branch}.remote"), remote).unwrap();
    config.set_str(&format!("branch.{branch}.merge"), &format!("refs/heads/{branch}")).unwrap();
}

#[test]
fn create_branch_creates_local_branch_and_rejects_existing_branch() {
    let (_dir, repo) = temp_repo("create");
    let oid = commit_file(&repo, "file.txt", "content\n", "initial");

    create_branch(&repo, "feature", oid).unwrap();
    assert!(create_branch(&repo, "feature", oid).is_err());
    assert_eq!(repo.find_branch("feature", BranchType::Local).unwrap().get().target(), Some(oid));
}

#[test]
fn renames_local_branch_and_preserves_target() {
    let (_dir, repo) = temp_repo("rename-preserves-target");
    let oid = commit_file(&repo, "file.txt", "content\n", "initial");
    let target = repo.find_commit(oid).unwrap();
    repo.branch("feature", &target, false).unwrap();

    rename_branch(&repo, "feature", "topic").unwrap();

    assert!(repo.find_branch("feature", BranchType::Local).is_err());
    let branch = repo.find_branch("topic", BranchType::Local).unwrap();
    assert_eq!(branch.get().target(), Some(oid));
}

#[test]
fn renames_current_branch() {
    let (_dir, repo) = temp_repo("rename-current");
    commit_file(&repo, "file.txt", "content\n", "initial");
    let current_branch = get_current_branch(&repo).unwrap();
    let renamed = if current_branch == "main" { "topic" } else { "main" };

    rename_branch(&repo, &current_branch, renamed).unwrap();

    assert_eq!(get_current_branch(&repo).as_deref(), Some(renamed));
    assert!(repo.find_branch(&current_branch, BranchType::Local).is_err());
    assert!(repo.find_branch(renamed, BranchType::Local).is_ok());
}

#[test]
fn renames_current_branch_and_preserves_upstream_config() {
    let (_dir, repo) = temp_repo("rename-current-upstream");
    let oid = commit_file(&repo, "file.txt", "content\n", "initial");
    let current_branch = get_current_branch(&repo).unwrap();
    let renamed = if current_branch == "main" { "topic" } else { "main" };

    set_branch_upstream(&repo, &current_branch, "origin", oid);

    rename_branch(&repo, &current_branch, renamed).unwrap();

    let config = repo.config().unwrap();
    assert_eq!(get_current_branch(&repo).as_deref(), Some(renamed));
    assert!(repo.find_branch(&current_branch, BranchType::Local).is_err());
    assert_eq!(repo.find_branch(renamed, BranchType::Local).unwrap().get().target(), Some(oid));
    assert_eq!(config.get_string(&format!("branch.{renamed}.remote")).unwrap(), "origin");
    assert_eq!(config.get_string(&format!("branch.{renamed}.merge")).unwrap(), format!("refs/heads/{current_branch}"));
    assert!(config.get_string(&format!("branch.{current_branch}.remote")).is_err());
    assert!(config.get_string(&format!("branch.{current_branch}.merge")).is_err());
    let expected_upstream = format!("refs/remotes/origin/{current_branch}");
    assert_eq!(repo.find_branch(renamed, BranchType::Local).unwrap().upstream().unwrap().get().name(), Some(expected_upstream.as_str()));
}

#[test]
fn rejects_empty_invalid_unchanged_and_existing_names() {
    let (_dir, repo) = temp_repo("rename-invalid");
    let oid = commit_file(&repo, "file.txt", "content\n", "initial");
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

#[test]
fn delete_branch_rejects_current_branch_and_removes_feature_config() {
    let (_path, repo) = temp_repo("delete-config");
    let oid = commit_file(&repo, "file.txt", "content\n", "initial");
    let current_branch = get_current_branch(&repo).unwrap();
    let target = repo.find_commit(oid).unwrap();
    repo.branch("feature", &target, false).unwrap();
    set_branch_upstream(&repo, "feature", "origin", oid);

    assert!(delete_branch(&repo, &current_branch).is_err());
    assert!(repo.find_branch(&current_branch, BranchType::Local).is_ok());

    delete_branch(&repo, "feature").unwrap();

    let config = repo.config().unwrap();
    assert!(repo.find_branch("feature", BranchType::Local).is_err());
    assert!(config.get_string("branch.feature.remote").is_err());
    assert!(config.get_string("branch.feature.merge").is_err());
}
