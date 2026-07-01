use crate::{
    core::oids::git2_to_gix_oid,
    git::actions::gix_support::{open_repo, tag_ref_name, to_git2_error},
};
use git2::{Error, Oid, Repository};

fn tag_ref_exists(repo: &gix::Repository, tag_ref_name: &gix::refs::FullName) -> Result<bool, Error> {
    Ok(repo.try_find_reference(tag_ref_name.as_ref()).map_err(to_git2_error)?.is_some())
}

pub fn tag(repo: &Repository, oid: git2::Oid, tag: &str) -> Result<Oid, Error> {
    let mut repo = open_repo(repo)?;
    let tag_ref_name = tag_ref_name(tag)?;
    if tag_ref_exists(&repo, &tag_ref_name)? {
        return Err(Error::from_str("tag name already exists"));
    }
    let object_id = {
        let object = repo.find_object(git2_to_gix_oid(oid)).map_err(to_git2_error)?;
        object.id
    };

    repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;
    repo.reference(tag_ref_name, object_id, gix::refs::transaction::PreviousValue::MustNotExist, "tag create").map_err(to_git2_error)?;

    Ok(oid)
}

pub fn untag(repo: &Repository, tag: &str) -> Result<(), Error> {
    let mut repo = open_repo(repo)?;
    let tag_ref_name = tag_ref_name(tag)?;
    repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;
    let tag_ref = repo.find_reference(tag_ref_name.as_ref()).map_err(to_git2_error)?;
    tag_ref.delete().map_err(to_git2_error)?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/git/actions/tagging.rs"]
mod tests;
