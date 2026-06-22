use super::*;
use git2::{Oid, Repository, Signature};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-file-history-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn write_file(root: &Path, file: &str, content: &str) {
    let path = root.join(file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn commit_index(repo: &Repository, message: &str) -> Oid {
    let mut index = repo.index().unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}

fn commit_file(repo: &Repository, root: &Path, file: &str, content: &str, message: &str) -> Oid {
    write_file(root, file, content);
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    commit_index(repo, message)
}

fn gix_file_status(repo: &gix::Repository, oid: Oid, path: &str) -> Option<FileStatus> {
    super::changed_file_status_at_commit_gix(repo, gix::ObjectId::from_bytes_or_panic(oid.as_bytes()), path).unwrap()
}

#[test]
fn root_add_modify_delete_and_non_matching_commits_are_classified() {
    let (path, repo) = temp_repo("statuses");
    let root = commit_file(&repo, &path, "tracked.txt", "one\n", "root");
    let other = commit_file(&repo, &path, "other.txt", "other\n", "other");
    let modified = commit_file(&repo, &path, "tracked.txt", "two\n", "modify");

    fs::remove_file(path.join("tracked.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("tracked.txt")).unwrap();
    let deleted = commit_index(&repo, "delete");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(gix_file_status(&gix_repo, root, "tracked.txt"), Some(FileStatus::Added));
    assert_eq!(gix_file_status(&gix_repo, other, "tracked.txt"), None);
    assert_eq!(gix_file_status(&gix_repo, modified, "tracked.txt"), Some(FileStatus::Modified));
    assert_eq!(gix_file_status(&gix_repo, deleted, "tracked.txt"), Some(FileStatus::Deleted));
}

#[test]
fn rename_matches_old_and_new_selected_path() {
    let (path, repo) = temp_repo("rename");
    commit_file(&repo, &path, "old.txt", "one\n", "root");

    fs::rename(path.join("old.txt"), path.join("new.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("old.txt")).unwrap();
    index.add_path(Path::new("new.txt")).unwrap();
    let renamed = commit_index(&repo, "rename");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(gix_file_status(&gix_repo, renamed, "old.txt"), Some(FileStatus::Renamed));
    assert_eq!(gix_file_status(&gix_repo, renamed, "new.txt"), Some(FileStatus::Renamed));
}

#[test]
fn copied_file_stays_an_added_change_in_file_history() {
    let (path, repo) = temp_repo("copy");
    commit_file(&repo, &path, "source.txt", "one\n", "root");

    write_file(&path, "copy.txt", "one\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("copy.txt")).unwrap();
    index.write().unwrap();
    let copied = commit_index(&repo, "copy");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(gix_file_status(&gix_repo, copied, "copy.txt"), Some(FileStatus::Added));
    assert_eq!(gix_file_status(&gix_repo, copied, "source.txt"), None);
}

#[cfg(unix)]
#[test]
fn typechange_is_reported_as_deleted_in_file_history() {
    let (path, repo) = temp_repo("typechange");
    commit_file(&repo, &path, "link.txt", "one\n", "root");

    fs::remove_file(path.join("link.txt")).unwrap();
    std::os::unix::fs::symlink("target.txt", path.join("link.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("link.txt")).unwrap();
    index.write().unwrap();
    let typechanged = commit_index(&repo, "typechange");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(gix_file_status(&gix_repo, typechanged, "link.txt"), Some(FileStatus::Deleted));
}

#[test]
fn directory_like_file_names_remain_file_history_entries() {
    let (path, repo) = temp_repo("directory-like");
    commit_file(&repo, &path, "docs/guide", "one\n", "root");

    write_file(&path, "docs/guide", "two\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("docs/guide")).unwrap();
    index.write().unwrap();
    let updated = commit_index(&repo, "update");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(gix_file_status(&gix_repo, updated, "docs/guide"), Some(FileStatus::Modified));
}

#[test]
fn empty_and_normalized_paths_match_plain_inputs() {
    let (path, repo) = temp_repo("normalize");
    let root = commit_file(&repo, &path, "tracked.txt", "one\n", "root");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(gix_file_status(&gix_repo, root, ""), None);

    assert_eq!(gix_file_status(&gix_repo, root, "./tracked.txt"), Some(FileStatus::Added));

    assert_eq!(gix_file_status(&gix_repo, root, r".\tracked.txt"), Some(FileStatus::Added));
}
