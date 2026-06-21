use super::*;
use crate::git::{
    actions::worktrees::{create_worktree, lock_worktree, remove_worktree, unlock_worktree},
    queries::commits::get_current_branch,
};
use git2::{Repository, Signature};
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
        let path = env::temp_dir().join(format!("guitar-{name}-{}-{suffix}", process::id()));
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
    let repo = Repository::init(path).unwrap();
    fs::write(path.join("file.txt"), "hello\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Tester", "tester@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
    drop(tree);
    repo
}

#[test]
fn lists_main_and_linked_worktrees_with_stable_metadata() {
    let dir = TestDir::new("worktree-list");
    let repo_path = dir.path.join("repo");
    let alpha_path = dir.path.join("repo-alpha");
    let zeta_path = dir.path.join("repo-zeta");
    fs::create_dir_all(&repo_path).unwrap();
    let repo = init_repo(&repo_path);
    let oid = repo.head().unwrap().target().unwrap();

    create_worktree(&repo, "zeta", &zeta_path, oid).unwrap();
    create_worktree(&repo, "alpha", &alpha_path, oid).unwrap();

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].name, "repo");
    assert!(entries[0].is_main());
    assert!(entries[0].is_current);
    assert_eq!(entries[0].branch.as_deref(), get_current_branch(&repo).as_deref());
    assert_eq!(entries[0].head, Some(oid));

    let linked_names: Vec<_> = entries.iter().skip(1).map(|entry| entry.name.as_str()).collect();
    assert_eq!(linked_names, vec!["alpha", "zeta"]);

    for entry in entries.iter().skip(1) {
        assert!(entry.is_linked());
        assert!(entry.is_valid);
        assert!(!entry.is_current);
        assert_eq!(entry.branch.as_deref(), Some(entry.name.as_str()));
        assert_eq!(entry.head, Some(oid));
        assert!(!entry.is_dirty);
        assert!(entry.locked_reason.is_none());
        assert!(!entry.is_prunable);
    }
}

#[test]
fn marks_current_linked_worktree() {
    let dir = TestDir::new("worktree-current");
    let repo_path = dir.path.join("repo");
    let worktree_path = dir.path.join("repo-feature");
    fs::create_dir_all(&repo_path).unwrap();
    let repo = init_repo(&repo_path);
    let oid = repo.head().unwrap().target().unwrap();

    create_worktree(&repo, "feature", &worktree_path, oid).unwrap();
    let linked_repo = Repository::open(&worktree_path).unwrap();

    let entries = list_worktrees(&linked_repo, Some(&worktree_path)).unwrap();
    let linked = entries.iter().find(|entry| entry.name == "feature").unwrap();
    let main = entries.iter().find(|entry| entry.is_main()).unwrap();

    assert!(linked.is_current);
    assert_eq!(linked.branch.as_deref(), Some("feature"));
    assert_eq!(linked.head, Some(oid));
    assert!(main.is_main());
    assert!(!main.is_current);
    assert_eq!(main.head, Some(oid));
}

#[test]
fn marks_dirty_worktrees_when_staged_files_exist() {
    let dir = TestDir::new("worktree-staged");
    let repo_path = dir.path.join("repo");
    let worktree_path = dir.path.join("repo-feature");
    fs::create_dir_all(&repo_path).unwrap();
    let repo = init_repo(&repo_path);
    let oid = repo.head().unwrap().target().unwrap();

    create_worktree(&repo, "feature", &worktree_path, oid).unwrap();
    let linked_repo = Repository::open(&worktree_path).unwrap();

    fs::write(worktree_path.join("staged.txt"), "staged\n").unwrap();
    let mut index = linked_repo.index().unwrap();
    index.add_path(Path::new("staged.txt")).unwrap();
    index.write().unwrap();

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let linked = entries.iter().find(|entry| entry.name == "feature").unwrap();

    assert!(linked.is_dirty);
    assert!(linked.is_linked());
}

#[test]
fn marks_dirty_worktrees_when_untracked_files_exist() {
    let dir = TestDir::new("worktree-dirty");
    let repo_path = dir.path.join("repo");
    fs::create_dir_all(&repo_path).unwrap();
    let repo = init_repo(&repo_path);

    fs::write(repo_path.join("untracked.txt"), "extra\n").unwrap();

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let main = entries.iter().find(|entry| entry.is_main()).unwrap();

    assert!(main.is_current);
    assert!(main.is_dirty);
}

#[test]
fn metadata_listing_skips_dirty_scan_but_keeps_identity() {
    let dir = TestDir::new("worktree-metadata");
    let repo_path = dir.path.join("repo");
    fs::create_dir_all(&repo_path).unwrap();
    let repo = init_repo(&repo_path);
    let oid = repo.head().unwrap().target().unwrap();

    fs::write(repo_path.join("untracked.txt"), "extra\n").unwrap();

    let entries = list_worktrees_metadata(&repo, Some(&repo_path)).unwrap();
    let main = entries.iter().find(|entry| entry.is_main()).unwrap();

    assert!(main.is_current);
    assert_eq!(main.head, Some(oid));
    assert_eq!(main.branch.as_deref(), get_current_branch(&repo).as_deref());
    assert!(!main.is_dirty);
}

#[test]
fn reports_lock_reason_and_prunability_for_stale_worktrees() {
    let dir = TestDir::new("worktree-stale");
    let repo_path = dir.path.join("repo");
    let locked_path = dir.path.join("repo-feature");
    let stale_path = dir.path.join("repo-stale");
    fs::create_dir_all(&repo_path).unwrap();
    let repo = init_repo(&repo_path);
    let oid = repo.head().unwrap().target().unwrap();

    create_worktree(&repo, "feature", &locked_path, oid).unwrap();
    lock_worktree(&repo, "feature", Some("keep it")).unwrap();

    let locked_entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let locked = locked_entries.iter().find(|entry| entry.name == "feature").unwrap();
    assert_eq!(locked.locked_reason.as_deref(), Some("keep it"));
    assert!(!locked.can_remove());

    unlock_worktree(&repo, "feature").unwrap();
    remove_worktree(&repo, "feature").unwrap();
    assert!(repo.find_worktree("feature").is_err());

    create_worktree(&repo, "stale", &stale_path, oid).unwrap();
    fs::remove_dir_all(&stale_path).unwrap();

    let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
    let stale = entries.iter().find(|entry| entry.name == "stale").unwrap();
    assert!(!stale.is_valid);
    assert!(stale.is_prunable);
}
