use crate::git::actions::worktrees::create_worktree;
use crate::helpers::localisation::{Language, set_active_language};
use git2::{Oid, Repository, RepositoryInitOptions, Signature, Time, build::CheckoutBuilder};
use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, OnceLock},
};

pub struct TestDir {
    dir: tempfile::TempDir,
}

pub struct LinkedWorktreeFixture {
    pub repo_path: PathBuf,
    pub repo: Repository,
    pub linked_path: PathBuf,
    pub linked_repo: Repository,
    pub base: Oid,
}

pub struct LanguageTestGuard {
    _guard: MutexGuard<'static, ()>,
}

impl Drop for LanguageTestGuard {
    fn drop(&mut self) {
        set_active_language(Language::English);
    }
}

pub fn language_test_guard() -> LanguageTestGuard {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LanguageTestGuard { _guard: LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|poisoned| poisoned.into_inner()) }
}

impl TestDir {
    pub fn new(name: &str) -> Self {
        let dir = tempfile::Builder::new().prefix(&format!("guitar-test-support-{name}-")).tempdir().unwrap();
        Self { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.path().join(path)
    }
}

impl Deref for TestDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

pub fn temp_path(prefix: &str, name: &str) -> PathBuf {
    tempfile::Builder::new().prefix(&format!("{prefix}-{name}-")).tempfile().unwrap().into_temp_path().keep().unwrap()
}

pub fn temp_json_path(prefix: &str, name: &str) -> PathBuf {
    tempfile::Builder::new().prefix(&format!("{prefix}-{name}-")).suffix(".json").tempfile().unwrap().into_temp_path().keep().unwrap()
}

pub fn temp_named_dir(prefix: &str, name: &str) -> PathBuf {
    tempfile::Builder::new().prefix(&format!("{prefix}-{name}-")).tempdir().unwrap().keep()
}

pub fn read_to_string(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

fn configure_user(repo: &Repository) {
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
}

pub fn init_repo_at(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("master");
    let repo = Repository::init_opts(path, &opts).unwrap();
    configure_user(&repo);
    repo
}

pub fn temp_repo(name: &str) -> (TestDir, Repository) {
    let dir = TestDir::new(name);
    let repo = init_repo_at(&dir.join("repo"));
    (dir, repo)
}

pub fn temp_repo_with_commit(name: &str) -> (TestDir, Repository, Oid) {
    let (dir, repo) = temp_repo(name);
    let oid = commit_file(&repo, "file.txt", "content\n", "commit");
    (dir, repo, oid)
}

pub fn init_bare_repo_at(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    Repository::init_bare(path).unwrap()
}

pub fn write_workdir_file(repo: &Repository, relative: &str, contents: &str) {
    write_path_file(repo.workdir().unwrap(), relative, contents);
}

pub fn write_path_file(root: &Path, relative: &str, contents: &str) {
    let full_path = root.join(relative);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full_path, contents).unwrap();
}

pub fn assert_workdir_file(repo: &Repository, relative: &str, expected: &str) {
    assert_eq!(fs::read_to_string(repo.workdir().unwrap().join(relative)).unwrap(), expected);
}

pub fn checkout_new_branch(repo: &Repository, name: &str) {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch(name, &head, false).unwrap();
    checkout_branch(repo, name);
}

pub fn checkout_branch(repo: &Repository, name: &str) {
    repo.set_head(&format!("refs/heads/{name}")).unwrap();
    repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
}

pub fn diverge_file(repo: &Repository, relative: &str) -> (Oid, Oid) {
    let (_, feature, main) = diverge_files(repo, relative, relative, relative);
    (feature, main)
}

pub fn diverge_files(repo: &Repository, base_file: &str, feature_file: &str, main_file: &str) -> (Oid, Oid, Oid) {
    let base = commit_file(repo, base_file, "base\n", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(repo, "feature");
    let feature = commit_file(repo, feature_file, "feature\n", "feature");
    checkout_branch(repo, &base_branch);
    let main = commit_file(repo, main_file, "main\n", "main");
    (base, feature, main)
}

pub fn stage_path(repo: &Repository, relative: &str) {
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}

pub fn commit_index(repo: &Repository, message: &str) -> Oid {
    let mut index = repo.index().unwrap();
    index.read(true).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = repo.signature().unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parents).unwrap()
}

pub fn commit_file(repo: &Repository, relative: &str, contents: &str, message: &str) -> Oid {
    write_workdir_file(repo, relative, contents);
    stage_path(repo, relative);
    commit_index(repo, message)
}

pub fn commit_named_file(repo: &Repository, relative: &str, message: &str) -> Oid {
    commit_file(repo, relative, &format!("{message}\n"), message)
}

pub fn commit_staged_file(repo: &Repository, relative: &str, message: &str) -> Oid {
    stage_path(repo, relative);
    commit_index(repo, message)
}

pub fn commit_named_files(repo: &Repository, files: &[&str], message: &str) -> Oid {
    for file in files {
        write_workdir_file(repo, file, "content\n");
        stage_path(repo, file);
    }
    commit_index(repo, message)
}

pub fn commit_file_with_parents(repo: &Repository, relative: &str, contents: &str, message: &str, parents: &[Oid], time: Option<i64>) -> Oid {
    write_workdir_file(repo, relative, contents);
    stage_path(repo, relative);

    let mut index = repo.index().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = time.map_or_else(|| Signature::now("Test User", "test@example.com").unwrap(), |time| Signature::new("Test User", "test@example.com", &Time::new(time, 0)).unwrap());
    let parent_commits: Vec<_> = parents.iter().map(|oid| repo.find_commit(*oid).unwrap()).collect();
    let parent_refs: Vec<&git2::Commit<'_>> = parent_commits.iter().collect();
    let head = repo.head().ok().and_then(|head| head.target());
    let update_ref = match (head, parents.first()) {
        (None, None) => Some("HEAD"),
        (Some(head), Some(parent)) if head == *parent => Some("HEAD"),
        _ => None,
    };

    repo.commit(update_ref, &signature, &signature, message, &tree, &parent_refs).unwrap()
}

pub fn stash_tracked_change(repo: &mut Repository, relative: &str, contents: &str, message: &str) -> Oid {
    write_workdir_file(repo, relative, contents);
    let sig = repo.signature().unwrap();
    repo.stash_save(&sig, message, None).unwrap()
}

pub fn create_branch(repo: &Repository, name: &str, target: Oid) {
    let commit = repo.find_commit(target).unwrap();
    repo.branch(name, &commit, false).unwrap();
}

pub fn add_remote_path(repo: &Repository, name: &str, remote_path: &Path) {
    repo.remote(name, remote_path.to_str().unwrap()).unwrap();
}

pub fn seed_remote(repo: &Repository, remote_name: &str, refspecs: &[&str]) {
    let mut remote = repo.find_remote(remote_name).unwrap();
    remote.push(refspecs, None).unwrap();
}

pub fn source_with_origin(dir: &TestDir) -> (Repository, PathBuf) {
    let source = init_repo_at(&dir.join("source"));
    let remote_path = dir.join("remote.git");
    init_bare_repo_at(&remote_path);
    add_remote_path(&source, "origin", &remote_path);
    (source, remote_path)
}

pub fn linked_worktree_fixture(dir: &TestDir, name: &str) -> LinkedWorktreeFixture {
    let repo_path = dir.join("repo");
    let repo = init_repo_at(&repo_path);
    let base = commit_file(&repo, "file.txt", "base\n", "base");
    let linked_path = dir.join(name);
    create_worktree(&repo, name, &linked_path, base).unwrap();
    let linked_repo = Repository::open(&linked_path).unwrap();
    LinkedWorktreeFixture { repo_path, repo, linked_path, linked_repo, base }
}

pub fn parent_with_submodule(dir: &TestDir) -> (Repository, PathBuf) {
    let child_path = dir.join("child");
    let parent_path = dir.join("parent");
    let child = init_repo_at(&child_path);
    commit_file(&child, "file.txt", "hello from child\n", "child");
    drop(child);

    let parent = init_repo_at(&parent_path);
    commit_file(&parent, "file.txt", "hello from parent\n", "parent");
    let mut submodule = parent.submodule(child_path.to_str().unwrap(), Path::new("deps/child"), true).unwrap();
    submodule.clone(None).unwrap();
    submodule.add_finalize().unwrap();
    commit_index(&parent, "add submodule");
    drop(submodule);

    (parent, child_path)
}
