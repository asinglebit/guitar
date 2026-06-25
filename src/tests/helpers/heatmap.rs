use super::*;
use crate::core::oids::git2_to_gix_oid as gix_oid;
use git2::{IndexAddOption, Oid, Repository, Signature, Time};
use std::fs;

fn commit_at(repo: &Repository, name: &str, seconds: i64) -> Oid {
    fs::write(repo.workdir().unwrap().join(name), name).unwrap();

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
fn heatmap_counts_repo_and_streamed_commits() {
    let dir = tempfile::Builder::new().prefix("guitar-heatmap-").tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let now = Utc::now();
    let today = now.timestamp();
    let yesterday = today - 24 * 60 * 60;
    let outside_grid = today - ((TOTAL_DAYS as i64) + 10) * 24 * 60 * 60;
    let first_today = commit_at(&repo, "first-today.txt", today);
    let second_today = commit_at(&repo, "second-today.txt", today);
    let yesterday_oid = commit_at(&repo, "yesterday.txt", yesterday);
    let old = commit_at(&repo, "old.txt", outside_grid);

    let gix_repo = gix::open(dir.path()).unwrap();
    let weekday_today = now.weekday().num_days_from_monday() as usize;
    let counts = commits_per_day(&gix_repo, [first_today, second_today, old].map(gix_oid));
    let stopped = commits_per_day(&gix_repo, [first_today, old, second_today].map(gix_oid));
    let grid = build_heatmap(&gix_repo, [gix_oid(first_today)]);

    assert_eq!(counts[0], 2);
    assert_eq!(counts.iter().sum::<usize>(), 2);
    assert_eq!(stopped[0], 2);
    assert_eq!(stopped.iter().sum::<usize>(), 2);
    assert_eq!(grid[weekday_today][WEEKS - 1], 1);

    let scanned = build_heatmap(&gix_repo, [first_today, yesterday_oid].map(gix_oid));
    let mut streamed = HeatmapCounts::default();
    streamed.add_commit_seconds(today);
    streamed.add_commit_seconds(yesterday);

    let mut filtered = HeatmapCounts::default();
    filtered.add_commit_seconds(today);
    filtered.add_commit_seconds(outside_grid);
    filtered.add_commit_seconds(today + 24 * 60 * 60);

    assert_eq!(streamed.build(), scanned);
    assert_eq!(filtered.build().iter().flatten().sum::<usize>(), 1);
}
