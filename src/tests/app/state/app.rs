use super::*;
use crate::core::graph_service::{GraphCommand, GraphEvent, GraphFileHistoryRow, GraphLookupKind, GraphLookupResult, GraphPane, GraphRow};
use crate::git::queries::helpers::FileStatus;
use git2::{Repository, Signature};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    rc::Rc,
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-app-state-{name}-{}-{suffix}", process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-app-state-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn commit_file(repo: &Repository, file: &str, message: &str) -> git2::Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), format!("{message}\n")).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    commit_index(repo, message)
}

fn commit_index(repo: &Repository, message: &str) -> git2::Oid {
    let mut index = repo.index().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}

fn init_repo_at(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    let repo = Repository::init(path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    commit_file(&repo, "file.txt", "initial");
    repo
}

fn parent_with_submodule(dir: &TestDir) -> Repository {
    let child_path = dir.path.join("child");
    let parent_path = dir.path.join("parent");
    let child = init_repo_at(&child_path);
    drop(child);
    let parent = init_repo_at(&parent_path);
    let mut submodule = parent.submodule(child_path.to_str().unwrap(), Path::new("deps/child"), true).unwrap();
    submodule.clone(None).unwrap();
    submodule.add_finalize().unwrap();
    commit_index(&parent, "add submodule");
    drop(submodule);
    parent
}

fn graph_row(index: usize, alias: u32, oid: git2::Oid) -> GraphRow {
    GraphRow {
        index,
        alias,
        oid,
        summary: "commit".to_string(),
        committer_date: String::new(),
        committer_name: String::new(),
        is_merge: false,
        has_any_branch: false,
        branches: Vec::new(),
        tags: Vec::new(),
        is_stash: false,
        stash_lane: None,
        worktrees: Vec::new(),
        reflog: None,
    }
}

fn history_row(index: usize, oid: git2::Oid) -> GraphFileHistoryRow {
    GraphFileHistoryRow { graph_index: index, oid, short_oid: oid.to_string()[..8].to_string(), summary: "history".to_string(), status: FileStatus::Modified }
}

fn stop_graph_service(app: &mut App) {
    if let Some(tx) = app.graph_tx.take() {
        let _ = tx.send(GraphCommand::Shutdown);
    }
    if let Some(cancel) = app.walker_cancel.take() {
        cancel.store(true, Ordering::SeqCst);
    }
    if let Some(handle) = app.walker_handle.take() {
        let _ = handle.join();
    }
}

#[test]
fn default_splash_draw_has_no_reset_backgrounds() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::default();

    terminal.draw(|frame| app.draw(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    assert!(buffer.content().iter().all(|cell| cell.bg != Color::Reset));
}

#[test]
fn splash_draws_recent_repository_actions() {
    let backend = TestBackend::new(140, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App { viewport: Viewport::Splash, focus: Focus::Viewport, recent: vec!["/repo/a".into(), "/repo/b".into()], ..Default::default() };
    app.layout.app = Rect::new(0, 0, 140, 24);
    app.layout.graph = Rect::new(0, 0, 140, 24);

    terminal.draw(|frame| app.draw_splash(frame)).unwrap();

    let rendered = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
    assert!(rendered.contains("recent repositories:"));
    assert!(rendered.contains("actions: remove (d) | move up (Shift + K) | move down (Shift + J)"));
    assert!(rendered.contains("/repo/a"));
    assert!(rendered.contains("/repo/b"));
}

#[test]
fn reload_captures_selected_commit_oid_and_visual_offset_for_restore() {
    let (path, repo) = temp_repo("restore-capture");
    let oid = commit_file(&repo, "selected.txt", "selected");
    let path_string = path.display().to_string();
    let mut app =
        App { path: Some(path_string.clone()), recent: vec![path_string], repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 4, ..Default::default() };
    app.graph_scroll.set(2);
    app.graph.graph_window = Some(GraphWindowCache { version: 1, start: 4, end: 5, head_alias: 9, rows: vec![graph_row(4, 9, oid)], history: Default::default(), is_stale: false });

    app.reload(None);

    assert_eq!(app.graph.pending_selection_restore, Some(GraphSelectionRestore { oid, selected_offset: 2 }));
    stop_graph_service(&mut app);
}

#[test]
fn reload_keeps_uncommitted_row_without_restore_lookup() {
    let (path, repo) = temp_repo("restore-uncommitted");
    commit_file(&repo, "head.txt", "head");
    let path_string = path.display().to_string();
    let mut app =
        App { path: Some(path_string.clone()), recent: vec![path_string], repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 0, ..Default::default() };

    app.reload(None);

    assert_eq!(app.graph_selected, 0);
    assert_eq!(app.graph.pending_selection_restore, None);
    stop_graph_service(&mut app);
}

#[test]
fn pending_restore_requests_oid_lookup_on_progress() {
    let (_path, repo) = temp_repo("restore-progress");
    let oid = commit_file(&repo, "selected.txt", "selected");
    let repo = Rc::new(repo);
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(repo.clone()), graph_tx: Some(cmd_tx), graph_rx: Some(event_rx), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    app.graph.generation = 7;
    app.graph.pending_selection_restore = Some(GraphSelectionRestore { oid, selected_offset: 2 });

    event_tx.send(GraphEvent::Progress { generation: 7, version: 1, total: 2, is_first: false, is_complete: false }).unwrap();
    app.sync(&repo);

    match cmd_rx.try_recv().unwrap() {
        GraphCommand::Lookup { generation, request_id, kind: GraphLookupKind::Oid { oid: actual_oid } } => {
            assert_eq!(generation, 7);
            assert_eq!(request_id, 1);
            assert_eq!(actual_oid, oid);
        },
        other => panic!("expected oid restore lookup, got {other:?}"),
    }

    let (pending_id, pending_action) = app.graph.pending_lookup.unwrap();
    assert_eq!(pending_id, 1);
    assert!(matches!(pending_action, PendingGraphLookup::RestoreSelection));
}

#[test]
fn first_graph_progress_with_dirty_submodule_status_stays_in_graph_view() {
    let dir = TestDir::new("dirty-submodule-progress");
    let parent = parent_with_submodule(&dir);
    fs::write(parent.workdir().unwrap().join("deps/child/file.txt"), "dirty\n").unwrap();
    let repo = Rc::new(parent);
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(repo.clone()), graph_rx: Some(event_rx), viewport: Viewport::Splash, focus: Focus::Viewport, ..Default::default() };
    app.graph.generation = 9;

    event_tx.send(GraphEvent::Progress { generation: 9, version: 1, total: 1, is_first: true, is_complete: false }).unwrap();
    app.sync(&repo);

    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.is_uncommitted_loaded);
    assert!(app.uncommitted.is_clean);
}

#[test]
fn restore_lookup_success_selects_index_and_preserves_visual_offset() {
    let (_path, repo) = temp_repo("restore-success");
    let oid = commit_file(&repo, "selected.txt", "selected");
    let repo = Rc::new(repo);
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(repo.clone()), graph_rx: Some(event_rx), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    app.graph.generation = 7;
    app.graph.total = 10;
    app.graph.pending_selection_restore = Some(GraphSelectionRestore { oid, selected_offset: 2 });
    app.graph.pending_lookup = Some((3, PendingGraphLookup::RestoreSelection));

    event_tx.send(GraphEvent::LookupResult { generation: 7, request_id: 3, result: GraphLookupResult::Index(Some(4)) }).unwrap();
    app.sync(&repo);

    assert_eq!(app.graph_selected, 4);
    assert_eq!(app.graph_scroll.get(), 2);
    assert_eq!(app.graph.pending_selection_restore, None);
}

#[test]
fn restore_lookup_success_clamps_scroll_offset_near_graph_top() {
    let (_path, repo) = temp_repo("restore-top");
    let oid = commit_file(&repo, "selected.txt", "selected");
    let repo = Rc::new(repo);
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(repo.clone()), graph_rx: Some(event_rx), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 6, ..Default::default() };
    app.graph.generation = 7;
    app.graph.total = 10;
    app.graph.pending_selection_restore = Some(GraphSelectionRestore { oid, selected_offset: 4 });
    app.graph.pending_lookup = Some((3, PendingGraphLookup::RestoreSelection));

    event_tx.send(GraphEvent::LookupResult { generation: 7, request_id: 3, result: GraphLookupResult::Index(Some(1)) }).unwrap();
    app.sync(&repo);

    assert_eq!(app.graph_selected, 1);
    assert_eq!(app.graph_scroll.get(), 0);
    assert_eq!(app.graph.pending_selection_restore, None);
}

#[test]
fn restore_lookup_missing_after_completion_clears_pending_restore() {
    let (_path, repo) = temp_repo("restore-missing");
    let oid = commit_file(&repo, "selected.txt", "selected");
    let repo = Rc::new(repo);
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(repo.clone()), graph_rx: Some(event_rx), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 2, ..Default::default() };
    app.graph.generation = 7;
    app.graph.total = 6;
    app.graph.is_complete = true;
    app.graph.pending_selection_restore = Some(GraphSelectionRestore { oid, selected_offset: 2 });
    app.graph.pending_lookup = Some((3, PendingGraphLookup::RestoreSelection));

    event_tx.send(GraphEvent::LookupResult { generation: 7, request_id: 3, result: GraphLookupResult::Index(None) }).unwrap();
    app.sync(&repo);

    assert_eq!(app.graph_selected, 2);
    assert_eq!(app.graph.pending_selection_restore, None);
}

#[test]
fn explicit_graph_navigation_clears_pending_restore() {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    app.graph.total = 5;
    app.graph.pending_selection_restore = Some(GraphSelectionRestore { oid: git2::Oid::zero(), selected_offset: 0 });

    app.on_scroll_down();

    assert_eq!(app.graph_selected, 2);
    assert_eq!(app.graph.pending_selection_restore, None);
}

#[test]
fn file_history_event_updates_only_matching_request() {
    let (_path, repo) = temp_repo("file-history-event");
    let oid = commit_file(&repo, "target.txt", "target");
    let repo = Rc::new(repo);
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut app = App {
        repo: Some(repo.clone()),
        graph_rx: Some(event_rx),
        viewport: Viewport::Graph,
        focus: Focus::Search,
        search_path: Some("target.txt".to_string()),
        search_request_id: Some(3),
        search_is_loading: true,
        ..Default::default()
    };
    app.graph.generation = 7;

    event_tx.send(GraphEvent::FileHistory { generation: 7, request_id: 2, path: "target.txt".to_string(), rows: vec![history_row(1, oid)], error: None }).unwrap();
    app.sync(&repo);

    assert!(app.search_is_loading);
    assert!(app.search_rows.is_empty());
    assert_eq!(app.search_request_id, Some(3));

    event_tx.send(GraphEvent::FileHistory { generation: 7, request_id: 3, path: "target.txt".to_string(), rows: vec![history_row(1, oid)], error: None }).unwrap();
    app.sync(&repo);

    assert!(!app.search_is_loading);
    assert_eq!(app.search_request_id, None);
    assert_eq!(app.search_rows.len(), 1);
    assert_eq!(app.search_rows[0].graph_index, 1);
}

#[test]
fn graph_window_request_reuses_cached_window_that_covers_range() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App { graph_tx: Some(tx), ..Default::default() };
    app.graph.generation = 7;
    app.graph.version = 2;
    app.graph.graph_window = Some(GraphWindowCache { version: 2, start: 0, end: 10, head_alias: 1, rows: Vec::new(), history: Default::default(), is_stale: false });

    app.request_graph_window(2, 8);

    assert!(rx.try_recv().is_err());

    app.request_graph_window(0, 11);

    match rx.try_recv().unwrap() {
        GraphCommand::QueryGraphWindow { generation, request_id, start, end } => {
            assert_eq!(generation, 7);
            assert_eq!(request_id, 1);
            assert_eq!((start, end), (0, 11));
        },
        other => panic!("expected graph window request, got {other:?}"),
    }
}

#[test]
fn pane_window_request_reuses_cached_window_that_covers_range() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App { graph_tx: Some(tx), ..Default::default() };
    app.graph.generation = 7;
    app.graph.version = 2;
    app.graph.branches_window = Some(PaneWindowCache { version: 2, start: 0, end: 10, total: 20, rows: Vec::new(), is_stale: false });

    app.request_pane_window(GraphPane::Branches, 2, 8);

    assert!(rx.try_recv().is_err());

    app.request_pane_window(GraphPane::Branches, 0, 11);

    match rx.try_recv().unwrap() {
        GraphCommand::QueryPaneWindow { generation, pane, start, end } => {
            assert_eq!(generation, 7);
            assert_eq!(pane, GraphPane::Branches);
            assert_eq!((start, end), (0, 11));
        },
        other => panic!("expected pane window request, got {other:?}"),
    }
}

fn watcher_temp_dir(name: &str) -> PathBuf {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-app-watcher-{name}-{id}"));
    fs::create_dir_all(path.join("src")).unwrap();
    fs::create_dir_all(path.join(".git/refs/heads")).unwrap();
    path
}

#[test]
fn file_watcher_events_owe_a_reload_once_the_burst_settles() {
    let root = watcher_temp_dir("owed-reload");
    let mut app = App { layout_config: LayoutConfig { is_file_watcher: true, ..Default::default() }, ..Default::default() };
    app.path = Some(root.to_str().unwrap().to_string());

    app.sync_file_watcher();
    assert!(app.file_watcher.is_some(), "the watcher should start when the toggle is on");
    std::thread::sleep(std::time::Duration::from_millis(300));

    fs::write(root.join("src/changed.txt"), "external edit").unwrap();

    // The regression: Access events from the app's own reads used to keep resetting the debounce,
    // so this flag never flipped and the graph never reloaded while the status bar kept updating.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !app.pending_reload && std::time::Instant::now() < deadline {
        app.poll_file_watcher();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let owed = app.pending_reload;
    fs::remove_dir_all(&root).ok();
    assert!(owed, "a working tree change should leave a reload owed");
}

#[test]
fn an_owed_reload_survives_until_it_is_safe_to_run() {
    let mut app = App { layout_config: LayoutConfig::default(), ..Default::default() };
    app.pending_reload = true;
    app.focus = Focus::ModalCommit;

    app.run_pending_reload();

    // Reloading under a prompt would discard what the user is typing, so it waits instead.
    assert!(app.pending_reload, "an owed reload must not be dropped while a modal is open");
    assert!(!app.is_auto_reload_safe());
}

fn pane_cache(is_stale: bool) -> PaneWindowCache {
    PaneWindowCache { version: 2, start: 0, end: 10, total: 20, rows: Vec::new(), is_stale }
}

#[test]
fn stale_graph_window_still_requests_a_replacement() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App { graph_tx: Some(tx), ..Default::default() };
    app.graph.generation = 7;
    app.graph.graph_window = Some(GraphWindowCache { version: 2, start: 0, end: 10, head_alias: 1, rows: Vec::new(), history: Default::default(), is_stale: true });

    // The range is covered and the retained version still beats the reset one, so without the
    // staleness check this request is skipped and the retained rows are never replaced.
    app.request_graph_window(2, 8);

    assert!(matches!(rx.try_recv(), Ok(GraphCommand::QueryGraphWindow { start: 2, end: 8, .. })));
}

#[test]
fn stale_pane_window_still_requests_a_replacement() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App { graph_tx: Some(tx), ..Default::default() };
    app.graph.generation = 7;
    app.graph.branches_window = Some(pane_cache(true));

    app.request_pane_window(GraphPane::Branches, 2, 8);

    assert!(matches!(rx.try_recv(), Ok(GraphCommand::QueryPaneWindow { pane: GraphPane::Branches, start: 2, end: 8, .. })));
}

#[test]
fn reload_retains_the_visible_view_for_the_same_repository() {
    let (path, repo) = temp_repo("retain-same-repo");
    let oid = commit_file(&repo, "retained.txt", "retained");
    let path_string = path.display().to_string();
    let mut app =
        App { path: Some(path_string.clone()), recent: vec![path_string], repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 4, ..Default::default() };
    app.graph_scroll.set(2);
    app.graph.total = 5000;
    app.graph.graph_window = Some(GraphWindowCache { version: 1, start: 4, end: 5, head_alias: 9, rows: vec![graph_row(4, 9, oid)], history: Default::default(), is_stale: false });
    app.graph.branches_window = Some(pane_cache(false));
    app.graph.tags_window = Some(pane_cache(false));
    app.graph.stashes_window = Some(pane_cache(false));
    app.graph.reflogs_window = Some(pane_cache(false));
    app.heatmap[0][0] = 7;
    app.is_uncommitted_loaded = true;

    app.reload(None);

    // Rows stay on screen so the panes do not blank.
    assert!(app.graph.graph_window.as_ref().is_some_and(|window| window.is_stale && window.rows.len() == 1));
    for window in [&app.graph.branches_window, &app.graph.tags_window, &app.graph.stashes_window, &app.graph.reflogs_window] {
        assert!(window.as_ref().is_some_and(|window| window.is_stale), "every pane window should be retained and marked stale");
    }
    // Retaining the count is what stops the draw pass clamping the selection and scroll to the top.
    assert_eq!(app.graph.total, 5000);
    assert_eq!(app.graph_commit_count(), 5000);
    assert_eq!(app.graph_selected, 4);
    assert_eq!(app.graph_scroll.get(), 2);
    assert_eq!(app.heatmap[0][0], 7);
    assert!(app.is_uncommitted_loaded);
    // The off-window lookup cache is deliberately dropped.
    assert!(app.graph.index_rows.is_empty());
    stop_graph_service(&mut app);
}

#[test]
fn reload_clears_the_visible_view_when_switching_repository() {
    let (path, repo) = temp_repo("retain-switch-from");
    let oid = commit_file(&repo, "from.txt", "from");
    let (other_path, other_repo) = temp_repo("retain-switch-to");
    commit_file(&other_repo, "to.txt", "to");
    let path_string = path.display().to_string();
    let other_string = other_path.display().to_string();
    let mut app = App {
        path: Some(path_string.clone()),
        recent: vec![path_string, other_string.clone()],
        repo: Some(Rc::new(repo)),
        viewport: Viewport::Graph,
        focus: Focus::Viewport,
        graph_selected: 4,
        ..Default::default()
    };
    app.graph.total = 5000;
    app.graph.graph_window = Some(GraphWindowCache { version: 1, start: 4, end: 5, head_alias: 9, rows: vec![graph_row(4, 9, oid)], history: Default::default(), is_stale: false });
    app.graph.branches_window = Some(pane_cache(false));
    app.heatmap[0][0] = 7;
    app.is_uncommitted_loaded = true;

    app.reload(Some(other_string));

    // Showing the previous repository's history here would present it as if it were the new one.
    assert!(app.graph.graph_window.is_none());
    assert!(app.graph.branches_window.is_none());
    assert_eq!(app.graph.total, 0);
    assert_eq!(app.heatmap[0][0], 0);
    assert!(!app.is_uncommitted_loaded);
    stop_graph_service(&mut app);
}

#[test]
fn a_delivered_window_replaces_the_retained_one() {
    let (_path, repo) = temp_repo("retain-replace");
    let oid = commit_file(&repo, "replace.txt", "replace");
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    app.graph.generation = 3;
    app.graph.graph_window = Some(GraphWindowCache { version: 1, start: 0, end: 1, head_alias: 9, rows: Vec::new(), history: Default::default(), is_stale: true });
    app.graph.branches_window = Some(pane_cache(true));
    app.graph.requested_graph = Some((1, 0, 1));

    app.handle_graph_event(
        &repo,
        GraphEvent::GraphWindow { generation: 3, request_id: 1, version: 5, start: 0, end: 1, total: 42, head_alias: 9, rows: vec![graph_row(0, 9, oid)], history: Default::default() },
    );
    app.handle_graph_event(&repo, GraphEvent::PaneWindow { generation: 3, version: 5, pane: GraphPane::Branches, start: 0, end: 1, total: 3, rows: Vec::new() });

    assert!(app.graph.graph_window.as_ref().is_some_and(|window| !window.is_stale));
    assert!(app.graph.branches_window.as_ref().is_some_and(|window| !window.is_stale));
    assert_eq!(app.graph.total, 42);
}

#[test]
fn a_clamped_graph_window_reply_releases_the_request_slot() {
    let (_path, repo) = temp_repo("clamped-reply");
    let oid = commit_file(&repo, "clamped.txt", "clamped");
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    app.graph.generation = 3;
    let retained = GraphWindowCache { version: 1, start: 0, end: 6, head_alias: 9, rows: vec![graph_row(0, 9, oid)], history: Default::default(), is_stale: true };
    app.graph.graph_window = Some(retained);
    app.graph.requested_graph = Some((1, 0, 6));

    // The worker clamps the range to what it has walked, which right after a reload is nothing.
    app.handle_graph_event(&repo, GraphEvent::GraphWindow { generation: 3, request_id: 1, version: 5, start: 0, end: 0, total: 0, head_alias: 9, rows: Vec::new(), history: Default::default() });

    // The narrower answer is not useful, so the retained rows stay on screen, but the slot must be
    // released. Leaving it pending makes request_graph_window suppress every later request for a
    // covered range, and the window is then never replaced again.
    assert_eq!(app.graph.requested_graph, None, "a clamped reply must not leave the request slot pending");
    assert!(app.graph.graph_window.as_ref().is_some_and(|window| window.is_stale && window.rows.len() == 1));
}

#[test]
fn reload_replaces_retained_rows_with_fresh_topology() {
    let (path, repo) = temp_repo("fresh-topology");
    for index in 0..4 {
        commit_file(&repo, "file.txt", &format!("commit {index}"));
    }
    let path_string = path.display().to_string();
    let mut app = App {
        path: Some(path_string.clone()),
        recent: vec![path_string],
        repo: Some(Rc::new(Repository::open(&path).unwrap())),
        viewport: Viewport::Graph,
        focus: Focus::Viewport,
        layout: Layout { graph: Rect::new(0, 0, 120, 20), graph_scrollbar: Rect::new(119, 0, 1, 20), ..Default::default() },
        ..Default::default()
    };

    let settle = |app: &mut App| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            let repo = app.repo.clone().unwrap();
            app.sync(&repo);
            let mut terminal = Terminal::new(TestBackend::new(120, 20)).unwrap();
            terminal.draw(|frame| app.draw_graph(frame, &repo)).unwrap();
            if app.graph.is_complete && app.graph.graph_window.as_ref().is_some_and(|window| !window.is_stale) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("graph window never settled");
    };

    app.reload(None);
    settle(&mut app);

    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("fresh-branch", &head_commit, false).unwrap();
    app.reload(None);
    settle(&mut app);

    let labels: Vec<String> = app.graph.graph_window.as_ref().map(|window| window.rows.iter().flat_map(|row| row.branches.iter().map(|label| label.name.clone())).collect()).unwrap_or_default();
    stop_graph_service(&mut app);
    assert!(labels.iter().any(|name| name == "fresh-branch"), "a branch created between reloads must reach the graph rows, got {labels:?}");
}
