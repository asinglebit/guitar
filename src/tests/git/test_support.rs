use super::*;
use crate::{
    core::oids::Oids,
    git::{
        actions::{
            checkout::{checkout_branch, checkout_head},
            committing::commit_staged,
            fetching::fetch_remote,
            merging::{MergeOutcome, start_merge},
            pushing::{delete_remote_branch, push_branch, push_tags},
            resetting::{reset_file, reset_to_commit},
            stashing::{pop, stash},
            tagging::{tag, untag},
            worktrees::{create_worktree, lock_worktree, remove_worktree, unlock_worktree},
        },
        auth::{AuthSession, NetworkResult},
        queries::{
            commits::{get_current_branch, get_stashed_commits, get_tag_oids, get_tip_oids},
            diffs::get_filenames_diff_at_workdir,
            submodules::list_submodules,
            worktrees::list_worktrees,
        },
    },
};
use git2::{BranchType, ErrorCode, ObjectType, Repository, ResetType, build::CheckoutBuilder};
use im::HashSet;
use std::{collections::HashMap, fs, path::Path};

fn checkout_branch_force(repo: &Repository, branch: &str) {
    repo.set_head(&format!("refs/heads/{branch}")).unwrap();
    repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
}

fn seed_remote(repo: &Repository, remote_name: &str, refspecs: &[&str]) {
    let mut remote = repo.find_remote(remote_name).unwrap();
    remote.push(refspecs, None).unwrap();
}

fn only_submodule_entry(repo: &Repository) -> crate::core::submodules::SubmoduleEntry {
    let entries = list_submodules(repo).unwrap();
    assert_eq!(entries.len(), 1);
    entries.into_iter().next().unwrap()
}

#[test]
fn unborn_repo_fixtures_support_root_commits() {
    let dir = TestDir::new("unborn");
    let repo = init_repo_at(&dir.join("repo"));

    match repo.head() {
        Err(err) => assert_eq!(err.code(), ErrorCode::UnbornBranch),
        Ok(_) => panic!("expected unborn HEAD"),
    }

    write_workdir_file(&repo, "file.txt", "root\n");
    stage_path(&repo, "file.txt");
    let oid = commit_staged(&repo, "root commit", "Test User", "test@example.com").unwrap();

    assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().id(), oid);
    let mut walk = repo.revwalk().unwrap();
    walk.push_head().unwrap();
    assert_eq!(walk.count(), 1);
}

#[test]
fn checkout_head_fixture_detaches_current_branch() {
    let dir = TestDir::new("checkout-head");
    let repo = init_repo_at(&dir.join("repo"));
    let first = commit_file(&repo, "file.txt", "first\n", "first");
    let _second = commit_file(&repo, "file.txt", "second\n", "second");

    checkout_head(&repo, first).unwrap();

    assert_eq!(get_current_branch(&repo), None);
    assert!(!repo.head().unwrap().is_branch());
    assert_eq!(fs::read_to_string(repo.workdir().unwrap().join("file.txt")).unwrap(), "first\n");
}

#[test]
fn checkout_branch_fixture_uses_existing_local_branches() {
    let dir = TestDir::new("checkout-local");
    let repo = init_repo_at(&dir.join("repo"));
    let target = commit_file(&repo, "file.txt", "base\n", "base");
    create_branch(&repo, "feature", target);

    let mut hidden_branch_names = HashSet::new();
    hidden_branch_names.insert("feature".to_string());
    let mut local = HashMap::new();

    checkout_branch(&repo, &mut hidden_branch_names, &mut local, 7, "feature").unwrap();

    assert_eq!(get_current_branch(&repo).as_deref(), Some("feature"));
    assert!(repo.find_branch("feature", BranchType::Local).is_ok());
    assert!(!hidden_branch_names.contains("feature"));
    assert!(local.is_empty());
}

#[test]
fn checkout_branch_fixture_materializes_remote_tracking_branches() {
    let dir = TestDir::new("checkout-remote");
    let repo = init_repo_at(&dir.join("repo"));
    let target = commit_file(&repo, "file.txt", "base\n", "base");
    repo.reference("refs/remotes/origin/topic", target, true, "remote").unwrap();
    repo.remote("origin", "https://example.com/origin.git").unwrap();

    let mut hidden_branch_names = HashSet::new();
    hidden_branch_names.insert("topic".to_string());
    let mut local = HashMap::new();

    checkout_branch(&repo, &mut hidden_branch_names, &mut local, 99, "origin/topic").unwrap();

    assert_eq!(get_current_branch(&repo).as_deref(), Some("topic"));
    assert!(!hidden_branch_names.contains("topic"));
    assert_eq!(local.get(&99), Some(&vec!["topic".to_string()]));
    assert!(repo.find_branch("topic", BranchType::Local).is_ok());
}

#[test]
fn reset_fixture_restores_commits_and_worktree_files() {
    let dir = TestDir::new("reset");
    let repo = init_repo_at(&dir.join("repo"));
    let first = commit_file(&repo, "file.txt", "first\n", "first");
    let _second = commit_file(&repo, "file.txt", "second\n", "second");

    reset_to_commit(&repo, first, ResetType::Hard).unwrap();

    assert_eq!(repo.head().unwrap().target(), Some(first));
    assert_eq!(fs::read_to_string(repo.workdir().unwrap().join("file.txt")).unwrap(), "first\n");
}

#[test]
fn reset_file_fixture_clears_staged_and_unstaged_changes() {
    let dir = TestDir::new("reset-file");
    let repo = init_repo_at(&dir.join("repo"));
    commit_file(&repo, "file.txt", "base\n", "base");

    write_workdir_file(&repo, "file.txt", "changed\n");
    stage_path(&repo, "file.txt");
    write_workdir_file(&repo, "file.txt", "dirty\n");

    let before = get_filenames_diff_at_workdir(&repo).unwrap();
    assert!(before.is_staged);
    assert!(before.is_unstaged);

    reset_file(&repo, Path::new("file.txt")).unwrap();

    let after = get_filenames_diff_at_workdir(&repo).unwrap();
    assert!(after.is_clean);
    assert_eq!(fs::read_to_string(repo.workdir().unwrap().join("file.txt")).unwrap(), "base\n");
}

#[test]
fn stash_fixture_saves_and_restores_dirty_worktrees() {
    let dir = TestDir::new("stash");
    let mut repo = init_repo_at(&dir.join("repo"));
    commit_file(&repo, "file.txt", "base\n", "base");

    write_workdir_file(&repo, "file.txt", "dirty\n");
    write_workdir_file(&repo, "untracked.txt", "extra\n");

    let stash_oid = stash(&mut repo).unwrap();
    let mut oids = Oids::default();
    let gix_repo = gix::open(repo.workdir().unwrap_or(repo.path())).unwrap();
    let stashed = get_stashed_commits(&gix_repo, &mut oids);
    assert_eq!(stashed.len(), 1);

    let after_stash = get_filenames_diff_at_workdir(&repo).unwrap();
    assert!(after_stash.is_clean);

    pop(&mut repo, &stash_oid, true).unwrap();

    let after_pop = get_filenames_diff_at_workdir(&repo).unwrap();
    assert!(after_pop.is_unstaged);
    assert!(after_pop.unstaged.modified.contains(&"file.txt".to_string()));
    assert!(after_pop.unstaged.added.contains(&"untracked.txt".to_string()));
    assert_eq!(fs::read_to_string(repo.workdir().unwrap().join("file.txt")).unwrap(), "dirty\n");
    assert_eq!(fs::read_to_string(repo.workdir().unwrap().join("untracked.txt")).unwrap(), "extra\n");

    let mut oids = Oids::default();
    let gix_repo = gix::open(repo.workdir().unwrap_or(repo.path())).unwrap();
    assert!(get_stashed_commits(&gix_repo, &mut oids).is_empty());
}

#[test]
fn worktree_fixture_lists_main_linked_locked_and_invalid_entries() {
    let dir = TestDir::new("worktrees");
    let repo = init_repo_at(&dir.join("repo"));
    let target = commit_file(&repo, "file.txt", "base\n", "base");

    let feature_path = dir.join("feature");
    create_worktree(&repo, "feature", &feature_path, target).unwrap();

    let entries = list_worktrees(&repo, Some(repo.workdir().unwrap())).unwrap();
    assert!(entries.iter().any(|entry| entry.is_main() && entry.is_current));
    assert!(entries.iter().any(|entry| entry.name == "feature" && entry.is_linked() && entry.is_valid));

    let linked_repo = Repository::open(&feature_path).unwrap();
    let linked_entries = list_worktrees(&linked_repo, Some(&feature_path)).unwrap();
    assert!(linked_entries.iter().any(|entry| entry.name == "feature" && entry.is_current));
    assert!(linked_entries.iter().any(|entry| entry.is_main() && !entry.is_current));

    lock_worktree(&repo, "feature", Some("keep it")).unwrap();
    let locked_entries = list_worktrees(&repo, Some(repo.workdir().unwrap())).unwrap();
    let locked = locked_entries.iter().find(|entry| entry.name == "feature").unwrap();
    assert_eq!(locked.locked_reason.as_deref(), Some("keep it"));
    assert!(!locked.can_remove());

    unlock_worktree(&repo, "feature").unwrap();
    remove_worktree(&repo, "feature").unwrap();
    assert!(repo.find_worktree("feature").is_err());

    let stale_path = dir.join("stale");
    create_worktree(&repo, "stale", &stale_path, target).unwrap();
    fs::remove_dir_all(&stale_path).unwrap();

    let entries = list_worktrees(&repo, Some(repo.workdir().unwrap())).unwrap();
    let stale = entries.iter().find(|entry| entry.name == "stale").unwrap();
    assert!(!stale.is_valid);
    assert!(stale.is_prunable);
}

#[test]
fn submodule_fixture_covers_clean_uninitialized_and_dirty_states() {
    let dir = TestDir::new("submodules");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let clean = only_submodule_entry(&parent);
    assert!(clean.is_open);
    assert!(!clean.is_dirty());
    assert!(clean.index.is_some());
    assert_eq!(clean.index, clean.workdir);

    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    commit_file(&sub_repo, "file.txt", "updated child\n", "advance child");
    write_workdir_file(&sub_repo, "file.txt", "dirty child\n");

    let dirty = only_submodule_entry(&parent);
    assert!(dirty.has_new_commits);
    assert!(dirty.has_modified_content);
    assert!(dirty.is_dirty());

    let clone_path = dir.join("clone");
    let clone = clone_repo(parent.workdir().unwrap(), &clone_path);
    let clone_entry = only_submodule_entry(&clone);
    assert!(!clone_entry.is_open);
    assert!(clone_entry.is_uninitialized || !clone_entry.is_in_workdir);
}

#[test]
fn conflict_fixture_reports_conflicts_in_the_index_and_workdir() {
    let dir = TestDir::new("conflict");
    let repo = init_repo_at(&dir.join("repo"));
    let base = commit_file(&repo, "file.txt", "base\n", "base");
    let base_branch = get_current_branch(&repo).unwrap();
    create_branch(&repo, "feature", base);

    checkout_branch_force(&repo, "feature");
    let feature = commit_file(&repo, "file.txt", "feature\n", "feature");

    checkout_branch_force(&repo, &base_branch);
    let _main = commit_file(&repo, "file.txt", "main\n", "main");

    assert_eq!(start_merge(&repo, feature).unwrap(), MergeOutcome::Conflict);
    assert!(repo.index().unwrap().has_conflicts());

    let diff = get_filenames_diff_at_workdir(&repo).unwrap();
    assert!(diff.has_conflicts);
    assert_eq!(diff.conflict_count, 1);
    assert_eq!(diff.conflicts, vec!["file.txt".to_string()]);
}

#[test]
fn tag_fixture_maps_commit_tags_and_ignores_blob_tags() {
    let dir = TestDir::new("tags");
    let repo = init_repo_at(&dir.join("repo"));
    let commit = commit_file(&repo, "file.txt", "base\n", "base");

    tag(&repo, commit, "v1.0.0").unwrap();
    let commit_obj = repo.find_object(commit, Some(ObjectType::Commit)).unwrap();
    let signature = repo.signature().unwrap();
    repo.tag("v2.0.0", &commit_obj, &signature, "release", false).unwrap();

    let blob_oid = repo.blob(b"ignored blob").unwrap();
    let blob_obj = repo.find_object(blob_oid, Some(ObjectType::Blob)).unwrap();
    repo.tag("blob-tag", &blob_obj, &signature, "blob", false).unwrap();

    let mut oids = Oids::default();
    let gix_repo = gix::open(&dir.join("repo")).unwrap();
    let (local_tips, remote_tips) = get_tip_oids(&gix_repo, &mut oids);
    let alias = oids.get_alias_by_oid(commit);
    let current_branch = get_current_branch(&repo).unwrap();
    assert!(local_tips.get(&alias).unwrap().contains(&current_branch));
    assert!(remote_tips.get(&alias).map_or(true, |names| names.is_empty()));

    let tags = get_tag_oids(&gix_repo, &mut oids);
    let tag_names = tags.get(&alias).unwrap();
    assert!(tag_names.contains(&"v1.0.0".to_string()));
    assert!(tag_names.contains(&"v2.0.0".to_string()));
    assert!(!tags.values().flatten().any(|name| name == "blob-tag"));

    untag(&repo, "v1.0.0").unwrap();
    assert!(repo.find_reference("refs/tags/v1.0.0").is_err());
}

#[test]
fn push_branch_fixture_updates_the_remote_branch() {
    let dir = TestDir::new("push-branch");
    let source = init_repo_at(&dir.join("source"));
    let commit = commit_file(&source, "file.txt", "source\n", "source");
    create_branch(&source, "feature", commit);
    let remote_path = dir.join("remote.git");
    init_bare_repo_at(&remote_path);
    add_remote_path(&source, "origin", &remote_path);

    let handle = push_branch(source.workdir().unwrap().to_str().unwrap(), "origin", "feature", false, AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    let remote = Repository::open(&remote_path).unwrap();
    assert_eq!(remote.find_reference("refs/heads/feature").unwrap().target(), Some(commit));
}

#[test]
fn push_tags_fixture_updates_remote_tags() {
    let dir = TestDir::new("push-tags");
    let source = init_repo_at(&dir.join("source"));
    let commit = commit_file(&source, "file.txt", "source\n", "source");
    tag(&source, commit, "v1.0.0").unwrap();
    let commit_obj = source.find_object(commit, Some(ObjectType::Commit)).unwrap();
    let signature = source.signature().unwrap();
    source.tag("v2.0.0", &commit_obj, &signature, "release", false).unwrap();
    let remote_path = dir.join("remote.git");
    init_bare_repo_at(&remote_path);
    add_remote_path(&source, "origin", &remote_path);

    let handle = push_tags(source.workdir().unwrap().to_str().unwrap(), "origin", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    let remote = Repository::open(&remote_path).unwrap();
    assert!(remote.find_reference("refs/tags/v1.0.0").is_ok());
    assert!(remote.find_reference("refs/tags/v2.0.0").is_ok());
}

#[test]
fn fetch_fixture_populates_remote_tracking_refs_and_tags() {
    let dir = TestDir::new("fetch");
    let source = init_repo_at(&dir.join("source"));
    let commit = commit_file(&source, "file.txt", "source\n", "source");
    create_branch(&source, "feature", commit);
    tag(&source, commit, "v1.0.0").unwrap();
    let commit_obj = source.find_object(commit, Some(ObjectType::Commit)).unwrap();
    let signature = source.signature().unwrap();
    source.tag("v2.0.0", &commit_obj, &signature, "release", false).unwrap();

    let remote_path = dir.join("remote.git");
    init_bare_repo_at(&remote_path);
    add_remote_path(&source, "origin", &remote_path);
    seed_remote(&source, "origin", &["refs/heads/feature:refs/heads/feature", "refs/tags/v1.0.0:refs/tags/v1.0.0", "refs/tags/v2.0.0:refs/tags/v2.0.0"]);

    let consumer = init_repo_at(&dir.join("consumer"));
    add_remote_path(&consumer, "origin", &remote_path);

    let handle = fetch_remote(consumer.workdir().unwrap().to_str().unwrap(), "origin", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    assert_eq!(consumer.find_reference("refs/remotes/origin/feature").unwrap().target(), Some(commit));
    assert!(consumer.find_reference("refs/tags/v1.0.0").is_ok());
    assert!(consumer.find_reference("refs/tags/v2.0.0").is_ok());
}

#[test]
fn fetch_fixture_errors_for_missing_remote() {
    let dir = TestDir::new("fetch-missing-remote");
    let consumer = init_repo_at(&dir.join("consumer"));

    let handle = fetch_remote(consumer.workdir().unwrap().to_str().unwrap(), "origin", AuthSession::default());
    match handle.join().unwrap() {
        NetworkResult::Failure(_) => {},
        other => panic!("unexpected fetch result: {other:?}"),
    }
}

#[test]
fn fetch_fixture_errors_when_remote_path_disappears() {
    let dir = TestDir::new("fetch-missing-path");
    let source = init_repo_at(&dir.join("source"));
    let commit = commit_file(&source, "file.txt", "source\n", "source");
    create_branch(&source, "feature", commit);

    let remote_path = dir.join("remote.git");
    init_bare_repo_at(&remote_path);
    add_remote_path(&source, "origin", &remote_path);
    seed_remote(&source, "origin", &["refs/heads/feature:refs/heads/feature"]);

    let consumer = init_repo_at(&dir.join("consumer"));
    add_remote_path(&consumer, "origin", &remote_path);
    fs::remove_dir_all(&remote_path).unwrap();

    let handle = fetch_remote(consumer.workdir().unwrap().to_str().unwrap(), "origin", AuthSession::default());
    match handle.join().unwrap() {
        NetworkResult::Failure(_) => {},
        other => panic!("unexpected fetch result: {other:?}"),
    }
}

#[test]
fn fetch_fixture_supports_linked_worktree_paths() {
    let dir = TestDir::new("fetch-linked-worktree");
    let source = init_repo_at(&dir.join("source"));
    let commit = commit_file(&source, "file.txt", "source\n", "source");
    create_branch(&source, "feature", commit);

    let remote_path = dir.join("remote.git");
    init_bare_repo_at(&remote_path);
    add_remote_path(&source, "origin", &remote_path);
    seed_remote(&source, "origin", &["refs/heads/feature:refs/heads/feature"]);

    let consumer = init_repo_at(&dir.join("consumer"));
    let consumer_commit = commit_file(&consumer, "consumer.txt", "consumer\n", "consumer");
    add_remote_path(&consumer, "origin", &remote_path);
    let linked_path = dir.join("linked");
    create_worktree(&consumer, "linked", &linked_path, consumer_commit).unwrap();

    let handle = fetch_remote(linked_path.to_str().unwrap(), "origin", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    let linked_repo = Repository::open(&linked_path).unwrap();
    assert_eq!(linked_repo.find_reference("refs/remotes/origin/feature").unwrap().target(), Some(commit));
}

#[test]
fn delete_remote_branch_fixture_removes_remote_tracking_refs() {
    let dir = TestDir::new("delete-remote-branch");
    let source = init_repo_at(&dir.join("source"));
    let commit = commit_file(&source, "file.txt", "source\n", "source");
    create_branch(&source, "feature", commit);

    let remote_path = dir.join("remote.git");
    init_bare_repo_at(&remote_path);
    add_remote_path(&source, "origin", &remote_path);
    seed_remote(&source, "origin", &["refs/heads/feature:refs/heads/feature"]);

    let handle = delete_remote_branch(source.workdir().unwrap().to_str().unwrap(), "origin", "feature", AuthSession::default());
    assert!(matches!(handle.join().unwrap(), NetworkResult::Success));

    let remote = Repository::open(&remote_path).unwrap();
    assert!(remote.find_reference("refs/heads/feature").is_err());
}

#[test]
fn large_history_fixture_builds_a_long_commit_chain() {
    let dir = TestDir::new("history");
    let repo = init_repo_at(&dir.join("repo"));

    for idx in 0..64 {
        let contents = format!("line {idx}\n");
        commit_file(&repo, "history.txt", &contents, &format!("commit {idx}"));
    }

    let mut walk = repo.revwalk().unwrap();
    walk.push_head().unwrap();
    assert_eq!(walk.count(), 64);
}
