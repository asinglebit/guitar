use super::*;
use crate::core::{
    submodules::SubmoduleStackEntry,
    worktrees::{WorktreeEntry, WorktreeKind, Worktrees},
};

fn test_oid(byte: u8) -> gix::ObjectId {
    gix::ObjectId::from_bytes_or_panic(&[byte; 20])
}

fn worktree_entry(name: &str, head: gix::ObjectId) -> WorktreeEntry {
    WorktreeEntry {
        name: name.into(),
        path: PathBuf::from(format!("/tmp/{name}")),
        branch: Some(name.into()),
        head: Some(head),
        alias: None,
        kind: WorktreeKind::Linked,
        is_current: false,
        is_valid: true,
        is_prunable: false,
        locked_reason: None,
        is_dirty: false,
    }
}

fn app_with_graph_worktrees(entries: Vec<WorktreeEntry>) -> App {
    let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };

    let head = entries.first().and_then(|entry| entry.head).unwrap_or_else(|| test_oid(1));
    let alias = app.oids.get_alias_by_oid(head);
    let uncommitted = app.oids.sorted_aliases[0];
    app.oids.sorted_aliases = vec![uncommitted, alias];
    app.worktrees = Worktrees::from_entries(entries);
    app
}

#[test]
fn graph_worktree_target_resolution_handles_zero_one_and_multiple_rows() {
    let empty = app_with_graph_worktrees(Vec::new());
    assert!(empty.graph_worktree_indices().is_empty());

    let head = test_oid(2);
    let one = app_with_graph_worktrees(vec![worktree_entry("feature", head)]);
    assert_eq!(one.graph_worktree_indices(), vec![0]);

    let multiple = app_with_graph_worktrees(vec![worktree_entry("feature", head), worktree_entry("review", head)]);
    assert_eq!(multiple.graph_worktree_indices(), vec![0, 1]);
}

#[test]
fn opening_worktree_clears_submodule_stack() {
    let head = test_oid(7);
    let mut app = App {
        focus: Focus::Worktrees,
        submodule_stack: vec![SubmoduleStackEntry::new(PathBuf::from("/repo"), PathBuf::from("deps/child"), "deps/child".into())],
        recent_save_path: Some(PathBuf::from("/tmp/guitar-worktree-stack-recent.json")),
        ..app_with_graph_worktrees(vec![worktree_entry("feature", head)])
    };

    app.open_selected_worktree();

    assert!(app.submodule_stack.is_empty());
    assert_eq!(app.focus, Focus::Viewport);
}

#[test]
fn graph_enter_opens_chooser_for_multiple_valid_worktrees() {
    let head = test_oid(3);
    let mut app = app_with_graph_worktrees(vec![worktree_entry("feature", head), worktree_entry("review", head)]);

    app.on_select();

    assert_eq!(app.focus, Focus::ModalWorktreeChooser);
    assert_eq!(app.modal_worktree_action, WorktreeModalAction::Open);
    assert_eq!(app.modal_worktree_candidates, vec![0, 1]);
    assert_eq!(app.modal_worktree_return_focus, Focus::Viewport);
}

#[test]
fn graph_remove_uses_existing_worktree_removal_guards() {
    let head = test_oid(4);
    let mut current = worktree_entry("current", head);
    current.is_current = true;
    let mut main = worktree_entry("main", head);
    main.kind = WorktreeKind::Main;
    let mut locked = worktree_entry("locked", head);
    locked.locked_reason = Some("keep".into());
    let mut app = app_with_graph_worktrees(vec![current, main, locked]);

    app.on_remove_worktree();

    assert_eq!(app.focus, Focus::ModalError);
    assert!(app.modal_error_message.contains("cannot remove current, main, or locked worktrees"));
}

#[test]
fn graph_remove_opens_confirmation_or_chooser_for_removable_worktrees() {
    let head = test_oid(5);
    let mut one = app_with_graph_worktrees(vec![worktree_entry("feature", head)]);
    one.on_remove_worktree();
    assert_eq!(one.focus, Focus::ModalRemoveWorktree);
    assert_eq!(one.modal_worktree_target, Some(0));
    assert_eq!(one.modal_worktree_return_focus, Focus::Viewport);

    let mut multiple = app_with_graph_worktrees(vec![worktree_entry("feature", head), worktree_entry("review", head)]);
    multiple.on_remove_worktree();
    assert_eq!(multiple.focus, Focus::ModalWorktreeChooser);
    assert_eq!(multiple.modal_worktree_action, WorktreeModalAction::Remove);
    assert_eq!(multiple.modal_worktree_candidates, vec![0, 1]);
}

#[test]
fn worktree_chooser_confirmation_routes_open_and_remove_actions() {
    let head = test_oid(6);
    let mut invalid = worktree_entry("invalid", head);
    invalid.is_valid = false;
    let mut open = app_with_graph_worktrees(vec![invalid]);
    open.open_worktree_chooser(WorktreeModalAction::Open, vec![0], Focus::Viewport);
    open.confirm_worktree_chooser();
    assert_eq!(open.focus, Focus::ModalError);
    assert!(open.modal_error_message.contains("path is invalid"));

    let mut remove = app_with_graph_worktrees(vec![worktree_entry("feature", head)]);
    remove.open_worktree_chooser(WorktreeModalAction::Remove, vec![0], Focus::Viewport);
    remove.confirm_worktree_chooser();
    assert_eq!(remove.focus, Focus::ModalRemoveWorktree);
    assert_eq!(remove.modal_worktree_target, Some(0));
}
