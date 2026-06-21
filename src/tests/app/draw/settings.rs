use super::*;
use crate::{
    app::app::{SettingsTab, Viewport},
    git::queries::remotes::GUITAR_DEFAULT_REMOTE_CONFIG,
    helpers::{
        keymap::{Command, InputMode, KeyBinding},
        layout::GRAPH_LANE_LIMIT_DEFAULT,
        localisation::Language,
        symbols::SymbolTheme,
    },
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
    app.layout_config.graph_lane_limit = GRAPH_LANE_LIMIT_DEFAULT;
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
fn settings_default_tab_is_general_and_renders_general_sections() {
    let (_path, repo) = temp_repo("default-tab");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 140, 120);
    app.layout.app = Rect::new(0, 0, 140, 120);

    let rendered = rendered_settings(&mut app, &repo, 140, 120);

    assert_eq!(app.settings_tab, SettingsTab::General);
    assert!(rendered.contains("version:"));
    assert!(rendered.contains("general"));
    assert!(rendered.contains("display"));
    assert!(rendered.contains("auth"));
    assert!(rendered.contains("repo"));
    assert!(rendered.contains("shortcuts"));
    assert!(rendered.contains("paths:"));
    assert!(rendered.contains("performance:"));
    assert!(rendered.contains("graph lane limit:"));
    let lane_limit_line = app.settings_lines(&repo).iter().map(line_text).find(|line| line.contains("graph lane limit:")).unwrap();
    assert!(lane_limit_line.contains(&GRAPH_LANE_LIMIT_DEFAULT.to_string()));
    assert!(lane_limit_line.contains("(enter)"));
    assert!(rendered.contains("recent repositories:"));
    assert!(app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::GraphLaneLimit));
    assert!(!rendered.contains("pane visibility:"));
    assert!(!rendered.contains("credentials:"));
    assert!(!rendered.contains("remotes:"));
    assert!(!rendered.contains("shortcuts / normal mode:"));
}

#[test]
fn settings_active_tabs_render_their_grouped_sections_only() {
    let (_path, repo) = temp_repo("tab-groups");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 160, 160);
    app.layout.app = Rect::new(0, 0, 160, 160);

    app.settings_tab = SettingsTab::Display;
    let display = rendered_settings(&mut app, &repo, 160, 160);
    assert!(display.contains("pane visibility:"));
    assert!(display.contains("graph metadata:"));
    assert!(display.contains("symbol themes:"));
    assert!(display.contains("themes:"));
    assert!(!display.contains("graph lane limit:"));
    assert!(!display.contains("recent repositories:"));
    assert!(!display.contains("remotes:"));

    app.settings_tab = SettingsTab::Auth;
    let auth = rendered_settings(&mut app, &repo, 160, 160);
    assert!(auth.contains("credentials:"));
    assert!(!auth.contains("themes:"));
    assert!(!auth.contains("remotes:"));

    app.settings_tab = SettingsTab::Shortcuts;
    let shortcuts = rendered_settings(&mut app, &repo, 160, 160);
    assert!(shortcuts.contains("shortcuts / normal mode:"));
    assert!(shortcuts.contains("shortcuts / action mode:"));
    assert!(!shortcuts.contains("paths:"));
    assert!(!shortcuts.contains("credentials:"));
}

#[test]
fn settings_shortcuts_render_graph_lane_limit_shortcuts() {
    crate::helpers::localisation::set_active_language(Language::English);
    let (_path, repo) = temp_repo("lane-limit-shortcuts");
    let mut app = settings_app();
    app.settings_tab = SettingsTab::Shortcuts;
    app.layout.graph = Rect::new(0, 0, 160, 80);
    app.layout.app = Rect::new(0, 0, 160, 80);
    let normal = app.keymaps.get_mut(&InputMode::Normal).unwrap();
    normal.insert(KeyBinding::new(KeyCode::Char('-'), KeyModifiers::NONE), Command::ShrinkGraphLaneLimit);
    normal.insert(KeyBinding::new(KeyCode::Char('+'), KeyModifiers::NONE), Command::GrowGraphLaneLimit);

    let rendered = app.settings_lines(&repo).iter().map(line_text).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains("Shrink Graph Lane Limit"));
    assert!(rendered.contains("Grow Graph Lane Limit"));
    assert!(rendered.contains("-"));
    assert!(rendered.contains("+"));
}

#[test]
fn settings_scroll_keeps_visible_selection_without_recentering() {
    let (_path, repo) = temp_repo("visible");
    let mut app = settings_app();
    app.settings_tab = SettingsTab::Display;
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
    app.settings_tab = SettingsTab::Display;
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
    app.settings_tab = SettingsTab::Display;
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
    app.settings_tab = SettingsTab::Display;
    app.layout.graph = Rect::new(0, 0, 120, 120);
    app.layout.app = Rect::new(0, 0, 120, 120);
    app.layout_config.is_branches = true;
    app.layout_config.is_shas = false;

    let rendered = rendered_settings(&mut app, &repo, 120, 120);

    assert!(rendered.contains("pane visibility:"));
    assert!(rendered.contains("graph metadata:"));
    assert!(rendered.contains("1 branches:"));
    assert!(rendered.contains("! SHAs:"));
    assert!(rendered.contains("6 submodules:"));
    assert!(rendered.contains("@ committer date/time:"));
    assert!(rendered.contains("$ refs:"));
    assert!(rendered.contains("0 reset layout:"));
    assert!(rendered.contains("🞕"));
    assert!(rendered.contains("🞎"));
    assert!(!rendered.contains("[*]"));
    assert!(!rendered.contains("[ ]"));
    assert!(rendered.contains("(enter)"));
    assert!(!app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::GraphLaneLimit));
}

#[test]
fn settings_renders_theme_rows_with_unicode_markers() {
    let (_path, repo) = temp_repo("theme-markers");
    let mut app = settings_app();
    app.settings_tab = SettingsTab::Display;
    app.layout.graph = Rect::new(0, 0, 120, 120);
    app.layout.app = Rect::new(0, 0, 120, 120);

    let rendered = rendered_settings(&mut app, &repo, 120, 120);

    assert!(rendered.contains("themes:"));
    assert!(rendered.contains("🞊"));
    assert!(rendered.contains("🞅"));
    assert!(!rendered.contains("(*)"));
    assert!(!rendered.contains("( )"));
}

#[test]
fn settings_general_tab_includes_symbols_json() {
    let (_path, repo) = temp_repo("symbols-path");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 140, 120);
    app.layout.app = Rect::new(0, 0, 140, 120);

    let rendered = rendered_settings(&mut app, &repo, 140, 120);

    assert!(rendered.contains("symbols:"));
    assert!(rendered.contains("/symbols.json"));
    assert!(rendered.contains("language:"));
    assert!(rendered.contains("/language.json"));
}

#[test]
fn settings_display_tab_renders_language_rows() {
    let (_path, repo) = temp_repo("language-rows");
    let mut app = settings_app();
    app.settings_tab = SettingsTab::Display;
    app.language = Language::French;
    app.layout.graph = Rect::new(0, 0, 120, 120);
    app.layout.app = Rect::new(0, 0, 120, 120);

    let lines = app.settings_lines(&repo);
    let rendered = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains("language:"));
    assert!(rendered.contains("English"));
    assert!(rendered.contains("Español"));
    assert!(rendered.contains("Français"));
    assert!(rendered.contains("Русский"));
    assert!(rendered.contains("Türkçe"));
    assert!(app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::Language(0)));
    assert!(app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::Language(4)));
    assert!(!app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::Language(5)));
}

#[test]
fn settings_display_tab_renders_symbol_theme_rows() {
    let (_path, repo) = temp_repo("symbol-theme-rows");
    let mut app = settings_app();
    app.settings_tab = SettingsTab::Display;
    app.layout.graph = Rect::new(0, 0, 120, 120);
    app.layout.app = Rect::new(0, 0, 120, 120);

    let rendered = rendered_settings(&mut app, &repo, 120, 120);

    assert!(rendered.contains("symbol themes:"));
    assert!(rendered.contains("main"));
    assert!(rendered.contains("ascii"));
    assert!(app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::SymbolTheme(0)));
    assert!(app.settings_selections.iter().any(|selection| selection.kind == SettingsSelectionKind::SymbolTheme(1)));
}

#[test]
fn settings_display_tab_uses_active_symbol_theme_form_markers() {
    let (_path, repo) = temp_repo("symbol-theme-markers");
    let mut app = settings_app();
    app.symbols = SymbolTheme::ascii();
    app.settings_tab = SettingsTab::Display;
    app.layout.graph = Rect::new(0, 0, 120, 120);
    app.layout.app = Rect::new(0, 0, 120, 120);

    let rendered = rendered_settings(&mut app, &repo, 120, 120);

    assert!(rendered.contains("(*)"));
    assert!(rendered.contains("( )"));
    assert!(rendered.contains("[x]"));
    assert!(rendered.contains("[ ]"));
    assert!(!rendered.contains("🞊"));
    assert!(!rendered.contains("🞅"));
    assert!(rendered.contains("Español"));
    assert!(rendered.contains("Türkçe"));
}

#[test]
fn settings_narrow_tab_bar_uses_compact_bullets() {
    let (_path, repo) = temp_repo("compact-tabs");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 30, 80);
    app.layout.app = Rect::new(0, 0, 30, 80);

    let lines = app.settings_lines(&repo);
    let tab_line = app.settings_tab_hitboxes.first().unwrap().line;
    let tab_text = line_text(&lines[tab_line]);

    assert!(tab_text.contains("• • • • •"));
    assert!(!tab_text.contains("general"));
    assert!(!tab_text.contains("display"));
    assert!(!tab_text.contains("shortcuts"));
    assert_eq!(app.settings_tab_hitboxes.len(), 5);
    assert!(app.settings_tab_hitboxes.iter().all(|hitbox| hitbox.end.saturating_sub(hitbox.start) == 1));

    let mut tiny_app = settings_app();
    tiny_app.layout.graph = Rect::new(0, 0, 12, 80);
    tiny_app.layout.app = Rect::new(0, 0, 12, 80);

    let tiny_lines = tiny_app.settings_lines(&repo);
    let tiny_tab_line = tiny_app.settings_tab_hitboxes.first().unwrap().line;
    let tiny_tab_text = line_text(&tiny_lines[tiny_tab_line]);
    let rendered_bullets = tiny_tab_text.chars().filter(|&character| character == '•').count();

    assert_eq!(rendered_bullets, 2);
    assert_eq!(tiny_app.settings_tab_hitboxes.len(), rendered_bullets);
    assert!(!tiny_tab_text.contains(' '));
    assert!(!tiny_tab_text.contains("general"));
}

#[test]
fn settings_section_names_use_highlight_color() {
    let (_path, repo) = temp_repo("section-highlight");
    let mut app = settings_app();
    app.layout.graph = Rect::new(0, 0, 120, 160);
    app.layout.app = Rect::new(0, 0, 120, 160);

    let backend = TestBackend::new(120, 160);
    let mut terminal = Terminal::new(backend).unwrap();

    for (tab, labels) in [
        (SettingsTab::General, vec!["paths:", "performance:", "recent repositories:"]),
        (SettingsTab::Display, vec!["pane visibility:", "graph metadata:", "symbol themes:", "themes:"]),
        (SettingsTab::Auth, vec!["credentials:"]),
        (SettingsTab::Repo, vec!["remotes:"]),
        (SettingsTab::Shortcuts, vec!["shortcuts / normal mode:", "shortcuts / action mode:"]),
    ] {
        app.settings_tab = tab;
        terminal.draw(|frame| app.draw_settings(frame, &repo)).unwrap();
        let buffer = terminal.backend().buffer();

        for label in labels {
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
}

#[test]
fn settings_layout_rows_use_current_normal_keymap_binding() {
    let (_path, repo) = temp_repo("layout-key");
    let mut app = settings_app();
    app.settings_tab = SettingsTab::Display;
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
    app.settings_tab = SettingsTab::Repo;
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
    app.settings_tab = SettingsTab::Repo;
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
    app.settings_tab = SettingsTab::Repo;
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
fn settings_marks_effective_default_remote() {
    let (_path, repo) = temp_repo("remote-default-marker");
    repo.remote("origin", "https://example.com/origin.git").unwrap();
    repo.remote("upstream", "https://example.com/upstream.git").unwrap();
    repo.config().unwrap().set_str(GUITAR_DEFAULT_REMOTE_CONFIG, "upstream").unwrap();
    let mut app = settings_app();
    app.settings_tab = SettingsTab::Repo;
    app.layout.graph = Rect::new(0, 0, 180, 140);
    app.layout.app = Rect::new(0, 0, 180, 140);

    let lines = app.settings_lines(&repo);
    let rendered = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
    let upstream_selection_texts = remote_selection_lines(&app, "upstream").iter().map(|line| line_text(&lines[*line])).collect::<Vec<_>>();
    let origin_selection_texts = remote_selection_lines(&app, "origin").iter().map(|line| line_text(&lines[*line])).collect::<Vec<_>>();

    assert!(rendered.contains("default remote:"));
    assert!(rendered.contains("upstream"));
    assert!(upstream_selection_texts.iter().any(|text| text.contains("default")));
    assert!(!origin_selection_texts.iter().any(|text| text.contains("default")));
}

#[test]
fn settings_truncates_long_remote_urls() {
    let (_path, repo) = temp_repo("remote-truncate");
    let long_url = "https://example.com/this/is/a/very/long/path/that/should/not/overflow/the/settings/row/repository.git";
    repo.remote("origin", long_url).unwrap();
    let mut app = settings_app();
    app.settings_tab = SettingsTab::Repo;
    app.layout.graph = Rect::new(0, 0, 80, 120);
    app.layout.app = Rect::new(0, 0, 80, 120);

    let rendered = rendered_settings(&mut app, &repo, 80, 120);

    assert!(rendered.contains("origin fetch:"));
    assert!(rendered.contains("..."));
    assert!(!rendered.contains(long_url));
}
