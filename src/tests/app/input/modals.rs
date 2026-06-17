use super::*;
use crate::app::app::Viewport;
use crate::core::graph_service::GraphCommand;
use git2::{Repository, Signature};
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
