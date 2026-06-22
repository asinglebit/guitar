use git2::Repository;
use gix::{bstr::ByteSlice, remote};

pub const GUITAR_DEFAULT_REMOTE_CONFIG: &str = "guitar.defaultRemote";
pub const PUSH_DEFAULT_CONFIG: &str = "remote.pushDefault";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEntry {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

pub fn list_remotes(repo: &Repository) -> Result<Vec<RemoteEntry>, git2::Error> {
    let gix_repo = open_gix_repo(repo)?;
    list_remotes_gix(&gix_repo)
}

pub fn effective_default_remote(repo: &Repository) -> Option<String> {
    let gix_repo = open_gix_repo(repo).ok()?;
    let remotes = list_remotes_gix(&gix_repo).ok()?;
    effective_default_remote_from_gix_remotes(&gix_repo, &remotes)
}

pub fn effective_default_remote_from_remotes(repo: &Repository, remotes: &[RemoteEntry]) -> Option<String> {
    let gix_repo = open_gix_repo(repo).ok()?;
    effective_default_remote_from_gix_remotes(&gix_repo, remotes)
}

pub fn list_remotes_from_gix(repo: &gix::Repository) -> Result<Vec<RemoteEntry>, git2::Error> {
    list_remotes_gix(repo)
}

pub fn effective_default_remote_from_gix_remotes(repo: &gix::Repository, remotes: &[RemoteEntry]) -> Option<String> {
    if remotes.is_empty() {
        return None;
    }

    if let Some(remote) = repo_config_remote_gix(repo, GUITAR_DEFAULT_REMOTE_CONFIG, remotes) {
        return Some(remote);
    }

    if let Some(remote) = repo_config_remote_gix(repo, PUSH_DEFAULT_CONFIG, remotes) {
        return Some(remote);
    }

    if let Some(remote) = current_branch_upstream_remote_gix(repo, remotes) {
        return Some(remote);
    }

    if remote_exists(remotes, "origin") {
        return Some("origin".to_string());
    }

    remotes.first().map(|remote| remote.name.clone())
}

fn open_gix_repo(repo: &Repository) -> Result<gix::Repository, git2::Error> {
    let path = repo.workdir().unwrap_or(repo.path());
    gix::open(path).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn list_remotes_gix(repo: &gix::Repository) -> Result<Vec<RemoteEntry>, git2::Error> {
    let mut entries = Vec::new();

    for name in repo.remote_names() {
        let Some(name) = name.to_str().ok().map(str::to_string) else {
            continue;
        };
        let remote = repo.find_remote(name.as_str()).map_err(|error| git2::Error::from_str(&error.to_string()))?;
        let push_url = remote_push_url_gix(repo, &name);
        entries.push(RemoteEntry { name, url: remote.url(remote::Direction::Fetch).map(|url| url.to_string()).unwrap_or_default(), push_url });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

fn remote_push_url_gix(repo: &gix::Repository, remote_name: &str) -> Option<String> {
    let key = format!("remote.{remote_name}.pushUrl");
    let value = repo.config_snapshot().string(&key)?;
    Some(value.to_str().ok()?.to_string())
}

fn remote_exists(remotes: &[RemoteEntry], name: &str) -> bool {
    remotes.iter().any(|remote| remote.name == name)
}

fn repo_config_remote_gix(repo: &gix::Repository, key: &str, remotes: &[RemoteEntry]) -> Option<String> {
    repo.config_snapshot().string(key).and_then(|value| value.to_str().ok().map(|value| value.trim().to_string())).filter(|name| remote_exists(remotes, name))
}

fn current_branch_upstream_remote_gix(repo: &gix::Repository, remotes: &[RemoteEntry]) -> Option<String> {
    let branch = repo.head_name().ok().flatten()?.shorten().to_str().ok()?.to_string();
    let key = format!("branch.{branch}.remote");
    repo_config_remote_gix(repo, &key, remotes)
}

#[cfg(test)]
#[path = "../../tests/git/queries/remotes.rs"]
mod tests;
