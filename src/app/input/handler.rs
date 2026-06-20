use crate::{
    app::app::{App, Focus, Viewport},
    helpers::keymap::{Command, InputMode, KeyBinding, command_for_key_binding, load_or_init_keymaps},
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    pub fn load_keymap(&mut self) {
        self.keymaps = load_or_init_keymaps();
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        let key_binding = KeyBinding::new(key_event.code, key_event.modifiers);
        let current_mode = self.mode;

        if key_event.code == KeyCode::Esc && key_event.modifiers == KeyModifiers::NONE && self.viewport == Viewport::Settings && self.focus == Focus::Viewport {
            self.on_back();
            self.mode = InputMode::Normal;
            return;
        }

        if self.handle_modal_key_event(key_event) {
            self.mode = InputMode::Normal;
            return;
        }

        if self.handle_context_menu_key_event(key_event) {
            self.mode = InputMode::Normal;
            return;
        }

        let command = self.keymaps.get(&self.mode).and_then(|mode_map| command_for_key_binding(mode_map, &key_binding));
        if let Some(command) = command {
            if self.viewport == Viewport::Splash && self.focus == Focus::Viewport {
                self.dispatch_splash_command(&command);
            } else {
                self.dispatch_command(&command);
            }
        }

        if current_mode == InputMode::Action {
            self.mode = InputMode::Normal;
        }
    }

    pub(crate) fn dispatch_command(&mut self, command: &Command) {
        match command {
            Command::WidenScope => self.on_widen_scope(),
            Command::NarrowScope => self.on_narrow_scope(),
            Command::FocusNextPane => self.on_focus_next_pane(),
            Command::FocusPreviousPane => self.on_focus_prev_pane(),
            Command::FocusPaneLeft => self.on_focus_pane_left(),
            Command::FocusPaneDown => self.on_focus_pane_down(),
            Command::FocusPaneUp => self.on_focus_pane_up(),
            Command::FocusPaneRight => self.on_focus_pane_right(),
            Command::Select => self.on_select(),
            Command::Back => self.on_back(),
            Command::Minimize => self.on_minimize(),
            Command::ResetLayout => self.on_reset_layout(),
            Command::ShrinkGraphLaneLimit => self.on_shrink_graph_lane_limit(),
            Command::GrowGraphLaneLimit => self.on_grow_graph_lane_limit(),
            Command::ResizePaneLeft => self.on_resize_pane_left(),
            Command::ResizePaneDown => self.on_resize_pane_down(),
            Command::ResizePaneUp => self.on_resize_pane_up(),
            Command::ResizePaneRight => self.on_resize_pane_right(),
            Command::ToggleZenMode => self.on_toggle_zen_mode(),
            Command::ToggleBranches => self.on_toggle_branches(),
            Command::ToggleTags => self.on_toggle_tags(),
            Command::ToggleStashes => self.on_toggle_stashes(),
            Command::ToggleReflogs => self.on_toggle_reflogs(),
            Command::ToggleGraphReflogs => self.on_toggle_graph_reflogs(),
            Command::ToggleGraphDates => self.on_toggle_graph_dates(),
            Command::ToggleGraphCommitters => self.on_toggle_graph_committers(),
            Command::ToggleGraphRefs => self.on_toggle_graph_refs(),
            Command::ToggleWorktrees => self.on_toggle_worktrees(),
            Command::ToggleSubmodules => self.on_toggle_submodules(),
            Command::ToggleSearch => self.on_toggle_search(),
            Command::ToggleStatus => self.on_toggle_status(),
            Command::ToggleInspector => self.on_toggle_inspector(),
            Command::ToggleShas => self.on_toggle_shas(),
            Command::ToggleHelp => self.on_toggle_help(),
            Command::ActionMode => self.on_action_mode(),
            Command::Exit => self.on_exit(),
            Command::RemoveRecentRepository => self.on_remove_recent_repository(),
            Command::MoveRecentRepositoryUp => self.on_move_recent_repository_up(),
            Command::MoveRecentRepositoryDown => self.on_move_recent_repository_down(),
            Command::ReturnToParentRepository => self.on_return_to_parent_repository(),
            Command::ScrollPageUp => self.on_scroll_page_up(),
            Command::ScrollPageDown => self.on_scroll_page_down(),
            Command::ScrollHalfPageUp => self.on_scroll_half_page_up(),
            Command::ScrollHalfPageDown => self.on_scroll_half_page_down(),
            Command::ScrollUp => self.on_scroll_up(),
            Command::ScrollDown => self.on_scroll_down(),
            Command::ScrollUpHalf => self.on_scroll_up_half(),
            Command::ScrollDownHalf => self.on_scroll_down_half(),
            Command::GoToBeginning => self.on_scroll_to_beginning(),
            Command::GoToEnd => self.on_scroll_to_end(),
            Command::ScrollUpBranch => self.on_scroll_up_branch(),
            Command::ScrollDownBranch => self.on_scroll_down_branch(),
            Command::ScrollUpCommit => self.on_scroll_up_commit(),
            Command::ScrollDownCommit => self.on_scroll_down_commit(),
            Command::Find => self.on_find(),
            Command::FindFile => self.on_find_file(),
            Command::SoloBranch => self.on_solo_branch(),
            Command::ToggleBranch => self.on_toggle_branch(),
            Command::ToggleHunkMode => self.on_toggle_hunk_mode(),
            Command::ToggleSplitDiffMode => self.on_toggle_split_diff_mode(),
            Command::Drop => self.on_drop(),
            Command::Pop => self.on_pop(),
            Command::Stash => self.on_stash(),
            Command::FetchAll => self.on_fetch_all(),
            Command::Checkout => self.on_checkout(),
            Command::HardReset => self.on_hard_reset(),
            Command::MixedReset => self.on_mixed_reset(),
            Command::Unstage => self.on_unstage(),
            Command::Stage => self.on_stage(),
            Command::Commit => self.on_commit(),
            Command::ForcePush => self.on_force_push(),
            Command::PushTags => self.on_push_tags(),
            Command::CreateBranch => self.on_create_branch(),
            Command::DeleteBranch => self.on_delete_branch(),
            Command::RenameBranch => self.on_rename_branch(),
            Command::CreateWorktree => self.on_create_worktree(),
            Command::RemoveWorktree => self.on_remove_worktree(),
            Command::ToggleWorktreeLock => self.on_toggle_worktree_lock(),
            Command::UpdateSubmodule => self.on_update_submodule(),
            Command::SyncSubmodule => self.on_sync_submodule(),
            Command::Tag => self.on_tag(),
            Command::Untag => self.on_untag(),
            Command::Cherrypick => self.on_cherrypick(),
            Command::Revert => self.on_revert(),
            Command::Rebase => self.on_rebase(),
            Command::Merge => self.on_merge(),
            Command::ContinueOperation => self.on_continue_operation(),
            Command::AbortOperation => self.on_abort_operation(),
            Command::Reload => self.on_reload(),
            Command::ReloadAllBranches => self.on_reload_all_branches(),
        }
    }

    fn dispatch_splash_command(&mut self, command: &Command) {
        match command {
            Command::NarrowScope => self.on_narrow_scope(),
            Command::Select => self.on_select(),
            Command::Back => self.on_back(),
            Command::Exit => self.on_exit(),
            Command::RemoveRecentRepository => self.on_remove_recent_repository(),
            Command::MoveRecentRepositoryUp => self.on_move_recent_repository_up(),
            Command::MoveRecentRepositoryDown => self.on_move_recent_repository_down(),
            Command::ReturnToParentRepository => self.on_return_to_parent_repository(),
            Command::ScrollPageUp => self.on_scroll_page_up(),
            Command::ScrollPageDown => self.on_scroll_page_down(),
            Command::ScrollHalfPageUp => self.on_scroll_half_page_up(),
            Command::ScrollHalfPageDown => self.on_scroll_half_page_down(),
            Command::ScrollUp => self.on_scroll_up(),
            Command::ScrollDown => self.on_scroll_down(),
            Command::ScrollUpHalf => self.on_scroll_up_half(),
            Command::ScrollDownHalf => self.on_scroll_down_half(),
            Command::GoToBeginning => self.on_scroll_to_beginning(),
            Command::GoToEnd => self.on_scroll_to_end(),
            _ => {},
        }
    }
}
