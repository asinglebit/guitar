use crate::core::oids::git2_to_gix_oid;
use git2::{Error, Oid, Repository};
use gix::bstr::ByteSlice;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use gix::refs::{FullName, Target};
use std::{borrow::Cow, fs::OpenOptions, io::Write, path::PathBuf};

fn gix_error(error: impl std::fmt::Display) -> Error {
    Error::from_str(&error.to_string())
}

fn open_repo(repo: &Repository) -> Result<gix::Repository, Error> {
    let path = repo.workdir().unwrap_or(repo.path());
    gix::open(path).map_err(gix_error)
}

fn head_ref_name() -> FullName {
    "HEAD".try_into().expect("HEAD is valid")
}

fn branch_ref_name(branch: &str) -> Result<FullName, Error> {
    format!("refs/heads/{branch}").try_into().map_err(|_| Error::from_str("branch name is invalid"))
}

fn load_local_config(path: PathBuf, source: gix::config::Source) -> Result<gix::config::File<'static>, Error> {
    gix::config::File::from_path_no_includes(path, source).map_err(gix_error)
}

fn write_local_config(config: &gix::config::File<'static>, source: gix::config::Source) -> Result<(), Error> {
    let path = config.meta().path.as_deref().ok_or_else(|| Error::from_str("Configuration path is missing"))?;
    let mut file = OpenOptions::new().create(false).write(true).truncate(true).open(path).map_err(gix_error)?;

    file.write_all(config.detect_newline_style()).map_err(gix_error)?;
    config.write_to_filter(&mut file, |section| section.meta().source == source).map_err(gix_error)
}

fn branch_config_source(repo: &gix::Repository) -> Result<(PathBuf, gix::config::Source), Error> {
    let snapshot = repo.config_snapshot();
    let plumbing = snapshot.plumbing();
    let path = plumbing.meta().path.clone().ok_or_else(|| Error::from_str("Configuration path is missing"))?;
    Ok((path, plumbing.meta().source))
}

fn update_branch_config(repo: &gix::Repository, mut update: impl FnMut(&mut gix::config::File<'static>) -> Result<bool, Error>) -> Result<(), Error> {
    let (path, source) = branch_config_source(repo)?;
    let mut config = load_local_config(path, source)?;
    if update(&mut config)? {
        write_local_config(&config, source)?;
    }
    Ok(())
}

fn rename_branch_config(repo: &gix::Repository, old_name: &str, new_name: &str) -> Result<(), Error> {
    update_branch_config(repo, |config| match config.rename_section("branch", Some(old_name.as_bytes().as_bstr()), "branch", Some(Cow::Owned(new_name.as_bytes().into()))) {
        Ok(()) => Ok(true),
        Err(gix::config::file::rename_section::Error::Lookup(_)) => Ok(false),
        Err(error) => Err(gix_error(error)),
    })
}

fn remove_branch_config(repo: &gix::Repository, branch: &str) -> Result<(), Error> {
    update_branch_config(repo, |config| Ok(config.remove_section("branch", Some(branch.as_bytes().as_bstr())).is_some()))
}

fn branch_ref_log(message: &str) -> LogChange {
    LogChange { mode: RefLog::AndReference, force_create_reflog: false, message: message.into() }
}

pub fn create_branch(repo: &Repository, branch_name: &str, target_oid: Oid) -> Result<(), Error> {
    // Branch creation is intentionally non-checkout; the graph stays on the current HEAD.
    let mut repo = open_repo(repo)?;
    let branch_ref_name = branch_ref_name(branch_name)?;
    if repo.try_find_reference(branch_ref_name.as_ref()).map_err(gix_error)?.is_some() {
        return Err(Error::from_str("branch name already exists"));
    }
    let target_oid = {
        let target_commit = repo.find_commit(git2_to_gix_oid(target_oid)).map_err(gix_error)?;
        target_commit.id
    };
    repo.committer_or_set_generic_fallback().map_err(gix_error)?;

    repo.reference(branch_ref_name, target_oid, PreviousValue::MustNotExist, branch_ref_log("branch create").message).map_err(gix_error)?;

    Ok(())
}

pub fn delete_branch(repo: &Repository, branch: &str) -> Result<(), git2::Error> {
    let mut repo = open_repo(repo)?;
    let branch_ref_name = branch_ref_name(branch)?;
    {
        repo.find_reference(branch_ref_name.as_ref()).map_err(gix_error)?;
    }

    if repo.head_name().map_err(gix_error)?.as_ref() == Some(&branch_ref_name) {
        return Err(Error::from_str("cannot delete the currently checked out branch"));
    }

    repo.committer_or_set_generic_fallback().map_err(gix_error)?;
    {
        let branch_ref = repo.find_reference(branch_ref_name.as_ref()).map_err(gix_error)?;
        branch_ref.delete().map_err(gix_error)?;
    }

    remove_branch_config(&repo, branch)?;
    Ok(())
}

pub fn rename_branch(repo: &Repository, old_name: &str, new_name: &str) -> Result<(), Error> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err(Error::from_str("branch name cannot be empty"));
    }
    if old_name == new_name {
        return Err(Error::from_str("new branch name must differ from current branch name"));
    }

    let mut repo = open_repo(repo)?;
    let old_ref_name = branch_ref_name(old_name)?;
    let new_ref_name = branch_ref_name(new_name)?;
    if repo.try_find_reference(new_ref_name.as_ref()).map_err(gix_error)?.is_some() {
        return Err(Error::from_str("branch name already exists"));
    }
    let target_oid = {
        let mut branch = repo.find_reference(old_ref_name.as_ref()).map_err(gix_error)?;
        branch.peel_to_id().map_err(gix_error)?.detach()
    };
    let current_head = repo.head_name().map_err(gix_error)?;
    let is_current_branch = current_head.as_ref() == Some(&old_ref_name);

    repo.committer_or_set_generic_fallback().map_err(gix_error)?;
    let mut edits = vec![
        RefEdit { change: Change::Update { log: branch_ref_log("branch rename"), expected: PreviousValue::MustNotExist, new: Target::Object(target_oid) }, name: new_ref_name.clone(), deref: false },
        RefEdit { change: Change::Delete { expected: PreviousValue::MustExistAndMatch(Target::Object(target_oid)), log: RefLog::AndReference }, name: old_ref_name.clone(), deref: false },
    ];
    if is_current_branch {
        edits.push(RefEdit {
            change: Change::Update { log: branch_ref_log("branch rename"), expected: PreviousValue::MustExistAndMatch(Target::Symbolic(old_ref_name)), new: Target::Symbolic(new_ref_name) },
            name: head_ref_name(),
            deref: false,
        });
    }
    repo.edit_references(edits).map_err(gix_error)?;

    rename_branch_config(&repo, old_name, new_name)?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/git/actions/branching.rs"]
mod tests;
