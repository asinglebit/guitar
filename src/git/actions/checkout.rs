use crate::core::oids::git2_to_gix_oid;
use crate::git::actions::gix_support::{branch_ref_name, checkout_tree, edit_repo_config, open_worktree_repo, remote_tracking_ref_name, set_head_to_object, set_head_to_symbolic, to_git2_error};
use git2::{Oid, Repository};
use gix::bstr::ByteSlice;
use gix::refs::FullName;
use gix::refs::transaction::PreviousValue;
use im::HashSet;
use std::collections::HashMap;

fn set_branch_upstream(repo: &gix::Repository, branch: &str, remote: &str) -> Result<(), git2::Error> {
    let merge_ref = branch_ref_name(branch)?;
    let branch_name = branch.as_bytes().as_bstr();
    edit_repo_config(repo, |config| {
        config.set_raw_value_by("branch", Some(branch_name), "remote", remote.as_bytes().as_bstr()).map_err(to_git2_error)?;
        config.set_raw_value_by("branch", Some(branch_name), "merge", merge_ref.as_ref()).map_err(to_git2_error)?;
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

enum BranchCheckout<'a> {
    Existing { ref_name: FullName, visible_name: &'a str },
    Remote { local_ref_name: FullName, remote_ref_name: FullName, remote: &'a str, branch: &'a str },
}

enum BranchLookup<'a> {
    Local { branch: &'a str },
    Remote { branch_name: &'a str, remote: &'a str, branch: &'a str },
}

impl<'a> BranchCheckout<'a> {
    fn apply(self, repo: &mut gix::Repository, local: &mut HashMap<u32, Vec<String>>, alias: u32) -> Result<&'a str, git2::Error> {
        match self {
            BranchCheckout::Existing { ref_name, visible_name } => checkout_existing_branch(repo, &ref_name).map(|()| visible_name),
            BranchCheckout::Remote { local_ref_name, remote_ref_name, remote, branch } => {
                checkout_remote_branch(repo, &local_ref_name, &remote_ref_name, remote, branch, local, alias).map(|()| branch)
            },
        }
    }
}

fn reference_exists(repo: &gix::Repository, ref_name: &FullName) -> Result<bool, git2::Error> {
    repo.try_find_reference(ref_name.as_ref()).map(|reference| reference.is_some()).map_err(to_git2_error)
}

impl<'a> BranchLookup<'a> {
    fn candidate(self, repo: &gix::Repository) -> Result<Option<BranchCheckout<'a>>, git2::Error> {
        match self {
            BranchLookup::Local { branch } => {
                let ref_name = branch_ref_name(branch)?;
                Ok(reference_exists(repo, &ref_name)?.then_some(BranchCheckout::Existing { ref_name, visible_name: branch }))
            },
            BranchLookup::Remote { branch_name, remote, branch } => {
                let local_ref_name = branch_ref_name(branch)?;
                let remote_ref_name = (!reference_exists(repo, &local_ref_name)?).then(|| remote_tracking_ref_name(branch_name)).transpose()?;

                match remote_ref_name {
                    None => Ok(Some(BranchCheckout::Existing { ref_name: local_ref_name, visible_name: branch })),
                    Some(remote_ref_name) => Ok(reference_exists(repo, &remote_ref_name)?.then_some(BranchCheckout::Remote { local_ref_name, remote_ref_name, remote, branch })),
                }
            },
        }
    }
}

fn branch_lookups(branch_name: &str) -> impl Iterator<Item = BranchLookup<'_>> {
    let local = std::iter::once(BranchLookup::Local { branch: branch_name });
    let remote = branch_name.split_once('/').into_iter().map(|(remote, branch)| BranchLookup::Remote { branch_name, remote, branch });

    local.chain(remote)
}

fn branch_checkout_candidate<'a>(repo: &gix::Repository, branch_name: &'a str) -> Result<Option<BranchCheckout<'a>>, git2::Error> {
    branch_lookups(branch_name).find_map(|lookup| lookup.candidate(repo).transpose()).transpose()
}

pub fn checkout_branch(repo: &Repository, hidden_branch_names: &mut HashSet<String>, local: &mut HashMap<u32, Vec<String>>, alias: u32, branch_name: &str) -> Result<(), git2::Error> {
    let mut repo = open_worktree_repo(repo)?;
    repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;

    let candidate = branch_checkout_candidate(&repo, branch_name)?.ok_or_else(|| git2::Error::from_str("No matching local or remote branch found"))?;
    let visible_name = candidate.apply(&mut repo, local, alias)?;
    hidden_branch_names.remove(visible_name);
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/git/actions/checkout.rs"]
mod tests;
