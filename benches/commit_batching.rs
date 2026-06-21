mod fixtures;

use divan::{Bencher, black_box};
use fixtures::graph_service_fixture;
use guitar::{
    core::{batcher::Batcher, oids::Oids, walker::Walker},
    git::queries::commits::get_sorted_oids,
};
use std::{cell::RefCell, rc::Rc};

fn main() {
    divan::main();
}

struct CommitBatchFixture {
    batcher: Batcher,
    _repo: Rc<RefCell<git2::Repository>>,
    scratch: Vec<git2::Oid>,
    amount: usize,
}

fn commit_batch_fixture(rounds: usize, amount: usize) -> CommitBatchFixture {
    let fixture = graph_service_fixture(rounds);
    let repo = Rc::new(RefCell::new(git2::Repository::open(&fixture.path).unwrap()));
    let batcher = Batcher::new(repo.clone(), &fixture.hidden_branch_names, &[]).unwrap();

    CommitBatchFixture { batcher, _repo: repo, scratch: Vec::with_capacity(amount), amount }
}

fn sorted_oid_pages(mut fixture: CommitBatchFixture) -> usize {
    let mut oids = Oids::default();
    let mut sorted = Vec::new();

    loop {
        let before = sorted.len();
        get_sorted_oids(&fixture.batcher, &mut oids, &mut sorted, fixture.amount, &mut fixture.scratch);
        if sorted.len() == before {
            break;
        }
    }

    black_box(sorted.len())
}

#[divan::bench(sample_count = 50, sample_size = 10)]
fn sorted_oid_pages_medium(bencher: Bencher) {
    let rounds = 24usize;
    let amount = 32usize;

    bencher.counter(divan::counter::ItemsCount::new(rounds.saturating_mul(4))).with_inputs(|| commit_batch_fixture(rounds, amount)).bench_local_values(|fixture| black_box(sorted_oid_pages(fixture)));
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
