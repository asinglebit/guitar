use git2::{build::CheckoutBuilder, BranchType, Cred, Error, ErrorCode, FetchOptions, Oid, PushOptions, RemoteCallbacks, Repository, ResetType, Signature, StatusOptions};
use git2::{CherrypickOptions, FetchPrune, StashApplyOptions, StashFlags};
use std::{collections::HashMap, thread};

pub fn commit_staged(repo: &Repository, message: &str, name: &str, email: &str) -> Result<Oid, Error> {
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Determine parent commit
    let parent_commit = match repo.head() {
        Ok(head_ref) => {
            // Try to peel to commit
            head_ref.peel_to_commit().ok()
        },
        Err(e) => {
            if e.code() == ErrorCode::UnbornBranch {
                None // empty repo, initial commit
            } else {
                return Err(e);
            }
        },
    };

    let signature = Signature::now(name, email)?;

    let commit_oid = if let Some(parent) = parent_commit {
        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent])?
    } else {
        // Initial commit
        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?
    };

    Ok(commit_oid)
}
