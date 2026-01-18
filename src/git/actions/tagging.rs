use git2::{Error, Oid, Repository};

pub fn tag(repo: &Repository, oid: git2::Oid, tag: &str) -> Result<Oid, Error> {
    repo.tag_lightweight(tag, &repo.find_object(oid, None)?, false)
}

pub fn untag(repo: &Repository, tag: &str) -> Result<(), Error> {
    repo.tag_delete(tag)
}
