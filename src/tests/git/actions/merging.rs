use super::*;
use git2::{Repository, Signature, build::CheckoutBuilder};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-merge-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn write(path: &Path, file: &str, content: &str) {
    fs::write(path.join(file), content).unwrap();
}

fn commit(repo: &Repository, file: &str, message: &str) -> Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap();
    assert!(workdir.join(file).exists());
    oid
}

fn checkout_new_branch(repo: &Repository, name: &str) {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch(name, &head, false).unwrap();
    repo.set_head(&format!("refs/heads/{name}")).unwrap();
    repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
}

fn checkout_branch(repo: &Repository, name: &str) {
    repo.set_head(&format!("refs/heads/{name}")).unwrap();
    repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
}

#[test]
fn fast_forward_updates_branch_and_workdir() {
    let (path, repo) = temp_repo("fast-forward");
    write(&path, "file.txt", "base\n");
    commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "file.txt", "feature\n");
    let feature = commit(&repo, "file.txt", "feature");
    checkout_branch(&repo, &base_branch);

    assert_eq!(start_merge(&repo, feature).unwrap(), MergeOutcome::FastForward { oid: feature });
    assert_eq!(repo.head().unwrap().target(), Some(feature));
    assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "feature\n");
    assert!(!is_merge_in_progress(&repo));
    let _ = fs::remove_dir_all(path);
}

#[test]
fn up_to_date_merge_does_nothing() {
    let (path, repo) = temp_repo("up-to-date");
    write(&path, "file.txt", "base\n");
    let base = commit(&repo, "file.txt", "base");

    assert_eq!(start_merge(&repo, base).unwrap(), MergeOutcome::UpToDate);
    assert_eq!(repo.head().unwrap().target(), Some(base));
    assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "base\n");
    assert!(!is_merge_in_progress(&repo));
    let _ = fs::remove_dir_all(path);
}

#[test]
fn divergent_clean_merge_creates_two_parent_commit() {
    let (path, repo) = temp_repo("clean-divergent");
    write(&path, "base.txt", "base\n");
    commit(&repo, "base.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "feature.txt", "feature\n");
    let feature = commit(&repo, "feature.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "main.txt", "main\n");
    let main = commit(&repo, "main.txt", "main");

    let MergeOutcome::Completed { oid } = start_merge(&repo, feature).unwrap() else {
        panic!("expected completed merge");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.parent_count(), 2);
    assert_eq!(head.parent(0).unwrap().id(), main);
    assert_eq!(head.parent(1).unwrap().id(), feature);
    assert_eq!(fs::read_to_string(path.join("feature.txt")).unwrap(), "feature\n");
    assert_eq!(fs::read_to_string(path.join("main.txt")).unwrap(), "main\n");
    assert!(!is_merge_in_progress(&repo));
    let _ = fs::remove_dir_all(path);
}

#[test]
fn conflict_then_continue_finishes_after_workdir_resolution() {
    let (path, repo) = temp_repo("conflict-continue");
    write(&path, "file.txt", "base\n");
    commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "file.txt", "feature\n");
    let feature = commit(&repo, "file.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "file.txt", "main\n");
    let main = commit(&repo, "file.txt", "main");

    assert_eq!(start_merge(&repo, feature).unwrap(), MergeOutcome::Conflict);
    assert!(is_merge_in_progress(&repo));
    assert!(repo.index().unwrap().has_conflicts());
    assert_eq!(continue_merge(&repo).unwrap(), MergeOutcome::Conflict);

    write(&path, "file.txt", "resolved\n");
    let MergeOutcome::Completed { oid } = continue_merge(&repo).unwrap() else {
        panic!("expected completed merge");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.parent_count(), 2);
    assert_eq!(head.parent(0).unwrap().id(), main);
    assert_eq!(head.parent(1).unwrap().id(), feature);
    assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "resolved\n");
    assert!(!is_merge_in_progress(&repo));
    let _ = fs::remove_dir_all(path);
}

#[test]
fn abort_restores_pre_merge_state() {
    let (path, repo) = temp_repo("abort");
    write(&path, "file.txt", "base\n");
    commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "file.txt", "feature\n");
    let feature = commit(&repo, "file.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "file.txt", "main\n");
    let main = commit(&repo, "file.txt", "main");

    assert_eq!(start_merge(&repo, feature).unwrap(), MergeOutcome::Conflict);
    assert_eq!(abort_merge(&repo).unwrap(), MergeOutcome::Aborted);
    assert!(!is_merge_in_progress(&repo));
    assert_eq!(repo.head().unwrap().target(), Some(main));
    assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "main\n");
    let _ = fs::remove_dir_all(path);
}

#[test]
fn dirty_worktree_is_refused_before_start() {
    let (path, repo) = temp_repo("dirty");
    write(&path, "file.txt", "base\n");
    commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "feature.txt", "feature\n");
    let feature = commit(&repo, "feature.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "file.txt", "dirty\n");

    let error = start_merge(&repo, feature).unwrap_err();
    assert!(error.message().contains("working tree must be clean"));
    let _ = fs::remove_dir_all(path);
}

#[test]
fn merge_ff_false_creates_merge_commit_for_fast_forward() {
    let (path, repo) = temp_repo("no-ff");
    write(&path, "file.txt", "base\n");
    let base = commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "file.txt", "feature\n");
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
    assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "feature\n");
    let _ = fs::remove_dir_all(path);
}

#[test]
fn merge_ff_only_refuses_divergent_history() {
    let (path, repo) = temp_repo("ff-only");
    write(&path, "base.txt", "base\n");
    commit(&repo, "base.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "feature.txt", "feature\n");
    let feature = commit(&repo, "feature.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "main.txt", "main\n");
    let main = commit(&repo, "main.txt", "main");
    repo.config().unwrap().set_str("merge.ff", "only").unwrap();

    let error = start_merge(&repo, feature).unwrap_err();
    assert!(error.message().contains("merge.ff=only"));
    assert_eq!(repo.head().unwrap().target(), Some(main));
    assert!(!is_merge_in_progress(&repo));
    let _ = fs::remove_dir_all(path);
}
