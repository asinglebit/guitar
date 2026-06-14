use crate::git::{
    actions::conflicts::{ensure_clean_workdir, mark_conflicts_resolved_from_workdir},
    queries::commits::get_current_branch,
};
use git2::{Error, Oid, Rebase, RebaseOptions, Repository, RepositoryState, build::CheckoutBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseOutcome {
    Completed { applied: usize },
    Conflict,
    Aborted,
}

fn is_rebase_state(state: RepositoryState) -> bool {
    matches!(state, RepositoryState::Rebase | RepositoryState::RebaseInteractive | RepositoryState::RebaseMerge | RepositoryState::ApplyMailboxOrRebase)
}

pub fn is_rebase_in_progress(repo: &Repository) -> bool {
    is_rebase_state(repo.state())
}

fn rebase_options<'a>() -> RebaseOptions<'a> {
    let mut checkout = CheckoutBuilder::new();
    checkout.allow_conflicts(true).conflict_style_merge(true);

    let mut opts = RebaseOptions::new();
    opts.checkout_options(checkout);
    opts
}

fn drive_rebase(repo: &Repository, rebase: &mut Rebase<'_>, mut applied: usize) -> Result<RebaseOutcome, Error> {
    let signature = repo.signature()?;

    loop {
        match rebase.next() {
            Some(Ok(_)) => {
                if repo.index()?.has_conflicts() {
                    return Ok(RebaseOutcome::Conflict);
                }
                rebase.commit(None, &signature, None)?;
                applied += 1;
            },
            Some(Err(error)) => return Err(error),
            None => {
                rebase.finish(Some(&signature))?;
                return Ok(RebaseOutcome::Completed { applied });
            },
        }
    }
}

pub fn start_rebase(repo: &Repository, upstream_oid: Oid) -> Result<RebaseOutcome, Error> {
    if get_current_branch(repo).is_none() {
        return Err(Error::from_str("rebasing requires a checked-out local branch"));
    }
    if is_rebase_in_progress(repo) {
        return Err(Error::from_str("rebase already in progress"));
    }

    let head_oid = repo.head()?.target().ok_or_else(|| Error::from_str("HEAD does not point to a commit"))?;
    if head_oid == upstream_oid {
        return Err(Error::from_str("selected commit is already HEAD"));
    }

    ensure_clean_workdir(repo, "rebasing")?;

    let upstream = repo.find_annotated_commit(upstream_oid)?;
    let mut opts = rebase_options();
    let mut rebase = repo.rebase(None, Some(&upstream), None, Some(&mut opts))?;
    drive_rebase(repo, &mut rebase, 0)
}

pub fn continue_rebase(repo: &Repository) -> Result<RebaseOutcome, Error> {
    if !is_rebase_in_progress(repo) {
        return Err(Error::from_str("no rebase in progress"));
    }

    mark_conflicts_resolved_from_workdir(repo)?;
    if repo.index()?.has_conflicts() {
        return Ok(RebaseOutcome::Conflict);
    }

    let mut opts = rebase_options();
    let mut rebase = repo.open_rebase(Some(&mut opts))?;
    let signature = repo.signature()?;
    let mut applied = 0;

    if rebase.operation_current().is_some() {
        rebase.commit(None, &signature, None)?;
        applied += 1;
    }

    drive_rebase(repo, &mut rebase, applied)
}

pub fn abort_rebase(repo: &Repository) -> Result<RebaseOutcome, Error> {
    if !is_rebase_in_progress(repo) {
        return Err(Error::from_str("no rebase in progress"));
    }

    let mut opts = rebase_options();
    let mut rebase = repo.open_rebase(Some(&mut opts))?;
    rebase.abort()?;
    Ok(RebaseOutcome::Aborted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_repo(name: &str) -> (PathBuf, Repository) {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-rebase-{name}-{id}"));
        fs::create_dir_all(&path).unwrap();
        let repo = Repository::init(&path).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        (path, repo)
    }

    fn write(path: &Path, file: &str, content: &str) {
        fs::write(path.join(file), content).unwrap();
    }

    fn commit(repo: &Repository, file: &str, message: &str) -> Oid {
        let workdir = repo.workdir().unwrap().to_path_buf();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
        let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap();
        assert!(workdir.join(file).exists());
        oid
    }

    fn checkout_new_branch(repo: &Repository, name: &str) {
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch(name, &head, false).unwrap();
        repo.set_head(&format!("refs/heads/{name}")).unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
    }

    fn checkout_branch(repo: &Repository, name: &str) {
        repo.set_head(&format!("refs/heads/{name}")).unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
    }

    #[test]
    fn clean_rebase_completes_and_updates_branch() {
        let (path, repo) = temp_repo("clean");
        write(&path, "file.txt", "base\n");
        let base = commit(&repo, "file.txt", "base");
        checkout_new_branch(&repo, "feature");
        write(&path, "feature.txt", "feature\n");
        commit(&repo, "feature.txt", "feature");
        checkout_branch(&repo, "master");
        write(&path, "main.txt", "main\n");
        let main = commit(&repo, "main.txt", "main");
        checkout_branch(&repo, "feature");

        let outcome = start_rebase(&repo, main).unwrap();
        assert_eq!(outcome, RebaseOutcome::Completed { applied: 1 });
        assert_eq!(repo.head().unwrap().shorthand(), Some("feature"));
        assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().parent(0).unwrap().id(), main);
        assert_ne!(repo.head().unwrap().target(), Some(base));
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn dirty_worktree_is_refused_before_start() {
        let (path, repo) = temp_repo("dirty");
        write(&path, "file.txt", "base\n");
        commit(&repo, "file.txt", "base");
        checkout_new_branch(&repo, "feature");
        write(&path, "feature.txt", "feature\n");
        commit(&repo, "feature.txt", "feature");
        checkout_branch(&repo, "master");
        write(&path, "main.txt", "main\n");
        let main = commit(&repo, "main.txt", "main");
        checkout_branch(&repo, "feature");
        write(&path, "file.txt", "dirty\n");

        let error = start_rebase(&repo, main).unwrap_err();
        assert!(error.message().contains("working tree must be clean"));
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn conflict_then_continue_finishes() {
        let (path, repo) = temp_repo("conflict-continue");
        write(&path, "file.txt", "base\n");
        commit(&repo, "file.txt", "base");
        checkout_new_branch(&repo, "feature");
        write(&path, "file.txt", "feature\n");
        commit(&repo, "file.txt", "feature");
        checkout_branch(&repo, "master");
        write(&path, "file.txt", "main\n");
        let main = commit(&repo, "file.txt", "main");
        checkout_branch(&repo, "feature");

        assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);
        assert!(is_rebase_in_progress(&repo));
        assert!(repo.index().unwrap().has_conflicts());

        write(&path, "file.txt", "resolved\n");
        assert_eq!(continue_rebase(&repo).unwrap(), RebaseOutcome::Completed { applied: 1 });
        assert!(!is_rebase_in_progress(&repo));
        assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "resolved\n");
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn abort_restores_pre_rebase_state() {
        let (path, repo) = temp_repo("abort");
        write(&path, "file.txt", "base\n");
        commit(&repo, "file.txt", "base");
        checkout_new_branch(&repo, "feature");
        write(&path, "file.txt", "feature\n");
        let original_feature = commit(&repo, "file.txt", "feature");
        checkout_branch(&repo, "master");
        write(&path, "file.txt", "main\n");
        let main = commit(&repo, "file.txt", "main");
        checkout_branch(&repo, "feature");

        assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);
        assert_eq!(abort_rebase(&repo).unwrap(), RebaseOutcome::Aborted);
        assert!(!is_rebase_in_progress(&repo));
        assert_eq!(repo.head().unwrap().target(), Some(original_feature));
        assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "feature\n");
        let _ = fs::remove_dir_all(path);
    }
}
