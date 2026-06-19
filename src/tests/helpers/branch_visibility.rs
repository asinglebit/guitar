use super::*;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_config_path(name: &str) -> PathBuf {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("guitar-branch-visibility-{name}-{id}.json"))
}

fn hidden(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn sorted_hidden(path: &Path, repo_path: &str) -> Vec<String> {
    let mut names: Vec<String> = load_branch_visibility_from_path(path, repo_path).into_iter().collect();
    names.sort();
    names
}

#[test]
fn missing_branch_visibility_file_loads_empty_without_creating_file() {
    let path = temp_config_path("missing");

    assert!(load_branch_visibility_from_path(&path, "/repo/a").is_empty());
    assert!(!path.exists());
}

#[test]
fn branch_visibility_saves_and_loads_per_repository() {
    let path = temp_config_path("per-repo");

    save_branch_visibility_to_path(&path, "/repo/a", &hidden(&["main", "origin/old"]));
    save_branch_visibility_to_path(&path, "/repo/b", &hidden(&["topic"]));

    assert_eq!(sorted_hidden(&path, "/repo/a"), vec!["main", "origin/old"]);
    assert_eq!(sorted_hidden(&path, "/repo/b"), vec!["topic"]);
    assert!(load_branch_visibility_from_path(&path, "/repo/c").is_empty());
}

#[test]
fn branch_visibility_save_sorts_and_dedupes_hidden_names() {
    let path = temp_config_path("sort");

    save_branch_visibility_to_path(&path, "/repo/a", &hidden(&["zeta", "alpha", "alpha"]));

    let contents = fs::read_to_string(&path).unwrap();
    assert!(contents.contains('\n'), "{contents}");
    assert!(contents.contains("\n  \"repositories\""), "{contents}");
    assert!(contents.contains("\n      \"hidden\""), "{contents}");
    let config = facet_json::from_str::<BranchVisibilityConfig>(&contents).unwrap();
    assert_eq!(config.repositories[0].hidden, vec!["alpha", "zeta"]);
}

#[test]
fn prune_hidden_branches_removes_names_that_no_longer_exist() {
    let mut hidden_names = hidden(&["main", "origin/old", "topic"]);
    let current = hidden(&["main", "topic"]);

    assert!(prune_hidden_branches(&mut hidden_names, &current));
    assert_eq!(sorted_unique(hidden_names.into_iter().collect::<Vec<_>>()), vec!["main", "topic"]);
}

#[test]
fn branch_visibility_empty_hidden_set_removes_repository_entry() {
    let path = temp_config_path("empty");

    save_branch_visibility_to_path(&path, "/repo/a", &hidden(&["main"]));
    save_branch_visibility_to_path(&path, "/repo/a", &HashSet::new());

    let contents = fs::read_to_string(&path).unwrap();
    let config = facet_json::from_str::<BranchVisibilityConfig>(&contents).unwrap();
    assert!(config.repositories.is_empty());
}
