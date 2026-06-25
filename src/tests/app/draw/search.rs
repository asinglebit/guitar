use crate::{
    app::{
        app::{App, Focus},
        state::layout::Layout,
    },
    core::graph_service::GraphFileHistoryRow,
    git::queries::helpers::FileStatus,
    helpers::layout::LayoutConfig,
};
use git2::Oid;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

fn rendered(terminal: &Terminal<TestBackend>) -> String {
    terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>()
}

fn search_app() -> App {
    let mut app = App {
        focus: Focus::Search,
        layout: Layout { search: Rect::new(0, 0, 40, 5), search_scrollbar: Rect::new(39, 0, 1, 5), ..Default::default() },
        layout_config: LayoutConfig { is_search: true, ..Default::default() },
        ..Default::default()
    };
    app.layout_config.is_zen = false;
    app
}

fn history_row(idx: usize, summary: &str, status: FileStatus) -> GraphFileHistoryRow {
    GraphFileHistoryRow { graph_index: idx, oid: Oid::from_str(&format!("11111111{idx:032x}")).unwrap(), summary: summary.to_string(), status }
}

#[test]
fn search_empty_state_renders_list_shell() {
    let mut app = search_app();

    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_search(frame)).unwrap();

    let rendered = rendered(&terminal);
    assert!(rendered.contains("search"), "{rendered}");
}

#[test]
fn search_empty_state_stripes_backdrop() {
    let mut app = search_app();
    let zebra = app.theme.background_or_default(app.theme.COLOR_GREY_900);

    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_search(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(2, 0)].bg, zebra);
    assert_ne!(buffer[(2, 1)].bg, zebra);
    assert_eq!(buffer[(2, 2)].bg, zebra);
}

#[test]
fn search_loading_state_renders_selected_path() {
    let mut app = search_app();
    app.search_path = Some("src/app/draw/search.rs".to_string());
    app.search_is_loading = true;

    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_search(frame)).unwrap();

    let rendered = rendered(&terminal);
    assert!(rendered.contains("loading"));
    assert!(rendered.contains("src/app"));
}

#[test]
fn search_results_render_rows_and_selection() {
    let mut app = search_app();
    app.search_rows = vec![history_row(1, "touch search pane", FileStatus::Modified)];

    let backend = TestBackend::new(50, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_search(frame)).unwrap();

    let rendered = rendered(&terminal);
    assert!(rendered.contains("~"));
    assert!(rendered.contains("11111111"));
    assert!(rendered.contains("touch search pane"));

    let buffer = terminal.backend().buffer();
    let selected_bg = app.theme.background_or_default(app.theme.COLOR_GREY_800);
    assert!(buffer.content().iter().any(|cell| cell.bg == selected_bg));
}

#[test]
fn search_results_scroll_long_lists() {
    let mut app = search_app();
    app.search_selected = 8;
    app.search_rows = (0..12).map(|idx| history_row(idx, &format!("commit {idx}"), FileStatus::Modified)).collect();

    let backend = TestBackend::new(50, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_search(frame)).unwrap();

    assert!(app.search_scroll.get() > 0);
    assert!(rendered(&terminal).contains("commit 8"));
}
