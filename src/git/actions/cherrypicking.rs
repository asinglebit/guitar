use git2::{build::CheckoutBuilder, BranchType, Cred, Error, ErrorCode, FetchOptions, Oid, PushOptions, RemoteCallbacks, Repository, ResetType, Signature, StatusOptions};
use git2::{CherrypickOptions, FetchPrune, StashApplyOptions, StashFlags};
use std::{collections::HashMap, thread};

pub fn cherry_pick_commit(
    repo: &Repository,
    commit_oid: Oid,
    message: Option<&str>, // optional override for commit message
    allow_conflicts: bool, // true -> force working dir changes
) -> Result<Oid, Error> {
    // Find the commit to cherry-pick
    let commit = repo.find_commit(commit_oid)?;

    // Get current HEAD commit
    let head_commit = repo.head()?.peel_to_commit()?;

    // Prepare cherry-pick options
    let mut cherrypick_opts = CherrypickOptions::new();

    // Perform cherry-pick
    repo.cherrypick(&commit, Some(&mut cherrypick_opts))?;

    // Get the index after cherry-pick
    let mut index = repo.index()?;

    // If conflicts exist
    if index.has_conflicts() {
        if allow_conflicts {
            let conflicts: Vec<_> = index.conflicts()?.flatten().filter_map(|e| e.our).collect();
            for conflict in conflicts {
                index.add(&conflict)?;
            }
        } else {
            return Err(Error::from_str("Cherry-pick conflicts detected"));
        }
    }

    // Write tree
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Create commit signature
    let sig = repo.signature()?;

    // Commit message
    let commit_message = message.unwrap_or_else(|| commit.message().unwrap_or("Cherry-pick commit"));

    // Determine parents: HEAD
    let parents = [&head_commit];

    // Create the new commit
    let new_commit_oid = repo.commit(Some("HEAD"), &sig, &sig, commit_message, &tree, &parents)?;

    // Update working directory
    repo.checkout_head(Some(CheckoutBuilder::default().allow_conflicts(allow_conflicts).force()))?;

    Ok(new_commit_oid)
}
