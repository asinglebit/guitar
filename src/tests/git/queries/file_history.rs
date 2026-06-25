use super::*;
use crate::{
    core::oids::git2_to_gix_oid,
    git::test_support::{TestDir, commit_file, commit_index, init_repo_at, stage_path, write_workdir_file},
};
use git2::{Oid, Repository};
use std::{fs, path::PathBuf};

fn temp_repo(name: &str) -> (TestDir, PathBuf, Repository) {
    let dir = TestDir::new(name);
    let path = dir.join("repo");
    let repo = init_repo_at(&path);
    (dir, path, repo)
}

fn file_status_from_repo(repo: &gix::Repository, oid: Oid, path: &str) -> Option<FileStatus> {
    super::changed_file_status_at_commit_from_repo(repo, git2_to_gix_oid(oid), path).unwrap()
}

#[test]
fn root_add_modify_delete_and_non_matching_commits_are_classified() {
    let (_dir, path, repo) = temp_repo("statuses");
    let root = commit_file(&repo, "tracked.txt", "one\n", "root");
    let other = commit_file(&repo, "other.txt", "other\n", "other");
    let modified = commit_file(&repo, "tracked.txt", "two\n", "modify");

    fs::remove_file(path.join("tracked.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(std::path::Path::new("tracked.txt")).unwrap();
    index.write().unwrap();
    let deleted = commit_index(&repo, "delete");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(file_status_from_repo(&gix_repo, root, "tracked.txt"), Some(FileStatus::Added));
    assert_eq!(file_status_from_repo(&gix_repo, other, "tracked.txt"), None);
    assert_eq!(file_status_from_repo(&gix_repo, modified, "tracked.txt"), Some(FileStatus::Modified));
    assert_eq!(file_status_from_repo(&gix_repo, deleted, "tracked.txt"), Some(FileStatus::Deleted));
}

#[test]
fn rename_matches_old_and_new_selected_path() {
    let (_dir, path, repo) = temp_repo("rename");
    commit_file(&repo, "old.txt", "one\n", "root");

    fs::rename(path.join("old.txt"), path.join("new.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(std::path::Path::new("old.txt")).unwrap();
    index.add_path(std::path::Path::new("new.txt")).unwrap();
    index.write().unwrap();
    let renamed = commit_index(&repo, "rename");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(file_status_from_repo(&gix_repo, renamed, "old.txt"), Some(FileStatus::Renamed));
    assert_eq!(file_status_from_repo(&gix_repo, renamed, "new.txt"), Some(FileStatus::Renamed));
}

#[test]
fn copied_file_stays_an_added_change_in_file_history() {
    let (_dir, path, repo) = temp_repo("copy");
    commit_file(&repo, "source.txt", "one\n", "root");

    write_workdir_file(&repo, "copy.txt", "one\n");
    stage_path(&repo, "copy.txt");
    let copied = commit_index(&repo, "copy");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(file_status_from_repo(&gix_repo, copied, "copy.txt"), Some(FileStatus::Added));
    assert_eq!(file_status_from_repo(&gix_repo, copied, "source.txt"), None);
}

#[cfg(unix)]
#[test]
fn typechange_is_reported_as_deleted_in_file_history() {
    let (_dir, path, repo) = temp_repo("typechange");
    commit_file(&repo, "link.txt", "one\n", "root");

    fs::remove_file(path.join("link.txt")).unwrap();
    std::os::unix::fs::symlink("target.txt", path.join("link.txt")).unwrap();
    stage_path(&repo, "link.txt");
    let typechanged = commit_index(&repo, "typechange");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(file_status_from_repo(&gix_repo, typechanged, "link.txt"), Some(FileStatus::Deleted));
}

#[test]
fn directory_like_file_names_remain_file_history_entries() {
    let (_dir, path, repo) = temp_repo("directory-like");
    commit_file(&repo, "docs/guide", "one\n", "root");

    write_workdir_file(&repo, "docs/guide", "two\n");
    stage_path(&repo, "docs/guide");
    let updated = commit_index(&repo, "update");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(file_status_from_repo(&gix_repo, updated, "docs/guide"), Some(FileStatus::Modified));
}

#[test]
fn empty_and_normalized_paths_match_plain_inputs() {
    let (_dir, path, repo) = temp_repo("normalize");
    let root = commit_file(&repo, "tracked.txt", "one\n", "root");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(file_status_from_repo(&gix_repo, root, ""), None);

    assert_eq!(file_status_from_repo(&gix_repo, root, "./tracked.txt"), Some(FileStatus::Added));

    assert_eq!(file_status_from_repo(&gix_repo, root, r".\tracked.txt"), Some(FileStatus::Added));
}
