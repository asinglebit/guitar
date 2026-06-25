use super::*;
use crate::core::submodules::{SubmoduleEntry, SubmoduleStackEntry, Submodules};
use crate::git::{actions::network::NetworkRequest, auth::NetworkResult, queries::submodules::list_submodules};
use crate::helpers::keymap::{Command, InputMode, KeyBinding};
use git2::{Repository, Signature};
use indexmap::IndexMap;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

fn entry(name: &str, open: bool) -> SubmoduleEntry {
    SubmoduleEntry {
        name: name.into(),
        path: PathBuf::from(name),
        absolute_path: PathBuf::from(format!("/tmp/{name}")),
        url: None,
        branch: None,
        head: None,
        index: None,
        workdir: None,
        is_open: open,
        is_uninitialized: !open,
        is_in_head: true,
        is_in_index: true,
        is_in_config: true,
        is_in_workdir: open,
        is_index_modified: false,
        is_workdir_modified: false,
        has_new_commits: false,
        has_modified_content: false,
        has_untracked_content: false,
    }
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = env::temp_dir().join(format!("guitar-submodule-input-{name}-{}-{suffix}", process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn init_repo(path: &Path) -> Repository {
    fs::create_dir_all(path).unwrap();
    let repo = Repository::init(path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    commit_file(&repo, "file.txt", "hello\n", "initial");
    repo
}

fn commit_file(repo: &Repository, file: &str, contents: &str, message: &str) -> git2::Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), contents).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    commit_index(repo, message)
}

fn commit_index(repo: &Repository, message: &str) -> git2::Oid {
    let mut index = repo.index().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}

fn parent_with_submodule(dir: &TestDir) -> Repository {
    let child_path = dir.path.join("child");
    let parent_path = dir.path.join("parent");
    let child = init_repo(&child_path);
    drop(child);
    let parent = init_repo(&parent_path);
    let mut submodule = parent.submodule(child_path.to_str().unwrap(), Path::new("deps/child"), true).unwrap();
    submodule.clone(None).unwrap();
    submodule.add_finalize().unwrap();
    commit_index(&parent, "add submodule");
    drop(submodule);
    parent
}

fn submodule_action_keymaps() -> IndexMap<InputMode, IndexMap<KeyBinding, Command>> {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(KeyCode::Char('a'), KeyModifiers::CONTROL), Command::ActionMode);
    let mut action = IndexMap::new();
    action.insert(KeyBinding::new(KeyCode::Char('i'), KeyModifiers::NONE), Command::UpdateSubmodule);
    action.insert(KeyBinding::new(KeyCode::Char('I'), KeyModifiers::SHIFT), Command::SyncSubmodule);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, action);
    maps
}

#[test]
fn selected_submodule_name_reads_current_selection() {
    let app = App { submodules: Submodules::from_entries(vec![entry("first", true), entry("second", true)]), submodules_selected: 1, ..Default::default() };

    assert_eq!(app.selected_submodule_name().as_deref(), Some("second"));
}

#[test]
fn opening_uninitialized_submodule_shows_error() {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Submodules, submodules: Submodules::from_entries(vec![entry("deps/child", false)]), ..Default::default() };

    app.open_selected_submodule();

    assert_eq!(app.focus, Focus::ModalError);
    assert!(app.modal_error_message.contains("not initialized"));
    assert!(app.submodule_stack.is_empty());
}

#[test]
fn opening_checked_out_submodule_pushes_stack_entry_and_reloads_submodule() {
    let dir = TestDir::new("open-stack");
    let parent = parent_with_submodule(&dir);
    let parent_path = fs::canonicalize(parent.workdir().unwrap()).unwrap();
    let entries = list_submodules(&parent).unwrap();
    let child_path = fs::canonicalize(entries[0].absolute_path.clone()).unwrap();
    let mut app = App {
        path: Some(parent_path.display().to_string()),
        repo: Some(crate::app::app::RepoHandle::from_repo(Rc::new(parent))),
        viewport: Viewport::Graph,
        focus: Focus::Submodules,
        submodules: Submodules::from_entries(entries),
        recent_save_path: Some(dir.path.join("recent-open.json")),
        ..Default::default()
    };

    app.open_selected_submodule();

    assert_eq!(app.submodule_stack.len(), 1);
    assert_eq!(app.submodule_stack[0].parent_path, parent_path);
    assert_eq!(app.submodule_stack[0].submodule_path, PathBuf::from("deps/child"));
    assert_eq!(app.submodule_stack[0].submodule_name, "deps/child");
    assert_eq!(app.path, Some(child_path.display().to_string()));
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Graph);
}

#[test]
fn opening_nested_submodule_appends_to_existing_stack() {
    let dir = TestDir::new("nested-stack");
    let root_path = dir.path.join("root");
    let child_path = dir.path.join("child");
    let grandchild_path = dir.path.join("grandchild");
    let child = init_repo(&child_path);
    let grandchild = init_repo(&grandchild_path);
    drop(grandchild);
    let mut nested = entry("vendor/grandchild", true);
    nested.path = PathBuf::from("vendor/grandchild");
    nested.absolute_path = grandchild_path.clone();
    let mut app = App {
        path: Some(child_path.display().to_string()),
        repo: Some(crate::app::app::RepoHandle::from_repo(Rc::new(child))),
        viewport: Viewport::Graph,
        focus: Focus::Submodules,
        submodules: Submodules::from_entries(vec![nested]),
        submodule_stack: vec![SubmoduleStackEntry::new(root_path.clone(), PathBuf::from("deps/child"), "deps/child".into())],
        recent_save_path: Some(dir.path.join("recent-nested.json")),
        ..Default::default()
    };

    app.open_selected_submodule();

    assert_eq!(app.submodule_stack.len(), 2);
    assert_eq!(app.submodule_stack[0].parent_path, root_path);
    assert_eq!(app.submodule_stack[1].parent_path, child_path);
    assert_eq!(app.submodule_stack[1].submodule_path, PathBuf::from("vendor/grandchild"));
    assert_eq!(app.path, Some(grandchild_path.display().to_string()));
}

#[test]
fn return_to_parent_repository_pops_one_stack_entry_and_reloads_parent() {
    let dir = TestDir::new("return-parent");
    let parent = init_repo(&dir.path.join("parent"));
    let child = init_repo(&dir.path.join("child"));
    let parent_path = fs::canonicalize(parent.workdir().unwrap()).unwrap();
    let child_path = fs::canonicalize(child.workdir().unwrap()).unwrap();
    drop(parent);
    let mut app = App {
        path: Some(child_path.display().to_string()),
        repo: Some(crate::app::app::RepoHandle::from_repo(Rc::new(child))),
        viewport: Viewport::Graph,
        focus: Focus::Submodules,
        submodule_stack: vec![SubmoduleStackEntry::new(parent_path.clone(), PathBuf::from("deps/child"), "deps/child".into())],
        recent_save_path: Some(dir.path.join("recent-return.json")),
        ..Default::default()
    };

    app.on_return_to_parent_repository();

    assert!(app.submodule_stack.is_empty());
    assert_eq!(app.path, Some(parent_path.display().to_string()));
    assert_eq!(app.focus, Focus::Viewport);
    assert_eq!(app.viewport, Viewport::Graph);
}

#[test]
fn return_to_parent_repository_noops_with_empty_stack() {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Submodules, graph_selected: 7, ..Default::default() };

    app.on_return_to_parent_repository();

    assert!(app.submodule_stack.is_empty());
    assert_eq!(app.focus, Focus::Submodules);
    assert_eq!(app.graph_selected, 7);
}

#[test]
fn action_keys_dispatch_update_and_sync_submodule() {
    let dir = TestDir::new("action-keys");
    let parent = parent_with_submodule(&dir);
    let parent_path = parent.workdir().unwrap().display().to_string();
    let entries = list_submodules(&parent).unwrap();

    let mut update_app = App {
        path: Some(parent_path.clone()),
        repo: Some(crate::app::app::RepoHandle::from_repo(Rc::new(parent))),
        viewport: Viewport::Graph,
        focus: Focus::Submodules,
        submodules: Submodules::from_entries(entries),
        keymaps: submodule_action_keymaps(),
        ..Default::default()
    };

    update_app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    update_app.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));

    assert_eq!(update_app.pending_network_request, Some(NetworkRequest::UpdateSubmodule { repo_path: parent_path.clone(), name: "deps/child".into() }));
    assert_eq!(update_app.focus, Focus::ModalNetworkProgress);
    let handle = update_app.network_handle.take().expect("update should start a network worker");
    match handle.join().unwrap() {
        NetworkResult::Success => {},
        other => panic!("unexpected update result: {other:?}"),
    }

    let parent = Repository::open(&parent_path).unwrap();
    let entries = list_submodules(&parent).unwrap();
    let mut sync_app = App {
        path: Some(parent_path),
        repo: Some(crate::app::app::RepoHandle::from_repo(Rc::new(parent))),
        viewport: Viewport::Graph,
        focus: Focus::Submodules,
        submodules: Submodules::from_entries(entries),
        keymaps: submodule_action_keymaps(),
        recent_save_path: Some(dir.path.join("recent.json")),
        ..Default::default()
    };

    sync_app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    sync_app.handle_key_event(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT));

    assert_eq!(sync_app.focus, Focus::Submodules);
    assert!(sync_app.pending_network_request.is_none());
    assert!(sync_app.network_handle.is_none());
}
