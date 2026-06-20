use super::*;

#[test]
fn defaults_include_recent_repository_bindings() {
    let maps = default_keymaps();

    for mode in [InputMode::Normal, InputMode::Action] {
        let mode_map = maps.get(&mode).unwrap();
        assert_eq!(mode_map.get(&KeyBinding::new(Char('d'), KeyModifiers::NONE)), Some(&Command::RemoveRecentRepository));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('K'), KeyModifiers::SHIFT)), Some(&Command::MoveRecentRepositoryUp));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('J'), KeyModifiers::SHIFT)), Some(&Command::MoveRecentRepositoryDown));
        assert_eq!(mode_map.get(&KeyBinding::new(Backspace, KeyModifiers::NONE)), Some(&Command::ReturnToParentRepository));
    }
}

#[test]
fn defaults_include_normal_only_graph_lane_limit_bindings() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();
    let action = maps.get(&InputMode::Action).unwrap();

    assert_eq!(normal.get(&KeyBinding::new(Char('-'), KeyModifiers::NONE)), Some(&Command::ShrinkGraphLaneLimit));
    assert_eq!(normal.get(&KeyBinding::new(Char('+'), KeyModifiers::NONE)), Some(&Command::GrowGraphLaneLimit));
    assert_eq!(action.get(&KeyBinding::new(Char('-'), KeyModifiers::NONE)), None);
    assert_eq!(action.get(&KeyBinding::new(Char('+'), KeyModifiers::NONE)), None);
    assert!(!action.values().any(|command| matches!(command, Command::ShrinkGraphLaneLimit | Command::GrowGraphLaneLimit)));
}

#[test]
fn defaults_include_numeric_ui_toggles() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();
    let action = maps.get(&InputMode::Action).unwrap();

    for mode_map in [normal, action] {
        assert_eq!(mode_map.get(&KeyBinding::new(Char('0'), KeyModifiers::NONE)), Some(&Command::ResetLayout));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('1'), KeyModifiers::NONE)), Some(&Command::ToggleBranches));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('2'), KeyModifiers::NONE)), Some(&Command::ToggleTags));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('3'), KeyModifiers::NONE)), Some(&Command::ToggleStashes));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('4'), KeyModifiers::NONE)), Some(&Command::ToggleReflogs));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('5'), KeyModifiers::NONE)), Some(&Command::ToggleWorktrees));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('6'), KeyModifiers::NONE)), Some(&Command::ToggleSubmodules));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('7'), KeyModifiers::NONE)), Some(&Command::ToggleSearch));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('8'), KeyModifiers::NONE)), Some(&Command::ToggleInspector));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('9'), KeyModifiers::NONE)), Some(&Command::ToggleStatus));
    }
}

#[test]
fn defaults_include_shift_numeric_graph_metadata_toggles() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();
    let action = maps.get(&InputMode::Action).unwrap();

    for mode_map in [normal, action] {
        assert_eq!(mode_map.get(&KeyBinding::new(Char('0'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphReflogs));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('1'), KeyModifiers::SHIFT)), Some(&Command::ToggleShas));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('2'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphDates));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('3'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphCommitters));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('4'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphRefs));
    }
}

#[test]
fn visual_string_renders_shift_digits_as_symbols() {
    assert_eq!(keybinding_to_visual_string(&KeyBinding::new(Char('0'), KeyModifiers::SHIFT)), ")");
    assert_eq!(keybinding_to_visual_string(&KeyBinding::new(Char('1'), KeyModifiers::SHIFT)), "!");
    assert_eq!(keybinding_to_visual_string(&KeyBinding::new(Char('2'), KeyModifiers::SHIFT)), "@");
    assert_eq!(keybinding_to_visual_string(&KeyBinding::new(Char('3'), KeyModifiers::SHIFT)), "#");
    assert_eq!(keybinding_to_visual_string(&KeyBinding::new(Char('4'), KeyModifiers::SHIFT)), "$");
    assert_eq!(keybinding_to_visual_string(&KeyBinding::new(Char('1'), KeyModifiers::ALT | KeyModifiers::SHIFT)), "Alt + !");
}

#[test]
fn command_lookup_falls_back_from_shifted_punctuation_to_shift_digits() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();

    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char(')'), KeyModifiers::NONE)), Some(Command::ToggleGraphReflogs));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('!'), KeyModifiers::NONE)), Some(Command::ToggleShas));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('@'), KeyModifiers::NONE)), Some(Command::ToggleGraphDates));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('#'), KeyModifiers::NONE)), Some(Command::ToggleGraphCommitters));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('$'), KeyModifiers::NONE)), Some(Command::ToggleGraphRefs));
}

#[test]
fn command_lookup_supports_common_non_us_shift_digit_punctuation() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();

    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('='), KeyModifiers::NONE)), Some(Command::ToggleGraphReflogs));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('"'), KeyModifiers::NONE)), Some(Command::ToggleGraphDates));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('§'), KeyModifiers::NONE)), Some(Command::ToggleGraphCommitters));
}

#[test]
fn command_lookup_prefers_exact_binding_over_shift_digit_fallback() {
    let mut map = IndexMap::new();
    map.insert(KeyBinding::new(Char('2'), KeyModifiers::SHIFT), Command::ToggleGraphDates);
    map.insert(KeyBinding::new(Char('@'), KeyModifiers::NONE), Command::Find);
    map.insert(KeyBinding::new(Char('3'), KeyModifiers::SHIFT), Command::ToggleGraphCommitters);
    map.insert(KeyBinding::new(Char('#'), KeyModifiers::SHIFT), Command::FindFile);

    assert_eq!(command_for_key_binding(&map, &KeyBinding::new(Char('@'), KeyModifiers::NONE)), Some(Command::Find));
    assert_eq!(command_for_key_binding(&map, &KeyBinding::new(Char('#'), KeyModifiers::SHIFT)), Some(Command::FindFile));
}

#[test]
fn command_lookup_falls_back_from_legacy_ctrl_digits_to_shift_digits() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();

    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('0'), KeyModifiers::CONTROL)), Some(Command::ToggleGraphReflogs));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('1'), KeyModifiers::CONTROL)), Some(Command::ToggleShas));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('2'), KeyModifiers::CONTROL)), Some(Command::ToggleGraphDates));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('3'), KeyModifiers::CONTROL)), Some(Command::ToggleGraphCommitters));
    assert_eq!(command_for_key_binding(normal, &KeyBinding::new(Char('4'), KeyModifiers::CONTROL)), Some(Command::ToggleGraphRefs));
}

#[test]
fn command_lookup_falls_back_to_existing_legacy_ctrl_digit_bindings() {
    let mut map = IndexMap::new();
    map.insert(KeyBinding::new(Char('2'), KeyModifiers::CONTROL), Command::ToggleGraphDates);
    map.insert(KeyBinding::new(Char('3'), KeyModifiers::CONTROL), Command::ToggleGraphCommitters);

    assert_eq!(command_for_key_binding(&map, &KeyBinding::new(Char('2'), KeyModifiers::SHIFT)), Some(Command::ToggleGraphDates));
    assert_eq!(command_for_key_binding(&map, &KeyBinding::new(Char('@'), KeyModifiers::NONE)), Some(Command::ToggleGraphDates));
    assert_eq!(command_for_key_binding(&map, &KeyBinding::new(Char('#'), KeyModifiers::NONE)), Some(Command::ToggleGraphCommitters));
}

#[test]
fn defaults_bind_normal_shift_r_to_reload_all_and_action_shift_r_to_revert() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();
    let action = maps.get(&InputMode::Action).unwrap();

    assert_eq!(normal.get(&KeyBinding::new(Char('R'), KeyModifiers::SHIFT)), Some(&Command::ReloadAllBranches));
    assert_eq!(action.get(&KeyBinding::new(Char('R'), KeyModifiers::SHIFT)), Some(&Command::Revert));
}

#[test]
fn defaults_include_file_search_binding() {
    let maps = default_keymaps();

    for mode in [InputMode::Normal, InputMode::Action] {
        let mode_map = maps.get(&mode).unwrap();
        assert_eq!(mode_map.get(&KeyBinding::new(Char('F'), KeyModifiers::SHIFT)), Some(&Command::FindFile));
    }
}

#[test]
fn existing_keymaps_gain_search_toggle_when_available() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let mut action = IndexMap::new();
    action.insert(KeyBinding::new(Char('k'), KeyModifiers::NONE), Command::ScrollUp);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, action);

    assert!(ensure_default_keymap_bindings(&mut maps));
    assert_eq!(maps.get(&InputMode::Normal).unwrap().get(&KeyBinding::new(Char('7'), KeyModifiers::NONE)), Some(&Command::ToggleSearch));
    assert_eq!(maps.get(&InputMode::Action).unwrap().get(&KeyBinding::new(Char('7'), KeyModifiers::NONE)), Some(&Command::ToggleSearch));
    assert_eq!(maps.get(&InputMode::Normal).unwrap().get(&KeyBinding::new(Char('F'), KeyModifiers::SHIFT)), Some(&Command::FindFile));
    assert_eq!(maps.get(&InputMode::Action).unwrap().get(&KeyBinding::new(Char('F'), KeyModifiers::SHIFT)), Some(&Command::FindFile));
    assert_eq!(maps.get(&InputMode::Normal).unwrap().get(&KeyBinding::new(Char('6'), KeyModifiers::NONE)), Some(&Command::ToggleSubmodules));
    assert_eq!(maps.get(&InputMode::Action).unwrap().get(&KeyBinding::new(Char('6'), KeyModifiers::NONE)), Some(&Command::ToggleSubmodules));
    assert_eq!(maps.get(&InputMode::Normal).unwrap().get(&KeyBinding::new(Char('2'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphDates));
    assert_eq!(maps.get(&InputMode::Action).unwrap().get(&KeyBinding::new(Char('4'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphRefs));
    assert_eq!(maps.get(&InputMode::Normal).unwrap().get(&KeyBinding::new(Char('R'), KeyModifiers::SHIFT)), Some(&Command::ReloadAllBranches));
    assert_eq!(maps.get(&InputMode::Normal).unwrap().get(&KeyBinding::new(Backspace, KeyModifiers::NONE)), Some(&Command::ReturnToParentRepository));
    assert_eq!(maps.get(&InputMode::Action).unwrap().get(&KeyBinding::new(Char('R'), KeyModifiers::SHIFT)), None);
    assert_eq!(maps.get(&InputMode::Action).unwrap().get(&KeyBinding::new(Backspace, KeyModifiers::NONE)), None);
}

#[test]
fn existing_keymaps_gain_graph_lane_limit_shortcuts_only_in_normal_when_available() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let mut action = IndexMap::new();
    action.insert(KeyBinding::new(Char('k'), KeyModifiers::NONE), Command::ScrollUp);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, action);

    assert!(ensure_default_keymap_bindings(&mut maps));

    let normal = maps.get(&InputMode::Normal).unwrap();
    let action = maps.get(&InputMode::Action).unwrap();
    assert_eq!(normal.get(&KeyBinding::new(Char('-'), KeyModifiers::NONE)), Some(&Command::ShrinkGraphLaneLimit));
    assert_eq!(normal.get(&KeyBinding::new(Char('+'), KeyModifiers::NONE)), Some(&Command::GrowGraphLaneLimit));
    assert_eq!(action.get(&KeyBinding::new(Char('-'), KeyModifiers::NONE)), None);
    assert_eq!(action.get(&KeyBinding::new(Char('+'), KeyModifiers::NONE)), None);
    assert!(!action.values().any(|command| matches!(command, Command::ShrinkGraphLaneLimit | Command::GrowGraphLaneLimit)));
}

#[test]
fn existing_keymaps_do_not_override_search_conflicts() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('`'), KeyModifiers::NONE), Command::Reload);
    normal.insert(KeyBinding::new(Char('s'), KeyModifiers::CONTROL), Command::ToggleSearch);
    normal.insert(KeyBinding::new(Char('F'), KeyModifiers::SHIFT), Command::FetchAll);
    normal.insert(KeyBinding::new(Char('f'), KeyModifiers::CONTROL), Command::FindFile);
    normal.insert(KeyBinding::new(Char('\\'), KeyModifiers::NONE), Command::Reload);
    normal.insert(KeyBinding::new(Char('x'), KeyModifiers::CONTROL), Command::ToggleSubmodules);
    normal.insert(KeyBinding::new(Char('R'), KeyModifiers::SHIFT), Command::Revert);
    normal.insert(KeyBinding::new(Backspace, KeyModifiers::NONE), Command::Reload);
    normal.insert(KeyBinding::new(Char('p'), KeyModifiers::CONTROL), Command::ReturnToParentRepository);
    let mut action = IndexMap::new();
    action.insert(KeyBinding::new(Char('`'), KeyModifiers::NONE), Command::Reload);
    action.insert(KeyBinding::new(Char('s'), KeyModifiers::CONTROL), Command::ToggleSearch);
    action.insert(KeyBinding::new(Char('F'), KeyModifiers::SHIFT), Command::FetchAll);
    action.insert(KeyBinding::new(Char('f'), KeyModifiers::CONTROL), Command::FindFile);
    action.insert(KeyBinding::new(Char('\\'), KeyModifiers::NONE), Command::Reload);
    action.insert(KeyBinding::new(Char('x'), KeyModifiers::CONTROL), Command::ToggleSubmodules);
    action.insert(KeyBinding::new(Char('B'), KeyModifiers::SHIFT), Command::RenameBranch);
    action.insert(KeyBinding::new(Char('i'), KeyModifiers::NONE), Command::Reload);
    action.insert(KeyBinding::new(Char('I'), KeyModifiers::SHIFT), Command::ReloadAllBranches);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, action);

    assert!(ensure_default_keymap_bindings(&mut maps));
    let normal = maps.get(&InputMode::Normal).unwrap();
    assert_eq!(normal.get(&KeyBinding::new(Char('`'), KeyModifiers::NONE)), Some(&Command::Reload));
    assert_eq!(normal.get(&KeyBinding::new(Char('s'), KeyModifiers::CONTROL)), Some(&Command::ToggleSearch));
    assert_eq!(normal.get(&KeyBinding::new(Char('F'), KeyModifiers::SHIFT)), Some(&Command::FetchAll));
    assert_eq!(normal.get(&KeyBinding::new(Char('f'), KeyModifiers::CONTROL)), Some(&Command::FindFile));
    assert_eq!(normal.get(&KeyBinding::new(Char('\\'), KeyModifiers::NONE)), Some(&Command::Reload));
    assert_eq!(normal.get(&KeyBinding::new(Char('x'), KeyModifiers::CONTROL)), Some(&Command::ToggleSubmodules));
    assert_eq!(normal.get(&KeyBinding::new(Backspace, KeyModifiers::NONE)), Some(&Command::Reload));
    assert_eq!(normal.get(&KeyBinding::new(Char('p'), KeyModifiers::CONTROL)), Some(&Command::ReturnToParentRepository));
}

#[test]
fn existing_keymaps_do_not_override_graph_lane_limit_shortcut_conflicts() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('-'), KeyModifiers::NONE), Command::Reload);
    normal.insert(KeyBinding::new(Char('+'), KeyModifiers::NONE), Command::ToggleSearch);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, IndexMap::new());

    assert!(ensure_default_keymap_bindings(&mut maps));

    let normal = maps.get(&InputMode::Normal).unwrap();
    assert_eq!(normal.get(&KeyBinding::new(Char('-'), KeyModifiers::NONE)), Some(&Command::Reload));
    assert_eq!(normal.get(&KeyBinding::new(Char('+'), KeyModifiers::NONE)), Some(&Command::ToggleSearch));
    assert!(!normal.values().any(|command| matches!(command, Command::ShrinkGraphLaneLimit | Command::GrowGraphLaneLimit)));
}

#[test]
fn existing_keymaps_rewrite_untouched_old_numeric_ui_defaults() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    for (key, command) in old_numeric_ui_defaults() {
        normal.insert(key, command);
    }
    let action = normal.clone();
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, action);

    assert!(ensure_default_keymap_bindings(&mut maps));

    for mode in [InputMode::Normal, InputMode::Action] {
        let mode_map = maps.get(&mode).unwrap();
        assert_eq!(mode_map.get(&KeyBinding::new(Char('4'), KeyModifiers::NONE)), Some(&Command::ToggleReflogs));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('6'), KeyModifiers::NONE)), Some(&Command::ToggleSubmodules));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('7'), KeyModifiers::NONE)), Some(&Command::ToggleSearch));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('8'), KeyModifiers::NONE)), Some(&Command::ToggleInspector));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('9'), KeyModifiers::NONE)), Some(&Command::ToggleStatus));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('1'), KeyModifiers::SHIFT)), Some(&Command::ToggleShas));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('4'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphRefs));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('\\'), KeyModifiers::NONE)), None);
        assert_eq!(mode_map.get(&KeyBinding::new(Char('`'), KeyModifiers::NONE)), None);
    }
}

#[test]
fn existing_keymaps_rewrite_untouched_ctrl_graph_metadata_defaults() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    for (key, command) in ctrl_graph_metadata_defaults() {
        normal.insert(key, command);
    }
    let action = normal.clone();
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, action);

    assert!(ensure_default_keymap_bindings(&mut maps));

    for mode in [InputMode::Normal, InputMode::Action] {
        let mode_map = maps.get(&mode).unwrap();
        assert_eq!(mode_map.get(&KeyBinding::new(Char('0'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphReflogs));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('1'), KeyModifiers::SHIFT)), Some(&Command::ToggleShas));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('2'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphDates));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('3'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphCommitters));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('4'), KeyModifiers::SHIFT)), Some(&Command::ToggleGraphRefs));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('0'), KeyModifiers::CONTROL)), None);
        assert_eq!(mode_map.get(&KeyBinding::new(Char('4'), KeyModifiers::CONTROL)), None);
    }
}

#[test]
fn defaults_include_keyboard_resize_bindings() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();
    let action = maps.get(&InputMode::Action).unwrap();
    let mods = KeyModifiers::CONTROL | KeyModifiers::ALT;

    for mode_map in [normal, action] {
        assert_eq!(mode_map.get(&KeyBinding::new(Char('h'), mods)), Some(&Command::ResizePaneLeft));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('j'), mods)), Some(&Command::ResizePaneDown));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('k'), mods)), Some(&Command::ResizePaneUp));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('l'), mods)), Some(&Command::ResizePaneRight));
        assert_eq!(mode_map.get(&KeyBinding::new(Left, mods)), Some(&Command::ResizePaneLeft));
        assert_eq!(mode_map.get(&KeyBinding::new(Down, mods)), Some(&Command::ResizePaneDown));
        assert_eq!(mode_map.get(&KeyBinding::new(Up, mods)), Some(&Command::ResizePaneUp));
        assert_eq!(mode_map.get(&KeyBinding::new(Right, mods)), Some(&Command::ResizePaneRight));
    }
}

#[test]
fn defaults_include_directional_focus_bindings() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();
    let action = maps.get(&InputMode::Action).unwrap();
    let mods = KeyModifiers::CONTROL;

    for mode_map in [normal, action] {
        assert_eq!(mode_map.get(&KeyBinding::new(Char('h'), mods)), Some(&Command::FocusPaneLeft));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('j'), mods)), Some(&Command::FocusPaneDown));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('k'), mods)), Some(&Command::FocusPaneUp));
        assert_eq!(mode_map.get(&KeyBinding::new(Char('l'), mods)), Some(&Command::FocusPaneRight));
        assert_eq!(mode_map.get(&KeyBinding::new(Left, mods)), Some(&Command::FocusPaneLeft));
        assert_eq!(mode_map.get(&KeyBinding::new(Down, mods)), Some(&Command::FocusPaneDown));
        assert_eq!(mode_map.get(&KeyBinding::new(Up, mods)), Some(&Command::FocusPaneUp));
        assert_eq!(mode_map.get(&KeyBinding::new(Right, mods)), Some(&Command::FocusPaneRight));
    }
}

#[test]
fn defaults_include_operation_bindings() {
    let maps = default_keymaps();
    let action = maps.get(&InputMode::Action).unwrap();

    assert_eq!(action.get(&KeyBinding::new(Char('B'), KeyModifiers::SHIFT)), Some(&Command::RenameBranch));
    assert_eq!(action.get(&KeyBinding::new(Char('r'), KeyModifiers::NONE)), Some(&Command::Rebase));
    assert_eq!(action.get(&KeyBinding::new(Char('R'), KeyModifiers::SHIFT)), Some(&Command::Revert));
    assert_eq!(action.get(&KeyBinding::new(Char('m'), KeyModifiers::NONE)), Some(&Command::Merge));
    assert_eq!(action.get(&KeyBinding::new(Char('C'), KeyModifiers::SHIFT)), Some(&Command::ContinueOperation));
    assert_eq!(action.get(&KeyBinding::new(Char('A'), KeyModifiers::SHIFT)), Some(&Command::AbortOperation));
    assert_eq!(action.get(&KeyBinding::new(Char('i'), KeyModifiers::NONE)), Some(&Command::UpdateSubmodule));
    assert_eq!(action.get(&KeyBinding::new(Char('I'), KeyModifiers::SHIFT)), Some(&Command::SyncSubmodule));
}

#[test]
fn remote_management_does_not_change_existing_default_keymaps() {
    let maps = default_keymaps();
    let normal = maps.get(&InputMode::Normal).unwrap();
    let action = maps.get(&InputMode::Action).unwrap();

    assert_eq!(normal.get(&KeyBinding::new(Char('f'), KeyModifiers::NONE)), Some(&Command::FetchAll));
    assert_eq!(action.get(&KeyBinding::new(Char('f'), KeyModifiers::NONE)), Some(&Command::FetchAll));
    assert_eq!(normal.get(&KeyBinding::new(Char('a'), KeyModifiers::CONTROL)), Some(&Command::ActionMode));
    assert_eq!(action.get(&KeyBinding::new(Char('D'), KeyModifiers::SHIFT)), Some(&Command::DeleteBranch));
}

#[test]
fn existing_keymaps_gain_rename_branch_when_available() {
    let mut maps = IndexMap::new();
    maps.insert(InputMode::Normal, IndexMap::new());
    maps.insert(InputMode::Action, IndexMap::new());

    assert!(ensure_default_keymap_bindings(&mut maps));

    let action = maps.get(&InputMode::Action).unwrap();
    assert_eq!(action.get(&KeyBinding::new(Char('B'), KeyModifiers::SHIFT)), Some(&Command::RenameBranch));
}

#[test]
fn existing_keymaps_do_not_override_rename_branch_conflicts() {
    let mut maps = IndexMap::new();
    maps.insert(InputMode::Normal, IndexMap::new());
    let mut action = IndexMap::new();
    action.insert(KeyBinding::new(Char('B'), KeyModifiers::SHIFT), Command::CreateBranch);
    maps.insert(InputMode::Action, action);

    assert!(ensure_default_keymap_bindings(&mut maps));

    let action = maps.get(&InputMode::Action).unwrap();
    assert_eq!(action.get(&KeyBinding::new(Char('B'), KeyModifiers::SHIFT)), Some(&Command::CreateBranch));
    assert!(!action.values().any(|command| command == &Command::RenameBranch));
}

#[test]
fn action_settings_filter_hides_only_identical_inherited_bindings() {
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    normal.insert(KeyBinding::new(Char('r'), KeyModifiers::NONE), Command::Reload);

    let mut action = IndexMap::new();
    action.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    action.insert(KeyBinding::new(Char('r'), KeyModifiers::NONE), Command::Rebase);
    action.insert(KeyBinding::new(Char('m'), KeyModifiers::NONE), Command::Merge);
    action.insert(KeyBinding::new(Char('C'), KeyModifiers::SHIFT), Command::ContinueOperation);
    action.insert(KeyBinding::new(Char('A'), KeyModifiers::SHIFT), Command::AbortOperation);

    let visible = action_keymap_visible_entries(Some(&normal), &action);

    assert_eq!(visible.get(&KeyBinding::new(Char('j'), KeyModifiers::NONE)), None);
    assert_eq!(visible.get(&KeyBinding::new(Char('r'), KeyModifiers::NONE)), Some(&Command::Rebase));
    assert_eq!(visible.get(&KeyBinding::new(Char('m'), KeyModifiers::NONE)), Some(&Command::Merge));
    assert_eq!(visible.get(&KeyBinding::new(Char('C'), KeyModifiers::SHIFT)), Some(&Command::ContinueOperation));
    assert_eq!(visible.get(&KeyBinding::new(Char('A'), KeyModifiers::SHIFT)), Some(&Command::AbortOperation));
}

#[test]
fn rebind_keymap_selection_updates_one_binding_and_preserves_order() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    normal.insert(KeyBinding::new(Char('r'), KeyModifiers::NONE), Command::Reload);
    normal.insert(KeyBinding::new(Char('f'), KeyModifiers::NONE), Command::FetchAll);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, IndexMap::new());

    let outcome =
        rebind_keymap_selection(&mut maps, &KeymapSelection::new(InputMode::Normal, KeyBinding::new(Char('r'), KeyModifiers::NONE), Command::Reload), KeyBinding::new(F(2), KeyModifiers::NONE))
            .unwrap();

    let normal = maps.get(&InputMode::Normal).unwrap();
    let keys: Vec<KeyBinding> = normal.keys().cloned().collect();
    assert_eq!(keys, vec![KeyBinding::new(Char('j'), KeyModifiers::NONE), KeyBinding::new(F(2), KeyModifiers::NONE), KeyBinding::new(Char('f'), KeyModifiers::NONE)]);
    assert_eq!(normal.get(&KeyBinding::new(F(2), KeyModifiers::NONE)), Some(&Command::Reload));
    assert!(!outcome.synced_action);
}

#[test]
fn rebind_keymap_selection_is_noop_for_same_key() {
    let mut maps = default_keymaps();
    let before = maps.clone();

    let outcome = rebind_keymap_selection(
        &mut maps,
        &KeymapSelection::new(InputMode::Normal, KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown),
        KeyBinding::new(Char('j'), KeyModifiers::NONE),
    )
    .unwrap();

    assert_eq!(maps, before);
    assert!(outcome.synced_action);
}

#[test]
fn rebind_keymap_selection_collapses_same_command_duplicate() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    normal.insert(KeyBinding::new(Down, KeyModifiers::NONE), Command::ScrollDown);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, IndexMap::new());

    rebind_keymap_selection(&mut maps, &KeymapSelection::new(InputMode::Normal, KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown), KeyBinding::new(Down, KeyModifiers::NONE))
        .unwrap();

    let normal = maps.get(&InputMode::Normal).unwrap();
    assert_eq!(normal.len(), 1);
    assert_eq!(normal.get(&KeyBinding::new(Down, KeyModifiers::NONE)), Some(&Command::ScrollDown));
}

#[test]
fn rebind_keymap_selection_blocks_same_mode_conflict() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    normal.insert(KeyBinding::new(Char('k'), KeyModifiers::NONE), Command::ScrollUp);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, IndexMap::new());
    let before = maps.clone();

    let result = rebind_keymap_selection(
        &mut maps,
        &KeymapSelection::new(InputMode::Normal, KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown),
        KeyBinding::new(Char('k'), KeyModifiers::NONE),
    );

    assert_eq!(result, Err(KeymapEditError::Conflict { mode: InputMode::Normal, key: KeyBinding::new(Char('k'), KeyModifiers::NONE), command: Command::ScrollUp }));
    assert_eq!(maps, before);
}

#[test]
fn rebind_keymap_selection_syncs_inherited_action_binding() {
    let mut maps = default_keymaps();

    let outcome = rebind_keymap_selection(
        &mut maps,
        &KeymapSelection::new(InputMode::Normal, KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown),
        KeyBinding::new(Char('n'), KeyModifiers::ALT),
    )
    .unwrap();

    assert!(outcome.synced_action);
    assert_eq!(maps.get(&InputMode::Normal).unwrap().get(&KeyBinding::new(Char('n'), KeyModifiers::ALT)), Some(&Command::ScrollDown));
    assert_eq!(maps.get(&InputMode::Action).unwrap().get(&KeyBinding::new(Char('n'), KeyModifiers::ALT)), Some(&Command::ScrollDown));
    assert_eq!(maps.get(&InputMode::Normal).unwrap().get(&KeyBinding::new(Char('j'), KeyModifiers::NONE)), None);
    assert_eq!(maps.get(&InputMode::Action).unwrap().get(&KeyBinding::new(Char('j'), KeyModifiers::NONE)), None);
}

#[test]
fn rebind_keymap_selection_rolls_back_when_action_sync_conflicts() {
    let mut maps = IndexMap::new();
    let mut normal = IndexMap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    let mut action = IndexMap::new();
    action.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    action.insert(KeyBinding::new(Char('x'), KeyModifiers::NONE), Command::Pop);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, action);
    let before = maps.clone();

    let result = rebind_keymap_selection(
        &mut maps,
        &KeymapSelection::new(InputMode::Normal, KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown),
        KeyBinding::new(Char('x'), KeyModifiers::NONE),
    );

    assert_eq!(result, Err(KeymapEditError::Conflict { mode: InputMode::Action, key: KeyBinding::new(Char('x'), KeyModifiers::NONE), command: Command::Pop }));
    assert_eq!(maps, before);
}

#[test]
fn keymaps_round_trip_through_disk() {
    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-keymap-round-trip-{id}")).join("keymap.json");
    let mut maps = default_keymaps();
    rebind_keymap_selection(&mut maps, &KeymapSelection::new(InputMode::Normal, KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown), KeyBinding::new(Char('n'), KeyModifiers::ALT))
        .unwrap();

    save_keymaps_to_path(path.as_path(), &maps).unwrap();
    let contents = std::fs::read_to_string(path.as_path()).unwrap();
    assert!(contents.contains('\n'), "{contents}");
    assert!(contents.contains("\n  \"normal\""), "{contents}");
    assert!(contents.contains("\n  \"action\""), "{contents}");
    let loaded = load_keymaps_from_path(path.as_path()).unwrap();

    assert_eq!(loaded, maps);
    assert_eq!(loaded.get(&InputMode::Action).unwrap().get(&KeyBinding::new(Char('R'), KeyModifiers::SHIFT)), Some(&Command::Revert));
}

#[test]
fn keymap_config_serialization_stays_english_while_visual_labels_localise() {
    crate::helpers::localisation::set_active_language(crate::helpers::localisation::Language::Spanish);
    assert_eq!(command_to_visual_string(&Command::ScrollDown), "Desplazar abajo");

    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-keymap-localised-{id}")).join("keymap.json");
    let mut maps = Keymaps::new();
    let mut normal = ModeKeymap::new();
    normal.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);
    maps.insert(InputMode::Normal, normal);
    maps.insert(InputMode::Action, ModeKeymap::new());

    save_keymaps_to_path(path.as_path(), &maps).unwrap();
    let contents = std::fs::read_to_string(path.as_path()).unwrap();

    assert!(contents.contains("\"command\": \"ScrollDown\""), "{contents}");
    assert!(!contents.contains("Desplazar"), "{contents}");

    crate::helpers::localisation::set_active_language(crate::helpers::localisation::Language::English);
}
