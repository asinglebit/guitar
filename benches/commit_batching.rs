mod fixtures;

use divan::{Bencher, black_box};
use fixtures::{RepoWalkFixture, graph_service_fixture, repo_walk_hidden_branches_fixture, repo_walk_linear_fixture, repo_walk_many_refs_fixture, repo_walk_merge_fixture};
use guitar::{
    core::{batcher::Batcher, oids::Oids, walker::Walker},
    git::queries::commits::get_sorted_oids,
};
use std::{cell::RefCell, rc::Rc};

fn main() {
    divan::main();
}

struct CommitBatchFixture {
    _fixture: RepoWalkFixture,
    batcher: Batcher,
    _repo: Rc<RefCell<git2::Repository>>,
    amount: usize,
    expected_commits: usize,
}

fn commit_batch_fixture(fixture: RepoWalkFixture) -> CommitBatchFixture {
    let repo = Rc::new(RefCell::new(git2::Repository::open(&fixture.path).unwrap()));
    let batcher = Batcher::new(repo.clone(), &fixture.hidden_branch_names, &[]).unwrap();
    let amount = fixture.amount;
    let expected_commits = fixture.expected_commits;

    CommitBatchFixture { _fixture: fixture, batcher, _repo: repo, amount, expected_commits }
}

fn sorted_oid_pages(fixture: CommitBatchFixture) -> usize {
    let mut oids = Oids::default();
    let mut sorted = Vec::new();

    loop {
        let before = sorted.len();
        get_sorted_oids(&fixture.batcher, &mut oids, &mut sorted, fixture.amount);
        if sorted.len() == before {
            break;
        }
    }

    assert_eq!(sorted.len(), fixture.expected_commits);
    sorted.len()
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

#[divan::bench(sample_count = 50, sample_size = 10)]
fn sorted_oid_pages_medium(bencher: Bencher) {
    let rounds = 24usize;
    let amount = 32usize;

    bencher
        .counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4)))
        .with_inputs(|| commit_batch_fixture(repo_walk_merge_fixture(rounds, amount)))
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
