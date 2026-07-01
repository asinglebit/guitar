use super::*;
use crate::git::{
    actions::tagging::tag,
    auth::{AuthSession, NetworkResult},
    test_support::{TestDir, commit_file, create_branch, source_with_origin},
};
use git2::{ObjectType, Repository};

#[test]
fn push_updates_branch_tags_and_deletes_the_remote_branch() {
    let dir = TestDir::new("push-branch");
    let (source, remote_path) = source_with_origin(&dir);
    let commit = commit_file(&source, "file.txt", "source\n", "source");
    create_branch(&source, "feature", commit);
    tag(&source, commit, "v1.0.0").unwrap();
    let commit_obj = source.find_object(commit, Some(ObjectType::Commit)).unwrap();
    let signature = source.signature().unwrap();
    source.tag("v2.0.0", &commit_obj, &signature, "release", false).unwrap();

    let handle = push_branch(source.workdir().unwrap().to_str().unwrap(), "origin", "feature", false, AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    let remote = Repository::open(&remote_path).unwrap();
    assert_eq!(remote.find_reference("refs/heads/feature").unwrap().target(), Some(commit));
    drop(remote);

    let handle = delete_remote_branch(source.workdir().unwrap().to_str().unwrap(), "origin", "feature", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    let remote = Repository::open(&remote_path).unwrap();
    assert!(remote.find_reference("refs/heads/feature").is_err());

    let handle = push_tags(source.workdir().unwrap().to_str().unwrap(), "origin", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    let remote = Repository::open(&remote_path).unwrap();
    assert!(remote.find_reference("refs/tags/v1.0.0").is_ok());
    assert!(remote.find_reference("refs/tags/v2.0.0").is_ok());
}
