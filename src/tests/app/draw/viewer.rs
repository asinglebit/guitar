use super::*;
use crate::{
    app::state::{defaults::ViewerMode, layout::Layout},
    git::queries::helpers::FileChanges,
    git::test_support::{TestDir, commit_named_file as commit},
    helpers::symbols::status as status_symbol,
};
use git2::Repository;
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};
use std::{fs, path::Path};

fn temp_repo(name: &str) -> (TestDir, Repository) {
    let dir = TestDir::new(name);
    let repo = Repository::init(dir.path()).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (dir, repo)
}

fn write(path: &Path, file: &str, content: &str) {
    fs::write(path.join(file), content).unwrap();
}

fn viewer_app() -> App {
    let mut app = App {
        layout: Layout {
            graph: Rect::new(0, 0, 80, 6),
            graph_scrollbar: Rect::new(79, 0, 1, 6),
            viewer_split_left: Rect::new(0, 0, 39, 6),
            divider_viewer_split: Rect::new(39, 0, 1, 6),
            viewer_split_right: Rect::new(40, 0, 40, 6),
            ..Default::default()
        },
        ..Default::default()
    };
    app.layout_config.is_zen = false;
    app.focus = Focus::StatusTop;
    app.viewport = Viewport::Viewer;
    app.file_name = Some("new.txt".to_string());
    app
}

fn rendered_row(buffer: &Buffer, row: u16) -> String {
    (0..buffer.area.width).map(|x| buffer[(x, row)].symbol()).collect::<String>()
}

fn find_text(buffer: &Buffer, needle: &str) -> Option<(u16, u16)> {
    (0..buffer.area.height).find_map(|y| rendered_row(buffer, y).find(needle).map(|x| (x as u16, y)))
}

#[test]
fn selected_status_file_helpers_match_rendered_group_order() {
    let mut app = App::default();
    app.uncommitted.conflicts = vec!["conflict.txt".to_string()];
    app.uncommitted.staged = FileChanges { modified: vec!["staged-modified.txt".to_string()], added: vec!["staged-added.txt".to_string()], deleted: vec!["staged-deleted.txt".to_string()] };
    app.uncommitted.unstaged = FileChanges { modified: vec!["unstaged-modified.txt".to_string()], added: vec!["unstaged-added.txt".to_string()], deleted: vec!["unstaged-deleted.txt".to_string()] };

    app.status_top_selected = 0;
    assert!(app.selected_staged_status_file_is_conflict());
    assert_eq!(app.selected_staged_status_file_name().as_deref(), Some("conflict.txt"));

    app.status_top_selected = 2;
    assert!(!app.selected_staged_status_file_is_conflict());
    assert_eq!(app.selected_staged_status_file_name().as_deref(), Some("staged-added.txt"));

    app.status_bottom_selected = 0;
    assert!(app.selected_unstaged_status_file_is_conflict());
    assert_eq!(app.selected_unstaged_status_file_name().as_deref(), Some("conflict.txt"));

    app.status_bottom_selected = 3;
    assert!(!app.selected_unstaged_status_file_is_conflict());
    assert_eq!(app.selected_unstaged_status_file_name().as_deref(), Some("unstaged-deleted.txt"));
}

#[test]
fn unstaged_added_file_viewer_renders_added_lines() {
    let (dir, repo) = temp_repo("unstaged-added");
    write(dir.path(), "tracked.txt", "base\n");
    commit(&repo, "tracked.txt", "initial");
    write(dir.path(), "new.txt", "alpha\nbeta\n");

    let mut app = viewer_app();
    app.update_viewer(Oid::zero(), &repo);

    let mut terminal = Terminal::new(TestBackend::new(80, 6)).unwrap();
    terminal.draw(|frame| app.draw_viewer(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let added_bg = app.theme.background_or_default(app.theme.COLOR_LIGHT_GREEN_900);
    let (plus_x, plus_y) = find_text(buffer, &format!("{}alpha", status_symbol::ADDED_SPACED)).unwrap();
    let alpha_x = plus_x + status_symbol::ADDED_SPACED.len() as u16;

    assert_eq!(buffer[(plus_x, plus_y)].fg, app.theme.COLOR_GREEN);
    assert_eq!(buffer[(plus_x, plus_y)].bg, added_bg);
    assert_eq!(buffer[(alpha_x, plus_y)].fg, app.theme.COLOR_GREEN);
    assert_eq!(buffer[(alpha_x, plus_y)].bg, added_bg);

    app.viewer_mode = ViewerMode::Split;
    app.viewer_selected = 0;
    app.viewer_scroll.set(0);

    let mut terminal = Terminal::new(TestBackend::new(80, 6)).unwrap();
    terminal.draw(|frame| app.draw_viewer(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let (split_plus_x, split_plus_y) = find_text(buffer, &format!("{}alpha", status_symbol::ADDED_SPACED)).unwrap();
    let split_row = rendered_row(buffer, split_plus_y);
    let left_text = &split_row[..app.layout.viewer_split_right.x as usize];

    assert!(split_plus_x >= app.layout.viewer_split_right.x);
    assert!(!left_text.contains("alpha"));
    assert_eq!(buffer[(split_plus_x, split_plus_y)].fg, app.theme.COLOR_GREEN);
    assert_eq!(buffer[(split_plus_x, split_plus_y)].bg, added_bg);
}
