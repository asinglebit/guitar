use crate::core::{
    batcher::{Batcher, WalkCommit},
    oids::Oids,
};
use crate::helpers::branch_visibility::branch_name_from_ref;
use git2::{Oid, Repository, Time};
use gix::prelude::HeaderExt;
use std::collections::HashMap;

// Map each ref tip to the compact alias used by the graph renderer.
pub fn get_tip_oids(repo: &gix::Repository, oids: &mut Oids) -> (HashMap<u32, Vec<String>>, HashMap<u32, Vec<String>>) {
    let mut local: HashMap<u32, Vec<String>> = HashMap::new();
    let mut remote: HashMap<u32, Vec<String>> = HashMap::new();

    let Ok(references) = repo.references() else {
        return (local, remote);
    };
    for (references, bucket) in [(references.local_branches(), &mut local), (references.remote_branches(), &mut remote)] {
        let Ok(references) = references else { continue };

        for reference in references {
            let Ok(reference) = reference else { continue };
            let Some(oid) = reference.try_id().map(|id| Oid::from_bytes(id.detach().as_bytes()).unwrap()) else { continue };
            let alias = oids.get_alias_by_oid(oid);

            if let Some(name) = branch_name_from_ref(reference.name().as_bstr()) {
                bucket.entry(alias).or_default().push(name.to_string());
            }
        }
    }

    (local, remote)
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
        let Ok(header) = repo.objects.header(&id) else { continue };
        if header.kind() != gix::object::Kind::Commit {
            continue;
        }
        let oid = Oid::from_bytes(id.as_bytes()).unwrap();
        let tag_name = String::from_utf8_lossy(name).into_owned();

        let alias = oids.get_alias_by_oid(oid);
        local.entry(alias).or_default().push(tag_name);
    }

    local
}

// Pull the next revwalk page into the global alias order.
pub fn get_sorted_oids(batcher: &mut Batcher, oids: &mut Oids, sorted: &mut Vec<u32>, amount: usize, scratch: &mut Vec<WalkCommit>) {
    scratch.clear();
    let fetched = batcher.next_into(amount, scratch);
    if fetched == 0 {
        return;
    }

    oids.reserve_aliases(fetched);
    sorted.reserve(fetched);

    for commit in scratch.iter() {
        let alias = oids.get_alias_by_oid(commit.oid);
        sorted.push(alias);
    }
}

// Return the current branch name, or None when HEAD is detached.
pub fn get_current_branch(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    if !head.is_branch() {
        return None;
    }
    head.shorthand().map(|s| s.to_string())
}

// Return all git timestamp variants for refs that need date metadata.
pub fn get_timestamps(repo: &Repository, _branches: &HashMap<Oid, Vec<String>>) -> HashMap<Oid, (Time, Time, Time)> {
    _branches
        .keys()
        .map(|&sha| {
            let commit = repo.find_commit(sha).unwrap();
            let author_time = commit.author().when();
            let committer_time = commit.committer().when();
            let time = commit.time();
            (sha, (time, committer_time, author_time))
        })
        .collect()
}

pub fn get_git_user_info(repo: &Repository) -> Result<(Option<String>, Option<String>), git2::Error> {
    let config = repo.config()?;
    let name = config.get_string("user.name").ok();
    let email = config.get_string("user.email").ok();
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
        let alias = oids.get_alias_by_oid(Oid::from_bytes(entry.new_oid.as_bytes()).unwrap());
        stashes.push(alias);
    }

    stashes
}
