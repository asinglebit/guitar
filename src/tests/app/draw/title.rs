use super::*;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

fn rendered_symbols(terminal: &Terminal<TestBackend>) -> String {
    terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>()
}

fn title_app(viewport: Viewport) -> App {
    let mut app = App { path: Some("/tmp/guitar-title-repo".to_string()), file_name: Some("src/lib.rs".to_string()), viewport, focus: Focus::StatusTop, ..Default::default() };
    app.layout.title_left = Rect::new(0, 0, 100, 1);
    app.layout.title_right = Rect::new(100, 0, 20, 1);
    app
}

#[test]
fn title_shows_repo_path_outside_viewer_even_with_cached_file_name() {
    let mut app = title_app(Viewport::Graph);
    let backend = TestBackend::new(120, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| app.draw_title(frame)).unwrap();

    let rendered = rendered_symbols(&terminal);
    assert!(rendered.contains("/tmp/guitar-title-repo"));
    assert!(!rendered.contains("src/lib.rs"));
}

#[test]
fn title_shows_repo_file_path_inside_viewer() {
    let mut app = title_app(Viewport::Viewer);
    let backend = TestBackend::new(120, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| app.draw_title(frame)).unwrap();

    let rendered = rendered_symbols(&terminal);
    assert!(rendered.contains("/tmp/guitar-title-repo/src/lib.rs"));
}
