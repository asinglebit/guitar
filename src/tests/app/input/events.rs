use super::*;
use crate::{
    app::{
        app::{SettingsSelection, SettingsSelectionKind},
        state::layout::Layout,
    },
    core::{
        chunk::NONE,
        reflogs::HeadReflogAliasEntry,
        worktrees::{WorktreeEntry, WorktreeKind},
    },
    helpers::{
        keymap::{Command, InputMode, KeyBinding, KeymapSelection},
        layout::LayoutConfig,
    },
};
use git2::{Oid, Repository, Signature};
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    layout::Rect,
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

fn graph_app() -> App {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, layout_config: LayoutConfig::default(), layout: Layout::default(), ..Default::default() };
    app.layout.graph = Rect::new(0, 0, 30, 8);
    app.graph.total = 20;
    app
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

#[test]
fn mouse_click_selects_graph_row() {
    let mut app = graph_app();
    app.graph_scroll.set(2);

    app.handle_mouse_event(left_down(1, 3));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.graph_selected, 5);
}

#[test]
fn mouse_click_selects_left_pane_rows() {
    let mut app = graph_app();
    app.layout_config.is_branches = true;
    app.layout_config.is_tags = true;
    app.layout_config.is_stashes = true;
    app.layout_config.is_reflogs = true;
    app.layout_config.is_worktrees = true;
    app.layout.branches = Rect::new(0, 0, 20, 6);
    app.layout.tags = Rect::new(20, 0, 20, 6);
    app.layout.stashes = Rect::new(40, 0, 20, 6);
    app.layout.reflogs = Rect::new(60, 0, 20, 6);
    app.layout.worktrees = Rect::new(80, 0, 20, 6);
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
    app.branches_scroll.set(2);
    app.tags_scroll.set(2);
    app.stashes_scroll.set(2);
    app.reflogs_scroll.set(2);
    app.worktrees_scroll.set(2);

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
}

#[test]
fn mouse_click_selects_status_rows() {
    let mut app = graph_app();
    app.layout_config.is_status = true;
    app.layout.status_top = Rect::new(0, 0, 30, 6);
    app.layout.status_bottom = Rect::new(0, 10, 30, 6);
    app.is_uncommitted_loaded = true;
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

    app.handle_mouse_event(left_down(1, 16));

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
