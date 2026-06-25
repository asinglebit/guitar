use super::*;
use crate::git::{
    actions::{tagging::tag, worktrees::create_worktree},
    auth::{AuthSession, NetworkResult},
    test_support::{TestDir, add_remote_path, commit_file, create_branch, init_repo_at, seed_remote, source_with_origin},
};
use git2::Repository;
use std::fs;

#[test]
fn fetch_reports_failures_and_populates_refs_tags_and_linked_worktrees() {
    let dir = TestDir::new("fetch");
    let (source, remote_path) = source_with_origin(&dir);
    let commit = commit_file(&source, "file.txt", "source\n", "source");
    create_branch(&source, "feature", commit);
    tag(&source, commit, "v1.0.0").unwrap();
    seed_remote(&source, "origin", &["refs/heads/feature:refs/heads/feature", "refs/tags/v1.0.0:refs/tags/v1.0.0"]);

    let consumer = init_repo_at(&dir.join("consumer"));
    let handle = fetch_remote(consumer.workdir().unwrap().to_str().unwrap(), "origin", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Failure(_)));

    add_remote_path(&consumer, "origin", &remote_path);

    let handle = fetch_remote(consumer.workdir().unwrap().to_str().unwrap(), "origin", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    assert_eq!(consumer.find_reference("refs/remotes/origin/feature").unwrap().target(), Some(commit));
    assert!(consumer.find_reference("refs/tags/v1.0.0").is_ok());

    let worktree_owner = init_repo_at(&dir.join("worktree-owner"));
    let owner_commit = commit_file(&worktree_owner, "consumer.txt", "consumer\n", "consumer");
    add_remote_path(&worktree_owner, "origin", &remote_path);
    let linked_path = dir.join("linked");
    create_worktree(&worktree_owner, "linked", &linked_path, owner_commit).unwrap();

    let handle = fetch_remote(linked_path.to_str().unwrap(), "origin", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    let linked_repo = Repository::open(&linked_path).unwrap();
    assert_eq!(linked_repo.find_reference("refs/remotes/origin/feature").unwrap().target(), Some(commit));

    fs::remove_dir_all(&remote_path).unwrap();
    let handle = fetch_remote(consumer.workdir().unwrap().to_str().unwrap(), "origin", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Failure(_)));
}
