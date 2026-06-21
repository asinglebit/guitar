use super::*;
use git2::{IndexAddOption, Repository, Signature, Time};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-heatmap-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    (path, repo)
}

fn commit_at(repo: &Repository, path: &Path, name: &str, seconds: i64) -> Oid {
    fs::write(path.join(name), name).unwrap();

    let mut index = repo.index().unwrap();
    index.add_all(["."], IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::new("Test User", "test@example.com", &Time::new(seconds, 0)).unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, name, &tree, &parents).unwrap()
}

#[test]
fn commits_per_day_counts_rendered_dates_without_allocating_a_map() {
    let (path, repo) = temp_repo("counts");
    let today = Utc::now().timestamp();
    let outside_grid = today - ((TOTAL_DAYS as i64) + 10) * 24 * 60 * 60;
    let first = commit_at(&repo, &path, "first.txt", today);
    let second = commit_at(&repo, &path, "second.txt", today);
    let old = commit_at(&repo, &path, "old.txt", outside_grid);

    let counts = commits_per_day(&repo, &[first, second, old]);

    assert_eq!(counts[0], 2);
    assert_eq!(counts.iter().sum::<usize>(), 2);
}

#[test]
fn build_heatmap_places_today_in_the_newest_week() {
    let (path, repo) = temp_repo("grid");
    let oid = commit_at(&repo, &path, "today.txt", Utc::now().timestamp());
    let grid = build_heatmap(&repo, &[oid]);
    let weekday_today = Utc::now().weekday().num_days_from_monday() as usize;

    assert_eq!(grid[weekday_today][WEEKS - 1], 1);
}
