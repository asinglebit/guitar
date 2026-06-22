use super::*;
use crate::git::actions::submodules::stage_submodule_head;
use git2::{Repository, Signature, build::CheckoutBuilder};
use std::{
    env, fs,
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
        let path = env::temp_dir().join(format!("guitar-submodule-query-{name}-{}-{suffix}", process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn init_repo(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    let repo = Repository::init(path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    commit_file(&repo, "file.txt", "hello\n", "initial");
    repo
}

fn commit_file(repo: &Repository, file: &str, contents: &str, message: &str) -> git2::Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), contents).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    commit_index(repo, message)
}

fn commit_index(repo: &Repository, message: &str) -> git2::Oid {
    let mut index = repo.index().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}

fn parent_with_submodule(dir: &TestDir) -> (Repository, PathBuf) {
    let child_path = dir.path.join("child");
    let parent_path = dir.path.join("parent");
    let child = init_repo(&child_path);
    drop(child);
    let parent = init_repo(&parent_path);
    let mut submodule = parent.submodule(child_path.to_str().unwrap(), Path::new("deps/child"), true).unwrap();
    submodule.clone(None).unwrap();
    submodule.add_finalize().unwrap();
    commit_index(&parent, "add submodule");
    drop(submodule);
    (parent, child_path)
}

fn only_entry(repo: &Repository) -> crate::core::submodules::SubmoduleEntry {
    let entries = list_submodules(repo).unwrap();
    assert_eq!(entries.len(), 1);
    entries.into_iter().next().unwrap()
}

#[test]
fn returns_empty_when_repo_has_no_submodule_metadata() {
    let dir = TestDir::new("no-submodule-metadata");
    let repo = init_repo(&dir.path.join("repo"));

    let entries = list_submodules(&repo).unwrap();

    assert!(entries.is_empty());
}

#[test]
fn detects_staged_gitmodules_without_workdir_or_head_entry() {
    let dir = TestDir::new("staged-gitmodules");
    let repo = init_repo(&dir.path.join("repo"));
    let gitmodules = repo.workdir().unwrap().join(".gitmodules");
    fs::write(&gitmodules, "[submodule \"deps/child\"]\n\tpath = deps/child\n\turl = ../child\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".gitmodules")).unwrap();
    index.write().unwrap();
    fs::remove_file(gitmodules).unwrap();

    assert!(has_submodule_metadata(&repo));
    assert!(!has_committed_or_workdir_submodule_metadata(&repo));
    assert!(list_submodules(&repo).unwrap().is_empty());
}

#[test]
fn detects_workdir_gitmodules_without_index_or_head_entry() {
    let dir = TestDir::new("workdir-gitmodules");
    let repo = init_repo(&dir.path.join("repo"));
    let gitmodules = repo.workdir().unwrap().join(".gitmodules");
    fs::write(&gitmodules, "[submodule \"deps/child\"]\n\tpath = deps/child\n\turl = ../child\n").unwrap();

    assert!(has_submodule_metadata(&repo));
    assert!(has_committed_or_workdir_submodule_metadata(&repo));
}

#[test]
fn detects_committed_gitmodules_without_workdir_file() {
    let dir = TestDir::new("committed-gitmodules");
    let repo = init_repo(&dir.path.join("repo"));
    let gitmodules = repo.workdir().unwrap().join(".gitmodules");
    commit_file(&repo, ".gitmodules", "[submodule \"deps/child\"]\n\tpath = deps/child\n\turl = ../child\n", "add gitmodules");
    fs::remove_file(&gitmodules).unwrap();

    assert!(has_submodule_metadata(&repo));
    assert!(has_committed_or_workdir_submodule_metadata(&repo));
}

#[test]
fn gitmodules_index_scan_matches_across_buffer_boundary() {
    let dir = TestDir::new("gitmodules-scan-boundary");
    let path = dir.path.join("index");
    let split = INDEX_SCAN_BUFFER - 4;
    let mut bytes = vec![b'x'; split];
    bytes.extend_from_slice(GITMODULES_PATH);
    fs::write(&path, bytes).unwrap();

    assert!(file_contains_gitmodules_path(&path));
}

#[test]
fn gitmodules_index_scan_returns_false_without_path() {
    let dir = TestDir::new("gitmodules-scan-missing");
    let path = dir.path.join("index");
    fs::write(&path, vec![b'x'; INDEX_SCAN_BUFFER + 64]).unwrap();

    assert!(!file_contains_gitmodules_path(&path));
}

fn assert_clean_submodule(entry: &crate::core::submodules::SubmoduleEntry) {
    assert!(entry.is_open);
    assert!(!entry.is_uninitialized);
    assert!(entry.is_in_head);
    assert!(entry.is_in_index);
    assert!(entry.is_in_config);
    assert!(entry.is_in_workdir);
    assert!(!entry.is_index_modified);
    assert!(!entry.is_workdir_modified);
    assert!(!entry.has_new_commits);
    assert!(!entry.has_modified_content);
    assert!(!entry.has_untracked_content);
}

#[test]
fn lists_clean_submodule() {
    let dir = TestDir::new("clean");
    let (parent, _child_path) = parent_with_submodule(&dir);

    let entry = only_entry(&parent);

    assert_eq!(entry.name, "deps/child");
    assert_eq!(entry.path, PathBuf::from("deps/child"));
    assert!(entry.branch.is_some());
    assert_clean_submodule(&entry);
    assert!(entry.index.is_some());
    assert_eq!(entry.index, entry.workdir);
    assert_eq!(entry.head, entry.index);
}

#[test]
fn detects_uninitialized_submodule_after_plain_clone() {
    let dir = TestDir::new("uninitialized");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let clone_path = dir.path.join("clone");
    let clone = Repository::clone(parent.workdir().unwrap().to_str().unwrap(), &clone_path).unwrap();

    let entry = only_entry(&clone);

    assert!(!entry.is_open);
    assert!(entry.is_uninitialized);
    assert!(entry.is_in_head);
    assert!(entry.is_in_index);
    assert!(!entry.is_in_config);
    assert!(!entry.is_in_workdir);
    assert!(entry.head.is_some());
    assert!(entry.index.is_some());
    assert_eq!(entry.head, entry.index);
    assert!(entry.workdir.is_none());
}

#[test]
fn detects_submodule_new_commits() {
    let dir = TestDir::new("new-commits");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();

    commit_file(&sub_repo, "file.txt", "changed\n", "advance child");
    let entry = only_entry(&parent);

    assert!(entry.has_new_commits);
    assert!(!entry.is_index_modified);
    assert!(entry.is_workdir_modified);
    assert!(entry.is_dirty());
    assert_eq!(entry.head, entry.index);
    assert_ne!(entry.index, entry.workdir);
}

#[test]
fn detects_submodule_modified_content() {
    let dir = TestDir::new("modified-content");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_path = parent.workdir().unwrap().join("deps/child");

    fs::write(sub_path.join("file.txt"), "dirty\n").unwrap();
    let entry = only_entry(&parent);

    assert!(entry.has_modified_content);
    assert!(!entry.has_new_commits);
    assert!(!entry.is_index_modified);
    assert!(entry.is_workdir_modified);
    assert!(entry.is_dirty());
}

#[test]
fn detects_submodule_untracked_content() {
    let dir = TestDir::new("untracked-content");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_path = parent.workdir().unwrap().join("deps/child");

    fs::write(sub_path.join("extra.txt"), "extra\n").unwrap();
    let entry = only_entry(&parent);

    assert!(entry.has_untracked_content);
    assert!(!entry.has_new_commits);
    assert!(!entry.is_index_modified);
    assert!(entry.is_workdir_modified);
    assert!(entry.is_dirty());
}

#[test]
fn reports_staged_submodule_pointer_changes_as_index_modified() {
    let dir = TestDir::new("staged-pointer");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();

    let advanced = commit_file(&sub_repo, "file.txt", "changed\n", "advance child");
    stage_submodule_head(&parent, "deps/child").unwrap();
    let entry = only_entry(&parent);

    assert!(entry.is_index_modified);
    assert!(!entry.has_new_commits);
    assert!(!entry.is_workdir_modified);
    assert!(entry.is_dirty());
    assert_eq!(entry.index, Some(advanced));
    assert_ne!(entry.head, entry.index);
}

#[test]
fn reports_open_submodule_branch_when_checked_out_on_a_branch() {
    let dir = TestDir::new("branch");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    let head_commit = sub_repo.head().unwrap().peel_to_commit().unwrap();
    let branch = sub_repo.branch("feature", &head_commit, false).unwrap();
    sub_repo.set_head(branch.get().name().unwrap()).unwrap();
    sub_repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();

    let entry = only_entry(&parent);

    assert_eq!(entry.branch.as_deref(), Some("feature"));
}

#[test]
fn lists_only_immediate_submodules_without_recursing() {
    let dir = TestDir::new("non-recursive");
    let grandchild_path = dir.path.join("grandchild");
    let child_path = dir.path.join("child");
    let child = init_repo(&child_path);
    let grandchild = init_repo(&grandchild_path);
    drop(grandchild);

    let mut nested = child.submodule(grandchild_path.to_str().unwrap(), Path::new("vendor/grandchild"), true).unwrap();
    nested.clone(None).unwrap();
    nested.add_finalize().unwrap();
    commit_index(&child, "add nested submodule");
    drop(nested);

    let child_entries = list_submodules(&child).unwrap();
    assert_eq!(child_entries.len(), 1);
    assert_eq!(child_entries[0].name, "vendor/grandchild");

    let parent_path = dir.path.join("parent");
    let parent = init_repo(&parent_path);
    let mut submodule = parent.submodule(child_path.to_str().unwrap(), Path::new("deps/child"), true).unwrap();
    submodule.clone(None).unwrap();
    submodule.add_finalize().unwrap();
    commit_index(&parent, "add submodule");
    drop(submodule);

    let entries = list_submodules(&parent).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "deps/child");
    assert_eq!(entries[0].path, PathBuf::from("deps/child"));
}
