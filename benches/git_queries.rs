mod fixtures;

use fixtures::{TempFixture, add_path, commit_index, temp_repo, write_text};
use git2::Repository;
use std::fs;

fn main() {
    divan::main();
}

struct WorkdirStatusFixture {
    repo: Repository,
    _temp: TempFixture,
    expected_changes: usize,
}

fn build_workdir_status_fixture(tracked_count: usize, untracked_count: usize) -> WorkdirStatusFixture {
    let (workdir, repo) = temp_repo("workdir-status-many");

    for i in 0..tracked_count {
        let path = format!("src/file-{i:03}.txt");
        write_text(&workdir, &path, &format!("base {i}\n"));
        add_path(&repo, &path);
    }
    let _ = commit_index(&repo, "root snapshot");

    for i in 0..tracked_count {
        let path = format!("src/file-{i:03}.txt");
        if i % 3 == 0 {
            write_text(&workdir, &path, &format!("modified {i}\n"));
        } else if i % 3 == 1 {
            write_text(&workdir, &path, &format!("staged {i}\n"));
            add_path(&repo, &path);
        } else {
            fs::remove_file(workdir.join(&path)).unwrap();
        }
    }

    for i in 0..untracked_count {
        write_text(&workdir, &format!("scratch/new-{i:03}.txt"), &format!("new {i}\n"));
    }

    WorkdirStatusFixture { _temp: workdir, repo, expected_changes: tracked_count + untracked_count }
}

fn build_untracked_tree_fixture(dir_count: usize, files_per_dir: usize, ignored_per_dir: usize) -> WorkdirStatusFixture {
    let (workdir, repo) = temp_repo("workdir-untracked-tree");

    write_text(&workdir, ".gitignore", "*.ignored\n");
    add_path(&repo, ".gitignore");
    let _ = commit_index(&repo, "root snapshot");

    for dir in 0..dir_count {
        for file in 0..files_per_dir {
            write_text(&workdir, &format!("scratch/dir-{dir:03}/file-{file:03}.txt"), &format!("new {dir} {file}\n"));
        }
        for file in 0..ignored_per_dir {
            write_text(&workdir, &format!("scratch/dir-{dir:03}/ignored-{file:03}.ignored"), "ignored\n");
        }
    }

    WorkdirStatusFixture { _temp: workdir, repo, expected_changes: dir_count * files_per_dir }
}

fn workdir_status_many(fixture: &WorkdirStatusFixture) -> usize {
    let changes = guitar::git::queries::diffs::get_filenames_diff_at_workdir(&fixture.repo).unwrap();
    changes.staged.modified.len()
        + changes.staged.added.len()
        + changes.staged.deleted.len()
        + changes.unstaged.modified.len()
        + changes.unstaged.added.len()
        + changes.unstaged.deleted.len()
        + changes.conflicts.len()
}

#[divan::bench(sample_count = 30, sample_size = 10)]
fn get_filenames_diff_at_workdir_many_changes(bencher: divan::Bencher) {
    let fixture = build_workdir_status_fixture(96, 48);

    bencher.counter(divan::counter::ItemsCount::new(fixture.expected_changes)).bench_local(|| divan::black_box(workdir_status_many(&fixture)));
}

#[divan::bench(sample_count = 30, sample_size = 5)]
fn get_filenames_diff_at_workdir_untracked_tree(bencher: divan::Bencher) {
    let fixture = build_untracked_tree_fixture(8, 16, 4);

    bencher.counter(divan::counter::ItemsCount::new(fixture.expected_changes)).bench_local(|| divan::black_box(workdir_status_many(&fixture)));
}
