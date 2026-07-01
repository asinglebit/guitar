use std::path::Path;

use git2::{Error, Oid, Repository, ResetType};

pub fn reset_to_commit(repo: &Repository, target: Oid, reset_type: ResetType) -> Result<(), Error> {
    let target_commit = repo.find_commit(target)?;

    let head = repo.head()?;

    if head.is_branch() {
        // Branch reset moves the checked-out ref before libgit2 updates index/workdir state.
        let branch_ref_name = head.name().ok_or_else(|| Error::from_str("Invalid branch reference name"))?;
        let mut branch_ref = repo.find_reference(branch_ref_name)?;
        branch_ref.set_target(target, "reset branch to commit")?;
    } else {
        // Detached reset has no branch ref to move, so HEAD itself becomes the target.
        let head_ref_name = head.name().unwrap_or("HEAD");
        let mut head_ref_obj = repo.find_reference(head_ref_name)?;
        head_ref_obj.set_target(target, "reset detached HEAD")?;
    }

    // ResetType controls whether workdir and index are rewritten after the ref move.
    repo.reset(&target_commit.into_object(), reset_type, None)?;

    Ok(())
}

// Reset one path to HEAD, removing both staged and working tree changes for that file.
pub fn reset_file(repo: &Repository, path: &Path) -> Result<(), Error> {
    // Remove any staged entry first so checkout_tree can restore a clean copy from HEAD.
    let mut index = repo.index()?;
    index.remove_path(path)?;
    index.write()?;

    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let tree = commit.tree()?;

    repo.checkout_tree(tree.as_object(), Some(git2::build::CheckoutBuilder::new().force().path(path)))?;

    Ok(())
}

#[cfg(test)]
#[path = "../../tests/git/actions/resetting.rs"]
mod tests;
