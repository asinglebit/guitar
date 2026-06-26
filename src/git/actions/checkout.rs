use crate::core::oids::git2_to_gix_oid;
use crate::git::actions::gix_support::{branch_ref_name, checkout_tree, edit_repo_config, open_worktree_repo, remote_tracking_ref_name, set_head_to_object, set_head_to_symbolic, to_git2_error};
use git2::{Oid, Repository};
use gix::bstr::ByteSlice;
use gix::refs::FullName;
use gix::refs::transaction::PreviousValue;
use im::HashSet;
use std::collections::HashMap;

fn set_branch_upstream(repo: &gix::Repository, branch: &str, remote: &str) -> Result<(), git2::Error> {
    edit_repo_config(repo, |config| {
        config.set_raw_value_by("branch", Some(branch.as_bytes().as_bstr()), "remote", remote.as_bytes().as_bstr()).map_err(to_git2_error)?;
        let merge_value = format!("refs/heads/{branch}");
        config.set_raw_value_by("branch", Some(branch.as_bytes().as_bstr()), "merge", merge_value.as_bytes().as_bstr()).map_err(to_git2_error)?;
        Ok(true)
    })
}

fn ref_commit_and_tree(repo: &mut gix::Repository, ref_name: &FullName, missing_message: &str) -> Result<(gix::hash::ObjectId, gix::hash::ObjectId), git2::Error> {
    let mut branch_ref = repo.try_find_reference(ref_name.as_ref()).map_err(to_git2_error)?.ok_or_else(|| git2::Error::from_str(missing_message))?;
    let commit = branch_ref.peel_to_commit().map_err(to_git2_error)?;
    let commit_id = commit.id;
    let tree_id = commit.tree_id().map_err(to_git2_error)?.detach();
    Ok((commit_id, tree_id))
}

fn checkout_existing_branch(repo: &mut gix::Repository, branch: &FullName) -> Result<(), git2::Error> {
    let (_, tree_id) = ref_commit_and_tree(repo, branch, "Branch not found")?;
    set_head_to_symbolic(repo, branch, "checkout")?;
    checkout_tree(repo, tree_id, false)
}

fn checkout_remote_branch(
    repo: &mut gix::Repository, branch_ref_name: &FullName, remote_ref_name: &FullName, remote: &str, branch: &str, local: &mut HashMap<u32, Vec<String>>, alias: u32,
) -> Result<(), git2::Error> {
    let (commit_id, tree_id) = ref_commit_and_tree(repo, remote_ref_name, "Remote tracking branch not found")?;
    repo.reference(branch_ref_name.clone(), commit_id, PreviousValue::MustNotExist, "checkout").map_err(to_git2_error)?;
    set_branch_upstream(repo, branch, remote)?;

    // Mirror the newly created branch in the in-memory branch map until reload rebuilds it.
    local.entry(alias).or_default().push(branch.to_string());

    set_head_to_symbolic(repo, branch_ref_name, "checkout")?;
    checkout_tree(repo, tree_id, false)?;
    Ok(())
}

pub fn checkout_head(repo: &Repository, oid: Oid) -> Result<(), git2::Error> {
    let mut repo = open_worktree_repo(repo)?;
    repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;

    let commit = repo.find_commit(git2_to_gix_oid(oid)).map_err(to_git2_error)?;
    let commit_id = commit.id;
    let tree_id = commit.tree_id().map_err(to_git2_error)?.detach();
    drop(commit);

    set_head_to_object(&repo, commit_id, "checkout")?;
    checkout_tree(&mut repo, tree_id, false)
}

pub fn checkout_branch(repo: &Repository, hidden_branch_names: &mut HashSet<String>, local: &mut HashMap<u32, Vec<String>>, alias: u32, branch_name: &str) -> Result<(), git2::Error> {
    let mut repo = open_worktree_repo(repo)?;
    repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;

    let local_branch_name = branch_ref_name(branch_name)?;
    if repo.try_find_reference(local_branch_name.as_ref()).map_err(to_git2_error)?.is_some() {
        let result = checkout_existing_branch(&mut repo, &local_branch_name);
        if result.is_ok() {
            hidden_branch_names.remove(branch_name);
        }
        return result;
    }

    if let Some((remote, branch)) = branch_name.split_once('/') {
        let local_branch_ref_name = branch_ref_name(branch)?;
        if repo.try_find_reference(local_branch_ref_name.as_ref()).map_err(to_git2_error)?.is_some() {
            let result = checkout_existing_branch(&mut repo, &local_branch_ref_name);
            if result.is_ok() {
                hidden_branch_names.remove(branch);
            }
            return result;
        }

        let remote_ref_name = remote_tracking_ref_name(branch_name)?;
        if repo.try_find_reference(remote_ref_name.as_ref()).map_err(to_git2_error)?.is_some() {
            let result = checkout_remote_branch(&mut repo, &local_branch_ref_name, &remote_ref_name, remote, branch, local, alias);
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
