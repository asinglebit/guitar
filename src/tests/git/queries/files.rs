use super::*;
use crate::git::test_support::{TestDir, commit_named_files as commit_files, init_repo_at};
use std::{fs, path::Path};

fn write_file(root: &Path, path: &str, content: &str) {
    let full_path = root.join(path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full_path, content).unwrap();
}

fn result_paths(results: &[FileSearchResult]) -> Vec<String> {
    results.iter().map(|result| result.path.clone()).collect()
}

#[test]
fn committed_index_files_are_searchable() {
    let dir = TestDir::new("committed");
    let repo = init_repo_at(dir.path());
    write_file(dir.path(), "src/app/draw/search.rs", "search\n");
    write_file(dir.path(), "README.md", "readme\n");
    commit_files(&repo, &["src/app/draw/search.rs", "README.md"], "initial");

    let results = search_tracked_files(&repo, "search", 10).unwrap();

    assert!(result_paths(&results).contains(&"src/app/draw/search.rs".to_string()));
    assert!(results.iter().find(|result| result.path == "src/app/draw/search.rs").unwrap().matched_indices.len() >= "search".len());
}

#[test]
fn staged_added_files_are_searchable_once_indexed() {
    let dir = TestDir::new("staged-added");
    let repo = init_repo_at(dir.path());
    write_file(dir.path(), "README.md", "readme\n");
    commit_files(&repo, &["README.md"], "initial");

    write_file(dir.path(), "src/git/queries/files.rs", "files\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("src/git/queries/files.rs")).unwrap();
    index.write().unwrap();

    let results = search_tracked_files(&repo, "files", 10).unwrap();

    assert_eq!(result_paths(&results), vec!["src/git/queries/files.rs".to_string()]);
}

#[test]
fn untracked_and_deleted_files_are_excluded() {
    let dir = TestDir::new("excluded");
    let repo = init_repo_at(dir.path());
    write_file(dir.path(), "kept.rs", "kept\n");
    write_file(dir.path(), "gone.rs", "gone\n");
    commit_files(&repo, &["kept.rs", "gone.rs"], "initial");

    fs::remove_file(dir.join("gone.rs")).unwrap();
    write_file(dir.path(), "target.rs", "untracked\n");
    write_file(dir.path(), ".gitignore", "*.log\n");
    write_file(dir.path(), "ignored.log", "ignored\n");

    assert_eq!(result_paths(&search_tracked_files(&repo, "kept", 10).unwrap()), vec!["kept.rs".to_string()]);
    assert!(search_tracked_files(&repo, "gone", 10).unwrap().is_empty());
    assert!(search_tracked_files(&repo, "target", 10).unwrap().is_empty());
    assert!(search_tracked_files(&repo, "ignored", 10).unwrap().is_empty());
}

#[test]
fn tracked_file_enumeration_includes_staged_files_and_excludes_deleted_files() {
    let dir = TestDir::new("tracked-enumeration");
    let repo = init_repo_at(dir.path());
    write_file(dir.path(), "tracked.rs", "tracked\n");
    write_file(dir.path(), "kept.rs", "kept\n");
    commit_files(&repo, &["tracked.rs", "kept.rs"], "initial");

    write_file(dir.path(), "staged.rs", "staged\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("staged.rs")).unwrap();
    index.write().unwrap();

    fs::remove_file(dir.join("kept.rs")).unwrap();
    write_file(dir.path(), ".gitignore", "*.log\n");
    write_file(dir.path(), "ignored.log", "ignored\n");

    let mut paths = result_paths(&search_tracked_files(&repo, ".rs", 10).unwrap());
    paths.sort();

    assert_eq!(paths, vec!["staged.rs".to_string(), "tracked.rs".to_string()]);
}

#[test]
fn empty_query_and_zero_limit_return_empty_results() {
    let paths = vec!["src/app/draw/search.rs".to_string()];

    assert!(rank_file_paths(&paths, "   ", 10).is_empty());
    assert!(rank_file_paths(&paths, "search", 0).is_empty());
}

#[test]
fn case_insensitive_and_backslash_queries_work() {
    let file_paths = vec!["src/app/draw/search.rs".to_string()];
    let results = rank_file_paths(&file_paths, "SRC\\APP search", 10);

    assert_eq!(result_paths(&results), vec!["src/app/draw/search.rs".to_string()]);
}

#[test]
fn basename_matches_outrank_weaker_path_matches() {
    let file_paths = vec!["src/file_history.rs".to_string(), "src/git/file_history.rs.bak".to_string()];
    let results = rank_file_paths(&file_paths, "file_history.rs", 10);

    assert_eq!(results[0].path, "src/file_history.rs");
    assert!(results[0].score > results[1].score);
}

#[test]
fn multi_term_queries_require_all_terms() {
    let file_paths = vec!["src/app/draw/search.rs".to_string(), "src/app/draw/status.rs".to_string(), "src/git/queries/search.rs".to_string()];
    let results = rank_file_paths(&file_paths, "draw search", 10);

    assert_eq!(result_paths(&results), vec!["src/app/draw/search.rs".to_string()]);
}

#[test]
fn fuzzy_subsequence_queries_return_valid_match_indices() {
    let file_paths = vec!["src/git/queries/files.rs".to_string()];
    let results = rank_file_paths(&file_paths, "gqf", 10);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, "src/git/queries/files.rs");
    assert_eq!(results[0].matched_indices, vec![4, 8, 16]);
}

#[test]
fn result_limit_and_tie_ordering_are_stable() {
    let file_paths = vec!["b.rs".to_string(), "a.rs".to_string(), "c.rs".to_string()];
    let results = rank_file_paths(&file_paths, "rs", 2);

    assert_eq!(result_paths(&results), vec!["a.rs".to_string(), "b.rs".to_string()]);
}
