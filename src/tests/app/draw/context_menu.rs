use super::*;
use crate::app::app::{CONTEXT_MENU_HEIGHT, CONTEXT_MENU_WIDTH, ContextMenuState};
use ratatui::{Terminal, backend::TestBackend};

fn rendered_symbols(terminal: &Terminal<TestBackend>) -> String {
    terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>()
}

#[test]
fn context_menu_renders_labels_rounded_corners_and_selected_row() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App { context_menu: Some(ContextMenuState { column: 4, row: 3, selected: 1 }), ..Default::default() };

    terminal.draw(|frame| app.draw_context_menu(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let rendered = rendered_symbols(&terminal);
    assert!(rendered.contains("Settings"), "{rendered}");
    assert!(rendered.contains("Splash screen"), "{rendered}");
    assert!(rendered.contains("Exit"), "{rendered}");
    assert_eq!(buffer[(4, 3)].symbol(), "╭");
    assert_eq!(buffer[(4 + CONTEXT_MENU_WIDTH - 1, 3)].symbol(), "╮");

    let selected_row = 3 + 1 + 1;
    assert_eq!(buffer[(5, selected_row)].bg, app.theme.background_or_default(app.theme.COLOR_GREY_800));
    assert_eq!(buffer[(5, 4)].fg, app.theme.COLOR_GREY_600);
}

#[test]
fn context_menu_clamps_to_lower_right_terminal_edge() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App { context_menu: Some(ContextMenuState { column: 79, row: 23, selected: 2 }), ..Default::default() };

    terminal.draw(|frame| app.draw_context_menu(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let x = 80 - CONTEXT_MENU_WIDTH;
    let y = 24 - CONTEXT_MENU_HEIGHT;
    assert_eq!(buffer[(x, y)].symbol(), "╭");
    assert_eq!(buffer[(x + CONTEXT_MENU_WIDTH - 1, y)].symbol(), "╮");
    assert_eq!(buffer[(x, y + CONTEXT_MENU_HEIGHT - 1)].symbol(), "╰");
    assert_eq!(buffer[(x + CONTEXT_MENU_WIDTH - 1, y + CONTEXT_MENU_HEIGHT - 1)].symbol(), "╯");
}
