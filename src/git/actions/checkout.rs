use git2::{BranchType, Oid, Repository, build::CheckoutBuilder};
use std::collections::HashMap;

pub fn checkout_head(repo: &Repository, oid: Oid) {
    // Find the commit object
    let commit = repo.find_commit(oid).unwrap();

    // Set HEAD to the commit (detached)
    repo.set_head_detached(commit.id()).unwrap();

    // Checkout the commit
    repo.checkout_head(Some(
        CheckoutBuilder::default().allow_conflicts(true).force(), // optional: force overwrite local changes
    ))
    .expect("Error checking out");
}

pub fn checkout_branch(repo: &Repository, visible: &mut HashMap<u32, Vec<String>>, local: &mut HashMap<u32, Vec<String>>, alias: u32, branch_name: &str) -> Result<(), git2::Error> {
    // Helper to checkout a local branch
    fn checkout(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
        let branch = repo.find_branch(branch_name, BranchType::Local)?;
        repo.set_head(branch.get().name().unwrap())?;
        repo.checkout_head(Some(CheckoutBuilder::default().allow_conflicts(true).force()))
    }

    // If branch_name already exists as a local branch, checkout directly
    if repo.find_branch(branch_name, BranchType::Local).is_ok() {
        return checkout(repo, branch_name);
    }

    // If branch_name is in the form <remote>/<branch>
    if let Some((_remote, branch)) = branch_name.split_once('/') {
        if repo.find_branch(branch, BranchType::Local).is_ok() {
            return checkout(repo, branch);
        }

        if repo.find_branch(branch_name, BranchType::Remote).is_ok() {
            let remote_branch = repo.find_branch(branch_name, BranchType::Remote)?;
            let commit = remote_branch.get().peel_to_commit()?;

            let mut local_branch = repo.branch(branch, &commit, false)?;
            local_branch.set_upstream(Some(branch_name))?;
            local.entry(alias).or_default().push(branch.to_string());
            visible.entry(alias).or_default().push(branch.to_string());

            return checkout(repo, branch);
        }
    }

    Err(git2::Error::from_str("No matching local or remote branch found for the given Oid"))
}
