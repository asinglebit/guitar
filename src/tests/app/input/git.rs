use super::*;
use crate::core::chunk::NONE;
use crate::core::reflogs::HeadReflogAliasEntry;
use crate::git::actions::merging::{MergeOutcome, start_merge};
use crate::git::actions::reverting::{RevertOutcome, start_revert};
use crate::git::auth::{AuthChallenge, AuthProtocol};
use git2::{Signature, build::CheckoutBuilder};
use std::{
    fs,
    path::Path,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (std::path::PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-input-git-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn commit(repo: &Repository, file: &str, message: &str) -> git2::Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), "content\n").unwrap();

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

fn commit_with_content(repo: &Repository, file: &str, content: &str, message: &str) -> git2::Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), content).unwrap();

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

#[test]
fn cherrypick_opens_message_modal_with_prefilled_summary() {
    let (_path, repo) = temp_repo("cherrypick-modal");
    let oid = commit(&repo, "file.txt", "original summary\n\nbody");

    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
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

    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
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
    checkout_new_branch(&repo, "feature");
    let feature = commit_with_content(&repo, "feature.txt", "feature\n", "feature");
    checkout_branch(&repo, "master");
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

    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
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

    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
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

    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    app.on_revert();

    assert_eq!(app.focus, Focus::ModalOperationProgress);
    assert_eq!(app.modal_operation_kind, OperationKind::Revert);
    assert_eq!(app.pending_operation_action, Some(PendingOperationAction::Continue));

    app.focus = Focus::Viewport;
    app.pending_operation_action = None;
    app.on_abort_operation();

    assert_eq!(app.focus, Focus::ModalOperationProgress);
    assert_eq!(app.modal_operation_kind, OperationKind::Revert);
    assert_eq!(app.pending_operation_action, Some(PendingOperationAction::Abort));
    let _ = fs::remove_dir_all(path);
}

#[test]
fn merge_state_routes_continue_and_abort_operations() {
    let (path, repo) = temp_repo("merge-active-operation");
    commit_with_content(&repo, "file.txt", "base\n", "base");
    checkout_new_branch(&repo, "feature");
    let feature = commit_with_content(&repo, "file.txt", "feature\n", "feature");
    checkout_branch(&repo, "master");
    commit_with_content(&repo, "file.txt", "main\n", "main");

    assert_eq!(start_merge(&repo, feature).unwrap(), MergeOutcome::Conflict);

    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
    app.on_continue_operation();

    assert_eq!(app.focus, Focus::ModalOperationProgress);
    assert_eq!(app.modal_operation_kind, OperationKind::Merge);
    assert_eq!(app.pending_operation_action, Some(PendingOperationAction::Continue));

    app.focus = Focus::Viewport;
    app.pending_operation_action = None;
    app.on_abort_operation();

    assert_eq!(app.focus, Focus::ModalOperationProgress);
    assert_eq!(app.modal_operation_kind, OperationKind::Merge);
    assert_eq!(app.pending_operation_action, Some(PendingOperationAction::Abort));
    let _ = fs::remove_dir_all(path);
}

#[test]
fn create_branch_from_reflog_uses_reflog_commit_target() {
    let (_path, repo) = temp_repo("reflog-branch-target");
    let graph_oid = commit(&repo, "graph.txt", "graph");
    let reflog_oid = commit(&repo, "reflog.txt", "reflog");

    let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Reflogs, graph_selected: 1, ..Default::default() };
    let graph_alias = app.oids.get_alias_by_oid(graph_oid);
    let reflog_alias = app.oids.get_alias_by_oid(reflog_oid);
    app.oids.sorted_aliases = vec![NONE, graph_alias, reflog_alias];
    app.reflogs.entries.push(HeadReflogAliasEntry {
        selector: "HEAD@{0}".to_string(),
        old_oid: graph_oid,
        new_oid: reflog_oid,
        new_alias: reflog_alias,
        message: "commit: reflog".to_string(),
        time: git2::Time::new(1, 0),
    });

    app.on_create_branch();

    assert_eq!(app.focus, Focus::ModalCreateBranch);
    assert_eq!(app.selected_branch_target_oid(), Some(reflog_oid));
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
