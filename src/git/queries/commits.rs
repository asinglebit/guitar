use crate::core::{
    batcher::{Batcher, WalkCommit},
    oids::Oids,
};
use git2::ObjectType;
use git2::{Oid, Repository, Time};
use std::collections::HashMap;

// Map each ref tip to the compact alias used by the graph renderer.
pub fn get_tip_oids(repo: &Repository, oids: &mut Oids) -> (HashMap<u32, Vec<String>>, HashMap<u32, Vec<String>>) {
    let mut local: HashMap<u32, Vec<String>> = HashMap::new();
    let mut remote: HashMap<u32, Vec<String>> = HashMap::new();

    for reference in repo.references().unwrap().flatten() {
        // Symbolic refs such as HEAD have no target and are skipped.
        if let Some(oid) = reference.target() {
            let alias = oids.get_alias_by_oid(oid);
            let name = reference.name().unwrap_or("unknown");

            if let Some(stripped) = name.strip_prefix("refs/heads/") {
                local.entry(alias).or_default().push(stripped.to_string());
            } else if let Some(stripped) = name.strip_prefix("refs/remotes/") {
                remote.entry(alias).or_default().push(stripped.to_string());
            }
        }
    }

    (local, remote)
}

// Map lightweight and annotated tags to the commit aliases they resolve to.
pub fn get_tag_oids(repo: &Repository, oids: &mut Oids) -> HashMap<u32, Vec<String>> {
    let mut local: HashMap<u32, Vec<String>> = HashMap::new();

    for reference in repo.references().unwrap().flatten() {
        let name = match reference.name() {
            Some(n) => n,
            None => continue,
        };

        let stripped = match name.strip_prefix("refs/tags/") {
            Some(s) => s,
            None => continue,
        };

        // Non-commit tags are ignored because the graph has no lane for blobs or trees.
        let obj = match reference.peel(ObjectType::Commit) {
            Ok(obj) => obj,
            Err(_) => continue,
        };

        let commit_oid = obj.id();
        let alias = oids.get_alias_by_oid(commit_oid);

        local.entry(alias).or_default().push(stripped.to_string());
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

pub fn get_stashed_commits(repo: &mut Repository, oids: &mut Oids) -> Vec<u32> {
    let mut stashes = Vec::new();

    // Stashes are real commits; assigning aliases lets them render beside normal history.
    repo.stash_foreach(|_, _, oid| {
        let alias = oids.get_alias_by_oid(*oid);
        stashes.push(alias);
        true
    })
    .unwrap();

    stashes
}
