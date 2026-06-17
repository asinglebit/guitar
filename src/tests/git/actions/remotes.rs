use super::*;
use crate::git::queries::remotes::{GUITAR_DEFAULT_REMOTE_CONFIG, PUSH_DEFAULT_CONFIG};
use git2::Repository;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (std::path::PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-remote-action-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    (path, repo)
}

#[test]
fn add_remote_creates_remote_with_url() {
    let (_path, repo) = temp_repo("add");

    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();

    assert_eq!(repo.find_remote("origin").unwrap().url(), Some("https://example.com/repo.git"));
}

#[test]
fn add_remote_rejects_invalid_duplicate_and_empty_values() {
    let (_path, repo) = temp_repo("invalid-add");
    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();

    assert!(add_remote(&repo, "", "https://example.com/other.git").is_err());
    assert!(add_remote(&repo, "bad\nname", "https://example.com/other.git").is_err());
    assert!(add_remote(&repo, "other", "").is_err());
    assert!(add_remote(&repo, "origin", "https://example.com/other.git").is_err());
}

#[test]
fn rename_remote_updates_remote_name() {
    let (_path, repo) = temp_repo("rename");
    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();

    rename_remote(&repo, "origin", "upstream").unwrap();

    assert!(repo.find_remote("origin").is_err());
    assert_eq!(repo.find_remote("upstream").unwrap().url(), Some("https://example.com/repo.git"));
}

#[test]
fn rename_remote_rejects_invalid_same_duplicate_and_missing_values() {
    let (_path, repo) = temp_repo("invalid-rename");
    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();
    add_remote(&repo, "upstream", "https://example.com/upstream.git").unwrap();

    assert!(rename_remote(&repo, "origin", "origin").is_err());
    assert!(rename_remote(&repo, "origin", "bad\nname").is_err());
    assert!(rename_remote(&repo, "origin", "upstream").is_err());
    assert!(rename_remote(&repo, "missing", "other").is_err());
}

#[test]
fn edit_fetch_and_push_urls() {
    let (_path, repo) = temp_repo("edit-url");
    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();

    set_remote_url(&repo, "origin", "https://example.com/renamed.git").unwrap();
    set_remote_push_url(&repo, "origin", Some("ssh://example.com/renamed.git")).unwrap();

    let remote = repo.find_remote("origin").unwrap();
    assert_eq!(remote.url(), Some("https://example.com/renamed.git"));
    assert_eq!(remote.pushurl(), Some("ssh://example.com/renamed.git"));
}

#[test]
fn edit_urls_reject_invalid_missing_and_empty_values() {
    let (_path, repo) = temp_repo("invalid-edit");
    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();

    assert!(set_remote_url(&repo, "origin", "").is_err());
    assert!(set_remote_url(&repo, "missing", "https://example.com/other.git").is_err());
    assert!(set_remote_push_url(&repo, "missing", Some("ssh://example.com/other.git")).is_err());
}

#[test]
fn empty_push_url_clears_dedicated_push_url() {
    let (_path, repo) = temp_repo("clear-push");
    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();
    set_remote_push_url(&repo, "origin", Some("ssh://example.com/repo.git")).unwrap();

    set_remote_push_url(&repo, "origin", Some("")).unwrap();

    assert_eq!(repo.find_remote("origin").unwrap().pushurl(), None);
}

#[test]
fn set_default_remote_writes_guitar_and_push_default_config() {
    let (_path, repo) = temp_repo("set-default");
    add_remote(&repo, "upstream", "https://example.com/repo.git").unwrap();

    set_default_remote(&repo, "upstream").unwrap();

    let config = repo.config().unwrap();
    assert_eq!(config.get_string(GUITAR_DEFAULT_REMOTE_CONFIG).unwrap(), "upstream");
    assert_eq!(config.get_string(PUSH_DEFAULT_CONFIG).unwrap(), "upstream");
}

#[test]
fn set_default_remote_rejects_missing_remote() {
    let (_path, repo) = temp_repo("set-default-missing");

    assert!(set_default_remote(&repo, "missing").is_err());
}

#[test]
fn rename_remote_preserves_matching_default_config() {
    let (_path, repo) = temp_repo("rename-default");
    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();
    set_default_remote(&repo, "origin").unwrap();

    rename_remote(&repo, "origin", "upstream").unwrap();

    let config = repo.config().unwrap();
    assert_eq!(config.get_string(GUITAR_DEFAULT_REMOTE_CONFIG).unwrap(), "upstream");
    assert_eq!(config.get_string(PUSH_DEFAULT_CONFIG).unwrap(), "upstream");
}

#[test]
fn delete_remote_removes_remote() {
    let (_path, repo) = temp_repo("delete");
    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();

    delete_remote(&repo, "origin").unwrap();

    assert!(repo.find_remote("origin").is_err());
}

#[test]
fn delete_remote_clears_matching_default_config() {
    let (_path, repo) = temp_repo("delete-default");
    add_remote(&repo, "origin", "https://example.com/repo.git").unwrap();
    set_default_remote(&repo, "origin").unwrap();

    delete_remote(&repo, "origin").unwrap();

    let config = repo.config().unwrap();
    assert!(config.get_string(GUITAR_DEFAULT_REMOTE_CONFIG).is_err());
    assert!(config.get_string(PUSH_DEFAULT_CONFIG).is_err());
}

#[test]
fn delete_remote_rejects_invalid_and_missing_names() {
    let (_path, repo) = temp_repo("invalid-delete");

    assert!(delete_remote(&repo, "").is_err());
    assert!(delete_remote(&repo, "bad\nname").is_err());
    assert!(delete_remote(&repo, "missing").is_err());
}
