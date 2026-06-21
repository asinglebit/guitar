use git2::{BranchType, Oid, Repository, build::CheckoutBuilder};
use im::HashSet;
use std::collections::HashMap;

pub fn checkout_head(repo: &Repository, oid: Oid) -> Result<(), git2::Error> {
    let commit = repo.find_commit(oid)?;

    // Detached checkout is used when a commit has no branch pointing at it.
    repo.set_head_detached(commit.id())?;

    repo.checkout_head(Some(
        // Force keeps the UI action decisive, matching a hard checkout of the selected commit.
        CheckoutBuilder::default().allow_conflicts(true).force(),
    ))?;

    Ok(())
}

pub fn checkout_branch(repo: &Repository, hidden_branch_names: &mut HashSet<String>, local: &mut HashMap<u32, Vec<String>>, alias: u32, branch_name: &str) -> Result<(), git2::Error> {
    fn checkout(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
        let branch = repo.find_branch(branch_name, BranchType::Local)?;
        let reference_name = branch.get().name().ok_or_else(|| git2::Error::from_str("Branch reference name is not valid UTF-8"))?;
        repo.set_head(reference_name)?;
        repo.checkout_head(Some(CheckoutBuilder::default().allow_conflicts(true).force()))
    }

    // Local branches can be checked out directly and only need visibility refreshed.
    if repo.find_branch(branch_name, BranchType::Local).is_ok() {
        let result = checkout(repo, branch_name);
        if result.is_ok() {
            hidden_branch_names.remove(branch_name);
        }
        return result;
    }

    // Remote names arrive as origin/foo; the local branch should be called foo.
    if let Some((_remote, branch)) = branch_name.split_once('/') {
        if repo.find_branch(branch, BranchType::Local).is_ok() {
            let result = checkout(repo, branch);
            if result.is_ok() {
                hidden_branch_names.remove(branch);
            }
            return result;
        }

        if repo.find_branch(branch_name, BranchType::Remote).is_ok() {
            let remote_branch = repo.find_branch(branch_name, BranchType::Remote)?;
            let commit = remote_branch.get().peel_to_commit()?;

            let mut local_branch = repo.branch(branch, &commit, false)?;
            local_branch.set_upstream(Some(branch_name))?;

            // Mirror the newly created branch in the in-memory branch map until reload rebuilds it.
            local.entry(alias).or_default().push(branch.to_string());

            let result = checkout(repo, branch);
            if result.is_ok() {
                // The checked-out local branch should remain visible under the hide-layer model.
                hidden_branch_names.remove(branch);
            }
            return result;
        }
    }

    Err(git2::Error::from_str("No matching local or remote branch found"))
}

#[cfg(test)]
#[path = "../../tests/git/actions/checkout.rs"]
mod tests;
