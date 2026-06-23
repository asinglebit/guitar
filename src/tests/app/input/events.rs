use super::*;
use crate::{
    app::{
        app::{ContextMenuAction, SettingsSelection, SettingsSelectionKind, SettingsTab},
        state::defaults::ViewerMode,
        state::layout::Layout,
    },
    core::{
        chunk::NONE,
        reflogs::HeadReflogAliasEntry,
        submodules::SubmoduleEntry,
        worktrees::{WorktreeEntry, WorktreeKind},
    },
    helpers::{
        keymap::{Command, InputMode, KeyBinding, KeymapSelection},
        layout::LayoutConfig,
    },
};
use git2::{Oid, Repository, Signature};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    widgets::ListItem,
};
use std::{
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-input-events-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn left_down(column: u16, row: u16) -> MouseEvent {
    MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, modifiers: KeyModifiers::NONE }
}

fn left_drag(column: u16, row: u16) -> MouseEvent {
    MouseEvent { kind: MouseEventKind::Drag(MouseButton::Left), column, row, modifiers: KeyModifiers::NONE }
}

fn left_up(column: u16, row: u16) -> MouseEvent {
    MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, modifiers: KeyModifiers::NONE }
}

fn right_down(column: u16, row: u16) -> MouseEvent {
    MouseEvent { kind: MouseEventKind::Down(MouseButton::Right), column, row, modifiers: KeyModifiers::NONE }
}

fn graph_app() -> App {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.app = Rect::new(0, 0, 30, 8);
    app.layout.graph = Rect::new(0, 0, 30, 8);
    app.graph.total = 20;
    app
}

fn context_menu_app() -> App {
    let mut app = graph_app();
    app.layout.app = Rect::new(0, 0, 80, 24);
    app.layout.graph = Rect::new(0, 0, 80, 24);
    app.graph.total = 0;
    app
}

fn context_menu_labels(app: &App) -> Vec<String> {
    app.context_menu.as_ref().map(|menu| menu.items.iter().map(|item| item.label.clone()).collect()).unwrap_or_default()
}

#[test]
fn right_click_opens_context_menu_with_first_enabled_item() {
    let mut app = context_menu_app();

    app.handle_mouse_event(right_down(5, 3));

    let menu = app.context_menu.unwrap();
    assert_eq!((menu.column, menu.row), (5, 3));
    assert_eq!(menu.items[menu.selected].action, ContextMenuAction::Splash);
}

#[test]
fn right_click_does_not_open_context_menu_over_modal_focus() {
    let mut app = context_menu_app();
    app.focus = Focus::ModalCommit;

    app.handle_mouse_event(right_down(5, 3));

    assert!(app.context_menu.is_none());
}

#[test]
fn modal_esc_corner_click_closes_dismissible_modal_like_escape() {
    let mut app = graph_app();
    app.focus = Focus::ModalCommit;
    app.modal_area = Some(Rect::new(10, 4, 40, 12));
    app.modal_input.set_value("draft message");

    app.handle_mouse_event(left_down(45, 4));

    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.modal_input.value().is_empty());
}

#[test]
fn modal_esc_corner_click_closes_error_modal_like_escape() {
    let mut app = graph_app();
    app.focus = Focus::ModalError;
    app.modal_area = Some(Rect::new(10, 4, 40, 12));
    app.modal_error_return_focus = Focus::Branches;
    app.modal_error_message = "boom".into();

    app.handle_mouse_event(left_down(45, 4));

    assert_eq!(app.focus, Focus::Branches);
    assert!(app.modal_error_message.is_empty());
}

#[test]
fn modal_esc_corner_click_does_not_dismiss_progress_modal() {
    let mut app = graph_app();
    app.focus = Focus::ModalOperationProgress;
    app.modal_area = Some(Rect::new(10, 4, 40, 12));

    app.handle_mouse_event(left_down(45, 4));

    assert_eq!(app.focus, Focus::ModalOperationProgress);
}

#[test]
fn modal_left_click_outside_esc_corner_is_consumed() {
    let mut app = graph_app();
    app.focus = Focus::ModalCommit;
    app.modal_area = Some(Rect::new(10, 4, 40, 12));
    app.graph_selected = 4;

    app.handle_mouse_event(left_down(1, 1));

    assert_eq!(app.focus, Focus::ModalCommit);
    assert_eq!(app.graph_selected, 4);
}

#[test]
fn left_click_outside_context_menu_dismisses_without_selecting_underlying_row() {
    let mut app = context_menu_app();
    app.graph_selected = 4;

    app.handle_mouse_event(right_down(20, 10));
    app.handle_mouse_event(left_down(1, 1));

    assert!(app.context_menu.is_none());
    assert_eq!(app.graph_selected, 4);
}

#[test]
fn left_click_context_menu_settings_opens_settings_when_repo_exists() {
    let (_path, repo) = temp_repo("context-menu-settings");
    let mut app = context_menu_app();
    app.repo = Some(Rc::new(repo));

    app.handle_mouse_event(right_down(5, 5));
    app.handle_mouse_event(left_down(6, 11));

    assert!(app.context_menu.is_none());
    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
}

#[test]
fn repo_context_menu_includes_reload_global_action() {
    let (_path, repo) = temp_repo("context-menu-reload");
    let mut app = context_menu_app();
    app.repo = Some(Rc::new(repo));

    app.handle_mouse_event(right_down(5, 5));

    let labels = context_menu_labels(&app);
    assert_eq!(labels.iter().map(String::as_str).collect::<Vec<_>>(), vec!["Reload", "", "", "", "Settings", "Splash screen", "Exit"]);
}

#[test]
fn context_menu_omits_reload_without_loaded_repo() {
    let mut app = context_menu_app();

    app.handle_mouse_event(right_down(5, 5));

    let labels = context_menu_labels(&app);
    assert!(!labels.iter().any(|label| label == "Reload"), "{labels:?}");
}

#[test]
fn left_click_context_menu_disabled_settings_keeps_menu_open() {
    let mut app = context_menu_app();

    app.handle_mouse_event(right_down(5, 5));
    app.handle_mouse_event(left_down(6, 7));

    assert!(app.context_menu.is_some());
    assert_eq!(app.viewport, Viewport::Graph);
}

#[test]
fn context_menu_keyboard_navigation_and_enter_activate_selected_item() {
    let mut app = context_menu_app();

    app.handle_mouse_event(right_down(5, 5));
    app.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    let menu = app.context_menu.as_ref().unwrap();
    assert_eq!(menu.items[menu.selected].action, ContextMenuAction::Exit);

    app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.context_menu.is_none());
    assert!(app.is_exit);
}

#[test]
fn right_click_selects_graph_row_and_opens_contextual_actions() {
    let mut app = graph_app();
    app.graph_scroll.set(2);

    app.handle_mouse_event(right_down(1, 3));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.graph_selected, 5);
    let labels = context_menu_labels(&app);
    assert!(labels.iter().any(|label| label == "Show details"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Create branch"), "{labels:?}");
}

#[test]
fn graph_context_menu_includes_fetch_and_push_when_repo_is_loaded() {
    let (_path, repo) = temp_repo("graph-network-menu");
    let mut app = graph_app();
    app.repo = Some(Rc::new(repo));
    app.graph_scroll.set(2);

    app.handle_mouse_event(right_down(1, 3));

    let labels = context_menu_labels(&app);
    assert!(labels.iter().any(|label| label == "Fetch"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Push"), "{labels:?}");
}

#[test]
fn graph_context_menu_groups_regular_action_mode_and_navigation_sections() {
    let (_path, repo) = temp_repo("graph-section-menu");
    let mut app = graph_app();
    app.repo = Some(Rc::new(repo));
    app.graph_scroll.set(2);

    app.handle_mouse_event(right_down(1, 3));

    let items = &app.context_menu.as_ref().unwrap().items;
    let divider_indices = items.iter().enumerate().filter_map(|(index, item)| matches!(item.action, ContextMenuAction::Divider).then_some(index)).collect::<Vec<_>>();
    assert_eq!(divider_indices.len(), 2, "{items:?}");

    let labels = context_menu_labels(&app);
    let first_divider = divider_indices[0];
    let second_divider = divider_indices[1];
    assert!(labels[..first_divider].contains(&"Fetch".to_string()), "{labels:?}");
    assert!(!labels[..first_divider].contains(&"Push".to_string()), "{labels:?}");
    assert!(labels[first_divider + 1..second_divider].contains(&"Push".to_string()), "{labels:?}");
    assert!(labels[first_divider + 1..second_divider].contains(&"Checkout".to_string()), "{labels:?}");
    assert_eq!(&labels[second_divider + 2..], &["Settings".to_string(), "Splash screen".to_string(), "Exit".to_string()]);
}

#[test]
fn uncommitted_graph_context_menu_includes_fetch_and_push_when_repo_is_loaded() {
    let (_path, repo) = temp_repo("uncommitted-network-menu");
    let mut app = graph_app();
    app.repo = Some(Rc::new(repo));

    app.handle_mouse_event(right_down(1, 0));

    assert_eq!(app.graph_selected, 0);
    let labels = context_menu_labels(&app);
    assert!(labels.iter().any(|label| label == "Fetch"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Push"), "{labels:?}");
}

#[test]
fn right_click_selects_left_pane_row_and_opens_contextual_actions() {
    let mut app = graph_app();
    app.layout_config.is_branches = true;
    app.layout.branches = Rect::new(0, 0, 20, 6);
    app.branches.sorted = (0..10).map(|idx| (idx, format!("branch-{idx}"))).collect();
    app.branches_scroll.set(2);

    app.handle_mouse_event(right_down(1, 1));

    assert_eq!((app.focus, app.branches_selected), (Focus::Branches, 3));
    let labels = context_menu_labels(&app);
    assert!(labels.iter().any(|label| label == "Open commit"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Checkout branch"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Toggle branch"), "{labels:?}");
}

#[test]
fn right_click_selects_status_row_and_opens_contextual_actions() {
    let mut app = graph_app();
    app.layout_config.is_status = true;
    app.layout.status_top = Rect::new(0, 0, 30, 6);
    app.is_uncommitted_loaded = true;
    app.uncommitted.is_staged = true;
    app.uncommitted.staged.modified = vec!["a".into(), "b".into(), "c".into(), "d".into()];
    app.status_top_scroll.set(1);

    app.handle_mouse_event(right_down(1, 1));

    assert_eq!((app.focus, app.status_top_selected), (Focus::StatusTop, 2));
    let labels = context_menu_labels(&app);
    assert!(labels.iter().any(|label| label == "Open file"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Unstage file"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Discard file changes"), "{labels:?}");
}

#[test]
fn right_click_selects_splash_recent_repo_and_opens_contextual_actions() {
    let mut app = App { viewport: Viewport::Splash, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.app = Rect::new(0, 0, 120, 20);
    app.layout.graph = Rect::new(0, 0, 120, 20);
    app.recent = vec!["/repo/a".into(), "/repo/b".into(), "/repo/c".into()];

    app.handle_mouse_event(right_down(1, 17));

    assert_eq!(app.splash_selected, 1);
    let labels = context_menu_labels(&app);
    assert!(labels.iter().any(|label| label == "Open repository"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Move up"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Move down"), "{labels:?}");
    assert!(!labels.iter().any(|label| label == "Back"), "{labels:?}");
    assert!(!labels.iter().any(|label| label == "Reload"), "{labels:?}");
    assert!(!labels.iter().any(|label| label == "Splash screen"), "{labels:?}");
    assert!(labels.iter().position(|label| label.is_empty()).is_some_and(|index| index < labels.iter().position(|label| label == "Exit").unwrap()), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Exit"), "{labels:?}");
}

#[test]
fn splash_context_menu_back_returns_to_graph_when_repo_loaded() {
    let (_path, repo) = temp_repo("splash-context-back");
    let mut app = App { viewport: Viewport::Splash, focus: Focus::Viewport, repo: Some(Rc::new(repo)), layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.app = Rect::new(0, 0, 80, 24);
    app.layout.graph = Rect::new(0, 0, 80, 24);

    app.handle_mouse_event(right_down(60, 20));

    let labels = context_menu_labels(&app);
    assert!(labels.iter().any(|label| label == "Back"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Reload"), "{labels:?}");
    assert!(!labels.iter().any(|label| label == "Splash screen"), "{labels:?}");
    let back_index = labels.iter().position(|label| label == "Back").unwrap();
    app.context_menu.as_mut().unwrap().selected = back_index;

    app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.context_menu.is_none());
}

#[test]
fn right_click_selects_settings_row_and_opens_contextual_actions() {
    let (_path, repo) = temp_repo("settings-context-reload");
    let mut app = App { viewport: Viewport::Settings, focus: Focus::Viewport, repo: Some(Rc::new(repo)), layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.graph = Rect::new(0, 0, 40, 5);
    app.settings_scroll.set(10);
    app.settings_selections = vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::Theme(0) }];

    app.handle_mouse_event(right_down(1, 2));

    assert_eq!(app.settings_selected, 12);
    let labels = context_menu_labels(&app);
    assert!(labels.iter().any(|label| label == "Apply theme"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Back"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Reload"), "{labels:?}");
    assert!(!labels.iter().any(|label| label == "Settings"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Splash screen"), "{labels:?}");
    assert!(labels.iter().any(|label| label == "Exit"), "{labels:?}");
    assert_eq!(labels.iter().map(String::as_str).collect::<Vec<_>>(), vec!["Reload", "Apply theme", "", "", "", "Back", "Splash screen", "Exit"]);
}

#[test]
fn settings_context_menu_back_returns_to_graph() {
    let mut app = App { viewport: Viewport::Settings, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.app = Rect::new(0, 0, 80, 24);
    app.layout.graph = Rect::new(0, 0, 80, 24);

    app.handle_mouse_event(right_down(60, 20));

    let labels = context_menu_labels(&app);
    assert_eq!(labels.first().map(String::as_str), Some("Back"), "{labels:?}");

    app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.context_menu.is_none());
}

fn test_oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).unwrap()
}

fn commit_file(repo: &Repository, file: &str, message: &str) -> Oid {
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

fn worktree_entry(name: &str) -> WorktreeEntry {
    WorktreeEntry {
        name: name.to_string(),
        path: PathBuf::from(format!("/tmp/{name}")),
        branch: Some(name.to_string()),
        head: Some(test_oid(9)),
        alias: None,
        kind: WorktreeKind::Linked,
        is_current: false,
        is_valid: true,
        is_prunable: false,
        locked_reason: None,
        is_dirty: false,
    }
}

fn submodule_entry(name: &str, absolute_path: PathBuf) -> SubmoduleEntry {
    SubmoduleEntry {
        name: name.to_string(),
        path: PathBuf::from(name),
        absolute_path,
        url: None,
        branch: Some("main".into()),
        head: None,
        index: None,
        workdir: None,
        is_open: true,
        is_uninitialized: false,
        is_in_head: true,
        is_in_index: true,
        is_in_config: true,
        is_in_workdir: true,
        is_index_modified: false,
        is_workdir_modified: false,
        has_new_commits: false,
        has_modified_content: false,
        has_untracked_content: false,
    }
}

#[test]
fn mouse_click_selects_graph_row() {
    let mut app = graph_app();
    app.graph_scroll.set(2);

    app.handle_mouse_event(left_down(1, 3));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.graph_selected, 5);
}

#[test]
fn mouse_click_on_graph_scrollbar_track_jumps_and_focuses() {
    let mut app = graph_app();
    app.layout.graph_scrollbar = Rect::new(29, 0, 1, 8);
    app.graph.total = 100;

    app.handle_mouse_event(left_down(29, 4));
    app.handle_mouse_event(left_up(29, 4));

    let max_scroll = app.graph_commit_count().saturating_sub(app.layout.graph.height as usize);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Graph);
    assert!(app.graph_scroll.get() > 0);
    assert!(app.graph_scroll.get() < max_scroll);
    assert!(app.graph_selected >= app.graph_scroll.get());
    assert!(app.graph_selected < app.graph_scroll.get() + app.layout.graph.height as usize);
}

#[test]
fn mouse_drag_on_graph_scrollbar_clamps_to_top_and_bottom() {
    let mut app = graph_app();
    app.layout.graph_scrollbar = Rect::new(29, 0, 1, 8);
    app.graph.total = 100;
    let max_scroll = app.graph_commit_count().saturating_sub(app.layout.graph.height as usize);

    app.handle_mouse_event(left_down(29, 1));
    app.handle_mouse_event(left_drag(29, 99));
    assert_eq!(app.graph_scroll.get(), max_scroll);
    assert!(app.graph_selected >= app.graph_scroll.get());

    app.handle_mouse_event(left_drag(29, 0));
    app.handle_mouse_event(left_up(29, 0));
    assert_eq!(app.graph_scroll.get(), 0);
    assert!(app.graph_selected < app.layout.graph.height as usize);
}

#[test]
fn mouse_drag_on_left_pane_scrollbar_scrolls_and_focuses_pane() {
    let mut app = graph_app();
    app.layout_config.is_branches = true;
    app.layout.branches = Rect::new(0, 0, 20, 8);
    app.layout.branches_scrollbar = Rect::new(0, 0, 20, 8);
    app.branches.sorted = (0..80).map(|idx| (idx, format!("branch-{idx}"))).collect();

    app.handle_mouse_event(left_down(19, 5));
    app.handle_mouse_event(left_up(19, 5));

    assert_eq!(app.focus, Focus::Branches);
    assert!(app.branches_scroll.get() > 0);
    assert!(app.branches_selected >= app.branches_scroll.get());
}

#[test]
fn mouse_scrollbars_work_for_inspector_and_status_panes() {
    let mut inspector = graph_app();
    inspector.layout_config.is_inspector = true;
    inspector.uncommitted.has_conflicts = true;
    inspector.layout.inspector = Rect::new(0, 0, 40, 5);
    inspector.layout.inspector_scrollbar = Rect::new(0, 0, 40, 5);

    inspector.handle_mouse_event(left_down(39, 3));
    inspector.handle_mouse_event(left_up(39, 3));

    assert_eq!(inspector.focus, Focus::Inspector);
    assert!(inspector.inspector_scroll.get() > 0);

    let mut status = graph_app();
    status.layout_config.is_status = true;
    status.layout.status_top = Rect::new(0, 0, 40, 5);
    status.layout.status_top_scrollbar = Rect::new(0, 0, 40, 5);
    status.is_uncommitted_loaded = true;
    status.uncommitted.is_staged = true;
    status.uncommitted.staged.modified = (0..20).map(|idx| format!("file-{idx}.rs")).collect();

    status.handle_mouse_event(left_down(39, 3));
    status.handle_mouse_event(left_up(39, 3));

    assert_eq!(status.focus, Focus::StatusTop);
    assert!(status.status_top_scroll.get() > 0);

    let mut bottom_status = graph_app();
    bottom_status.layout_config.is_status = true;
    bottom_status.layout.status_top = Rect::new(0, 0, 40, 3);
    bottom_status.layout.status_top_scrollbar = Rect::new(0, 0, 40, 3);
    bottom_status.layout.status_bottom = Rect::new(0, 2, 40, 5);
    bottom_status.layout.status_bottom_scrollbar = Rect::new(0, 2, 40, 5);
    bottom_status.is_uncommitted_loaded = true;
    bottom_status.is_uncommitted_detail_loaded = true;
    bottom_status.uncommitted.is_unstaged = true;
    bottom_status.uncommitted.unstaged.modified = (0..20).map(|idx| format!("file-{idx}.rs")).collect();

    bottom_status.handle_mouse_event(left_down(39, 5));
    bottom_status.handle_mouse_event(left_up(39, 5));

    assert_eq!(bottom_status.focus, Focus::StatusBottom);
    assert!(bottom_status.status_bottom_scroll.get() > 0);
}

#[test]
fn mouse_click_on_settings_scrollbar_scrolls_settings() {
    let (_path, repo) = temp_repo("settings-scrollbar");
    let mut app = App { viewport: Viewport::Settings, focus: Focus::Viewport, repo: Some(Rc::new(repo)), layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.app = Rect::new(0, 0, 90, 10);
    app.layout.graph = Rect::new(0, 0, 90, 10);

    app.handle_mouse_event(left_down(89, 8));
    app.handle_mouse_event(left_up(89, 8));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Settings);
    assert!(app.settings_scroll.get() > 0);
    assert!(app.settings_selections.iter().any(|selection| selection.line == app.settings_selected));
}

#[test]
fn non_scrollable_or_too_short_scrollbars_fall_back_to_row_clicks() {
    let mut non_scrollable = graph_app();
    non_scrollable.layout.graph_scrollbar = Rect::new(29, 0, 1, 8);
    non_scrollable.graph.total = 5;

    non_scrollable.handle_mouse_event(left_down(29, 3));

    assert_eq!(non_scrollable.graph_scroll.get(), 0);
    assert_eq!(non_scrollable.graph_selected, 3);

    let mut too_short = graph_app();
    too_short.layout.graph_scrollbar = Rect::new(29, 0, 1, 2);
    too_short.graph.total = 100;

    too_short.handle_mouse_event(left_down(29, 1));

    assert_eq!(too_short.graph_scroll.get(), 0);
    assert_eq!(too_short.graph_selected, 1);
}

#[test]
fn mouse_click_selects_viewer_row_without_leaving_viewer() {
    let mut app = graph_app();
    app.viewport = Viewport::Viewer;
    app.viewer_lines = (0..10).map(|idx| ListItem::new(format!("line {idx}"))).collect();
    app.viewer_scroll.set(2);

    app.handle_mouse_event(left_down(1, 3));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Viewer);
    assert_eq!(app.viewer_selected, 5);

    app.handle_mouse_event(left_down(1, 3));

    assert_eq!(app.viewport, Viewport::Viewer);
    assert_eq!(app.viewer_selected, 5);
}

#[test]
fn mouse_click_selects_left_pane_rows() {
    let mut app = graph_app();
    app.layout_config.is_branches = true;
    app.layout_config.is_tags = true;
    app.layout_config.is_stashes = true;
    app.layout_config.is_reflogs = true;
    app.layout_config.is_worktrees = true;
    app.layout_config.is_submodules = true;
    app.layout.branches = Rect::new(0, 0, 20, 6);
    app.layout.tags = Rect::new(20, 0, 20, 6);
    app.layout.stashes = Rect::new(40, 0, 20, 6);
    app.layout.reflogs = Rect::new(60, 0, 20, 6);
    app.layout.worktrees = Rect::new(80, 0, 20, 6);
    app.layout.submodules = Rect::new(100, 0, 20, 6);
    app.branches.sorted = (0..10).map(|idx| (idx, format!("branch-{idx}"))).collect();
    app.tags.sorted = (0..10).map(|idx| (idx, format!("tag-{idx}"))).collect();
    app.oids.stashes = (0..10).collect();
    app.reflogs.entries = (0..10)
        .map(|idx| HeadReflogAliasEntry {
            selector: format!("HEAD@{{{idx}}}"),
            old_oid: test_oid(1),
            new_oid: test_oid(2),
            new_alias: idx,
            message: format!("commit {idx}"),
            time: git2::Time::new(idx as i64, 0),
        })
        .collect();
    app.worktrees.entries = (0..10).map(|idx| worktree_entry(&format!("worktree-{idx}"))).collect();
    app.submodules.entries = (0..10).map(|idx| submodule_entry(&format!("submodule-{idx}"), PathBuf::from(format!("/tmp/submodule-{idx}")))).collect();
    app.branches_scroll.set(2);
    app.tags_scroll.set(2);
    app.stashes_scroll.set(2);
    app.reflogs_scroll.set(2);
    app.worktrees_scroll.set(2);
    app.submodules_scroll.set(2);

    app.handle_mouse_event(left_down(1, 1));
    assert_eq!((app.focus, app.branches_selected), (Focus::Branches, 3));

    app.handle_mouse_event(left_down(21, 1));
    assert_eq!((app.focus, app.tags_selected), (Focus::Tags, 3));

    app.handle_mouse_event(left_down(41, 1));
    assert_eq!((app.focus, app.stashes_selected), (Focus::Stashes, 3));

    app.handle_mouse_event(left_down(61, 1));
    assert_eq!((app.focus, app.reflogs_selected), (Focus::Reflogs, 3));

    app.handle_mouse_event(left_down(81, 1));
    assert_eq!((app.focus, app.worktrees_selected), (Focus::Worktrees, 3));

    app.handle_mouse_event(left_down(101, 1));
    assert_eq!((app.focus, app.submodules_selected), (Focus::Submodules, 3));
}

#[test]
fn mouse_click_selects_status_rows() {
    let mut app = graph_app();
    app.layout_config.is_status = true;
    app.layout.status_top = Rect::new(0, 0, 30, 6);
    app.layout.status_bottom = Rect::new(0, 10, 30, 6);
    app.is_uncommitted_loaded = true;
    app.is_uncommitted_detail_loaded = true;
    app.uncommitted.is_staged = true;
    app.uncommitted.is_unstaged = true;
    app.uncommitted.staged.modified = vec!["a".into(), "b".into(), "c".into(), "d".into()];
    app.uncommitted.unstaged.modified = vec!["e".into(), "f".into(), "g".into(), "h".into()];
    app.status_top_scroll.set(1);
    app.status_bottom_scroll.set(1);

    app.handle_mouse_event(left_down(1, 1));
    assert_eq!((app.focus, app.status_top_selected), (Focus::StatusTop, 2));

    app.handle_mouse_event(left_down(1, 12));
    assert_eq!((app.focus, app.status_bottom_selected), (Focus::StatusBottom, 2));
}

#[test]
fn mouse_click_selects_inspector_row_when_loaded() {
    let mut app = graph_app();
    let oid = test_oid(5);
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];
    app.graph_selected = 1;
    app.layout_config.is_inspector = true;
    app.layout.inspector = Rect::new(0, 0, 30, 6);
    app.inspector_scroll.set(2);

    app.handle_mouse_event(left_down(1, 1));

    assert_eq!((app.focus, app.inspector_selected), (Focus::Inspector, 3));
}

#[test]
fn mouse_click_selects_splash_recent_repo() {
    let mut app = App { viewport: Viewport::Splash, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.app = Rect::new(0, 0, 120, 20);
    app.layout.graph = Rect::new(0, 0, 120, 20);
    app.recent = vec!["/repo/a".into(), "/repo/b".into(), "/repo/c".into()];

    app.handle_mouse_event(left_down(1, 17));

    assert_eq!(app.splash_selected, 1);
}

#[test]
fn mouse_click_selects_only_selectable_settings_rows() {
    let mut app = App { viewport: Viewport::Settings, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.graph = Rect::new(0, 0, 40, 5);
    app.settings_scroll.set(10);
    app.settings_selected = 12;
    app.settings_selections = vec![SettingsSelection { line: 12, kind: SettingsSelectionKind::Info }];

    app.handle_mouse_event(left_down(1, 2));
    assert_eq!(app.settings_selected, 12);

    app.handle_mouse_event(left_down(1, 3));
    assert_eq!(app.settings_selected, 12);
}

#[test]
fn mouse_click_on_settings_tab_switches_active_tab() {
    let (_path, repo) = temp_repo("settings-tab-click");
    let repo = Rc::new(repo);
    let mut app = App {
        repo: Some(repo.clone()),
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        layout_config: LayoutConfig::default(),
        layout: Layout::default(),
        settings_tab: SettingsTab::General,
        settings_selected: 99,
        ..Default::default()
    };
    app.layout.app = Rect::new(0, 0, 120, 40);
    app.layout.graph = Rect::new(0, 0, 120, 40);
    app.settings_scroll.set(8);
    app.settings_lines(&repo);
    let hitbox = *app.settings_tab_hitboxes.iter().find(|hitbox| hitbox.tab == SettingsTab::Repo).unwrap();
    let column = hitbox.start.saturating_add(hitbox.end.saturating_sub(hitbox.start) / 2);
    let row = hitbox.line.saturating_sub(app.settings_scroll.get()) as u16;

    app.handle_mouse_event(left_down(column, row));

    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.settings_tab, SettingsTab::Repo);
    assert_eq!(app.settings_scroll.get(), 0);
    assert!(app.settings_selections.iter().any(|selection| selection.line == app.settings_selected));
    assert!(app.settings_selected > app.settings_tab_hitboxes.first().unwrap().line);
}

#[test]
fn mouse_single_click_on_settings_layout_row_toggles_once() {
    let mut app = App { viewport: Viewport::Settings, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.graph = Rect::new(0, 0, 40, 5);
    app.layout_config.is_branches = true;
    app.settings_selections = vec![SettingsSelection { line: 2, kind: SettingsSelectionKind::LayoutCommand(Command::ToggleBranches) }];

    app.handle_mouse_event(left_down(1, 2));
    assert!(!app.layout_config.is_branches);
    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.settings_selected, 2);

    app.handle_mouse_event(left_down(1, 2));
    assert!(!app.layout_config.is_branches);
}

#[test]
fn double_click_on_settings_selectable_row_acts_like_enter() {
    let key_selection = KeymapSelection::new(InputMode::Normal, KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let mut app = App { viewport: Viewport::Settings, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.graph = Rect::new(0, 0, 40, 5);
    app.settings_selections = vec![SettingsSelection { line: 2, kind: SettingsSelectionKind::KeyBinding(key_selection.clone()) }];

    app.handle_mouse_event(left_down(1, 2));
    app.handle_mouse_event(left_down(1, 2));

    assert_eq!(app.focus, Focus::ModalKeyCapture);
    assert_eq!(app.modal_key_capture_selection, Some(key_selection));
}

#[test]
fn double_click_on_settings_recent_repository_row_is_selection_only() {
    let mut app = App { viewport: Viewport::Settings, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.graph = Rect::new(0, 0, 40, 5);
    app.recent = vec!["/repo/a".into()];
    app.settings_selections = vec![SettingsSelection { line: 2, kind: SettingsSelectionKind::RecentRepository(0) }];

    app.handle_mouse_event(left_down(1, 2));
    app.handle_mouse_event(left_down(1, 2));

    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.settings_selected, 2);
    assert!(app.repo.is_none());
}

#[test]
fn double_click_on_branch_row_acts_like_enter() {
    let (_path, repo) = temp_repo("branch-double");
    let mut app = graph_app();
    let oid = commit_file(&repo, "feature.txt", "feature");
    app.repo = Some(Rc::new(repo));
    app.layout_config.is_branches = true;
    app.layout.branches = Rect::new(0, 0, 20, 6);
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];
    app.branches.sorted = vec![(alias, "feature".into())];

    app.handle_mouse_event(left_down(1, 0));
    app.handle_mouse_event(left_down(1, 0));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.graph_selected, 1);
}

#[test]
fn viewer_mode_double_click_on_branch_row_acts_like_enter() {
    let (_path, repo) = temp_repo("viewer-branch-double");
    let mut app = graph_app();
    let oid = commit_file(&repo, "feature.txt", "feature");
    app.repo = Some(Rc::new(repo));
    app.viewport = Viewport::Viewer;
    app.layout_config.is_branches = true;
    app.layout.branches = Rect::new(0, 0, 20, 6);
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];
    app.branches.sorted = vec![(alias, "feature".into())];

    app.handle_mouse_event(left_down(1, 0));
    app.handle_mouse_event(left_down(1, 0));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.graph_selected, 1);
}

#[test]
fn double_click_on_tag_stash_and_reflog_rows_act_like_enter() {
    let (_path, repo) = temp_repo("pane-double");
    let tag_oid = commit_file(&repo, "tag.txt", "tag");
    let stash_oid = commit_file(&repo, "stash.txt", "stash");
    let reflog_oid = commit_file(&repo, "reflog.txt", "reflog");
    let repo = Rc::new(repo);

    let mut tag_app = graph_app();
    tag_app.repo = Some(repo.clone());
    tag_app.layout_config.is_tags = true;
    tag_app.layout.tags = Rect::new(0, 0, 20, 6);
    let tag_alias = tag_app.oids.get_alias_by_oid(tag_oid);
    tag_app.oids.sorted_aliases = vec![NONE, tag_alias];
    tag_app.tags.sorted = vec![(tag_alias, "v1.0.0".into())];

    tag_app.handle_mouse_event(left_down(1, 0));
    tag_app.handle_mouse_event(left_down(1, 0));
    assert_eq!((tag_app.focus, tag_app.graph_selected), (Focus::Viewport, 1));

    let mut stash_app = graph_app();
    stash_app.repo = Some(repo.clone());
    stash_app.layout_config.is_stashes = true;
    stash_app.layout.stashes = Rect::new(0, 0, 20, 6);
    let stash_alias = stash_app.oids.get_alias_by_oid(stash_oid);
    stash_app.oids.sorted_aliases = vec![NONE, stash_alias];
    stash_app.oids.stashes = vec![stash_alias];

    stash_app.handle_mouse_event(left_down(1, 0));
    stash_app.handle_mouse_event(left_down(1, 0));
    assert_eq!((stash_app.focus, stash_app.graph_selected), (Focus::Viewport, 1));

    let mut reflog_app = graph_app();
    reflog_app.repo = Some(repo);
    reflog_app.layout_config.is_reflogs = true;
    reflog_app.layout.reflogs = Rect::new(0, 0, 20, 6);
    let reflog_alias = reflog_app.oids.get_alias_by_oid(reflog_oid);
    reflog_app.oids.sorted_aliases = vec![NONE, reflog_alias];
    reflog_app.reflogs.entries =
        vec![HeadReflogAliasEntry { selector: "HEAD@{0}".into(), old_oid: tag_oid, new_oid: reflog_oid, new_alias: reflog_alias, message: "commit reflog".into(), time: git2::Time::new(0, 0) }];

    reflog_app.handle_mouse_event(left_down(1, 0));
    reflog_app.handle_mouse_event(left_down(1, 0));
    assert_eq!((reflog_app.focus, reflog_app.graph_selected), (Focus::Viewport, 1));
}

#[test]
fn double_click_on_worktree_row_acts_like_enter() {
    let (current_path, repo) = temp_repo("worktree-current");
    let (target_path, _target_repo) = temp_repo("worktree-target");
    let mut app = graph_app();
    app.repo = Some(Rc::new(repo));
    app.path = Some(current_path.display().to_string());
    let canonical_target = fs::canonicalize(&target_path).unwrap().display().to_string();
    app.recent = vec![canonical_target.clone()];
    app.layout_config.is_worktrees = true;
    app.layout.worktrees = Rect::new(0, 0, 20, 6);
    app.worktrees.entries = vec![WorktreeEntry {
        name: "linked".into(),
        path: target_path.clone(),
        branch: Some("main".into()),
        head: None,
        alias: None,
        kind: WorktreeKind::Linked,
        is_current: false,
        is_valid: true,
        is_prunable: false,
        locked_reason: None,
        is_dirty: false,
    }];

    app.handle_mouse_event(left_down(1, 0));
    app.handle_mouse_event(left_down(1, 0));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.graph_selected, 0);
    assert_eq!(app.path.as_deref(), Some(canonical_target.as_str()));
}

#[test]
fn double_click_on_submodule_row_acts_like_enter() {
    let (current_path, repo) = temp_repo("submodule-current");
    let (target_path, _target_repo) = temp_repo("submodule-target");
    let mut app = graph_app();
    app.repo = Some(Rc::new(repo));
    app.path = Some(current_path.display().to_string());
    let canonical_target = fs::canonicalize(&target_path).unwrap().display().to_string();
    app.recent = vec![canonical_target.clone()];
    app.layout_config.is_submodules = true;
    app.layout.submodules = Rect::new(0, 0, 20, 6);
    app.submodules.entries = vec![submodule_entry("deps/child", target_path)];

    app.handle_mouse_event(left_down(1, 0));
    app.handle_mouse_event(left_down(1, 0));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Graph);
    assert_eq!(app.graph_selected, 0);
    assert_eq!(app.path.as_deref(), Some(canonical_target.as_str()));
}

#[test]
fn double_click_on_status_row_acts_like_enter() {
    let (path, repo) = temp_repo("status-double");
    fs::write(path.join("file.txt"), "hello\n").unwrap();
    let mut app = graph_app();
    app.repo = Some(Rc::new(repo));
    app.path = Some(path.display().to_string());
    app.layout_config.is_status = true;
    app.layout.status_top = Rect::new(0, 0, 30, 6);
    app.is_uncommitted_loaded = true;
    app.uncommitted.is_staged = true;
    app.uncommitted.staged.modified = vec!["file.txt".into()];

    app.handle_mouse_event(left_down(1, 0));
    app.handle_mouse_event(left_down(1, 0));

    assert_eq!(app.viewport, Viewport::Viewer);
    assert_eq!(app.file_name.as_deref(), Some("file.txt"));
}

#[test]
fn viewer_mode_double_click_on_status_row_refreshes_viewer_file() {
    let (path, repo) = temp_repo("viewer-status-double");
    fs::write(path.join("old.txt"), "old\n").unwrap();
    fs::write(path.join("new.txt"), "new\n").unwrap();
    let mut app = graph_app();
    app.repo = Some(Rc::new(repo));
    app.path = Some(path.display().to_string());
    app.viewport = Viewport::Viewer;
    app.viewer_mode = ViewerMode::Full;
    app.file_name = Some("old.txt".into());
    app.layout_config.is_status = true;
    app.layout.status_top = Rect::new(40, 0, 30, 6);
    app.is_uncommitted_loaded = true;
    app.uncommitted.is_staged = true;
    app.uncommitted.staged.modified = vec!["old.txt".into(), "new.txt".into()];

    app.handle_mouse_event(left_down(41, 1));
    app.handle_mouse_event(left_down(41, 1));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Viewer);
    assert_eq!(app.file_name.as_deref(), Some("new.txt"));
}

#[test]
fn double_click_on_graph_and_splash_only_selects() {
    let mut app = graph_app();
    app.graph_scroll.set(2);

    app.handle_mouse_event(left_down(1, 1));
    app.handle_mouse_event(left_down(1, 1));
    assert_eq!(app.graph_selected, 3);
    assert_eq!(app.focus, Focus::Viewport);

    let mut splash = App { viewport: Viewport::Splash, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    splash.layout.app = Rect::new(0, 0, 120, 20);
    splash.layout.graph = Rect::new(0, 0, 120, 20);
    splash.recent = vec!["/repo/a".into(), "/repo/b".into()];

    splash.handle_mouse_event(left_down(1, 15));
    splash.handle_mouse_event(left_down(1, 15));

    assert_eq!(splash.splash_selected, 0);
    assert_eq!(splash.viewport, Viewport::Splash);
    assert_eq!(splash.path, None);
}

fn resizable_columns_app() -> App {
    let mut app = graph_app();
    app.layout.app = Rect::new(0, 0, 100, 30);
    app.layout.pane_left = Rect::new(0, 0, 30, 30);
    app.layout.pane_right = Rect::new(70, 0, 30, 30);
    app.layout_config.width_left_pane = 30;
    app.layout_config.width_right_pane = 30;
    app.layout_config.is_branches = true;
    app.layout_config.is_status = true;
    app
}

fn shared_graph_scrollbar_divider_app() -> App {
    let mut app = resizable_columns_app();
    app.layout.graph = Rect::new(30, 0, 40, 30);
    app.layout.graph_scrollbar = Rect::new(30, 0, 40, 30);
    app.layout.divider_right = Rect::new(69, 0, 1, 30);
    app.graph.total = 100;
    app.graph_selected = 0;
    app.graph_scroll.set(0);
    app
}

#[test]
fn horizontal_drag_on_shared_scrollbar_divider_resizes_pane() {
    let mut app = shared_graph_scrollbar_divider_app();

    app.handle_mouse_event(left_down(69, 10));
    app.handle_mouse_event(left_drag(60, 10));

    assert_eq!(app.graph_scroll.get(), 0);
    assert!(app.layout_config.width_right_pane > 30);
}

#[test]
fn vertical_drag_on_shared_scrollbar_divider_scrolls_graph() {
    let mut app = shared_graph_scrollbar_divider_app();

    app.handle_mouse_event(left_down(69, 10));
    app.handle_mouse_event(left_drag(69, 20));
    app.handle_mouse_event(left_up(69, 20));

    assert_eq!(app.layout_config.width_right_pane, 30);
    assert!(app.graph_scroll.get() > 0);
    assert_eq!(app.focus, Focus::Viewport);
}

#[test]
fn click_on_shared_scrollbar_divider_scrolls_without_resizing() {
    let mut app = shared_graph_scrollbar_divider_app();

    app.handle_mouse_event(left_down(69, 20));
    app.handle_mouse_event(left_up(69, 20));

    assert_eq!(app.layout_config.width_right_pane, 30);
    assert!(app.graph_scroll.get() > 0);
}

#[test]
fn keyboard_resize_updates_side_columns_for_focused_panes_and_viewport() {
    let mut app = resizable_columns_app();

    app.focus = Focus::Branches;
    app.on_resize_pane_right();
    assert_eq!(app.layout_config.width_left_pane, 31);
    app.on_resize_pane_left();
    assert_eq!(app.layout_config.width_left_pane, 30);

    app.focus = Focus::Search;
    app.on_resize_pane_right();
    assert_eq!(app.layout_config.width_left_pane, 31);
    app.on_resize_pane_left();
    assert_eq!(app.layout_config.width_left_pane, 30);

    app.focus = Focus::StatusTop;
    app.on_resize_pane_left();
    assert_eq!(app.layout_config.width_right_pane, 31);
    app.on_resize_pane_right();
    assert_eq!(app.layout_config.width_right_pane, 30);

    app.focus = Focus::Viewport;
    app.on_resize_pane_left();
    assert_eq!(app.layout_config.width_left_pane, 29);
    app.on_resize_pane_right();
    assert_eq!(app.layout_config.width_right_pane, 29);
}

#[test]
fn keyboard_resize_clamps_side_columns() {
    let mut app = resizable_columns_app();
    app.focus = Focus::Branches;
    app.layout.app = Rect::new(0, 0, 70, 30);
    app.layout.pane_right = Rect::new(36, 0, 34, 30);
    app.layout_config.width_left_pane = 16;

    app.on_resize_pane_left();
    assert_eq!(app.layout_config.width_left_pane, 16);

    app.layout.app = Rect::new(0, 0, 100, 30);
    app.layout.pane_right = Rect::new(84, 0, 16, 30);
    app.layout_config.width_left_pane = 64;

    app.on_resize_pane_right();
    assert_eq!(app.layout_config.width_left_pane, 64);
}

fn left_stack_app() -> App {
    let mut app = graph_app();
    app.layout_config.is_branches = true;
    app.layout_config.is_tags = true;
    app.layout_config.is_stashes = true;
    app.layout_config.weight_branches = 100;
    app.layout_config.weight_tags = 100;
    app.layout_config.weight_stashes = 100;
    app.layout.pane_branches = Rect::new(0, 0, 30, 10);
    app.layout.pane_tags = Rect::new(0, 10, 30, 10);
    app.layout.pane_stashes = Rect::new(0, 20, 30, 10);
    app
}

#[test]
fn keyboard_resize_grows_focused_left_stack_toward_direction() {
    let mut app = left_stack_app();
    app.focus = Focus::Tags;

    app.on_resize_pane_up();
    assert!(app.layout_config.weight_branches < 100);
    assert!(app.layout_config.weight_tags > 100);
    assert_eq!(app.layout_config.weight_stashes, 100);

    let mut app = left_stack_app();
    app.focus = Focus::Tags;

    app.on_resize_pane_down();
    assert_eq!(app.layout_config.weight_branches, 100);
    assert!(app.layout_config.weight_tags > 100);
    assert!(app.layout_config.weight_stashes < 100);
}

#[test]
fn keyboard_resize_edge_stack_direction_shrinks_from_opposite_edge() {
    let mut app = left_stack_app();
    app.focus = Focus::Branches;

    app.on_resize_pane_up();
    assert!(app.layout_config.weight_branches < 100);
    assert!(app.layout_config.weight_tags > 100);

    let mut app = left_stack_app();
    app.focus = Focus::Stashes;

    app.on_resize_pane_down();
    assert!(app.layout_config.weight_tags > 100);
    assert!(app.layout_config.weight_stashes < 100);
}

#[test]
fn keyboard_resize_updates_search_stack_weight() {
    let mut app = left_stack_app();
    app.layout_config.is_search = true;
    app.layout_config.weight_search = 100;
    app.layout.pane_search = Rect::new(0, 30, 30, 10);
    app.focus = Focus::Search;

    app.on_resize_pane_up();

    assert!(app.layout_config.weight_stashes < 100);
    assert!(app.layout_config.weight_search > 100);
}

#[test]
fn mouse_resize_updates_search_stack_weight() {
    let mut app = graph_app();
    app.layout_config.is_worktrees = true;
    app.layout_config.is_search = true;
    app.layout_config.weight_worktrees = 100;
    app.layout_config.weight_search = 100;
    app.layout.pane_worktrees = Rect::new(0, 0, 30, 10);
    app.layout.pane_search = Rect::new(0, 10, 30, 10);
    app.layout.divider_worktrees_search = Rect::new(0, 9, 30, 1);

    app.handle_mouse_event(left_down(1, 9));
    app.handle_mouse_event(left_drag(1, 12));

    assert!(app.layout_config.weight_worktrees > 100);
    assert!(app.layout_config.weight_search < 100);
}

#[test]
fn keyboard_resize_updates_right_stack_weights() {
    let mut app = graph_app();
    app.graph_selected = 1;
    app.focus = Focus::StatusTop;
    app.layout_config.is_inspector = true;
    app.layout_config.is_status = true;
    app.layout_config.weight_inspector = 100;
    app.layout_config.weight_status = 100;
    app.layout_config.weight_status_top = 100;
    app.layout.pane_inspector = Rect::new(70, 0, 30, 10);
    app.layout.pane_status = Rect::new(70, 10, 30, 10);
    app.layout.pane_status_top = Rect::new(70, 10, 30, 10);

    app.on_resize_pane_up();
    assert!(app.layout_config.weight_inspector < 100);
    assert!(app.layout_config.weight_status > 100);
    assert_eq!(app.layout_config.weight_status_top, 100);

    let mut app = graph_app();
    app.graph_selected = 0;
    app.focus = Focus::StatusTop;
    app.layout_config.is_inspector = false;
    app.layout_config.is_status = true;
    app.layout_config.weight_status_top = 100;
    app.layout_config.weight_status_bottom = 100;
    app.layout.pane_status_top = Rect::new(70, 0, 30, 10);
    app.layout.pane_status_bottom = Rect::new(70, 10, 30, 10);

    app.on_resize_pane_down();
    assert!(app.layout_config.weight_status_top > 100);
    assert!(app.layout_config.weight_status_bottom < 100);
}

fn split_viewer_app(is_zen: bool) -> App {
    let mut app = resizable_columns_app();
    app.viewport = Viewport::Viewer;
    app.focus = Focus::Viewport;
    app.viewer_mode = ViewerMode::Split;
    app.layout_config.is_zen = is_zen;
    app.layout_config.weight_viewer_split_left = 100;
    app.layout_config.weight_viewer_split_right = 100;
    app.layout.viewer_split_left = Rect::new(10, 0, 20, 10);
    app.layout.viewer_split_right = Rect::new(31, 0, 20, 10);
    app
}

#[test]
fn keyboard_resize_in_split_viewer_resizes_outer_columns() {
    let mut app = split_viewer_app(false);

    app.on_resize_pane_left();
    assert_eq!(app.layout_config.width_left_pane, 29);
    assert_eq!(app.layout_config.width_right_pane, 30);
    assert_eq!(app.layout_config.weight_viewer_split_left, 100);
    assert_eq!(app.layout_config.weight_viewer_split_right, 100);
    assert!(app.is_viewer_layout_dirty);

    app.is_viewer_layout_dirty = false;
    app.on_resize_pane_right();
    assert_eq!(app.layout_config.width_left_pane, 29);
    assert_eq!(app.layout_config.width_right_pane, 29);
    assert_eq!(app.layout_config.weight_viewer_split_left, 100);
    assert_eq!(app.layout_config.weight_viewer_split_right, 100);
    assert!(app.is_viewer_layout_dirty);
}

#[test]
fn keyboard_resize_in_zen_split_viewer_is_noop() {
    let mut app = split_viewer_app(true);

    app.on_resize_pane_right();
    assert_eq!(app.layout_config.width_left_pane, 30);
    assert_eq!(app.layout_config.width_right_pane, 30);
    assert_eq!(app.layout_config.weight_viewer_split_left, 100);
    assert_eq!(app.layout_config.weight_viewer_split_right, 100);
    assert!(!app.is_viewer_layout_dirty);
}

#[test]
fn keyboard_resize_noops_in_settings_modals_and_non_split_zen() {
    let mut app = resizable_columns_app();
    app.viewport = Viewport::Settings;
    app.focus = Focus::Viewport;
    app.on_resize_pane_left();
    assert_eq!(app.layout_config.width_left_pane, 30);

    let mut app = resizable_columns_app();
    app.focus = Focus::ModalCheckout;
    app.on_resize_pane_right();
    assert_eq!(app.layout_config.width_right_pane, 30);

    let mut app = resizable_columns_app();
    app.layout_config.is_zen = true;
    app.focus = Focus::Branches;
    app.on_resize_pane_right();
    assert_eq!(app.layout_config.width_left_pane, 30);
}
