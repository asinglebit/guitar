use crate::{
    git::actions::gix_support::{open_repo_path, to_git2_error},
    git::auth::{AuthAttempt, AuthSession, NetworkResult, network_result},
    helpers::localisation::network,
};
use git2::Repository;
use gix::bstr::ByteSlice;
use std::{collections::HashSet, thread};

const TAG_FETCH_REFSPEC: &str = "refs/tags/*:refs/tags/*";

struct FetchRemoteJob {
    repo_path: String,
    remote_name: String,
    auth_session: AuthSession,
}

impl FetchRemoteJob {
    fn new(repo_path: &str, remote_name: &str, auth_session: AuthSession) -> Self {
        Self { repo_path: repo_path.to_string(), remote_name: remote_name.to_string(), auth_session }
    }

    fn run(self) -> NetworkResult {
        let operation = network::FETCH();
        let attempt = AuthAttempt::new(self.auth_session, operation);
        let result = fetch_remote_result(&self.repo_path, &self.remote_name, &attempt);
        network_result(operation, &attempt, result)
    }
}

fn fetch_remote_result(repo_path: &str, remote_name: &str, attempt: &AuthAttempt) -> Result<(), git2::Error> {
    let repo = prepare_fetch_repo(repo_path)?;
    let remote = repo.find_remote(remote_name).map_err(to_git2_error)?;
    let remote = configure_fetch_remote(remote, remote_name)?;
    let remote_url = remote.url(gix::remote::Direction::Fetch).ok_or_else(|| git2::Error::from_str("Remote URL is missing"))?.to_owned();

    let mut connection = remote.connect(gix::remote::Direction::Fetch).map_err(to_git2_error)?;
    let mut configured_credentials = connection.configured_credentials(remote_url).map_err(to_git2_error)?;
    let auth = attempt.clone();
    connection.set_credentials(move |action| auth.gix_credentials_with(action, &mut configured_credentials));

    let mut progress = gix::progress::Discard;
    let pending_pack = connection.prepare_fetch(&mut progress, Default::default()).map_err(to_git2_error)?;
    let should_interrupt = std::sync::atomic::AtomicBool::new(false);
    let outcome = pending_pack.receive(&mut progress, &should_interrupt).map_err(to_git2_error)?;
    prune_stale_remote_tracking_refs(repo_path, remote_name, &outcome.ref_map)
}

fn prepare_fetch_repo(repo_path: &str) -> Result<gix::Repository, git2::Error> {
    let mut repo = open_repo_path(repo_path)?;
    repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;
    Ok(repo)
}

fn heads_fetch_refspec(remote_name: &str) -> String {
    format!("+refs/heads/*:refs/remotes/{remote_name}/*")
}

fn configure_fetch_remote<'repo>(remote: gix::Remote<'repo>, remote_name: &str) -> Result<gix::Remote<'repo>, git2::Error> {
    let heads = heads_fetch_refspec(remote_name);
    remote.with_fetch_tags(gix::remote::fetch::Tags::All).with_refspecs([heads.as_str(), TAG_FETCH_REFSPEC], gix::remote::Direction::Fetch).map_err(to_git2_error)
}

fn prune_stale_remote_tracking_refs(repo_path: &str, remote_name: &str, ref_map: &gix::remote::fetch::RefMap) -> Result<(), git2::Error> {
    let prefix = format!("refs/remotes/{remote_name}/");
    let remote_head = format!("{prefix}HEAD");
    let advertised: HashSet<String> = ref_map.mappings.iter().filter_map(|mapping| mapping.local.as_ref()?.to_str().ok()).filter(|name| name.starts_with(&prefix)).map(str::to_owned).collect();
    let repo = Repository::open(repo_path)?;
    let stale_refs = repo
        .references()?
        .filter_map(Result::ok)
        .filter_map(|reference| reference.name().map(str::to_owned))
        .filter(|name| name.starts_with(&prefix) && name != &remote_head && !advertised.contains(name))
        .collect::<Vec<_>>();

    for name in stale_refs {
        if let Ok(mut reference) = repo.find_reference(&name) {
            reference.delete()?;
        }
    }

    Ok(())
}

// Run fetch on a worker thread so auth prompts and network latency stay outside the draw loop.
pub fn fetch_remote(repo_path: &str, remote_name: &str, auth_session: AuthSession) -> thread::JoinHandle<NetworkResult> {
    let job = FetchRemoteJob::new(repo_path, remote_name, auth_session);
    thread::spawn(move || job.run())
}

#[cfg(test)]
#[path = "../../tests/git/actions/fetching.rs"]
mod tests;
