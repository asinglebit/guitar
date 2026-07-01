use super::*;
use crate::app::app::RepoHandle;
use crate::core::chunk::NONE;
use crate::core::reflogs::HeadReflogAliasEntry;
use crate::git::actions::cherrypicking::{CherrypickOutcome, start_cherrypick};
use crate::git::actions::merging::{MergeOutcome, start_merge};
use crate::git::actions::rebasing::{RebaseOutcome, start_rebase};
use crate::git::actions::remotes::set_default_remote;
use crate::git::actions::reverting::{RevertOutcome, start_revert};
use crate::git::auth::{AuthChallenge, AuthProtocol};
use crate::git::queries::diffs::get_filenames_diff_at_workdir;
use crate::git::test_support::{commit_file as commit_with_content, commit_index, commit_named_file as commit, init_repo_at, temp_named_dir};
use crate::helpers::keymap::{Command, InputMode, KeyBinding};
use git2::{Signature, build::CheckoutBuilder};
use indexmap::IndexMap;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

fn temp_repo(name: &str) -> (std::path::PathBuf, Repository) {
    let path = temp_named_dir("guitar-input-git", name);
    let repo = init_repo_at(&path);
    (path, repo)
}

fn add_local_bare_remote(repo: &Repository, name: &str) -> PathBuf {
    let path = temp_named_dir("guitar-input-git-remote", name);
    Repository::init_bare(&path).unwrap();
    repo.remote(name, path.to_str().unwrap()).unwrap();
    path
}

fn repo_handle(repo: Repository) -> RepoHandle {
    RepoHandle::from_repo(Rc::new(repo))
}

fn join_network_worker(app: &mut App) {
    if let Some(handle) = app.network_handle.take() {
        let _ = handle.join();
    }
}

fn parent_with_submodule(name: &str) -> (PathBuf, Repository) {
    let (child_path, child) = temp_repo(&format!("{name}-child"));
    commit(&child, "file.txt", "initial child");
    drop(child);

    let (parent_path, parent) = temp_repo(&format!("{name}-parent"));
    commit(&parent, "file.txt", "initial parent");
    let mut submodule = parent.submodule(child_path.to_str().unwrap(), Path::new("deps/child"), true).unwrap();
    submodule.clone(None).unwrap();
    submodule.add_finalize().unwrap();
    commit_index(&parent, "add submodule");
    drop(submodule);

    (parent_path, parent)
}

fn checkout_new_branch(repo: &Repository, name: &str) {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch(name, &head, false).unwrap();
    repo.set_head(&format!("refs/heads/{name}")).unwrap();
    repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
}

fn checkout_branch(repo: &Repository, name: &str) {
    repo.set_head(&format!("refs/heads/{name}")).unwrap();
    repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
}

fn current_branch_name(repo: &Repository) -> String {
    repo.head().unwrap().shorthand().unwrap().to_string()
}

fn file_search_keymaps() -> crate::helpers::keymap::Keymaps {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(KeyCode::Char('F'), KeyModifiers::SHIFT), Command::FindFile);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, IndexMap::new());
    maps
}

#[test]
fn shift_f_opens_file_search_modal_from_repo_views() {
    let (_path, repo) = temp_repo("file-search-shortcut");
    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Branches, keymaps: file_search_keymaps(), ..Default::default() };

    app.handle_key_event(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT));

    assert_eq!(app.focus, Focus::ModalFileSearch);
    assert_eq!(app.modal_file_search_return_focus, Focus::Branches);
}

#[test]
fn file_search_modal_does_not_open_from_splash_or_settings() {
    let (_path, repo) = temp_repo("file-search-blocked");
    let repo = Rc::new(repo);

    let mut splash = App { repo: Some(crate::app::app::RepoHandle::from_repo(repo.clone())), viewport: Viewport::Splash, focus: Focus::Viewport, ..Default::default() };
    splash.on_find_file();
    assert_eq!(splash.focus, Focus::Viewport);

    let mut settings = App { repo: Some(crate::app::app::RepoHandle::from_repo(repo)), viewport: Viewport::Settings, focus: Focus::Viewport, ..Default::default() };
    settings.on_find_file();
    assert_eq!(settings.focus, Focus::Viewport);
}

#[test]
fn status_panes_stage_and_unstage_submodule_pointer_change() {
    let (parent_path, parent) = parent_with_submodule("status-submodule-pointer");
    let sub_repo = Repository::open(parent.workdir().unwrap().join("deps/child")).unwrap();
    commit_with_content(&sub_repo, "file.txt", "advanced\n", "advance child");

    let mut app = App {
        path: Some(parent_path.display().to_string()),
        repo: Some(repo_handle(parent)),
        viewport: Viewport::Graph,
        focus: Focus::StatusBottom,
        graph_selected: 0,
        is_uncommitted_loaded: true,
        recent_save_path: Some(parent_path.join("recent.json")),
        ..Default::default()
    };
    app.uncommitted = get_filenames_diff_at_workdir(app.repo.as_ref().unwrap()).unwrap();
    app.status_bottom_selected = 0;

    app.on_stage();

    let staged = get_filenames_diff_at_workdir(app.repo.as_ref().unwrap()).unwrap();
    assert_eq!(staged.staged.modified, vec!["deps/child".to_string()]);
    assert!(staged.unstaged.modified.is_empty());

    app.uncommitted = staged;
    app.is_uncommitted_loaded = true;
    app.focus = Focus::StatusTop;
    app.status_top_selected = 0;

    app.on_unstage();

    let unstaged = get_filenames_diff_at_workdir(app.repo.as_ref().unwrap()).unwrap();
    assert!(unstaged.staged.modified.is_empty());
    assert_eq!(unstaged.unstaged.modified, vec!["deps/child".to_string()]);
}

fn app_with_default_remote(name: &str) -> (App, String, String) {
    let (path, repo) = temp_repo(name);
    commit(&repo, "file.txt", "initial");
    let _remote_path = add_local_bare_remote(&repo, "upstream");
    set_default_remote(&repo, "upstream").unwrap();
    let branch = current_branch_name(&repo);
    let path_string = path.display().to_string();
    let app = App { path: Some(path_string.clone()), repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    (app, path_string, branch)
}

fn assert_active_operation_routes(app: &mut App, kind: OperationKind, continue_action: impl FnOnce(&mut App)) {
    continue_action(app);

    assert_eq!(app.focus, Focus::ModalOperationProgress);
    assert_eq!(app.modal_operation_kind, kind);
    assert_eq!(app.pending_operation_action, Some(PendingOperationAction::Continue));

    app.focus = Focus::Viewport;
    app.pending_operation_action = None;
    app.on_abort_operation();

    assert_eq!(app.focus, Focus::ModalOperationProgress);
    assert_eq!(app.modal_operation_kind, kind);
    assert_eq!(app.pending_operation_action, Some(PendingOperationAction::Abort));
}

#[test]
fn fetch_all_uses_configured_default_remote() {
    let (mut app, path_string, _) = app_with_default_remote("fetch-default-remote");

    app.on_fetch_all();

    assert_eq!(app.pending_network_request, Some(NetworkRequest::Fetch { repo_path: path_string, remote_name: "upstream".to_string() }));
    join_network_worker(&mut app);
}

#[test]
fn force_push_uses_configured_default_remote() {
    let (mut app, path_string, branch) = app_with_default_remote("push-default-remote");

    app.on_force_push();

    assert_eq!(app.pending_network_request, Some(NetworkRequest::PushBranch { repo_path: path_string, remote_name: "upstream".to_string(), branch, force: true }));
    join_network_worker(&mut app);
}

#[test]
fn push_tags_uses_configured_default_remote() {
    let (mut app, path_string, _) = app_with_default_remote("push-tags-default-remote");

    app.on_push_tags();

    assert_eq!(app.pending_network_request, Some(NetworkRequest::PushTags { repo_path: path_string, remote_name: "upstream".to_string() }));
    join_network_worker(&mut app);
}

#[test]
fn cherrypick_opens_message_modal_with_prefilled_summary() {
    let (_path, repo) = temp_repo("cherrypick-modal");
    let oid = commit(&repo, "file.txt", "original summary\n\nbody");

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];

    app.on_cherrypick();

    assert_eq!(app.focus, Focus::ModalCherrypick);
    assert_eq!(app.pending_cherrypick_oid, Some(oid));
    assert_eq!(app.modal_input.value(), "cherrypicked: original summary");
}

#[test]
fn revert_opens_message_modal_with_prefilled_summary() {
    let (_path, repo) = temp_repo("revert-modal");
    let oid = commit(&repo, "file.txt", "original summary\n\nbody");

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];

    app.on_revert();

    assert_eq!(app.focus, Focus::ModalRevert);
    assert_eq!(app.pending_revert_oid, Some(oid));
    assert_eq!(app.modal_input.value(), "reverted: original summary");
}

#[test]
fn revert_rejects_merge_commits_before_opening_modal() {
    let (_path, repo) = temp_repo("revert-merge-modal");
    commit_with_content(&repo, "base.txt", "base\n", "base");
    let main_branch = current_branch_name(&repo);
    checkout_new_branch(&repo, "feature");
    let feature = commit_with_content(&repo, "feature.txt", "feature\n", "feature");
    checkout_branch(&repo, &main_branch);
    let main = commit_with_content(&repo, "main.txt", "main\n", "main");

    let merge = {
        let feature_commit = repo.find_commit(feature).unwrap();
        let main_commit = repo.find_commit(main).unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "merge", &tree, &[&main_commit, &feature_commit]).unwrap()
    };

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    let alias = app.oids.get_alias_by_oid(merge);
    app.oids.sorted_aliases = vec![NONE, alias];

    app.on_revert();

    assert_eq!(app.focus, Focus::ModalError);
    assert!(app.modal_error_message.contains("merge commits"));
    assert_eq!(app.pending_revert_oid, None);
}

#[test]
fn merge_queues_selected_commit_operation() {
    let (_path, repo) = temp_repo("merge-queue");
    let oid = commit(&repo, "file.txt", "merge target");

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];

    app.on_merge();

    assert_eq!(app.focus, Focus::ModalOperationProgress);
    assert_eq!(app.modal_operation_kind, OperationKind::Merge);
    assert_eq!(app.pending_operation_action, Some(PendingOperationAction::Start { kind: OperationKind::Merge, oid }));
}

#[test]
fn revert_state_routes_continue_and_abort_operations() {
    let (path, repo) = temp_repo("revert-active-operation");
    commit_with_content(&repo, "file.txt", "base\n", "base");
    let feature = commit_with_content(&repo, "file.txt", "feature\n", "feature");
    commit_with_content(&repo, "file.txt", "main\n", "main");

    assert_eq!(start_revert(&repo, feature, "reverted: feature").unwrap(), RevertOutcome::Conflict);

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    assert_active_operation_routes(&mut app, OperationKind::Revert, |app| app.on_revert());
    let _ = fs::remove_dir_all(path);
}

#[test]
fn cherrypick_state_routes_continue_and_abort_operations() {
    let (path, repo) = temp_repo("cherrypick-active-operation");
    commit_with_content(&repo, "file.txt", "base\n", "base");
    let feature = commit_with_content(&repo, "file.txt", "feature\n", "feature");
    commit_with_content(&repo, "file.txt", "main\n", "main");

    assert_eq!(start_cherrypick(&repo, feature, "cherrypicked: feature").unwrap(), CherrypickOutcome::Conflict);

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    assert_active_operation_routes(&mut app, OperationKind::Cherrypick, |app| app.on_continue_operation());
    let _ = fs::remove_dir_all(path);
}

#[test]
fn rebase_state_routes_continue_and_abort_operations() {
    let (path, repo) = temp_repo("rebase-active-operation");
    commit_with_content(&repo, "file.txt", "base\n", "base");
    let base_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    checkout_new_branch(&repo, "feature");
    commit_with_content(&repo, "file.txt", "feature\n", "feature");
    checkout_branch(&repo, &base_branch);
    let main = commit_with_content(&repo, "file.txt", "main\n", "main");
    checkout_branch(&repo, "feature");

    assert_eq!(start_rebase(&repo, main).unwrap(), RebaseOutcome::Conflict);

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    assert_active_operation_routes(&mut app, OperationKind::Rebase, |app| app.on_rebase());
    let _ = fs::remove_dir_all(path);
}

#[test]
fn merge_state_routes_continue_and_abort_operations() {
    let (path, repo) = temp_repo("merge-active-operation");
    commit_with_content(&repo, "file.txt", "base\n", "base");
    let main_branch = current_branch_name(&repo);
    checkout_new_branch(&repo, "feature");
    let feature = commit_with_content(&repo, "file.txt", "feature\n", "feature");
    checkout_branch(&repo, &main_branch);
    commit_with_content(&repo, "file.txt", "main\n", "main");

    assert_eq!(start_merge(&repo, feature).unwrap(), MergeOutcome::Conflict);

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    assert_active_operation_routes(&mut app, OperationKind::Merge, |app| app.on_continue_operation());
    let _ = fs::remove_dir_all(path);
}

#[test]
fn create_branch_from_reflog_uses_reflog_commit_target() {
    let (_path, repo) = temp_repo("reflog-branch-target");
    let graph_oid = commit(&repo, "graph.txt", "graph");
    let reflog_oid = commit(&repo, "reflog.txt", "reflog");

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Reflogs, graph_selected: 1, ..Default::default() };
    let graph_alias = app.oids.get_alias_by_oid(graph_oid);
    let reflog_alias = app.oids.get_alias_by_oid(reflog_oid);
    app.oids.sorted_aliases = vec![NONE, graph_alias, reflog_alias];
    app.reflogs.entries.push(HeadReflogAliasEntry {
        selector: "HEAD@{0}".to_string(),
        old_oid: graph_oid,
        new_oid: reflog_oid,
        new_alias: reflog_alias,
        message: "commit: reflog".to_string(),
        time: gix::date::Time::new(1, 0),
    });

    app.on_create_branch();

    assert_eq!(app.focus, Focus::ModalCreateBranch);
    assert_eq!(app.selected_branch_target_oid(), Some(reflog_oid));
}

#[test]
fn create_worktree_from_graph_uses_the_name_for_branch_and_default_path() {
    let (path, repo) = temp_repo("create-worktree-modal");
    let oid = commit(&repo, "file.txt", "initial");
    let path_string = path.display().to_string();
    let expected_path = path.parent().unwrap().join(format!("{}-feature", path.file_name().unwrap().to_string_lossy())).display().to_string();

    let mut app = App { path: Some(path_string.clone()), repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];

    app.on_create_worktree();

    assert_eq!(app.focus, Focus::ModalCreateWorktreeName);

    app.modal_input.set_value("feature");
    app.handle_modal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::ModalCreateWorktreePath);
    assert_eq!(app.modal_worktree_name, "feature");
    assert_eq!(app.modal_input.value(), expected_path);

    app.handle_modal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.focus, Focus::Viewport);
    assert!(app.modal_input.value().is_empty());
    assert!(Repository::open(&path_string).unwrap().find_worktree("feature").is_ok());
}

#[test]
fn rename_branch_from_pane_opens_prefilled_modal_for_local_branch() {
    let (_path, repo) = temp_repo("rename-pane-local");
    let oid = commit(&repo, "file.txt", "initial");
    let target = repo.find_commit(oid).unwrap();
    repo.branch("feature", &target, false).unwrap();
    drop(target);

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Branches, ..Default::default() };
    app.branches.sorted = vec![(1, "feature".to_string())];

    app.on_rename_branch();

    assert_eq!(app.focus, Focus::ModalRenameBranch);
    assert_eq!(app.modal_rename_branch_source.as_deref(), Some("feature"));
    assert_eq!(app.modal_input.value(), "feature");
}

#[test]
fn rename_branch_from_pane_rejects_remote_branch() {
    let (_path, repo) = temp_repo("rename-pane-remote");
    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Branches, ..Default::default() };
    app.branches.sorted = vec![(1, "origin/feature".to_string())];

    app.on_rename_branch();

    assert_eq!(app.focus, Focus::ModalError);
    assert!(app.modal_error_message.contains("only local branches"));
    assert_eq!(app.modal_rename_branch_source, None);
}

#[test]
fn rename_branch_from_graph_single_local_label_opens_modal() {
    let (_path, repo) = temp_repo("rename-graph-single");
    let oid = commit(&repo, "file.txt", "initial");
    let target = repo.find_commit(oid).unwrap();
    repo.branch("feature", &target, false).unwrap();
    drop(target);

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];
    app.branches.sorted = vec![(alias, "feature".to_string())];

    app.on_rename_branch();

    assert_eq!(app.focus, Focus::ModalRenameBranch);
    assert_eq!(app.modal_rename_branch_source.as_deref(), Some("feature"));
}

#[test]
fn rename_branch_from_graph_multiple_local_labels_uses_branch_choice_modal() {
    let (_path, repo) = temp_repo("rename-graph-multiple");
    let oid = commit(&repo, "file.txt", "initial");
    let target = repo.find_commit(oid).unwrap();
    repo.branch("feature", &target, false).unwrap();
    repo.branch("topic", &target, false).unwrap();
    drop(target);

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];
    app.branches.sorted = vec![(alias, "feature".to_string()), (alias, "topic".to_string())];

    app.on_rename_branch();

    assert_eq!(app.focus, Focus::ModalSolo);
    assert_eq!(app.modal_branch_action, BranchModalAction::Rename);

    app.modal_solo_selected = 1;
    app.on_select();

    assert_eq!(app.focus, Focus::ModalRenameBranch);
    assert_eq!(app.modal_rename_branch_source.as_deref(), Some("topic"));
    assert_eq!(app.modal_input.value(), "topic");
}

#[test]
fn rename_branch_from_graph_rejects_remote_only_labels() {
    let (_path, repo) = temp_repo("rename-graph-remote");
    let oid = commit(&repo, "file.txt", "initial");
    repo.reference("refs/remotes/origin/feature", oid, true, "remote").unwrap();

    let mut app = App { repo: Some(repo_handle(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
    let alias = app.oids.get_alias_by_oid(oid);
    app.oids.sorted_aliases = vec![NONE, alias];
    app.branches.sorted = vec![(alias, "origin/feature".to_string())];

    app.on_rename_branch();

    assert_eq!(app.focus, Focus::ModalError);
    assert!(app.modal_error_message.contains("only local branches"));
}

#[test]
fn auth_required_network_result_opens_auth_modal() {
    let challenge = AuthChallenge {
        url: "https://github.com/asinglebit/guitar.git".to_string(),
        username: Some("octo".to_string()),
        protocol: AuthProtocol::Https,
        operation: "Fetch".to_string(),
        key_path: None,
    };
    let mut app = App {
        pending_network_request: Some(NetworkRequest::Fetch { repo_path: ".".to_string(), remote_name: "origin".to_string() }),
        viewport: Viewport::Graph,
        focus: Focus::ModalNetworkProgress,
        ..Default::default()
    };

    app.handle_network_result(NetworkResult::AuthRequired(AuthRequired { challenge: challenge.clone(), rejected: Vec::new() }));

    assert_eq!(app.focus, Focus::ModalAuth);
    assert_eq!(app.pending_auth_prompt, Some(challenge));
    assert_eq!(app.auth_username_input.value(), "octo");
    assert_eq!(app.auth_input_field, AuthInputField::Secret);
}

#[test]
fn submitting_https_auth_stores_session_secret_and_retries_request() {
    let challenge = AuthChallenge { url: "https://github.com/asinglebit/guitar.git".to_string(), username: None, protocol: AuthProtocol::Https, operation: "Fetch".to_string(), key_path: None };
    let mut app = App {
        pending_network_request: Some(NetworkRequest::Fetch { repo_path: "/tmp/missing".to_string(), remote_name: "origin".to_string() }),
        pending_auth_prompt: Some(challenge.clone()),
        focus: Focus::ModalAuth,
        ..Default::default()
    };
    app.auth_username_input.set_value("octo");
    app.auth_secret_input.set_value("token");

    app.submit_auth_prompt();

    assert_eq!(app.pending_auth_prompt, None);
    assert_eq!(app.network_auth_attempts, 1);
    assert_eq!(app.focus, Focus::ModalNetworkProgress);
    let handle = app.network_handle.take().expect("retry should start a worker");
    let _ = handle.join();
    assert!(app.auth_session.has_secret_for(&challenge, Some("octo")));
}

#[test]
fn submitting_ssh_auth_stores_session_secret_and_retries_request() {
    let challenge = AuthChallenge {
        url: "ssh://git@github.com/asinglebit/guitar.git".to_string(),
        username: Some("git".to_string()),
        protocol: AuthProtocol::Ssh,
        operation: "Fetch".to_string(),
        key_path: Some(PathBuf::from("/tmp/id_ed25519")),
    };
    let mut app = App {
        pending_network_request: Some(NetworkRequest::Fetch { repo_path: "/tmp/missing".to_string(), remote_name: "origin".to_string() }),
        pending_auth_prompt: Some(challenge.clone()),
        focus: Focus::ModalAuth,
        ..Default::default()
    };
    app.auth_secret_input.set_value("passphrase");

    app.submit_auth_prompt();

    assert_eq!(app.pending_auth_prompt, None);
    assert_eq!(app.network_auth_attempts, 1);
    assert_eq!(app.focus, Focus::ModalNetworkProgress);
    let handle = app.network_handle.take().expect("retry should start a worker");
    let _ = handle.join();
    assert!(app.auth_session.has_secret_for(&challenge, Some("git")));
}

#[test]
fn cancelling_auth_prompt_clears_pending_network_state() {
    let mut app = App {
        pending_network_request: Some(NetworkRequest::Fetch { repo_path: ".".to_string(), remote_name: "origin".to_string() }),
        pending_auth_prompt: Some(AuthChallenge {
            url: "https://github.com/asinglebit/guitar.git".to_string(),
            username: None,
            protocol: AuthProtocol::Https,
            operation: "Fetch".to_string(),
            key_path: None,
        }),
        focus: Focus::ModalAuth,
        ..Default::default()
    };

    app.cancel_auth_prompt();

    assert!(app.pending_network_request.is_none());
    assert!(app.pending_auth_prompt.is_none());
    assert_eq!(app.focus, Focus::ModalError);
}
