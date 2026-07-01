use crate::{
    app::{
        app::{App, Focus},
        state::layout::Layout,
    },
    core::worktrees::{WorktreeEntry, WorktreeKind, Worktrees},
};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use std::path::PathBuf;

fn worktree_entry(name: &str) -> WorktreeEntry {
    WorktreeEntry {
        name: name.into(),
        path: PathBuf::from(format!("/tmp/{name}")),
        branch: Some(name.into()),
        head: Some(gix::ObjectId::from_bytes_or_panic(&[1; 20])),
        alias: None,
        kind: WorktreeKind::Linked,
        is_current: false,
        is_valid: true,
        is_prunable: false,
        locked_reason: None,
        is_dirty: false,
    }
}

fn worktrees_app(entries: Vec<WorktreeEntry>) -> App {
    let mut app = App { focus: Focus::Viewport, layout: Layout { worktrees: Rect::new(0, 0, 40, 5), worktrees_scrollbar: Rect::new(39, 0, 1, 5), ..Default::default() }, ..Default::default() };
    app.layout_config.is_zen = false;
    app.worktrees = Worktrees::from_entries(entries);
    app
}

#[test]
fn worktrees_short_page_stripes_blank_tail_rows() {
    let mut app = worktrees_app(vec![worktree_entry("feature")]);
    let zebra = app.theme.background_or_default(app.theme.COLOR_GREY_900);

    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_worktrees(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(2, 0)].bg, zebra);
    assert_ne!(buffer[(2, 1)].bg, zebra);
    assert_eq!(buffer[(2, 2)].bg, zebra);
}

#[test]
fn worktrees_empty_state_stripes_backdrop() {
    let mut app = worktrees_app(Vec::new());
    let zebra = app.theme.background_or_default(app.theme.COLOR_GREY_900);

    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.draw_worktrees(frame)).unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(2, 0)].bg, zebra);
    assert_ne!(buffer[(2, 1)].bg, zebra);
    assert_eq!(buffer[(2, 2)].bg, zebra);
}
