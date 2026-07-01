use facet::Facet;
use git2::Repository;
use im::HashSet;
use std::{
    fs,
    path::{Path, PathBuf},
    str,
};

const LOCAL_BRANCH_PREFIX: &[u8] = b"refs/heads/";
const REMOTE_BRANCH_PREFIX: &[u8] = b"refs/remotes/";

#[derive(Facet, Clone, Default)]
pub struct RepositoryBranchVisibility {
    pub path: String,
    #[facet(default)]
    pub hidden: Vec<String>,
}

#[derive(Facet, Clone, Default)]
pub struct BranchVisibilityConfig {
    #[facet(default)]
    pub repositories: Vec<RepositoryBranchVisibility>,
}

pub fn branch_visibility_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap();
    path.push("guitar");
    path.push("branch_visibility.json");
    path
}

pub fn current_branch_names(repo: &Repository) -> HashSet<String> {
    let mut names = HashSet::new();
    let Ok(references) = repo.references() else {
        return names;
    };

    for reference in references.flatten() {
        if reference.target().is_none() {
            continue;
        }

        let Some(name) = reference.name() else {
            continue;
        };

        let branch_name = name.strip_prefix("refs/heads/").or_else(|| name.strip_prefix("refs/remotes/"));
        if let Some(branch_name) = branch_name
            && !branch_name.is_empty()
        {
            names.insert(branch_name.to_string());
        }
    }

    names
}

pub(crate) fn branch_name_from_ref(name: &[u8]) -> Option<&str> {
    name.strip_prefix(LOCAL_BRANCH_PREFIX).or_else(|| name.strip_prefix(REMOTE_BRANCH_PREFIX)).filter(|branch_name| !branch_name.is_empty()).and_then(|branch_name| str::from_utf8(branch_name).ok())
}

pub fn prune_hidden_branches(hidden: &mut HashSet<String>, current: &HashSet<String>) -> bool {
    let pruned: HashSet<String> = hidden.iter().filter(|name| current.contains(*name)).cloned().collect();
    let changed = pruned.len() != hidden.len();
    *hidden = pruned;
    changed
}

pub fn load_branch_visibility(repo_path: &str) -> HashSet<String> {
    load_branch_visibility_from_path(&branch_visibility_config_path(), repo_path)
}

pub fn save_branch_visibility(repo_path: &str, hidden: &HashSet<String>) {
    save_branch_visibility_to_path(&branch_visibility_config_path(), repo_path, hidden);
}

pub fn load_branch_visibility_from_path(path: &Path, repo_path: &str) -> HashSet<String> {
    let config = load_config_from_path(path);
    config.repositories.into_iter().find(|entry| entry.path == repo_path).map(|entry| entry.hidden.into_iter().collect()).unwrap_or_default()
}

pub fn save_branch_visibility_to_path(path: &Path, repo_path: &str, hidden: &HashSet<String>) {
    let mut config = load_config_from_path(path);
    let hidden = sorted_unique(hidden.iter().cloned());

    config.repositories.retain(|entry| entry.path != repo_path);
    if !hidden.is_empty() {
        config.repositories.push(RepositoryBranchVisibility { path: repo_path.to_string(), hidden });
    }
    config.repositories.sort_by(|a, b| a.path.cmp(&b.path));

    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(config_string) = facet_json::to_string_pretty(&config) {
        let _ = fs::write(path, config_string);
    }
}

fn load_config_from_path(path: &Path) -> BranchVisibilityConfig {
    if !path.exists() {
        return BranchVisibilityConfig::default();
    }

    let Ok(contents) = fs::read_to_string(path) else {
        return BranchVisibilityConfig::default();
    };
    facet_json::from_str::<BranchVisibilityConfig>(&contents).unwrap_or_default()
}

fn sorted_unique(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut names: Vec<String> = names.into_iter().collect();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
#[path = "../tests/helpers/branch_visibility.rs"]
mod tests;
