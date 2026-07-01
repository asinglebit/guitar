use super::*;
use crate::git::test_support::{assert_workdir_file, checkout_branch, diverge_file, diverge_files, temp_repo, write_workdir_file};

#[test]
fn clean_rebase_completes_and_updates_branch() {
    let (_dir, repo) = temp_repo("clean");
    let (base, _, main) = diverge_files(&repo, "file.txt", "feature.txt", "main.txt");
    checkout_branch(&repo, "feature");

    let outcome = start_rebase(&repo, main).unwrap();
    assert_eq!(outcome, RebaseOutcome::Completed { applied: 1 });
    assert_eq!(repo.head().unwrap().shorthand(), Some("feature"));
    assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().parent(0).unwrap().id(), main);
    assert_ne!(repo.head().unwrap().target(), Some(base));
}

#[test]
fn dirty_worktree_is_refused_before_start() {
    let (_dir, repo) = temp_repo("dirty");
    let (_, _, main) = diverge_files(&repo, "file.txt", "feature.txt", "main.txt");
    checkout_branch(&repo, "feature");
    write_workdir_file(&repo, "file.txt", "dirty\n");

    let error = start_rebase(&repo, main).unwrap_err();
    assert!(error.message().contains("working tree must be clean"));
}

#[test]
fn conflict_then_continue_finishes() {
    let (_dir, repo) = temp_repo("conflict-continue");
    let (_, main) = diverge_file(&repo, "file.txt");
    checkout_branch(&repo, "feature");

    assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);
    assert!(is_rebase_in_progress(&repo));
    assert!(repo.index().unwrap().has_conflicts());

    write_workdir_file(&repo, "file.txt", "resolved\n");
    assert_eq!(continue_rebase(&repo).unwrap(), RebaseOutcome::Completed { applied: 1 });
    assert!(!is_rebase_in_progress(&repo));
    assert_workdir_file(&repo, "file.txt", "resolved\n");
}

#[test]
fn abort_restores_pre_rebase_state() {
    let (_dir, repo) = temp_repo("abort");
    let (original_feature, main) = diverge_file(&repo, "file.txt");
    checkout_branch(&repo, "feature");

    assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);
    assert_eq!(abort_rebase(&repo).unwrap(), RebaseOutcome::Aborted);
    assert!(!is_rebase_in_progress(&repo));
    assert_eq!(repo.head().unwrap().target(), Some(original_feature));
    assert_workdir_file(&repo, "file.txt", "feature\n");
}
