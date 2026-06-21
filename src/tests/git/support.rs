use git2::{Oid, Repository, Signature};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct TestDir {
    pub path: PathBuf,
}

impl TestDir {
    pub fn new(prefix: &str, name: &str) -> Self {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-{prefix}-{name}-{}-{suffix}", process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn init_repo(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    let repo = Repository::init(path).unwrap();
    configure_repo(&repo);
    write(path, "file.txt", "hello\n");
    commit(&repo, "file.txt", "initial");
    repo
}

pub fn configure_repo(repo: &Repository) {
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
}

pub fn write(root: &Path, file: &str, content: &str) {
    let file_path = root.join(file);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(file_path, content).unwrap();
}

pub fn commit(repo: &Repository, file: &str, message: &str) -> Oid {
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    commit_index(repo, message)
}

#[allow(dead_code)]
pub fn stage(repo: &Repository, file: &str) {
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
}

#[allow(dead_code)]
pub fn commit_file(repo: &Repository, file: &str, contents: &str, message: &str) -> Oid {
    write(repo.workdir().unwrap(), file, contents);
    commit(repo, file, message)
}

pub fn commit_index(repo: &Repository, message: &str) -> Oid {
    let mut index = repo.index().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}
