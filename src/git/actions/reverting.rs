use crate::git::actions::conflicts::{ensure_clean_workdir, mark_conflicts_resolved_from_workdir};
use git2::{Error, Oid, Repository, RepositoryState, RevertOptions, build::CheckoutBuilder};
use std::{fs, path::PathBuf};

const GUITAR_REVERT_MSG: &str = "GUITAR_REVERT_MSG";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevertOutcome {
    Committed { oid: Oid },
    Conflict,
    Aborted,
}

pub fn is_revert_in_progress(repo: &Repository) -> bool {
    matches!(repo.state(), RepositoryState::Revert | RepositoryState::RevertSequence)
}

fn message_path(repo: &Repository) -> PathBuf {
    repo.path().join(GUITAR_REVERT_MSG)
}

fn persist_message(repo: &Repository, message: &str) -> Result<(), Error> {
    fs::write(message_path(repo), message).map_err(|error| Error::from_str(&format!("write revert message failed: {error}")))
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
        .unwrap_or_else(|| "reverted: Revert commit".to_string())
}

fn revert_options<'a>() -> RevertOptions<'a> {
    let mut checkout = CheckoutBuilder::new();
    checkout.allow_conflicts(true).conflict_style_merge(true);

    let mut opts = RevertOptions::new();
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

pub fn start_revert(repo: &Repository, commit_oid: Oid, message: &str) -> Result<RevertOutcome, Error> {
    if is_revert_in_progress(repo) {
        return Err(Error::from_str("revert already in progress"));
    }
    ensure_clean_workdir(repo, "reverting")?;

    let commit = repo.find_commit(commit_oid)?;
    if commit.parent_count() > 1 {
        return Err(Error::from_str("reverting merge commits is not supported"));
    }

    persist_message(repo, message)?;

    let mut opts = revert_options();
    if let Err(error) = repo.revert(&commit, Some(&mut opts)) {
        cleanup_message(repo);
        return Err(error);
    }

    if repo.index()?.has_conflicts() {
        return Ok(RevertOutcome::Conflict);
    }

    commit_index(repo, message).map(|oid| RevertOutcome::Committed { oid })
}

pub fn continue_revert(repo: &Repository) -> Result<RevertOutcome, Error> {
    if !is_revert_in_progress(repo) {
        return Err(Error::from_str("no revert in progress"));
    }

    mark_conflicts_resolved_from_workdir(repo)?;
    if repo.index()?.has_conflicts() {
        return Ok(RevertOutcome::Conflict);
    }

    let message = read_message(repo);
    commit_index(repo, &message).map(|oid| RevertOutcome::Committed { oid })
}

pub fn abort_revert(repo: &Repository) -> Result<RevertOutcome, Error> {
    if !is_revert_in_progress(repo) {
        return Err(Error::from_str("no revert in progress"));
    }

    repo.reset(&repo.head()?.peel_to_commit()?.into_object(), git2::ResetType::Hard, Some(CheckoutBuilder::default().force()))?;
    repo.cleanup_state()?;
    cleanup_message(repo);
    Ok(RevertOutcome::Aborted)
}

#[cfg(test)]
#[path = "../../tests/git/actions/reverting.rs"]
mod tests;
