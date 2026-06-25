use super::*;
use crate::{
    app::state::layout::Layout,
    core::graph_service::{GraphIndexIdentity, GraphRow},
    git::queries::helpers::{FileChange, FileChanges},
};
use git2::Oid;
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};

fn status_app() -> App {
    let mut app = App {
        layout: Layout {
            status_top: Rect::new(0, 0, 48, 5),
            status_top_scrollbar: Rect::new(47, 0, 1, 5),
            status_bottom: Rect::new(0, 5, 48, 5),
            status_bottom_scrollbar: Rect::new(47, 5, 1, 5),
            ..Default::default()
        },
        ..Default::default()
    };
    app.layout_config.is_zen = false;
    app.layout_config.is_inspector = false;
    app
}

fn rendered(terminal: &Terminal<TestBackend>) -> String {
    terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>()
}

fn row_contains(buffer: &Buffer, row: u16, text: &str) -> bool {
    let line = (0..buffer.area.width).map(|x| buffer[(x, row)].symbol()).collect::<String>();
    line.contains(text)
}

fn path_col(buffer: &Buffer, row: u16, symbol: &str) -> u16 {
    (0..buffer.area.width).find(|&x| buffer[(x, row)].symbol() == symbol).unwrap()
}

fn graph_row(index: usize, alias: u32, oid: Oid) -> GraphRow {
    GraphRow {
        index,
        alias,
        oid,
        summary: "summary".to_string(),
        committer_date: String::new(),
        committer_name: String::new(),
        is_merge: false,
        has_any_branch: false,
        branches: Default::default(),
        tags: Default::default(),
        is_stash: false,
        stash_lane: None,
        worktrees: Default::default(),
        has_current_worktree: false,
        reflog: None,
    }
}

#[test]
fn status_highlights_matching_staged_and_unstaged_paths_when_search_pane_is_open() {
    let mut app = status_app();
    app.graph_selected = 0;
    app.is_uncommitted_loaded = true;
    app.focus = Focus::Viewport;
    app.layout_config.is_search = true;
    app.search_path = Some("src/selected.rs".to_string());
    app.uncommitted.staged = FileChanges { modified: vec!["src/selected.rs".to_string(), "src/other.rs".to_string()], ..Default::default() };
    app.uncommitted.unstaged = FileChanges { added: vec!["src/selected.rs".to_string()], ..Default::default() };

    let backend = TestBackend::new(48, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_status(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let selected_bg = app.theme.background_or_default(app.theme.COLOR_GREY_800);
    let top_col = path_col(buffer, 0, "s");
    let bottom_col = path_col(buffer, 6, "s");

    assert!(row_contains(buffer, 0, "src/selected.rs"));
    assert_eq!(buffer[(top_col, 0)].bg, selected_bg);
    assert_eq!(buffer[(top_col, 0)].fg, app.theme.COLOR_HIGHLIGHTED);
    assert!(row_contains(buffer, 6, "src/selected.rs"));
    assert_eq!(buffer[(bottom_col, 6)].bg, selected_bg);
    assert_eq!(buffer[(bottom_col, 6)].fg, app.theme.COLOR_HIGHLIGHTED);
}

#[test]
fn status_highlights_matching_conflict_paths_when_search_pane_is_open() {
    let mut app = status_app();
    app.graph_selected = 0;
    app.is_uncommitted_loaded = true;
    app.focus = Focus::Viewport;
    app.layout_config.is_search = true;
    app.search_path = Some("src/conflict.rs".to_string());
    app.uncommitted.conflicts = vec!["src/conflict.rs".to_string()];

    let backend = TestBackend::new(48, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_status(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let selected_bg = app.theme.background_or_default(app.theme.COLOR_GREY_800);
    let top_col = path_col(buffer, 0, "s");
    let bottom_col = path_col(buffer, 6, "s");

    assert!(row_contains(buffer, 0, "src/conflict.rs"));
    assert_eq!(buffer[(top_col, 0)].bg, selected_bg);
    assert_eq!(buffer[(top_col, 0)].fg, app.theme.COLOR_HIGHLIGHTED);
    assert!(row_contains(buffer, 6, "src/conflict.rs"));
    assert_eq!(buffer[(bottom_col, 6)].bg, selected_bg);
    assert_eq!(buffer[(bottom_col, 6)].fg, app.theme.COLOR_HIGHLIGHTED);
}

#[test]
fn status_does_not_highlight_partial_matches_or_closed_search_pane() {
    let selected_bg;
    {
        let mut app = status_app();
        app.graph_selected = 0;
        app.is_uncommitted_loaded = true;
        app.focus = Focus::Viewport;
        app.layout_config.is_search = true;
        app.search_path = Some("src/selected.rs".to_string());
        app.uncommitted.staged = FileChanges { modified: vec!["src/selected.rs.bak".to_string()], ..Default::default() };
        selected_bg = app.theme.background_or_default(app.theme.COLOR_GREY_800);

        let backend = TestBackend::new(48, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.draw_status(frame)).unwrap();

        let buffer = terminal.backend().buffer();
        let col = path_col(buffer, 0, "s");
        assert!(row_contains(buffer, 0, "src/selected.rs.bak"));
        assert_ne!(buffer[(col, 0)].bg, selected_bg);
    }

    let mut app = status_app();
    app.graph_selected = 0;
    app.is_uncommitted_loaded = true;
    app.focus = Focus::Viewport;
    app.layout_config.is_search = false;
    app.search_path = Some("src/selected.rs".to_string());
    app.uncommitted.staged = FileChanges { modified: vec!["src/selected.rs".to_string()], ..Default::default() };

    let backend = TestBackend::new(48, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_status(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let col = path_col(buffer, 0, "s");
    assert!(row_contains(buffer, 0, "src/selected.rs"));
    assert_ne!(buffer[(col, 0)].bg, selected_bg);
}

#[test]
fn status_shows_loading_instead_of_stale_commit_diff() {
    let mut app = status_app();
    app.graph_selected = 1;
    app.current_diff = vec![FileChange { filename: "stale.txt".to_string(), status: FileStatus::Modified }];
    app.current_diff_identity = None;

    let backend = TestBackend::new(48, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_status(frame)).unwrap();

    let rendered = rendered(&terminal);
    assert!(rendered.contains("loading"), "{rendered}");
    assert!(!rendered.contains("stale.txt"), "{rendered}");
}

#[test]
fn status_preserves_known_empty_commit_diff_state() {
    let mut app = status_app();
    let identity = GraphIndexIdentity { index: 1, alias: 1, oid: Oid::zero() };
    app.graph_selected = 1;
    app.graph.index_rows.insert(1, graph_row(1, 1, identity.oid));
    app.current_diff_identity = Some(identity);

    let backend = TestBackend::new(48, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_status(frame)).unwrap();

    let rendered = rendered(&terminal);
    assert!(rendered.contains("⊘ no staged changes"), "{rendered}");
    assert!(!rendered.contains("loading"), "{rendered}");
}

#[test]
fn staged_status_short_page_stripes_blank_tail_rows() {
    let mut app = status_app();
    app.graph_selected = 0;
    app.is_uncommitted_loaded = true;
    app.focus = Focus::Viewport;
    app.uncommitted.staged = FileChanges { modified: vec!["staged.txt".to_string()], ..Default::default() };
    let zebra = app.theme.background_or_default(app.theme.COLOR_GREY_900);

    let backend = TestBackend::new(48, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_status(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 0)].bg, zebra);
    assert_ne!(buffer[(1, 1)].bg, zebra);
    assert_eq!(buffer[(1, 2)].bg, zebra);
}

#[test]
fn unstaged_status_short_page_stripes_blank_tail_rows() {
    let mut app = status_app();
    app.graph_selected = 0;
    app.is_uncommitted_loaded = true;
    app.focus = Focus::Viewport;
    app.uncommitted.unstaged = FileChanges { modified: vec!["unstaged.txt".to_string()], ..Default::default() };
    let zebra = app.theme.background_or_default(app.theme.COLOR_GREY_900);

    let backend = TestBackend::new(48, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_status(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 6)].bg, zebra);
    assert_ne!(buffer[(1, 7)].bg, zebra);
    assert_eq!(buffer[(1, 8)].bg, zebra);
}
