mod fixtures;

use divan::{Bencher, black_box, counter::ItemsCount};
use fixtures::{TempFixture, add_path, commit_file, temp_repo, write_text};
use git2::Repository;
use guitar::git::actions::worktrees::create_worktree;
use std::path::{Path, PathBuf};

fn main() {
    divan::main();
}

struct WorktreeFixture {
    repo: Repository,
    _temp: TempFixture,
    current_path: PathBuf,
    expected_entries: usize,
}

fn tracked_repo(name: &str, tracked_files: usize) -> (TempFixture, Repository) {
    let (path, repo) = temp_repo(name);
    commit_file(&repo, "README.md", "root\n", "root");
    for index in 0..tracked_files {
        commit_file(&repo, &format!("src/file-{index:03}.txt"), &format!("tracked {index}\n"), &format!("tracked {index}"));
    }
    (path, repo)
}

fn linked_worktree_path(path: &Path, name: &str) -> PathBuf {
    path.parent().unwrap_or_else(|| Path::new(".")).join(format!("{}-{name}", path.file_name().and_then(|name| name.to_str()).unwrap_or("repo")))
}

fn worktree_fixture(linked_count: usize, dirty_every: usize, tracked_files: usize) -> WorktreeFixture {
    let (path, repo) = tracked_repo("list", tracked_files);
    let head = repo.head().unwrap().target().unwrap();

    for index in 0..linked_count {
        let name = format!("wt-{index:03}");
        let worktree_path = linked_worktree_path(&path, &name);
        create_worktree(&repo, &name, &worktree_path, head).unwrap();

        if dirty_every != 0 && index % dirty_every == 0 {
            write_text(&worktree_path, &format!("dirty-{index}.txt"), "dirty\n");
        }
    }

    WorktreeFixture { current_path: path.to_path_buf(), _temp: path, repo, expected_entries: linked_count + 1 }
}

fn staged_worktree_fixture(linked_count: usize, tracked_files: usize) -> WorktreeFixture {
    let (path, repo) = tracked_repo("staged", tracked_files);
    let head = repo.head().unwrap().target().unwrap();

    for index in 0..linked_count {
        let name = format!("wt-{index:03}");
        let worktree_path = linked_worktree_path(&path, &name);
        create_worktree(&repo, &name, &worktree_path, head).unwrap();

        let staged_file = format!("staged-{index}.txt");
        write_text(&worktree_path, &staged_file, "staged\n");
        let linked_repo = Repository::open(&worktree_path).unwrap();
        add_path(&linked_repo, &staged_file);
    }

    WorktreeFixture { current_path: path.to_path_buf(), _temp: path, repo, expected_entries: linked_count + 1 }
}

fn list_worktrees(fixture: &WorktreeFixture) -> usize {
    guitar::git::queries::worktrees::list_worktrees(&fixture.repo, Some(&fixture.current_path)).unwrap().len()
}

#[divan::bench(sample_count = 40, sample_size = 10)]
fn list_worktrees_small(bencher: Bencher) {
    let fixture = worktree_fixture(4, 2, 1);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(list_worktrees(&fixture)));
}

#[divan::bench(sample_count = 30, sample_size = 10)]
fn list_worktrees_many(bencher: Bencher) {
    let fixture = worktree_fixture(16, 4, 1);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(list_worktrees(&fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn list_worktrees_many_tracked_files(bencher: Bencher) {
    let fixture = worktree_fixture(16, 4, 128);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(list_worktrees(&fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn list_worktrees_dirty_many_tracked_files(bencher: Bencher) {
    let fixture = worktree_fixture(16, 1, 128);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(list_worktrees(&fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn list_worktrees_staged_many_tracked_files(bencher: Bencher) {
    let fixture = staged_worktree_fixture(16, 128);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(list_worktrees(&fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 5)]
fn list_worktrees_clean_many_tracked_files(bencher: Bencher) {
    let fixture = worktree_fixture(16, 0, 128);
    bencher.counter(ItemsCount::new(fixture.expected_entries)).bench_local(|| black_box(list_worktrees(&fixture)));
}
