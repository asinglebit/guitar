use git2::{build::CheckoutBuilder, BranchType, Cred, Error, ErrorCode, FetchOptions, Oid, PushOptions, RemoteCallbacks, Repository, ResetType, Signature, StatusOptions};
use git2::{CherrypickOptions, FetchPrune, StashApplyOptions, StashFlags};
use std::{collections::HashMap, thread};

pub fn stage_all(repo: &Repository) -> Result<(), Error> {
    let mut index = repo.index()?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true).include_ignored(false).include_unmodified(false);

    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            let path = std::path::Path::new(path);

            match entry.status() {
                s if s.is_wt_deleted() || s.is_index_deleted() => {
                    // Stage deletions (whether from working dir or already staged)
                    if index.get_path(path, 0).is_some() {
                        index.remove_path(path)?;
                    }
                },
                _ => {
                    // Stage new or modified files
                    index.add_path(path)?;
                },
            }
        }
    }

    index.write()?;
    Ok(())
}

pub fn unstage_all(repo: &Repository) -> Result<(), git2::Error> {
    // Get HEAD commit
    let head = match repo.head() {
        Ok(head) => head.peel_to_commit()?,
        Err(_) => {
            // If no HEAD exists (fresh repo), there's nothing to unstage
            return Ok(());
        },
    };

    // Perform mixed reset - keeps working directory changes but resets index to HEAD
    repo.reset(&head.into_object(), ResetType::Mixed, None)?;

    Ok(())
}

pub fn stage_file(repo: &Repository, path: &std::path::Path) -> Result<(), git2::Error> {
    let mut index = repo.index()?;

    // If the file exists, add it (new or modified)
    if path.exists() {
        index.add_path(path)?;
    } else {
        // File deleted: remove from index
        index.remove_path(path)?;
    }

    index.write()?;
    Ok(())
}

pub fn unstage_file(repo: &Repository, path: &std::path::Path) -> Result<(), git2::Error> {
    let head = match repo.head() {
        Ok(h) => h.peel_to_commit()?,
        Err(_) => {
            // No HEAD (initial commit case)
            let mut index = repo.index()?;
            index.remove_path(path)?;
            index.write()?;
            return Ok(());
        },
    };

    // Reset only this path in the index
    repo.reset_default(Some(&head.into_object()), [path])?;
    Ok(())
}
