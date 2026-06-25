use super::*;
use crate::{
    core::oids::git2_to_gix_oid,
    git::{
        actions::staging::stage_all,
        auth::NetworkResult,
        queries::submodules::list_submodules,
        test_support::{TestDir, commit_file, init_repo_at, parent_with_submodule},
    },
};
use git2::Repository;
use gix::bstr::ByteSlice;
use std::{fs, path::Path};

fn rewrite_submodule_url(repo: &Repository, new_url: &str) {
    let gitmodules = repo.workdir().unwrap().join(".gitmodules");
    let contents = fs::read_to_string(&gitmodules).unwrap();
    let old_url = repo.config().unwrap().get_string("submodule.deps/child.url").unwrap();
    let updated = contents.replace(&old_url, new_url);
    fs::write(gitmodules, updated).unwrap();
}

fn submodule_remote_url(repo: &Repository, submodule_path: &str) -> String {
    let sub_repo = Repository::open(repo.workdir().unwrap().join(submodule_path)).unwrap();
    sub_repo.find_remote("origin").unwrap().url().unwrap().to_string()
}

fn stage_submodule_to_oid(repo: &Repository, name: &str, oid: git2::Oid) {
    let gix_repo = gix::open(repo.workdir().unwrap()).unwrap();
    let mut index = gix_repo.index_or_load_from_head_or_empty().unwrap().into_owned();
    stage_commit_pointer(&mut index, name.as_bytes().as_bstr(), git2_to_gix_oid(oid));
    write_index(&mut index).unwrap();
}

fn update_submodule_result(repo_path: &Path, name: &str) -> NetworkResult {
    update_submodule(repo_path.to_str().unwrap(), name, Default::default()).join().unwrap()
}

#[test]
fn stages_and_unstages_submodule_pointer() {
    let dir = TestDir::new("stage-pointer");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    let original = list_submodules(&parent).unwrap()[0].index;

    let advanced = git2_to_gix_oid(commit_file(&sub_repo, "file.txt", "changed\n", "advance child"));
    stage_submodule_head(&parent, "deps/child").unwrap();

    let staged = list_submodules(&parent).unwrap()[0].clone();
    assert_eq!(staged.index, Some(git2_to_gix_oid(advanced)));

    unstage_submodule(&parent, "deps/child").unwrap();
    let unstaged = list_submodules(&parent).unwrap()[0].clone();
    assert_eq!(unstaged.index, original);
}

#[test]
fn stage_all_stages_submodule_pointer_without_staging_inner_content() {
    let dir = TestDir::new("stage-all-pointer");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();

    let advanced = git2_to_gix_oid(commit_file(&sub_repo, "file.txt", "changed\n", "advance child"));
    fs::write(sub_repo.workdir().unwrap().join("file.txt"), "dirty\n").unwrap();

    stage_all(&parent).unwrap();

    let entry = list_submodules(&parent).unwrap()[0].clone();
    assert_eq!(entry.index, Some(git2_to_gix_oid(advanced)));
    assert_eq!(entry.workdir, Some(git2_to_gix_oid(advanced)));
    assert!(entry.has_modified_content);
}

#[test]
fn stage_all_stages_regular_changes_without_submodule_metadata() {
    let dir = TestDir::new("stage-all-no-submodules");
    let repo = init_repo_at(dir.path());
    commit_file(&repo, "file.txt", "hello\n", "initial");
    fs::write(dir.path().join("file.txt"), "changed\n").unwrap();
    fs::write(dir.path().join("new.txt"), "new\n").unwrap();

    stage_all(&repo).unwrap();

    let statuses = repo.statuses(None).unwrap();
    let staged = statuses.iter().map(|entry| (entry.path().unwrap_or("").to_string(), entry.status())).collect::<Vec<_>>();

    assert!(staged.iter().any(|(path, status)| path == "file.txt" && status.is_index_modified()));
    assert!(staged.iter().any(|(path, status)| path == "new.txt" && status.is_index_new()));
    assert!(staged.iter().all(|(_, status)| !status.is_wt_modified() && !status.is_wt_new()));
}

#[test]
fn sync_submodule_updates_both_superproject_and_checked_out_remote_urls() {
    let dir = TestDir::new("sync");
    let (parent, child_path) = parent_with_submodule(&dir);
    let replacement_child = init_repo_at(dir.path().join("replacement-child").as_path());
    let replacement_url = replacement_child.workdir().unwrap().to_str().unwrap().to_string();

    let before_superproject_url = parent.config().unwrap().get_string("submodule.deps/child.url").unwrap();
    let before_submodule_url = submodule_remote_url(&parent, "deps/child");
    rewrite_submodule_url(&parent, &replacement_url);
    drop(replacement_child);
    drop(parent);

    let parent = Repository::open(dir.path().join("parent")).unwrap();
    assert_eq!(before_superproject_url, child_path.to_str().unwrap());
    assert_eq!(before_submodule_url, child_path.to_str().unwrap());

    sync_submodule(&parent, "deps/child").unwrap();

    let parent = Repository::open(dir.path().join("parent")).unwrap();
    assert_eq!(parent.config().unwrap().get_string("submodule.deps/child.url").unwrap(), replacement_url);
    assert_eq!(submodule_remote_url(&parent, "deps/child"), replacement_url);
}

#[test]
fn sync_submodule_errors_for_unknown_submodule() {
    let dir = TestDir::new("sync-missing");
    let (parent, _child_path) = parent_with_submodule(&dir);

    let error = sync_submodule(&parent, "deps/missing").unwrap_err();
    assert!(error.to_string().contains("Submodule not found"));
    assert!(matches!(update_submodule_result(parent.workdir().unwrap(), "deps/missing"), NetworkResult::Failure(_)));
}

#[test]
fn update_submodule_initializes_plain_clone_then_refreshes_checkout() {
    let dir = TestDir::new("update");
    let (parent, child_path) = parent_with_submodule(&dir);
    let parent_entry = list_submodules(&parent).unwrap()[0].clone();
    let clone_path = dir.path().join("clone");
    let clone = Repository::clone(parent.workdir().unwrap().to_str().unwrap(), &clone_path).unwrap();
    assert!(!list_submodules(&clone).unwrap()[0].is_open);
    drop(clone);

    assert!(matches!(update_submodule_result(&clone_path, "deps/child"), NetworkResult::Success));

    let clone = Repository::open(&clone_path).unwrap();
    let submodule = list_submodules(&clone).unwrap()[0].clone();
    assert!(submodule.is_open);
    assert_eq!(submodule.workdir, parent_entry.head);
    assert_eq!(submodule_remote_url(&clone, "deps/child"), child_path.to_str().unwrap());

    let advanced = commit_file(&Repository::open(&child_path).unwrap(), "file.txt", "changed\n", "advance child");
    stage_submodule_to_oid(&Repository::open(&clone_path).unwrap(), "deps/child", advanced);

    assert!(matches!(update_submodule_result(&clone_path, "deps/child"), NetworkResult::Success));

    let clone = Repository::open(&clone_path).unwrap();
    let submodule = list_submodules(&clone).unwrap()[0].clone();
    assert_eq!(submodule.index, Some(git2_to_gix_oid(advanced)));
    assert_eq!(submodule.workdir, Some(git2_to_gix_oid(advanced)));
    let sub_repo = Repository::open(clone.workdir().unwrap().join("deps/child")).unwrap();
    assert_eq!(sub_repo.head().unwrap().peel_to_commit().unwrap().id(), advanced);
}

#[test]
fn update_submodule_errors_for_unreachable_remote_url() {
    let dir = TestDir::new("update-unreachable");
    let (parent, child_path) = parent_with_submodule(&dir);
    let clone_path = dir.path().join("clone");
    let clone = Repository::clone(parent.workdir().unwrap().to_str().unwrap(), &clone_path).unwrap();
    drop(clone);
    fs::remove_dir_all(&child_path).unwrap();

    assert!(matches!(update_submodule_result(&clone_path, "deps/child"), NetworkResult::Failure(_)));
}
