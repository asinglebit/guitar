use super::*;
use crate::git::test_support::{TestDir, commit_file, init_repo_at};

fn repo_with_commit(name: &str) -> (TestDir, git2::Repository, git2::Oid) {
    let dir = TestDir::new(name);
    let repo = init_repo_at(&dir.join("repo"));
    let oid = commit_file(&repo, "file.txt", "content\n", "initial");
    (dir, repo, oid)
}

#[test]
fn tag_creates_rejects_existing_and_untag_removes_lightweight_reference() {
    let (_dir, repo, oid) = repo_with_commit("tag-lightweight");

    tag(&repo, oid, "v1.0.0").unwrap();
    assert_eq!(repo.find_reference("refs/tags/v1.0.0").unwrap().target(), Some(oid));

    assert!(tag(&repo, oid, "v1.0.0").is_err());
    assert_eq!(repo.find_reference("refs/tags/v1.0.0").unwrap().target(), Some(oid));

    untag(&repo, "v1.0.0").unwrap();
    assert!(repo.find_reference("refs/tags/v1.0.0").is_err());

    assert!(untag(&repo, "v1.0.0").is_err());
}
