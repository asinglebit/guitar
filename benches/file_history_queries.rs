mod fixtures;

use fixtures::{TempFixture, add_path, commit_file, commit_index, temp_repo};
use git2::{Oid, Repository};
use std::{fs, path::Path};

fn main() {
    divan::main();
}

struct FileHistoryFixture {
    repo: Repository,
    _temp: TempFixture,
    path: String,
    oid: Oid,
}

fn rename_file(repo: &Repository, root: &Path, old: &str, new: &str, message: &str) -> Oid {
    fs::rename(root.join(old), root.join(new)).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new(old)).unwrap();
    index.write().unwrap();
    add_path(repo, new);
    commit_index(repo, message)
}

fn sparse_fixture(unrelated_commits: usize) -> FileHistoryFixture {
    let (path, repo) = temp_repo("sparse");
    let _ = commit_file(&repo, "tracked.txt", "root\n", "root");
    let mut latest = commit_file(&repo, "noise-0.txt", "noise 0\n", "noise 0");
    for i in 1..unrelated_commits {
        latest = commit_file(&repo, &format!("noise-{i}.txt"), &format!("noise {i}\n"), &format!("noise {i}"));
    }

    FileHistoryFixture { _temp: path, repo, path: "tracked.txt".to_string(), oid: latest }
}

fn frequent_fixture(commits: usize) -> FileHistoryFixture {
    let (path, repo) = temp_repo("frequent");
    let _ = commit_file(&repo, "tracked.txt", "root\n", "root");
    let mut latest = commit_file(&repo, "tracked.txt", "line 0\n", "update 0");

    for i in 1..commits {
        latest = commit_file(&repo, "tracked.txt", &format!("line {i}\n"), &format!("update {i}"));
    }

    FileHistoryFixture { _temp: path, repo, path: "tracked.txt".to_string(), oid: latest }
}

fn rename_fixture(unrelated_commits: usize) -> FileHistoryFixture {
    let (path, repo) = temp_repo("rename");
    let _ = commit_file(&repo, "old.txt", "root\n", "root");
    let _ = commit_file(&repo, "noise-0.txt", "noise 0\n", "noise 0");
    for i in 1..unrelated_commits {
        let _ = commit_file(&repo, &format!("noise-{i}.txt"), &format!("noise {i}\n"), &format!("noise {i}"));
    }

    let renamed = rename_file(&repo, &path, "old.txt", "new.txt", "rename");

    FileHistoryFixture { _temp: path, repo, path: "old.txt".to_string(), oid: renamed }
}

fn changed_file_status(fixture: &FileHistoryFixture) -> Option<guitar::git::queries::helpers::FileStatus> {
    guitar::git::queries::file_history::changed_file_status_at_commit(&fixture.repo, fixture.oid, &fixture.path).unwrap()
}

#[divan::bench(sample_count = 75, sample_size = 25)]
fn changed_file_status_sparse_small(bencher: divan::Bencher) {
    let fixture = sparse_fixture(16);

    bencher.counter(divan::counter::ItemsCount::new(1usize)).bench_local(|| divan::black_box(changed_file_status(&fixture)));
}

#[divan::bench(sample_count = 75, sample_size = 25)]
fn changed_file_status_frequent_medium(bencher: divan::Bencher) {
    let fixture = frequent_fixture(48);

    bencher.counter(divan::counter::ItemsCount::new(1usize)).bench_local(|| divan::black_box(changed_file_status(&fixture)));
}

#[divan::bench(sample_count = 75, sample_size = 25)]
fn changed_file_status_rename_stress(bencher: divan::Bencher) {
    let fixture = rename_fixture(96);

    bencher.counter(divan::counter::ItemsCount::new(1usize)).bench_local(|| divan::black_box(changed_file_status(&fixture)));
}
