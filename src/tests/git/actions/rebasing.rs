use super::*;
use git2::{Repository, Signature};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-rebase-{name}-{id}"));
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
fn clean_rebase_completes_and_updates_branch() {
    let (path, repo) = temp_repo("clean");
    write(&path, "file.txt", "base\n");
    let base = commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "feature.txt", "feature\n");
    commit(&repo, "feature.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "main.txt", "main\n");
    let main = commit(&repo, "main.txt", "main");
    checkout_branch(&repo, "feature");

    let outcome = start_rebase(&repo, main).unwrap();
    assert_eq!(outcome, RebaseOutcome::Completed { applied: 1 });
    assert_eq!(repo.head().unwrap().shorthand(), Some("feature"));
    assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().parent(0).unwrap().id(), main);
    assert_ne!(repo.head().unwrap().target(), Some(base));
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
    commit(&repo, "feature.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "main.txt", "main\n");
    let main = commit(&repo, "main.txt", "main");
    checkout_branch(&repo, "feature");
    write(&path, "file.txt", "dirty\n");

    let error = start_rebase(&repo, main).unwrap_err();
    assert!(error.message().contains("working tree must be clean"));
    let _ = fs::remove_dir_all(path);
}

#[test]
fn conflict_then_continue_finishes() {
    let (path, repo) = temp_repo("conflict-continue");
    write(&path, "file.txt", "base\n");
    commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "file.txt", "feature\n");
    commit(&repo, "file.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "file.txt", "main\n");
    let main = commit(&repo, "file.txt", "main");
    checkout_branch(&repo, "feature");

    assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);
    assert!(is_rebase_in_progress(&repo));
    assert!(repo.index().unwrap().has_conflicts());

    write(&path, "file.txt", "resolved\n");
    assert_eq!(continue_rebase(&repo).unwrap(), RebaseOutcome::Completed { applied: 1 });
    assert!(!is_rebase_in_progress(&repo));
    assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "resolved\n");
    let _ = fs::remove_dir_all(path);
}

#[test]
fn abort_restores_pre_rebase_state() {
    let (path, repo) = temp_repo("abort");
    write(&path, "file.txt", "base\n");
    commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "file.txt", "feature\n");
    let original_feature = commit(&repo, "file.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "file.txt", "main\n");
    let main = commit(&repo, "file.txt", "main");
    checkout_branch(&repo, "feature");

    assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);
    assert_eq!(abort_rebase(&repo).unwrap(), RebaseOutcome::Aborted);
    assert!(!is_rebase_in_progress(&repo));
    assert_eq!(repo.head().unwrap().target(), Some(original_feature));
    assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "feature\n");
    let _ = fs::remove_dir_all(path);
}
