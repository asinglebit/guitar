use super::*;
use crate::{
    app::{
        app::{GraphWindowCache, Viewport},
        state::layout::Layout,
    },
    core::{
        chunk::NONE,
        graph_service::{GraphCommand, GraphFileHistoryRow, GraphHistory, GraphRow, GraphSnapshot},
    },
    git::queries::helpers::FileStatus,
    helpers::symbols::SymbolTheme,
};
use git2::{Oid, Repository, Signature};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository, Oid) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-graph-draw-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    fs::write(path.join("file.txt"), "content\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let oid = repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &[]).unwrap();
    drop(tree);
    (path, repo, oid)
}

fn temp_unborn_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-graph-draw-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    (path, repo)
}

fn graph_row(index: usize, alias: u32, oid: Oid, summary: &str) -> GraphRow {
    GraphRow {
        index,
        alias,
        oid,
        short_oid: oid.to_string()[..8].to_string(),
        summary: summary.to_string(),
        committer_date: String::new(),
        committer_name: String::new(),
        is_merge: false,
        has_any_branch: false,
        branches: Vec::new(),
        tags: Vec::new(),
        is_stash: false,
        stash_lane: None,
        worktrees: Vec::new(),
        has_current_worktree: false,
        reflog: None,
    }
}

fn history_row(graph_index: usize, oid: Oid) -> GraphFileHistoryRow {
    GraphFileHistoryRow { graph_index, oid, short_oid: oid.to_string()[..8].to_string(), summary: "history".to_string(), status: FileStatus::Modified }
}

fn app_with_cached_window(start: usize, summaries: &[&str], oid: Oid) -> App {
    let mut app = App {
        viewport: Viewport::Graph,
        focus: Focus::Viewport,
        layout: Layout { graph: Rect::new(0, 0, 80, 3), graph_scrollbar: Rect::new(79, 0, 1, 3), ..Default::default() },
        ..Default::default()
    };
    app.layout_config.is_shas = false;
    app.layout_config.is_zen = false;
    app.graph.total = 4;
    app.graph.graph_window = Some(GraphWindowCache {
        version: 1,
        start,
        end: start + summaries.len(),
        head_alias: 1,
        rows: summaries.iter().enumerate().map(|(offset, summary)| graph_row(start + offset, (start + offset + 1) as u32, oid, summary)).collect(),
        history: GraphHistory::new(),
    });
    app
}

fn graph_history(len: usize) -> GraphHistory {
    let mut history = GraphHistory::new();
    for _ in 0..len {
        history.push(GraphSnapshot::default());
    }
    history
}

fn app_with_uncommitted_window(window_end: usize, history_len: usize, oid: Oid) -> App {
    let mut app = App {
        viewport: Viewport::Graph,
        focus: Focus::Viewport,
        layout: Layout { graph: Rect::new(0, 0, 80, 3), graph_scrollbar: Rect::new(79, 0, 1, 3), ..Default::default() },
        ..Default::default()
    };
    app.layout_config.is_shas = false;
    app.layout_config.is_zen = false;
    app.graph.total = 3;
    app.uncommitted.modified_count = 2;
    app.graph.graph_window = Some(GraphWindowCache {
        version: 1,
        start: 0,
        end: window_end,
        head_alias: 1,
        rows: (0..window_end).map(|index| if index == 0 { graph_row(index, NONE, Oid::zero(), "") } else { graph_row(index, index as u32, oid, &format!("row{index}")) }).collect(),
        history: graph_history(history_len),
    });
    app
}

fn rendered_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height).map(|y| (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect::<String>()).collect()
}

#[test]
fn graph_highlights_file_history_rows_when_search_pane_is_open() {
    let (_path, repo, oid) = temp_repo("file-search-highlight");
    let mut app = app_with_cached_window(0, &["uncommitted", "touch searched file", "other commit"], oid);
    app.focus = Focus::Search;
    app.layout_config.is_search = true;
    app.graph_selected = 2;
    app.search_path = Some("src/lib.rs".to_string());
    app.search_rows = vec![history_row(1, oid)];

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    let selected_bg = app.theme.background_or_default(app.theme.COLOR_GREY_800);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 1)].bg, selected_bg);
    assert_ne!(buffer[(1, 2)].bg, selected_bg);
}

#[test]
fn graph_does_not_highlight_file_history_rows_when_search_pane_is_closed() {
    let (_path, repo, oid) = temp_repo("file-search-highlight-closed");
    let mut app = app_with_cached_window(0, &["uncommitted", "touch searched file", "other commit"], oid);
    app.focus = Focus::Search;
    app.layout_config.is_search = false;
    app.graph_selected = 2;
    app.search_path = Some("src/lib.rs".to_string());
    app.search_rows = vec![history_row(1, oid)];

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    let selected_bg = app.theme.background_or_default(app.theme.COLOR_GREY_800);
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[(1, 1)].bg, selected_bg);
}

#[test]
fn graph_cached_rows_shift_up_when_requested_window_moves_down() {
    let (_path, repo, oid) = temp_repo("shift-down");
    let mut app = app_with_cached_window(0, &["row0", "row1", "row2"], oid);
    app.graph_selected = 1;
    app.graph_scroll.set(1);

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    let lines = rendered_lines(&terminal);
    assert!(lines[0].contains("row1"), "{lines:?}");
    assert!(lines[1].contains("row2"), "{lines:?}");
    assert!(!lines.iter().any(|line| line.contains("row0")), "{lines:?}");
    assert!(!lines[2].contains("row"), "{lines:?}");
}

#[test]
fn graph_cached_rows_shift_down_when_requested_window_moves_up() {
    let (_path, repo, oid) = temp_repo("shift-up");
    let mut app = app_with_cached_window(1, &["row1", "row2", "row3"], oid);
    app.graph_selected = 0;
    app.graph_scroll.set(0);

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    let lines = rendered_lines(&terminal);
    assert!(!lines[0].contains("row"), "{lines:?}");
    assert!(lines[1].contains("row1"), "{lines:?}");
    assert!(lines[2].contains("row2"), "{lines:?}");
    assert!(!lines.iter().any(|line| line.contains("row3")), "{lines:?}");
}

#[test]
fn graph_short_page_stripes_blank_tail_rows() {
    let (_path, repo, oid) = temp_repo("blank-tail");
    let mut app = app_with_cached_window(0, &["row0"], oid);
    app.graph.total = 1;
    app.graph_selected = 0;
    app.graph_scroll.set(0);
    let zebra = app.theme.background_or_default(app.theme.COLOR_GREY_900);

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    let lines = rendered_lines(&terminal);
    assert!(lines[0].contains("row0"), "{lines:?}");
    assert!(!lines[2].contains("row"), "{lines:?}");
    assert_eq!(terminal.backend().buffer()[(1, 2)].bg, zebra);
}

#[test]
fn graph_ascii_symbol_theme_renders_ascii_only_output() {
    let (_path, repo, oid) = temp_repo("ascii-theme");
    let mut app = app_with_cached_window(0, &["row0", "row1"], oid);
    app.symbols = SymbolTheme::ascii();

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_graph(frame, &repo)).unwrap();

    let rendered = rendered_lines(&terminal).join("");
    assert!(rendered.is_ascii(), "{rendered:?}");
}

#[test]
fn graph_empty_state_stripes_backdrop() {
    let (_path, repo) = temp_unborn_repo("empty-backdrop");
    let mut app = App {
        viewport: Viewport::Graph,
        focus: Focus::Viewport,
        layout: Layout { graph: Rect::new(0, 0, 80, 3), graph_scrollbar: Rect::new(79, 0, 1, 3), ..Default::default() },
        ..Default::default()
    };
    app.layout_config.is_zen = false;
    let zebra = app.theme.background_or_default(app.theme.COLOR_GREY_900);

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    let lines = rendered_lines(&terminal);
    assert!(lines.iter().any(|line| line.contains("⊘ no commits")), "{lines:?}");
    assert_eq!(terminal.backend().buffer()[(1, 2)].bg, zebra);
}

#[test]
fn uncommitted_row_waits_for_visible_page_before_rendering() {
    let (_path, repo, oid) = temp_repo("uncommitted-waits");
    let mut app = app_with_uncommitted_window(2, 2, oid);
    app.graph_selected = 0;
    app.graph_scroll.set(0);

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    let lines = rendered_lines(&terminal);
    assert!(!lines[0].contains("◌"), "{lines:?}");
    assert!(!lines[0].contains("~ 2"), "{lines:?}");
}

#[test]
fn uncommitted_row_renders_when_visible_page_is_ready() {
    let (_path, repo, oid) = temp_repo("uncommitted-ready");
    let mut app = app_with_uncommitted_window(3, 3, oid);
    app.graph_selected = 0;
    app.graph_scroll.set(0);

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    let lines = rendered_lines(&terminal);
    assert!(lines[0].contains("◌"), "{lines:?}");
    assert!(lines[0].contains("~ 2"), "{lines:?}");
}

#[test]
fn graph_draw_prefetches_one_screen_before_and_after_visible_window() {
    let (_path, repo, _oid) = temp_repo("prefetch-window");
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App {
        viewport: Viewport::Graph,
        focus: Focus::Viewport,
        graph_tx: Some(tx),
        layout: Layout { graph: Rect::new(0, 0, 80, 3), graph_scrollbar: Rect::new(79, 0, 1, 3), ..Default::default() },
        ..Default::default()
    };
    app.layout_config.is_shas = false;
    app.layout_config.is_zen = false;
    app.graph.generation = 7;
    app.graph.total = 20;
    app.graph_selected = 5;
    app.graph_scroll.set(5);

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    match rx.try_recv().unwrap() {
        GraphCommand::QueryGraphWindow { generation, request_id, start, end } => {
            assert_eq!(generation, 7);
            assert_eq!(request_id, 1);
            assert_eq!((start, end), (2, 11));
        },
        other => panic!("expected graph window request, got {other:?}"),
    }
}

#[test]
fn graph_draw_keeps_prefetched_rows_out_of_visible_table() {
    let (_path, repo, oid) = temp_repo("prefetch-render");
    let mut app = app_with_cached_window(2, &["row2", "row3", "row4", "row5", "row6", "row7", "row8", "row9", "row10"], oid);
    app.graph.total = 20;
    app.graph_selected = 5;
    app.graph_scroll.set(5);

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    let lines = rendered_lines(&terminal);
    assert!(lines[0].contains("row5"), "{lines:?}");
    assert!(lines[1].contains("row6"), "{lines:?}");
    assert!(lines[2].contains("row7"), "{lines:?}");
    assert!(!lines.iter().any(|line| line.contains("row4")), "{lines:?}");
    assert!(!lines.iter().any(|line| line.contains("row8")), "{lines:?}");
}

#[test]
fn zero_sized_graph_draw_does_not_request_empty_window() {
    let (_path, repo, oid) = temp_repo("zero-graph");
    let mut app = app_with_cached_window(0, &["row0"], oid);
    let (tx, rx) = std::sync::mpsc::channel();
    app.graph_tx = Some(tx);
    app.graph.generation = 7;
    app.layout.graph = Rect::new(0, 0, 0, 0);
    app.layout.graph_scrollbar = Rect::new(0, 0, 0, 0);

    let backend = TestBackend::new(20, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            app.draw_graph(frame, &repo);
        })
        .unwrap();

    assert!(rx.try_recv().is_err());
}
