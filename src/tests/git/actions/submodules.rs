use super::*;
use crate::{git::actions::staging::stage_all, git::auth::NetworkResult, git::queries::submodules::list_submodules};
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
        let path = env::temp_dir().join(format!("guitar-submodule-action-{name}-{}-{suffix}", process::id()));
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

#[test]
fn stages_and_unstages_submodule_pointer() {
    let dir = TestDir::new("stage-pointer");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    let original = list_submodules(&parent).unwrap()[0].index;

    let advanced = commit_file(&sub_repo, "file.txt", "changed\n", "advance child");
    stage_submodule_head(&parent, "deps/child").unwrap();

    let staged = list_submodules(&parent).unwrap()[0].clone();
    assert_eq!(staged.index, Some(advanced));

    unstage_submodule(&parent, "deps/child").unwrap();
    let unstaged = list_submodules(&parent).unwrap()[0].clone();
    assert_eq!(unstaged.index, original);
}

#[test]
fn stage_all_stages_submodule_pointer_without_staging_inner_content() {
    let dir = TestDir::new("stage-all-pointer");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();

    let advanced = commit_file(&sub_repo, "file.txt", "changed\n", "advance child");
    fs::write(sub_repo.workdir().unwrap().join("file.txt"), "dirty\n").unwrap();

    stage_all(&parent).unwrap();

    let entry = list_submodules(&parent).unwrap()[0].clone();
    assert_eq!(entry.index, Some(advanced));
    assert_eq!(entry.workdir, Some(advanced));
    assert!(entry.has_modified_content);
}

#[test]
fn stage_all_stages_regular_changes_without_submodule_metadata() {
    let dir = TestDir::new("stage-all-no-submodules");
    let repo = init_repo(&dir.path);
    fs::write(dir.path.join("file.txt"), "changed\n").unwrap();
    fs::write(dir.path.join("new.txt"), "new\n").unwrap();

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
    let replacement_child = init_repo(&dir.path.join("replacement-child"));
    let replacement_url = replacement_child.workdir().unwrap().to_str().unwrap().to_string();

    let before_superproject_url = parent.config().unwrap().get_string("submodule.deps/child.url").unwrap();
    let before_submodule_url = submodule_remote_url(&parent, "deps/child");
    rewrite_submodule_url(&parent, &replacement_url);
    drop(replacement_child);
    drop(parent);

    let parent = Repository::open(dir.path.join("parent")).unwrap();
    assert_eq!(before_superproject_url, child_path.to_str().unwrap());
    assert_eq!(before_submodule_url, child_path.to_str().unwrap());

    sync_submodule(&parent, "deps/child").unwrap();

    let parent = Repository::open(dir.path.join("parent")).unwrap();
    assert_eq!(parent.config().unwrap().get_string("submodule.deps/child.url").unwrap(), replacement_url);
    assert_eq!(submodule_remote_url(&parent, "deps/child"), replacement_url);
}

#[test]
fn sync_submodule_errors_for_unknown_submodule() {
    let dir = TestDir::new("sync-missing");
    let (parent, _child_path) = parent_with_submodule(&dir);

    let error = sync_submodule(&parent, "deps/missing").unwrap_err();
    assert!(error.to_string().contains("Submodule not found"));
}

#[test]
fn update_submodule_initializes_plain_clone() {
    let dir = TestDir::new("update");
    let (parent, _child_path) = parent_with_submodule(&dir);
    let clone_path = dir.path.join("clone");
    let clone = Repository::clone(parent.workdir().unwrap().to_str().unwrap(), &clone_path).unwrap();
    assert!(!list_submodules(&clone).unwrap()[0].is_open);
    drop(clone);

    let handle = update_submodule(clone_path.to_str().unwrap(), "deps/child", Default::default());
    match handle.join().unwrap() {
        NetworkResult::Success => {},
        other => panic!("unexpected update result: {other:?}"),
    }

    let clone = Repository::open(&clone_path).unwrap();
    assert!(list_submodules(&clone).unwrap()[0].is_open);
}

#[test]
fn update_submodule_errors_for_unknown_submodule() {
    let dir = TestDir::new("update-missing");
    let parent = init_repo(&dir.path.join("parent"));

    let handle = update_submodule(parent.workdir().unwrap().to_str().unwrap(), "deps/missing", Default::default());
    match handle.join().unwrap() {
        NetworkResult::Failure(_) => {},
        other => panic!("unexpected update result: {other:?}"),
    }
}
