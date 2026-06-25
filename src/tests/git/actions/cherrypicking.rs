use super::*;
use crate::git::test_support::{assert_workdir_file, diverge_file, diverge_files, temp_repo, write_workdir_file};

#[test]
fn clean_cherrypick_commits_with_edited_message() {
    let (_dir, repo) = temp_repo("clean");
    let (_, feature, main) = diverge_files(&repo, "base.txt", "feature.txt", "main.txt");

    let outcome = start_cherrypick(&repo, feature, "cherrypicked: feature").unwrap();
    let CherrypickOutcome::Committed { oid } = outcome else {
        panic!("expected committed outcome");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.parent(0).unwrap().id(), main);
    assert_eq!(head.summary(), Some("cherrypicked: feature"));
    assert!(!is_cherrypick_in_progress(&repo));
    assert!(!message_path(&repo).exists());
}

#[test]
fn conflict_then_continue_commits_with_persisted_message() {
    let (_dir, repo) = temp_repo("conflict-continue");
    let (feature, _) = diverge_file(&repo, "file.txt");

    assert_eq!(start_cherrypick(&repo, feature, "cherrypicked: feature").unwrap(), CherrypickOutcome::Conflict);
    assert!(is_cherrypick_in_progress(&repo));
    assert!(repo.index().unwrap().has_conflicts());
    assert_eq!(continue_cherrypick(&repo).unwrap(), CherrypickOutcome::Conflict);

    write_workdir_file(&repo, "file.txt", "resolved\n");
    let outcome = continue_cherrypick(&repo).unwrap();
    let CherrypickOutcome::Committed { oid } = outcome else {
        panic!("expected committed outcome");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.summary(), Some("cherrypicked: feature"));
    assert_workdir_file(&repo, "file.txt", "resolved\n");
    assert!(!is_cherrypick_in_progress(&repo));
    assert!(!message_path(&repo).exists());
}

#[test]
fn abort_restores_pre_cherrypick_state() {
    let (_dir, repo) = temp_repo("abort");
    let (feature, main) = diverge_file(&repo, "file.txt");

    assert_eq!(start_cherrypick(&repo, feature, "cherrypicked: feature").unwrap(), CherrypickOutcome::Conflict);
    assert_eq!(abort_cherrypick(&repo).unwrap(), CherrypickOutcome::Aborted);
    assert!(!is_cherrypick_in_progress(&repo));
    assert_eq!(repo.head().unwrap().target(), Some(main));
    assert_workdir_file(&repo, "file.txt", "main\n");
    assert!(!message_path(&repo).exists());
}
