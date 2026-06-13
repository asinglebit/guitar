use std::path::{Path, PathBuf};

// Walk upward from any path until a non-bare repository root is found.
pub fn try_into_git_repo_root(start_path: impl AsRef<Path>) -> Option<PathBuf> {
    let mut current_path = start_path.as_ref();
    if current_path.is_file() {
        current_path = current_path.parent()?;
    }

    loop {
        let git_path = current_path.join(".git");
        if git_path.exists() {
            return Some(current_path.to_path_buf());
        }

        if let Some(parent) = current_path.parent() {
            current_path = parent;
        } else {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env, fs,
        path::PathBuf,
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

    #[test]
    fn finds_repo_root_with_git_directory() {
        let dir = TestDir::new("root-dir");
        let repo = dir.path.join("repo");
        let nested = repo.join("src/app");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(try_into_git_repo_root(&nested).as_deref(), Some(repo.as_path()));
        assert_eq!(try_into_git_repo_root(&repo).as_deref(), Some(repo.as_path()));
    }

    #[test]
    fn finds_worktree_root_with_git_file() {
        let dir = TestDir::new("root-file");
        let worktree = dir.path.join("repo-feature");
        let nested = worktree.join("src/app");
        fs::create_dir_all(&nested).unwrap();
        fs::write(worktree.join(".git"), "gitdir: ../repo/.git/worktrees/repo-feature\n").unwrap();

        assert_eq!(try_into_git_repo_root(&nested).as_deref(), Some(worktree.as_path()));
        assert_eq!(try_into_git_repo_root(&worktree).as_deref(), Some(worktree.as_path()));
    }
}
