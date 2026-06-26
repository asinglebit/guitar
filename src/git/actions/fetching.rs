use crate::{
    git::auth::{AuthAttempt, AuthSession, NetworkResult, network_result},
    helpers::localisation::network,
};
use std::thread;

// Run fetch on a worker thread so auth prompts and network latency stay outside the draw loop.
pub fn fetch_remote(repo_path: &str, remote_name: &str, auth_session: AuthSession) -> thread::JoinHandle<NetworkResult> {
    // Own the inputs before crossing the thread boundary.
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();

    thread::spawn(move || {
        let attempt = AuthAttempt::new(auth_session, network::FETCH());
        let result = (|| -> Result<(), git2::Error> {
            let mut repo = gix::open(&repo_path).map_err(|error| git2::Error::from_str(&error.to_string()))?;
            repo.committer_or_set_generic_fallback().map_err(|error| git2::Error::from_str(&error.to_string()))?;

            let remote_name = remote_name.as_str();
            let mut remote = repo.find_remote(remote_name).map_err(|error| git2::Error::from_str(&error.to_string()))?;
            let remote_url = remote.url(gix::remote::Direction::Fetch).ok_or_else(|| git2::Error::from_str("Remote URL is missing"))?.to_owned();

            remote = remote.with_fetch_tags(gix::remote::fetch::Tags::All);
            let heads = format!("+refs/heads/*:refs/remotes/{remote_name}/*");
            remote = remote.with_refspecs(Some(heads.as_str()), gix::remote::Direction::Fetch).map_err(|error| git2::Error::from_str(&error.to_string()))?;
            remote = remote.with_refspecs(Some("refs/tags/*:refs/tags/*"), gix::remote::Direction::Fetch).map_err(|error| git2::Error::from_str(&error.to_string()))?;

            let mut connection = remote.connect(gix::remote::Direction::Fetch).map_err(|error| git2::Error::from_str(&error.to_string()))?;
            let mut configured_credentials = connection.configured_credentials(remote_url).map_err(|error| git2::Error::from_str(&error.to_string()))?;
            let auth = attempt.clone();
            connection.set_credentials(move |action| auth.gix_credentials_with(action, &mut configured_credentials));

            let mut progress = gix::progress::Discard;
            let pending_pack = connection.prepare_fetch(&mut progress, Default::default()).map_err(|error| git2::Error::from_str(&error.to_string()))?;
            let should_interrupt = std::sync::atomic::AtomicBool::new(false);
            pending_pack.receive(&mut progress, &should_interrupt).map_err(|error| git2::Error::from_str(&error.to_string()))?;
            Ok(())
        })();

        network_result(network::FETCH(), &attempt, result)
    })
}

#[cfg(test)]
#[path = "../../tests/git/actions/fetching.rs"]
mod tests;
