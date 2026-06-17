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

fn rendered_settings(app: &mut App, repo: &Repository, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_settings(frame, repo)).unwrap();
    terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>()
}

fn line_text(line: &ratatui::text::Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect::<String>()
}

fn remote_selection_lines(app: &App, remote_name: &str) -> Vec<usize> {
    app.settings_selections
        .iter()
        .filter_map(|selection| match &selection.kind {
            SettingsSelectionKind::Remote(name) if name == remote_name => Some(selection.line),
            _ => None,
        })
        .collect()
}

#[test]
fn settings_scroll_keeps_visible_selection_without_recentering() {
    let (_path, repo) = temp_repo("visible");
    let mut app = settings_app();
    draw_settings_once(&mut app, &repo);

    let visible_height = app.layout.graph.height as usize;
    let last_selectable = app.settings_selections.last().unwrap().line;
    let selected = app.settings_selections.iter().map(|selection| selection.line).find(|line| *line > visible_height * 2 && last_selectable.saturating_sub(*line) > visible_height).unwrap();
    let scroll = selected.saturating_sub(2);
    app.settings_scroll.set(scroll);
    app.settings_selected = selected;

    draw_settings_once(&mut app, &repo);

    assert_eq!(app.settings_scroll.get(), scroll);
}

#[test]
fn settings_scroll_moves_only_when_selection_leaves_view() {
    let (_path, repo) = temp_repo("bounded");
    let mut app = settings_app();
    draw_settings_once(&mut app, &repo);

    let visible_height = app.layout.graph.height as usize;
    let below = app.settings_selections.iter().map(|selection| selection.line).find(|line| *line >= visible_height * 2).unwrap();
    app.settings_scroll.set(0);
    app.settings_selected = below;

    draw_settings_once(&mut app, &repo);

    assert_eq!(app.settings_scroll.get(), below.saturating_sub(visible_height).saturating_add(1));

    app.settings_scroll.set(below.saturating_add(3));
    app.settings_selected = below;

    draw_settings_once(&mut app, &repo);

    assert_eq!(app.settings_scroll.get(), below);
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

#[test]
fn settings_renders_layout_visibility_rows_with_states() {
    let (_path, repo) = temp_repo("layout-section");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 120, 120);
    app.layout.app = Rect::new(0, 0, 120, 120);
    app.layout_config.is_branches = true;
    app.layout_config.is_shas = false;

    let rendered = rendered_settings(&mut app, &repo, 120, 120);

    assert!(rendered.contains("layout visibility:"));
    assert!(rendered.contains("1 branches:"));
    assert!(rendered.contains("8 SHAs:"));
    assert!(rendered.contains("0 reset layout:"));
    assert!(rendered.contains("[*]"));
    assert!(rendered.contains("[ ]"));
    assert!(rendered.contains("(enter)"));
}

#[test]
fn settings_section_names_use_highlight_color() {
    let (_path, repo) = temp_repo("section-highlight");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 120, 160);
    app.layout.app = Rect::new(0, 0, 120, 160);

    let backend = TestBackend::new(120, 160);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_settings(frame, &repo)).unwrap();
    let buffer = terminal.backend().buffer();

    for label in ["paths:", "recent repositories:", "remotes:", "credentials:", "themes:", "layout visibility:", "shortcuts / normal mode:", "shortcuts / action mode:"] {
        let row = (0..buffer.area.height)
            .find(|&y| {
                let line = (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect::<String>();
                line.contains(label)
            })
            .unwrap();
        let col = (0..buffer.area.width).find(|&x| buffer[(x, row)].symbol() == &label[0..1]).unwrap();

        assert_eq!(buffer[(col, row)].fg, app.theme.COLOR_HIGHLIGHTED);
    }
}

#[test]
fn settings_layout_rows_use_current_normal_keymap_binding() {
    let (_path, repo) = temp_repo("layout-key");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 120, 120);
    app.layout.app = Rect::new(0, 0, 120, 120);
    app.keymaps.get_mut(&InputMode::Normal).unwrap().insert(KeyBinding::new(KeyCode::Char('s'), KeyModifiers::NONE), Command::ToggleShas);

    let rendered = rendered_settings(&mut app, &repo, 120, 120);

    assert!(rendered.contains("s SHAs:"));
}

#[test]
fn settings_renders_recent_repositories_section_with_actions_and_selectable_rows() {
    let (_path, repo) = temp_repo("recent-section");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 140, 120);
    app.layout.app = Rect::new(0, 0, 140, 120);
    app.recent = vec!["/repo/a".into(), "/repo/b".into()];

    let rendered = rendered_settings(&mut app, &repo, 140, 120);

    assert!(rendered.contains("recent file:"));
    assert!(rendered.contains("recent repositories:"));
    assert!(rendered.contains("actions:"));
    assert!(rendered.contains("remove (d) | move up (Shift + K) | move down (Shift + J)"));
    assert!(!rendered.contains("actions: remove (d)"));
    assert!(rendered.contains("/repo/a"));
    assert!(rendered.contains("/repo/b"));
    assert!(app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::RecentRepository(0)));
    assert!(app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::RecentRepository(1)));
}

#[test]
fn settings_recent_repository_actions_use_split_row_and_current_keymap_bindings() {
    let (_path, repo) = temp_repo("recent-actions-keymap");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 140, 120);
    app.layout.app = Rect::new(0, 0, 140, 120);
    let normal = app.keymaps.get_mut(&InputMode::Normal).unwrap();
    normal.insert(KeyBinding::new(KeyCode::Char('x'), KeyModifiers::NONE), Command::RemoveRecentRepository);
    normal.insert(KeyBinding::new(KeyCode::Char('U'), KeyModifiers::SHIFT), Command::MoveRecentRepositoryUp);
    normal.insert(KeyBinding::new(KeyCode::F(2), KeyModifiers::NONE), Command::MoveRecentRepositoryDown);

    let lines = app.settings_lines(&repo);
    let actions = lines.iter().map(line_text).find(|line| line.contains("remove (x) | move up (Shift + U) | move down (F(2))")).unwrap();

    assert!(actions.starts_with(" actions:"));
    assert!(actions.contains("actions:  "));
    assert!(actions.ends_with("remove (x) | move up (Shift + U) | move down (F(2)) "));
}

#[test]
fn settings_empty_recent_repositories_row_is_not_selectable() {
    let (_path, repo) = temp_repo("empty-recent-section");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 140, 120);
    app.layout.app = Rect::new(0, 0, 140, 120);

    let rendered = rendered_settings(&mut app, &repo, 140, 120);

    assert!(rendered.contains("no recent repositories"));
    assert!(!app.settings_selections.iter().any(|selection| matches!(selection.kind, SettingsSelectionKind::RecentRepository(_))));
}

#[test]
fn settings_renders_remotes_section_with_add_and_empty_state() {
    let (_path, repo) = temp_repo("empty-remotes-section");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 140, 120);
    app.layout.app = Rect::new(0, 0, 140, 120);

    let rendered = rendered_settings(&mut app, &repo, 140, 120);

    assert!(rendered.contains("remotes:"));
    assert!(rendered.contains("+ add remote"));
    assert!(rendered.contains("no remotes"));
    assert!(app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::RemoteAdd));
    assert!(!app.settings_selections.iter().any(|selection| matches!(selection.kind, SettingsSelectionKind::Remote(_))));
}

#[test]
fn settings_renders_remote_rows_with_fetch_and_push_urls() {
    let (_path, repo) = temp_repo("remote-rows");
    repo.remote("origin", "https://example.com/repo.git").unwrap();
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 180, 140);
    app.layout.app = Rect::new(0, 0, 180, 140);

    let lines = app.settings_lines(&repo);
    let rendered = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
    let selection_lines = remote_selection_lines(&app, "origin");
    let selection_texts = selection_lines.iter().map(|line| line_text(&lines[*line])).collect::<Vec<_>>();

    assert!(rendered.contains("origin fetch:"));
    assert!(rendered.contains("origin push:"));
    assert_eq!(rendered.matches("https://example.com/repo.git").count(), 2);
    assert_eq!(selection_lines.len(), 2);
    assert!(selection_texts.iter().any(|text| text.contains("origin fetch:")));
    assert!(selection_texts.iter().any(|text| text.contains("origin push:")));
}

#[test]
fn settings_renders_remote_rows_with_explicit_push_url() {
    let (_path, repo) = temp_repo("remote-push-url");
    repo.remote("origin", "https://example.com/repo.git").unwrap();
    repo.remote_set_pushurl("origin", Some("ssh://example.com/repo.git")).unwrap();
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 180, 140);
    app.layout.app = Rect::new(0, 0, 180, 140);

    let lines = app.settings_lines(&repo);
    let selection_lines = remote_selection_lines(&app, "origin");
    let selection_texts = selection_lines.iter().map(|line| line_text(&lines[*line])).collect::<Vec<_>>();
    let fetch_line = selection_texts.iter().find(|text| text.contains("origin fetch:")).unwrap();
    let push_line = selection_texts.iter().find(|text| text.contains("origin push:")).unwrap();

    assert_eq!(selection_lines.len(), 2);
    assert!(fetch_line.contains("https://example.com/repo.git"));
    assert!(push_line.contains("ssh://example.com/repo.git"));
}

#[test]
fn settings_truncates_long_remote_urls() {
    let (_path, repo) = temp_repo("remote-truncate");
    let long_url = "https://example.com/this/is/a/very/long/path/that/should/not/overflow/the/settings/row/repository.git";
    repo.remote("origin", long_url).unwrap();
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 80, 120);
    app.layout.app = Rect::new(0, 0, 80, 120);

    let rendered = rendered_settings(&mut app, &repo, 80, 120);

    assert!(rendered.contains("origin fetch:"));
    assert!(rendered.contains("..."));
    assert!(!rendered.contains(long_url));
}
