use super::*;
use crate::git::actions::{
    rebasing::{RebaseOutcome, start_rebase},
    submodules::{stage_submodule_head, unstage_submodule},
};
use git2::{Repository, Signature, build::CheckoutBuilder};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-diff-{name}-{}-{suffix}", process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-diff-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn write(path: &Path, file: &str, content: &str) {
    fs::write(path.join(file), content).unwrap();
}

fn commit(repo: &Repository, file: &str, message: &str) -> Oid {
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    commit_index(repo, message)
}

fn commit_index(repo: &Repository, message: &str) -> Oid {
    let mut index = repo.index().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}

fn init_repo_at(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    let repo = Repository::init(path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    write(path, "file.txt", "hello\n");
    commit(&repo, "file.txt", "initial");
    repo
}

fn parent_with_submodule(dir: &TestDir) -> Repository {
    let child_path = dir.path.join("child");
    let parent_path = dir.path.join("parent");
    let child = init_repo_at(&child_path);
    drop(child);
    let parent = init_repo_at(&parent_path);
    let mut submodule = parent.submodule(child_path.to_str().unwrap(), Path::new("deps/child"), true).unwrap();
    submodule.clone(None).unwrap();
    submodule.add_finalize().unwrap();
    commit_index(&parent, "add submodule");
    drop(submodule);
    parent
}

fn assert_no_file_status_rows(changes: &UncommittedChanges) {
    assert!(changes.staged.modified.is_empty());
    assert!(changes.staged.added.is_empty());
    assert!(changes.staged.deleted.is_empty());
    assert!(changes.unstaged.modified.is_empty());
    assert!(changes.unstaged.added.is_empty());
    assert!(changes.unstaged.deleted.is_empty());
    assert!(changes.conflicts.is_empty());
    assert!(changes.is_clean);
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
fn workdir_diff_marks_conflicted_paths() {
    let (path, repo) = temp_repo("conflict");
    write(&path, "file.txt", "base\n");
    commit(&repo, "file.txt", "base");
    checkout_new_branch(&repo, "feature");
    write(&path, "file.txt", "feature\n");
    commit(&repo, "file.txt", "feature");
    checkout_branch(&repo, "master");
    write(&path, "file.txt", "main\n");
    let main = commit(&repo, "file.txt", "main");
    checkout_branch(&repo, "feature");

    assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);

    let changes = get_filenames_diff_at_workdir(&repo).unwrap();
    assert!(changes.has_conflicts);
    assert!(changes.is_staged);
    assert!(changes.is_unstaged);
    assert_eq!(changes.conflict_count, 1);
    assert_eq!(changes.conflicts, vec!["file.txt".to_string()]);

    let conflict = get_conflict_file(&repo, "file.txt").unwrap().unwrap();
    assert!(!conflict.ours.is_empty());
    assert!(!conflict.theirs.is_empty());
    assert!(conflict.workdir.iter().any(|line| line.starts_with("<<<<<<<")));
    assert!(conflict.workdir.iter().any(|line| line.starts_with("=======")));
    assert!(conflict.workdir.iter().any(|line| line.starts_with(">>>>>>>")));

    let _ = fs::remove_dir_all(path);
}

#[test]
fn workdir_diff_ignores_clean_initialized_submodule() {
    let dir = TestDir::new("submodule-clean");
    let parent = parent_with_submodule(&dir);

    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert_no_file_status_rows(&changes);
}

#[test]
fn workdir_diff_ignores_dirty_tracked_submodule_content() {
    let dir = TestDir::new("submodule-dirty");
    let parent = parent_with_submodule(&dir);
    fs::write(parent.workdir().unwrap().join("deps/child/file.txt"), "dirty\n").unwrap();

    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert_no_file_status_rows(&changes);
}

#[test]
fn workdir_diff_ignores_untracked_submodule_content() {
    let dir = TestDir::new("submodule-untracked");
    let parent = parent_with_submodule(&dir);
    fs::write(parent.workdir().unwrap().join("deps/child/extra.txt"), "extra\n").unwrap();

    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert_no_file_status_rows(&changes);
}

#[test]
fn workdir_diff_ignores_uninitialized_submodule() {
    let dir = TestDir::new("submodule-uninitialized");
    let parent = parent_with_submodule(&dir);
    let clone_path = dir.path.join("clone");
    let clone = Repository::clone(parent.workdir().unwrap().to_str().unwrap(), &clone_path).unwrap();

    let changes = get_filenames_diff_at_workdir(&clone).unwrap();

    assert_no_file_status_rows(&changes);
}

#[test]
fn workdir_diff_lists_changed_submodule_pointer_as_unstaged_modified() {
    let dir = TestDir::new("submodule-pointer");
    let parent = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    write(sub_repo.workdir().unwrap(), "file.txt", "advanced\n");
    commit(&sub_repo, "file.txt", "advance child");

    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert!(changes.staged.modified.is_empty());
    assert_eq!(changes.unstaged.modified, vec!["deps/child".to_string()]);
    assert_eq!(changes.modified_count, 1);
    assert!(changes.is_unstaged);
    assert!(!changes.is_staged);
    assert!(!changes.is_clean);
}

#[test]
fn workdir_diff_lists_staged_submodule_pointer_as_staged_modified() {
    let dir = TestDir::new("submodule-pointer-staged");
    let parent = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    write(sub_repo.workdir().unwrap(), "file.txt", "advanced\n");
    commit(&sub_repo, "file.txt", "advance child");

    stage_submodule_head(&parent, "deps/child").unwrap();
    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert_eq!(changes.staged.modified, vec!["deps/child".to_string()]);
    assert!(changes.unstaged.modified.is_empty());
    assert_eq!(changes.modified_count, 1);
    assert!(changes.is_staged);
    assert!(!changes.is_unstaged);
    assert!(!changes.is_clean);

    unstage_submodule(&parent, "deps/child").unwrap();
    let changes = get_filenames_diff_at_workdir(&parent).unwrap();

    assert!(changes.staged.modified.is_empty());
    assert_eq!(changes.unstaged.modified, vec!["deps/child".to_string()]);
}

#[test]
fn submodule_status_path_guard_matches_exact_paths_and_children() {
    let submodule_paths = vec![PathBuf::from("deps/child")];

    assert!(is_submodule_status_path("deps/child", &submodule_paths));
    assert!(is_submodule_status_path("deps/child/", &submodule_paths));
    assert!(is_submodule_status_path("deps/child/file.txt", &submodule_paths));
    assert!(!is_submodule_status_path("deps/childish", &submodule_paths));
    assert!(!is_submodule_status_path("deps", &submodule_paths));
}
