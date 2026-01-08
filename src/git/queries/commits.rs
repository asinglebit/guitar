use crate::core::{batcher::Batcher, oids::Oids};
use git2::ObjectType;
use git2::{Oid, Repository, Time};
use std::collections::HashMap;

// Returns a map of commit OIDs to the branch names that point to them
pub fn get_tip_oids(
    repo: &Repository,
    oids: &mut Oids,
) -> (HashMap<u32, Vec<String>>, HashMap<u32, Vec<String>>) {
    let mut local: HashMap<u32, Vec<String>> = HashMap::new();
    let mut remote: HashMap<u32, Vec<String>> = HashMap::new();

    // Iterate all refs once
    for reference in repo.references().unwrap().flatten() {
        // Only handle direct refs (skip symbolic ones like HEAD)
        if let Some(oid) = reference.target() {
            // Get the alias
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

// Get all tags in a repo
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

        // Peel ref to commit
        let obj = match reference.peel(ObjectType::Commit) {
            Ok(obj) => obj,
            Err(_) => continue, // Ignore if the tag points to non-commit
        };

        let commit_oid = obj.id();
        let alias = oids.get_alias_by_oid(commit_oid);

        local.entry(alias).or_default().push(stripped.to_string());
    }

    local
}

// Outcomes:
// Update the oids vector
pub fn get_sorted_oids(batcher: &Batcher, oids: &mut Oids, sorted: &mut Vec<u32>, amount: usize) {
    // Get the next batch of commits
    let chunk = batcher.next(amount);
    if chunk.is_empty() {
        // No more commits left
        return;
    }

    // Walk all commits topologically
    for oid in chunk {
        // Get the alias
        let alias = oids.get_alias_by_oid(oid);
        sorted.push(alias);
    }
}

// Returns the name of the currently checked-out branch, or None if detached HEAD
pub fn get_current_branch(repo: &Repository) -> Option<String> {
    let head = repo.head().unwrap();
    if head.is_branch() {
        head.shorthand().map(|s| s.to_string())
    } else {
        None
    }
}

// Returns a map of commit OIDs to their timestamps:
// (commit time, committer time, author time)
pub fn get_timestamps(
    repo: &Repository,
    _branches: &HashMap<Oid, Vec<String>>,
) -> HashMap<Oid, (Time, Time, Time)> {
    _branches
        .keys()
        .map(|&sha| {
            let commit = repo.find_commit(sha).unwrap();
            let author_time = commit.author().when();
            let committer_time = commit.committer().when();
            let time = commit.time();
            // Map each OID to its associated timestamps
            (sha, (time, committer_time, author_time))
        })
        .collect()
}

pub fn get_git_user_info(
    repo: &Repository,
) -> Result<(Option<String>, Option<String>), git2::Error> {
    let config = repo.config()?;
    let name = config.get_string("user.name").ok();
    let email = config.get_string("user.email").ok();
    Ok((name, email))
}

pub fn get_stashed_commits(repo: &mut Repository, oids: &mut Oids) -> Vec<u32> {
    let mut stashes = Vec::new();

    repo.stash_foreach(|_, _, oid| {
        let alias = oids.get_alias_by_oid(*oid);
        stashes.push(alias);
        true
    })
    .unwrap();

    stashes
}
