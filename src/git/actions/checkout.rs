use git2::{build::CheckoutBuilder, BranchType, Oid, Repository};
use im::HashSet;
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

pub fn checkout_branch(repo: &Repository, visible_branch_names: &mut HashSet<String>, local: &mut HashMap<u32, Vec<String>>, alias: u32, branch_name: &str) -> Result<(), git2::Error> {
    fn checkout(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
        let branch = repo.find_branch(branch_name, BranchType::Local)?;
        repo.set_head(branch.get().name().unwrap())?;
        repo.checkout_head(Some(CheckoutBuilder::default().allow_conflicts(true).force()))
    }

    // Already local
    if repo.find_branch(branch_name, BranchType::Local).is_ok() {
        if !visible_branch_names.is_empty() {
            visible_branch_names.insert(branch_name.to_string());
        }
        return checkout(repo, branch_name);
    }

    // Remote case: origin/foo
    if let Some((_remote, branch)) = branch_name.split_once('/') {
        if repo.find_branch(branch, BranchType::Local).is_ok() {
            if !visible_branch_names.is_empty() {
                visible_branch_names.insert(branch.to_string());
            }
            return checkout(repo, branch);
        }

        if repo.find_branch(branch_name, BranchType::Remote).is_ok() {
            let remote_branch = repo.find_branch(branch_name, BranchType::Remote)?;
            let commit = remote_branch.get().peel_to_commit()?;

            let mut local_branch = repo.branch(branch, &commit, false)?;
            local_branch.set_upstream(Some(branch_name))?;

            // Track locally
            local.entry(alias).or_default().push(branch.to_string());

            // Make visible (UI)
            if !visible_branch_names.is_empty() {
                visible_branch_names.insert(branch.to_string());
            }

            return checkout(repo, branch);
        }
    }

    Err(git2::Error::from_str("No matching local or remote branch found"))
}
