use crate::core::{
    batcher::{Batcher, WalkedCommit},
    oids::Oids,
};
use crate::git::gix::for_each_branch_tip;
use git2::Repository;
use gix::{bstr::ByteSlice, prelude::HeaderExt};
use im::HashSet;
use std::{
    collections::{HashMap, HashSet as StdHashSet},
    path::Path,
};

// Map each ref tip to the compact alias used by the graph renderer.
pub fn get_tip_oids(repo: &gix::Repository, oids: &mut Oids, hidden_branch_names: &HashSet<String>) -> (HashMap<u32, Vec<String>>, HashMap<u32, Vec<String>>, Vec<gix::ObjectId>) {
    let mut local: HashMap<u32, Vec<String>> = HashMap::new();
    let mut remote: HashMap<u32, Vec<String>> = HashMap::new();
    let mut visible_roots = Vec::new();
    let mut visible_root_set = StdHashSet::new();

    let _ = for_each_branch_tip(repo, |is_local, name, oid| {
        let bucket = if is_local { &mut local } else { &mut remote };
        bucket.entry(oids.get_alias_by_oid(oid)).or_default().push(name.to_string());
        if !hidden_branch_names.contains(name) && visible_root_set.insert(oid) {
            visible_roots.push(oid);
        }
    });

    (local, remote, visible_roots)
}

// Map lightweight and annotated tags to the commit aliases they resolve to.
pub fn get_tag_oids(repo: &gix::Repository, oids: &mut Oids) -> HashMap<u32, Vec<String>> {
    let mut local: HashMap<u32, Vec<String>> = HashMap::new();

    let Ok(references) = repo.references() else {
        return local;
    };

    let Ok(tags) = references.tags() else {
        return local;
    };

    let Ok(tags) = tags.peeled() else {
        return local;
    };

    for reference in tags.flatten() {
        let Some(name) = reference.name().as_bstr().strip_prefix(b"refs/tags/") else {
            continue;
        };
        let Some(id) = reference.try_id() else { continue };
        let id = id.detach();
        let Ok(header) = repo.objects.header(id) else { continue };
        if header.kind() != gix::object::Kind::Commit {
            continue;
        }
        let tag_name = String::from_utf8_lossy(name).into_owned();

        let alias = oids.get_alias_by_oid(id);
        local.entry(alias).or_default().push(tag_name);
    }

    local
}

// Pull the next revwalk page into the global alias order.
pub fn get_sorted_oids(batcher: &mut Batcher, oids: &mut Oids, sorted: &mut Vec<u32>, amount: usize, scratch: &mut Vec<WalkedCommit>) {
    scratch.clear();
    let fetched = batcher.next_aliased_into(amount, scratch, oids);
    if fetched == 0 {
        return;
    }

    sorted.reserve(fetched);
    sorted.extend(scratch.iter().map(|commit| commit.alias));
}

// Return the current branch name, or None when HEAD is detached.
pub fn get_current_branch(repo: &Repository) -> Option<String> {
    repo.head().ok().filter(|head| head.is_branch()).and_then(|head| head.shorthand().map(str::to_string))
}

pub fn get_git_user_info(repo: &Repository) -> Result<(Option<String>, Option<String>), git2::Error> {
    let config = repo.config()?;
    let name = config.get_string("user.name").ok();
    let email = config.get_string("user.email").ok();
    Ok((name, email))
}

pub fn get_git_user_info_from_path(path: impl AsRef<Path>) -> Result<(Option<String>, Option<String>), git2::Error> {
    let repo = gix::open(path.as_ref()).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let config = repo.config_snapshot();
    let name = config.string("user.name").and_then(|value| value.to_str().ok().map(str::to_string));
    let email = config.string("user.email").and_then(|value| value.to_str().ok().map(str::to_string));
    Ok((name, email))
}

// Enumerate stash roots, keeping the newest stash first.
pub fn get_stashed_commits(repo: &gix::Repository, oids: &mut Oids) -> Vec<u32> {
    let mut stashes = Vec::new();

    let Some(reference) = repo.try_find_reference("refs/stash").ok().flatten() else {
        return stashes;
    };

    let mut log_iter = reference.log_iter();
    let Some(logs) = log_iter.rev().ok().flatten() else {
        return stashes;
    };

    for entry in logs.filter_map(Result::ok) {
        let alias = oids.get_alias_by_oid(entry.new_oid);
        stashes.push(alias);
    }

    stashes
}
