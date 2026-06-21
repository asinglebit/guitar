mod fixtures;

use divan::{Bencher, black_box};
use fixtures::{GraphServiceFixture, TempFixture, graph_service_fixture};
use git2::{IndexAddOption, Oid, Repository, Signature, Time};
use guitar::helpers::heatmap::{HeatmapCounts, build_heatmap};
use std::{fs, path::Path};

fn main() {
    divan::main();
}

struct HeatmapFixture {
    _graph_fixture: Option<GraphServiceFixture>,
    _temp_fixture: Option<TempFixture>,
    repo: gix::Repository,
    oids: Vec<Oid>,
    seconds: Vec<i64>,
}

fn heatmap_fixture(rounds: usize) -> HeatmapFixture {
    let fixture = graph_service_fixture(rounds);
    let git2_repo = Repository::open(&fixture.path).unwrap();
    let oids = collect_reachable_oids(&git2_repo);
    let seconds = collect_commit_seconds(&git2_repo, &oids);
    let repo = gix::open(&fixture.path).unwrap();

    HeatmapFixture { _graph_fixture: Some(fixture), _temp_fixture: None, repo, oids, seconds }
}

fn collect_reachable_oids(repo: &Repository) -> Vec<Oid> {
    let mut revwalk = repo.revwalk().unwrap();

    for reference in repo.references().unwrap().flatten() {
        if let Some(oid) = reference.target() {
            let _ = revwalk.push(oid);
        }
    }

    revwalk.filter_map(Result::ok).collect()
}

fn dated_heatmap_fixture(recent_commits: usize, old_commits: usize) -> HeatmapFixture {
    let fixture = fixtures::temp_dir("heatmap-dated");
    let repo = Repository::init(fixture.path()).unwrap();
    let now = chrono::Utc::now().timestamp();
    let old = now - 400 * 24 * 60 * 60;
    let mut recent_oids = Vec::with_capacity(recent_commits);
    let mut old_oids = Vec::with_capacity(old_commits);

    for index in 0..recent_commits {
        recent_oids.push(commit_at(&repo, fixture.path(), &format!("recent-{index}.txt"), now - index as i64 * 24 * 60 * 60));
    }

    for index in 0..old_commits {
        old_oids.push(commit_at(&repo, fixture.path(), &format!("old-{index}.txt"), old - index as i64 * 24 * 60 * 60));
    }

    let mut oids = recent_oids;
    oids.extend(old_oids);
    let seconds = collect_commit_seconds(&repo, &oids);
    let gix_repo = gix::open(fixture.path()).unwrap();

    HeatmapFixture { _graph_fixture: None, _temp_fixture: Some(fixture), repo: gix_repo, oids, seconds }
}

fn collect_commit_seconds(repo: &Repository, oids: &[Oid]) -> Vec<i64> {
    oids.iter().filter_map(|oid| repo.find_commit(*oid).ok().map(|commit| commit.time().seconds())).collect()
}

fn commit_at(repo: &Repository, root: &Path, file: &str, seconds: i64) -> Oid {
    fs::write(root.join(file), file).unwrap();

    let mut index = repo.index().unwrap();
    index.add_all(["."], IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::new("Benchmark Runner", "bench@example.com", &Time::new(seconds, 0)).unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, file, &tree, &parents).unwrap()
}

#[divan::bench(sample_count = 30, sample_size = 5)]
fn heatmap_build_medium(bencher: Bencher<'_, '_>) {
    let fixture = heatmap_fixture(256);
    let commits = fixture.oids.len() as u64;

    bencher.counter(divan::counter::ItemsCount::new(commits)).bench_local(|| black_box(build_heatmap(&fixture.repo, &fixture.oids)));
}

#[divan::bench(sample_count = 30, sample_size = 10)]
fn heatmap_build_with_old_tail(bencher: Bencher<'_, '_>) {
    let fixture = dated_heatmap_fixture(32, 512);
    let commits = fixture.oids.len() as u64;

    bencher.counter(divan::counter::ItemsCount::new(commits)).bench_local(|| black_box(build_heatmap(&fixture.repo, &fixture.oids)));
}

#[divan::bench(sample_count = 30, sample_size = 100)]
fn heatmap_build_from_streamed_counts(bencher: Bencher<'_, '_>) {
    let fixture = dated_heatmap_fixture(32, 512);
    let commits = fixture.seconds.len() as u64;

    bencher.counter(divan::counter::ItemsCount::new(commits)).bench_local(|| {
        let mut counts = HeatmapCounts::default();
        for seconds in &fixture.seconds {
            counts.add_commit_seconds(*seconds);
        }
        black_box(counts.build())
    });
}
