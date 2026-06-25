mod fixtures;

use divan::{Bencher, black_box};
use fixtures::{RepoWalkFixture, graph_service_fixture, repo_walk_hidden_branches_fixture, repo_walk_linear_fixture, repo_walk_many_refs_fixture, repo_walk_merge_fixture};
use guitar::{
    core::{
        batcher::{Batcher, WalkCommit},
        oids::Oids,
        walker::Walker,
    },
    git::queries::commits::get_sorted_oids,
};

type BenchResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

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

fn commit_batch_fixture(fixture: RepoWalkFixture) -> BenchResult<CommitBatchFixture> {
    let repo = gix::open(&fixture.path)?;
    let extra_roots: [gix::ObjectId; 0] = [];
    let batcher = Batcher::new(&repo, &fixture.hidden_branch_names, extra_roots)?;
    let amount = fixture.amount;
    let expected_commits = fixture.expected_commits;

    Ok(CommitBatchFixture { _fixture: fixture, batcher, _repo: repo, scratch: Vec::with_capacity(amount), amount, expected_commits })
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

fn walker_walk_pages(fixture: RepoWalkFixture, full_walk: bool) -> Result<usize, git2::Error> {
    let mut walker = Walker::new(fixture.path.display().to_string(), fixture.amount, fixture.hidden_branch_names, fixture.include_head_reflog_roots, fixture.graph_lane_limit)?;

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
    Ok(walked)
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn batcher_walk_linear_history(bencher: Bencher) {
    let commits = 256usize;
    let amount = 64usize;

    bencher
        .counter(divan::counter::ItemsCount::new(commits))
        .with_inputs(|| commit_batch_fixture(repo_walk_linear_fixture(commits, amount)).expect("linear benchmark fixture is valid"))
        .bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn batcher_walk_many_refs(bencher: Bencher) {
    let commits = 160usize;
    let refs = 96usize;
    let amount = 64usize;

    bencher
        .counter(divan::counter::ItemsCount::new(commits.saturating_add(refs)))
        .with_inputs(|| commit_batch_fixture(repo_walk_many_refs_fixture(commits, refs, amount)).expect("many-refs benchmark fixture is valid"))
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
        .with_inputs(|| commit_batch_fixture(repo_walk_hidden_branches_fixture(visible_commits, hidden_branches, hidden_commits, amount)).expect("hidden-branches benchmark fixture is valid"))
        .bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn walker_first_page_linear_history(bencher: Bencher) {
    let commits = 256usize;
    let amount = 64usize;

    bencher
        .counter(divan::counter::ItemsCount::new(amount))
        .with_inputs(|| repo_walk_linear_fixture(commits, amount))
        .bench_local_values(|fixture| black_box(walker_walk_pages(fixture, false).expect("linear benchmark walker is valid")));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn walker_full_walk_linear_history(bencher: Bencher) {
    let commits = 256usize;
    let amount = 64usize;

    bencher
        .counter(divan::counter::ItemsCount::new(commits))
        .with_inputs(|| repo_walk_linear_fixture(commits, amount))
        .bench_local_values(|fixture| black_box(walker_walk_pages(fixture, true).expect("linear benchmark walker is valid")));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn walker_full_walk_merge_heavy(bencher: Bencher) {
    let rounds = 16usize;
    let amount = 64usize;

    bencher
        .counter(divan::counter::ItemsCount::new(rounds.saturating_mul(3).saturating_add(1)))
        .with_inputs(|| repo_walk_merge_fixture(rounds, amount))
        .bench_local_values(|fixture| black_box(walker_walk_pages(fixture, true).expect("merge-heavy benchmark walker is valid")));
}

#[divan::bench(sample_count = 50, sample_size = 10)]
fn sorted_oid_pages_medium(bencher: Bencher) {
    let rounds = 24usize;
    let amount = 32usize;

    bencher
        .counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4)))
        .with_inputs(|| commit_batch_fixture(repo_walk_merge_fixture(rounds, amount)).expect("merge benchmark fixture is valid"))
        .bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
}

fn walk_all_pages(rounds: usize) -> Result<usize, git2::Error> {
    let fixture = graph_service_fixture(rounds);
    let mut walker = Walker::new(fixture.path.display().to_string(), fixture.amount, fixture.hidden_branch_names, fixture.include_head_reflog_roots, fixture.graph_lane_limit)?;

    while walker.walk() {}

    Ok(black_box(walker.oids.get_sorted_aliases().len()))
}

#[divan::bench(sample_count = 50, sample_size = 10)]
fn walker_walk_pages_medium(bencher: Bencher) {
    let rounds = 24usize;

    bencher.counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4))).bench(|| black_box(walk_all_pages(rounds).expect("graph-service benchmark walker is valid")));
}
