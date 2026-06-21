mod fixtures;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, channel},
    },
    time::{Duration, Instant},
};

use divan::{Bencher, black_box};
use fixtures::{GraphServiceFixture, graph_service_fixture};
use guitar::core::graph_service::{GraphCommand, GraphEvent, GraphServiceConfig, spawn_graph_service};

fn main() {
    divan::main();
}

const SERVICE_WAIT: Duration = Duration::from_secs(10);

struct RunningService {
    cmd_tx: Sender<GraphCommand>,
    event_rx: Receiver<GraphEvent>,
    handle: std::thread::JoinHandle<()>,
    cancel: Arc<AtomicBool>,
    generation: u64,
}

fn spawn_fixture_service(fixture: &GraphServiceFixture) -> RunningService {
    let generation = 7;
    let (cmd_tx, cmd_rx) = channel();
    let (event_tx, event_rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let handle = spawn_graph_service(
        GraphServiceConfig {
            generation,
            path: fixture.path.display().to_string(),
            amount: fixture.amount,
            hidden_branch_names: fixture.hidden_branch_names.clone(),
            include_head_reflog_roots: fixture.include_head_reflog_roots,
            graph_lane_limit: fixture.graph_lane_limit,
            worktrees: fixture.worktrees.clone(),
            symbols: fixture.symbols.clone(),
        },
        cmd_rx,
        event_tx,
        cancel.clone(),
    );

    RunningService { cmd_tx, event_rx, handle, cancel, generation }
}

fn wait_for_completion(service: &RunningService) -> usize {
    let deadline = Instant::now() + SERVICE_WAIT;
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            panic!("timed out waiting for graph service completion");
        };
        match service.event_rx.recv_timeout(remaining) {
            Ok(GraphEvent::Progress { generation: event_generation, total, is_complete, .. }) if event_generation == service.generation && is_complete => {
                return total;
            },
            Ok(_) => {},
            Err(error) => panic!("graph service closed before completion: {error}"),
        }
    }
}

fn wait_for_graph_window(service: &RunningService, request_id: u64) -> usize {
    let deadline = Instant::now() + SERVICE_WAIT;
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            panic!("timed out waiting for graph window request {request_id}");
        };
        match service.event_rx.recv_timeout(remaining) {
            Ok(GraphEvent::GraphWindow { generation: event_generation, request_id: event_request_id, rows, history, .. })
                if event_generation == service.generation && event_request_id == request_id =>
            {
                return rows.len() + history.len();
            },
            Ok(_) => {},
            Err(error) => panic!("graph service closed before graph window request {request_id}: {error}"),
        }
    }
}

fn wait_for_pane_window(service: &RunningService, pane: guitar::core::graph_service::GraphPane) -> usize {
    let deadline = Instant::now() + SERVICE_WAIT;
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            panic!("timed out waiting for pane window request {pane:?}");
        };
        match service.event_rx.recv_timeout(remaining) {
            Ok(GraphEvent::PaneWindow { generation: event_generation, pane: event_pane, rows, .. }) if event_generation == service.generation && event_pane == pane => {
                return rows.len();
            },
            Ok(_) => {},
            Err(error) => panic!("graph service closed before pane window request {pane:?}: {error}"),
        }
    }
}

fn shutdown_service(service: RunningService) {
    let _ = service.cmd_tx.send(GraphCommand::Shutdown);
    service.cancel.store(true, Ordering::SeqCst);
    service.handle.join().expect("graph service worker panicked");
}

fn graph_service_repeated_window_roundtrip(service: &RunningService, total: usize, start: usize, end: usize) -> usize {
    let mut rows = 0;
    for request_id in [12, 13] {
        service.cmd_tx.send(GraphCommand::QueryGraphWindow { generation: service.generation, request_id, start, end }).unwrap();
        rows += wait_for_graph_window(service, request_id);
    }

    black_box(total + rows)
}

fn graph_service_repeated_pane_roundtrip(service: &RunningService, pane: guitar::core::graph_service::GraphPane, start: usize, end: usize) -> usize {
    let mut rows = 0;
    for _ in 0..4 {
        service.cmd_tx.send(GraphCommand::QueryPaneWindow { generation: service.generation, pane, start, end }).unwrap();
        rows += wait_for_pane_window(service, pane);
    }

    black_box(rows)
}

fn graph_service_initial_load(rounds: usize) -> usize {
    let fixture = graph_service_fixture(rounds);
    let service = spawn_fixture_service(&fixture);
    let total = wait_for_completion(&service);
    shutdown_service(service);
    black_box(total)
}

#[divan::bench(sample_count = 20, sample_size = 10)]
fn graph_service_initial_load_medium(bencher: Bencher) {
    bencher.counter(divan::counter::ItemsCount::new(12usize.saturating_mul(8))).bench_local(|| black_box(graph_service_initial_load(12)));
}

#[divan::bench(sample_count = 50, sample_size = 25)]
fn graph_service_repeated_windows_small(bencher: Bencher) {
    let fixture = graph_service_fixture(4);
    let service = spawn_fixture_service(&fixture);
    let total = wait_for_completion(&service);

    bencher.counter(divan::counter::ItemsCount::new(fixture.amount.saturating_mul(2))).bench_local(|| black_box(graph_service_repeated_window_roundtrip(&service, total, 0, 24)));
    shutdown_service(service);
}

#[divan::bench(sample_count = 50, sample_size = 25)]
fn graph_service_repeated_branch_panes_small(bencher: Bencher) {
    let fixture = graph_service_fixture(4);
    let service = spawn_fixture_service(&fixture);
    let _total = wait_for_completion(&service);

    bencher
        .counter(divan::counter::ItemsCount::new(24usize.saturating_mul(4)))
        .bench_local(|| black_box(graph_service_repeated_pane_roundtrip(&service, guitar::core::graph_service::GraphPane::Branches, 0, 24)));
    shutdown_service(service);
}
