use crate::git::queries::commits::get_current_branch;
use git2::Repository;

pub const GUITAR_DEFAULT_REMOTE_CONFIG: &str = "guitar.defaultRemote";
pub const PUSH_DEFAULT_CONFIG: &str = "remote.pushDefault";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEntry {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

pub fn list_remotes(repo: &Repository) -> Result<Vec<RemoteEntry>, git2::Error> {
    let mut entries = Vec::new();

    for name in repo.remotes()?.iter().flatten() {
        let remote = repo.find_remote(name)?;
        entries.push(RemoteEntry { name: name.to_string(), url: remote.url().unwrap_or_default().to_string(), push_url: remote.pushurl().map(str::to_string) });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

pub fn effective_default_remote(repo: &Repository) -> Option<String> {
    let remotes = list_remotes(repo).ok()?;
    if remotes.is_empty() {
        return None;
    }

    if let Some(remote) = repo_config_remote(repo, GUITAR_DEFAULT_REMOTE_CONFIG, &remotes) {
        return Some(remote);
    }

    if let Some(remote) = repo_config_remote(repo, PUSH_DEFAULT_CONFIG, &remotes) {
        return Some(remote);
    }

    if let Some(remote) = current_branch_upstream_remote(repo, &remotes) {
        return Some(remote);
    }

    if remote_exists(&remotes, "origin") {
        return Some("origin".to_string());
    }

    remotes.first().map(|remote| remote.name.clone())
}

fn repo_config_remote(repo: &Repository, key: &str, remotes: &[RemoteEntry]) -> Option<String> {
    repo.config().ok().and_then(|config| config.get_string(key).ok()).map(|value| value.trim().to_string()).filter(|name| remote_exists(remotes, name))
}

fn current_branch_upstream_remote(repo: &Repository, remotes: &[RemoteEntry]) -> Option<String> {
    let branch = get_current_branch(repo)?;
    let refname = format!("refs/heads/{branch}");
    repo.branch_upstream_remote(&refname).ok().and_then(|remote| remote.as_str().map(str::to_string)).filter(|name| remote_exists(remotes, name))
}

fn remote_exists(remotes: &[RemoteEntry], name: &str) -> bool {
    remotes.iter().any(|remote| remote.name == name)
}

#[cfg(test)]
#[path = "../../tests/git/queries/remotes.rs"]
mod tests;
