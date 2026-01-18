use git2::{build::CheckoutBuilder, BranchType, Cred, Error, ErrorCode, FetchOptions, Oid, PushOptions, RemoteCallbacks, Repository, ResetType, Signature, StatusOptions};
use git2::{CherrypickOptions, FetchPrune, StashApplyOptions, StashFlags};
use std::{collections::HashMap, thread};

pub fn stash(repo: &mut Repository) -> Result<Oid, git2::Error> {
    let flags = StashFlags::DEFAULT | StashFlags::INCLUDE_UNTRACKED;

    let message = {
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let short_id = commit.id().to_string()[..7].to_string();
        let summary = commit.summary().unwrap_or("WIP");
        format!("{} {}", short_id, summary)
    };

    let stash_index = repo.stash_save(&repo.signature()?, message.as_str(), Some(flags))?;

    Ok(stash_index)
}

pub fn pop(repo: &mut Repository, target_oid: &Oid, apply: bool) -> Result<(), git2::Error> {
    let mut stash_index: Option<usize> = None;

    repo.stash_foreach(|index, _message, oid| {
        if oid == target_oid {
            stash_index = Some(index);
            false
        } else {
            true
        }
    })?;

    if let Some(index) = stash_index {
        if apply {
            let mut opts = StashApplyOptions::new();
            repo.stash_apply(index, Some(&mut opts))?;
        }
        repo.stash_drop(index)?;
    }

    Ok(())
}
