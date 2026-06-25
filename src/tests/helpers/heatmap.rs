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

    let scanned = build_heatmap(&gix_repo, [first_today, yesterday_oid].map(gix_oid));

    assert_eq!(counts[0], 2);
    assert_eq!(counts.iter().sum::<usize>(), 2);
    assert_eq!(stopped[0], 2);
    assert_eq!(stopped.iter().sum::<usize>(), 2);
    assert_eq!(grid[weekday_today][WEEKS - 1], 1);
    assert_eq!(scanned[weekday_today][WEEKS - 1], 1);
}

#[test]
fn heatmap_counts_match_streamed_helper() {
    let dir = tempfile::Builder::new().prefix("guitar-heatmap-stream-").tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let today = Utc::now().timestamp();
    let yesterday = today - 24 * 60 * 60;
    let first_today = commit_at(&repo, "first-today.txt", today);
    let yesterday_oid = commit_at(&repo, "yesterday.txt", yesterday);

    let gix_repo = gix::open(dir.path()).unwrap();
    let scanned = build_heatmap(&gix_repo, [first_today, yesterday_oid].map(gix_oid));
    let mut streamed = HeatmapCounts::default();
    streamed.add_commit_seconds(today);
    streamed.add_commit_seconds(yesterday);

    assert_eq!(streamed.build(), scanned);
}

#[test]
fn heatmap_counts_ignore_out_of_range_commits() {
    let mut counts = HeatmapCounts::default();
    let today = Utc::now().timestamp();
    let outside_grid = today - ((TOTAL_DAYS as i64) + 10) * 24 * 60 * 60;

    counts.add_commit_seconds(today);
    counts.add_commit_seconds(outside_grid);
    counts.add_commit_seconds(today + 24 * 60 * 60);

    assert_eq!(counts.build().iter().flatten().sum::<usize>(), 1);
}

#[test]
fn prefix_matching_accepts_partial_hex() {
    let dir = tempfile::Builder::new().prefix("guitar-prefix-").tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let oid = commit_at(&repo, "prefix.txt", Utc::now().timestamp());
    let gix_oid = crate::core::oids::git2_to_gix_oid(oid);
    let mut oids = crate::core::oids::Oids::default();

    let alias = oids.get_alias_by_oid(gix_oid);

    assert_eq!(oids.get_alias_by_prefix(&gix_oid.to_hex_with_len(7).to_string()), Some(alias));
    assert_eq!(oids.get_alias_by_prefix("not-hex"), None);
}

#[test]
fn timestamp_counter_skips_future_and_stops_after_window() {
    let today = NaiveDate::from_ymd_opt(2026, 6, 25).unwrap();
    let today_seconds = Utc.with_ymd_and_hms(2026, 6, 25, 12, 0, 0).unwrap().timestamp();
    let yesterday = today_seconds - 24 * 60 * 60;
    let future = today_seconds + 24 * 60 * 60;
    let outside_grid = today_seconds - ((TOTAL_DAYS as i64) + 1) * 24 * 60 * 60;

    let counts = counts_from_commit_seconds_for_day([today_seconds, future, yesterday, outside_grid, today_seconds], today);

    assert_eq!(counts[0], 1);
    assert_eq!(counts[1], 1);
    assert_eq!(counts.iter().sum::<usize>(), 2);
}
