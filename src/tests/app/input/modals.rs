use super::*;
use crate::app::app::{RemoteInputAction, SettingsSelection, SettingsSelectionKind, Viewport};
use crate::core::graph_service::GraphCommand;
use crate::git::actions::network::NetworkRequest;
use crate::git::queries::remotes::{GUITAR_DEFAULT_REMOTE_CONFIG, PUSH_DEFAULT_CONFIG};
use crate::helpers::keymap::{Command, InputMode, KeyBinding, Keymaps};
use git2::{Repository, Signature};
use indexmap::IndexMap;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-input-modals-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn write_file(root: &Path, file: &str) {
    let path = root.join(file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, "content\n").unwrap();
}

fn commit_files(repo: &Repository, files: &[&str], message: &str) {
    let workdir = repo.workdir().unwrap().to_path_buf();
    for file in files {
        write_file(&workdir, file);
    }

    let mut index = repo.index().unwrap();
    for file in files {
        index.add_path(Path::new(file)).unwrap();
    }
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap();
}

fn modal_app(name: &str) -> App {
    let (_path, repo) = temp_repo(name);
    commit_files(&repo, &["src/app/draw/search.rs", "src/app/draw/status.rs", "src/app/draw/stashes.rs"], "files");
    App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::ModalFileSearch, modal_file_search_return_focus: Focus::Branches, ..Default::default() }
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn graph_lane_limit_shortcut_keymaps() -> Keymaps {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(KeyCode::Char('-'), KeyModifiers::NONE), Command::ShrinkGraphLaneLimit);
    normal.insert(KeyBinding::new(KeyCode::Char('+'), KeyModifiers::NONE), Command::GrowGraphLaneLimit);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, IndexMap::new());
    maps
}

fn shutdown_graph_worker(app: &mut App) {
    if let Some(tx) = app.graph_tx.take() {
        let _ = tx.send(GraphCommand::Shutdown);
    }
    if let Some(cancel) = &app.walker_cancel {
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    if let Some(handle) = app.walker_handle.take() {
        handle.join().unwrap();
    }
}

#[test]
fn file_search_esc_closes_and_restores_prior_focus() {
    let (_path, repo) = temp_repo("esc");
    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Reflogs, ..Default::default() };
    app.on_find_file();
    app.modal_input.set_value("search");
    app.modal_file_search_results.push(crate::git::queries::files::FileSearchResult { path: "src/app/draw/search.rs".to_string(), score: 1, matched_indices: vec![13] });

    app.handle_modal_key_event(key(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::Reflogs);
    assert!(app.modal_input.value().is_empty());
    assert!(app.modal_file_search_results.is_empty());
}

#[test]
fn file_search_typing_updates_results_and_resets_selection() {
    let mut app = modal_app("typing");
    app.modal_file_search_selected = 2;
    app.modal_file_search_scroll.set(2);

    app.handle_modal_key_event(key(KeyCode::Char('s'), KeyModifiers::NONE));

    assert_eq!(app.modal_input.value(), "s");
    assert!(!app.modal_file_search_results.is_empty());
    assert_eq!(app.modal_file_search_selected, 0);
    assert_eq!(app.modal_file_search_scroll.get(), 0);
}

#[test]
fn file_search_ctrl_and_arrow_keys_move_selection() {
    let mut app = modal_app("navigation");
    app.handle_modal_key_event(key(KeyCode::Char('s'), KeyModifiers::NONE));
    assert!(app.modal_file_search_results.len() > 1);

    app.handle_modal_key_event(key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.modal_file_search_selected, 1);

    app.handle_modal_key_event(key(KeyCode::Char('k'), KeyModifiers::CONTROL));
    assert_eq!(app.modal_file_search_selected, 0);

    app.handle_modal_key_event(key(KeyCode::Char('j'), KeyModifiers::CONTROL));
    assert_eq!(app.modal_file_search_selected, 1);

    app.handle_modal_key_event(key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.modal_file_search_selected, 0);
}

#[test]
fn file_search_enter_starts_file_history_search() {
    let mut app = modal_app("enter");
    let (tx, rx) = std::sync::mpsc::channel();
    app.graph.generation = 5;
    app.graph_tx = Some(tx);
    for ch in "search".chars() {
        app.handle_modal_key_event(key(KeyCode::Char(ch), KeyModifiers::NONE));
    }

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.modal_input.value().is_empty());
    assert_eq!(app.focus, Focus::Search);
    assert_eq!(app.viewport, Viewport::Graph);
    assert!(app.layout_config.is_search);
    assert_eq!(app.search_path.as_deref(), Some("src/app/draw/search.rs"));
    assert!(app.search_is_loading);

    match rx.try_recv().unwrap() {
        GraphCommand::QueryFileHistory { generation, request_id, path } => {
            assert_eq!(generation, 5);
            assert_eq!(request_id, 1);
            assert_eq!(path, "src/app/draw/search.rs");
        },
        other => panic!("expected file history request, got {other:?}"),
    }
}

#[test]
fn file_search_plain_l_is_inserted_into_input() {
    let mut app = modal_app("plain-l");

    app.handle_modal_key_event(key(KeyCode::Char('l'), KeyModifiers::NONE));

    assert_eq!(app.modal_input.value(), "l");
}

#[test]
fn rename_branch_submit_renames_and_clears_modal_state() {
    let (path, repo) = temp_repo("rename-submit");
    commit_files(&repo, &["file.txt"], "initial");
    let target = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("feature", &target, false).unwrap();
    drop(target);

    let mut app = App {
        path: Some(path.display().to_string()),
        recent: vec![path.display().to_string()],
        repo: Some(Rc::new(repo)),
        viewport: Viewport::Graph,
        focus: Focus::ModalRenameBranch,
        modal_rename_branch_source: Some("feature".to_string()),
        ..Default::default()
    };
    app.modal_input.set_value("topic");

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.modal_input.value().is_empty());
    assert_eq!(app.modal_rename_branch_source, None);

    let repo = Repository::open(path).unwrap();
    assert!(repo.find_branch("feature", git2::BranchType::Local).is_err());
    assert!(repo.find_branch("topic", git2::BranchType::Local).is_ok());
}

#[test]
fn rename_branch_error_returns_to_input_with_text_preserved() {
    let (_path, repo) = temp_repo("rename-error");
    commit_files(&repo, &["file.txt"], "initial");
    let target = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("feature", &target, false).unwrap();
    repo.branch("existing", &target, false).unwrap();
    drop(target);

    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::ModalRenameBranch, modal_rename_branch_source: Some("feature".to_string()), ..Default::default() };
    app.modal_input.set_value("existing");

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::ModalError);
    assert_eq!(app.modal_error_return_focus, Focus::ModalRenameBranch);
    assert_eq!(app.modal_input.value(), "existing");
    assert_eq!(app.modal_rename_branch_source.as_deref(), Some("feature"));

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::ModalRenameBranch);
    assert_eq!(app.modal_input.value(), "existing");
}

#[test]
fn rename_branch_esc_closes_and_clears_modal_state() {
    let (_path, repo) = temp_repo("rename-esc");
    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::ModalRenameBranch, modal_rename_branch_source: Some("feature".to_string()), ..Default::default() };
    app.modal_input.set_value("topic");

    app.handle_modal_key_event(key(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.modal_input.value().is_empty());
    assert_eq!(app.modal_rename_branch_source, None);
}

fn remote_app(name: &str) -> (PathBuf, App) {
    let (path, repo) = temp_repo(name);
    commit_files(&repo, &["file.txt"], "initial");
    let path_string = path.display().to_string();
    let app = App { path: Some(path_string.clone()), recent: vec![path_string], repo: Some(Rc::new(repo)), viewport: Viewport::Settings, focus: Focus::Viewport, ..Default::default() };
    (path, app)
}

#[test]
fn settings_remote_add_row_opens_add_name_prompt() {
    let (_path, mut app) = remote_app("settings-add");
    app.settings_selected = 10;
    app.settings_selections = vec![SettingsSelection { line: 10, kind: SettingsSelectionKind::RemoteAdd }];

    app.on_select();

    assert_eq!(app.focus, Focus::ModalRemoteName);
    assert_eq!(app.modal_remote_input_action, RemoteInputAction::AddName);
    assert!(app.modal_input.value().is_empty());
}

#[test]
fn settings_remote_row_opens_remote_action_modal() {
    let (_path, mut app) = remote_app("settings-remote");
    app.settings_selected = 10;
    app.settings_selections = vec![SettingsSelection { line: 10, kind: SettingsSelectionKind::Remote("origin".to_string()) }];

    app.on_select();

    assert_eq!(app.focus, Focus::ModalRemoteAction);
    assert_eq!(app.modal_remote_target.as_deref(), Some("origin"));
    assert_eq!(app.modal_remote_selected, 0);
}

#[test]
fn settings_graph_lane_limit_row_opens_prefilled_prompt() {
    let (_path, mut app) = remote_app("settings-lane-limit-open");
    app.layout_config.graph_lane_limit = 34;
    app.settings_selected = 10;
    app.settings_selections = vec![SettingsSelection { line: 10, kind: SettingsSelectionKind::GraphLaneLimit }];

    app.on_select();

    assert_eq!(app.focus, Focus::ModalGraphLaneLimit);
    assert_eq!(app.modal_input.value(), "34");
}

#[test]
fn graph_lane_limit_input_ignores_zero_and_invalid_values() {
    let mut app = App { viewport: Viewport::Settings, focus: Focus::ModalGraphLaneLimit, ..Default::default() };
    app.layout_config.graph_lane_limit = 20;

    app.modal_input.set_value("0");
    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::ModalGraphLaneLimit);
    assert_eq!(app.layout_config.graph_lane_limit, 20);
    assert_eq!(app.modal_input.value(), "0");

    app.modal_input.set_value("not-a-number");
    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::ModalGraphLaneLimit);
    assert_eq!(app.layout_config.graph_lane_limit, 20);
    assert_eq!(app.modal_input.value(), "not-a-number");
}

#[test]
fn graph_lane_limit_input_updates_and_saves_value() {
    let mut app = App { viewport: Viewport::Settings, focus: Focus::ModalGraphLaneLimit, settings_selected: 12, ..Default::default() };
    app.settings_scroll.set(4);
    app.modal_input.set_value("7");

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.layout_config.graph_lane_limit, 7);
    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.settings_selected, 12);
    assert_eq!(app.settings_scroll.get(), 4);
    assert!(app.modal_input.value().is_empty());
    assert!(app.graph_tx.is_none());
}

#[test]
fn graph_lane_limit_shortcuts_adjust_and_clamp_without_repo() {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, keymaps: graph_lane_limit_shortcut_keymaps(), ..Default::default() };
    app.layout_config.graph_lane_limit = 3;

    app.handle_key_event(key(KeyCode::Char('-'), KeyModifiers::NONE));
    assert_eq!(app.layout_config.graph_lane_limit, 2);

    app.handle_key_event(key(KeyCode::Char('+'), KeyModifiers::NONE));
    assert_eq!(app.layout_config.graph_lane_limit, 3);

    app.layout_config.graph_lane_limit = 1;
    app.handle_key_event(key(KeyCode::Char('-'), KeyModifiers::NONE));

    assert_eq!(app.layout_config.graph_lane_limit, 1);
    assert!(app.graph_tx.is_none());
}

#[test]
fn graph_lane_limit_shortcut_reloads_open_repo_and_preserves_settings_position() {
    let (path, repo) = temp_repo("lane-limit-shortcut-reload");
    commit_files(&repo, &["file.txt"], "initial");
    let repo_path = path.display().to_string();
    let mut app = App {
        path: Some(repo_path.clone()),
        recent: vec![repo_path],
        repo: Some(Rc::new(repo)),
        viewport: Viewport::Settings,
        focus: Focus::Viewport,
        settings_selected: 12,
        keymaps: graph_lane_limit_shortcut_keymaps(),
        ..Default::default()
    };
    app.settings_scroll.set(4);
    app.layout_config.graph_lane_limit = 20;

    app.handle_key_event(key(KeyCode::Char('-'), KeyModifiers::NONE));

    assert_eq!(app.layout_config.graph_lane_limit, 19);
    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.settings_selected, 12);
    assert_eq!(app.settings_scroll.get(), 4);
    assert!(app.graph_tx.is_some());
    assert!(app.walker_handle.is_some());

    shutdown_graph_worker(&mut app);
}

#[test]
fn graph_lane_limit_input_reloads_open_repo_and_preserves_settings_position() {
    let (path, repo) = temp_repo("lane-limit-reload");
    commit_files(&repo, &["file.txt"], "initial");
    let repo_path = path.display().to_string();
    let mut app = App {
        path: Some(repo_path.clone()),
        recent: vec![repo_path],
        repo: Some(Rc::new(repo)),
        viewport: Viewport::Settings,
        focus: Focus::ModalGraphLaneLimit,
        settings_selected: 12,
        ..Default::default()
    };
    app.settings_scroll.set(4);
    app.layout_config.graph_lane_limit = 20;
    app.modal_input.set_value("7");

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.layout_config.graph_lane_limit, 7);
    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.settings_selected, 12);
    assert_eq!(app.settings_scroll.get(), 4);
    assert!(app.graph_tx.is_some());
    assert!(app.walker_handle.is_some());

    shutdown_graph_worker(&mut app);
}

#[test]
fn add_remote_flow_creates_remote_and_returns_to_settings() {
    let (path, mut app) = remote_app("add-remote");
    app.begin_add_remote();

    app.modal_input.set_value("origin");
    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.focus, Focus::ModalRemoteUrl);
    assert_eq!(app.modal_remote_input_action, RemoteInputAction::AddUrl);
    assert_eq!(app.modal_remote_name, "origin");

    app.modal_input.set_value("https://example.com/repo.git");
    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.modal_input.value().is_empty());
    assert_eq!(Repository::open(path).unwrap().find_remote("origin").unwrap().url(), Some("https://example.com/repo.git"));
}

#[test]
fn add_remote_invalid_name_returns_to_name_prompt_with_text_preserved() {
    let (_path, mut app) = remote_app("add-invalid-name");
    app.begin_add_remote();
    app.modal_input.set_value("bad\nname");

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::ModalError);
    assert_eq!(app.modal_error_return_focus, Focus::ModalRemoteName);
    assert_eq!(app.modal_input.value(), "bad\nname");
}

#[test]
fn rename_remote_flow_rewrites_hidden_remote_branch_names() {
    let (path, mut app) = remote_app("rename-remote");
    app.repo.as_ref().unwrap().remote("origin", "https://example.com/repo.git").unwrap();
    {
        let mut config = app.repo.as_ref().unwrap().config().unwrap();
        config.set_str(GUITAR_DEFAULT_REMOTE_CONFIG, "origin").unwrap();
        config.set_str(PUSH_DEFAULT_CONFIG, "origin").unwrap();
    }
    let oid = app.repo.as_ref().unwrap().head().unwrap().target().unwrap();
    app.repo.as_ref().unwrap().reference("refs/remotes/origin/topic", oid, true, "test").unwrap();
    app.repo.as_ref().unwrap().reference("refs/remotes/other/topic", oid, true, "test").unwrap();
    app.branches.hidden_branch_names.insert("origin/topic".to_string());
    app.branches.hidden_branch_names.insert("other/topic".to_string());
    app.modal_remote_target = Some("origin".to_string());
    app.modal_remote_input_action = RemoteInputAction::Rename;
    app.focus = Focus::ModalRemoteName;
    app.modal_input.set_value("upstream");

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    let repo = Repository::open(path).unwrap();
    assert!(repo.find_remote("origin").is_err());
    assert!(repo.find_remote("upstream").is_ok());
    let config = repo.config().unwrap();
    assert_eq!(config.get_string(GUITAR_DEFAULT_REMOTE_CONFIG).unwrap(), "upstream");
    assert_eq!(config.get_string(PUSH_DEFAULT_CONFIG).unwrap(), "upstream");
    assert!(app.branches.hidden_branch_names.contains("upstream/topic"));
    assert!(app.branches.hidden_branch_names.contains("other/topic"));
    assert!(!app.branches.hidden_branch_names.contains("origin/topic"));
}

#[test]
fn remote_action_sets_selected_remote_as_default() {
    let (path, mut app) = remote_app("set-default");
    app.repo.as_ref().unwrap().remote("upstream", "https://example.com/repo.git").unwrap();
    app.modal_remote_target = Some("upstream".to_string());
    app.modal_remote_selected = 1;
    app.focus = Focus::ModalRemoteAction;

    app.on_select();

    let repo = Repository::open(path).unwrap();
    let config = repo.config().unwrap();
    assert_eq!(config.get_string(GUITAR_DEFAULT_REMOTE_CONFIG).unwrap(), "upstream");
    assert_eq!(config.get_string(PUSH_DEFAULT_CONFIG).unwrap(), "upstream");
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Settings);
}

#[test]
fn edit_remote_fetch_url_flow_updates_url() {
    let (path, mut app) = remote_app("edit-fetch-url");
    app.repo.as_ref().unwrap().remote("origin", "https://example.com/repo.git").unwrap();
    app.modal_remote_target = Some("origin".to_string());
    app.modal_remote_input_action = RemoteInputAction::EditUrl;
    app.focus = Focus::ModalRemoteUrl;
    app.modal_input.set_value("https://example.com/renamed.git");

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(Repository::open(path).unwrap().find_remote("origin").unwrap().url(), Some("https://example.com/renamed.git"));
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Settings);
}

#[test]
fn edit_remote_empty_push_url_clears_push_url() {
    let (path, mut app) = remote_app("edit-push-url");
    app.repo.as_ref().unwrap().remote("origin", "https://example.com/repo.git").unwrap();
    app.repo.as_ref().unwrap().remote_set_pushurl("origin", Some("ssh://example.com/repo.git")).unwrap();
    app.modal_remote_target = Some("origin".to_string());
    app.modal_remote_input_action = RemoteInputAction::EditPushUrl;
    app.focus = Focus::ModalRemoteUrl;
    app.modal_input.set_value("");

    app.handle_modal_key_event(key(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(Repository::open(path).unwrap().find_remote("origin").unwrap().pushurl(), None);
}

#[test]
fn delete_remote_confirmation_deletes_remote_and_prunes_hidden_remote_branches() {
    let (path, mut app) = remote_app("delete-remote");
    app.repo.as_ref().unwrap().remote("origin", "https://example.com/repo.git").unwrap();
    {
        let mut config = app.repo.as_ref().unwrap().config().unwrap();
        config.set_str(GUITAR_DEFAULT_REMOTE_CONFIG, "origin").unwrap();
        config.set_str(PUSH_DEFAULT_CONFIG, "origin").unwrap();
    }
    let oid = app.repo.as_ref().unwrap().head().unwrap().target().unwrap();
    app.repo.as_ref().unwrap().reference("refs/remotes/other/topic", oid, true, "test").unwrap();
    app.branches.hidden_branch_names.insert("origin/topic".to_string());
    app.branches.hidden_branch_names.insert("other/topic".to_string());
    app.modal_remote_target = Some("origin".to_string());
    app.focus = Focus::ModalRemoteDelete;

    app.on_select();

    let repo = Repository::open(path).unwrap();
    assert!(repo.find_remote("origin").is_err());
    let config = repo.config().unwrap();
    assert!(config.get_string(GUITAR_DEFAULT_REMOTE_CONFIG).is_err());
    assert!(config.get_string(PUSH_DEFAULT_CONFIG).is_err());
    assert!(!app.branches.hidden_branch_names.contains("origin/topic"));
    assert!(app.branches.hidden_branch_names.contains("other/topic"));
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Settings);
}

#[test]
fn remote_modal_cancel_clears_pending_state_and_returns_to_settings() {
    let (_path, mut app) = remote_app("cancel");
    app.modal_remote_target = Some("origin".to_string());
    app.modal_remote_name = "origin".to_string();
    app.modal_input.set_value("value");
    app.focus = Focus::ModalRemoteUrl;

    app.handle_modal_key_event(key(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Settings);
    assert_eq!(app.modal_remote_target, None);
    assert!(app.modal_remote_name.is_empty());
    assert!(app.modal_input.value().is_empty());
}

#[test]
fn selected_remote_fetch_uses_selected_remote_name() {
    let (_path, mut app) = remote_app("fetch-selected");
    {
        let mut config = app.repo.as_ref().unwrap().config().unwrap();
        config.set_str(GUITAR_DEFAULT_REMOTE_CONFIG, "origin").unwrap();
        config.set_str(PUSH_DEFAULT_CONFIG, "origin").unwrap();
    }
    app.modal_remote_target = Some("upstream".to_string());
    app.modal_remote_selected = 0;
    app.focus = Focus::ModalRemoteAction;

    app.on_select();

    assert_eq!(app.pending_network_request, Some(NetworkRequest::Fetch { repo_path: app.path.clone().unwrap(), remote_name: "upstream".to_string() }));
    assert_eq!(app.focus, Focus::ModalNetworkProgress);
}
