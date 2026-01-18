use git2::{BranchType, Error, Oid, Repository};

use crate::git::actions::pushing::delete_remote_branch_ssh;


pub fn create_branch(repo: &Repository, branch_name: &str, target_oid: Oid) -> Result<(), Error> {
    // Find the commit you want the branch to point to
    let target_commit = repo.find_commit(target_oid)?;

    // Create the branch
    repo.branch(branch_name, &target_commit, false)?;

    Ok(())
}

pub fn delete_branch(repo: &Repository, branch: &str) -> Result<(), git2::Error> {
    // Try deleting as a local branch first
    if let Ok(mut local) = repo.find_branch(branch, BranchType::Local) {
        local.delete()?;
        return Ok(());
    }

    // Not found locally → assume remote branch
    // Split at first `/` to get remote and branch name
    let (remote_name, remote_branch) = if let Some((remote, b)) = branch.split_once('/') { (remote, b) } else { ("origin", branch) };

    // Use thread-based SSH deletion
    let repo_path = repo.path().to_str().ok_or_else(|| git2::Error::from_str("Invalid repo path"))?;
    let handle = delete_remote_branch_ssh(repo_path, remote_name, remote_branch);

    // Wait for the thread to finish and propagate errors
    handle.join().map_err(|_| git2::Error::from_str("Failed to join remote deletion thread"))??;

    Ok(())
}
