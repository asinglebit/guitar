use crate::git::actions::conflicts::{ensure_clean_workdir, mark_conflicts_resolved_from_workdir};
use git2::{CherrypickOptions, Error, Oid, Repository, RepositoryState, build::CheckoutBuilder};
use std::{fs, path::PathBuf};

const GUITAR_CHERRYPICK_MSG: &str = "GUITAR_CHERRYPICK_MSG";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CherrypickOutcome {
    Committed { oid: Oid },
    Conflict,
    Aborted,
}

pub fn is_cherrypick_in_progress(repo: &Repository) -> bool {
    matches!(repo.state(), RepositoryState::CherryPick | RepositoryState::CherryPickSequence)
}

fn message_path(repo: &Repository) -> PathBuf {
    repo.path().join(GUITAR_CHERRYPICK_MSG)
}

fn persist_message(repo: &Repository, message: &str) -> Result<(), Error> {
    fs::write(message_path(repo), message).map_err(|error| Error::from_str(&format!("write cherry-pick message failed: {error}")))
}

fn cleanup_message(repo: &Repository) {
    let _ = fs::remove_file(message_path(repo));
}

fn read_message(repo: &Repository) -> String {
    fs::read_to_string(message_path(repo))
        .ok()
        .filter(|message| !message.trim().is_empty())
        .or_else(|| fs::read_to_string(repo.path().join("MERGE_MSG")).ok())
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| "cherrypicked: Cherry-pick commit".to_string())
}

fn cherrypick_options<'a>() -> CherrypickOptions<'a> {
    let mut checkout = CheckoutBuilder::new();
    checkout.allow_conflicts(true).conflict_style_merge(true);

    let mut opts = CherrypickOptions::new();
    opts.checkout_builder(checkout);
    opts
}

fn commit_index(repo: &Repository, message: &str) -> Result<Oid, Error> {
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let sig = repo.signature()?;
    let head_commit = repo.head()?.peel_to_commit()?;
    let parents = [&head_commit];
    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    repo.cleanup_state()?;
    cleanup_message(repo);
    repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
    Ok(oid)
}

pub fn start_cherrypick(repo: &Repository, commit_oid: Oid, message: &str) -> Result<CherrypickOutcome, Error> {
    if is_cherrypick_in_progress(repo) {
        return Err(Error::from_str("cherry-pick already in progress"));
    }
    ensure_clean_workdir(repo, "cherry-picking")?;
    persist_message(repo, message)?;

    let commit = repo.find_commit(commit_oid)?;
    let mut opts = cherrypick_options();
    if let Err(error) = repo.cherrypick(&commit, Some(&mut opts)) {
        cleanup_message(repo);
        return Err(error);
    }

    if repo.index()?.has_conflicts() {
        return Ok(CherrypickOutcome::Conflict);
    }

    commit_index(repo, message).map(|oid| CherrypickOutcome::Committed { oid })
}

pub fn continue_cherrypick(repo: &Repository) -> Result<CherrypickOutcome, Error> {
    if !is_cherrypick_in_progress(repo) {
        return Err(Error::from_str("no cherry-pick in progress"));
    }

    mark_conflicts_resolved_from_workdir(repo)?;
    if repo.index()?.has_conflicts() {
        return Ok(CherrypickOutcome::Conflict);
    }

    let message = read_message(repo);
    commit_index(repo, &message).map(|oid| CherrypickOutcome::Committed { oid })
}

pub fn abort_cherrypick(repo: &Repository) -> Result<CherrypickOutcome, Error> {
    if !is_cherrypick_in_progress(repo) {
        return Err(Error::from_str("no cherry-pick in progress"));
    }

    repo.reset(&repo.head()?.peel_to_commit()?.into_object(), git2::ResetType::Hard, Some(CheckoutBuilder::default().force()))?;
    repo.cleanup_state()?;
    cleanup_message(repo);
    Ok(CherrypickOutcome::Aborted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::{
        fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_repo(name: &str) -> (PathBuf, Repository) {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-cherrypick-{name}-{id}"));
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
    fn clean_cherrypick_commits_with_edited_message() {
        let (path, repo) = temp_repo("clean");
        write(&path, "base.txt", "base\n");
        commit(&repo, "base.txt", "base");
        checkout_new_branch(&repo, "feature");
        write(&path, "feature.txt", "feature\n");
        let feature = commit(&repo, "feature.txt", "feature");
        checkout_branch(&repo, "master");
        write(&path, "main.txt", "main\n");
        let main = commit(&repo, "main.txt", "main");

        let outcome = start_cherrypick(&repo, feature, "cherrypicked: feature").unwrap();
        let CherrypickOutcome::Committed { oid } = outcome else {
            panic!("expected committed outcome");
        };

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.id(), oid);
        assert_eq!(head.parent(0).unwrap().id(), main);
        assert_eq!(head.summary(), Some("cherrypicked: feature"));
        assert!(!is_cherrypick_in_progress(&repo));
        assert!(!message_path(&repo).exists());
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn conflict_then_continue_commits_with_persisted_message() {
        let (path, repo) = temp_repo("conflict-continue");
        write(&path, "file.txt", "base\n");
        commit(&repo, "file.txt", "base");
        checkout_new_branch(&repo, "feature");
        write(&path, "file.txt", "feature\n");
        let feature = commit(&repo, "file.txt", "feature");
        checkout_branch(&repo, "master");
        write(&path, "file.txt", "main\n");
        commit(&repo, "file.txt", "main");

        assert_eq!(start_cherrypick(&repo, feature, "cherrypicked: feature").unwrap(), CherrypickOutcome::Conflict);
        assert!(is_cherrypick_in_progress(&repo));
        assert!(repo.index().unwrap().has_conflicts());
        assert_eq!(continue_cherrypick(&repo).unwrap(), CherrypickOutcome::Conflict);

        write(&path, "file.txt", "resolved\n");
        let outcome = continue_cherrypick(&repo).unwrap();
        let CherrypickOutcome::Committed { oid } = outcome else {
            panic!("expected committed outcome");
        };

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.id(), oid);
        assert_eq!(head.summary(), Some("cherrypicked: feature"));
        assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "resolved\n");
        assert!(!is_cherrypick_in_progress(&repo));
        assert!(!message_path(&repo).exists());
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn abort_restores_pre_cherrypick_state() {
        let (path, repo) = temp_repo("abort");
        write(&path, "file.txt", "base\n");
        commit(&repo, "file.txt", "base");
        checkout_new_branch(&repo, "feature");
        write(&path, "file.txt", "feature\n");
        let feature = commit(&repo, "file.txt", "feature");
        checkout_branch(&repo, "master");
        write(&path, "file.txt", "main\n");
        let main = commit(&repo, "file.txt", "main");

        assert_eq!(start_cherrypick(&repo, feature, "cherrypicked: feature").unwrap(), CherrypickOutcome::Conflict);
        assert_eq!(abort_cherrypick(&repo).unwrap(), CherrypickOutcome::Aborted);
        assert!(!is_cherrypick_in_progress(&repo));
        assert_eq!(repo.head().unwrap().target(), Some(main));
        assert_eq!(fs::read_to_string(path.join("file.txt")).unwrap(), "main\n");
        assert!(!message_path(&repo).exists());
        let _ = fs::remove_dir_all(path);
    }
}
