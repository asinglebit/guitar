use super::*;
use ratatui::{Terminal, backend::TestBackend};

fn rendered_symbols(terminal: &Terminal<TestBackend>) -> String {
    terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>()
}

#[test]
fn input_modal_renders_esc_title_action_row_and_bordered_input() {
    let mut app = App::default();
    app.modal_input.set_value("commit message");

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_modal_input(frame, "commit")).unwrap();

    let rendered = rendered_symbols(&terminal);
    assert!(rendered.contains("(esc)"));
    assert!(rendered.contains("confirm (enter)"));
    assert!(!rendered.contains("cancel (esc)"));
    assert!(rendered.contains("commit message"));
    assert!(rendered.contains("─"));
}
