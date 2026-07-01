use crate::git::gix::gix_error;
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
    let path = path.as_ref();
    gix::open(path).or_else(|_| gix::open_opts(path, gix::open::Options::default().open_path_as_is(true))).map_err(gix_error)
}

fn list_remotes_from_repo(repo: &gix::Repository) -> Result<Vec<RemoteEntry>, git2::Error> {
    let mut entries: Vec<_> = repo
        .remote_names()
        .into_iter()
        .filter_map(|name| name.to_str().ok().map(str::to_string))
        .map(|name| {
            let remote = repo.find_remote(name.as_str()).map_err(gix_error)?;
            Ok(RemoteEntry { push_url: remote_push_url(repo, &name), name, url: remote.url(remote::Direction::Fetch).map(|url| url.to_string()).unwrap_or_default() })
        })
        .collect::<Result<_, git2::Error>>()?;

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
    let name = repo.config_snapshot().string(key)?;
    let name = name.to_str().ok()?.trim();
    remote_exists(remotes, name).then(|| name.to_string())
}

fn current_branch_upstream_remote(repo: &gix::Repository, remotes: &[RemoteEntry]) -> Option<String> {
    let head_name = repo.head_name().ok().flatten()?;
    let branch = head_name.shorten().to_str().ok()?;
    let key = format!("branch.{branch}.remote");
    repo_config_remote(repo, &key, remotes)
}

#[cfg(test)]
#[path = "../../tests/git/queries/remotes.rs"]
mod tests;
