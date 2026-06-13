use git2::{BranchType, Error, Oid, Repository, WorktreeAddOptions, WorktreeLockStatus, WorktreePruneOptions};
use std::path::Path;

fn worktree_owner(repo: &Repository) -> Result<Repository, Error> {
    Repository::open(repo.commondir())
}

pub fn is_valid_worktree_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name != "." && name != ".." && !name.contains('/') && !name.contains('\\')
}

pub fn create_worktree(repo: &Repository, name: &str, path: &Path, target_oid: Oid) -> Result<(), Error> {
    if !is_valid_worktree_name(name) {
        return Err(Error::from_str("Worktree names cannot be empty or contain path separators"));
    }

    let repo = worktree_owner(repo)?;
    let target_commit = repo.find_commit(target_oid)?;

    let result = {
        let branch = repo.branch(name, &target_commit, false)?;
        let reference = branch.into_reference();
        let mut opts = WorktreeAddOptions::new();
        opts.reference(Some(&reference));
        repo.worktree(name, path, Some(&opts)).map(|_| ())
    };

    if let Err(error) = result {
        if let Ok(mut branch) = repo.find_branch(name, BranchType::Local) {
            let _ = branch.delete();
        }
        return Err(error);
    }

    Ok(())
}

pub fn remove_worktree(repo: &Repository, name: &str) -> Result<(), Error> {
    let repo = worktree_owner(repo)?;
    let worktree = repo.find_worktree(name)?;

    match worktree.is_locked()? {
        WorktreeLockStatus::Unlocked => {},
        WorktreeLockStatus::Locked(_) => return Err(Error::from_str("Cannot remove a locked worktree")),
    }

    let mut opts = WorktreePruneOptions::new();
    if worktree.validate().is_ok() {
        opts.valid(true).working_tree(true);
    }

    worktree.prune(Some(&mut opts))
}

pub fn lock_worktree(repo: &Repository, name: &str, reason: Option<&str>) -> Result<(), Error> {
    let repo = worktree_owner(repo)?;
    let worktree = repo.find_worktree(name)?;
    let reason = reason.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    });
    worktree.lock(reason)
}

pub fn unlock_worktree(repo: &Repository, name: &str) -> Result<(), Error> {
    let repo = worktree_owner(repo)?;
    let worktree = repo.find_worktree(name)?;
    worktree.unlock()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::queries::worktrees::list_worktrees;
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
    fn validates_v1_worktree_names() {
        assert!(is_valid_worktree_name("feature"));
        assert!(!is_valid_worktree_name(""));
        assert!(!is_valid_worktree_name("../feature"));
        assert!(!is_valid_worktree_name("topic/feature"));
        assert!(!is_valid_worktree_name("topic\\feature"));
    }

    #[test]
    fn creates_locks_unlocks_and_removes_worktree() {
        let dir = TestDir::new("worktree-actions");
        let repo_path = dir.path.join("repo");
        let worktree_path = dir.path.join("repo-feature");
        fs::create_dir_all(&repo_path).unwrap();
        let repo = init_repo(&repo_path);
        let oid = repo.head().unwrap().target().unwrap();

        create_worktree(&repo, "feature", &worktree_path, oid).unwrap();
        assert!(worktree_path.join(".git").is_file());
        assert!(repo.find_branch("feature", BranchType::Local).is_ok());

        lock_worktree(&repo, "feature", Some("keep it")).unwrap();
        let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
        let feature = entries.iter().find(|entry| entry.name == "feature").unwrap();
        assert_eq!(feature.locked_reason.as_deref(), Some("keep it"));
        assert!(remove_worktree(&repo, "feature").is_err());

        unlock_worktree(&repo, "feature").unwrap();
        remove_worktree(&repo, "feature").unwrap();
        assert!(!worktree_path.exists());
        assert!(repo.find_worktree("feature").is_err());
    }

    #[test]
    fn prunes_invalid_worktree_metadata() {
        let dir = TestDir::new("worktree-prune");
        let repo_path = dir.path.join("repo");
        let worktree_path = dir.path.join("repo-feature");
        fs::create_dir_all(&repo_path).unwrap();
        let repo = init_repo(&repo_path);
        let oid = repo.head().unwrap().target().unwrap();

        create_worktree(&repo, "feature", &worktree_path, oid).unwrap();
        fs::remove_dir_all(&worktree_path).unwrap();

        let entries = list_worktrees(&repo, Some(&repo_path)).unwrap();
        let feature = entries.iter().find(|entry| entry.name == "feature").unwrap();
        assert!(!feature.is_valid);

        remove_worktree(&repo, "feature").unwrap();
        assert!(repo.find_worktree("feature").is_err());
    }

    #[test]
    fn creates_worktree_from_linked_worktree_repository() {
        let dir = TestDir::new("worktree-from-linked");
        let repo_path = dir.path.join("repo");
        let first_path = dir.path.join("repo-feature");
        let second_path = dir.path.join("repo-second");
        fs::create_dir_all(&repo_path).unwrap();
        let repo = init_repo(&repo_path);
        let oid = repo.head().unwrap().target().unwrap();

        create_worktree(&repo, "feature", &first_path, oid).unwrap();
        let linked_repo = Repository::open(&first_path).unwrap();

        create_worktree(&linked_repo, "second", &second_path, oid).unwrap();
        assert!(second_path.join(".git").is_file());
        assert!(repo.find_worktree("second").is_ok());
    }
}
