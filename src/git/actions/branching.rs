use crate::core::oids::git2_to_gix_oid;
use crate::git::actions::gix_support::{branch_ref_name, edit_repo_config, head_ref_name, open_repo, ref_log, to_git2_error};
use git2::{Error, Oid, Repository};
use gix::bstr::ByteSlice;
use gix::refs::transaction::{Change, PreviousValue, RefEdit, RefLog};
use gix::refs::{FullName, Target};
use std::borrow::Cow;

fn rename_branch_config(repo: &gix::Repository, old_name: &str, new_name: &str) -> Result<(), Error> {
    edit_repo_config(repo, |config| match config.rename_section("branch", Some(old_name.as_bytes().as_bstr()), "branch", Some(Cow::Owned(new_name.as_bytes().into()))) {
        Ok(()) => Ok(true),
        Err(gix::config::file::rename_section::Error::Lookup(_)) => Ok(false),
        Err(error) => Err(to_git2_error(error)),
    })
}

fn remove_branch_config(repo: &gix::Repository, branch: &str) -> Result<(), Error> {
    edit_repo_config(repo, |config| Ok(config.remove_section("branch", Some(branch.as_bytes().as_bstr())).is_some()))
}

fn normalize_renamed_branch<'name>(old_name: &str, new_name: &'name str) -> Result<&'name str, Error> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err(Error::from_str("branch name cannot be empty"));
    }
    if old_name == new_name {
        return Err(Error::from_str("new branch name must differ from current branch name"));
    }
    Ok(new_name)
}

fn ensure_branch_name_available(repo: &gix::Repository, ref_name: &FullName) -> Result<(), Error> {
    if repo.try_find_reference(ref_name.as_ref()).map_err(to_git2_error)?.is_some() {
        return Err(Error::from_str("branch name already exists"));
    }
    Ok(())
}

fn branch_target(repo: &gix::Repository, ref_name: &FullName) -> Result<gix::hash::ObjectId, Error> {
    let mut branch = repo.find_reference(ref_name.as_ref()).map_err(to_git2_error)?;
    branch.peel_to_id().map(|id| id.detach()).map_err(to_git2_error)
}

fn is_current_branch_ref(repo: &gix::Repository, ref_name: &FullName) -> Result<bool, Error> {
    Ok(repo.head_name().map_err(to_git2_error)?.as_ref() == Some(ref_name))
}

fn ensure_branch_is_not_current(repo: &gix::Repository, ref_name: &FullName) -> Result<(), Error> {
    if is_current_branch_ref(repo, ref_name)? {
        return Err(Error::from_str("cannot delete the currently checked out branch"));
    }
    Ok(())
}

fn delete_branch_ref(repo: &mut gix::Repository, ref_name: FullName, target_oid: gix::hash::ObjectId) -> Result<(), Error> {
    repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;
    repo.edit_reference(RefEdit { change: Change::Delete { expected: PreviousValue::MustExistAndMatch(Target::Object(target_oid)), log: RefLog::AndReference }, name: ref_name, deref: false })
        .map(drop)
        .map_err(to_git2_error)
}

fn rename_branch_refs(repo: &mut gix::Repository, old_ref_name: FullName, new_ref_name: FullName, target_oid: gix::hash::ObjectId) -> Result<(), Error> {
    repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;
    let head_edit = is_current_branch_ref(repo, &old_ref_name)?.then(|| RefEdit {
        change: Change::Update { log: ref_log("branch rename"), expected: PreviousValue::MustExistAndMatch(Target::Symbolic(old_ref_name.clone())), new: Target::Symbolic(new_ref_name.clone()) },
        name: head_ref_name(),
        deref: false,
    });
    let branch_edits = [
        RefEdit { change: Change::Update { log: ref_log("branch rename"), expected: PreviousValue::MustNotExist, new: Target::Object(target_oid) }, name: new_ref_name, deref: false },
        RefEdit { change: Change::Delete { expected: PreviousValue::MustExistAndMatch(Target::Object(target_oid)), log: RefLog::AndReference }, name: old_ref_name, deref: false },
    ];
    repo.edit_references(branch_edits.into_iter().chain(head_edit)).map(drop).map_err(to_git2_error)
}

fn target_commit_oid(repo: &gix::Repository, target_oid: Oid) -> Result<gix::hash::ObjectId, Error> {
    repo.find_commit(git2_to_gix_oid(target_oid)).map(|commit| commit.id).map_err(to_git2_error)
}

struct BranchCreate {
    ref_name: FullName,
    target_oid: gix::hash::ObjectId,
}

impl BranchCreate {
    fn prepare(repo: &gix::Repository, branch_name: &str, target_oid: Oid) -> Result<Self, Error> {
        let ref_name = branch_ref_name(branch_name)?;
        ensure_branch_name_available(repo, &ref_name)?;
        let target_oid = target_commit_oid(repo, target_oid)?;
        Ok(Self { ref_name, target_oid })
    }

    fn apply(self, repo: &mut gix::Repository) -> Result<(), Error> {
        repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;
        repo.reference(self.ref_name, self.target_oid, PreviousValue::MustNotExist, ref_log("branch create").message).map(drop).map_err(to_git2_error)
    }
}

struct BranchDelete {
    ref_name: FullName,
    target_oid: gix::hash::ObjectId,
}

impl BranchDelete {
    fn prepare(repo: &gix::Repository, branch: &str) -> Result<Self, Error> {
        let ref_name = branch_ref_name(branch)?;
        let target_oid = branch_target(repo, &ref_name)?;
        ensure_branch_is_not_current(repo, &ref_name)?;
        Ok(Self { ref_name, target_oid })
    }

    fn apply(self, repo: &mut gix::Repository) -> Result<(), Error> {
        delete_branch_ref(repo, self.ref_name, self.target_oid)
    }
}

struct BranchRename<'name> {
    old_name: &'name str,
    new_name: &'name str,
    old_ref_name: FullName,
    new_ref_name: FullName,
    target_oid: gix::hash::ObjectId,
}

impl<'name> BranchRename<'name> {
    fn prepare(repo: &gix::Repository, old_name: &'name str, new_name: &'name str) -> Result<Self, Error> {
        let new_name = normalize_renamed_branch(old_name, new_name)?;
        let old_ref_name = branch_ref_name(old_name)?;
        let new_ref_name = branch_ref_name(new_name)?;
        ensure_branch_name_available(repo, &new_ref_name)?;
        let target_oid = branch_target(repo, &old_ref_name)?;
        Ok(Self { old_name, new_name, old_ref_name, new_ref_name, target_oid })
    }

    fn apply(self, repo: &mut gix::Repository) -> Result<(), Error> {
        rename_branch_refs(repo, self.old_ref_name, self.new_ref_name, self.target_oid)?;
        rename_branch_config(repo, self.old_name, self.new_name)
    }
}

pub fn create_branch(repo: &Repository, branch_name: &str, target_oid: Oid) -> Result<(), Error> {
    // Branch creation is intentionally non-checkout; the graph stays on the current HEAD.
    let mut repo = open_repo(repo)?;
    BranchCreate::prepare(&repo, branch_name, target_oid)?.apply(&mut repo)
}

pub fn delete_branch(repo: &Repository, branch: &str) -> Result<(), git2::Error> {
    let mut repo = open_repo(repo)?;
    BranchDelete::prepare(&repo, branch)?.apply(&mut repo)?;
    remove_branch_config(&repo, branch)
}

pub fn rename_branch(repo: &Repository, old_name: &str, new_name: &str) -> Result<(), Error> {
    let mut repo = open_repo(repo)?;
    BranchRename::prepare(&repo, old_name, new_name)?.apply(&mut repo)
}

#[cfg(test)]
#[path = "../../tests/git/actions/branching.rs"]
mod tests;
