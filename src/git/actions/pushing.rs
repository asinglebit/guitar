use crate::{
    git::auth::{AuthAttempt, AuthSession, NetworkResult, network_result},
    git::repository::open,
    helpers::localisation::network,
};
use git2::{PushOptions, RemoteCallbacks};
use std::thread;

fn auth_callbacks<'a>(attempt: AuthAttempt, config: git2::Config) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, allowed| attempt.credentials(&config, url, username_from_url, allowed));
    callbacks
}

fn auth_push_callbacks<'a>(attempt: AuthAttempt, config: git2::Config) -> RemoteCallbacks<'a> {
    let mut callbacks = auth_callbacks(attempt, config);
    callbacks.push_update_reference(|refname, status| {
        if let Some(err) = status {
            return Err(git2::Error::from_str(&format!("Failed to update {refname}: {err}")));
        }
        Ok(())
    });
    callbacks
}

// Pushes are threaded so network latency does not have to live inside command handlers.
pub fn push_branch(repo_path: &str, remote_name: &str, branch: &str, force: bool, auth_session: AuthSession) -> thread::JoinHandle<NetworkResult> {
    // Own the inputs before crossing the thread boundary.
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();
    let branch = branch.to_string();

    thread::spawn(move || {
        let attempt = AuthAttempt::new(auth_session, network::PUSH());
        let result = (|| -> Result<(), git2::Error> {
            let repo = open(&repo_path)?;
            let mut remote = repo.find_remote(&remote_name)?;
            let config = repo.config()?;

            // Configure push options
            let mut push_options = PushOptions::new();
            push_options.remote_callbacks(auth_push_callbacks(attempt.clone(), config));

            // Match `git push --force <remote> <branch>`: update the current branch only.
            // Tags are intentionally excluded because plain force push does not update them.
            let branch_refspec = if force { format!("+refs/heads/{0}:refs/heads/{0}", branch) } else { format!("refs/heads/{0}:refs/heads/{0}", branch) };

            remote.push(&[branch_refspec.as_str()], Some(&mut push_options))?;

            // println!("Push complete for branch '{}'", branch);
            Ok(())
        })();

        network_result(network::PUSH(), &attempt, result)
    })
}

pub fn push_tags(repo_path: &str, remote_name: &str, auth_session: AuthSession) -> thread::JoinHandle<NetworkResult> {
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();

    thread::spawn(move || {
        let attempt = AuthAttempt::new(auth_session, network::PUSH_TAGS());
        let result = (|| -> Result<(), git2::Error> {
            let repo = open(&repo_path)?;
            let mut remote = repo.find_remote(&remote_name)?;
            let config = repo.config()?;

            let mut push_options = PushOptions::new();
            push_options.remote_callbacks(auth_push_callbacks(attempt.clone(), config));

            // Build one explicit refspec per local tag so existing branches are untouched.
            let tag_refspecs = repo.tag_names(None)?.iter().flatten().map(|tag_name| format!("refs/tags/{0}:refs/tags/{0}", tag_name)).collect::<Vec<_>>();

            if tag_refspecs.is_empty() {
                return Ok(());
            }

            let refspecs = tag_refspecs.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            remote.push(&refspecs, Some(&mut push_options))?;

            Ok(())
        })();

        network_result(network::PUSH_TAGS(), &attempt, result)
    })
}

pub fn delete_remote_branch(repo_path: &str, remote_name: &str, branch: &str, auth_session: AuthSession) -> thread::JoinHandle<NetworkResult> {
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();
    let branch = branch.to_string();

    thread::spawn(move || {
        let attempt = AuthAttempt::new(auth_session, network::DELETE_REMOTE_BRANCH());
        let result = (|| -> Result<(), git2::Error> {
            let repo = open(&repo_path)?;
            let mut remote = repo.find_remote(&remote_name)?;
            let config = repo.config()?;

            let mut push_options = PushOptions::new();
            push_options.remote_callbacks(auth_push_callbacks(attempt.clone(), config));

            // An empty source refspec asks the remote to delete the destination branch.
            let refspec = format!(":refs/heads/{}", branch);

            remote.push(&[&refspec], Some(&mut push_options))?;

            Ok(())
        })();

        network_result(network::DELETE_REMOTE_BRANCH(), &attempt, result)
    })
}
