use super::*;
use crate::{
    app::app::Viewport,
    helpers::keymap::{Command, InputMode, KeyBinding},
};
use git2::Repository;
use indexmap::IndexMap;
use ratatui::{
    Terminal,
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::Rect,
};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (std::path::PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-settings-draw-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn settings_app() -> App {
    let mut app = App { viewport: Viewport::Settings, focus: Focus::Viewport, ..Default::default() };
    let mut keymaps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(KeyCode::Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let mut action = IndexMap::new();
    action.insert(KeyBinding::new(KeyCode::Char('k'), KeyModifiers::NONE), Command::ScrollUp);
    keymaps.insert(InputMode::Normal, normal);
    keymaps.insert(InputMode::Action, action);
    app.keymaps = keymaps;
    app.layout_config.is_zen = false;
    app.layout.graph = Rect::new(0, 0, 90, 10);
    app.layout.app = Rect::new(0, 0, 90, 10);
    app
}

fn draw_settings_once(app: &mut App, repo: &Repository) {
    let backend = TestBackend::new(90, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_settings(frame, repo)).unwrap();
}

#[test]
fn settings_scroll_centers_middle_selection() {
    let (_path, repo) = temp_repo("center");
    let mut app = settings_app();
    draw_settings_once(&mut app, &repo);

    let visible_height = app.layout.graph.height as usize;
    let last_selectable = app.settings_selections.last().unwrap().line;
    let selected = app.settings_selections.iter().map(|selection| selection.line).find(|line| *line >= visible_height / 2 && last_selectable.saturating_sub(*line) > visible_height).unwrap();
    app.settings_selected = selected;

    draw_settings_once(&mut app, &repo);

    assert_eq!(app.settings_scroll.get(), selected - visible_height / 2);
}

#[test]
fn settings_scroll_clamps_at_top_and_bottom() {
    let (_path, repo) = temp_repo("clamp");
    let mut app = settings_app();
    draw_settings_once(&mut app, &repo);

    let visible_height = app.layout.graph.height as usize;
    let first = app.settings_selections.first().unwrap().line;
    let last = app.settings_selections.last().unwrap().line;

    app.settings_selected = first;
    draw_settings_once(&mut app, &repo);
    assert_eq!(app.settings_scroll.get(), 0);

    app.settings_selected = usize::MAX;
    draw_settings_once(&mut app, &repo);
    assert_eq!(app.settings_selected, last);
    assert_eq!(app.settings_scroll.get(), last.saturating_add(1).saturating_sub(visible_height));
}

#[test]
fn settings_selection_snaps_to_selectable_line() {
    let (_path, repo) = temp_repo("snap");
    let mut app = settings_app();
    app.settings_selected = 0;

    draw_settings_once(&mut app, &repo);

    assert!(app.settings_selections.iter().any(|selection| selection.line == app.settings_selected));
}
