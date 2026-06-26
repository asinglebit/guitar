use crate::git::gix::gix_error as gix_error_impl;
use git2::{Error, Repository};
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use gix::refs::{FullName, Target};
use std::{fs::OpenOptions, io::Write, path::PathBuf, sync::atomic::AtomicBool};

pub(crate) fn open_repo(repo: &Repository) -> Result<gix::Repository, Error> {
    let path = repo.workdir().unwrap_or(repo.path());
    gix::open(path.to_path_buf()).map_err(to_git2_error)
}

pub(crate) fn open_worktree_repo(repo: &Repository) -> Result<gix::Repository, Error> {
    let workdir = repo.workdir().ok_or_else(|| Error::from_str("Repository has no worktree"))?;
    gix::open(workdir.to_path_buf()).map_err(to_git2_error)
}

pub(crate) fn head_ref_name() -> FullName {
    "HEAD".try_into().expect("HEAD is valid")
}

pub(crate) fn branch_ref_name(branch: &str) -> Result<FullName, Error> {
    format!("refs/heads/{branch}").try_into().map_err(|_| Error::from_str("branch name is invalid"))
}

pub(crate) fn remote_tracking_ref_name(branch_name: &str) -> Result<FullName, Error> {
    format!("refs/remotes/{branch_name}").try_into().map_err(|_| Error::from_str("remote tracking branch name is invalid"))
}

pub(crate) fn tag_ref_name(tag: &str) -> Result<FullName, Error> {
    format!("refs/tags/{tag}").try_into().map_err(|_| Error::from_str("tag name is invalid"))
}

pub(crate) fn ref_log(message: &str) -> LogChange {
    LogChange { mode: RefLog::AndReference, force_create_reflog: false, message: message.into() }
}

pub(crate) fn repo_config_location(repo: &gix::Repository) -> Result<(PathBuf, gix::config::Source), Error> {
    let snapshot = repo.config_snapshot();
    let plumbing = snapshot.plumbing();
    let path = plumbing.meta().path.clone().ok_or_else(|| Error::from_str("Configuration path is missing"))?;
    Ok((path, plumbing.meta().source))
}

pub(crate) fn load_config_file(path: PathBuf, source: gix::config::Source) -> Result<gix::config::File<'static>, Error> {
    gix::config::File::from_path_no_includes(path, source).map_err(to_git2_error)
}

pub(crate) fn edit_config_file(path: PathBuf, source: gix::config::Source, update: impl FnOnce(&mut gix::config::File<'static>) -> Result<bool, Error>) -> Result<(), Error> {
    let mut config = load_config_file(path, source)?;
    if update(&mut config)? {
        let path = config.meta().path.as_deref().ok_or_else(|| Error::from_str("Configuration path is missing"))?;
        let mut file = OpenOptions::new().create(false).write(true).truncate(true).open(path).map_err(to_git2_error)?;

        file.write_all(config.detect_newline_style()).map_err(to_git2_error)?;
        config.write_to_filter(&mut file, |section| section.meta().source == source).map_err(to_git2_error)?;
    }
    Ok(())
}

pub(crate) fn edit_repo_config(repo: &gix::Repository, update: impl FnOnce(&mut gix::config::File<'static>) -> Result<bool, Error>) -> Result<(), Error> {
    let (path, source) = repo_config_location(repo)?;
    edit_config_file(path, source, update)
}

pub(crate) fn write_index(index: &mut gix::index::File) -> Result<(), Error> {
    index.sort_entries();
    index.write(Default::default()).map_err(to_git2_error)
}

pub(crate) fn set_head_to_object(repo: &gix::Repository, commit_id: gix::hash::ObjectId, message: &str) -> Result<(), Error> {
    repo.edit_reference(RefEdit { change: Change::Update { log: ref_log(message), expected: PreviousValue::Any, new: Target::Object(commit_id) }, name: head_ref_name(), deref: false })
        .map_err(to_git2_error)?;
    Ok(())
}

pub(crate) fn set_head_to_symbolic(repo: &gix::Repository, branch: &FullName, message: &str) -> Result<(), Error> {
    repo.edit_reference(RefEdit { change: Change::Update { log: ref_log(message), expected: PreviousValue::Any, new: Target::Symbolic(branch.clone()) }, name: head_ref_name(), deref: false })
        .map_err(to_git2_error)?;
    Ok(())
}

pub(crate) fn checkout_tree(repo: &mut gix::Repository, tree_id: gix::hash::ObjectId, destination_is_initially_empty: bool) -> Result<(), Error> {
    let mut index = repo.index_from_tree(&tree_id).map_err(to_git2_error)?;
    let mut options = repo.checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping).map_err(to_git2_error)?;
    options.overwrite_existing = true;
    options.destination_is_initially_empty = destination_is_initially_empty;

    let workdir = repo.workdir().ok_or_else(|| Error::from_str("Repository has no worktree"))?;
    let should_interrupt = AtomicBool::new(false);
    let files = gix::features::progress::Discard;
    let bytes = gix::features::progress::Discard;
    gix::worktree::state::checkout(&mut index, workdir, repo.objects.clone().into_arc().map_err(to_git2_error)?, &files, &bytes, &should_interrupt, options).map_err(to_git2_error)?;

    write_index(&mut index)
}

pub(crate) fn to_git2_error(error: impl std::fmt::Display) -> Error {
    gix_error_impl(error)
}
