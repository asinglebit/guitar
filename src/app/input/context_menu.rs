use crate::{
    app::{
        app::{App, ContextMenuAction, ContextMenuItem, ContextMenuState, Direction, Focus, MouseSelectionTarget, SettingsSelectionKind, SettingsTab, Viewport},
        input::remotes::REMOTE_ACTIONS,
        state::defaults::ViewerMode,
    },
    git::queries::commits::get_current_branch,
    helpers::keymap::{Command, command_to_visual_string},
};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
};

impl App {
    pub(crate) fn open_context_menu(&mut self, column: u16, row: u16) {
        if self.is_modal_focus() {
            return;
        }

        self.mouse_drag = None;
        self.last_mouse_click = None;

        let target = self.mouse_selection_target_at(column, row);
        if let Some(target) = target {
            self.select_mouse_target(target);
        }

        let items = self.context_menu_items_for_target(target);
        if items.is_empty() {
            self.context_menu = None;
            return;
        }

        self.context_menu = Some(ContextMenuState { column, row, selected: Self::first_enabled_context_menu_index(&items), items });
    }

    pub(crate) fn close_context_menu(&mut self) {
        self.context_menu = None;
    }

    pub(crate) fn handle_context_menu_key_event(&mut self, key_event: KeyEvent) -> bool {
        if self.context_menu.is_none() || self.is_modal_focus() {
            return false;
        }

        if key_event.modifiers != KeyModifiers::NONE {
            return true;
        }

        match key_event.code {
            KeyCode::Esc => self.close_context_menu(),
            KeyCode::Enter => self.activate_context_menu_selected(),
            KeyCode::Up | KeyCode::Char('k') => self.move_context_menu_selection(Direction::Up),
            KeyCode::Down | KeyCode::Char('j') => self.move_context_menu_selection(Direction::Down),
            _ => {},
        }

        true
    }

    pub(crate) fn handle_context_menu_left_click(&mut self, column: u16, row: u16) -> bool {
        if self.context_menu.is_none() {
            return false;
        }

        self.mouse_drag = None;
        self.last_mouse_click = None;

        if let Some(index) = self.context_menu_item_at(column, row) {
            let item = self.context_menu.as_ref().and_then(|menu| menu.items.get(index)).cloned();
            if let Some(item) = item
                && item.enabled
            {
                if let Some(menu) = &mut self.context_menu {
                    menu.selected = index;
                }
                self.activate_context_menu_action(item.action);
            }
            return true;
        }

        if self.context_menu_area_for_input().is_some_and(|area| rect_contains(area, column, row)) {
            return true;
        }

        self.close_context_menu();
        true
    }

    pub(crate) fn context_menu_area_for_bounds(&self, bounds: Rect) -> Option<Rect> {
        self.context_menu.as_ref().map(|menu| menu.area(bounds)).filter(|area| area.width > 0 && area.height > 0)
    }

    fn context_menu_items_for_target(&self, target: Option<MouseSelectionTarget>) -> Vec<ContextMenuItem> {
        let mut items = match target {
            Some(MouseSelectionTarget::Graph(index)) => self.graph_context_menu_items(index, false, true),
            Some(MouseSelectionTarget::Viewer(_)) => self.viewer_context_menu_items(),
            Some(MouseSelectionTarget::Branches(_)) => self.branch_context_menu_items(),
            Some(MouseSelectionTarget::Tags(_)) => self.tag_context_menu_items(),
            Some(MouseSelectionTarget::Stashes(_)) => self.stash_context_menu_items(),
            Some(MouseSelectionTarget::Reflogs(_)) => self.reflog_context_menu_items(),
            Some(MouseSelectionTarget::Worktrees(index)) => self.worktree_context_menu_items(index),
            Some(MouseSelectionTarget::Submodules(index)) => self.submodule_context_menu_items(index),
            Some(MouseSelectionTarget::Inspector(_)) => self.inspector_context_menu_items(),
            Some(MouseSelectionTarget::StatusTop(_)) => self.status_context_menu_items(true),
            Some(MouseSelectionTarget::StatusBottom(_)) => self.status_context_menu_items(false),
            Some(MouseSelectionTarget::Search(_)) => self.search_context_menu_items(),
            Some(MouseSelectionTarget::Splash(index)) => self.splash_context_menu_items(index),
            Some(MouseSelectionTarget::Settings(index)) => self.settings_context_menu_items(index),
            Some(MouseSelectionTarget::SettingsTab(tab)) => vec![Self::item(format!("Open {}", tab.label()), ContextMenuAction::SwitchSettingsTab(tab), true)],
            None => Vec::new(),
        };

        if self.repo.is_some() {
            items.insert(0, Self::command_item("Reload", Command::Reload));
        }

        let global_items = self.global_context_menu_items();
        if !items.is_empty() && !global_items.is_empty() {
            items.push(Self::spacer_item());
            items.push(Self::divider_item());
            items.push(Self::spacer_item());
        }
        items.extend(global_items);
        items
    }

    fn item(label: impl Into<String>, action: ContextMenuAction, enabled: bool) -> ContextMenuItem {
        ContextMenuItem { label: label.into(), action, enabled }
    }

    fn divider_item() -> ContextMenuItem {
        Self::item("", ContextMenuAction::Divider, false)
    }

    fn spacer_item() -> ContextMenuItem {
        Self::item("", ContextMenuAction::Spacer, false)
    }

    fn command_item(label: impl Into<String>, command: Command) -> ContextMenuItem {
        Self::item(label, ContextMenuAction::Command(command), true)
    }

    fn graph_command_item(label: impl Into<String>, command: Command, force_graph_focus: bool) -> ContextMenuItem {
        let action = if force_graph_focus { ContextMenuAction::GraphCommand(command) } else { ContextMenuAction::Command(command) };
        Self::item(label, action, true)
    }

    fn global_context_menu_items(&self) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();
        if self.viewport == Viewport::Settings {
            items.push(Self::command_item("Back", Command::Back));
        } else {
            items.push(Self::item("Settings", ContextMenuAction::Settings, self.repo.is_some()));
        }
        if self.viewport == Viewport::Splash {
            if self.repo.is_some() {
                items.push(Self::command_item("Back", Command::Back));
            }
        } else {
            items.push(Self::item("Splash screen", ContextMenuAction::Splash, true));
        }
        items.push(Self::item("Exit", ContextMenuAction::Exit, true));
        items
    }

    fn graph_context_menu_items(&self, index: usize, force_graph_focus: bool, include_details: bool) -> Vec<ContextMenuItem> {
        if index == 0 {
            return self.uncommitted_graph_context_menu_items();
        }

        let mut items = Vec::new();
        if include_details {
            items.push(Self::graph_command_item("Show details", Command::NarrowScope, force_graph_focus));
        }
        if !self.graph_open_worktree_indices().is_empty() {
            items.push(Self::graph_command_item("Open worktree", Command::Select, force_graph_focus));
        }

        items.extend([
            Self::graph_command_item("Create branch", Command::CreateBranch, force_graph_focus),
            Self::graph_command_item("Create worktree", Command::CreateWorktree, force_graph_focus),
            Self::graph_command_item("Create tag", Command::Tag, force_graph_focus),
            Self::graph_command_item("Checkout", Command::Checkout, force_graph_focus),
            Self::graph_command_item("Hard reset", Command::HardReset, force_graph_focus),
            Self::graph_command_item("Mixed reset", Command::MixedReset, force_graph_focus),
            Self::graph_command_item("Cherry-pick", Command::Cherrypick, force_graph_focus),
            Self::graph_command_item("Revert", Command::Revert, force_graph_focus),
            Self::graph_command_item("Rebase", Command::Rebase, force_graph_focus),
            Self::graph_command_item("Merge", Command::Merge, force_graph_focus),
        ]);

        if let Some(alias) = self.graph_alias_at(index) {
            let branches = self.graph_branch_choices(alias);
            if !branches.is_empty() {
                items.push(Self::graph_command_item("Solo branch", Command::SoloBranch, force_graph_focus));
                items.push(Self::graph_command_item("Toggle branch", Command::ToggleBranch, force_graph_focus));
            }
            if !self.graph_local_branch_choices(alias).is_empty() {
                items.push(Self::graph_command_item("Rename branch", Command::RenameBranch, force_graph_focus));
            }
            if let Some(repo) = self.repo.as_ref() {
                let current = get_current_branch(repo);
                if !self.graph_deletable_branch_choices(alias, current.as_deref()).is_empty() {
                    items.push(Self::graph_command_item("Delete branch", Command::DeleteBranch, force_graph_focus));
                }
            }
        }

        if !self.graph_tag_names_at(index).is_empty() {
            items.push(Self::graph_command_item("Delete tag", Command::Untag, force_graph_focus));
        }

        if self.graph_row_at(index).is_some_and(|row| row.is_stash) {
            items.push(Self::graph_command_item("Pop stash", Command::Pop, force_graph_focus));
            items.push(Self::graph_command_item("Drop stash", Command::Drop, force_graph_focus));
        }

        items
    }

    fn uncommitted_graph_context_menu_items(&self) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();
        if self.uncommitted.is_unstaged {
            items.push(Self::command_item("Stage all", Command::Stage));
        }
        if self.uncommitted.is_staged {
            items.push(Self::command_item("Unstage all", Command::Unstage));
            items.push(Self::command_item("Commit", Command::Commit));
        }
        if !self.uncommitted.is_clean {
            items.push(Self::command_item("Stash changes", Command::Stash));
        }
        if self.repo.as_ref().is_some_and(|repo| Self::active_operation_kind(repo).is_some()) {
            items.push(Self::command_item("Continue operation", Command::ContinueOperation));
            items.push(Self::command_item("Abort operation", Command::AbortOperation));
        }
        items.push(Self::command_item("Find", Command::Find));
        if self.repo.is_some() {
            items.push(Self::command_item("Find file", Command::FindFile));
        }
        items
    }

    fn graph_tag_names_at(&self, index: usize) -> Vec<String> {
        self.graph_row_at(index)
            .map(|row| row.tags.iter().map(|tag| tag.name.clone()).collect())
            .or_else(|| self.graph_alias_at(index).map(|alias| self.tags.local.get(&alias).cloned().unwrap_or_default()))
            .unwrap_or_default()
    }

    fn viewer_context_menu_items(&self) -> Vec<ContextMenuItem> {
        let hunk_label = match self.viewer_mode {
            ViewerMode::Hunks => "Show full diff",
            ViewerMode::Full | ViewerMode::Split => "Show hunk rows",
        };
        let split_label = if self.viewer_mode == ViewerMode::Split { "Show unified diff" } else { "Show split diff" };
        let mut items =
            vec![Self::command_item(hunk_label, Command::ToggleHunkMode), Self::command_item(split_label, Command::ToggleSplitDiffMode), Self::command_item("Back to graph", Command::Back)];
        if self.repo.is_some() {
            items.push(Self::command_item("Find file", Command::FindFile));
        }
        items
    }

    fn branch_context_menu_items(&self) -> Vec<ContextMenuItem> {
        let mut items = vec![
            Self::command_item("Open commit", Command::Select),
            Self::command_item("Checkout branch", Command::Checkout),
            Self::command_item("Solo branch", Command::SoloBranch),
            Self::command_item("Toggle branch", Command::ToggleBranch),
        ];

        if self.branch_name_at_pane_selection().is_some_and(|branch| self.is_local_branch_name(&branch)) {
            items.push(Self::command_item("Rename branch", Command::RenameBranch));
        }
        items.push(Self::command_item("Delete branch", Command::DeleteBranch));
        items
    }

    fn tag_context_menu_items(&self) -> Vec<ContextMenuItem> {
        vec![Self::command_item("Open commit", Command::Select), Self::command_item("Delete tag", Command::Untag)]
    }

    fn stash_context_menu_items(&self) -> Vec<ContextMenuItem> {
        vec![Self::command_item("Open stash commit", Command::Select), Self::command_item("Pop stash", Command::Pop), Self::command_item("Drop stash", Command::Drop)]
    }

    fn reflog_context_menu_items(&self) -> Vec<ContextMenuItem> {
        vec![Self::command_item("Open commit", Command::Select), Self::command_item("Create branch here", Command::CreateBranch)]
    }

    fn worktree_context_menu_items(&self, index: usize) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();
        let Some(entry) = self.worktrees.entries.get(index) else {
            return items;
        };
        if entry.is_valid {
            items.push(Self::command_item("Open worktree", Command::Select));
        }
        if entry.can_remove() {
            items.push(Self::command_item("Remove worktree", Command::RemoveWorktree));
        }
        if entry.can_lock() {
            let label = if entry.locked_reason.is_some() { "Unlock worktree" } else { "Lock worktree" };
            items.push(Self::command_item(label, Command::ToggleWorktreeLock));
        }
        items
    }

    fn submodule_context_menu_items(&self, index: usize) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();
        let Some(entry) = self.submodules.entries.get(index) else {
            return items;
        };
        if entry.can_open() {
            items.push(Self::command_item("Open submodule", Command::Select));
        }
        items.push(Self::command_item("Update/init submodule", Command::UpdateSubmodule));
        items.push(Self::command_item("Sync URL", Command::SyncSubmodule));
        if entry.is_dirty() {
            items.push(Self::command_item("Stage submodule", Command::Stage));
        }
        if entry.is_index_modified {
            items.push(Self::command_item("Unstage submodule", Command::Unstage));
        }
        if !self.submodule_stack.is_empty() {
            items.push(Self::command_item("Return to parent repository", Command::ReturnToParentRepository));
        }
        items
    }

    fn inspector_context_menu_items(&self) -> Vec<ContextMenuItem> {
        let mut items = vec![Self::command_item("Show files/status", Command::NarrowScope), Self::command_item("Back to graph", Command::WidenScope)];
        if self.graph_selected != 0 {
            items.extend(self.graph_context_menu_items(self.graph_selected, true, false));
        }
        items
    }

    fn status_context_menu_items(&self, is_top: bool) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();
        let has_file = if is_top { self.status_top_clickable_count_for_context() > 0 } else { self.status_bottom_clickable_count_for_context() > 0 };
        if !has_file {
            return items;
        }

        items.push(Self::command_item("Open file", Command::Select));
        if self.graph_selected == 0 {
            if is_top {
                if !self.selected_staged_status_file_is_conflict() {
                    items.push(Self::command_item("Unstage file", Command::Unstage));
                    items.push(Self::command_item("Discard file changes", Command::HardReset));
                }
            } else if !self.selected_unstaged_status_file_is_conflict() {
                items.push(Self::command_item("Stage file", Command::Stage));
                items.push(Self::command_item("Discard file changes", Command::HardReset));
            }
        }
        items
    }

    fn status_top_clickable_count_for_context(&self) -> usize {
        if self.graph_selected == 0 {
            if !self.is_uncommitted_loaded || !self.uncommitted.is_staged {
                return 0;
            }
            self.uncommitted.conflicts.len() + self.uncommitted.staged.modified.len() + self.uncommitted.staged.added.len() + self.uncommitted.staged.deleted.len()
        } else if self.selected_commit_diff_is_loaded() {
            self.current_diff.len()
        } else {
            0
        }
    }

    fn status_bottom_clickable_count_for_context(&self) -> usize {
        if self.graph_selected != 0 || !self.is_uncommitted_loaded || !self.uncommitted.is_unstaged {
            return 0;
        }
        self.uncommitted.conflicts.len() + self.uncommitted.unstaged.modified.len() + self.uncommitted.unstaged.added.len() + self.uncommitted.unstaged.deleted.len()
    }

    fn search_context_menu_items(&self) -> Vec<ContextMenuItem> {
        let mut items = vec![Self::command_item("Open commit", Command::Select)];
        if self.repo.is_some() {
            items.push(Self::command_item("Find file", Command::FindFile));
        }
        items
    }

    fn splash_context_menu_items(&self, index: usize) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();
        if index < self.recent.len() {
            items.push(Self::command_item("Open repository", Command::Select));
            items.push(Self::command_item("Remove", Command::RemoveRecentRepository));
            if index > 0 {
                items.push(Self::command_item("Move up", Command::MoveRecentRepositoryUp));
            }
            if index + 1 < self.recent.len() {
                items.push(Self::command_item("Move down", Command::MoveRecentRepositoryDown));
            }
        }
        items
    }

    fn settings_context_menu_items(&self, line: usize) -> Vec<ContextMenuItem> {
        let Some(kind) = self.settings_selections.iter().find(|selection| selection.line == line).map(|selection| selection.kind.clone()) else {
            return Vec::new();
        };

        match kind {
            SettingsSelectionKind::Info => Vec::new(),
            SettingsSelectionKind::RecentRepository(index) => self.settings_recent_context_menu_items(index),
            SettingsSelectionKind::RemoteAdd => vec![Self::command_item("Add remote", Command::Select)],
            SettingsSelectionKind::Remote(name) => {
                REMOTE_ACTIONS.iter().enumerate().map(|(index, action)| Self::item(remote_action_label(action), ContextMenuAction::RemoteAction { name: name.clone(), index }, true)).collect()
            },
            SettingsSelectionKind::Theme(_) => vec![Self::command_item("Apply theme", Command::Select)],
            SettingsSelectionKind::KeyBinding(_) => vec![Self::command_item("Rebind shortcut", Command::Select)],
            SettingsSelectionKind::LayoutCommand(command) => {
                let label = format!("Run {}", command_to_visual_string(&command));
                vec![Self::command_item(label, Command::Select)]
            },
        }
    }

    fn settings_recent_context_menu_items(&self, index: usize) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();
        if index < self.recent.len() {
            items.push(Self::item("Open repository", ContextMenuAction::OpenRecentRepository(index), true));
            items.push(Self::command_item("Remove", Command::RemoveRecentRepository));
            if index > 0 {
                items.push(Self::command_item("Move up", Command::MoveRecentRepositoryUp));
            }
            if index + 1 < self.recent.len() {
                items.push(Self::command_item("Move down", Command::MoveRecentRepositoryDown));
            }
        }
        items
    }

    fn context_menu_item_at(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.context_menu_area_for_input()?;
        if !rect_contains(area, column, row) {
            return None;
        }

        if column <= area.x || column >= area.x.saturating_add(area.width).saturating_sub(1) {
            return None;
        }

        if row <= area.y.saturating_add(1) || row >= area.y.saturating_add(area.height).saturating_sub(2) {
            return None;
        }

        let index = row.saturating_sub(area.y).saturating_sub(2) as usize;
        self.context_menu.as_ref().is_some_and(|menu| index < menu.items.len()).then_some(index)
    }

    fn context_menu_area_for_input(&self) -> Option<Rect> {
        self.context_menu_area_for_bounds(self.context_menu_bounds())
    }

    fn context_menu_bounds(&self) -> Rect {
        let mut bounds = self.layout.app;
        for rect in [self.layout.title_left, self.layout.title_right, self.layout.statusbar_left, self.layout.statusbar_right] {
            bounds = union_rect(bounds, rect);
        }
        bounds
    }

    fn first_enabled_context_menu_index(items: &[ContextMenuItem]) -> usize {
        items.iter().position(|item| item.enabled).unwrap_or(0)
    }

    fn move_context_menu_selection(&mut self, direction: Direction) {
        let Some(menu) = self.context_menu.as_ref() else {
            return;
        };

        let count = menu.items.len();
        if count == 0 {
            return;
        }
        let selected = menu.selected;

        for offset in 1..=count {
            let index = match direction {
                Direction::Down => (selected + offset) % count,
                Direction::Up => (selected + count - (offset % count)) % count,
            };
            if menu.items[index].enabled {
                if let Some(menu) = &mut self.context_menu {
                    menu.selected = index;
                }
                return;
            }
        }
    }

    fn activate_context_menu_selected(&mut self) {
        let Some(item) = self.context_menu.as_ref().and_then(|menu| menu.items.get(menu.selected)).cloned() else {
            self.close_context_menu();
            return;
        };
        if item.enabled {
            self.activate_context_menu_action(item.action);
        }
    }

    fn activate_context_menu_action(&mut self, action: ContextMenuAction) {
        self.close_context_menu();
        match action {
            ContextMenuAction::Command(command) => self.dispatch_command(&command),
            ContextMenuAction::GraphCommand(command) => {
                self.viewport = Viewport::Graph;
                self.focus = Focus::Viewport;
                self.dispatch_command(&command);
            },
            ContextMenuAction::OpenRecentRepository(index) => self.open_recent_repository_from_context_menu(index),
            ContextMenuAction::RemoteAction { name, index } => self.activate_remote_context_menu_action(name, index),
            ContextMenuAction::SwitchSettingsTab(tab) => self.switch_settings_tab(tab),
            ContextMenuAction::Settings => self.open_settings_from_context_menu(),
            ContextMenuAction::Splash => self.open_splash_from_context_menu(),
            ContextMenuAction::Exit => self.exit(),
            ContextMenuAction::Divider | ContextMenuAction::Spacer => {},
        }
    }

    fn activate_remote_context_menu_action(&mut self, name: String, index: usize) {
        self.modal_remote_selected = index as i32;
        self.modal_remote_target = Some(name);
        self.modal_input.clear();
        self.focus = Focus::ModalRemoteAction;
        self.confirm_remote_action();
    }

    fn open_recent_repository_from_context_menu(&mut self, index: usize) {
        let Some(path) = self.recent.get(index).cloned() else {
            return;
        };
        self.submodule_stack.clear();
        self.reload(Some(path));
        self.graph_selected = 0;
        self.viewport = Viewport::Graph;
        self.focus = Focus::Viewport;
        self.last_input_direction = None;
    }

    fn open_settings_from_context_menu(&mut self) {
        if self.viewport != Viewport::Settings {
            self.settings_tab = SettingsTab::Paths;
            self.settings_selected = 0;
            self.settings_scroll.set(0);
        }
        self.viewport = Viewport::Settings;
        self.focus = Focus::Viewport;
        self.last_input_direction = None;
    }

    fn open_splash_from_context_menu(&mut self) {
        self.viewer_selected = 0;
        self.file_name = None;
        self.viewport = Viewport::Splash;
        self.focus = Focus::Viewport;
        self.last_input_direction = None;

        let selected = self.path.as_ref().and_then(|path| self.recent.iter().position(|recent| recent == path)).unwrap_or(0);
        self.splash_selected = selected.min(self.recent.len().saturating_sub(1));
    }
}

fn remote_action_label(action: &str) -> String {
    match action {
        "fetch" => "Fetch".to_string(),
        "set as default" => "Set as default".to_string(),
        "rename" => "Rename remote".to_string(),
        "edit fetch URL" => "Edit fetch URL".to_string(),
        "edit push URL" => "Edit push URL".to_string(),
        "delete" => "Delete remote".to_string(),
        other => other.to_string(),
    }
}

fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
    rect.width > 0 && rect.height > 0 && column >= rect.x && column < rect.x.saturating_add(rect.width) && row >= rect.y && row < rect.y.saturating_add(rect.height)
}

fn union_rect(first: Rect, second: Rect) -> Rect {
    if first.width == 0 || first.height == 0 {
        return second;
    }
    if second.width == 0 || second.height == 0 {
        return first;
    }

    let left = first.x.min(second.x);
    let top = first.y.min(second.y);
    let right = first.x.saturating_add(first.width).max(second.x.saturating_add(second.width));
    let bottom = first.y.saturating_add(first.height).max(second.y.saturating_add(second.height));

    Rect::new(left, top, right.saturating_sub(left), bottom.saturating_sub(top))
}
