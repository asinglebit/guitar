#[rustfmt::skip]
use std::{
    thread,
    collections::{
        HashMap
    }
};
use git2::{CherrypickOptions, FetchPrune, StashApplyOptions, StashFlags};
#[rustfmt::skip]
use git2::{
    Oid,
    Cred,
    RemoteCallbacks,
    Error,
    ErrorCode,
    Signature,
    StatusOptions,
    BranchType,
    ResetType,
    Repository,
    FetchOptions,
    PushOptions,
    build::CheckoutBuilder
};

pub fn checkout_head(repo: &Repository, oid: Oid) {
    // Find the commit object
    let commit = repo.find_commit(oid).unwrap();

    // Set HEAD to the commit (detached)
    repo.set_head_detached(commit.id()).unwrap();

    // Checkout the commit
    repo.checkout_head(Some(
        CheckoutBuilder::default().allow_conflicts(true).force(), // optional: force overwrite local changes
    ))
    .expect("Error checking out");
}

pub fn checkout_branch(
    repo: &Repository,
    visible: &mut HashMap<u32, Vec<String>>,
    local: &mut HashMap<u32, Vec<String>>,
    alias: u32,
    branch_name: &str,
) -> Result<(), git2::Error> {
    // Helper to checkout a local branch
    fn checkout(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
        let branch = repo.find_branch(branch_name, BranchType::Local)?;
        repo.set_head(branch.get().name().unwrap())?;
        repo.checkout_head(Some(CheckoutBuilder::default().allow_conflicts(true).force()))
    }

    // If branch_name already exists as a local branch, checkout directly
    if repo.find_branch(branch_name, BranchType::Local).is_ok() {
        return checkout(repo, branch_name);
    }

    // If branch_name is in the form <remote>/<branch>
    if let Some((_remote, branch)) = branch_name.split_once('/') {
        if repo.find_branch(branch, BranchType::Local).is_ok() {
            return checkout(repo, branch);
        }

        if repo.find_branch(branch_name, BranchType::Remote).is_ok() {
            let remote_branch = repo.find_branch(branch_name, BranchType::Remote)?;
            let commit = remote_branch.get().peel_to_commit()?;

            let mut local_branch = repo.branch(branch, &commit, false)?;
            local_branch.set_upstream(Some(branch_name))?;
            local.entry(alias)
                .or_default()
                .push(branch.to_string());
            visible
                .entry(alias)
                .or_default()
                .push(branch.to_string());

            return checkout(repo, branch);
        }
    }

    Err(git2::Error::from_str(
        "No matching local or remote branch found for the given Oid",
    ))
}

pub fn git_add_all(repo: &Repository) -> Result<(), Error> {
    let mut index = repo.index()?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .include_unmodified(false);

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
                }
                _ => {
                    // Stage new or modified files
                    index.add_path(path)?;
                }
            }
        }
    }

    index.write()?;
    Ok(())
}

pub fn commit_staged(
    repo: &Repository,
    message: &str,
    name: &str,
    email: &str,
) -> Result<Oid, Error> {
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Determine parent commit
    let parent_commit = match repo.head() {
        Ok(head_ref) => {
            // Try to peel to commit
            head_ref.peel_to_commit().ok()
        }
        Err(e) => {
            if e.code() == ErrorCode::UnbornBranch {
                None // empty repo, initial commit
            } else {
                return Err(e);
            }
        }
    };

    let signature = Signature::now(name, email)?;

    let commit_oid = if let Some(parent) = parent_commit {
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent],
        )?
    } else {
        // Initial commit
        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?
    };

    Ok(commit_oid)
}

pub fn reset_to_commit(repo: &Repository, target: Oid, reset_type: ResetType) -> Result<(), Error> {
    // Resolve the target commit object
    let target_commit = repo.find_commit(target)?;

    // Get HEAD reference
    let head = repo.head()?;

    if head.is_branch() {
        // Normal branch: move branch reference
        let branch_ref_name = head
            .name()
            .ok_or_else(|| Error::from_str("Invalid branch reference name"))?;
        let mut branch_ref = repo.find_reference(branch_ref_name)?;
        branch_ref.set_target(target, "reset branch to commit")?;
    } else {
        // Detached HEAD: move HEAD directly
        let head_ref_name = head.name().unwrap_or("HEAD");
        let mut head_ref_obj = repo.find_reference(head_ref_name)?;
        head_ref_obj.set_target(target, "reset detached HEAD")?;
    }

    // Perform the reset (Hard, Soft, or Mixed)
    repo.reset(&target_commit.into_object(), reset_type, None)?;

    Ok(())
}

pub fn unstage_all(repo: &Repository) -> Result<(), git2::Error> {
    // Get HEAD commit
    let head = match repo.head() {
        Ok(head) => head.peel_to_commit()?,
        Err(_) => {
            // If no HEAD exists (fresh repo), there's nothing to unstage
            return Ok(());
        }
    };

    // Perform mixed reset - keeps working directory changes but resets index to HEAD
    repo.reset(&head.into_object(), ResetType::Mixed, None)?;

    Ok(())
}

pub fn fetch_over_ssh(
    repo_path: &str,
    remote_name: &str,
) -> thread::JoinHandle<Result<(), git2::Error>> {
    // Clone the strings so the thread owns them
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();

    thread::spawn(move || {
        let repo = Repository::open(repo_path)?;
        let mut remote = repo.find_remote(&remote_name)?;

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _| {
            Cred::ssh_key_from_agent(username_from_url.unwrap())
        });

        callbacks.transfer_progress(|_stats| {
            // println!("Received {}/{} objects", stats.received_objects(), stats.total_objects());
            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        fetch_options.prune(FetchPrune::On);

        remote.fetch(
            &["refs/heads/*:refs/remotes/origin/*", "refs/tags/*:refs/tags/*"],
            Some(&mut fetch_options),
            None,
        )?;
        Ok(())
    })
}

pub fn push_over_ssh(
    repo_path: &str,
    remote_name: &str,
    branch: &str,
    force: bool,
) -> thread::JoinHandle<Result<(), git2::Error>> {
    // Clone inputs so they can move into the thread safely
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();
    let branch = branch.to_string();

    thread::spawn(move || {
        // Open the repository
        let repo = Repository::open(&repo_path)?;
        let mut remote = repo.find_remote(&remote_name)?;

        // Configure SSH authentication
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _| Cred::ssh_key_from_agent("git"));

        // Track progress
        callbacks.push_update_reference(|_refname, status| {
            if let Some(_err) = status {
                // eprintln!("Failed to update {refname}: {err}");
            } else {
                // println!("Updated {refname}");
            }
            Ok(())
        });

        // Configure push options
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        // Build refspecs
        let mut refspecs = vec![];
        
        // Branch
        let branch_refspec = if force {
            format!("+refs/heads/{0}:refs/heads/{0}", branch)
        } else {
            format!("refs/heads/{0}:refs/heads/{0}", branch)
        };
        refspecs.push(branch_refspec);

        // Local tags
        for tag_name in repo.tag_names(None)?.iter().flatten() {
            let tag_refspec = format!("refs/tags/{0}:refs/tags/{0}", tag_name);
            refspecs.push(tag_refspec);
        }

        // Perform the push
        remote.push(&refspecs.iter().map(|s| s.as_str()).collect::<Vec<_>>(), Some(&mut push_options))?;

        // println!("Push complete for branch '{}'", branch);
        Ok(())
    })
}

pub fn create_branch(repo: &Repository, branch_name: &str, target_oid: Oid) -> Result<(), Error> {
    // Find the commit you want the branch to point to
    let target_commit = repo.find_commit(target_oid)?;

    // Create the branch
    repo.branch(branch_name, &target_commit, false)?;

    Ok(())
}

pub fn delete_branch(repo: &Repository, branch: &str) -> Result<(), Error> {

    // Try deleting as a local branch first
    if let Ok(mut local_branch) = repo.find_branch(branch, BranchType::Local) {

        // Delete the local branch
        local_branch.delete()?;
    } else {

        // Delete remote-tracking branch (assume "origin" remote for now)
        let ref_name = format!("refs/remotes/origin/{}", branch);

        if let Ok(mut reference) = repo.find_reference(&ref_name) {
            reference.delete()?;
        } else {
            // Branch not found locally or remotely
            return Err(Error::from_str(&format!("Branch '{}' not found", branch)));
        }
    }

    Ok(())
}

pub fn stash(
    repo: &mut Repository,
) -> Result<Oid, git2::Error> {

    let flags = StashFlags::DEFAULT | StashFlags::INCLUDE_UNTRACKED;

    let message = {
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let short_id = commit.id().to_string()[..7].to_string();
        let summary = commit.summary().unwrap_or("WIP");
        format!("{} {}", short_id, summary)
    };

    let stash_index = repo.stash_save(
        &repo.signature()?,
        message.as_str(),
        Some(flags),
    )?;

    Ok(stash_index)
}

pub fn pop(
    repo: &mut Repository,
    target_oid: &Oid,
    apply: bool
) -> Result<(), git2::Error> {

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
            let mut opts = StashApplyOptions::new();
            repo.stash_apply(index, Some(&mut opts))?;
        }
        repo.stash_drop(index)?;
    }

    Ok(())
}

pub fn tag(
    repo: &Repository,
    oid: git2::Oid,
    tag: &str,
) -> Result<Oid, Error> {
    repo.tag_lightweight(tag, &repo.find_object(oid, None)?, false)
}

pub fn untag(
    repo: &Repository,
    tag: &str
) -> Result<(), Error> {
    repo.tag_delete(tag)
}

pub fn cherry_pick_commit(
    repo: &Repository,
    commit_oid: Oid,
    message: Option<&str>, // optional override for commit message
    allow_conflicts: bool, // true -> force working dir changes
) -> Result<Oid, Error> {
    // Find the commit to cherry-pick
    let commit = repo.find_commit(commit_oid)?;

    // Get current HEAD commit
    let head_commit = repo.head()?.peel_to_commit()?;

    // Prepare cherry-pick options
    let mut cherrypick_opts = CherrypickOptions::new();

    // Perform cherry-pick
    repo.cherrypick(&commit, Some(&mut cherrypick_opts))?;

    // Get the index after cherry-pick
    let mut index = repo.index()?;

    // If conflicts exist
    if index.has_conflicts() {
        if allow_conflicts {
            let conflicts: Vec<_> = index
                .conflicts()?
                .flatten()
                .filter_map(|e| e.our)
                .collect();
            for conflict in conflicts {
                index.add(&conflict)?;
            }
        } else {
            return Err(Error::from_str("Cherry-pick conflicts detected"));
        }
    }

    // Write tree
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Create commit signature
    let sig = repo.signature()?;

    // Commit message
    let commit_message = message.unwrap_or_else(|| commit.message().unwrap_or("Cherry-pick commit"));

    // Determine parents: HEAD
    let parents = [&head_commit];

    // Create the new commit
    let new_commit_oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        commit_message,
        &tree,
        &parents,
    )?;

    // Update working directory
    repo.checkout_head(Some(CheckoutBuilder::default().allow_conflicts(allow_conflicts).force()))?;

    Ok(new_commit_oid)
}
