use super::*;
use crate::helpers::keymap::{Command, InputMode, KeyBinding, KeymapSelection};
use ratatui::{
    Terminal,
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyModifiers},
};

fn rendered_symbols(terminal: &Terminal<TestBackend>) -> String {
    terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>()
}

fn key_capture_app() -> App {
    App { modal_key_capture_selection: Some(KeymapSelection::new(InputMode::Normal, KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown)), ..Default::default() }
}

#[test]
fn key_capture_modal_renders_esc_title_and_press_key_action_row() {
    let mut app = key_capture_app();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| app.draw_modal_key_capture(frame)).unwrap();

    let rendered = rendered_symbols(&terminal);
    assert!(rendered.contains("(esc)"));
    assert!(rendered.contains("press key"));
    assert!(!rendered.contains("cancel (esc)"));
    assert!(!rendered.contains("Ctrl+C"));
}

#[test]
fn key_capture_modal_renders_save_action_row_for_valid_candidate() {
    let mut app = key_capture_app();
    app.modal_key_capture_candidate = Some(KeyBinding::new(KeyCode::Char('n'), KeyModifiers::ALT));
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| app.draw_modal_key_capture(frame)).unwrap();

    let rendered = rendered_symbols(&terminal);
    assert!(rendered.contains("save (enter)"));
    assert!(!rendered.contains("cancel (esc)"));
}
