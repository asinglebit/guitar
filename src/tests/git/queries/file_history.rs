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

fn git2_file_status(repo: &Repository, oid: Oid, path: &str) -> Option<FileStatus> {
    super::changed_file_status_at_commit_git2(repo, oid, path).unwrap()
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

    assert_eq!(git2_file_status(&repo, root, "tracked.txt"), Some(FileStatus::Added));
    assert_eq!(git2_file_status(&repo, other, "tracked.txt"), None);
    assert_eq!(git2_file_status(&repo, modified, "tracked.txt"), Some(FileStatus::Modified));
    assert_eq!(git2_file_status(&repo, deleted, "tracked.txt"), Some(FileStatus::Deleted));
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

    assert_eq!(git2_file_status(&repo, renamed, "old.txt"), Some(FileStatus::Renamed));
    assert_eq!(git2_file_status(&repo, renamed, "new.txt"), Some(FileStatus::Renamed));
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

    assert_eq!(git2_file_status(&repo, copied, "copy.txt"), Some(FileStatus::Added));
    assert_eq!(git2_file_status(&repo, copied, "source.txt"), None);
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

    assert_eq!(git2_file_status(&repo, typechanged, "link.txt"), Some(FileStatus::Deleted));
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

    assert_eq!(git2_file_status(&repo, updated, "docs/guide"), Some(FileStatus::Modified));
}

#[test]
fn empty_and_normalized_paths_match_plain_inputs() {
    let (path, repo) = temp_repo("normalize");
    let root = commit_file(&repo, &path, "tracked.txt", "one\n", "root");
    let gix_repo = gix::open(&path).unwrap();

    assert_eq!(git2_file_status(&repo, root, ""), None);
    assert_eq!(gix_file_status(&gix_repo, root, ""), None);

    assert_eq!(git2_file_status(&repo, root, "./tracked.txt"), Some(FileStatus::Added));
    assert_eq!(gix_file_status(&gix_repo, root, "./tracked.txt"), Some(FileStatus::Added));

    assert_eq!(git2_file_status(&repo, root, r".\tracked.txt"), Some(FileStatus::Added));
    assert_eq!(gix_file_status(&gix_repo, root, r".\tracked.txt"), Some(FileStatus::Added));
}

#[test]
fn gitoxide_matches_libgit2_file_history_statuses_for_current_cases() {
    let (path, repo) = temp_repo("parity");
    let root = commit_file(&repo, &path, "tracked.txt", "one\n", "root");
    let other = commit_file(&repo, &path, "other.txt", "other\n", "other");
    let modified = commit_file(&repo, &path, "tracked.txt", "two\n", "modify");

    fs::remove_file(path.join("tracked.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("tracked.txt")).unwrap();
    let deleted = commit_index(&repo, "delete");

    let gix_repo = gix::open(&path).unwrap();

    let libgit2_root = git2_file_status(&repo, root, "tracked.txt");
    let gix_root = gix_file_status(&gix_repo, root, "tracked.txt");
    assert_eq!(libgit2_root, gix_root);
    assert_eq!(gix_root, Some(FileStatus::Added));

    let libgit2_other = git2_file_status(&repo, other, "tracked.txt");
    let gix_other = gix_file_status(&gix_repo, other, "tracked.txt");
    assert_eq!(libgit2_other, gix_other);
    assert_eq!(gix_other, None);

    let libgit2_modified = git2_file_status(&repo, modified, "tracked.txt");
    let gix_modified = gix_file_status(&gix_repo, modified, "tracked.txt");
    assert_eq!(libgit2_modified, gix_modified);
    assert_eq!(gix_modified, Some(FileStatus::Modified));

    let libgit2_deleted = git2_file_status(&repo, deleted, "tracked.txt");
    let gix_deleted = gix_file_status(&gix_repo, deleted, "tracked.txt");
    assert_eq!(libgit2_deleted, gix_deleted);
    assert_eq!(gix_deleted, Some(FileStatus::Deleted));

    let (path, repo) = temp_repo("parity-rename");
    commit_file(&repo, &path, "old.txt", "one\n", "root");
    fs::rename(path.join("old.txt"), path.join("new.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("old.txt")).unwrap();
    index.add_path(Path::new("new.txt")).unwrap();
    let renamed = commit_index(&repo, "rename");
    let gix_repo = gix::open(&path).unwrap();
    let libgit2_rename_old = git2_file_status(&repo, renamed, "old.txt");
    let libgit2_rename_new = git2_file_status(&repo, renamed, "new.txt");
    let gix_rename_old = gix_file_status(&gix_repo, renamed, "old.txt");
    let gix_rename_new = gix_file_status(&gix_repo, renamed, "new.txt");
    assert_eq!(libgit2_rename_old, gix_rename_old);
    assert_eq!(libgit2_rename_new, gix_rename_new);
    assert_eq!(gix_rename_old, Some(FileStatus::Renamed));
    assert_eq!(gix_rename_new, Some(FileStatus::Renamed));

    let (path, repo) = temp_repo("parity-copy");
    commit_file(&repo, &path, "source.txt", "one\n", "root");
    write_file(&path, "copy.txt", "one\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("copy.txt")).unwrap();
    index.write().unwrap();
    let copied = commit_index(&repo, "copy");
    let gix_repo = gix::open(&path).unwrap();
    let libgit2_copy_new = git2_file_status(&repo, copied, "copy.txt");
    let libgit2_copy_old = git2_file_status(&repo, copied, "source.txt");
    let gix_copy_new = gix_file_status(&gix_repo, copied, "copy.txt");
    let gix_copy_old = gix_file_status(&gix_repo, copied, "source.txt");
    assert_eq!(libgit2_copy_new, gix_copy_new);
    assert_eq!(libgit2_copy_old, gix_copy_old);
    assert_eq!(gix_copy_new, Some(FileStatus::Added));
    assert_eq!(gix_copy_old, None);

    #[cfg(unix)]
    {
        let (path, repo) = temp_repo("parity-typechange");
        commit_file(&repo, &path, "link.txt", "one\n", "root");

        fs::remove_file(path.join("link.txt")).unwrap();
        std::os::unix::fs::symlink("target.txt", path.join("link.txt")).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("link.txt")).unwrap();
        index.write().unwrap();
        let typechanged = commit_index(&repo, "typechange");
        let gix_repo = gix::open(&path).unwrap();
        let libgit2_typechange = git2_file_status(&repo, typechanged, "link.txt");
        let gix_typechange = gix_file_status(&gix_repo, typechanged, "link.txt");
        assert_eq!(libgit2_typechange, gix_typechange);
        assert_eq!(gix_typechange, Some(FileStatus::Deleted));
    }

    let (path, repo) = temp_repo("parity-directory-like");
    commit_file(&repo, &path, "docs/guide", "one\n", "root");
    write_file(&path, "docs/guide", "two\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("docs/guide")).unwrap();
    index.write().unwrap();
    let updated = commit_index(&repo, "update");
    let gix_repo = gix::open(&path).unwrap();
    let libgit2_dirlike = git2_file_status(&repo, updated, "docs/guide");
    let gix_dirlike = gix_file_status(&gix_repo, updated, "docs/guide");
    assert_eq!(libgit2_dirlike, gix_dirlike);
    assert_eq!(gix_dirlike, Some(FileStatus::Modified));
}
