use crate::core::oids::git2_to_gix_oid;
use crate::git::queries::reflogs::get_head_reflog_entries;
use crate::git::test_support::{commit_file, temp_repo};
use git2::{Oid, Repository, ResetType};
use std::{fs, io::Write};

fn append_reflog_entry(repo: &Repository, new_oid: Oid, message: &str) {
    let log_path = repo.path().join("logs/HEAD");
    let mut log = fs::OpenOptions::new().append(true).open(log_path).unwrap();
    writeln!(log, "{} {} Skip <skip@example.com> 0 +0000\t{}", Oid::zero(), new_oid, message).unwrap();
}

#[test]
fn head_reflog_skips_entries_that_no_longer_point_to_commits() {
    let (dir, repo) = temp_repo("skip-non-commit");
    let path = dir.join("repo");
    let base = commit_file(&repo, "file.txt", "base", "base");
    let lost = commit_file(&repo, "file.txt", "lost", "lost");
    let base_commit = repo.find_commit(base).unwrap();
    repo.reset(base_commit.as_object(), ResetType::Hard, None).unwrap();

    let skipped_oid = repo.blob(b"skip-me").unwrap();
    append_reflog_entry(&repo, skipped_oid, "skip-me");

    let gix_repo = gix::open(path).unwrap();
    let entries = get_head_reflog_entries(&gix_repo).unwrap();

    assert!(entries.iter().any(|entry| entry.new_oid == git2_to_gix_oid(lost)));
    assert!(!entries.iter().any(|entry| entry.message == "skip-me"));
    assert_eq!(entries.first().map(|entry| entry.selector.as_str()), Some("HEAD@{1}"));
    assert_eq!(repo.head().unwrap().target(), Some(base));
}

#[test]
fn missing_head_reflog_returns_an_error() {
    let (dir, _repo) = temp_repo("missing");
    let gix_repo = gix::open(dir.join("repo")).unwrap();
    assert!(get_head_reflog_entries(&gix_repo).is_err());
}
