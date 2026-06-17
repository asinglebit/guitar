use super::*;
use crate::app::app::{ContextMenuAction, ContextMenuItem, ContextMenuState};
use ratatui::{Terminal, backend::TestBackend};

fn rendered_symbols(terminal: &Terminal<TestBackend>) -> String {
    terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>()
}

fn item(label: &str, action: ContextMenuAction, enabled: bool) -> ContextMenuItem {
    ContextMenuItem { label: label.to_string(), action, enabled }
}

fn divider() -> ContextMenuItem {
    item("", ContextMenuAction::Divider, false)
}

fn spacer() -> ContextMenuItem {
    item("", ContextMenuAction::Spacer, false)
}

fn menu_state(column: u16, row: u16, selected: usize) -> ContextMenuState {
    ContextMenuState {
        column,
        row,
        selected,
        items: vec![
            item("Apply theme", ContextMenuAction::Settings, true),
            spacer(),
            divider(),
            spacer(),
            item("Settings", ContextMenuAction::Settings, false),
            item("Splash screen", ContextMenuAction::Splash, true),
            item("Exit", ContextMenuAction::Exit, true),
        ],
    }
}

#[test]
fn context_menu_renders_labels_rounded_corners_and_selected_row() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = menu_state(4, 3, 5);
    let width = state.width();
    let mut app = App { context_menu: Some(state), ..Default::default() };

    terminal.draw(|frame| app.draw_context_menu(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let rendered = rendered_symbols(&terminal);
    assert!(rendered.contains("Apply theme"), "{rendered}");
    assert!(rendered.contains("────"), "{rendered}");
    assert!(rendered.contains("Settings"), "{rendered}");
    assert!(rendered.contains("Splash screen"), "{rendered}");
    assert!(rendered.contains("Exit"), "{rendered}");
    assert_eq!(width, "Splash screen".chars().count() as u16 + 7);
    assert_eq!(buffer[(4, 3)].symbol(), "╭");
    assert_eq!(buffer[(4 + width - 1, 3)].symbol(), "╮");

    let selected_row = 3 + 2 + 5;
    assert_eq!(buffer[(5, selected_row)].bg, app.theme.background_or_default(app.theme.COLOR_GREY_800));
    assert_eq!(buffer[(5, 6)].bg, app.theme.background_or_default(app.theme.COLOR_GREY_900));
    assert_eq!(buffer[(6, 7)].symbol(), "─");
    assert_eq!(buffer[(5, 8)].bg, app.theme.background_or_default(app.theme.COLOR_GREY_900));
    assert_eq!(buffer[(5, 9)].fg, app.theme.COLOR_GREY_600);
    assert_eq!(buffer[(5, 4)].bg, app.theme.background_or_default(app.theme.COLOR_GREY_900));
    assert_eq!(buffer[(5, 12)].bg, app.theme.background_or_default(app.theme.COLOR_GREY_900));
}

#[test]
fn context_menu_clamps_to_lower_right_terminal_edge() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = menu_state(79, 23, 2);
    let width = state.width();
    let height = state.height();
    let mut app = App { context_menu: Some(state), ..Default::default() };

    terminal.draw(|frame| app.draw_context_menu(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let x = 80 - width;
    let y = 24 - height;
    assert_eq!(buffer[(x, y)].symbol(), "╭");
    assert_eq!(buffer[(x + width - 1, y)].symbol(), "╮");
    assert_eq!(buffer[(x, y + height - 1)].symbol(), "╰");
    assert_eq!(buffer[(x + width - 1, y + height - 1)].symbol(), "╯");
}
