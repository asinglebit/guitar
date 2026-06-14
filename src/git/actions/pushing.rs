use git2::{Cred, PushOptions, RemoteCallbacks, Repository};
use std::thread;

fn ssh_callbacks<'a>() -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _| Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")));
    callbacks
}

fn ssh_push_callbacks<'a>() -> RemoteCallbacks<'a> {
    let mut callbacks = ssh_callbacks();
    callbacks.push_update_reference(|refname, status| {
        if let Some(err) = status {
            return Err(git2::Error::from_str(&format!("Failed to update {refname}: {err}")));
        }
        Ok(())
    });
    callbacks
}

// Pushes are threaded so network latency does not have to live inside command handlers.
pub fn push_over_ssh(repo_path: &str, remote_name: &str, branch: &str, force: bool) -> thread::JoinHandle<Result<(), git2::Error>> {
    // Own the inputs before crossing the thread boundary.
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();
    let branch = branch.to_string();

    thread::spawn(move || {
        let repo = Repository::open(&repo_path)?;
        let mut remote = repo.find_remote(&remote_name)?;

        // Configure push options
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(ssh_push_callbacks());

        // Match `git push --force <remote> <branch>`: update the current branch only.
        // Tags are intentionally excluded because plain force push does not update them.
        let branch_refspec = if force { format!("+refs/heads/{0}:refs/heads/{0}", branch) } else { format!("refs/heads/{0}:refs/heads/{0}", branch) };

        remote.push(&[branch_refspec.as_str()], Some(&mut push_options))?;

        // println!("Push complete for branch '{}'", branch);
        Ok(())
    })
}

pub fn push_tags_over_ssh(repo_path: &str, remote_name: &str) -> thread::JoinHandle<Result<(), git2::Error>> {
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();

    thread::spawn(move || {
        let repo = Repository::open(&repo_path)?;
        let mut remote = repo.find_remote(&remote_name)?;

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(ssh_push_callbacks());

        // Build one explicit refspec per local tag so existing branches are untouched.
        let tag_refspecs = repo.tag_names(None)?.iter().flatten().map(|tag_name| format!("refs/tags/{0}:refs/tags/{0}", tag_name)).collect::<Vec<_>>();

        if tag_refspecs.is_empty() {
            return Ok(());
        }

        let refspecs = tag_refspecs.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        remote.push(&refspecs, Some(&mut push_options))?;

        Ok(())
    })
}

pub fn delete_remote_branch_ssh(repo_path: &str, remote_name: &str, branch: &str) -> thread::JoinHandle<Result<(), git2::Error>> {
    let repo_path = repo_path.to_string();
    let remote_name = remote_name.to_string();
    let branch = branch.to_string();

    thread::spawn(move || {
        let repo = Repository::open(&repo_path)?;
        let mut remote = repo.find_remote(&remote_name)?;

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(ssh_callbacks());

        // An empty source refspec asks the remote to delete the destination branch.
        let refspec = format!(":refs/heads/{}", branch);

        remote.push(&[&refspec], Some(&mut push_options))?;

        Ok(())
    })
}
