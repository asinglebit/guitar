use super::*;
use git2::{Repository, Signature};
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-cherrypick-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = crate::git::test_support::init_repo_at(&path);
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
fn clean_cherrypick_commits_with_edited_message() {
    let (path, repo) = temp_repo("clean");
    write(&path, "base.txt", "base\n");
    commit(&repo, "base.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "feature.txt", "feature\n");
    let feature = commit(&repo, "feature.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "main.txt", "main\n");
    let main = commit(&repo, "main.txt", "main");

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
    let _ = fs::remove_dir_all(path);
}

#[test]
fn conflict_then_continue_commits_with_persisted_message() {
    let (path, repo) = temp_repo("conflict-continue");
    write(&path, "file.txt", "base\n");
    commit(&repo, "file.txt", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    write(&path, "file.txt", "feature\n");
    let feature = commit(&repo, "file.txt", "feature");
    checkout_branch(&repo, &base_branch);
    write(&path, "file.txt", "main\n");
    commit(&repo, "file.txt", "main");

    assert_eq!(start_cherrypick(&repo, feature, "cherrypicked: feature").unwrap(), CherrypickOutcome::Conflict);
    assert!(is_cherrypick_in_progress(&repo));
    assert!(repo.index().unwrap().has_conflicts());
    assert_eq!(continue_cherrypick(&repo).unwrap(), CherrypickOutcome::Conflict);

    write(&path, "file.txt", "resolved\n");
    let outcome = continue_cherrypick(&repo).unwrap();
    let CherrypickOutcome::Committed { oid } = outcome else {
        panic!("expected committed outcome");
    };

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id(), oid);
    assert_eq!(head.summary(), Some("cherrypicked: feature"));
    assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "resolved\n");
    assert!(!is_cherrypick_in_progress(&repo));
    assert!(!message_path(&repo).exists());
    let _ = fs::remove_dir_all(path);
}

#[test]
fn abort_restores_pre_cherrypick_state() {
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

    assert_eq!(start_cherrypick(&repo, feature, "cherrypicked: feature").unwrap(), CherrypickOutcome::Conflict);
    assert_eq!(abort_cherrypick(&repo).unwrap(), CherrypickOutcome::Aborted);
    assert!(!is_cherrypick_in_progress(&repo));
    assert_eq!(repo.head().unwrap().target(), Some(main));
    assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "main\n");
    assert!(!message_path(&repo).exists());
    let _ = fs::remove_dir_all(path);
}
