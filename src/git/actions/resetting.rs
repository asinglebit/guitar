use std::path::Path;

use git2::{Error, Oid, Repository, ResetType};

pub fn reset_to_commit(repo: &Repository, target: Oid, reset_type: ResetType) -> Result<(), Error> {
    // Resolve the target commit object
    let target_commit = repo.find_commit(target)?;

    // Get HEAD reference
    let head = repo.head()?;

    if head.is_branch() {
        // Normal branch: move branch reference
        let branch_ref_name = head.name().ok_or_else(|| Error::from_str("Invalid branch reference name"))?;
        let mut branch_ref = repo.find_reference(branch_ref_name)?;
        branch_ref.set_target(target, "reset branch to commit")?;
    } else {
        // Detached HEAD: move HEAD directly
        let head_ref_name = head.name().unwrap_or("HEAD");
        let mut head_ref_obj = repo.find_reference(head_ref_name)?;
        head_ref_obj.set_target(target, "reset detached HEAD")?;
    }

    // Perform the reset (Hard, Soft, or Mixed)
    repo.reset(&target_commit.into_object(), reset_type, None)?;

    Ok(())
}

// Resets a file to the state in HEAD (unstages it and discards working directory changes)
pub fn reset_file(repo: &Repository, path: &Path) -> Result<(), Error> {

    // Get HEAD tree
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let tree = commit.tree()?;

    // Checkout the file from the tree
    repo.checkout_tree(
        tree.as_object(),
        Some(git2::build::CheckoutBuilder::new().force().path(path)),
    )?;

    // Remove from index if staged
    let mut index = repo.index()?;
    index.remove_path(path)?;
    index.write()?;

    Ok(())
}
