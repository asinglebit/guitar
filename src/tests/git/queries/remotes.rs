use super::*;
use crate::git::test_support::{TestDir, commit_file, temp_repo};
use git2::{BranchType, Repository};

fn set_current_branch_upstream(repo: &Repository, remote: &str) {
    let oid = commit_file(repo, "file.txt", "content\n", "commit");
    let current_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    repo.reference(&format!("refs/remotes/{remote}/{current_branch}"), oid, true, "remote").unwrap();
    repo.find_branch(&current_branch, BranchType::Local).unwrap().set_upstream(Some(&format!("{remote}/{current_branch}"))).unwrap();
}

#[test]
fn list_remotes_returns_sorted_names_and_urls() {
    let (dir, repo) = temp_repo("list");
    let path = dir.join("repo");
    repo.remote("zeta", "https://example.com/zeta.git").unwrap();
    repo.remote("alpha", "https://example.com/alpha.git").unwrap();
    repo.remote_set_pushurl("alpha", Some("ssh://example.com/alpha.git")).unwrap();

    let remotes = list_remotes(path.as_path()).unwrap();

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
    let (dir, _repo) = temp_repo("empty");
    let path = dir.join("repo");

    assert!(list_remotes(path.as_path()).unwrap().is_empty());
}

#[test]
fn list_remotes_handles_bare_repo_paths() {
    let dir = TestDir::new("bare-remotes");
    let path = dir.join("repo.git");
    let repo = Repository::init_bare(&path).unwrap();
    repo.remote("origin", "https://example.com/origin.git").unwrap();

    let remotes = list_remotes(repo.path()).unwrap();

    assert_eq!(remotes, vec![RemoteEntry { name: "origin".to_string(), url: "https://example.com/origin.git".to_string(), push_url: None }]);
}

#[test]
fn effective_default_remote_prefers_origin_before_first_sorted_remote() {
    let (dir, repo) = temp_repo("origin-fallback");
    let path = dir.join("repo");
    repo.remote("zeta", "https://example.com/zeta.git").unwrap();
    repo.remote("origin", "https://example.com/origin.git").unwrap();

    assert_eq!(effective_default_remote(path.as_path()).as_deref(), Some("origin"));
}

#[test]
fn effective_default_remote_uses_config_precedence() {
    let (dir, repo) = temp_repo("default-precedence");
    let path = dir.join("repo");
    repo.remote("origin", "https://example.com/origin.git").unwrap();
    repo.remote("upstream", "https://example.com/upstream.git").unwrap();
    set_current_branch_upstream(&repo, "upstream");

    assert_eq!(effective_default_remote(path.as_path()).as_deref(), Some("upstream"));

    {
        let mut config = repo.config().unwrap();
        config.set_str(PUSH_DEFAULT_CONFIG, "origin").unwrap();
    }
    assert_eq!(effective_default_remote(path.as_path()).as_deref(), Some("origin"));

    {
        let mut config = repo.config().unwrap();
        config.set_str(GUITAR_DEFAULT_REMOTE_CONFIG, "upstream").unwrap();
    }
    assert_eq!(effective_default_remote(path.as_path()).as_deref(), Some("upstream"));
}

#[test]
fn effective_default_remote_from_remotes_matches_full_resolution() {
    let (dir, repo) = temp_repo("default-from-remotes");
    let path = dir.join("repo");
    repo.remote("origin", "https://example.com/origin.git").unwrap();
    repo.remote("upstream", "https://example.com/upstream.git").unwrap();
    set_current_branch_upstream(&repo, "upstream");

    let remotes = list_remotes(path.as_path()).unwrap();

    assert_eq!(effective_default_remote_from_remotes(path.as_path(), &remotes), effective_default_remote(path.as_path()));
}

#[test]
fn effective_default_remote_ignores_stale_config_and_falls_back() {
    let (dir, repo) = temp_repo("default-stale");
    let path = dir.join("repo");
    repo.remote("zeta", "https://example.com/zeta.git").unwrap();
    repo.remote("alpha", "https://example.com/alpha.git").unwrap();

    {
        let mut config = repo.config().unwrap();
        config.set_str(GUITAR_DEFAULT_REMOTE_CONFIG, "missing").unwrap();
        config.set_str(PUSH_DEFAULT_CONFIG, "also-missing").unwrap();
    }

    assert_eq!(effective_default_remote(path.as_path()).as_deref(), Some("alpha"));
}
