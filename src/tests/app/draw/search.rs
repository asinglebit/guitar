use crate::{
    app::{
        app::{App, Focus},
        state::layout::Layout,
    },
    helpers::layout::LayoutConfig,
};
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
