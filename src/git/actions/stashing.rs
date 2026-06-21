use git2::{Oid, Repository};
use git2::{StashApplyOptions, StashFlags};

pub fn stash(repo: &mut Repository) -> Result<Oid, git2::Error> {
    // Include untracked files so the uncommitted pseudo-row can become fully clean.
    let flags = StashFlags::DEFAULT | StashFlags::INCLUDE_UNTRACKED;

    let message = {
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let short_id = commit.id().to_string()[..7].to_string();
        let summary = commit.summary().unwrap_or("WIP");
        format!("{} {}", short_id, summary)
    };

    let stash_index = repo.stash_save(&repo.signature()?, message.as_str(), Some(flags))?;

    Ok(stash_index)
}

pub fn pop(repo: &mut Repository, target_oid: &Oid, apply: bool) -> Result<(), git2::Error> {
    // Libgit2 addresses stashes by stack index, so find the index for the rendered OID.
    let mut stash_index: Option<usize> = None;

    repo.stash_foreach(|index, _message, oid| {
        if oid == target_oid {
            stash_index = Some(index);
            false
        } else {
            true
        }
    })?;

    if let Some(index) = stash_index {
        if apply {
            // The same path handles "pop" and "drop"; apply controls whether changes return.
            let mut opts = StashApplyOptions::new();
            repo.stash_apply(index, Some(&mut opts))?;
        }
        repo.stash_drop(index)?;
    }

    Ok(())
}

#[cfg(test)]
#[path = "../../tests/git/actions/stashing.rs"]
mod tests;
