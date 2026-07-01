use super::*;
use crate::git::test_support::{assert_workdir_file, commit_file, diverge_file, diverge_files, temp_repo, write_workdir_file};
use git2::Signature;

#[test]
fn clean_revert_commits_with_edited_message() {
    let (_dir, repo) = temp_repo("clean");
    let base = commit_file(&repo, "base.txt", "base\n", "base");
    let feature = commit_file(&repo, "feature.txt", "feature\n", "feature");

    let outcome = start_revert(&repo, feature, "reverted: feature").unwrap();
    let RevertOutcome::Committed { oid } = outcome else {
        panic!("expected committed outcome");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.parent(0).unwrap().id(), feature);
    assert_eq!(head.summary(), Some("reverted: feature"));
    assert!(!repo.workdir().unwrap().join("feature.txt").exists());
    assert!(repo.workdir().unwrap().join("base.txt").exists());
    assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().parent(0).unwrap().parent(0).unwrap().id(), base);
    assert!(!is_revert_in_progress(&repo));
    assert!(!message_path(&repo).exists());
}

#[test]
fn conflict_then_continue_commits_with_persisted_message() {
    let (_dir, repo) = temp_repo("conflict-continue");
    let (feature, _) = diverge_file(&repo, "file.txt");

    assert_eq!(start_revert(&repo, feature, "reverted: feature").unwrap(), RevertOutcome::Conflict);
    assert!(is_revert_in_progress(&repo));
    assert!(repo.index().unwrap().has_conflicts());
    assert_eq!(continue_revert(&repo).unwrap(), RevertOutcome::Conflict);

    write_workdir_file(&repo, "file.txt", "resolved\n");
    let outcome = continue_revert(&repo).unwrap();
    let RevertOutcome::Committed { oid } = outcome else {
        panic!("expected committed outcome");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.summary(), Some("reverted: feature"));
    assert_workdir_file(&repo, "file.txt", "resolved\n");
    assert!(!is_revert_in_progress(&repo));
    assert!(!message_path(&repo).exists());
}

#[test]
fn abort_restores_pre_revert_state() {
    let (_dir, repo) = temp_repo("abort");
    let (feature, main) = diverge_file(&repo, "file.txt");

    assert_eq!(start_revert(&repo, feature, "reverted: feature").unwrap(), RevertOutcome::Conflict);
    assert_eq!(abort_revert(&repo).unwrap(), RevertOutcome::Aborted);
    assert!(!is_revert_in_progress(&repo));
    assert_eq!(repo.head().unwrap().target(), Some(main));
    assert_workdir_file(&repo, "file.txt", "main\n");
    assert!(!message_path(&repo).exists());
}

#[test]
fn merge_commits_are_rejected() {
    let (_dir, repo) = temp_repo("merge-reject");
    let (_, feature, main) = diverge_files(&repo, "base.txt", "feature.txt", "main.txt");

    let feature_commit = repo.find_commit(feature).unwrap();
    let main_commit = repo.find_commit(main).unwrap();
    let mut index = repo.index().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let merge = repo.commit(Some("HEAD"), &sig, &sig, "merge", &tree, &[&main_commit, &feature_commit]).unwrap();

    let error = start_revert(&repo, merge, "reverted: merge").unwrap_err();
    assert!(error.message().contains("merge commits"));
    assert!(!is_revert_in_progress(&repo));
    assert!(!message_path(&repo).exists());
}
