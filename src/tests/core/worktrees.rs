use super::*;

fn named_entry(name: &str, current: bool, locked: Option<&str>) -> WorktreeEntry {
    WorktreeEntry {
        name: name.into(),
        path: PathBuf::from(format!("/tmp/{name}")),
        branch: Some(name.into()),
        head: None,
        alias: None,
        kind: WorktreeKind::Linked,
        is_current: current,
        is_valid: true,
        is_prunable: false,
        locked_reason: locked.map(str::to_string),
        is_dirty: false,
    }
}

fn linked_entry(current: bool, locked: Option<&str>) -> WorktreeEntry {
    named_entry("feature", current, locked)
}

#[test]
fn guards_current_main_and_locked_removal() {
    let mut main = linked_entry(false, None);
    main.kind = WorktreeKind::Main;
    assert!(!main.can_remove());
    assert!(!main.can_lock());

    let current = linked_entry(true, None);
    assert!(!current.can_remove());
    assert!(current.can_lock());

    let locked = linked_entry(false, Some("keep"));
    assert!(!locked.can_remove());
    assert!(locked.can_lock());

    let removable = linked_entry(false, None);
    assert!(removable.can_remove());
}

#[test]
fn current_name_returns_the_current_worktree() {
    let worktrees = Worktrees::from_entries(vec![named_entry("main", false, None), named_entry("feature", true, None)]);
    assert_eq!(worktrees.current_name(), Some("feature"));

    let worktrees = Worktrees::from_entries(vec![named_entry("main", false, None)]);
    assert_eq!(worktrees.current_name(), None);
}
