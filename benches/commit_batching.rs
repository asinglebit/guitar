mod fixtures;

use divan::{Bencher, black_box};
use fixtures::{
    RepoWalkFixture, graph_service_fixture, repo_walk_hidden_branches_fixture, repo_walk_linear_fixture, repo_walk_many_refs_fixture, repo_walk_many_tags_fixture, repo_walk_merge_fixture,
};
use guitar::{
    core::{
        batcher::{Batcher, WalkCommit},
        oids::Oids,
        walker::Walker,
    },
    git::{
        gix::enable_history_object_cache,
        queries::commits::{get_sorted_oids, get_stashed_commits_from_gix, get_tag_oids_from_gix},
    },
    helpers::layout::GRAPH_LANE_LIMIT_DEFAULT,
};
use std::{env, path::PathBuf, process::Command};

fn main() {
    divan::main();
}

struct CommitBatchFixture {
    _fixture: RepoWalkFixture,
    batcher: Batcher,
    _repo: gix::Repository,
    scratch: Vec<WalkCommit>,
    amount: usize,
    expected_commits: usize,
}

struct BatcherInitFixture {
    path: PathBuf,
    hidden_branch_names: im::HashSet<String>,
    _fixture: fixtures::GraphServiceFixture,
}

fn batcher_init_fixture(rounds: usize) -> BatcherInitFixture {
    let fixture = graph_service_fixture(rounds);
    BatcherInitFixture { path: fixture.path.clone(), hidden_branch_names: fixture.hidden_branch_names.clone(), _fixture: fixture }
}

fn commit_batch_fixture(fixture: RepoWalkFixture) -> CommitBatchFixture {
    let repo = gix::open(&fixture.path).unwrap();
    let amount = fixture.amount;
    let expected_commits = fixture.expected_commits;
    let batcher = Batcher::new(&repo, &fixture.hidden_branch_names, &[]).unwrap();

    CommitBatchFixture { _fixture: fixture, batcher, _repo: repo, scratch: Vec::with_capacity(amount), amount, expected_commits }
}

fn write_commit_graph(path: &PathBuf) {
    let status = Command::new("git").arg("-C").arg(path).args(["commit-graph", "write", "--reachable"]).status().unwrap();
    assert!(status.success());
}

fn commit_batch_fixture_with_commit_graph(fixture: RepoWalkFixture) -> CommitBatchFixture {
    write_commit_graph(&fixture.path);
    commit_batch_fixture(fixture)
}

fn sorted_oid_pages(mut fixture: CommitBatchFixture) -> usize {
    let mut oids = Oids::default();
    let mut sorted = Vec::new();

    loop {
        let before = sorted.len();
        get_sorted_oids(&mut fixture.batcher, &mut oids, &mut sorted, fixture.amount, &mut fixture.scratch);
        if sorted.len() == before {
            break;
        }
    }

    assert_eq!(sorted.len(), fixture.expected_commits);
    sorted.len()
}

fn initialize_batcher(fixture: BatcherInitFixture) -> usize {
    let repo = gix::open(&fixture.path).unwrap();
    let batcher = Batcher::new(&repo, &fixture.hidden_branch_names, &[]).unwrap();
    black_box(batcher.remaining())
}

fn walker_walk_pages(fixture: RepoWalkFixture, full_walk: bool) -> usize {
    let mut walker = Walker::new(fixture.path.display().to_string(), fixture.amount, fixture.hidden_branch_names, fixture.include_head_reflog_roots, fixture.graph_lane_limit).unwrap();

    if full_walk {
        while walker.walk() {}
    } else {
        let _ = walker.walk();
    }

    let walked = walker.oids.get_sorted_aliases().len();
    if full_walk {
        assert_eq!(walked, fixture.expected_walker_rows);
    } else {
        assert!(walked <= fixture.amount.saturating_add(1));
    }
    walked
}

fn walker_startup(fixture: RepoWalkFixture) -> usize {
    let walker = Walker::new(fixture.path.display().to_string(), fixture.amount, fixture.hidden_branch_names, fixture.include_head_reflog_roots, fixture.graph_lane_limit).unwrap();
    let refs = walker.branches_local.values().map(Vec::len).sum::<usize>() + walker.branches_remote.values().map(Vec::len).sum::<usize>() + walker.tags_local.values().map(Vec::len).sum::<usize>();
    black_box(refs)
}

fn collect_tag_oids(fixture: RepoWalkFixture) -> usize {
    let repo = gix::open(&fixture.path).unwrap();
    let mut oids = Oids::default();
    let tags = get_tag_oids_from_gix(&repo, &mut oids);
    black_box(tags.values().map(Vec::len).sum())
}

fn walker_walk_pages_with_commit_graph(fixture: RepoWalkFixture, full_walk: bool) -> usize {
    write_commit_graph(&fixture.path);
    walker_walk_pages(fixture, full_walk)
}

fn walk_external_repo(path_env: &str) -> usize {
    let Ok(path) = env::var(path_env) else {
        return 0;
    };
    let lane_limit = env::var("GUITAR_BENCH_LANE_LIMIT").ok().and_then(|value| value.parse().ok()).unwrap_or(GRAPH_LANE_LIMIT_DEFAULT);
    let mut walker = Walker::new(path, 10_000, im::HashSet::new(), false, lane_limit).unwrap();

    while walker.walk() {}

    black_box(walker.oids.get_sorted_aliases().len())
}

fn batch_external_repo(path_env: &str, include_tags: bool) -> usize {
    let Ok(path) = env::var(path_env) else {
        return 0;
    };
    let mut repo = gix::open(path).unwrap();
    enable_history_object_cache(&mut repo);

    let mut oids = Oids::default();
    let extra_roots = if include_tags {
        let tags = get_tag_oids_from_gix(&repo, &mut oids);
        let stashes = get_stashed_commits_from_gix(&repo, &mut oids);
        tags.keys().chain(stashes.iter()).map(|&alias| *oids.get_oid_by_alias(alias)).collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut batcher = Batcher::new(&repo, &im::HashSet::new(), &extra_roots).unwrap();
    let mut count = 0usize;
    let mut scratch = Vec::with_capacity(10_000);
    loop {
        scratch.clear();
        let read = batcher.next_into(10_000, &mut scratch);
        if read == 0 {
            break;
        }
        count += read;
    }

    black_box(count)
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn batcher_walk_linear_history(bencher: Bencher) {
    let commits = 256usize;
    let amount = 64usize;

    bencher
        .counter(divan::counter::ItemsCount::new(commits))
        .with_inputs(|| commit_batch_fixture(repo_walk_linear_fixture(commits, amount)))
        .bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn batcher_walk_many_refs(bencher: Bencher) {
    let commits = 160usize;
    let refs = 96usize;
    let amount = 64usize;

    bencher
        .counter(divan::counter::ItemsCount::new(commits.saturating_add(refs)))
        .with_inputs(|| commit_batch_fixture(repo_walk_many_refs_fixture(commits, refs, amount)))
        .bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn batcher_walk_hidden_branches(bencher: Bencher) {
    let visible_commits = 96usize;
    let hidden_branches = 24usize;
    let hidden_commits = 3usize;
    let amount = 64usize;

    bencher
        .counter(divan::counter::ItemsCount::new(visible_commits.saturating_add(hidden_branches.saturating_mul(hidden_commits))))
        .with_inputs(|| commit_batch_fixture(repo_walk_hidden_branches_fixture(visible_commits, hidden_branches, hidden_commits, amount)))
        .bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn walker_first_page_linear_history(bencher: Bencher) {
    let commits = 256usize;
    let amount = 64usize;

    bencher.counter(divan::counter::ItemsCount::new(amount)).with_inputs(|| repo_walk_linear_fixture(commits, amount)).bench_local_values(|fixture| black_box(walker_walk_pages(fixture, false)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn walker_full_walk_linear_history(bencher: Bencher) {
    let commits = 256usize;
    let amount = 64usize;

    bencher.counter(divan::counter::ItemsCount::new(commits)).with_inputs(|| repo_walk_linear_fixture(commits, amount)).bench_local_values(|fixture| black_box(walker_walk_pages(fixture, true)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn walker_full_walk_merge_heavy(bencher: Bencher) {
    let rounds = 16usize;
    let amount = 64usize;

    bencher
        .counter(divan::counter::ItemsCount::new(rounds.saturating_mul(3).saturating_add(1)))
        .with_inputs(|| repo_walk_merge_fixture(rounds, amount))
        .bench_local_values(|fixture| black_box(walker_walk_pages(fixture, true)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn walker_startup_many_tags(bencher: Bencher) {
    let commits = 256usize;
    let tags = 512usize;
    let amount = 64usize;

    bencher.counter(divan::counter::ItemsCount::new(tags)).with_inputs(|| repo_walk_many_tags_fixture(commits, tags, amount)).bench_local_values(|fixture| black_box(walker_startup(fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn tag_oid_collection_many_tags(bencher: Bencher) {
    let commits = 256usize;
    let tags = 512usize;
    let amount = 64usize;

    bencher.counter(divan::counter::ItemsCount::new(tags)).with_inputs(|| repo_walk_many_tags_fixture(commits, tags, amount)).bench_local_values(|fixture| black_box(collect_tag_oids(fixture)));
}

#[divan::bench(sample_count = 50, sample_size = 10)]
fn batcher_new_medium(bencher: Bencher) {
    let rounds = 24usize;

    bencher.counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4))).with_inputs(|| batcher_init_fixture(rounds)).bench_local_values(|fixture| black_box(initialize_batcher(fixture)));
}

#[divan::bench(sample_count = 50, sample_size = 10)]
fn sorted_oid_pages_medium(bencher: Bencher) {
    let rounds = 24usize;
    let amount = 32usize;

    bencher
        .counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4)))
        .with_inputs(|| commit_batch_fixture(repo_walk_merge_fixture(rounds, amount)))
        .bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn sorted_oid_pages_large(bencher: Bencher) {
    let rounds = 160usize;
    let amount = 256usize;

    bencher
        .counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4)))
        .with_inputs(|| commit_batch_fixture(repo_walk_merge_fixture(rounds, amount)))
        .bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn sorted_oid_pages_large_commit_graph(bencher: Bencher) {
    let rounds = 160usize;
    let amount = 256usize;

    bencher
        .counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4)))
        .with_inputs(|| commit_batch_fixture_with_commit_graph(repo_walk_merge_fixture(rounds, amount)))
        .bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
}

fn walk_all_pages(rounds: usize) -> usize {
    let fixture = graph_service_fixture(rounds);
    let mut walker = Walker::new(fixture.path.display().to_string(), fixture.amount, fixture.hidden_branch_names, fixture.include_head_reflog_roots, fixture.graph_lane_limit).unwrap();

    while walker.walk() {}

    black_box(walker.oids.get_sorted_aliases().len())
}

#[divan::bench(sample_count = 50, sample_size = 10)]
fn walker_walk_pages_medium(bencher: Bencher) {
    let rounds = 24usize;

    bencher.counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4))).bench(|| black_box(walk_all_pages(rounds)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn walker_walk_pages_large(bencher: Bencher) {
    let rounds = 160usize;

    bencher.counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4))).bench(|| black_box(walk_all_pages(rounds)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn walker_walk_pages_large_commit_graph(bencher: Bencher) {
    let rounds = 160usize;

    bencher.counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4))).bench(|| black_box(walker_walk_pages_with_commit_graph(repo_walk_merge_fixture(rounds, 32), true)));
}

#[divan::bench(sample_count = 1, sample_size = 1)]
fn walker_external_repo_full_walk(bencher: Bencher) {
    bencher.bench(|| black_box(walk_external_repo("GUITAR_BENCH_REPO")));
}

#[divan::bench(sample_count = 1, sample_size = 1)]
fn batcher_external_repo_branch_walk(bencher: Bencher) {
    bencher.bench(|| black_box(batch_external_repo("GUITAR_BENCH_REPO", false)));
}

#[divan::bench(sample_count = 1, sample_size = 1)]
fn batcher_external_repo_tag_walk(bencher: Bencher) {
    bencher.bench(|| black_box(batch_external_repo("GUITAR_BENCH_REPO", true)));
}
