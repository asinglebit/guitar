mod fixtures;

use divan::{Bencher, black_box, counter::ItemsCount};
use fixtures::{TempFixture, add_path, commit_file, temp_repo, write_text};
use guitar::{App, core::graph_service::GraphCommand, git::actions::worktrees::create_worktree};
use im::HashSet;
use std::{
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use tempfile::NamedTempFile;

fn main() {
    divan::main();
}

struct StartupFixture {
    _temp: TempFixture,
    path: PathBuf,
    recent_path: PathBuf,
    expected_worktrees: usize,
    expected_dirty_files: usize,
}

fn startup_fixture(commits: usize, linked_worktrees: usize, dirty_files: usize) -> StartupFixture {
    let (path, repo) = temp_repo("startup");
    commit_file(&repo, "README.md", "root\n", "root");

    for index in 1..commits {
        commit_file(&repo, &format!("src/file-{index:03}.txt"), &format!("tracked {index}\n"), &format!("tracked {index}"));
    }

    let head = repo.head().unwrap().target().unwrap();
    for index in 0..linked_worktrees {
        let name = format!("wt-{index:03}");
        let worktree_path = path.parent().unwrap_or_else(|| Path::new(".")).join(format!("{}-{name}", path.file_name().and_then(|name| name.to_str()).unwrap_or("repo")));
        create_worktree(&repo, &name, &worktree_path, head).unwrap();
    }

    for index in 0..dirty_files {
        let file = format!("dirty/file-{index:03}.txt");
        write_text(&path, &file, &format!("dirty {index}\n"));
        if index % 2 == 0 {
            add_path(&repo, &file);
        }
    }

    let path_buf = path.to_path_buf();
    let recent_path = path_buf.join(".bench-config").join("recent.json");
    StartupFixture { _temp: path, path: path_buf, recent_path, expected_worktrees: linked_worktrees + 1, expected_dirty_files: dirty_files }
}

fn reload_app(fixture: &StartupFixture) -> App {
    let mut app = App { recent_save_path: Some(fixture.recent_path.clone()), ..Default::default() };
    app.branches.hidden_branch_names = HashSet::new();
    app.reload(Some(fixture.path.display().to_string()));
    app
}

fn shutdown_app(app: &mut App) {
    if let Some(cancel) = &app.walker_cancel {
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    if let Some(tx) = app.graph_tx.take() {
        let _ = tx.send(GraphCommand::Shutdown);
    }
    if let Some(handle) = app.walker_handle.take() {
        let _ = handle.join();
    }
}

fn reload_startup_components(fixture: StartupFixture) -> usize {
    let mut app = reload_app(&fixture);
    let loaded = app.repo.is_some() as usize + app.worktrees.entries.len() + app.submodules.entries.len() + app.recent.len() + app.graph_tx.is_some() as usize;

    assert!(app.repo.is_some());
    assert_eq!(app.worktrees.entries.len(), fixture.expected_worktrees);
    shutdown_app(&mut app);
    loaded
}

fn reload_until_uncommitted_metadata(fixture: StartupFixture) -> usize {
    let mut app = reload_app(&fixture);
    let repo = app.repo.clone().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);

    while !app.is_uncommitted_loaded && Instant::now() < deadline {
        app.sync(&repo);
        thread::sleep(Duration::from_millis(1));
    }

    assert!(app.is_uncommitted_loaded);
    assert!(app.uncommitted.staged.added.len() + app.uncommitted.unstaged.added.len() >= fixture.expected_dirty_files);
    let loaded = app.worktrees.entries.len() + app.uncommitted.staged.added.len() + app.uncommitted.unstaged.added.len();
    shutdown_app(&mut app);
    loaded
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn app_reload_startup_components(bencher: Bencher) {
    let commits = 96usize;
    let linked_worktrees = 8usize;
    let dirty_files = 24usize;

    bencher
        .counter(ItemsCount::new(commits.saturating_add(linked_worktrees).saturating_add(dirty_files)))
        .with_inputs(|| startup_fixture(commits, linked_worktrees, dirty_files))
        .bench_local_values(|fixture| black_box(reload_startup_components(fixture)));
}

#[divan::bench(sample_count = 20, sample_size = 1)]
fn app_reload_until_uncommitted_metadata(bencher: Bencher) {
    let commits = 96usize;
    let linked_worktrees = 8usize;
    let dirty_files = 24usize;

    bencher
        .counter(ItemsCount::new(commits.saturating_add(linked_worktrees).saturating_add(dirty_files)))
        .with_inputs(|| startup_fixture(commits, linked_worktrees, dirty_files))
        .bench_local_values(|fixture| black_box(reload_until_uncommitted_metadata(fixture)));
}

#[divan::bench(sample_count = 50, sample_size = 10)]
fn app_default_state(bencher: Bencher) {
    bencher.bench_local(|| black_box(App::default()));
}

#[divan::bench(sample_count = 50, sample_size = 10)]
fn load_branch_visibility_for_startup_repo(bencher: Bencher) {
    let config = NamedTempFile::new().unwrap();
    let config_path = config.path();
    let repo_path = "/tmp/guitar/startup";
    let hidden = ["main".to_string(), "origin/slow".to_string()].into_iter().collect();
    guitar::helpers::branch_visibility::save_branch_visibility_to_path(config_path, repo_path, &hidden);

    bencher.bench_local(|| black_box(guitar::helpers::branch_visibility::load_branch_visibility_from_path(config_path, repo_path)));
}

#[divan::bench(sample_count = 50, sample_size = 10)]
fn load_symbol_theme_for_startup(bencher: Bencher) {
    let config = NamedTempFile::new().unwrap();
    let config_path = config.path();
    guitar::helpers::symbols::save_symbol_theme_to_path(config_path, &guitar::helpers::symbols::SymbolTheme::ascii());

    bencher.bench_local(|| black_box(guitar::helpers::symbols::load_symbol_theme_from_path(config_path)));
}
