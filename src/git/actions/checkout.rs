use crate::core::oids::git2_to_gix_oid;
use git2::{Oid, Repository};
use im::HashSet;
use std::{collections::HashMap, fs::OpenOptions, io::Write, sync::atomic::AtomicBool};

use gix::bstr::ByteSlice;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use gix::refs::{FullName, Target};

fn gix_error(error: impl std::fmt::Display) -> git2::Error {
    git2::Error::from_str(&error.to_string())
}

fn open_repo(repo: &Repository) -> Result<gix::Repository, git2::Error> {
    let workdir = repo.workdir().ok_or_else(|| git2::Error::from_str("Repository has no worktree"))?;
    gix::open(workdir).map_err(gix_error)
}

fn checkout_log() -> LogChange {
    LogChange { mode: RefLog::AndReference, force_create_reflog: false, message: "checkout".into() }
}

fn head_ref_name() -> FullName {
    "HEAD".try_into().expect("HEAD is valid")
}

fn local_branch_ref_name(branch: &str) -> Result<FullName, git2::Error> {
    format!("refs/heads/{branch}").try_into().map_err(gix_error)
}

fn remote_tracking_ref_name(branch_name: &str) -> Result<FullName, git2::Error> {
    format!("refs/remotes/{branch_name}").try_into().map_err(gix_error)
}

fn load_local_config(path: std::path::PathBuf, source: gix::config::Source) -> Result<gix::config::File<'static>, git2::Error> {
    gix::config::File::from_path_no_includes(path, source).map_err(gix_error)
}

fn write_local_config(config: &gix::config::File<'static>, source: gix::config::Source) -> Result<(), git2::Error> {
    let path = config.meta().path.as_deref().ok_or_else(|| git2::Error::from_str("Configuration path is missing"))?;
    let mut file = OpenOptions::new().create(false).write(true).truncate(true).open(path).map_err(gix_error)?;

    file.write_all(config.detect_newline_style()).map_err(gix_error)?;
    config.write_to_filter(&mut file, |section| section.meta().source == source).map_err(gix_error)
}

fn set_head_detached(repo: &gix::Repository, commit_id: gix::hash::ObjectId) -> Result<(), git2::Error> {
    repo.edit_reference(RefEdit { change: Change::Update { log: checkout_log(), expected: PreviousValue::Any, new: Target::Object(commit_id) }, name: head_ref_name(), deref: false })
        .map_err(gix_error)?;
    Ok(())
}

fn set_head_symbolic(repo: &gix::Repository, branch: &FullName) -> Result<(), git2::Error> {
    repo.edit_reference(RefEdit { change: Change::Update { log: checkout_log(), expected: PreviousValue::Any, new: Target::Symbolic(branch.clone()) }, name: head_ref_name(), deref: false })
        .map_err(gix_error)?;
    Ok(())
}

fn set_branch_upstream(repo: &gix::Repository, branch: &str, remote: &str) -> Result<(), git2::Error> {
    let config_snapshot = repo.config_snapshot();
    let snapshot = config_snapshot.plumbing();
    let source = snapshot.meta().source;
    let path = snapshot.meta().path.clone().ok_or_else(|| git2::Error::from_str("Configuration path is missing"))?;
    let mut config = load_local_config(path, source)?;

    config.set_raw_value_by("branch", Some(branch.as_bytes().as_bstr()), "remote", remote.as_bytes().as_bstr()).map_err(gix_error)?;
    let merge_value = format!("refs/heads/{branch}");
    config.set_raw_value_by("branch", Some(branch.as_bytes().as_bstr()), "merge", merge_value.as_bytes().as_bstr()).map_err(gix_error)?;
    write_local_config(&config, source)
}

fn checkout_worktree(repo: &mut gix::Repository, tree_id: gix::hash::ObjectId) -> Result<(), git2::Error> {
    let mut index = repo.index_from_tree(&tree_id).map_err(gix_error)?;
    let mut options = repo.checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping).map_err(gix_error)?;
    options.overwrite_existing = true;

    let files = gix::features::progress::Discard;
    let bytes = gix::features::progress::Discard;
    let should_interrupt = AtomicBool::new(false);
    let workdir = repo.workdir().ok_or_else(|| git2::Error::from_str("Repository has no worktree"))?;
    let objects = repo.objects.clone().into_arc().map_err(gix_error)?;

    gix::worktree::state::checkout(&mut index, workdir, objects, &files, &bytes, &should_interrupt, options).map_err(gix_error)?;
    index.write(Default::default()).map_err(gix_error)?;
    Ok(())
}

fn checkout_existing_branch(repo: &mut gix::Repository, branch: &FullName) -> Result<(), git2::Error> {
    let tree_id = {
        let mut branch_ref = repo.try_find_reference(branch.as_ref()).map_err(gix_error)?.ok_or_else(|| git2::Error::from_str("Branch not found"))?;
        let commit = branch_ref.peel_to_commit().map_err(gix_error)?;
        commit.tree_id().map_err(gix_error)?.detach()
    };

    set_head_symbolic(repo, branch)?;
    checkout_worktree(repo, tree_id)
}

pub fn checkout_head(repo: &Repository, oid: Oid) -> Result<(), git2::Error> {
    let mut repo = open_repo(repo)?;
    repo.committer_or_set_generic_fallback().map_err(gix_error)?;

    let commit = repo.find_commit(git2_to_gix_oid(oid)).map_err(gix_error)?;
    let commit_id = commit.id;
    let tree_id = commit.tree_id().map_err(gix_error)?.detach();
    drop(commit);

    set_head_detached(&repo, commit_id)?;
    checkout_worktree(&mut repo, tree_id)
}

pub fn checkout_branch(repo: &Repository, hidden_branch_names: &mut HashSet<String>, local: &mut HashMap<u32, Vec<String>>, alias: u32, branch_name: &str) -> Result<(), git2::Error> {
    let mut repo = open_repo(repo)?;
    repo.committer_or_set_generic_fallback().map_err(gix_error)?;

    let local_branch_name = local_branch_ref_name(branch_name)?;
    if repo.try_find_reference(local_branch_name.as_ref()).map_err(gix_error)?.is_some() {
        let result = checkout_existing_branch(&mut repo, &local_branch_name);
        if result.is_ok() {
            hidden_branch_names.remove(branch_name);
        }
        return result;
    }

    if let Some((remote, branch)) = branch_name.split_once('/') {
        let branch_ref_name = local_branch_ref_name(branch)?;
        if repo.try_find_reference(branch_ref_name.as_ref()).map_err(gix_error)?.is_some() {
            let result = checkout_existing_branch(&mut repo, &branch_ref_name);
            if result.is_ok() {
                hidden_branch_names.remove(branch);
            }
            return result;
        }

        let remote_ref_name = remote_tracking_ref_name(branch_name)?;
        if repo.try_find_reference(remote_ref_name.as_ref()).map_err(gix_error)?.is_some() {
            let (commit_id, tree_id) = {
                let mut remote_ref = repo.find_reference(remote_ref_name.as_ref()).map_err(gix_error)?;
                let commit = remote_ref.peel_to_commit().map_err(gix_error)?;
                let commit_id = commit.id;
                let tree_id = commit.tree_id().map_err(gix_error)?.detach();
                (commit_id, tree_id)
            };

            repo.reference(branch_ref_name.clone(), commit_id, PreviousValue::MustNotExist, "checkout").map_err(gix_error)?;
            set_branch_upstream(&repo, branch, remote)?;

            // Mirror the newly created branch in the in-memory branch map until reload rebuilds it.
            local.entry(alias).or_default().push(branch.to_string());

            set_head_symbolic(&repo, &branch_ref_name)?;
            checkout_worktree(&mut repo, tree_id)?;

            // The checked-out local branch should remain visible under the hide-layer model.
            hidden_branch_names.remove(branch);
            return Ok(());
        }
    }

    Err(git2::Error::from_str("No matching local or remote branch found"))
}

#[cfg(test)]
#[path = "../../tests/git/actions/checkout.rs"]
mod tests;
