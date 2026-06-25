use super::*;
use crate::helpers::symbols::SymbolTheme;
use git2::{Oid, Repository, Signature, build::CheckoutBuilder};
use im::HashSet;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{Receiver, Sender, channel},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-graph-service-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = crate::git::test_support::init_repo_at(&path);
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

struct GraphServiceHarness {
    generation: u64,
    cmd_tx: Sender<GraphCommand>,
    event_rx: Receiver<GraphEvent>,
    cancel: Arc<AtomicBool>,
    handle: std::thread::JoinHandle<()>,
}

impl GraphServiceHarness {
    fn spawn(path: PathBuf, generation: u64, amount: usize, hidden_branch_names: HashSet<String>, include_head_reflog_roots: bool, worktrees: Vec<WorktreeEntry>) -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (event_tx, event_rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let handle = spawn_graph_service(
            GraphServiceConfig { generation, path: path.display().to_string(), amount, hidden_branch_names, include_head_reflog_roots, graph_lane_limit: 20, worktrees, symbols: SymbolTheme::main() },
            cmd_rx,
            event_tx,
            cancel.clone(),
        );

        Self { generation, cmd_tx, event_rx, cancel, handle }
    }

    fn wait_for_progress(&self, is_complete: bool) -> usize {
        for _ in 0..20 {
            match self.event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
                GraphEvent::Progress { generation, total, is_complete: event_complete, .. } if generation == self.generation && event_complete == is_complete => {
                    return total;
                },
                _ => {},
            }
        }
        panic!("timed out waiting for graph service progress");
    }

    fn wait_for_window(&self, request_id: u64) -> (Vec<GraphRow>, GraphHistory, usize) {
        for _ in 0..20 {
            match self.event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
                GraphEvent::GraphWindow { generation, request_id: event_request_id, rows, history, total, .. } if generation == self.generation && event_request_id == request_id => {
                    return (rows, history, total);
                },
                _ => {},
            }
        }
        panic!("timed out waiting for graph window");
    }

    fn wait_for_lookup(&self, request_id: u64) -> GraphLookupResult {
        for _ in 0..20 {
            match self.event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
                GraphEvent::LookupResult { generation, request_id: event_request_id, result, .. } if generation == self.generation && event_request_id == request_id => {
                    return result;
                },
                _ => {},
            }
        }
        panic!("timed out waiting for lookup result");
    }

    fn wait_for_worktrees(&self) -> (u64, Vec<WorktreeEntry>) {
        for _ in 0..20 {
            match self.event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
                GraphEvent::Worktrees { generation, version, worktrees } if generation == self.generation => {
                    return (version, worktrees);
                },
                _ => {},
            }
        }
        panic!("timed out waiting for worktree update");
    }

    fn wait_for_pane_window(&self, pane: GraphPane) -> Vec<GraphPaneRow> {
        for _ in 0..20 {
            match self.event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
                GraphEvent::PaneWindow { generation, pane: event_pane, rows, .. } if generation == self.generation && event_pane == pane => {
                    return rows;
                },
                _ => {},
            }
        }
        panic!("timed out waiting for pane window");
    }

    fn wait_for_file_history(&self, request_id: u64) -> (String, Vec<GraphFileHistoryRow>, Option<String>) {
        for _ in 0..20 {
            match self.event_rx.recv_timeout(Duration::from_millis(250)).unwrap() {
                GraphEvent::FileHistory { generation, request_id: event_request_id, path, rows, error } if generation == self.generation && event_request_id == request_id => {
                    return (path, rows, error);
                },
                _ => {},
            }
        }
        panic!("timed out waiting for file history");
    }

    fn send(&self, command: GraphCommand) {
        self.cmd_tx.send(command).unwrap();
    }

    fn shutdown(self) {
        let _ = self.cmd_tx.send(GraphCommand::Shutdown);
        self.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        self.handle.join().unwrap();
    }
}

#[test]
fn graph_service_reports_progress_and_answers_visible_window() {
    let (path, repo) = temp_repo("window");
    commit(&repo, "one.txt", "one");
    let two = commit(&repo, "two.txt", "two");
    let harness = GraphServiceHarness::spawn(path.clone(), 42, 1, HashSet::new(), false, Vec::new());

    assert!(harness.wait_for_progress(false) > 0);

    harness.send(GraphCommand::QueryGraphWindow { generation: 42, request_id: 7, start: 0, end: 2 });
    let (rows, history, total) = harness.wait_for_window(7);
    assert!(total >= rows.len());
    assert!(!rows.is_empty());
    assert!(!history.is_empty());
    assert!(rows.len() <= 2);

    harness.send(GraphCommand::Lookup { generation: 42, request_id: 8, kind: GraphLookupKind::ShaPrefix { prefix: two.to_string()[..8].to_string() } });
    assert!(matches!(harness.wait_for_lookup(8), GraphLookupResult::Index(Some(_))));

    harness.send(GraphCommand::Lookup { generation: 42, request_id: 9, kind: GraphLookupKind::Oid { oid: two } });
    assert!(matches!(harness.wait_for_lookup(9), GraphLookupResult::Index(Some(1))));

    let updated = vec![WorktreeEntry {
        name: "repo".to_string(),
        path,
        branch: Some("master".to_string()),
        head: Some(two),
        alias: None,
        kind: crate::core::worktrees::WorktreeKind::Main,
        is_current: true,
        is_valid: true,
        is_prunable: false,
        locked_reason: None,
        is_dirty: true,
    }];
    harness.send(GraphCommand::UpdateWorktrees { generation: 42, worktrees: updated.clone() });
    let (_, worktrees) = harness.wait_for_worktrees();
    assert_eq!(worktrees, updated);

    harness.shutdown();
}

#[test]
fn graph_service_file_history_returns_visible_graph_indices() {
    let (path, repo) = temp_repo("file-history");
    let first = commit(&repo, "target.txt", "first");
    commit(&repo, "other.txt", "other");
    let latest = commit(&repo, "target.txt", "latest");
    let harness = GraphServiceHarness::spawn(path, 77, 10000, HashSet::new(), false, Vec::new());

    assert!(harness.wait_for_progress(true) > 0);

    harness.send(GraphCommand::QueryFileHistory { generation: 78, request_id: 41, path: "target.txt".to_string() });
    harness.send(GraphCommand::QueryFileHistory { generation: 77, request_id: 42, path: "target.txt".to_string() });

    let (path, rows, error) = harness.wait_for_file_history(42);
    assert_eq!(path, "target.txt");
    assert_eq!(error, None);
    assert_eq!(rows.iter().map(|row| row.graph_index).collect::<Vec<_>>(), vec![1, 3]);
    assert_eq!(rows.iter().map(|row| row.oid).collect::<Vec<_>>(), vec![latest, first]);

    harness.shutdown();
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

    let harness = GraphServiceHarness::spawn(path, 88, 10000, hidden_set(&["hidden"]), false, Vec::new());

    assert!(harness.wait_for_progress(true) > 0);

    harness.send(GraphCommand::QueryGraphWindow { generation: 88, request_id: 91, start: 0, end: 10 });
    let (rows, _, _) = harness.wait_for_window(91);
    assert!(rows.iter().any(|row| row.oid == visible));
    assert!(!rows.iter().any(|row| row.oid == hidden));

    harness.send(GraphCommand::QueryPaneWindow { generation: 88, pane: GraphPane::Branches, start: 0, end: 10 });
    let rows = harness.wait_for_pane_window(GraphPane::Branches);
    assert!(rows.iter().any(|row| matches!(row, GraphPaneRow::Branch { name, .. } if name == "hidden")));

    harness.shutdown();
}

#[test]
fn graph_service_omits_hidden_labels_on_visible_commits() {
    let (path, repo) = temp_repo("hidden-labels");
    let oid = commit(&repo, "one.txt", "one");
    let current_branch = repo.head().unwrap().shorthand().unwrap().to_string();
    let commit = repo.find_commit(oid).unwrap();
    repo.branch("hidden", &commit, false).unwrap();
    repo.reference("refs/remotes/origin/archive", oid, true, "test").unwrap();

    let harness = GraphServiceHarness::spawn(path, 89, 10000, hidden_set(&["hidden", "origin/archive"]), false, Vec::new());

    assert!(harness.wait_for_progress(true) > 0);

    harness.send(GraphCommand::QueryGraphWindow { generation: 89, request_id: 92, start: 0, end: 2 });
    let (rows, _, _) = harness.wait_for_window(92);
    let row = rows.iter().find(|row| row.oid == oid).unwrap();
    let labels: Vec<_> = row.branches.iter().map(|branch| branch.name.as_str()).collect();
    assert!(!labels.contains(&"hidden"));
    assert!(!labels.contains(&"origin/archive"));
    assert!(labels.contains(&current_branch.as_str()));

    harness.shutdown();
}

fn hidden_set(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}
