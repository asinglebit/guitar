use crate::{
    git::auth::{AuthAttempt, AuthSession, NetworkResult, network_result},
    helpers::localisation::network,
};
use git2::FetchPrune;
use git2::{FetchOptions, RemoteCallbacks, Repository};
use std::thread;

// Run fetch on a worker thread so auth prompts and network latency stay outside the draw loop.
pub fn fetch_remote(repo_path: &str, remote_name: &str, auth_session: AuthSession) -> thread::JoinHandle<NetworkResult> {
    // Own the inputs before crossing the thread boundary.
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();

    thread::spawn(move || {
        let attempt = AuthAttempt::new(auth_session, network::FETCH());
        let result = (|| -> Result<(), git2::Error> {
            let repo = Repository::open(repo_path)?;
            let mut remote = repo.find_remote(&remote_name)?;
            let config = repo.config()?;

            let mut callbacks = RemoteCallbacks::new();
            let auth = attempt.clone();
            callbacks.credentials(move |url, username_from_url, allowed| auth.credentials(&config, url, username_from_url, allowed));

            callbacks.transfer_progress(|_stats| {
                // println!("Received {}/{} objects", stats.received_objects(), stats.total_objects());
                true
            });

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);
            fetch_options.prune(FetchPrune::On);

            // Fetch heads and tags explicitly because libgit2 does not expand all refspecs by default.
            let heads = format!("refs/heads/*:refs/remotes/{remote_name}/*");
            remote.fetch(&[heads.as_str(), "refs/tags/*:refs/tags/*"], Some(&mut fetch_options), None)?;
            Ok(())
        })();

        network_result(network::FETCH(), &attempt, result)
    })
}

// Result of a background fetch. `changed` drives whether the app reloads at all: an auto fetch that
// moved nothing must leave the UI completely alone, because reload() rewalks the whole graph.
pub struct QuietFetchOutcome {
    pub changed: bool,
    pub ok: bool,
}

// Snapshot the refs a fetch can move, so the worker can tell whether it actually brought anything in.
fn ref_snapshot(repo: &Repository, remote_name: &str) -> Vec<(String, git2::Oid)> {
    let mut refs = Vec::new();
    for glob in [format!("refs/remotes/{remote_name}/*"), "refs/tags/*".to_string()] {
        let Ok(found) = repo.references_glob(&glob) else {
            continue;
        };
        refs.extend(found.flatten().filter_map(|reference| Some((reference.name()?.to_string(), reference.target()?))));
    }
    refs.sort_by(|left, right| left.0.cmp(&right.0));
    refs
}

// The silent sibling of fetch_remote, used by the background auto fetcher. It never prompts and
// never reports: an auth challenge or any other failure simply comes back as `ok: false`, which the
// app turns into a back-off rather than a modal.
pub fn fetch_remote_quiet(repo_path: &str, remote_name: &str, auth_session: AuthSession) -> thread::JoinHandle<QuietFetchOutcome> {
    // Own the inputs before crossing the thread boundary.
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();

    thread::spawn(move || {
        let attempt = AuthAttempt::new(auth_session, network::FETCH());
        let result = (|| -> Result<bool, git2::Error> {
            let repo = Repository::open(repo_path)?;
            let before = ref_snapshot(&repo, &remote_name);

            {
                let mut remote = repo.find_remote(&remote_name)?;
                let config = repo.config()?;

                let mut callbacks = RemoteCallbacks::new();
                let auth = attempt.clone();
                callbacks.credentials(move |url, username_from_url, allowed| auth.credentials(&config, url, username_from_url, allowed));

                let mut fetch_options = FetchOptions::new();
                fetch_options.remote_callbacks(callbacks);
                fetch_options.prune(FetchPrune::On);

                let heads = format!("refs/heads/*:refs/remotes/{remote_name}/*");
                remote.fetch(&[heads.as_str(), "refs/tags/*:refs/tags/*"], Some(&mut fetch_options), None)?;
            }

            Ok(ref_snapshot(&repo, &remote_name) != before)
        })();

        match result {
            Ok(changed) => QuietFetchOutcome { changed, ok: true },
            Err(_) => QuietFetchOutcome { changed: false, ok: false },
        }
    })
}
