use super::*;
use git2::{Repository, Signature};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = env::temp_dir().join(format!("guitar-files-{name}-{}-{suffix}", process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn init_repo(path: &Path) -> Repository {
    let repo = Repository::init(path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    repo
}

fn write_file(root: &Path, path: &str, content: &str) {
    let full_path = root.join(path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full_path, content).unwrap();
}

fn commit_files(repo: &Repository, files: &[&str], message: &str) {
    let mut index = repo.index().unwrap();
    for file in files {
        index.add_path(Path::new(file)).unwrap();
    }
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap();
}

fn result_paths(results: &[FileSearchResult]) -> Vec<String> {
    results.iter().map(|result| result.path.clone()).collect()
}

fn sorted(mut paths: Vec<String>) -> Vec<String> {
    paths.sort();
    paths
}

#[test]
fn committed_index_files_are_searchable() {
    let dir = TestDir::new("committed");
    let repo = init_repo(&dir.path);
    write_file(&dir.path, "src/app/draw/search.rs", "search\n");
    write_file(&dir.path, "README.md", "readme\n");
    commit_files(&repo, &["src/app/draw/search.rs", "README.md"], "initial");

    let results = search_tracked_files(&repo, "search", 10).unwrap();

    assert!(result_paths(&results).contains(&"src/app/draw/search.rs".to_string()));
    assert!(results.iter().find(|result| result.path == "src/app/draw/search.rs").unwrap().matched_indices.len() >= "search".len());
}

#[test]
fn staged_added_files_are_searchable_once_indexed() {
    let dir = TestDir::new("staged-added");
    let repo = init_repo(&dir.path);
    write_file(&dir.path, "README.md", "readme\n");
    commit_files(&repo, &["README.md"], "initial");

    write_file(&dir.path, "src/git/queries/files.rs", "files\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("src/git/queries/files.rs")).unwrap();
    index.write().unwrap();

    let results = search_tracked_files(&repo, "files", 10).unwrap();

    assert_eq!(result_paths(&results), vec!["src/git/queries/files.rs".to_string()]);
}

#[test]
fn untracked_and_deleted_files_are_excluded() {
    let dir = TestDir::new("excluded");
    let repo = init_repo(&dir.path);
    write_file(&dir.path, "kept.rs", "kept\n");
    write_file(&dir.path, "gone.rs", "gone\n");
    commit_files(&repo, &["kept.rs", "gone.rs"], "initial");

    fs::remove_file(dir.path.join("gone.rs")).unwrap();
    write_file(&dir.path, "target.rs", "untracked\n");
    write_file(&dir.path, ".gitignore", "*.log\n");
    write_file(&dir.path, "ignored.log", "ignored\n");

    assert_eq!(result_paths(&search_tracked_files(&repo, "kept", 10).unwrap()), vec!["kept.rs".to_string()]);
    assert!(search_tracked_files(&repo, "gone", 10).unwrap().is_empty());
    assert!(search_tracked_files(&repo, "target", 10).unwrap().is_empty());
    assert!(search_tracked_files(&repo, "ignored", 10).unwrap().is_empty());
}

#[test]
fn gitoxide_matches_libgit2_tracked_file_enumeration_for_search() {
    let dir = TestDir::new("parity");
    let repo = init_repo(&dir.path);
    write_file(&dir.path, "tracked.rs", "tracked\n");
    write_file(&dir.path, "kept.rs", "kept\n");
    commit_files(&repo, &["tracked.rs", "kept.rs"], "initial");

    write_file(&dir.path, "staged.rs", "staged\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("staged.rs")).unwrap();
    index.write().unwrap();

    fs::remove_file(dir.path.join("kept.rs")).unwrap();
    write_file(&dir.path, ".gitignore", "*.log\n");
    write_file(&dir.path, "ignored.log", "ignored\n");

    let gix_repo = gix::open(&dir.path).unwrap();
    let git2_paths = sorted(super::tracked_file_paths_git2(&repo).unwrap());
    let gix_paths = sorted(super::tracked_file_paths_gix(&gix_repo).unwrap());

    assert_eq!(git2_paths, gix_paths);
    assert_eq!(gix_paths, vec!["staged.rs".to_string(), "tracked.rs".to_string()]);
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
