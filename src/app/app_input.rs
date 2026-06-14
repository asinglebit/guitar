use crate::{
    app::app::{App, Focus, Viewport},
    helpers::keymap::{Command, InputMode, KeyBinding, load_or_init_keymaps},
};
use ratatui::crossterm::event::KeyEvent;

impl App {
    pub fn load_keymap(&mut self) {
        self.keymaps = load_or_init_keymaps();
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        let key_binding = KeyBinding::new(key_event.code, key_event.modifiers);
        let current_mode = self.mode;

        if self.handle_modal_key_event(key_event) {
            self.mode = InputMode::Normal;
            return;
        }

        let command = self.keymaps.get(&self.mode).and_then(|mode_map| mode_map.get(&key_binding)).cloned();
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

    fn dispatch_command(&mut self, command: &Command) {
        match command {
            Command::WidenScope => self.on_widen_scope(),
            Command::NarrowScope => self.on_narrow_scope(),
            Command::FocusNextPane => self.on_focus_next_pane(),
            Command::FocusPreviousPane => self.on_focus_prev_pane(),
            Command::Select => self.on_select(),
            Command::Back => self.on_back(),
            Command::Minimize => self.on_minimize(),
            Command::ToggleZenMode => self.on_toggle_zen_mode(),
            Command::ToggleBranches => self.on_toggle_branches(),
            Command::ToggleTags => self.on_toggle_tags(),
            Command::ToggleStashes => self.on_toggle_stashes(),
            Command::ToggleWorktrees => self.on_toggle_worktrees(),
            Command::ToggleStatus => self.on_toggle_status(),
            Command::ToggleInspector => self.on_toggle_inspector(),
            Command::ToggleShas => self.on_toggle_shas(),
            Command::ToggleHelp => self.on_toggle_help(),
            Command::ActionMode => self.on_action_mode(),
            Command::Exit => self.on_exit(),
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
            Command::CreateWorktree => self.on_create_worktree(),
            Command::RemoveWorktree => self.on_remove_worktree(),
            Command::ToggleWorktreeLock => self.on_toggle_worktree_lock(),
            Command::Tag => self.on_tag(),
            Command::Untag => self.on_untag(),
            Command::Cherrypick => self.on_cherrypick(),
            Command::Rebase => self.on_rebase(),
            Command::AbortRebase => self.on_abort_rebase(),
            Command::Reload => self.on_reload(),
        }
    }

    fn dispatch_splash_command(&mut self, command: &Command) {
        match command {
            Command::NarrowScope => self.on_narrow_scope(),
            Command::Select => self.on_select(),
            Command::Back => self.on_back(),
            Command::Exit => self.on_exit(),
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
