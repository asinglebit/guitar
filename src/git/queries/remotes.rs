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
    effective_default_remote_gix(&gix_repo)
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

fn effective_default_remote_gix(repo: &gix::Repository) -> Option<String> {
    let remotes = list_remotes_gix(repo).ok()?;
    if remotes.is_empty() {
        return None;
    }

    if let Some(remote) = repo_config_remote_gix(repo, GUITAR_DEFAULT_REMOTE_CONFIG, &remotes) {
        return Some(remote);
    }

    if let Some(remote) = repo_config_remote_gix(repo, PUSH_DEFAULT_CONFIG, &remotes) {
        return Some(remote);
    }

    if let Some(remote) = current_branch_upstream_remote_gix(repo, &remotes) {
        return Some(remote);
    }

    if remote_exists(&remotes, "origin") {
        return Some("origin".to_string());
    }

    remotes.first().map(|remote| remote.name.clone())
}

fn repo_config_remote_gix(repo: &gix::Repository, key: &str, remotes: &[RemoteEntry]) -> Option<String> {
    repo.config_snapshot().string(key).and_then(|value| value.to_str().ok().map(|value| value.trim().to_string())).filter(|name| remote_exists(remotes, name))
}

fn current_branch_upstream_remote_gix(repo: &gix::Repository, remotes: &[RemoteEntry]) -> Option<String> {
    let reference = repo.head().ok()?.try_into_referent()?;
    let remote = reference.remote_name(remote::Direction::Fetch)?;
    let name = remote.as_symbol()?.to_string();
    remote_exists(remotes, &name).then_some(name)
}

fn remote_exists(remotes: &[RemoteEntry], name: &str) -> bool {
    remotes.iter().any(|remote| remote.name == name)
}

#[cfg(test)]
fn list_remotes_git2(repo: &Repository) -> Result<Vec<RemoteEntry>, git2::Error> {
    let mut entries = Vec::new();

    for name in repo.remotes()?.iter().flatten() {
        let remote = repo.find_remote(name)?;
        entries.push(RemoteEntry { name: name.to_string(), url: remote.url().unwrap_or_default().to_string(), push_url: remote.pushurl().map(str::to_string) });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

#[cfg(test)]
fn effective_default_remote_git2(repo: &Repository) -> Option<String> {
    let remotes = list_remotes_git2(repo).ok()?;
    if remotes.is_empty() {
        return None;
    }

    if let Some(remote) = repo_config_remote_git2(repo, GUITAR_DEFAULT_REMOTE_CONFIG, &remotes) {
        return Some(remote);
    }

    if let Some(remote) = repo_config_remote_git2(repo, PUSH_DEFAULT_CONFIG, &remotes) {
        return Some(remote);
    }

    if let Some(remote) = current_branch_upstream_remote_git2(repo, &remotes) {
        return Some(remote);
    }

    if remote_exists(&remotes, "origin") {
        return Some("origin".to_string());
    }

    remotes.first().map(|remote| remote.name.clone())
}

#[cfg(test)]
fn repo_config_remote_git2(repo: &Repository, key: &str, remotes: &[RemoteEntry]) -> Option<String> {
    repo.config().ok().and_then(|config| config.get_string(key).ok()).map(|value| value.trim().to_string()).filter(|name| remote_exists(remotes, name))
}

#[cfg(test)]
fn current_branch_upstream_remote_git2(repo: &Repository, remotes: &[RemoteEntry]) -> Option<String> {
    let branch = crate::git::queries::commits::get_current_branch(repo)?;
    let refname = format!("refs/heads/{branch}");
    repo.branch_upstream_remote(&refname).ok().and_then(|remote| remote.as_str().map(str::to_string)).filter(|name| remote_exists(remotes, name))
}

#[cfg(test)]
#[path = "../../tests/git/queries/remotes.rs"]
mod tests;
