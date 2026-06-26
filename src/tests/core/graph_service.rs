use super::*;
use crate::helpers::symbols::SymbolTheme;
use git2::{Oid, Repository, Signature, build::CheckoutBuilder};
use im::HashSet;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool, mpsc::channel},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-graph-service-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn commit(repo: &Repository, file: &str, message: &str) -> Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), message).unwrap();

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

#[test]
fn graph_service_reports_progress_and_answers_visible_window() {
    let (path, repo) = temp_repo("window");
    commit(&repo, "one.txt", "one");
    let two = commit(&repo, "two.txt", "two");

    let generation = 42;
    let (cmd_tx, cmd_rx) = channel();
    let (event_tx, event_rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let handle = spawn_graph_service(
        GraphServiceConfig {
            generation,
            path: path.display().to_string(),
            amount: 1,
            hidden_branch_names: HashSet::new(),
            include_head_reflog_roots: false,
            graph_lane_limit: 20,
            worktrees: Vec::new(),
            symbols: SymbolTheme::main(),
        },
        cmd_rx,
        event_tx,
        cancel.clone(),
    );

    let mut saw_progress = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::Progress { generation: event_generation, total, .. } if event_generation == generation => {
                saw_progress = true;
                assert!(total > 0);
                break;
            },
            _ => {},
        }
    }
    assert!(saw_progress);

    cmd_tx.send(GraphCommand::QueryGraphWindow { generation, request_id: 7, start: 0, end: 2 }).unwrap();

    let mut saw_window = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::GraphWindow { generation: event_generation, request_id, rows, history, total, .. } if event_generation == generation && request_id == 7 => {
                saw_window = true;
                assert!(total >= rows.len());
                assert!(!rows.is_empty());
                assert!(!history.is_empty());
                assert!(rows.len() <= 2);
                break;
            },
            _ => {},
        }
    }
    assert!(saw_window);

    cmd_tx.send(GraphCommand::Lookup { generation, request_id: 8, kind: GraphLookupKind::ShaPrefix { prefix: two.to_string()[..8].to_string() } }).unwrap();

    let mut saw_lookup = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::LookupResult { generation: event_generation, request_id, result: GraphLookupResult::Index(Some(_)), .. } if event_generation == generation && request_id == 8 => {
                saw_lookup = true;
                break;
            },
            _ => {},
        }
    }
    assert!(saw_lookup);

    cmd_tx.send(GraphCommand::Lookup { generation, request_id: 9, kind: GraphLookupKind::Oid { oid: two } }).unwrap();

    let mut saw_oid_lookup = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::LookupResult { generation: event_generation, request_id, result: GraphLookupResult::Index(Some(index)), .. } if event_generation == generation && request_id == 9 => {
                saw_oid_lookup = true;
                assert_eq!(index, 1);
                break;
            },
            _ => {},
        }
    }
    assert!(saw_oid_lookup);

    let _ = cmd_tx.send(GraphCommand::Shutdown);
    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    handle.join().unwrap();
}

#[test]
fn graph_service_file_history_returns_visible_graph_indices() {
    let (path, repo) = temp_repo("file-history");
    let first = commit(&repo, "target.txt", "first");
    commit(&repo, "other.txt", "other");
    let latest = commit(&repo, "target.txt", "latest");

    let generation = 77;
    let (cmd_tx, cmd_rx) = channel();
    let (event_tx, event_rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let handle = spawn_graph_service(
        GraphServiceConfig {
            generation,
            path: path.display().to_string(),
            amount: 10000,
            hidden_branch_names: HashSet::new(),
            include_head_reflog_roots: false,
            graph_lane_limit: 20,
            worktrees: Vec::new(),
            symbols: SymbolTheme::main(),
        },
        cmd_rx,
        event_tx,
        cancel.clone(),
    );

    let mut complete = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::Progress { generation: event_generation, is_complete, .. } if event_generation == generation && is_complete => {
                complete = true;
                break;
            },
            _ => {},
        }
    }
    assert!(complete);

    cmd_tx.send(GraphCommand::QueryFileHistory { generation: generation + 1, request_id: 41, path: "target.txt".to_string() }).unwrap();
    cmd_tx.send(GraphCommand::QueryFileHistory { generation, request_id: 42, path: "target.txt".to_string() }).unwrap();

    let mut saw_history = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::FileHistory { generation: event_generation, request_id, path, rows, error } if event_generation == generation && request_id == 42 => {
                saw_history = true;
                assert_eq!(path, "target.txt");
                assert_eq!(error, None);
                assert_eq!(rows.iter().map(|row| row.graph_index).collect::<Vec<_>>(), vec![1, 3]);
                assert_eq!(rows.iter().map(|row| row.oid).collect::<Vec<_>>(), vec![latest, first]);
                break;
            },
            GraphEvent::FileHistory { request_id: 41, .. } => panic!("stale generation should not produce file history"),
            _ => {},
        }
    }
    assert!(saw_history);

    let _ = cmd_tx.send(GraphCommand::Shutdown);
    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    handle.join().unwrap();
}

#[test]
fn graph_service_uses_hidden_branch_names_as_deny_list() {
    let (path, repo) = temp_repo("hidden-branches");
    let root = commit(&repo, "root.txt", "root");
    let root_commit = repo.find_commit(root).unwrap();
    repo.branch("hidden", &root_commit, false).unwrap();
    let visible = commit(&repo, "visible.txt", "visible");

    repo.set_head("refs/heads/hidden").unwrap();
    repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
    let hidden = commit(&repo, "hidden.txt", "hidden");

    let generation = 88;
    let (cmd_tx, cmd_rx) = channel();
    let (event_tx, event_rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let handle = spawn_graph_service(
        GraphServiceConfig {
            generation,
            path: path.display().to_string(),
            amount: 10000,
            hidden_branch_names: hidden_set(&["hidden"]),
            include_head_reflog_roots: false,
            graph_lane_limit: 20,
            worktrees: Vec::new(),
            symbols: SymbolTheme::main(),
        },
        cmd_rx,
        event_tx,
        cancel.clone(),
    );

    let mut complete = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::Progress { generation: event_generation, is_complete, .. } if event_generation == generation && is_complete => {
                complete = true;
                break;
            },
            _ => {},
        }
    }
    assert!(complete);

    cmd_tx.send(GraphCommand::QueryGraphWindow { generation, request_id: 91, start: 0, end: 10 }).unwrap();

    let mut saw_window = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::GraphWindow { generation: event_generation, request_id: 91, rows, .. } if event_generation == generation => {
                saw_window = true;
                assert!(rows.iter().any(|row| row.oid == visible));
                assert!(!rows.iter().any(|row| row.oid == hidden));
                break;
            },
            _ => {},
        }
    }
    assert!(saw_window);

    cmd_tx.send(GraphCommand::QueryPaneWindow { generation, pane: GraphPane::Branches, start: 0, end: 10 }).unwrap();

    let mut saw_pane = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::PaneWindow { generation: event_generation, pane: GraphPane::Branches, rows, .. } if event_generation == generation => {
                saw_pane = true;
                assert!(rows.iter().any(|row| matches!(row, GraphPaneRow::Branch { name, .. } if name == "hidden")));
                break;
            },
            _ => {},
        }
    }
    assert!(saw_pane);

    let _ = cmd_tx.send(GraphCommand::Shutdown);
    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    handle.join().unwrap();
}

#[test]
fn graph_service_omits_hidden_labels_on_visible_commits() {
    let (path, repo) = temp_repo("hidden-labels");
    let oid = commit(&repo, "one.txt", "one");
    let commit = repo.find_commit(oid).unwrap();
    repo.branch("hidden", &commit, false).unwrap();
    repo.reference("refs/remotes/origin/archive", oid, true, "test").unwrap();

    let generation = 89;
    let (cmd_tx, cmd_rx) = channel();
    let (event_tx, event_rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let handle = spawn_graph_service(
        GraphServiceConfig {
            generation,
            path: path.display().to_string(),
            amount: 10000,
            hidden_branch_names: hidden_set(&["hidden", "origin/archive"]),
            include_head_reflog_roots: false,
            graph_lane_limit: 20,
            worktrees: Vec::new(),
            symbols: SymbolTheme::main(),
        },
        cmd_rx,
        event_tx,
        cancel.clone(),
    );

    let mut complete = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::Progress { generation: event_generation, is_complete, .. } if event_generation == generation && is_complete => {
                complete = true;
                break;
            },
            _ => {},
        }
    }
    assert!(complete);

    cmd_tx.send(GraphCommand::QueryGraphWindow { generation, request_id: 92, start: 0, end: 2 }).unwrap();

    let mut saw_window = false;
    for _ in 0..20 {
        match event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
            GraphEvent::GraphWindow { generation: event_generation, request_id: 92, rows, .. } if event_generation == generation => {
                saw_window = true;
                let row = rows.iter().find(|row| row.oid == oid).unwrap();
                let labels: Vec<_> = row.branches.iter().map(|branch| branch.name.as_str()).collect();
                assert!(!labels.contains(&"hidden"));
                assert!(!labels.contains(&"origin/archive"));
                assert!(labels.contains(&"master"));
                break;
            },
            _ => {},
        }
    }
    assert!(saw_window);

    let _ = cmd_tx.send(GraphCommand::Shutdown);
    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    handle.join().unwrap();
}

fn hidden_set(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}
