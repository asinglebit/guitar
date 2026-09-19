use super::*;
use git2::{Repository, Signature};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn commit_file(repo: &Repository, path: &Path, name: &str, contents: &str, message: &str) {
    fs::write(path.join(name), contents).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(name)).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let signature = Signature::now("Test User", "test@example.com").unwrap();
    let parents = match repo.head().ok().and_then(|head| head.target()) {
        Some(oid) => vec![repo.find_commit(oid).unwrap()],
        None => Vec::new(),
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parent_refs).unwrap();
}

// An upstream repository plus a local one whose "origin" points at it over a plain path, so the
// fetch needs no network and no credentials.
fn linked_repos(name: &str) -> (PathBuf, Repository, PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let base = std::env::temp_dir().join(format!("guitar-fetching-{name}-{id}"));
    let upstream_path = base.join("upstream");
    let local_path = base.join("local");
    fs::create_dir_all(&upstream_path).unwrap();
    fs::create_dir_all(&local_path).unwrap();

    let upstream = Repository::init(&upstream_path).unwrap();
    commit_file(&upstream, &upstream_path, "file.txt", "one\n", "first");

    let local = Repository::init(&local_path).unwrap();
    local.remote("origin", upstream_path.to_str().unwrap()).unwrap();

    (upstream_path, upstream, local_path, local)
}

#[test]
fn quiet_fetch_brings_refs_in_and_reports_the_change() {
    let (upstream_path, upstream, local_path, local) = linked_repos("changed");

    let outcome = fetch_remote_quiet(local_path.to_str().unwrap(), "origin", AuthSession::default()).join().unwrap();

    assert!(outcome.ok, "a plain path remote should fetch without credentials");
    assert!(outcome.changed, "the first fetch brings refs in, so it changed something");
    assert!(local.find_reference("refs/remotes/origin/master").is_ok() || local.find_reference("refs/remotes/origin/main").is_ok());

    // A second fetch with nothing new upstream must report no change, so the app stays still.
    let outcome = fetch_remote_quiet(local_path.to_str().unwrap(), "origin", AuthSession::default()).join().unwrap();
    assert!(outcome.ok);
    assert!(!outcome.changed, "an unchanged remote must not trigger a reload");

    // A new upstream commit moves the remote ref, which must be reported.
    commit_file(&upstream, &upstream_path, "file.txt", "two\n", "second");
    let outcome = fetch_remote_quiet(local_path.to_str().unwrap(), "origin", AuthSession::default()).join().unwrap();
    assert!(outcome.ok);
    assert!(outcome.changed, "a moved upstream ref must be reported as a change");

    fs::remove_dir_all(upstream_path.parent().unwrap()).ok();
}

#[test]
fn quiet_fetch_reports_failure_instead_of_prompting() {
    let (upstream_path, _upstream, local_path, local) = linked_repos("missing");
    local.remote_set_url("origin", "/guitar/definitely/not/a/repository").unwrap();

    let outcome = fetch_remote_quiet(local_path.to_str().unwrap(), "origin", AuthSession::default()).join().unwrap();

    assert!(!outcome.ok, "a broken remote must come back as a failure the app can back off on");
    assert!(!outcome.changed);

    fs::remove_dir_all(upstream_path.parent().unwrap()).ok();
}
