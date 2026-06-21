use super::*;
use git2::{BranchType, Repository, Signature};
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (std::path::PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-remote-query-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    (path, repo)
}

fn commit(repo: &Repository, file: &str) -> git2::Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), "content\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &parents).unwrap()
}

#[test]
fn list_remotes_returns_sorted_names_and_urls() {
    let (_path, repo) = temp_repo("list");
    repo.remote("zeta", "https://example.com/zeta.git").unwrap();
    repo.remote("alpha", "https://example.com/alpha.git").unwrap();
    repo.remote_set_pushurl("alpha", Some("ssh://example.com/alpha.git")).unwrap();

    let remotes = list_remotes(&repo).unwrap();

    assert_eq!(remotes.len(), 2);
    assert_eq!(remotes[0].name, "alpha");
    assert_eq!(remotes[0].url, "https://example.com/alpha.git");
    assert_eq!(remotes[0].push_url.as_deref(), Some("ssh://example.com/alpha.git"));
    assert_eq!(remotes[1].name, "zeta");
    assert_eq!(remotes[1].url, "https://example.com/zeta.git");
    assert_eq!(remotes[1].push_url, None);
}

#[test]
fn list_remotes_returns_empty_for_repo_without_remotes() {
    let (_path, repo) = temp_repo("empty");

    assert!(list_remotes(&repo).unwrap().is_empty());
}

#[test]
fn effective_default_remote_uses_config_precedence() {
    let (_path, repo) = temp_repo("default-precedence");
    repo.remote("origin", "https://example.com/origin.git").unwrap();
    repo.remote("upstream", "https://example.com/upstream.git").unwrap();
    let oid = commit(&repo, "file.txt");
    let current_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    repo.reference(&format!("refs/remotes/upstream/{current_branch}"), oid, true, "remote").unwrap();
    repo.find_branch(&current_branch, BranchType::Local).unwrap().set_upstream(Some(&format!("upstream/{current_branch}"))).unwrap();

    assert_eq!(effective_default_remote(&repo).as_deref(), Some("upstream"));

    {
        let mut config = repo.config().unwrap();
        config.set_str(PUSH_DEFAULT_CONFIG, "origin").unwrap();
    }
    assert_eq!(effective_default_remote(&repo).as_deref(), Some("origin"));

    {
        let mut config = repo.config().unwrap();
        config.set_str(GUITAR_DEFAULT_REMOTE_CONFIG, "upstream").unwrap();
    }
    assert_eq!(effective_default_remote(&repo).as_deref(), Some("upstream"));
}

#[test]
fn effective_default_remote_ignores_stale_config_and_falls_back() {
    let (_path, repo) = temp_repo("default-stale");
    repo.remote("zeta", "https://example.com/zeta.git").unwrap();
    repo.remote("alpha", "https://example.com/alpha.git").unwrap();

    {
        let mut config = repo.config().unwrap();
        config.set_str(GUITAR_DEFAULT_REMOTE_CONFIG, "missing").unwrap();
        config.set_str(PUSH_DEFAULT_CONFIG, "also-missing").unwrap();
    }

    assert_eq!(effective_default_remote(&repo).as_deref(), Some("alpha"));
}
