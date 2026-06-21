use super::*;
use crate::git::queries::commits::get_stashed_commits;
use git2::{Repository, Signature};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-stash-actions-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn commit(repo: &Repository, file: &str, message: &str) -> git2::Oid {
    commit_with_content(repo, file, "content\n", message)
}

fn commit_with_content(repo: &Repository, file: &str, content: &str, message: &str) -> git2::Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), content).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}

#[test]
fn stash_message_uses_head_short_sha_and_summary_and_includes_untracked_files() {
    let (_path, mut repo) = temp_repo("message");
    let oid = commit(&repo, "file.txt", "base summary");
    fs::write(repo.workdir().unwrap().join("extra.txt"), "untracked\n").unwrap();
    fs::write(repo.workdir().unwrap().join("file.txt"), "dirty\n").unwrap();

    let stash_oid = stash(&mut repo).unwrap();
    let short_id = oid.to_string()[..7].to_string();
    let summary = {
        let stash_commit = repo.find_commit(stash_oid).unwrap();
        stash_commit.summary().unwrap().to_string()
    };

    assert!(summary.contains(&short_id), "stash summary should include the HEAD short SHA");
    assert!(summary.contains("base summary"), "stash summary should include the commit summary");
    assert_eq!(fs::read_to_string(repo.workdir().unwrap().join("file.txt")).unwrap(), "content\n");
    assert!(!repo.workdir().unwrap().join("extra.txt").exists());
    let mut oids = crate::core::oids::Oids::default();
    assert_eq!(get_stashed_commits(&mut repo, &mut oids).len(), 1);
}

#[test]
fn pop_without_applying_drops_stash_without_restoring_changes() {
    let (_path, mut repo) = temp_repo("drop-only");
    commit(&repo, "file.txt", "base summary");
    fs::write(repo.workdir().unwrap().join("file.txt"), "dirty\n").unwrap();
    fs::write(repo.workdir().unwrap().join("extra.txt"), "untracked\n").unwrap();

    let stash_oid = stash(&mut repo).unwrap();
    pop(&mut repo, &stash_oid, false).unwrap();

    let mut oids = crate::core::oids::Oids::default();
    assert!(get_stashed_commits(&mut repo, &mut oids).is_empty());
    assert_eq!(fs::read_to_string(repo.workdir().unwrap().join("file.txt")).unwrap(), "content\n");
    assert!(!repo.workdir().unwrap().join("extra.txt").exists());
}

#[test]
fn pop_with_apply_leaves_conflicts_and_drops_the_stash() {
    let (_path, mut repo) = temp_repo("conflict");
    commit(&repo, "file.txt", "base summary");
    fs::write(repo.workdir().unwrap().join("file.txt"), "ours\n").unwrap();

    let stash_oid = stash(&mut repo).unwrap();
    commit_with_content(&repo, "file.txt", "theirs\n", "conflict base");

    assert!(pop(&mut repo, &stash_oid, true).is_ok());
    assert!(repo.index().unwrap().has_conflicts());
    let mut oids = crate::core::oids::Oids::default();
    assert!(get_stashed_commits(&mut repo, &mut oids).is_empty());
}
