use git2::{build::CheckoutBuilder, BranchType, Cred, Error, ErrorCode, FetchOptions, Oid, PushOptions, RemoteCallbacks, Repository, ResetType, Signature, StatusOptions};
use git2::{CherrypickOptions, FetchPrune, StashApplyOptions, StashFlags};
use std::{collections::HashMap, thread};

pub fn tag(repo: &Repository, oid: git2::Oid, tag: &str) -> Result<Oid, Error> {
    repo.tag_lightweight(tag, &repo.find_object(oid, None)?, false)
}

pub fn untag(repo: &Repository, tag: &str) -> Result<(), Error> {
    repo.tag_delete(tag)
}
