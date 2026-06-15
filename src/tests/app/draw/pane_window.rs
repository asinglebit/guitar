use super::*;
use crate::core::graph_service::GraphPaneRow;
use crate::helpers::palette::Theme;
use ratatui::{Terminal, backend::TestBackend, layout::Rect, widgets::List};

fn branch_row(name: &str) -> GraphPaneRow {
    GraphPaneRow::Branch { alias: 1, name: name.to_string(), is_local: true, lane: None, graph_index: None }
}

#[test]
fn cached_rows_shift_up_when_requested_window_moves_down() {
    let window = PaneWindowCache { start: 0, end: 3, total: 4, rows: vec![branch_row("row0"), branch_row("row1"), branch_row("row2")], ..Default::default() };

    let rows = aligned_pane_rows(&window, 1, 4).expect("overlap");

    assert!(matches!(rows[0], Some(GraphPaneRow::Branch { name, .. }) if name == "row1"));
    assert!(matches!(rows[1], Some(GraphPaneRow::Branch { name, .. }) if name == "row2"));
    assert!(rows[2].is_none());
}

#[test]
fn cached_rows_shift_down_when_requested_window_moves_up() {
    let window = PaneWindowCache { start: 1, end: 4, total: 4, rows: vec![branch_row("row1"), branch_row("row2"), branch_row("row3")], ..Default::default() };

    let rows = aligned_pane_rows(&window, 0, 3).expect("overlap");

    assert!(rows[0].is_none());
    assert!(matches!(rows[1], Some(GraphPaneRow::Branch { name, .. }) if name == "row1"));
    assert!(matches!(rows[2], Some(GraphPaneRow::Branch { name, .. }) if name == "row2"));
}

#[test]
fn preloaded_pane_window_adds_one_visible_page_on_each_side() {
    assert_eq!(preloaded_pane_window(5, 8, 20, 3), (2, 11));
    assert_eq!(preloaded_pane_window(0, 3, 20, 3), (0, 6));
    assert_eq!(preloaded_pane_window(17, 20, 20, 3), (14, 20));
    assert_eq!(preloaded_pane_window(0, 0, 0, 3), (0, 0));
}

#[test]
fn zebra_list_items_fill_blank_tail_rows() {
    let theme = Theme::default();
    let items = zebra_list_items(&[Line::from("row")], 3, 0, usize::MAX, false, false, &theme);
    let zebra = theme.background_or_default(theme.COLOR_GREY_900);

    let backend = TestBackend::new(10, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| frame.render_widget(List::new(items), Rect::new(0, 0, 10, 3))).unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].bg, zebra);
    assert_ne!(buffer[(0, 1)].bg, zebra);
    assert_eq!(buffer[(0, 2)].bg, zebra);
}
