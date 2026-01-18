use git2::{build::CheckoutBuilder, BranchType, Cred, Error, ErrorCode, FetchOptions, Oid, PushOptions, RemoteCallbacks, Repository, ResetType, Signature, StatusOptions};
use git2::{CherrypickOptions, FetchPrune, StashApplyOptions, StashFlags};
use std::{collections::HashMap, thread};

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
