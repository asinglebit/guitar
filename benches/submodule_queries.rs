mod fixtures;

use divan::{Bencher, black_box, counter::ItemsCount};
use fixtures::{TempFixture, add_path, commit_file, commit_index, temp_dir};
use git2::Repository;
use std::{fs, path::Path};

fn main() {
    divan::main();
}

struct SubmoduleFixture {
    repo: Repository,
    _temp: TempFixture,
    expected_entries: usize,
}

fn init_repo(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    let repo = Repository::init(path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Benchmark Runner").unwrap();
        config.set_str("user.email", "bench@example.com").unwrap();
    }
    commit_file(&repo, "file.txt", "root\n", "root");
    repo
}

fn parent_with_submodules(submodule_count: usize) -> (TempFixture, Repository) {
    let root = temp_dir("submodule-parent-root");
    let parent_path = root.join("parent");
    let parent = init_repo(&parent_path);

    for index in 0..submodule_count {
        let child_path = root.join(format!("child-{index:03}"));
        let child = init_repo(&child_path);
        drop(child);

        let path = format!("deps/child-{index:03}");
        let mut submodule = parent.submodule(child_path.to_str().unwrap(), Path::new(&path), true).unwrap();
        submodule.clone(None).unwrap();
        submodule.add_finalize().unwrap();
    }

    commit_index(&parent, "add submodules");
    (root, parent)
}

fn initialized_fixture(submodule_count: usize) -> SubmoduleFixture {
    let (_root, repo) = parent_with_submodules(submodule_count);
    SubmoduleFixture { _temp: _root, repo, expected_entries: submodule_count }
}

fn uninitialized_fixture(submodule_count: usize) -> SubmoduleFixture {
    let (root, parent) = parent_with_submodules(submodule_count);
    let clone_path = root.join("clone");
    let repo = Repository::clone(parent.workdir().unwrap().to_str().unwrap(), &clone_path).unwrap();
    SubmoduleFixture { _temp: root, repo, expected_entries: submodule_count }
}

fn no_submodules_fixture(tracked_count: usize) -> SubmoduleFixture {
    let root = temp_dir("submodule-none-root");
    let repo = init_repo(&root.join("repo"));

    for index in 0..tracked_count {
        let path = format!("src/file-{index:03}.txt");
        let full_path = repo.workdir().unwrap().join(&path);
        fs::create_dir_all(full_path.parent().unwrap()).unwrap();
        fs::write(full_path, format!("tracked {index}\n")).unwrap();
        add_path(&repo, &path);
    }

    if tracked_count > 0 {
        commit_index(&repo, "add tracked files");
    }

    SubmoduleFixture { _temp: root, repo, expected_entries: 0 }
}

fn submodule_count(fixture: &SubmoduleFixture) -> usize {
    guitar::git::queries::submodules::list_submodules(&fixture.repo).unwrap().len()
}

#[divan::bench(sample_count = 50, sample_size = 25)]
fn list_submodules_none(bencher: Bencher) {
    let fixture = no_submodules_fixture(0);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(submodule_count(&fixture)));
}

#[divan::bench(sample_count = 50, sample_size = 25)]
fn list_submodules_none_many_tracked_files(bencher: Bencher) {
    let fixture = no_submodules_fixture(256);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(submodule_count(&fixture)));
}

#[divan::bench(sample_count = 30, sample_size = 10)]
fn list_submodules_none_large_index(bencher: Bencher) {
    let fixture = no_submodules_fixture(4096);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(submodule_count(&fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn list_submodules_initialized_many(bencher: Bencher) {
    let fixture = initialized_fixture(16);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(submodule_count(&fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn list_submodules_uninitialized_many(bencher: Bencher) {
    let fixture = uninitialized_fixture(16);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(submodule_count(&fixture)));
}
