use crate::{
    git::auth::{AuthAttempt, AuthSession, NetworkResult, network_result},
    helpers::localisation::network,
};
use git2::{FetchOptions, RemoteCallbacks, Repository, SubmoduleUpdateOptions};
use std::thread;

pub fn sync_submodule(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let mut submodule = repo.find_submodule(name)?;
    submodule.sync()
}

pub fn stage_submodule_head(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let mut submodule = repo.find_submodule(name)?;
    submodule.add_to_index(true)
}

pub fn unstage_submodule(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let submodule = repo.find_submodule(name)?;
    let path = submodule.path().to_path_buf();
    let head = match repo.head() {
        Ok(head) => head.peel_to_commit()?,
        Err(_) => {
            let mut index = repo.index()?;
            index.remove_path(path.as_path())?;
            index.write()?;
            return Ok(());
        },
    };

    repo.reset_default(Some(&head.into_object()), [path.as_path()])?;
    Ok(())
}

pub fn update_submodule(repo_path: &str, name: &str, auth_session: AuthSession) -> thread::JoinHandle<NetworkResult> {
    let repo_path = repo_path.to_string();
    let name = name.to_string();

    thread::spawn(move || {
        let attempt = AuthAttempt::new(auth_session, network::UPDATE_SUBMODULE());
        let result = (|| -> Result<(), git2::Error> {
            let repo = Repository::open(&repo_path)?;
            let config = repo.config()?;
            let mut submodule = repo.find_submodule(&name)?;

            let mut callbacks = RemoteCallbacks::new();
            let auth = attempt.clone();
            callbacks.credentials(move |url, username_from_url, allowed| auth.credentials(&config, url, username_from_url, allowed));

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            let mut options = SubmoduleUpdateOptions::new();
            options.fetch(fetch_options);

            submodule.update(true, Some(&mut options))
        })();

        network_result(network::UPDATE_SUBMODULE(), &attempt, result)
    })
}

#[cfg(test)]
#[path = "../../tests/git/actions/submodules.rs"]
mod tests;
