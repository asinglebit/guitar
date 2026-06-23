use gix::{bstr::ByteSlice, remote};
use std::path::Path;

pub const GUITAR_DEFAULT_REMOTE_CONFIG: &str = "guitar.defaultRemote";
pub const PUSH_DEFAULT_CONFIG: &str = "remote.pushDefault";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEntry {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

pub fn list_remotes(path: impl AsRef<Path>) -> Result<Vec<RemoteEntry>, git2::Error> {
    let repo = open_repo(path)?;
    list_remotes_from_repo(&repo)
}

pub fn effective_default_remote(path: impl AsRef<Path>) -> Option<String> {
    let repo = open_repo(path).ok()?;
    let remotes = list_remotes_from_repo(&repo).ok()?;
    resolve_effective_default_remote(&repo, &remotes)
}

pub fn effective_default_remote_from_remotes(path: impl AsRef<Path>, remotes: &[RemoteEntry]) -> Option<String> {
    let repo = open_repo(path).ok()?;
    resolve_effective_default_remote(&repo, remotes)
}

fn resolve_effective_default_remote(repo: &gix::Repository, remotes: &[RemoteEntry]) -> Option<String> {
    if remotes.is_empty() {
        return None;
    }

    if let Some(remote) = repo_config_remote(repo, GUITAR_DEFAULT_REMOTE_CONFIG, remotes) {
        return Some(remote);
    }

    if let Some(remote) = repo_config_remote(repo, PUSH_DEFAULT_CONFIG, remotes) {
        return Some(remote);
    }

    if let Some(remote) = current_branch_upstream_remote(repo, remotes) {
        return Some(remote);
    }

    if remote_exists(remotes, "origin") {
        return Some("origin".to_string());
    }

    remotes.first().map(|remote| remote.name.clone())
}

fn open_repo(path: impl AsRef<Path>) -> Result<gix::Repository, git2::Error> {
    gix::open(path.as_ref()).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn list_remotes_from_repo(repo: &gix::Repository) -> Result<Vec<RemoteEntry>, git2::Error> {
    let mut entries = Vec::new();

    for name in repo.remote_names() {
        let Some(name) = name.to_str().ok().map(str::to_string) else {
            continue;
        };
        let remote = repo.find_remote(name.as_str()).map_err(|error| git2::Error::from_str(&error.to_string()))?;
        let push_url = remote_push_url(repo, &name);
        entries.push(RemoteEntry { name, url: remote.url(remote::Direction::Fetch).map(|url| url.to_string()).unwrap_or_default(), push_url });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

fn remote_push_url(repo: &gix::Repository, remote_name: &str) -> Option<String> {
    let key = format!("remote.{remote_name}.pushUrl");
    let value = repo.config_snapshot().string(&key)?;
    Some(value.to_str().ok()?.to_string())
}

fn remote_exists(remotes: &[RemoteEntry], name: &str) -> bool {
    remotes.iter().any(|remote| remote.name == name)
}

fn repo_config_remote(repo: &gix::Repository, key: &str, remotes: &[RemoteEntry]) -> Option<String> {
    repo.config_snapshot().string(key).and_then(|value| value.to_str().ok().map(|value| value.trim().to_string())).filter(|name| remote_exists(remotes, name))
}

fn current_branch_upstream_remote(repo: &gix::Repository, remotes: &[RemoteEntry]) -> Option<String> {
    let branch = repo.head_name().ok().flatten()?.shorten().to_str().ok()?.to_string();
    let key = format!("branch.{branch}.remote");
    repo_config_remote(repo, &key, remotes)
}

#[cfg(test)]
#[path = "../../tests/git/queries/remotes.rs"]
mod tests;
