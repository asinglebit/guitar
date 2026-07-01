use super::*;
use crate::git::test_support::{assert_workdir_file, checkout_branch, checkout_new_branch, commit_file, commit_named_file as commit, diverge_file, diverge_files, temp_repo, write_workdir_file};

#[test]
fn fast_forward_updates_branch_and_workdir() {
    let (_dir, repo) = temp_repo("fast-forward");
    commit_file(&repo, "file.txt", "base\n", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    let feature = commit_file(&repo, "file.txt", "feature\n", "feature");
    checkout_branch(&repo, &base_branch);

    assert_eq!(start_merge(&repo, feature).unwrap(), MergeOutcome::FastForward { oid: feature });
    assert_eq!(repo.head().unwrap().target(), Some(feature));
    assert_workdir_file(&repo, "file.txt", "feature\n");
    assert!(!is_merge_in_progress(&repo));
}

#[test]
fn up_to_date_merge_does_nothing() {
    let (_dir, repo) = temp_repo("up-to-date");
    let base = commit_file(&repo, "file.txt", "base\n", "base");

    assert_eq!(start_merge(&repo, base).unwrap(), MergeOutcome::UpToDate);
    assert_eq!(repo.head().unwrap().target(), Some(base));
    assert_workdir_file(&repo, "file.txt", "base\n");
    assert!(!is_merge_in_progress(&repo));
}

#[test]
fn divergent_clean_merge_creates_two_parent_commit() {
    let (_dir, repo) = temp_repo("clean-divergent");
    let (_, feature, main) = diverge_files(&repo, "base.txt", "feature.txt", "main.txt");

    let MergeOutcome::Completed { oid } = start_merge(&repo, feature).unwrap() else {
        panic!("expected completed merge");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.parent_count(), 2);
    assert_eq!(head.parent(0).unwrap().id(), main);
    assert_eq!(head.parent(1).unwrap().id(), feature);
    assert_workdir_file(&repo, "feature.txt", "feature\n");
    assert_workdir_file(&repo, "main.txt", "main\n");
    assert!(!is_merge_in_progress(&repo));
}

#[test]
fn conflict_then_continue_finishes_after_workdir_resolution() {
    let (_dir, repo) = temp_repo("conflict-continue");
    let (feature, main) = diverge_file(&repo, "file.txt");

    assert_eq!(start_merge(&repo, feature).unwrap(), MergeOutcome::Conflict);
    assert!(is_merge_in_progress(&repo));
    assert!(repo.index().unwrap().has_conflicts());
    assert_eq!(continue_merge(&repo).unwrap(), MergeOutcome::Conflict);

    write_workdir_file(&repo, "file.txt", "resolved\n");
    let MergeOutcome::Completed { oid } = continue_merge(&repo).unwrap() else {
        panic!("expected completed merge");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.parent_count(), 2);
    assert_eq!(head.parent(0).unwrap().id(), main);
    assert_eq!(head.parent(1).unwrap().id(), feature);
    assert_workdir_file(&repo, "file.txt", "resolved\n");
    assert!(!is_merge_in_progress(&repo));
}

#[test]
fn abort_restores_pre_merge_state() {
    let (_dir, repo) = temp_repo("abort");
    let (feature, main) = diverge_file(&repo, "file.txt");

    assert_eq!(start_merge(&repo, feature).unwrap(), MergeOutcome::Conflict);
    assert_eq!(abort_merge(&repo).unwrap(), MergeOutcome::Aborted);
    assert!(!is_merge_in_progress(&repo));
    assert_eq!(repo.head().unwrap().target(), Some(main));
    assert_workdir_file(&repo, "file.txt", "main\n");
}

#[test]
fn dirty_worktree_is_refused_before_start() {
    let (_dir, repo) = temp_repo("dirty");
    commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    let feature = commit(&repo, "feature.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write_workdir_file(&repo, "file.txt", "dirty\n");

    let error = start_merge(&repo, feature).unwrap_err();
    assert!(error.message().contains("working tree must be clean"));
}

#[test]
fn merge_ff_false_creates_merge_commit_for_fast_forward() {
    let (_dir, repo) = temp_repo("no-ff");
    let base = commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    let feature = commit(&repo, "file.txt", "feature");
    checkout_branch(&repo, &base_branch);
    repo.config().unwrap().set_str("merge.ff", "false").unwrap();

    let MergeOutcome::Completed { oid } = start_merge(&repo, feature).unwrap() else {
        panic!("expected merge commit");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.parent_count(), 2);
    assert_eq!(head.parent(0).unwrap().id(), base);
    assert_eq!(head.parent(1).unwrap().id(), feature);
    assert_workdir_file(&repo, "file.txt", "feature\n");
}

#[test]
fn merge_ff_only_refuses_divergent_history() {
    let (_dir, repo) = temp_repo("ff-only");
    let (feature, main) = diverge_file(&repo, "file.txt");
    repo.config().unwrap().set_str("merge.ff", "only").unwrap();

    let error = start_merge(&repo, feature).unwrap_err();
    assert!(error.message().contains("merge.ff=only"));
    assert_eq!(repo.head().unwrap().target(), Some(main));
    assert!(!is_merge_in_progress(&repo));
}
