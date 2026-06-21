use crate::git::{
    actions::worktrees::create_worktree,
    repository::{open, open_worktree_owner},
    test_support::{TestDir, commit_file, init_repo_at},
};

#[test]
fn open_repository_can_open_a_linked_worktree_path() {
    let dir = TestDir::new("repository-open");
    let repo = init_repo_at(&dir.join("repo"));
    let target = commit_file(&repo, "file.txt", "base\n", "base");
    let linked_path = dir.join("feature");
    create_worktree(&repo, "feature", &linked_path, target).unwrap();

    let linked_repo = open(&linked_path).unwrap();

    assert!(linked_repo.is_worktree());
    assert_eq!(linked_repo.commondir(), repo.commondir());
}

#[test]
fn open_worktree_owner_returns_the_shared_owner_repo() {
    let dir = TestDir::new("repository-owner");
    let repo = init_repo_at(&dir.join("repo"));
    let target = commit_file(&repo, "file.txt", "base\n", "base");
    let linked_path = dir.join("feature");
    create_worktree(&repo, "feature", &linked_path, target).unwrap();
    let linked_repo = open(&linked_path).unwrap();

    let owner = open_worktree_owner(&linked_repo).unwrap();

    assert_eq!(owner.workdir(), repo.workdir());
    assert!(owner.find_worktree("feature").is_ok());
}
