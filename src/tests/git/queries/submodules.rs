use super::*;
#[path = "../support.rs"]
mod support;
use git2::Repository;
use std::{
    fs,
    path::{Path, PathBuf},
};
use support::{TestDir, commit_file, commit_index, init_repo};

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

fn parent_with_submodules(dir: &TestDir, count: usize) -> Repository {
    let parent_path = dir.path.join("parent-many");
    let parent = init_repo(&parent_path);

    for index in 0..count {
        let child_path = dir.path.join(format!("child-{index:03}"));
        let child = init_repo(&child_path);
        drop(child);

        let path = format!("deps/child-{index:03}");
        let mut submodule = parent.submodule(child_path.to_str().unwrap(), Path::new(&path), true).unwrap();
        submodule.clone(None).unwrap();
        submodule.add_finalize().unwrap();
    }

    commit_index(&parent, "add submodules");
    parent
}

fn only_entry(repo: &Repository) -> crate::core::submodules::SubmoduleEntry {
    let entries = list_submodules(repo).unwrap();
    assert_eq!(entries.len(), 1);
    entries.into_iter().next().unwrap()
}

#[test]
fn lists_multiple_uninitialized_submodules_without_opening_repositories() {
    let dir = TestDir::new("submodule-query", "many-uninitialized");
    let parent = parent_with_submodules(&dir, 3);
    let clone_path = dir.path.join("clone-many");
    let clone = Repository::clone(parent.workdir().unwrap().to_str().unwrap(), &clone_path).unwrap();

    let entries = list_submodules(&clone).unwrap();

    let paths = entries.iter().map(|entry| entry.path.clone()).collect::<Vec<_>>();
    assert_eq!(paths, vec![PathBuf::from("deps/child-000"), PathBuf::from("deps/child-001"), PathBuf::from("deps/child-002"),]);
    assert!(entries.iter().all(|entry| !entry.is_open));
    assert!(entries.iter().all(|entry| entry.is_uninitialized || !entry.is_in_workdir));
}

#[test]
fn lists_no_submodules_for_repo_without_gitmodules() {
    let dir = TestDir::new("submodule-query", "none");
    let repo = init_repo(&dir.path);

    let entries = list_submodules(&repo).unwrap();

    assert!(entries.is_empty());
    assert!(submodules_if_present(&repo).unwrap().is_empty());
    assert!(!has_submodule_metadata(&repo));
}

#[test]
fn detects_submodule_metadata_from_workdir() {
    let dir = TestDir::new("submodule-query", "workdir-gitmodules");
    let repo = init_repo(&dir.path);
    fs::write(repo.workdir().unwrap().join(".gitmodules"), "[submodule \"deps/child\"]\n\tpath = deps/child\n\turl = ../child\n").unwrap();

    assert!(has_submodule_metadata(&repo));
}

#[test]
fn detects_submodule_metadata_from_head_tree() {
    let dir = TestDir::new("submodule-query", "head-gitmodules");
    let repo = init_repo(&dir.path);
    commit_file(&repo, ".gitmodules", "[submodule \"deps/child\"]\n\tpath = deps/child\n\turl = ../child\n", "add gitmodules");
    fs::remove_file(repo.workdir().unwrap().join(".gitmodules")).unwrap();

    assert!(has_submodule_metadata(&repo));
}

#[test]
fn detects_submodule_metadata_from_index_when_workdir_and_head_are_missing_gitmodules() {
    let dir = TestDir::new("submodule-query", "index-only-gitmodules");
    let repo = init_repo(&dir.path);
    let gitmodules = repo.workdir().unwrap().join(".gitmodules");
    fs::write(&gitmodules, "[submodule \"deps/child\"]\n\tpath = deps/child\n\turl = ../child\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".gitmodules")).unwrap();
    index.write().unwrap();
    fs::remove_file(gitmodules).unwrap();

    assert!(has_submodule_metadata(&repo));
}

#[test]
fn lists_submodules_when_gitmodules_is_missing_from_workdir() {
    let dir = TestDir::new("submodule-query", "missing-gitmodules");
    let (parent, _) = parent_with_submodule(&dir);
    fs::remove_file(parent.workdir().unwrap().join(".gitmodules")).unwrap();

    let entries = list_submodules(&parent).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, PathBuf::from("deps/child"));
}

#[test]
fn lists_clean_submodule() {
    let dir = TestDir::new("submodule-query", "clean");
    let (parent, _child_path) = parent_with_submodule(&dir);

    let entry = only_entry(&parent);

    assert_eq!(entry.name, "deps/child");
    assert_eq!(entry.path, PathBuf::from("deps/child"));
    assert!(entry.is_open);
    assert!(!entry.is_dirty());
    assert!(entry.index.is_some());
    assert_eq!(entry.index, entry.workdir);
}

#[test]
fn detects_uninitialized_submodule_after_plain_clone() {
    let dir = TestDir::new("submodule-query", "uninitialized");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let clone_path = dir.path.join("clone");
    let clone = Repository::clone(parent.workdir().unwrap().to_str().unwrap(), &clone_path).unwrap();

    let entry = only_entry(&clone);

    assert!(!entry.is_open);
    assert!(entry.is_uninitialized || !entry.is_in_workdir);
}

#[test]
fn detects_submodule_new_commits() {
    let dir = TestDir::new("submodule-query", "new-commits");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();

    commit_file(&sub_repo, "file.txt", "changed\n", "advance child");
    let entry = only_entry(&parent);

    assert!(entry.has_new_commits);
    assert!(entry.is_dirty());
    assert_ne!(entry.index, entry.workdir);
}

#[test]
fn detects_submodule_modified_content() {
    let dir = TestDir::new("submodule-query", "modified-content");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_path = parent.workdir().unwrap().join("deps/child");

    fs::write(sub_path.join("file.txt"), "dirty\n").unwrap();
    let entry = only_entry(&parent);

    assert!(entry.has_modified_content);
    assert!(entry.is_dirty());
}

#[test]
fn detects_submodule_untracked_content() {
    let dir = TestDir::new("submodule-query", "untracked-content");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_path = parent.workdir().unwrap().join("deps/child");

    fs::write(sub_path.join("extra.txt"), "extra\n").unwrap();
    let entry = only_entry(&parent);

    assert!(entry.has_untracked_content);
    assert!(entry.is_dirty());
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
