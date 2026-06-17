use super::*;
use crate::app::app::{GraphWindowCache, PaneWindowCache};
use crate::core::{
    chunk::NONE,
    graph_service::{GraphCommand, GraphEvent, GraphFileHistoryRow, GraphLookupKind, GraphLookupResult, GraphPane, GraphPaneRow, GraphReflogLabel, GraphRow},
    reflogs::HeadReflogAliasEntry,
};
use crate::{
    app::{
        app::{SettingsSelection, SettingsSelectionKind, SettingsTab},
        state::layout::Layout,
    },
    git::queries::helpers::FileStatus,
    helpers::{
        keymap::{Command, InputMode, KeyBinding, KeymapSelection, Keymaps, load_keymaps_from_path},
        layout::LayoutConfig,
    },
};
use git2::{Repository, Signature};
use indexmap::IndexMap;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
};
use std::{
    fs,
    path::Path,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_non_repo_path(name: &str) -> String {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-input-navigation-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    path.display().to_string()
}

fn temp_keymap_path(name: &str) -> std::path::PathBuf {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("guitar-input-navigation-{name}-{id}")).join("keymap.json")
}

fn temp_recent_path(name: &str) -> std::path::PathBuf {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("guitar-input-navigation-{name}-{id}")).join("recent.json")
}

fn minimal_keymaps() -> Keymaps {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    normal.insert(KeyBinding::new(KeyCode::Char('k'), KeyModifiers::NONE), Command::ScrollUp);
    let mut action = IndexMap::new();
    action.insert(KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, action);
    maps
}

fn temp_repo(name: &str) -> (std::path::PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-input-navigation-{name}-{id}"));
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
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}

fn graph_app_with_history() -> (App, git2::Oid, git2::Oid, git2::Oid) {
    let (path, repo) = temp_repo("history");
    let root_oid = commit_file(&repo, "root.txt", "root");
    let parent_oid = commit_file(&repo, "parent.txt", "parent");
    let child_oid = commit_file(&repo, "child.txt", "child");

    let mut app = App { path: Some(path.display().to_string()), repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    let root_alias = app.oids.get_alias_by_oid(root_oid);
    let parent_alias = app.oids.get_alias_by_oid(parent_oid);
    let child_alias = app.oids.get_alias_by_oid(child_oid);
    app.oids.sorted_aliases = vec![NONE, child_alias, parent_alias, root_alias];

    app.branches.all.insert(parent_alias, vec!["parent".to_string()]);
    app.branches.sorted.push((parent_alias, "parent".to_string()));

    (app, root_oid, parent_oid, child_oid)
}

fn diff_filenames(app: &App) -> Vec<String> {
    app.current_diff.iter().map(|change| change.filename.clone()).collect()
}

fn stop_graph_service(app: &mut App) {
    if let Some(tx) = app.graph_tx.take() {
        let _ = tx.send(GraphCommand::Shutdown);
    }
}

fn search_history_row(graph_index: usize, oid: git2::Oid) -> GraphFileHistoryRow {
    GraphFileHistoryRow { graph_index, oid, short_oid: oid.to_string()[..8].to_string(), summary: "history".to_string(), status: FileStatus::Modified }
}

fn branch_app() -> App {
    let mut app = App { path: Some(temp_non_repo_path("branches")), viewport: Viewport::Graph, ..Default::default() };
    app.branches.sorted = vec![(0, "feature".to_string()), (1, "main".to_string())];
    app.branches.all.insert(0, vec!["feature".to_string()]);
    app.branches.all.insert(1, vec!["main".to_string()]);
    app.oids.sorted_aliases = vec![NONE, 1];
    app
}

fn hidden_branches(app: &App) -> Vec<String> {
    let mut branches: Vec<String> = app.branches.hidden_branch_names.iter().cloned().collect();
    branches.sort();
    branches
}

fn directional_focus_app() -> App {
    App {
        viewport: Viewport::Graph,
        focus: Focus::Viewport,
        layout: Layout::default(),
        layout_config: LayoutConfig { is_branches: false, is_tags: false, is_stashes: false, is_reflogs: false, is_worktrees: false, is_status: false, is_inspector: false, ..Default::default() },
        ..Default::default()
    }
}

#[test]
fn directional_focus_moves_left_and_right_between_side_panes_and_viewport() {
    let mut app = directional_focus_app();
    app.layout_config.is_branches = true;
    app.layout.pane_branches = Rect::new(0, 0, 20, 20);
    app.layout.graph = Rect::new(20, 0, 40, 20);

    app.on_focus_pane_left();
    assert_eq!(app.focus, Focus::Branches);

    app.on_focus_pane_right();
    assert_eq!(app.focus, Focus::Viewport);
}

#[test]
fn directional_focus_moves_up_and_down_inside_left_stack() {
    let mut app = directional_focus_app();
    app.layout_config.is_branches = true;
    app.layout_config.is_tags = true;
    app.layout_config.is_stashes = true;
    app.focus = Focus::Tags;
    app.layout.pane_branches = Rect::new(0, 0, 20, 5);
    app.layout.pane_tags = Rect::new(0, 5, 20, 5);
    app.layout.pane_stashes = Rect::new(0, 10, 20, 5);
    app.layout.graph = Rect::new(20, 0, 40, 15);

    app.on_focus_pane_up();
    assert_eq!(app.focus, Focus::Branches);

    app.focus = Focus::Tags;
    app.on_focus_pane_down();
    assert_eq!(app.focus, Focus::Stashes);
}

#[test]
fn directional_focus_reaches_submodules_then_search_at_bottom_of_left_stack() {
    let mut app = directional_focus_app();
    app.layout_config.is_worktrees = true;
    app.layout_config.is_submodules = true;
    app.layout_config.is_search = true;
    app.focus = Focus::Worktrees;
    app.layout.pane_worktrees = Rect::new(0, 0, 20, 5);
    app.layout.pane_submodules = Rect::new(0, 5, 20, 5);
    app.layout.pane_search = Rect::new(0, 10, 20, 5);
    app.layout.graph = Rect::new(20, 0, 40, 15);

    app.on_focus_pane_down();

    assert_eq!(app.focus, Focus::Submodules);

    app.on_focus_pane_down();

    assert_eq!(app.focus, Focus::Search);
}

#[test]
fn directional_focus_moves_up_and_down_inside_right_stack() {
    let mut app = directional_focus_app();
    app.layout_config.is_status = true;
    app.layout_config.is_inspector = true;
    app.uncommitted.has_conflicts = true;
    app.focus = Focus::StatusTop;
    app.layout.graph = Rect::new(20, 0, 40, 20);
    app.layout.pane_inspector = Rect::new(60, 0, 20, 6);
    app.layout.pane_status_top = Rect::new(60, 6, 20, 7);
    app.layout.pane_status_bottom = Rect::new(60, 13, 20, 7);

    app.on_focus_pane_up();
    assert_eq!(app.focus, Focus::Inspector);

    app.focus = Focus::StatusTop;
    app.on_focus_pane_down();
    assert_eq!(app.focus, Focus::StatusBottom);
}

#[test]
fn directional_focus_chooses_nearest_perpendicular_center() {
    let mut app = directional_focus_app();
    app.layout_config.is_status = true;
    app.layout_config.is_inspector = true;
    app.graph_selected = 1;
    app.layout.graph = Rect::new(20, 0, 40, 20);
    app.layout.pane_inspector = Rect::new(60, 0, 20, 6);
    app.layout.pane_status_top = Rect::new(60, 6, 20, 7);

    app.on_focus_pane_right();

    assert_eq!(app.focus, Focus::StatusTop);
}

#[test]
fn directional_focus_noops_for_strict_diagonal_candidate() {
    let mut app = directional_focus_app();
    app.layout_config.is_branches = true;
    app.focus = Focus::Branches;
    app.layout.pane_branches = Rect::new(0, 0, 10, 5);
    app.layout.graph = Rect::new(20, 10, 40, 10);

    app.on_focus_pane_right();

    assert_eq!(app.focus, Focus::Branches);
}

#[test]
fn directional_focus_noops_in_settings_and_without_candidate() {
    let mut settings = directional_focus_app();
    settings.viewport = Viewport::Settings;
    settings.layout_config.is_branches = true;
    settings.layout.pane_branches = Rect::new(0, 0, 20, 20);
    settings.layout.graph = Rect::new(20, 0, 40, 20);

    settings.on_focus_pane_left();
    assert_eq!(settings.focus, Focus::Viewport);

    let mut splash = directional_focus_app();
    splash.viewport = Viewport::Splash;
    splash.layout_config.is_branches = true;
    splash.layout.pane_branches = Rect::new(0, 0, 20, 20);
    splash.layout.graph = Rect::new(20, 0, 40, 20);

    splash.on_focus_pane_left();
    assert_eq!(splash.focus, Focus::Viewport);

    let mut solo = directional_focus_app();
    solo.layout.graph = Rect::new(20, 0, 40, 20);

    solo.on_focus_pane_left();
    assert_eq!(solo.focus, Focus::Viewport);
}

#[test]
fn solo_branch_from_pane_keeps_selected_as_only_visible() {
    let mut app = branch_app();
    app.focus = Focus::Branches;
    app.branches_selected = 1;

    app.on_solo_branch();

    assert_eq!(hidden_branches(&app), vec!["feature"]);
}

#[test]
fn toggle_visible_branch_adds_selected_branch_to_hidden_layer() {
    let mut app = branch_app();
    app.focus = Focus::Branches;
    app.branches_selected = 1;

    app.on_toggle_branch();

    assert_eq!(hidden_branches(&app), vec!["main"]);
}

#[test]
fn toggle_hidden_branch_removes_selected_branch_from_hidden_layer() {
    let mut app = branch_app();
    app.focus = Focus::Branches;
    app.branches_selected = 1;
    app.branches.hidden_branch_names.insert("main".to_string());

    app.on_toggle_branch();

    assert!(app.branches.hidden_branch_names.is_empty());
}

#[test]
fn toggle_last_visible_branch_clears_hidden_layer() {
    let mut app = branch_app();
    app.focus = Focus::Branches;
    app.branches_selected = 1;
    app.branches.hidden_branch_names.insert("feature".to_string());

    app.on_toggle_branch();

    assert!(app.branches.hidden_branch_names.is_empty());
}

#[test]
fn graph_solo_uses_selected_commit_branch() {
    let mut app = branch_app();
    app.focus = Focus::Viewport;
    app.graph_selected = 1;

    app.on_solo_branch();

    assert_eq!(hidden_branches(&app), vec!["feature"]);
}

#[test]
fn graph_toggle_uses_selected_commit_branch() {
    let mut app = branch_app();
    app.focus = Focus::Viewport;
    app.graph_selected = 1;

    app.on_toggle_branch();

    assert_eq!(hidden_branches(&app), vec!["main"]);
}

#[test]
fn graph_toggle_multiple_branch_commit_opens_toggle_modal() {
    let mut app = branch_app();
    app.focus = Focus::Viewport;
    app.graph_selected = 1;
    app.branches.sorted.push((1, "release".to_string()));

    app.on_toggle_branch();

    assert_eq!(app.focus, Focus::ModalSolo);
    assert_eq!(app.modal_branch_action, BranchModalAction::Toggle);
    assert_eq!(app.modal_solo_selected, 0);
}

#[test]
fn branch_toggle_uses_git_branch_universe_when_pane_window_is_partial() {
    let (path, repo) = temp_repo("branch-window-toggle");
    let oid = commit_file(&repo, "main.txt", "main");
    {
        let commit = repo.find_commit(oid).unwrap();
        repo.branch("feature", &commit, false).unwrap();
        repo.branch("main", &commit, false).unwrap();
    }

    let mut app = App {
        path: Some(path.display().to_string()),
        repo: Some(Rc::new(repo)),
        recent_save_path: Some(temp_recent_path("branch-window-toggle")),
        viewport: Viewport::Graph,
        focus: Focus::Branches,
        branches_selected: 1,
        ..Default::default()
    };
    app.graph.branches_window =
        Some(PaneWindowCache { version: 1, start: 1, end: 2, total: 3, rows: vec![GraphPaneRow::Branch { alias: 1, name: "main".to_string(), is_local: true, lane: None, graph_index: Some(1) }] });

    app.on_toggle_branch();

    assert_eq!(hidden_branches(&app), vec!["main"]);
}

#[test]
fn reload_all_branches_clears_hidden_branch_layer() {
    let mut app = App { path: Some(temp_non_repo_path("reload-all-branches")), viewport: Viewport::Graph, focus: Focus::Branches, ..Default::default() };
    app.branches.hidden_branch_names.insert("main".to_string());
    app.branches.hidden_branch_names.insert("origin/archive".to_string());

    app.on_reload_all_branches();

    assert!(app.branches.hidden_branch_names.is_empty());
    assert_eq!(app.focus, Focus::Branches);
}

#[test]
fn search_pane_navigation_uses_result_count() {
    let (_path, repo) = temp_repo("search-nav");
    let oid = commit_file(&repo, "target.txt", "target");
    let mut app = App { repo: Some(Rc::new(repo)), focus: Focus::Search, ..Default::default() };
    app.layout.search.height = 5;
    app.search_rows = (0..10).map(|idx| search_history_row(idx, oid)).collect();

    app.on_scroll_down();
    assert_eq!(app.search_selected, 1);

    app.on_scroll_page_down();
    assert_eq!(app.search_selected, 5);

    app.on_scroll_to_end();
    assert_eq!(app.search_selected, 9);

    app.on_scroll_down();
    assert_eq!(app.search_selected, 9);

    app.on_scroll_page_up();
    assert_eq!(app.search_selected, 5);
}

#[test]
fn search_narrow_jumps_to_related_graph_commit() {
    let (mut app, _root, parent, _child) = graph_app_with_history();
    app.focus = Focus::Search;
    app.layout.graph.height = 10;
    app.search_selected = 0;
    app.search_rows = vec![search_history_row(2, parent)];

    app.on_narrow_scope();

    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.graph_selected, 2);
    assert_eq!(app.graph_scroll.get(), 0);
    assert_eq!(diff_filenames(&app), vec!["parent.txt".to_string()]);
}

#[test]
fn empty_recent_splash_scrolls_keep_selection_at_zero() {
    let mut app = App { viewport: Viewport::Splash, focus: Focus::Viewport, ..Default::default() };
    app.layout.graph.height = 10;
    app.splash_selected = 3;

    app.on_scroll_down();
    assert_eq!(app.splash_selected, 0);

    app.splash_selected = 3;
    app.on_scroll_page_down();
    assert_eq!(app.splash_selected, 0);

    app.splash_selected = 3;
    app.on_scroll_half_page_down();
    assert_eq!(app.splash_selected, 0);

    app.splash_selected = 3;
    app.on_scroll_to_end();
    assert_eq!(app.splash_selected, 0);
}

#[test]
fn splash_d_key_removes_selected_recent_repository_and_persists() {
    let path = temp_recent_path("remove-middle");
    let mut keymaps = minimal_keymaps();
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('d'), KeyModifiers::NONE), Command::RemoveRecentRepository);
    let mut app = App {
        viewport: Viewport::Splash,
        focus: Focus::Viewport,
        keymaps,
        recent: vec!["/repo/a".into(), "/repo/b".into(), "/repo/c".into()],
        splash_selected: 1,
        recent_save_path: Some(path.clone()),
        ..Default::default()
    };

    app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

    assert_eq!(app.recent, vec!["/repo/a".to_string(), "/repo/c".to_string()]);
    assert_eq!(app.splash_selected, 1);
    let saved: Vec<String> = facet_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(saved, app.recent);
}

#[test]
fn splash_remove_recent_last_item_selects_previous() {
    let mut app = App {
        viewport: Viewport::Splash,
        focus: Focus::Viewport,
        recent: vec!["/repo/a".into(), "/repo/b".into()],
        splash_selected: 1,
        recent_save_path: Some(temp_recent_path("remove-last")),
        ..Default::default()
    };

    app.on_remove_recent_repository();

    assert_eq!(app.recent, vec!["/repo/a".to_string()]);
    assert_eq!(app.splash_selected, 0);
}

#[test]
fn splash_remove_recent_current_repo_keeps_repo_open() {
    let (repo_path, repo) = temp_repo("remove-current");
    let current = repo_path.display().to_string();
    let mut app = App {
        path: Some(current.clone()),
        repo: Some(Rc::new(repo)),
        viewport: Viewport::Splash,
        focus: Focus::Viewport,
        recent: vec![current.clone(), "/repo/other".into()],
        splash_selected: 0,
        recent_save_path: Some(temp_recent_path("remove-current")),
        ..Default::default()
    };

    app.on_remove_recent_repository();

    assert_eq!(app.recent, vec!["/repo/other".to_string()]);
    assert_eq!(app.path.as_deref(), Some(current.as_str()));
    assert!(app.repo.is_some());
    assert_eq!(app.viewport, Viewport::Splash);
}

#[test]
fn splash_remove_recent_empty_list_normalizes_selection_without_saving() {
    let path = temp_recent_path("remove-empty");
    let mut app = App { viewport: Viewport::Splash, focus: Focus::Viewport, splash_selected: 7, recent_save_path: Some(path.clone()), ..Default::default() };

    app.on_remove_recent_repository();

    assert!(app.recent.is_empty());
    assert_eq!(app.splash_selected, 0);
    assert!(!path.exists());
}

#[test]
fn splash_remove_recent_noops_while_loading() {
    let path = temp_recent_path("remove-loading");
    let mut app =
        App { viewport: Viewport::Splash, focus: Focus::Viewport, recent: vec!["/repo/a".into(), "/repo/b".into()], splash_selected: 1, recent_save_path: Some(path.clone()), ..Default::default() };
    app.spinner.running.store(true, std::sync::atomic::Ordering::SeqCst);

    app.on_remove_recent_repository();

    assert_eq!(app.recent, vec!["/repo/a".to_string(), "/repo/b".to_string()]);
    assert_eq!(app.splash_selected, 1);
    assert!(!path.exists());
}

#[test]
fn splash_shift_k_moves_selected_recent_repository_up_and_persists() {
    let path = temp_recent_path("move-up");
    let mut keymaps = minimal_keymaps();
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('K'), KeyModifiers::SHIFT), Command::MoveRecentRepositoryUp);
    let mut app = App {
        viewport: Viewport::Splash,
        focus: Focus::Viewport,
        keymaps,
        recent: vec!["/repo/a".into(), "/repo/b".into(), "/repo/c".into()],
        splash_selected: 1,
        recent_save_path: Some(path.clone()),
        ..Default::default()
    };

    app.handle_key_event(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));

    assert_eq!(app.recent, vec!["/repo/b".to_string(), "/repo/a".to_string(), "/repo/c".to_string()]);
    assert_eq!(app.splash_selected, 0);
    let saved: Vec<String> = facet_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(saved, app.recent);
}

#[test]
fn splash_shift_j_moves_selected_recent_repository_down_and_persists() {
    let path = temp_recent_path("move-down");
    let mut keymaps = minimal_keymaps();
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('J'), KeyModifiers::SHIFT), Command::MoveRecentRepositoryDown);
    let mut app = App {
        viewport: Viewport::Splash,
        focus: Focus::Viewport,
        keymaps,
        recent: vec!["/repo/a".into(), "/repo/b".into(), "/repo/c".into()],
        splash_selected: 1,
        recent_save_path: Some(path.clone()),
        ..Default::default()
    };

    app.handle_key_event(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::SHIFT));

    assert_eq!(app.recent, vec!["/repo/a".to_string(), "/repo/c".to_string(), "/repo/b".to_string()]);
    assert_eq!(app.splash_selected, 2);
    let saved: Vec<String> = facet_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(saved, app.recent);
}

#[test]
fn splash_move_recent_boundary_noops_without_saving() {
    let path = temp_recent_path("move-boundary");
    let mut app =
        App { viewport: Viewport::Splash, focus: Focus::Viewport, recent: vec!["/repo/a".into(), "/repo/b".into()], splash_selected: 0, recent_save_path: Some(path.clone()), ..Default::default() };

    app.on_move_recent_repository_up();
    assert_eq!(app.recent, vec!["/repo/a".to_string(), "/repo/b".to_string()]);
    assert_eq!(app.splash_selected, 0);
    assert!(!path.exists());

    app.splash_selected = 1;
    app.on_move_recent_repository_down();
    assert_eq!(app.recent, vec!["/repo/a".to_string(), "/repo/b".to_string()]);
    assert_eq!(app.splash_selected, 1);
    assert!(!path.exists());
}

#[test]
fn settings_recent_repository_remove_persists_selected_row() {
    let path = temp_recent_path("settings-remove");
    let mut app = App {
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        recent: vec!["/repo/a".into(), "/repo/b".into(), "/repo/c".into()],
        settings_selected: 12,
        settings_selections: vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::RecentRepository(1) }],
        recent_save_path: Some(path.clone()),
        ..Default::default()
    };

    app.on_remove_recent_repository();

    assert_eq!(app.recent, vec!["/repo/a".to_string(), "/repo/c".to_string()]);
    assert_eq!(app.settings_selected, 12);
    let saved: Vec<String> = facet_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(saved, app.recent);
}

#[test]
fn settings_recent_repository_move_up_and_down_persist_and_follow_row() {
    let up_path = temp_recent_path("settings-move-up");
    let mut up = App {
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        recent: vec!["/repo/a".into(), "/repo/b".into(), "/repo/c".into()],
        settings_selected: 12,
        settings_selections: vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::RecentRepository(1) }],
        recent_save_path: Some(up_path.clone()),
        ..Default::default()
    };

    up.on_move_recent_repository_up();

    assert_eq!(up.recent, vec!["/repo/b".to_string(), "/repo/a".to_string(), "/repo/c".to_string()]);
    assert_eq!(up.settings_selected, 11);
    let saved: Vec<String> = facet_json::from_str(&fs::read_to_string(up_path).unwrap()).unwrap();
    assert_eq!(saved, up.recent);

    let down_path = temp_recent_path("settings-move-down");
    let mut down = App {
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        recent: vec!["/repo/a".into(), "/repo/b".into(), "/repo/c".into()],
        settings_selected: 12,
        settings_selections: vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::RecentRepository(1) }],
        recent_save_path: Some(down_path.clone()),
        ..Default::default()
    };

    down.on_move_recent_repository_down();

    assert_eq!(down.recent, vec!["/repo/a".to_string(), "/repo/c".to_string(), "/repo/b".to_string()]);
    assert_eq!(down.settings_selected, 13);
    let saved: Vec<String> = facet_json::from_str(&fs::read_to_string(down_path).unwrap()).unwrap();
    assert_eq!(saved, down.recent);
}

#[test]
fn settings_recent_repository_commands_noop_on_non_recent_rows() {
    let path = temp_recent_path("settings-non-recent");
    let mut keymaps = minimal_keymaps();
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('d'), KeyModifiers::NONE), Command::RemoveRecentRepository);
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('K'), KeyModifiers::SHIFT), Command::MoveRecentRepositoryUp);
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('J'), KeyModifiers::SHIFT), Command::MoveRecentRepositoryDown);
    let mut app = App {
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        keymaps,
        recent: vec!["/repo/a".into(), "/repo/b".into()],
        settings_selected: 12,
        settings_selections: vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::Info }],
        recent_save_path: Some(path.clone()),
        ..Default::default()
    };

    app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::SHIFT));

    assert_eq!(app.recent, vec!["/repo/a".to_string(), "/repo/b".to_string()]);
    assert_eq!(app.settings_selected, 12);
    assert!(!path.exists());
}

#[test]
fn empty_branch_pane_select_and_narrow_are_noops() {
    let (_path, repo) = temp_repo("empty-branches");
    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Branches, ..Default::default() };

    app.on_select();
    assert_eq!(app.focus, Focus::Branches);

    app.on_narrow_scope();
    assert_eq!(app.focus, Focus::Branches);
}

fn assert_offscreen_pane_narrow_requests_walker_row(focus: Focus, pane: GraphPane, selection: usize) {
    let (_path, repo) = temp_repo("offscreen-pane");
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(Rc::new(repo)), graph_tx: Some(tx), viewport: Viewport::Graph, focus, ..Default::default() };
    app.graph.generation = 7;

    match pane {
        GraphPane::Branches => app.branches_selected = selection,
        GraphPane::Tags => app.tags_selected = selection,
        GraphPane::Stashes => app.stashes_selected = selection,
        GraphPane::Reflogs => app.reflogs_selected = selection,
    }

    app.on_narrow_scope();

    let command = rx.try_recv().unwrap();
    match command {
        GraphCommand::Lookup { generation, request_id, kind: GraphLookupKind::PaneRowAt { pane: actual_pane, index } } => {
            assert_eq!(generation, 7);
            assert_eq!(request_id, 1);
            assert_eq!(actual_pane, pane);
            assert_eq!(index, selection);
        },
        other => panic!("expected pane row lookup, got {other:?}"),
    }
}

#[test]
fn offscreen_pane_narrow_requests_selected_row_from_walker() {
    assert_offscreen_pane_narrow_requests_walker_row(Focus::Branches, GraphPane::Branches, 42);
    assert_offscreen_pane_narrow_requests_walker_row(Focus::Tags, GraphPane::Tags, 17);
    assert_offscreen_pane_narrow_requests_walker_row(Focus::Stashes, GraphPane::Stashes, 9);
    assert_offscreen_pane_narrow_requests_walker_row(Focus::Reflogs, GraphPane::Reflogs, 23);
}

#[test]
fn offscreen_graph_narrow_requests_row_before_opening_inspector() {
    let (_path, repo) = temp_repo("offscreen-inspector");
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(Rc::new(repo)), graph_tx: Some(tx), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    app.graph.generation = 7;
    app.graph_selected = 42;
    app.layout_config.is_zen = false;

    app.on_narrow_scope();

    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.layout_config.is_inspector);
    let command = rx.try_recv().unwrap();
    match command {
        GraphCommand::Lookup { generation, request_id, kind: GraphLookupKind::GraphRowAt { index } } => {
            assert_eq!(generation, 7);
            assert_eq!(request_id, 1);
            assert_eq!(index, 42);
        },
        other => panic!("expected graph row lookup, got {other:?}"),
    }
}

#[test]
fn zen_offscreen_graph_narrow_opens_inspector_while_requesting_row() {
    let (_path, repo) = temp_repo("zen-offscreen-inspector");
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(Rc::new(repo)), graph_tx: Some(tx), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    app.graph.generation = 7;
    app.graph_selected = 42;
    app.layout_config.is_zen = true;

    app.on_narrow_scope();

    assert_eq!(app.focus, Focus::Inspector);
    assert!(app.layout_config.is_inspector);
    let command = rx.try_recv().unwrap();
    match command {
        GraphCommand::Lookup { generation, request_id, kind: GraphLookupKind::GraphRowAt { index } } => {
            assert_eq!(generation, 7);
            assert_eq!(request_id, 1);
            assert_eq!(index, 42);
        },
        other => panic!("expected graph row lookup, got {other:?}"),
    }
}

#[test]
fn zen_graph_narrow_promotes_cached_window_row_before_opening_inspector() {
    let (_path, repo) = temp_repo("zen-cached-inspector");
    let oid = commit_file(&repo, "cached.txt", "cached");
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(Rc::new(repo)), graph_tx: Some(tx), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    app.graph.generation = 7;
    app.graph.total = 43;
    app.graph_selected = 42;
    app.layout_config.is_zen = true;
    app.graph.graph_window = Some(GraphWindowCache {
        version: 1,
        start: 42,
        end: 43,
        head_alias: 99,
        rows: vec![GraphRow {
            index: 42,
            alias: 99,
            oid,
            summary: "cached".to_string(),
            committer_date: String::new(),
            committer_name: String::new(),
            has_any_branch: false,
            branches: Vec::new(),
            tags: Vec::new(),
            is_stash: false,
            stash_lane: None,
            worktrees: Vec::new(),
            reflog: None,
        }],
        history: Default::default(),
    });

    app.on_narrow_scope();

    assert_eq!(app.focus, Focus::Inspector);
    assert!(app.layout_config.is_inspector);
    assert!(rx.try_recv().is_err());

    app.graph.graph_window = None;
    let identity = app.graph_identity_at(42).unwrap();
    assert_eq!(identity.alias, 99);
    assert_eq!(identity.oid, oid);
}

#[test]
fn graph_row_lookup_result_opens_inspector_with_reflog() {
    let (_path, repo) = temp_repo("offscreen-inspector-result");
    let oid = commit_file(&repo, "commit.txt", "commit");
    let repo = Rc::new(repo);
    let (cmd_tx, _cmd_rx) = std::sync::mpsc::channel();
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut app = App { repo: Some(repo.clone()), graph_tx: Some(cmd_tx), graph_rx: Some(event_rx), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 42, ..Default::default() };
    app.graph.generation = 7;
    app.graph.pending_lookup = Some((3, PendingGraphLookup::OpenInspector));

    event_tx
        .send(GraphEvent::LookupResult {
            generation: 7,
            request_id: 3,
            result: GraphLookupResult::GraphRow(Some(GraphRow {
                index: 42,
                alias: 99,
                oid,
                summary: "commit".to_string(),
                committer_date: String::new(),
                committer_name: String::new(),
                has_any_branch: false,
                branches: Vec::new(),
                tags: Vec::new(),
                is_stash: false,
                stash_lane: None,
                worktrees: Vec::new(),
                reflog: Some(GraphReflogLabel { selector: "HEAD@{0}".to_string(), message: "commit: commit".to_string(), lane: Some(2) }),
            })),
        })
        .unwrap();
    app.sync(&repo);

    assert_eq!(app.focus, Focus::Inspector);
    assert_eq!(app.graph_alias_at(42), Some(99));
    assert_eq!(app.graph_oid_at(42), Some(oid));
    assert_eq!(app.graph_row_at(42).and_then(|row| row.reflog.as_ref()).map(|entry| entry.selector.as_str()), Some("HEAD@{0}"));
    assert_eq!(diff_filenames(&app), vec!["commit.txt"]);
}

#[test]
fn pane_row_jump_uses_graph_index_and_refreshes_diff() {
    let (mut app, _root_oid, _parent_oid, _child_oid) = graph_app_with_history();
    app.focus = Focus::Branches;
    app.graph_selected = 3;
    app.current_diff.clear();

    assert!(app.open_graph_pane_row(GraphPaneRow::Branch { alias: 99, name: "parent".to_string(), is_local: true, lane: None, graph_index: Some(2) }));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.graph_selected, 2);
    assert_eq!(diff_filenames(&app), vec!["parent.txt"]);
}

#[test]
fn pane_row_jump_centers_selected_graph_row() {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Branches, ..Default::default() };
    app.graph.total = 100;
    app.layout.graph.height = 10;
    app.layout_config.is_zen = false;

    assert!(app.open_graph_pane_row(GraphPaneRow::Branch { alias: 99, name: "feature".to_string(), is_local: true, lane: None, graph_index: Some(40) }));

    assert_eq!(app.graph_selected, 40);
    assert_eq!(app.graph_scroll.get(), 35);
}

#[test]
fn pane_row_jump_centering_clamps_near_graph_edges() {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Branches, ..Default::default() };
    app.graph.total = 100;
    app.layout.graph.height = 10;
    app.layout_config.is_zen = false;

    assert!(app.open_graph_pane_row(GraphPaneRow::Tag { alias: 99, name: "v1".to_string(), lane: None, graph_index: Some(2) }));
    assert_eq!(app.graph_scroll.get(), 0);

    assert!(app.open_graph_pane_row(GraphPaneRow::Stash { alias: 99, summary: "stash".to_string(), lane: None, graph_index: Some(98) }));
    assert_eq!(app.graph_scroll.get(), 90);
}

#[test]
fn zen_pane_row_jump_uses_inner_graph_height_for_centering() {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Reflogs, ..Default::default() };
    app.graph.total = 100;
    app.layout.graph.height = 10;
    app.layout_config.is_zen = true;

    assert!(app.open_graph_pane_row(GraphPaneRow::Reflog { selector: "HEAD@{0}".to_string(), message: "commit: feature".to_string(), alias: 99, lane: None, graph_index: Some(40) }));

    assert_eq!(app.graph_selected, 40);
    assert_eq!(app.graph_scroll.get(), 36);
}

#[test]
fn pane_alias_fallback_jump_centers_selected_graph_row() {
    let (_path, repo) = temp_repo("pane-alias-center");
    let oid = commit_file(&repo, "feature.txt", "feature");
    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Branches, ..Default::default() };
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE; 100];
    app.oids.sorted_aliases[40] = alias;
    app.branches.sorted = vec![(alias, "feature".to_string())];
    app.graph.total = 100;
    app.layout.graph.height = 10;
    app.layout_config.is_zen = false;

    app.on_narrow_scope();

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.graph_selected, 40);
    assert_eq!(app.graph_scroll.get(), 35);
}

#[test]
fn empty_delete_tag_modal_navigation_stays_at_zero() {
    let mut app = App { focus: Focus::ModalDeleteTag, modal_delete_tag_selected: 4, ..Default::default() };

    app.on_scroll_up();
    assert_eq!(app.modal_delete_tag_selected, 0);

    app.modal_delete_tag_selected = 4;
    app.on_scroll_down();
    assert_eq!(app.modal_delete_tag_selected, 0);
}

#[test]
fn settings_shortcut_selection_opens_key_capture() {
    let key_selection = KeymapSelection::new(InputMode::Normal, KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let mut app = App {
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        settings_selected: 12,
        settings_selections: vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::KeyBinding(key_selection.clone()) }],
        ..Default::default()
    };

    app.on_select();

    assert_eq!(app.focus, Focus::ModalKeyCapture);
    assert_eq!(app.modal_key_capture_selection, Some(key_selection));
    assert_eq!(app.modal_key_capture_candidate, None);
    assert_eq!(app.modal_key_capture_error, None);
}

#[test]
fn settings_tab_commands_cycle_tabs_and_reset_selection() {
    let (_path, repo) = temp_repo("settings-tabs");
    let mut keymaps = minimal_keymaps();
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Tab, KeyModifiers::NONE), Command::FocusNextPane);
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::BackTab, KeyModifiers::SHIFT), Command::FocusPreviousPane);
    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Settings, focus: Focus::Viewport, settings_tab: SettingsTab::Paths, settings_selected: 99, keymaps, ..Default::default() };
    app.layout.graph = Rect::new(0, 0, 120, 40);
    app.layout.app = Rect::new(0, 0, 120, 40);
    app.settings_scroll.set(12);

    app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    assert_eq!(app.settings_tab, SettingsTab::Display);
    assert_eq!(app.settings_scroll.get(), 0);
    assert!(app.settings_selections.iter().any(|selection| selection.line == app.settings_selected));
    assert!(app.settings_selected > app.settings_tab_hitboxes.first().unwrap().line);

    app.handle_key_event(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));

    assert_eq!(app.settings_tab, SettingsTab::Paths);
    assert_eq!(app.settings_scroll.get(), 0);
    assert!(app.settings_selections.iter().any(|selection| selection.line == app.settings_selected));
    assert!(app.settings_selected > app.settings_tab_hitboxes.first().unwrap().line);
}

#[test]
fn toggle_help_opens_settings_on_paths_tab() {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, settings_tab: SettingsTab::Shortcuts, settings_selected: 42, ..Default::default() };
    app.settings_scroll.set(9);

    app.on_toggle_help();

    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.settings_tab, SettingsTab::Paths);
    assert_eq!(app.settings_selected, 0);
    assert_eq!(app.settings_scroll.get(), 0);
}

#[test]
fn settings_layout_command_toggles_and_stays_in_settings() {
    let mut app = App {
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        settings_selected: 12,
        settings_selections: vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::LayoutCommand(Command::ToggleBranches) }],
        ..Default::default()
    };
    app.layout_config.is_branches = true;
    app.settings_scroll.set(4);

    app.on_select();

    assert!(!app.layout_config.is_branches);
    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.settings_selected, 12);
    assert_eq!(app.settings_scroll.get(), 4);
}

#[test]
fn settings_submodule_layout_command_toggles_and_stays_in_settings() {
    let mut app = App {
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        settings_selected: 12,
        settings_selections: vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::LayoutCommand(Command::ToggleSubmodules) }],
        ..Default::default()
    };
    app.layout_config.is_submodules = false;
    app.settings_scroll.set(4);

    app.on_select();

    assert!(app.layout_config.is_submodules);
    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.settings_selected, 12);
    assert_eq!(app.settings_scroll.get(), 4);
}

#[test]
fn settings_reset_layout_command_resets_and_stays_in_settings() {
    let mut app = App {
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        settings_selected: 12,
        settings_selections: vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::LayoutCommand(Command::ResetLayout) }],
        ..Default::default()
    };
    app.layout_config.is_branches = false;
    app.layout_config.is_shas = false;

    app.on_select();

    assert!(app.layout_config.is_branches);
    assert!(app.layout_config.is_shas);
    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
}

#[test]
fn toggle_shas_shortcut_works_from_left_and_right_panes() {
    let mut keymaps = minimal_keymaps();
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('1'), KeyModifiers::SHIFT), Command::ToggleShas);
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Branches, keymaps, ..Default::default() };
    app.layout_config.is_shas = true;

    app.handle_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::SHIFT));
    assert!(!app.layout_config.is_shas);

    app.focus = Focus::StatusTop;
    app.handle_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::SHIFT));
    assert!(app.layout_config.is_shas);
}

#[test]
fn graph_metadata_shift_digit_shortcuts_toggle_display_flags() {
    let mut keymaps = minimal_keymaps();
    let normal = keymaps.get_mut(&InputMode::Normal).unwrap();
    normal.insert(KeyBinding::new(KeyCode::Char('2'), KeyModifiers::SHIFT), Command::ToggleGraphDates);
    normal.insert(KeyBinding::new(KeyCode::Char('3'), KeyModifiers::SHIFT), Command::ToggleGraphCommitters);
    normal.insert(KeyBinding::new(KeyCode::Char('4'), KeyModifiers::SHIFT), Command::ToggleGraphRefs);
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, keymaps, ..Default::default() };
    app.layout_config.is_graph_dates = false;
    app.layout_config.is_graph_committers = false;
    app.layout_config.is_graph_refs = true;

    app.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::SHIFT));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::SHIFT));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::SHIFT));

    assert!(app.layout_config.is_graph_dates);
    assert!(app.layout_config.is_graph_committers);
    assert!(!app.layout_config.is_graph_refs);
}

#[test]
fn graph_metadata_shifted_punctuation_aliases_toggle_display_flags() {
    let mut keymaps = minimal_keymaps();
    let normal = keymaps.get_mut(&InputMode::Normal).unwrap();
    normal.insert(KeyBinding::new(KeyCode::Char('0'), KeyModifiers::SHIFT), Command::ToggleGraphReflogs);
    normal.insert(KeyBinding::new(KeyCode::Char('1'), KeyModifiers::SHIFT), Command::ToggleShas);
    normal.insert(KeyBinding::new(KeyCode::Char('2'), KeyModifiers::SHIFT), Command::ToggleGraphDates);
    normal.insert(KeyBinding::new(KeyCode::Char('3'), KeyModifiers::SHIFT), Command::ToggleGraphCommitters);
    normal.insert(KeyBinding::new(KeyCode::Char('4'), KeyModifiers::SHIFT), Command::ToggleGraphRefs);
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, keymaps, ..Default::default() };
    app.layout_config.is_graph_reflogs = false;
    app.layout_config.is_shas = true;
    app.layout_config.is_graph_dates = false;
    app.layout_config.is_graph_committers = false;
    app.layout_config.is_graph_refs = true;

    app.handle_key_event(KeyEvent::new(KeyCode::Char(')'), KeyModifiers::NONE));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('#'), KeyModifiers::NONE));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('$'), KeyModifiers::NONE));

    assert!(app.layout_config.is_graph_reflogs);
    assert!(!app.layout_config.is_shas);
    assert!(app.layout_config.is_graph_dates);
    assert!(app.layout_config.is_graph_committers);
    assert!(!app.layout_config.is_graph_refs);
}

#[test]
fn graph_metadata_legacy_ctrl_digit_aliases_toggle_display_flags() {
    let mut keymaps = minimal_keymaps();
    let normal = keymaps.get_mut(&InputMode::Normal).unwrap();
    normal.insert(KeyBinding::new(KeyCode::Char('2'), KeyModifiers::SHIFT), Command::ToggleGraphDates);
    normal.insert(KeyBinding::new(KeyCode::Char('3'), KeyModifiers::SHIFT), Command::ToggleGraphCommitters);
    normal.insert(KeyBinding::new(KeyCode::Char('4'), KeyModifiers::SHIFT), Command::ToggleGraphRefs);
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, keymaps, ..Default::default() };
    app.layout_config.is_graph_dates = false;
    app.layout_config.is_graph_committers = false;
    app.layout_config.is_graph_refs = true;

    app.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::CONTROL));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::CONTROL));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::CONTROL));

    assert!(app.layout_config.is_graph_dates);
    assert!(app.layout_config.is_graph_committers);
    assert!(!app.layout_config.is_graph_refs);
}

#[test]
fn graph_reflog_shift_digit_shortcut_toggles_and_reloads() {
    let (path, repo) = temp_repo("graph-reflog-shortcut");
    commit_file(&repo, "head.txt", "head");
    let path = path.display().to_string();
    let mut keymaps = minimal_keymaps();
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('0'), KeyModifiers::SHIFT), Command::ToggleGraphReflogs);
    let mut app = App { path: Some(path.clone()), recent: vec![path], repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Branches, keymaps, ..Default::default() };

    app.handle_key_event(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::SHIFT));

    assert!(app.layout_config.is_graph_reflogs);
    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.graph_tx.is_some());
    stop_graph_service(&mut app);
}

#[test]
fn toggle_search_shortcut_opens_and_closes_search_pane() {
    let mut keymaps = minimal_keymaps();
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('7'), KeyModifiers::NONE), Command::ToggleSearch);
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, keymaps, ..Default::default() };
    app.layout_config.is_search = false;

    app.handle_key_event(KeyEvent::new(KeyCode::Char('7'), KeyModifiers::NONE));
    assert!(app.layout_config.is_search);
    assert_eq!(app.focus, Focus::Search);

    app.handle_key_event(KeyEvent::new(KeyCode::Char('7'), KeyModifiers::NONE));
    assert!(!app.layout_config.is_search);
    assert_eq!(app.focus, Focus::Viewport);
}

#[test]
fn toggle_submodules_shortcut_opens_and_closes_submodule_pane() {
    let mut keymaps = minimal_keymaps();
    keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('6'), KeyModifiers::NONE), Command::ToggleSubmodules);
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, keymaps, ..Default::default() };
    app.layout_config.is_submodules = false;

    app.handle_key_event(KeyEvent::new(KeyCode::Char('6'), KeyModifiers::NONE));
    assert!(app.layout_config.is_submodules);
    assert_eq!(app.focus, Focus::Submodules);

    app.handle_key_event(KeyEvent::new(KeyCode::Char('6'), KeyModifiers::NONE));
    assert!(!app.layout_config.is_submodules);
    assert_eq!(app.focus, Focus::Viewport);
}

#[test]
fn key_capture_conflict_does_not_change_keymaps() {
    let key_selection = KeymapSelection::new(InputMode::Normal, KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let keymaps = minimal_keymaps();
    let mut app = App { viewport: Viewport::Settings, focus: Focus::ModalKeyCapture, keymaps: keymaps.clone(), modal_key_capture_selection: Some(key_selection), ..Default::default() };

    app.handle_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::ModalKeyCapture);
    assert!(app.modal_key_capture_error.is_some());
    assert_eq!(app.keymaps, keymaps);
}

#[test]
fn key_capture_confirm_updates_memory_and_persists_keymap() {
    let path = temp_keymap_path("capture-save");
    let key_selection = KeymapSelection::new(InputMode::Normal, KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let mut app = App {
        viewport: Viewport::Settings,
        focus: Focus::ModalKeyCapture,
        keymaps: minimal_keymaps(),
        keymap_save_path: Some(path.clone()),
        modal_key_capture_selection: Some(key_selection),
        ..Default::default()
    };

    let new_key = KeyBinding::new(KeyCode::Char('n'), KeyModifiers::ALT);
    app.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::ALT));
    app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.keymaps.get(&InputMode::Normal).unwrap().get(&new_key), Some(&Command::ScrollDown));
    assert_eq!(app.keymaps.get(&InputMode::Action).unwrap().get(&new_key), Some(&Command::ScrollDown));
    assert_eq!(app.keymaps.get(&InputMode::Normal).unwrap().get(&KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE)), None);

    let loaded = load_keymaps_from_path(path.as_path()).unwrap();
    assert_eq!(loaded, app.keymaps);
}

#[test]
fn key_capture_can_assign_enter_key() {
    let path = temp_keymap_path("capture-enter");
    let key_selection = KeymapSelection::new(InputMode::Normal, KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let mut app = App {
        viewport: Viewport::Settings,
        focus: Focus::ModalKeyCapture,
        keymaps: minimal_keymaps(),
        keymap_save_path: Some(path.clone()),
        modal_key_capture_selection: Some(key_selection),
        ..Default::default()
    };

    let enter_key = KeyBinding::new(KeyCode::Enter, KeyModifiers::NONE);
    app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.focus, Focus::ModalKeyCapture);
    assert_eq!(app.modal_key_capture_candidate, Some(enter_key.clone()));

    app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.keymaps.get(&InputMode::Normal).unwrap().get(&enter_key), Some(&Command::ScrollDown));
    assert_eq!(load_keymaps_from_path(path.as_path()).unwrap(), app.keymaps);
}

#[test]
fn key_capture_esc_closes_without_capturing_key() {
    let key_selection = KeymapSelection::new(InputMode::Normal, KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let mut app = App { viewport: Viewport::Settings, focus: Focus::ModalKeyCapture, keymaps: minimal_keymaps(), modal_key_capture_selection: Some(key_selection), ..Default::default() };

    app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.modal_key_capture_selection, None);
    assert_eq!(app.modal_key_capture_candidate, None);
    assert_eq!(app.modal_key_capture_error, None);
}

#[test]
fn raw_esc_exits_settings_even_without_keymap_binding() {
    let mut app = App { viewport: Viewport::Settings, focus: Focus::Viewport, keymaps: Keymaps::new(), settings_selected: 12, settings_tab: SettingsTab::Shortcuts, ..Default::default() };

    app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.mode, InputMode::Normal);
}

#[test]
fn raw_esc_closes_choice_and_confirmation_modals() {
    let mut checkout = App { focus: Focus::ModalCheckout, modal_checkout_selected: 3, ..Default::default() };
    checkout.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(checkout.focus, Focus::Viewport);
    assert_eq!(checkout.modal_checkout_selected, 0);

    let mut solo = App { focus: Focus::ModalSolo, modal_solo_selected: 2, modal_branch_action: BranchModalAction::Rename, ..Default::default() };
    solo.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(solo.focus, Focus::Viewport);
    assert_eq!(solo.modal_solo_selected, 0);
    assert_eq!(solo.modal_branch_action, BranchModalAction::Solo);

    let mut delete_branch = App { focus: Focus::ModalDeleteBranch, modal_delete_branch_selected: 4, ..Default::default() };
    delete_branch.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(delete_branch.focus, Focus::Viewport);
    assert_eq!(delete_branch.modal_delete_branch_selected, 0);

    let mut delete_tag = App { focus: Focus::ModalDeleteTag, modal_delete_tag_selected: 5, ..Default::default() };
    delete_tag.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(delete_tag.focus, Focus::Viewport);
    assert_eq!(delete_tag.modal_delete_tag_selected, 0);
}

#[test]
fn raw_esc_closes_remote_action_modals() {
    let mut action = App { focus: Focus::ModalRemoteAction, modal_remote_selected: 3, modal_remote_target: Some("origin".into()), ..Default::default() };
    action.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(action.focus, Focus::Viewport);
    assert_eq!(action.modal_remote_selected, 0);
    assert_eq!(action.modal_remote_target, None);

    let mut delete = App { focus: Focus::ModalRemoteDelete, modal_remote_selected: 5, modal_remote_target: Some("origin".into()), ..Default::default() };
    delete.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(delete.focus, Focus::Viewport);
    assert_eq!(delete.modal_remote_selected, 0);
    assert_eq!(delete.modal_remote_target, None);
}

#[test]
fn raw_esc_does_not_dismiss_live_progress_modals() {
    let mut network = App { focus: Focus::ModalNetworkProgress, ..Default::default() };
    network.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(network.focus, Focus::ModalNetworkProgress);

    let mut operation = App { focus: Focus::ModalOperationProgress, ..Default::default() };
    operation.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(operation.focus, Focus::ModalOperationProgress);
}

#[test]
fn graph_branch_and_commit_jumps_refresh_current_diff() {
    let (mut app, _root_oid, _parent_oid, _child_oid) = graph_app_with_history();

    app.graph_selected = 3;
    app.current_diff.clear();
    app.on_scroll_up_branch();
    assert_eq!(app.graph_selected, 2);
    assert_eq!(diff_filenames(&app), vec!["parent.txt"]);

    app.current_diff.clear();
    app.on_scroll_up_commit();
    assert_eq!(app.graph_selected, 1);
    assert_eq!(diff_filenames(&app), vec!["child.txt"]);

    app.current_diff.clear();
    app.on_scroll_down_commit();
    assert_eq!(app.graph_selected, 2);
    assert_eq!(diff_filenames(&app), vec!["parent.txt"]);
}

#[test]
fn reflog_selection_refreshes_current_diff() {
    let (mut app, root_oid, _parent_oid, child_oid) = graph_app_with_history();
    let child_alias = app.oids.get_alias_by_oid(child_oid);
    let root_alias = app.oids.get_alias_by_oid(root_oid);
    app.focus = Focus::Reflogs;
    app.reflogs.entries.push(HeadReflogAliasEntry {
        selector: "HEAD@{0}".to_string(),
        old_oid: root_oid,
        new_oid: child_oid,
        new_alias: child_alias,
        message: "commit: child".to_string(),
        time: git2::Time::new(1, 0),
    });
    app.graph_selected = 3;
    app.current_diff.clear();

    app.on_select();

    assert_eq!(app.graph_selected, 1);
    assert_eq!(app.oids.get_alias_by_idx(3), root_alias);
    assert_eq!(diff_filenames(&app), vec!["child.txt"]);
}
