use git2::{Cred, PushOptions, RemoteCallbacks, Repository};
use std::thread;

pub fn push_over_ssh(repo_path: &str, remote_name: &str, branch: &str, force: bool) -> thread::JoinHandle<Result<(), git2::Error>> {
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
        callbacks.credentials(|_url, username_from_url, _| Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")));

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

        // Match `git push --force <remote> <branch>`: update the current branch only.
        // Tags are intentionally not included because plain `git push --force`
        // does not push them, and some servers reject tag updates.
        let branch_refspec = if force { format!("+refs/heads/{0}:refs/heads/{0}", branch) } else { format!("refs/heads/{0}:refs/heads/{0}", branch) };

        // Perform the push
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

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _| Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")));

        callbacks.push_update_reference(|_refname, status| {
            if let Some(_err) = status {
                // eprintln!("Failed to update {refname}: {err}");
            }
            Ok(())
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

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

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _| Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")));

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        // Deletion refspec
        let refspec = format!(":refs/heads/{}", branch);

        remote.push(&[&refspec], Some(&mut push_options))?;

        Ok(())
    })
}
