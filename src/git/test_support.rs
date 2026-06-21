use git2::{Oid, Repository};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(name: &str) -> Self {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = env::temp_dir().join(format!("guitar-test-support-{name}-{}-{suffix}", process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.path.join(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn configure_user(repo: &Repository) {
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
}

pub fn init_repo_at(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    let repo = Repository::init(path).unwrap();
    configure_user(&repo);
    repo
}

pub fn init_bare_repo_at(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    Repository::init_bare(path).unwrap()
}

pub fn clone_repo(source: &Path, target: &Path) -> Repository {
    let repo = Repository::clone(source.to_str().unwrap(), target).unwrap();
    configure_user(&repo);
    repo
}

pub fn write_workdir_file(repo: &Repository, relative: &str, contents: &str) {
    let workdir = repo.workdir().unwrap();
    let full_path = workdir.join(relative);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full_path, contents).unwrap();
}

pub fn stage_path(repo: &Repository, relative: &str) {
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}

pub fn commit_index(repo: &Repository, message: &str) -> Oid {
    let mut index = repo.index().unwrap();
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

pub fn create_branch(repo: &Repository, name: &str, target: Oid) {
    let commit = repo.find_commit(target).unwrap();
    repo.branch(name, &commit, false).unwrap();
}

pub fn add_remote_path(repo: &Repository, name: &str, remote_path: &Path) {
    repo.remote(name, remote_path.to_str().unwrap()).unwrap();
}

pub fn add_remote_url(repo: &Repository, name: &str, remote_url: &str) {
    repo.remote(name, remote_url).unwrap();
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

#[cfg(test)]
#[path = "../tests/git/test_support.rs"]
mod tests;
