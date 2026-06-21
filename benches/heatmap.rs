mod fixtures;

use divan::{Bencher, black_box};
use fixtures::{GraphServiceFixture, graph_service_fixture};
use git2::{Oid, Repository};
use guitar::helpers::heatmap::build_heatmap;

fn main() {
    divan::main();
}

struct HeatmapFixture {
    _fixture: GraphServiceFixture,
    repo: Repository,
    oids: Vec<Oid>,
}

fn heatmap_fixture(rounds: usize) -> HeatmapFixture {
    let fixture = graph_service_fixture(rounds);
    let repo = Repository::open(&fixture.path).unwrap();
    let oids = collect_reachable_oids(&repo);

    HeatmapFixture { _fixture: fixture, repo, oids }
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

#[divan::bench(sample_count = 30, sample_size = 5)]
fn heatmap_build_medium(bencher: Bencher<'_, '_>) {
    let fixture = heatmap_fixture(256);
    let commits = fixture.oids.len() as u64;

    bencher.counter(divan::counter::ItemsCount::new(commits)).bench_local(|| black_box(build_heatmap(&fixture.repo, &fixture.oids)));
}
