use git2::{build::CheckoutBuilder, BranchType, Cred, Error, ErrorCode, FetchOptions, Oid, PushOptions, RemoteCallbacks, Repository, ResetType, Signature, StatusOptions};
use git2::{CherrypickOptions, FetchPrune, StashApplyOptions, StashFlags};
use std::{collections::HashMap, thread};

pub fn fetch_over_ssh(repo_path: &str, remote_name: &str) -> thread::JoinHandle<Result<(), git2::Error>> {
    // Clone the strings so the thread owns them
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();

    thread::spawn(move || {
        let repo = Repository::open(repo_path)?;
        let mut remote = repo.find_remote(&remote_name)?;

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _| Cred::ssh_key_from_agent(username_from_url.unwrap()));

        callbacks.transfer_progress(|_stats| {
            // println!("Received {}/{} objects", stats.received_objects(), stats.total_objects());
            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        fetch_options.prune(FetchPrune::On);

        remote.fetch(&["refs/heads/*:refs/remotes/origin/*", "refs/tags/*:refs/tags/*"], Some(&mut fetch_options), None)?;
        Ok(())
    })
}
