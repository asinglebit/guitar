use crate::{
    git::actions::gix_support::{open_repo_path, to_git2_error},
    git::auth::{AuthAttempt, AuthSession, NetworkResult, network_result},
    helpers::localisation::network,
};
use std::thread;

fn fetch_remote_result(repo_path: &str, remote_name: &str, attempt: &AuthAttempt) -> Result<(), git2::Error> {
    let mut repo = open_repo_path(repo_path)?;
    repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;

    let mut remote = repo.find_remote(remote_name).map_err(to_git2_error)?;
    let remote_url = remote.url(gix::remote::Direction::Fetch).ok_or_else(|| git2::Error::from_str("Remote URL is missing"))?.to_owned();

    remote = remote.with_fetch_tags(gix::remote::fetch::Tags::All);
    let heads = format!("+refs/heads/*:refs/remotes/{remote_name}/*");
    remote = remote.with_refspecs(Some(heads.as_str()), gix::remote::Direction::Fetch).map_err(to_git2_error)?;
    remote = remote.with_refspecs(Some("refs/tags/*:refs/tags/*"), gix::remote::Direction::Fetch).map_err(to_git2_error)?;

    let mut connection = remote.connect(gix::remote::Direction::Fetch).map_err(to_git2_error)?;
    let mut configured_credentials = connection.configured_credentials(remote_url).map_err(to_git2_error)?;
    let auth = attempt.clone();
    connection.set_credentials(move |action| auth.gix_credentials_with(action, &mut configured_credentials));

    let mut progress = gix::progress::Discard;
    let pending_pack = connection.prepare_fetch(&mut progress, Default::default()).map_err(to_git2_error)?;
    let should_interrupt = std::sync::atomic::AtomicBool::new(false);
    pending_pack.receive(&mut progress, &should_interrupt).map(drop).map_err(to_git2_error)
}

// Run fetch on a worker thread so auth prompts and network latency stay outside the draw loop.
pub fn fetch_remote(repo_path: &str, remote_name: &str, auth_session: AuthSession) -> thread::JoinHandle<NetworkResult> {
    // Own the inputs before crossing the thread boundary.
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();

    thread::spawn(move || {
        let operation = network::FETCH();
        let attempt = AuthAttempt::new(auth_session, operation);
        let result = fetch_remote_result(&repo_path, &remote_name, &attempt);
        network_result(operation, &attempt, result)
    })
}

#[cfg(test)]
#[path = "../../tests/git/actions/fetching.rs"]
mod tests;
