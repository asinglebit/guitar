use crate::{
    app::{
        app::{App, BranchModalAction, Direction, Focus, PendingGraphLookup, SettingsSelectionKind, SettingsTab, Viewport},
        state::defaults::ViewerMode,
    },
    core::graph_service::{GraphBranchJumpDirection, GraphLookupKind, GraphPane, GraphPaneRow},
    git::{
        actions::{checkout::checkout_branch, tagging::untag},
        queries::{commits::get_current_branch, diffs::get_filenames_diff_at_oid},
    },
    helpers::{
        branch_visibility::{current_branch_names as git_current_branch_names, save_branch_visibility},
        keymap::{Command, InputMode},
        layout::LayoutConfig,
        localisation::errors,
        palette::Theme,
        symbols::SymbolTheme,
    },
};
use git2::BranchType;
use ratatui::layout::Rect;

#[derive(Clone, Copy)]
enum PaneFocusDirection {
    Left,
    Down,
    Up,
    Right,
}

impl App {
    fn last_index(len: usize) -> usize {
        len.saturating_sub(1)
    }

    fn clamp_selection(selection: usize, len: usize) -> usize {
        selection.min(Self::last_index(len))
    }

    fn search_result_count(&self) -> usize {
        self.search_rows.len()
    }

    fn wrap_modal_selection(selection: &mut i32, len: usize, direction: Direction) {
        if len == 0 {
            *selection = 0;
            return;
        }

        let len = len as i32;
        let current = (*selection).rem_euclid(len);
        *selection = match direction {
            Direction::Up => (current - 1).rem_euclid(len),
            Direction::Down => (current + 1).rem_euclid(len),
        };
    }

    fn clamp_splash_selection(&mut self) {
        self.splash_selected = Self::clamp_selection(self.splash_selected, self.recent.len());
    }

    fn selected_settings_recent_repository(&self) -> Option<(usize, usize)> {
        self.settings_selections.iter().find(|selection| selection.line == self.settings_selected).and_then(|selection| match &selection.kind {
            SettingsSelectionKind::RecentRepository(index) => Some((selection.line, *index)),
            _ => None,
        })
    }

    fn selected_recent_repository_index(&mut self) -> Option<usize> {
        if self.focus != Focus::Viewport {
            return None;
        }

        match self.viewport {
            Viewport::Splash => {
                if self.spinner.is_running() {
                    return None;
                }

                if self.recent.is_empty() {
                    self.splash_selected = 0;
                    return None;
                }

                self.clamp_splash_selection();
                Some(self.splash_selected)
            },
            Viewport::Settings => self.selected_settings_recent_repository().map(|(_, index)| index),
            _ => None,
        }
    }

    pub fn on_remove_recent_repository(&mut self) {
        let settings_selection = self.selected_settings_recent_repository();

        let Some(index) = self.selected_recent_repository_index() else {
            return;
        };

        if index >= self.recent.len() {
            return;
        }

        self.recent.remove(index);
        self.save_recent();

        match self.viewport {
            Viewport::Splash => self.clamp_splash_selection(),
            Viewport::Settings => {
                if let Some((line, _)) = settings_selection {
                    self.settings_selected = if self.recent.is_empty() || index < self.recent.len() { line } else { line.saturating_sub(1) };
                }
            },
            _ => {},
        }
    }

    fn move_recent_repository(&mut self, is_up: bool) {
        let settings_selection = self.selected_settings_recent_repository();

        let Some(index) = self.selected_recent_repository_index() else {
            return;
        };

        if index >= self.recent.len() {
            return;
        }

        let target = if is_up { index.checked_sub(1) } else { (index + 1 < self.recent.len()).then_some(index + 1) };
        let Some(target) = target else {
            return;
        };

        self.recent.swap(index, target);
        self.save_recent();

        match self.viewport {
            Viewport::Splash => {
                self.splash_selected = target;
            },
            Viewport::Settings => {
                if let Some((line, _)) = settings_selection {
                    self.settings_selected = if is_up { line.saturating_sub(1) } else { line.saturating_add(1) };
                }
            },
            _ => {},
        }
    }

    pub fn on_move_recent_repository_up(&mut self) {
        self.move_recent_repository(true);
    }

    pub fn on_move_recent_repository_down(&mut self) {
        self.move_recent_repository(false);
    }

    fn refresh_current_diff_for_graph_selection(&mut self) {
        self.current_diff.clear();
        self.current_diff_identity = None;
        if self.graph_selected == 0 || self.graph_selected >= self.graph_commit_count() {
            return;
        }

        if let Some(repo) = self.repo.clone() {
            let Some(identity) = self.graph_identity_at(self.graph_selected) else {
                return;
            };
            self.current_diff = get_filenames_diff_at_oid(&repo, identity.oid);
            self.current_diff_identity = Some(identity);
        }
    }

    pub(crate) fn select_graph_index(&mut self, idx: usize) {
        self.graph.pending_selection_restore = None;
        self.graph_selected = Self::clamp_selection(idx, self.graph_commit_count());
        self.refresh_current_diff_for_graph_selection();
    }

    fn center_graph_scroll_on_selection(&self) {
        let total = self.graph_commit_count();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };

        if total == 0 || visible_height == 0 {
            self.graph_scroll.set(0);
            return;
        }

        let max_scroll = total.saturating_sub(visible_height);
        let centered = self.graph_selected.saturating_sub(visible_height / 2);
        self.graph_scroll.set(centered.min(max_scroll));
    }

    fn select_graph_alias(&mut self, alias: u32) -> bool {
        if let Some(window) = &self.graph.graph_window
            && let Some(idx) = window.rows.iter().find(|row| row.alias == alias).map(|row| row.index)
        {
            self.select_graph_index(idx);
            return true;
        }

        let Some(idx) = self.oids.get_sorted_aliases().iter().position(|&current| current == alias) else {
            return false;
        };
        self.select_graph_index(idx);
        true
    }

    fn open_graph_at_alias(&mut self, alias: u32) -> bool {
        if !self.select_graph_alias(alias) {
            return false;
        }
        self.viewport = Viewport::Graph;
        self.focus = Focus::Viewport;
        self.center_graph_scroll_on_selection();
        true
    }

    pub(crate) fn open_graph_pane_row(&mut self, row: GraphPaneRow) -> bool {
        let graph_index = match row {
            GraphPaneRow::Branch { graph_index, .. } | GraphPaneRow::Tag { graph_index, .. } | GraphPaneRow::Stash { graph_index, .. } => graph_index,
            GraphPaneRow::Reflog { graph_index, .. } => {
                if graph_index.is_none() {
                    self.show_error(errors::REFLOG_COMMIT_HIDDEN());
                }
                graph_index
            },
        };

        let Some(graph_index) = graph_index else {
            return false;
        };

        self.select_graph_index(graph_index);
        self.center_graph_scroll_on_selection();
        self.viewport = Viewport::Graph;
        self.focus = Focus::Viewport;
        true
    }

    fn open_search_result(&mut self) -> bool {
        let Some(row) = self.search_rows.get(self.search_selected).cloned() else {
            return false;
        };

        self.select_graph_index(row.graph_index);
        self.center_graph_scroll_on_selection();
        self.viewport = Viewport::Graph;
        self.focus = Focus::Viewport;

        if self.graph_tx.is_some() && self.graph_row_at(row.graph_index).is_none() {
            self.request_graph_row_lookup(row.graph_index, PendingGraphLookup::CacheGraphRow);
        }

        true
    }

    fn cached_graph_pane_row(&self, pane: GraphPane) -> Option<GraphPaneRow> {
        let (selection, window) = match pane {
            GraphPane::Branches => (self.branches_selected, self.graph.branches_window.as_ref()),
            GraphPane::Tags => (self.tags_selected, self.graph.tags_window.as_ref()),
            GraphPane::Stashes => (self.stashes_selected, self.graph.stashes_window.as_ref()),
            GraphPane::Reflogs => (self.reflogs_selected, self.graph.reflogs_window.as_ref()),
        };

        let window = window?;
        if selection < window.start || selection >= window.end {
            return None;
        }
        window.rows.get(selection - window.start).cloned()
    }

    fn selected_graph_pane_index(&self, pane: GraphPane) -> usize {
        match pane {
            GraphPane::Branches => self.branches_selected,
            GraphPane::Tags => self.tags_selected,
            GraphPane::Stashes => self.stashes_selected,
            GraphPane::Reflogs => self.reflogs_selected,
        }
    }

    fn request_selected_graph_pane_row(&mut self, pane: GraphPane) {
        let index = self.selected_graph_pane_index(pane);
        self.request_graph_lookup(GraphLookupKind::PaneRowAt { pane, index }, PendingGraphLookup::SelectPaneRow);
    }

    fn open_selected_graph_pane_row(&mut self, pane: GraphPane) {
        if self.repo.is_none() {
            return;
        }

        if let Some(row) = self.cached_graph_pane_row(pane) {
            self.open_graph_pane_row(row);
            return;
        }

        if self.graph_tx.is_some() {
            self.request_selected_graph_pane_row(pane);
            return;
        }

        let alias = match pane {
            GraphPane::Branches => self.branch_alias_at_pane_selection(),
            GraphPane::Tags => self.tag_alias_at_pane_selection(),
            GraphPane::Stashes => self.stash_alias_at_pane_selection(),
            GraphPane::Reflogs => self.reflog_alias_at_pane_selection(),
        };

        if let Some(alias) = alias
            && !self.open_graph_at_alias(alias)
            && pane == GraphPane::Reflogs
        {
            self.show_error(errors::REFLOG_COMMIT_HIDDEN());
        }
    }

    fn branch_alias_at_pane_selection(&self) -> Option<u32> {
        if let Some(window) = &self.graph.branches_window
            && self.branches_selected >= window.start
            && self.branches_selected < window.end
            && let Some(GraphPaneRow::Branch { alias, .. }) = window.rows.get(self.branches_selected - window.start)
        {
            return Some(*alias);
        }
        self.branches.sorted.get(self.branches_selected).map(|(alias, _)| *alias)
    }

    fn tag_alias_at_pane_selection(&self) -> Option<u32> {
        if let Some(window) = &self.graph.tags_window
            && self.tags_selected >= window.start
            && self.tags_selected < window.end
            && let Some(GraphPaneRow::Tag { alias, .. }) = window.rows.get(self.tags_selected - window.start)
        {
            return Some(*alias);
        }
        self.tags.sorted.get(self.tags_selected).map(|(alias, _)| *alias)
    }

    pub(crate) fn stash_alias_at_pane_selection(&self) -> Option<u32> {
        if let Some(window) = &self.graph.stashes_window
            && self.stashes_selected >= window.start
            && self.stashes_selected < window.end
            && let Some(GraphPaneRow::Stash { alias, .. }) = window.rows.get(self.stashes_selected - window.start)
        {
            return Some(*alias);
        }
        self.oids.stashes.get(self.stashes_selected).copied()
    }

    fn reflog_alias_at_pane_selection(&self) -> Option<u32> {
        if let Some(window) = &self.graph.reflogs_window
            && self.reflogs_selected >= window.start
            && self.reflogs_selected < window.end
            && let Some(GraphPaneRow::Reflog { alias, .. }) = window.rows.get(self.reflogs_selected - window.start)
        {
            return Some(*alias);
        }
        self.reflogs.entries.get(self.reflogs_selected).map(|entry| entry.new_alias)
    }

    fn graph_visible_branch_indices(&self) -> Vec<usize> {
        if let Some(window) = &self.graph.branches_window
            && window.start == 0
            && window.end >= window.total
        {
            let mut visible_indices: Vec<usize> = window
                .rows
                .iter()
                .filter_map(|row| match row {
                    GraphPaneRow::Branch { name, graph_index, .. } if !self.branches.hidden_branch_names.contains(name) => *graph_index,
                    _ => None,
                })
                .collect();
            visible_indices.sort_unstable();
            if !visible_indices.is_empty() {
                return visible_indices;
            }
        }

        let mut visible_indices: Vec<usize> = self
            .branches
            .all
            .iter()
            .filter_map(|(&alias, all_branches)| {
                let has_visible_branch = all_branches.iter().any(|branch| !self.branches.hidden_branch_names.contains(branch));
                has_visible_branch.then(|| self.oids.get_sorted_aliases().iter().position(|&current| current == alias)).flatten()
            })
            .collect();
        visible_indices.sort_unstable();
        visible_indices
    }

    fn branch_count(&self) -> usize {
        self.graph.branches_window.as_ref().map(|window| window.total).unwrap_or(self.branches.sorted.len())
    }

    pub(crate) fn graph_deletable_branch_choices(&self, alias: u32, current: Option<&str>) -> Vec<String> {
        self.graph_branch_choices(alias).into_iter().filter(|branch| current != Some(branch.as_str())).collect()
    }

    fn get_focusable_panes(&self) -> Vec<Focus> {
        let mut order = Vec::new();
        if self.viewport == Viewport::Settings || self.viewport == Viewport::Splash {
            return order;
        }
        for focus in &[
            Focus::Viewport,
            Focus::Inspector,
            Focus::StatusTop,
            Focus::StatusBottom,
            Focus::Search,
            Focus::Submodules,
            Focus::Worktrees,
            Focus::Reflogs,
            Focus::Stashes,
            Focus::Tags,
            Focus::Branches,
        ] {
            match focus {
                Focus::Viewport => order.push(Focus::Viewport),
                Focus::Inspector if self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) => order.push(Focus::Inspector),
                Focus::StatusTop if self.layout_config.is_status => order.push(*focus),
                Focus::StatusBottom if self.layout_config.is_status && self.graph_selected == 0 => order.push(*focus),
                Focus::Search if self.layout_config.is_search => order.push(Focus::Search),
                Focus::Branches if self.layout_config.is_branches => order.push(Focus::Branches),
                Focus::Tags if self.layout_config.is_tags => order.push(Focus::Tags),
                Focus::Stashes if self.layout_config.is_stashes => order.push(Focus::Stashes),
                Focus::Reflogs if self.layout_config.is_reflogs => order.push(Focus::Reflogs),
                Focus::Worktrees if self.layout_config.is_worktrees => order.push(Focus::Worktrees),
                Focus::Submodules if self.layout_config.is_submodules => order.push(Focus::Submodules),
                _ => {},
            }
        }
        order
    }

    pub fn on_action_mode(&mut self) {
        self.mode = InputMode::Action;
    }

    fn begin_key_capture(&mut self, selection: crate::helpers::keymap::KeymapSelection) {
        self.modal_key_capture_selection = Some(selection);
        self.modal_key_capture_candidate = None;
        self.modal_key_capture_error = None;
        self.focus = Focus::ModalKeyCapture;
    }

    fn begin_graph_lane_limit_input(&mut self) {
        self.modal_input.set_value(self.layout_config.graph_lane_limit.to_string());
        self.focus = Focus::ModalGraphLaneLimit;
    }

    fn activate_settings_layout_command(&mut self, command: Command) {
        let selected = self.settings_selected;
        let scroll = self.settings_scroll.get();

        match command {
            Command::ResetLayout => {
                let config = LayoutConfig::default();
                let should_reload = self.repo.is_some() && self.layout_config.is_graph_reflogs != config.is_graph_reflogs;
                self.layout_config = config;
                self.mouse_drag = None;
                self.mark_viewer_layout_dirty();
                self.file_name = None;
                self.save_layout();
                if should_reload {
                    self.reload(None);
                }
            },
            Command::ToggleBranches => {
                self.layout_config.is_branches = !self.layout_config.is_branches;
                self.mark_viewer_layout_dirty();
                self.save_layout();
            },
            Command::ToggleTags => {
                self.layout_config.is_tags = !self.layout_config.is_tags;
                self.mark_viewer_layout_dirty();
                self.save_layout();
            },
            Command::ToggleStashes => {
                self.layout_config.is_stashes = !self.layout_config.is_stashes;
                self.mark_viewer_layout_dirty();
                self.save_layout();
            },
            Command::ToggleStatus => {
                self.layout_config.is_status = !self.layout_config.is_status;
                self.mark_viewer_layout_dirty();
                self.save_layout();
            },
            Command::ToggleInspector => {
                self.layout_config.is_inspector = !self.layout_config.is_inspector;
                self.mark_viewer_layout_dirty();
                self.save_layout();
            },
            Command::ToggleWorktrees => {
                self.layout_config.is_worktrees = !self.layout_config.is_worktrees;
                self.mark_viewer_layout_dirty();
                self.save_layout();
            },
            Command::ToggleSubmodules => {
                self.layout_config.is_submodules = !self.layout_config.is_submodules;
                self.mark_viewer_layout_dirty();
                self.save_layout();
            },
            Command::ToggleSearch => {
                self.layout_config.is_search = !self.layout_config.is_search;
                self.mark_viewer_layout_dirty();
                self.save_layout();
            },
            Command::ToggleReflogs => {
                self.layout_config.is_reflogs = !self.layout_config.is_reflogs;
                self.mark_viewer_layout_dirty();
                self.save_layout();
            },
            Command::ToggleShas => {
                self.layout_config.is_shas = !self.layout_config.is_shas;
                self.save_layout();
            },
            Command::ToggleGraphReflogs => {
                self.layout_config.is_graph_reflogs = !self.layout_config.is_graph_reflogs;
                self.mark_viewer_layout_dirty();
                self.save_layout();
                if self.repo.is_some() {
                    self.reload(None);
                }
            },
            Command::ToggleGraphDates => {
                self.layout_config.is_graph_dates = !self.layout_config.is_graph_dates;
                self.save_layout();
            },
            Command::ToggleGraphCommitters => {
                self.layout_config.is_graph_committers = !self.layout_config.is_graph_committers;
                self.save_layout();
            },
            Command::ToggleGraphRefs => {
                self.layout_config.is_graph_refs = !self.layout_config.is_graph_refs;
                self.save_layout();
            },
            _ => {},
        }

        self.viewport = Viewport::Settings;
        self.focus = Focus::Viewport;
        self.settings_selected = selected;
        self.settings_scroll.set(scroll);
    }

    pub fn on_select(&mut self) {
        match self.focus {
            Focus::Viewport => match self.viewport {
                Viewport::Settings => {
                    let selected = self.settings_selections.iter().find(|selection| selection.line == self.settings_selected).map(|selection| selection.kind.clone());
                    match selected {
                        Some(SettingsSelectionKind::Theme(theme_idx)) => {
                            let Some(preset) = Theme::presets().get(theme_idx) else {
                                return;
                            };
                            self.set_theme(preset.theme);
                            self.save_theme_config();
                            self.reload(None);
                        },
                        Some(SettingsSelectionKind::SymbolTheme(symbol_idx)) => {
                            let Some(preset) = SymbolTheme::presets().get(symbol_idx) else {
                                return;
                            };
                            let selected_line = self.settings_selected;
                            let scroll = self.settings_scroll.get();
                            self.set_symbol_theme(preset.theme());
                            self.save_symbol_theme_config();
                            if self.repo.is_some() {
                                self.reload(None);
                            } else {
                                self.mark_viewer_layout_dirty();
                            }
                            self.viewport = Viewport::Settings;
                            self.focus = Focus::Viewport;
                            self.settings_selected = selected_line;
                            self.settings_scroll.set(scroll);
                        },
                        Some(SettingsSelectionKind::Language(language_idx)) => {
                            let Some(language) = crate::helpers::localisation::Language::ALL.get(language_idx).copied() else {
                                return;
                            };
                            let selected_line = self.settings_selected;
                            let scroll = self.settings_scroll.get();
                            self.set_language(language);
                            self.save_language_config();
                            self.viewport = Viewport::Settings;
                            self.focus = Focus::Viewport;
                            self.settings_selected = selected_line;
                            self.settings_scroll.set(scroll);
                        },
                        Some(SettingsSelectionKind::KeyBinding(selection)) => {
                            self.begin_key_capture(selection);
                        },
                        Some(SettingsSelectionKind::LayoutCommand(command)) => {
                            self.activate_settings_layout_command(command);
                        },
                        Some(SettingsSelectionKind::GraphLaneLimit) => {
                            self.begin_graph_lane_limit_input();
                        },
                        Some(SettingsSelectionKind::RemoteAdd) => {
                            self.begin_add_remote();
                        },
                        Some(SettingsSelectionKind::Remote(remote_name)) => {
                            self.begin_remote_action(remote_name);
                        },
                        _ => {},
                    }
                },
                Viewport::Splash => {
                    if let Some(path) = self.recent.get(self.splash_selected).cloned() {
                        self.submodule_stack.clear();
                        self.reload(Some(path));
                        self.graph_selected = 0;
                    }
                },
                Viewport::Graph => {
                    self.open_graph_worktree();
                },
                _ => {},
            },
            Focus::Branches => {
                self.open_selected_graph_pane_row(GraphPane::Branches);
            },
            Focus::Tags => {
                self.open_selected_graph_pane_row(GraphPane::Tags);
            },
            Focus::Stashes => {
                self.open_selected_graph_pane_row(GraphPane::Stashes);
            },
            Focus::Reflogs => {
                self.open_selected_graph_pane_row(GraphPane::Reflogs);
            },
            Focus::Worktrees => {
                self.open_selected_worktree();
            },
            Focus::Submodules => {
                self.open_selected_submodule();
            },
            Focus::Search => {
                self.open_search_result();
            },
            Focus::ModalWorktreeChooser => {
                self.confirm_worktree_chooser();
            },
            Focus::ModalRemoteAction => {
                self.confirm_remote_action();
            },
            Focus::ModalRemoteDelete => {
                self.confirm_delete_remote();
            },
            Focus::StatusTop | Focus::StatusBottom => {
                if let Some(repo) = &self.repo.clone() {
                    self.open_viewer(repo);
                    self.focus = Focus::Viewport;
                }
            },
            Focus::ModalCheckout => {
                if let Some(repo) = &self.repo {
                    let Some(alias) = self.graph_alias_at(self.graph_selected) else {
                        return;
                    };
                    let branch_names = self.graph_branch_choices(alias);

                    if let Some(branch_name) = branch_names.get(self.modal_checkout_selected as usize) {
                        match checkout_branch(repo, &mut self.branches.hidden_branch_names, &mut self.branches.local, alias, branch_name) {
                            Ok(_) => {
                                self.save_hidden_branch_names();
                                self.modal_checkout_selected = 0;
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(errors::with_error(errors::CHECKOUT(), error)),
                        }
                    }
                }
            },
            Focus::ModalSolo => {
                let Some(alias) = self.graph_alias_at(self.graph_selected) else {
                    return;
                };
                let branch_names = self.modal_branch_action_choices(alias);
                let mut should_reload = false;

                if let Some(branch) = branch_names.get(self.modal_solo_selected as usize).cloned() {
                    match self.modal_branch_action {
                        BranchModalAction::Solo => {
                            self.solo_branch_name(&branch);
                            should_reload = true;
                        },
                        BranchModalAction::Toggle => {
                            self.toggle_branch_name(&branch);
                            should_reload = true;
                        },
                        BranchModalAction::Rename => {
                            self.modal_solo_selected = 0;
                            self.modal_branch_action = BranchModalAction::Solo;
                            self.open_branch_rename_modal(branch);
                            return;
                        },
                    }
                }

                self.modal_solo_selected = 0;
                self.modal_branch_action = BranchModalAction::Solo;
                self.focus = Focus::Viewport;
                if should_reload {
                    self.reload(None);
                }
            },

            Focus::ModalDeleteBranch => {
                if let Some(repo) = &self.repo {
                    let Some(alias) = self.graph_alias_at(self.graph_selected) else {
                        return;
                    };
                    let current = get_current_branch(repo);
                    let branch_names = self.graph_deletable_branch_choices(alias, current.as_deref());

                    if let Some(branch) = branch_names.get(self.modal_delete_branch_selected as usize) {
                        self.delete_branch_from_ui(branch);
                    }
                }
            },
            Focus::ModalDeleteTag => {
                if let Some(repo) = &self.repo {
                    let Some(alias) = self.graph_alias_at(self.graph_selected) else {
                        return;
                    };
                    let tags = self.tags.local.get(&alias).cloned().unwrap_or_default();
                    if let Some(tag) = tags.get(self.modal_delete_tag_selected as usize) {
                        match untag(repo, tag) {
                            Ok(_) => {
                                self.modal_delete_tag_selected = 0;
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(errors::with_error(errors::DELETE_TAG(), error)),
                        }
                    }
                }
            },
            _ => {},
        };
    }

    pub fn on_widen_scope(&mut self) {
        match self.focus {
            Focus::Viewport => match self.viewport {
                Viewport::Settings => {
                    self.viewport = Viewport::Graph;
                },
                Viewport::Viewer => {
                    self.layout_config.is_status = true;
                    self.file_name = None;
                    if self.graph_selected == 0 && self.uncommitted.is_unstaged {
                        self.focus = Focus::StatusBottom;
                    } else {
                        self.focus = Focus::StatusTop;
                    }
                },
                Viewport::Graph => {
                    self.layout_config.is_branches = true;
                    self.focus = Focus::Branches;
                },
                _ => {},
            },
            Focus::Inspector => {
                self.focus = Focus::Viewport;
                self.viewport = Viewport::Graph;
            },
            Focus::StatusTop => {
                if self.graph_selected != 0 || self.uncommitted.has_conflicts {
                    self.layout_config.is_inspector = true;
                    self.focus = Focus::Inspector;
                } else {
                    self.focus = Focus::Viewport;
                    self.viewport = Viewport::Graph;
                }
            },
            Focus::StatusBottom => {
                if self.uncommitted.is_staged {
                    self.focus = Focus::StatusTop;
                } else {
                    self.focus = Focus::Viewport;
                    self.viewport = Viewport::Graph;
                }
            },
            _ => {},
        }
        self.save_layout();
    }

    pub fn on_narrow_scope(&mut self) {
        match self.focus {
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    if self.graph_selected != 0 {
                        self.layout_config.is_inspector = true;
                        if self.layout_config.is_zen {
                            self.focus = Focus::Inspector;
                            if let Some(row) = self.graph_row_at(self.graph_selected).cloned() {
                                self.cache_graph_row(row);
                            } else {
                                self.request_graph_row_lookup(self.graph_selected, PendingGraphLookup::OpenInspector);
                            }
                        } else if self.graph_row_at(self.graph_selected).is_some() || self.graph_tx.is_none() {
                            self.focus = Focus::Inspector;
                        } else {
                            self.request_graph_row_lookup(self.graph_selected, PendingGraphLookup::OpenInspector);
                        }
                    } else {
                        if self.uncommitted.is_clean {
                            return;
                        }
                        self.layout_config.is_status = true;
                        if self.uncommitted.is_staged {
                            self.focus = Focus::StatusTop;
                        } else {
                            self.focus = Focus::StatusBottom;
                        }
                    }
                },
                Viewport::Splash => {
                    if let Some(path) = self.recent.get(self.splash_selected).cloned() {
                        self.submodule_stack.clear();
                        self.reload(Some(path));
                        self.graph_selected = 0;
                    }
                },
                _ => {},
            },
            Focus::Inspector => {
                self.layout_config.is_status = true;
                self.focus = Focus::StatusTop;
            },
            Focus::Branches => {
                self.open_selected_graph_pane_row(GraphPane::Branches);
            },
            Focus::Tags => {
                self.open_selected_graph_pane_row(GraphPane::Tags);
            },
            Focus::Stashes => {
                self.open_selected_graph_pane_row(GraphPane::Stashes);
            },
            Focus::Reflogs => {
                self.open_selected_graph_pane_row(GraphPane::Reflogs);
            },
            Focus::Worktrees => {
                self.open_selected_worktree();
            },
            Focus::Submodules => {
                self.open_selected_submodule();
            },
            Focus::Search => {
                self.open_search_result();
            },
            Focus::StatusTop | Focus::StatusBottom => {
                if let Some(repo) = &self.repo.clone() {
                    self.open_viewer(repo);
                    self.focus = Focus::Viewport;
                }
            },
            _ => {},
        }
        self.save_layout();
    }

    pub fn on_focus_next_pane(&mut self) {
        if self.viewport == Viewport::Settings && self.focus == Focus::Viewport {
            self.switch_settings_tab(self.settings_tab.next());
            return;
        }

        let active = self.get_focusable_panes();
        if active.is_empty() {
            return;
        }
        let idx = active.iter().position(|&f| f == self.focus).unwrap_or(0);
        self.focus = active[(idx + 1) % active.len()];
    }

    pub fn on_focus_prev_pane(&mut self) {
        if self.viewport == Viewport::Settings && self.focus == Focus::Viewport {
            self.switch_settings_tab(self.settings_tab.previous());
            return;
        }

        let active = self.get_focusable_panes();
        if active.is_empty() {
            return;
        }
        let idx = active.iter().position(|&f| f == self.focus).unwrap_or(0);
        self.focus = active[(idx + active.len() - 1) % active.len()];
    }

    pub fn on_focus_pane_left(&mut self) {
        self.focus_pane_in_direction(PaneFocusDirection::Left);
    }

    pub fn on_focus_pane_down(&mut self) {
        self.focus_pane_in_direction(PaneFocusDirection::Down);
    }

    pub fn on_focus_pane_up(&mut self) {
        self.focus_pane_in_direction(PaneFocusDirection::Up);
    }

    pub fn on_focus_pane_right(&mut self) {
        self.focus_pane_in_direction(PaneFocusDirection::Right);
    }

    fn focus_pane_in_direction(&mut self, direction: PaneFocusDirection) {
        let active = self.get_focusable_panes();
        let Some(current_order) = active.iter().position(|&focus| focus == self.focus) else {
            return;
        };

        let current_rect = self.focus_pane_rect(self.focus);
        if Self::is_empty_rect(current_rect) {
            return;
        }

        let target = active
            .iter()
            .enumerate()
            .filter_map(|(order, &focus)| {
                if order == current_order {
                    return None;
                }

                let rect = self.focus_pane_rect(focus);
                let (gap, perpendicular_distance) = Self::pane_focus_score(current_rect, rect, direction)?;
                Some((focus, gap, perpendicular_distance, order))
            })
            .min_by_key(|&(_, gap, perpendicular_distance, order)| (gap, perpendicular_distance, order))
            .map(|(focus, _, _, _)| focus);

        if let Some(focus) = target {
            self.focus = focus;
        }
    }

    fn focus_pane_rect(&self, focus: Focus) -> Rect {
        match focus {
            Focus::Viewport => self.layout.graph,
            Focus::Inspector => self.layout.pane_inspector,
            Focus::StatusTop => self.layout.pane_status_top,
            Focus::StatusBottom => self.layout.pane_status_bottom,
            Focus::Branches => self.layout.pane_branches,
            Focus::Tags => self.layout.pane_tags,
            Focus::Stashes => self.layout.pane_stashes,
            Focus::Reflogs => self.layout.pane_reflogs,
            Focus::Worktrees => self.layout.pane_worktrees,
            Focus::Submodules => self.layout.pane_submodules,
            Focus::Search => self.layout.pane_search,
            _ => Rect::default(),
        }
    }

    fn is_empty_rect(rect: Rect) -> bool {
        rect.width == 0 || rect.height == 0
    }

    fn rect_right(rect: Rect) -> u16 {
        rect.x.saturating_add(rect.width)
    }

    fn rect_bottom(rect: Rect) -> u16 {
        rect.y.saturating_add(rect.height)
    }

    fn rect_center_x2(rect: Rect) -> i32 {
        rect.x as i32 * 2 + rect.width as i32
    }

    fn rect_center_y2(rect: Rect) -> i32 {
        rect.y as i32 * 2 + rect.height as i32
    }

    fn rects_overlap_horizontally(first: Rect, second: Rect) -> bool {
        first.x < Self::rect_right(second) && second.x < Self::rect_right(first)
    }

    fn rects_overlap_vertically(first: Rect, second: Rect) -> bool {
        first.y < Self::rect_bottom(second) && second.y < Self::rect_bottom(first)
    }

    fn pane_focus_score(current: Rect, candidate: Rect, direction: PaneFocusDirection) -> Option<(u16, u32)> {
        if Self::is_empty_rect(candidate) {
            return None;
        }

        match direction {
            PaneFocusDirection::Left => {
                if Self::rect_right(candidate) > current.x || !Self::rects_overlap_vertically(current, candidate) {
                    return None;
                }
                Some((current.x.saturating_sub(Self::rect_right(candidate)), Self::rect_center_y2(current).abs_diff(Self::rect_center_y2(candidate))))
            },
            PaneFocusDirection::Right => {
                if Self::rect_right(current) > candidate.x || !Self::rects_overlap_vertically(current, candidate) {
                    return None;
                }
                Some((candidate.x.saturating_sub(Self::rect_right(current)), Self::rect_center_y2(current).abs_diff(Self::rect_center_y2(candidate))))
            },
            PaneFocusDirection::Up => {
                if Self::rect_bottom(candidate) > current.y || !Self::rects_overlap_horizontally(current, candidate) {
                    return None;
                }
                Some((current.y.saturating_sub(Self::rect_bottom(candidate)), Self::rect_center_x2(current).abs_diff(Self::rect_center_x2(candidate))))
            },
            PaneFocusDirection::Down => {
                if Self::rect_bottom(current) > candidate.y || !Self::rects_overlap_horizontally(current, candidate) {
                    return None;
                }
                Some((candidate.y.saturating_sub(Self::rect_bottom(current)), Self::rect_center_x2(current).abs_diff(Self::rect_center_x2(candidate))))
            },
        }
    }

    pub fn on_scroll_page_up(&mut self) {
        match self.focus {
            Focus::Branches => {
                let page = self.layout.branches.height as usize - 1;
                self.branches_selected = self.branches_selected.saturating_sub(page);
            },
            Focus::Tags => {
                let page = self.layout.tags.height as usize - 1;
                self.tags_selected = self.tags_selected.saturating_sub(page);
            },
            Focus::Stashes => {
                let page = self.layout.stashes.height as usize - 1;
                self.stashes_selected = self.stashes_selected.saturating_sub(page);
            },
            Focus::Reflogs => {
                let page = self.layout.reflogs.height as usize - 1;
                self.reflogs_selected = self.reflogs_selected.saturating_sub(page);
            },
            Focus::Worktrees => {
                let page = self.layout.worktrees.height as usize - 1;
                self.worktrees_selected = self.worktrees_selected.saturating_sub(page);
            },
            Focus::Submodules => {
                let page = self.layout.submodules.height as usize - 1;
                self.submodules_selected = self.submodules_selected.saturating_sub(page);
            },
            Focus::Search => {
                let page = self.layout.search.height as usize - 1;
                self.search_selected = self.search_selected.saturating_sub(page);
            },
            Focus::Viewport => {
                let page = self.layout.graph.height as usize - 1;
                match self.viewport {
                    Viewport::Graph => {
                        self.select_graph_index(self.graph_selected.saturating_sub(page));
                    },
                    Viewport::Viewer => {
                        self.viewer_selected = self.viewer_selected.saturating_sub(page);
                    },
                    Viewport::Settings => {
                        self.settings_selected = self.settings_selected.saturating_sub(page);
                        self.last_input_direction = Some(Direction::Up);
                    },
                    Viewport::Splash => {
                        self.splash_selected = self.splash_selected.saturating_sub(page);
                        self.last_input_direction = Some(Direction::Up);
                    },
                }
            },
            Focus::Inspector => {
                let page = self.layout.inspector.height as usize - 3;
                self.inspector_selected = self.inspector_selected.saturating_sub(page);
            },
            Focus::StatusTop => {
                let page = self.layout.status_top.height as usize - 3;
                self.status_top_selected = self.status_top_selected.saturating_sub(page);
            },
            Focus::StatusBottom => {
                let page = self.layout.status_bottom.height as usize - 3;
                self.status_bottom_selected = self.status_bottom_selected.saturating_sub(page);
            },
            _ => {},
        };
    }

    pub fn on_scroll_page_down(&mut self) {
        match self.focus {
            Focus::Branches => {
                let page = self.layout.branches.height as usize - 1;
                self.branches_selected += page;
            },
            Focus::Tags => {
                let page = self.layout.tags.height as usize - 1;
                self.tags_selected += page;
            },
            Focus::Stashes => {
                let page = self.layout.stashes.height as usize - 1;
                self.stashes_selected += page;
            },
            Focus::Reflogs => {
                let page = self.layout.reflogs.height as usize - 1;
                self.reflogs_selected += page;
            },
            Focus::Worktrees => {
                let page = self.layout.worktrees.height as usize - 1;
                self.worktrees_selected += page;
            },
            Focus::Submodules => {
                let page = self.layout.submodules.height as usize - 1;
                self.submodules_selected += page;
            },
            Focus::Search => {
                let page = self.layout.search.height as usize - 1;
                self.search_selected = Self::clamp_selection(self.search_selected.saturating_add(page), self.search_result_count());
            },
            Focus::Viewport => {
                let page = self.layout.graph.height as usize - 1;
                match self.viewport {
                    Viewport::Graph => {
                        self.select_graph_index(self.graph_selected.saturating_add(page));
                    },
                    Viewport::Viewer => {
                        let total = self.viewer_row_count();
                        if total == 0 {
                            self.viewer_selected = 0;
                        } else if self.viewer_selected + page < total {
                            self.viewer_selected += page;
                        } else {
                            self.viewer_selected = total - 1;
                        }
                    },
                    Viewport::Settings => {
                        self.settings_selected += page;
                        self.last_input_direction = Some(Direction::Down);
                    },
                    Viewport::Splash => {
                        self.splash_selected = self.splash_selected.saturating_add(page);
                        self.clamp_splash_selection();
                        self.last_input_direction = Some(Direction::Down);
                    },
                }
            },
            Focus::Inspector => {
                let page = self.layout.inspector.height as usize - 3;
                self.inspector_selected += page;
            },
            Focus::StatusTop => {
                let page = self.layout.status_top.height as usize - 3;
                self.status_top_selected += page;
            },
            Focus::StatusBottom => {
                let page = self.layout.status_bottom.height as usize - 3;
                self.status_bottom_selected += page;
            },
            _ => {},
        };
    }

    pub fn on_scroll_up(&mut self) {
        match self.focus {
            Focus::Branches => {
                self.branches_selected = self.branches_selected.saturating_sub(1);
            },
            Focus::Tags => {
                self.tags_selected = self.tags_selected.saturating_sub(1);
            },
            Focus::Stashes => {
                self.stashes_selected = self.stashes_selected.saturating_sub(1);
            },
            Focus::Reflogs => {
                self.reflogs_selected = self.reflogs_selected.saturating_sub(1);
            },
            Focus::Worktrees => {
                self.worktrees_selected = self.worktrees_selected.saturating_sub(1);
            },
            Focus::Submodules => {
                self.submodules_selected = self.submodules_selected.saturating_sub(1);
            },
            Focus::Search => {
                self.search_selected = self.search_selected.saturating_sub(1);
            },
            Focus::Viewport => {
                match self.viewport {
                    Viewport::Graph => {
                        if self.graph_selected > 0 {
                            self.select_graph_index(self.graph_selected - 1);
                            if self.graph_selected == 0 && self.focus == Focus::Inspector && !self.uncommitted.has_conflicts {
                                self.focus = Focus::Viewport;
                            }
                        }
                    },
                    Viewport::Viewer => {
                        if self.viewer_selected > 0 {
                            self.viewer_selected -= 1;
                        }
                    },
                    Viewport::Settings => {
                        self.settings_selected = self.settings_selected.saturating_sub(1);
                        self.last_input_direction = Some(Direction::Up);
                    },
                    Viewport::Splash => {
                        self.splash_selected = self.splash_selected.saturating_sub(1);
                        self.last_input_direction = Some(Direction::Up);
                    },
                }
                if self.viewport == Viewport::Graph {}
            },
            Focus::Inspector => {
                self.inspector_selected = self.inspector_selected.saturating_sub(1);
            },
            Focus::StatusTop => {
                self.status_top_selected = self.status_top_selected.saturating_sub(1);
            },
            Focus::StatusBottom => {
                self.status_bottom_selected = self.status_bottom_selected.saturating_sub(1);
            },
            Focus::ModalCheckout => {
                if let Some(alias) = self.graph_alias_at(self.graph_selected) {
                    let branch_names = self.graph_branch_choices(alias);
                    Self::wrap_modal_selection(&mut self.modal_checkout_selected, branch_names.len(), Direction::Up);
                }
            },
            Focus::ModalSolo => {
                if let Some(alias) = self.graph_alias_at(self.graph_selected) {
                    let branch_names = self.modal_branch_action_choices(alias);
                    Self::wrap_modal_selection(&mut self.modal_solo_selected, branch_names.len(), Direction::Up);
                }
            },
            Focus::ModalDeleteBranch => {
                if let Some(repo) = &self.repo {
                    if let Some(alias) = self.graph_alias_at(self.graph_selected) {
                        let current = get_current_branch(repo);
                        let branch_names = self.graph_deletable_branch_choices(alias, current.as_deref());
                        Self::wrap_modal_selection(&mut self.modal_delete_branch_selected, branch_names.len(), Direction::Up);
                    }
                }
            },
            Focus::ModalDeleteTag => {
                let tags: Vec<String> = self
                    .graph_row_at(self.graph_selected)
                    .map(|row| row.tags.iter().map(|tag| tag.name.clone()).collect())
                    .or_else(|| self.graph_alias_at(self.graph_selected).map(|alias| self.tags.local.get(&alias).cloned().unwrap_or_default()))
                    .unwrap_or_default();
                Self::wrap_modal_selection(&mut self.modal_delete_tag_selected, tags.len(), Direction::Up);
            },
            Focus::ModalWorktreeChooser => {
                Self::wrap_modal_selection(&mut self.modal_worktree_selected, self.modal_worktree_candidates.len(), Direction::Up);
            },
            Focus::ModalRemoteAction => {
                self.move_remote_action_selection(Direction::Up);
            },
            _ => {},
        }
    }

    pub fn on_scroll_down(&mut self) {
        match self.focus {
            Focus::Branches => {
                self.branches_selected += 1;
            },
            Focus::Tags => {
                self.tags_selected += 1;
            },
            Focus::Stashes => {
                self.stashes_selected += 1;
            },
            Focus::Reflogs => {
                self.reflogs_selected += 1;
            },
            Focus::Worktrees => {
                self.worktrees_selected += 1;
            },
            Focus::Submodules => {
                self.submodules_selected += 1;
            },
            Focus::Search => {
                self.search_selected = Self::clamp_selection(self.search_selected.saturating_add(1), self.search_result_count());
            },
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    self.select_graph_index(self.graph_selected.saturating_add(1));
                },
                Viewport::Viewer => {
                    if self.viewer_selected + 1 < self.viewer_row_count() {
                        self.viewer_selected += 1;
                    }
                },
                Viewport::Settings => {
                    self.settings_selected += 1;
                    self.last_input_direction = Some(Direction::Down);
                },
                Viewport::Splash => {
                    self.splash_selected = self.splash_selected.saturating_add(1);
                    self.clamp_splash_selection();
                    self.last_input_direction = Some(Direction::Down);
                },
            },
            Focus::Inspector => {
                self.inspector_selected += 1;
            },
            Focus::StatusTop => {
                self.status_top_selected += 1;
            },
            Focus::StatusBottom => {
                self.status_bottom_selected += 1;
            },
            Focus::ModalCheckout => {
                if let Some(alias) = self.graph_alias_at(self.graph_selected) {
                    let branch_names = self.graph_branch_choices(alias);
                    Self::wrap_modal_selection(&mut self.modal_checkout_selected, branch_names.len(), Direction::Down);
                }
            },
            Focus::ModalSolo => {
                if let Some(alias) = self.graph_alias_at(self.graph_selected) {
                    let branch_names = self.modal_branch_action_choices(alias);
                    Self::wrap_modal_selection(&mut self.modal_solo_selected, branch_names.len(), Direction::Down);
                }
            },
            Focus::ModalDeleteBranch => {
                if let Some(repo) = &self.repo {
                    if let Some(alias) = self.graph_alias_at(self.graph_selected) {
                        let current = get_current_branch(repo);
                        let branch_names = self.graph_deletable_branch_choices(alias, current.as_deref());
                        Self::wrap_modal_selection(&mut self.modal_delete_branch_selected, branch_names.len(), Direction::Down);
                    }
                }
            },
            Focus::ModalDeleteTag => {
                let tags: Vec<String> = self
                    .graph_row_at(self.graph_selected)
                    .map(|row| row.tags.iter().map(|tag| tag.name.clone()).collect())
                    .or_else(|| self.graph_alias_at(self.graph_selected).map(|alias| self.tags.local.get(&alias).cloned().unwrap_or_default()))
                    .unwrap_or_default();
                Self::wrap_modal_selection(&mut self.modal_delete_tag_selected, tags.len(), Direction::Down);
            },
            Focus::ModalWorktreeChooser => {
                Self::wrap_modal_selection(&mut self.modal_worktree_selected, self.modal_worktree_candidates.len(), Direction::Down);
            },
            Focus::ModalRemoteAction => {
                self.move_remote_action_selection(Direction::Down);
            },
            _ => {},
        }
    }

    pub fn on_scroll_up_half(&mut self) {
        match self.focus {
            Focus::Viewport => {
                if self.viewport == Viewport::Graph {
                    self.select_graph_index(self.graph_selected / 2);
                }
            },
            Focus::Branches => self.branches_selected /= 2,
            Focus::Tags => self.tags_selected /= 2,
            Focus::Stashes => self.stashes_selected /= 2,
            Focus::Reflogs => self.reflogs_selected /= 2,
            Focus::Worktrees => self.worktrees_selected /= 2,
            Focus::Submodules => self.submodules_selected /= 2,
            Focus::Search => self.search_selected /= 2,
            _ => {},
        };
    }

    pub fn on_scroll_down_half(&mut self) {
        match self.focus {
            Focus::Viewport => {
                if self.viewport == Viewport::Graph {
                    let total = self.graph_commit_count();
                    let next = self.graph_selected.saturating_add(total.saturating_sub(self.graph_selected) / 2);
                    self.select_graph_index(next);
                }
            },
            Focus::Branches => {
                let total = self.branch_count();
                self.branches_selected = self.branches_selected + total.saturating_sub(self.branches_selected) / 2
            },
            Focus::Tags => {
                let total = self.tags.sorted.len();
                self.tags_selected = self.tags_selected + (total - self.tags_selected) / 2
            },
            Focus::Stashes => {
                let total = self.oids.stashes.len();
                self.stashes_selected = self.stashes_selected + (total - self.stashes_selected) / 2
            },
            Focus::Reflogs => {
                let total = self.reflogs.entries.len();
                self.reflogs_selected = self.reflogs_selected + (total - self.reflogs_selected) / 2
            },
            Focus::Worktrees => {
                let total = self.worktrees.entries.len();
                self.worktrees_selected = self.worktrees_selected + (total - self.worktrees_selected) / 2
            },
            Focus::Submodules => {
                let total = self.submodules.entries.len();
                self.submodules_selected = self.submodules_selected + total.saturating_sub(self.submodules_selected) / 2
            },
            Focus::Search => {
                let total = self.search_result_count();
                self.search_selected = self.search_selected + (total.saturating_sub(self.search_selected)) / 2;
            },
            _ => {},
        };
    }

    pub fn on_scroll_half_page_up(&mut self) {
        match self.focus {
            Focus::Branches => {
                let half = (self.layout.branches.height as usize - 1) / 2;
                self.branches_selected = self.branches_selected.saturating_sub(half);
            },
            Focus::Tags => {
                let half = (self.layout.tags.height as usize - 1) / 2;
                self.tags_selected = self.tags_selected.saturating_sub(half);
            },
            Focus::Stashes => {
                let half = (self.layout.stashes.height as usize - 1) / 2;
                self.stashes_selected = self.stashes_selected.saturating_sub(half);
            },
            Focus::Reflogs => {
                let half = (self.layout.reflogs.height as usize - 1) / 2;
                self.reflogs_selected = self.reflogs_selected.saturating_sub(half);
            },
            Focus::Worktrees => {
                let half = (self.layout.worktrees.height as usize - 1) / 2;
                self.worktrees_selected = self.worktrees_selected.saturating_sub(half);
            },
            Focus::Submodules => {
                let half = (self.layout.submodules.height as usize - 1) / 2;
                self.submodules_selected = self.submodules_selected.saturating_sub(half);
            },
            Focus::Search => {
                let half = (self.layout.search.height as usize - 1) / 2;
                self.search_selected = self.search_selected.saturating_sub(half);
            },
            Focus::Viewport => {
                let half = (self.layout.graph.height as usize - 1) / 2;
                match self.viewport {
                    Viewport::Graph => {
                        self.select_graph_index(self.graph_selected.saturating_sub(half));
                    },
                    Viewport::Viewer => match self.viewer_mode {
                        ViewerMode::Full => {
                            if let Some(&prev) = self.viewer_edges.iter().rev().find(|&h| h < &self.viewer_selected) {
                                self.viewer_selected = prev;
                            }
                        },
                        ViewerMode::Hunks => {
                            self.viewer_selected = self.viewer_selected.saturating_sub(half);
                        },
                        ViewerMode::Split => {
                            self.viewer_selected = self.viewer_selected.saturating_sub(half);
                        },
                    },
                    Viewport::Settings => {
                        self.settings_selected = self.settings_selected.saturating_sub(half);
                        self.last_input_direction = Some(Direction::Up);
                    },
                    Viewport::Splash => {
                        self.splash_selected = self.splash_selected.saturating_sub(half);
                        self.last_input_direction = Some(Direction::Up);
                    },
                }
            },
            Focus::Inspector => {
                let half = (self.layout.inspector.height as usize - 3) / 2;
                self.inspector_selected = self.inspector_selected.saturating_sub(half);
            },
            Focus::StatusTop => {
                let half = (self.layout.status_top.height as usize - 3) / 2;
                self.status_top_selected = self.status_top_selected.saturating_sub(half);
            },
            Focus::StatusBottom => {
                let half = (self.layout.status_bottom.height as usize - 3) / 2;
                self.status_bottom_selected = self.status_bottom_selected.saturating_sub(half);
            },
            _ => {},
        }
    }

    pub fn on_scroll_half_page_down(&mut self) {
        match self.focus {
            Focus::Branches => {
                let half = (self.layout.branches.height.saturating_sub(1) as usize) / 2;
                self.branches_selected += half;
            },
            Focus::Tags => {
                let half = (self.layout.tags.height.saturating_sub(1) as usize) / 2;
                self.tags_selected += half;
            },
            Focus::Stashes => {
                let half = (self.layout.stashes.height.saturating_sub(1) as usize) / 2;
                self.stashes_selected += half;
            },
            Focus::Reflogs => {
                let half = (self.layout.reflogs.height.saturating_sub(1) as usize) / 2;
                self.reflogs_selected += half;
            },
            Focus::Worktrees => {
                let half = (self.layout.worktrees.height.saturating_sub(1) as usize) / 2;
                self.worktrees_selected += half;
            },
            Focus::Submodules => {
                let half = (self.layout.submodules.height.saturating_sub(1) as usize) / 2;
                self.submodules_selected += half;
            },
            Focus::Search => {
                let half = (self.layout.search.height.saturating_sub(1) as usize) / 2;
                self.search_selected = Self::clamp_selection(self.search_selected.saturating_add(half), self.search_result_count());
            },
            Focus::Viewport => {
                let half = (self.layout.graph.height.saturating_sub(1) as usize) / 2;
                match self.viewport {
                    Viewport::Graph => {
                        self.select_graph_index(self.graph_selected.saturating_add(half));
                    },
                    Viewport::Viewer => match self.viewer_mode {
                        ViewerMode::Full => {
                            if let Some(&next) = self.viewer_edges.iter().find(|&h| h > &self.viewer_selected) {
                                self.viewer_selected = next;
                            }
                        },
                        ViewerMode::Hunks => {
                            self.viewer_selected += half;
                        },
                        ViewerMode::Split => {
                            self.viewer_selected += half;
                        },
                    },
                    Viewport::Settings => {
                        self.settings_selected += half;
                        self.last_input_direction = Some(Direction::Down);
                    },
                    Viewport::Splash => {
                        self.splash_selected = self.splash_selected.saturating_add(half);
                        self.clamp_splash_selection();
                        self.last_input_direction = Some(Direction::Down);
                    },
                }
            },
            Focus::Inspector => {
                let half = (self.layout.inspector.height.saturating_sub(3) as usize) / 2;
                self.inspector_selected += half;
            },
            Focus::StatusTop => {
                let half = (self.layout.status_top.height.saturating_sub(3) as usize) / 2;
                self.status_top_selected += half;
            },
            Focus::StatusBottom => {
                let half = (self.layout.status_bottom.height.saturating_sub(3) as usize) / 2;
                self.status_bottom_selected += half;
            },
            _ => {},
        }
    }

    pub fn on_scroll_up_branch(&mut self) {
        if self.focus != Focus::Viewport || self.viewport != Viewport::Graph {
            return;
        }

        if self.graph_tx.is_some() {
            self.request_graph_lookup(GraphLookupKind::BranchIndex { from: self.graph_selected, direction: GraphBranchJumpDirection::Previous }, PendingGraphLookup::SelectIndex);
            return;
        }

        if let Some(&next) = self.graph_visible_branch_indices().iter().rev().find(|&&idx| idx < self.graph_selected) {
            self.select_graph_index(next);
        }
    }

    pub fn on_scroll_down_branch(&mut self) {
        if self.focus != Focus::Viewport || self.viewport != Viewport::Graph {
            return;
        }

        if self.graph_tx.is_some() {
            self.request_graph_lookup(GraphLookupKind::BranchIndex { from: self.graph_selected, direction: GraphBranchJumpDirection::Next }, PendingGraphLookup::SelectIndex);
            return;
        }

        if let Some(&next) = self.graph_visible_branch_indices().iter().find(|&&idx| idx > self.graph_selected) {
            self.select_graph_index(next);
        }
    }

    pub fn on_scroll_up_commit(&mut self) {
        if let Some(repo) = &self.repo
            && self.focus == Focus::Viewport
            && self.viewport == Viewport::Graph
        {
            if self.graph_tx.is_some() {
                self.request_graph_lookup(GraphLookupKind::ChildIndex { index: self.graph_selected }, PendingGraphLookup::SelectIndex);
                return;
            }

            let Some(oid) = self.graph_oid_at(self.graph_selected) else {
                return;
            };

            if oid == git2::Oid::zero() {
                return;
            }

            let mut child_positions: Vec<usize> = self
                .graph
                .graph_window
                .as_ref()
                .into_iter()
                .flat_map(|window| window.rows.iter())
                .filter_map(|row| {
                    let commit = repo.find_commit(row.oid).ok()?;
                    if commit.parent_ids().any(|parent_oid| parent_oid == oid) { Some(row.index) } else { None }
                })
                .collect();

            if child_positions.is_empty() {
                child_positions = self
                    .oids
                    .get_sorted_aliases()
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, &alias)| {
                        let child_oid = self.oids.get_oid_by_alias(alias);
                        let commit = repo.find_commit(*child_oid).ok()?;
                        if commit.parent_ids().any(|parent_oid| parent_oid == oid) { Some(idx) } else { None }
                    })
                    .collect();
            }

            if let Some(&next) = child_positions.first() {
                self.select_graph_index(next);
            }
        }
    }

    pub fn on_scroll_down_commit(&mut self) {
        if let Some(repo) = &self.repo
            && self.focus == Focus::Viewport
            && self.viewport == Viewport::Graph
        {
            if self.graph_tx.is_some() {
                self.request_graph_lookup(GraphLookupKind::ParentIndex { index: self.graph_selected }, PendingGraphLookup::SelectIndex);
                return;
            }

            let Some(oid) = self.graph_oid_at(self.graph_selected) else {
                return;
            };

            if oid == git2::Oid::zero() {
                self.select_graph_index(1);
                return;
            }

            let next = {
                let Ok(commit) = repo.find_commit(oid) else {
                    return;
                };
                let Some(parent_oid) = commit.parent_ids().next() else {
                    return;
                };
                self.graph.graph_window.as_ref().and_then(|window| window.rows.iter().find(|row| row.oid == parent_oid).map(|row| row.index)).or_else(|| {
                    let parent_alias = self.oids.aliases.get(&parent_oid).copied()?;
                    self.oids.get_sorted_aliases().iter().position(|&alias| alias == parent_alias)
                })
            };

            if let Some(next) = next {
                self.select_graph_index(next);
            }
        }
    }

    pub fn on_toggle_hunk_mode(&mut self) {
        // Preserve the closest logical diff location when switching viewer modes.
        match self.viewer_mode {
            ViewerMode::Full => {
                let full_idx = self.viewer_selected;
                let hunk_view_idx = self.viewer_hunks.iter().enumerate().min_by_key(|(_, h)| h.abs_diff(full_idx)).map(|(i, _)| i).unwrap_or(0);
                self.viewer_mode = ViewerMode::Hunks;
                self.viewer_selected = hunk_view_idx;
            },
            ViewerMode::Hunks => {
                let hunk_view_idx = self.viewer_selected;
                if let Some(&full_idx) = self.viewer_hunks.get(hunk_view_idx) {
                    self.viewer_mode = ViewerMode::Full;
                    self.viewer_selected = full_idx;
                } else {
                    self.viewer_mode = ViewerMode::Full;
                    self.viewer_selected = 0;
                }
            },
            ViewerMode::Split => {
                let full_idx = self.split_unified_index(self.viewer_selected);
                let hunk_view_idx = self.viewer_hunks.iter().enumerate().min_by_key(|(_, h)| h.abs_diff(full_idx)).map(|(i, _)| i).unwrap_or(0);
                self.viewer_mode = ViewerMode::Hunks;
                self.viewer_selected = hunk_view_idx;
            },
        }

        self.viewer_scroll.set(self.viewer_selected);
    }

    pub fn on_toggle_split_diff_mode(&mut self) {
        match self.viewer_mode {
            ViewerMode::Split => {
                let full_idx = self.split_unified_index(self.viewer_selected);
                self.viewer_mode = ViewerMode::Full;
                self.viewer_selected = full_idx.min(self.viewer_lines.len().saturating_sub(1));
            },
            ViewerMode::Full => {
                let full_idx = self.viewer_selected;
                self.viewer_mode = ViewerMode::Split;
                self.viewer_selected = self.closest_split_row_for_unified(full_idx);
            },
            ViewerMode::Hunks => {
                let full_idx = self.viewer_hunks.get(self.viewer_selected).copied().unwrap_or(0);
                self.viewer_mode = ViewerMode::Split;
                self.viewer_selected = self.closest_split_row_for_unified(full_idx);
            },
        }

        self.mark_viewer_layout_dirty();
        self.viewer_scroll.set(self.viewer_selected);
        self.save_layout();
    }

    pub fn on_scroll_to_beginning(&mut self) {
        match self.focus {
            Focus::Branches => {
                self.branches_selected = 0;
            },
            Focus::Tags => {
                self.tags_selected = 0;
            },
            Focus::Stashes => {
                self.stashes_selected = 0;
            },
            Focus::Reflogs => {
                self.reflogs_selected = 0;
            },
            Focus::Worktrees => {
                self.worktrees_selected = 0;
            },
            Focus::Submodules => {
                self.submodules_selected = 0;
            },
            Focus::Search => {
                self.search_selected = 0;
            },
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    self.select_graph_index(0);
                },
                Viewport::Viewer => {
                    self.viewer_selected = 0;
                },
                Viewport::Settings => {
                    self.settings_selected = 0;
                },
                Viewport::Splash => {
                    self.splash_selected = 0;
                },
            },
            Focus::Inspector => {
                self.inspector_selected = 0;
            },
            Focus::StatusTop => {
                self.status_top_selected = 0;
            },
            Focus::StatusBottom => {
                self.status_bottom_selected = 0;
            },
            _ => {},
        };
    }

    pub fn on_scroll_to_end(&mut self) {
        match self.focus {
            Focus::Branches => {
                self.branches_selected = usize::MAX;
            },
            Focus::Tags => {
                self.tags_selected = usize::MAX;
            },
            Focus::Stashes => {
                self.stashes_selected = usize::MAX;
            },
            Focus::Reflogs => {
                self.reflogs_selected = usize::MAX;
            },
            Focus::Worktrees => {
                self.worktrees_selected = usize::MAX;
            },
            Focus::Submodules => {
                self.submodules_selected = usize::MAX;
            },
            Focus::Search => {
                self.search_selected = Self::last_index(self.search_result_count());
            },
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    self.select_graph_index(Self::last_index(self.graph_commit_count()));
                },
                Viewport::Viewer => {
                    self.viewer_selected = usize::MAX;
                },
                Viewport::Settings => {
                    self.settings_selected = usize::MAX;
                },
                Viewport::Splash => {
                    self.splash_selected = Self::last_index(self.recent.len());
                },
            },
            Focus::Inspector => {
                self.inspector_selected = usize::MAX;
            },
            Focus::StatusTop => {
                self.status_top_selected = usize::MAX;
            },
            Focus::StatusBottom => {
                self.status_bottom_selected = usize::MAX;
            },
            _ => {},
        };
    }

    pub(crate) fn branch_name_at_pane_selection(&self) -> Option<String> {
        if let Some(window) = &self.graph.branches_window
            && self.branches_selected >= window.start
            && self.branches_selected < window.end
            && let Some(GraphPaneRow::Branch { name, .. }) = window.rows.get(self.branches_selected - window.start)
        {
            return Some(name.clone());
        }
        self.branches.sorted.get(self.branches_selected).map(|(_, branch)| branch.clone())
    }

    fn all_branch_names(&self) -> im::HashSet<String> {
        if let Some(repo) = &self.repo {
            let names = git_current_branch_names(repo);
            if !names.is_empty() {
                return names;
            }
        }

        let mut names: im::HashSet<String> = self.branches.sorted.iter().map(|(_, branch)| branch.clone()).collect();
        if let Some(window) = &self.graph.branches_window {
            for row in &window.rows {
                if let GraphPaneRow::Branch { name, .. } = row {
                    names.insert(name.clone());
                }
            }
        }
        names
    }

    fn save_hidden_branch_names(&self) {
        if self.repo.is_some()
            && let Some(path) = &self.path
        {
            save_branch_visibility(path, &self.branches.hidden_branch_names);
        }
    }

    fn solo_branch_name(&mut self, branch: &str) {
        let all_branches = self.all_branch_names();
        if all_branches.len() <= 1 || !all_branches.contains(branch) {
            self.branches.hidden_branch_names.clear();
        } else {
            self.branches.hidden_branch_names = all_branches;
            self.branches.hidden_branch_names.remove(branch);
        }
        self.save_hidden_branch_names();
    }

    fn toggle_branch_name(&mut self, branch: &str) {
        if self.branches.hidden_branch_names.contains(branch) {
            self.branches.hidden_branch_names.remove(branch);
        } else {
            self.branches.hidden_branch_names.insert(branch.to_string());
        }

        let all_branches = self.all_branch_names();
        let has_visible_branch = all_branches.iter().any(|branch| !self.branches.hidden_branch_names.contains(branch));
        if !all_branches.is_empty() && !has_visible_branch {
            self.branches.hidden_branch_names.clear();
        }
        self.save_hidden_branch_names();
    }

    pub(crate) fn graph_branch_choices(&self, alias: u32) -> Vec<String> {
        let mut choices: Vec<String> = self
            .graph
            .graph_window
            .as_ref()
            .into_iter()
            .flat_map(|window| window.rows.iter())
            .find(|row| row.alias == alias)
            .map(|row| row.branches.iter().map(|branch| branch.name.clone()).collect())
            .unwrap_or_default();

        if !choices.is_empty() {
            return choices;
        }

        choices =
            self.branches.sorted.iter().filter(|(branch_alias, branch)| *branch_alias == alias && !self.branches.hidden_branch_names.contains(branch)).map(|(_, branch)| branch.clone()).collect();

        choices
    }

    pub(crate) fn is_local_branch_name(&self, branch: &str) -> bool {
        if let Some(repo) = &self.repo {
            return repo.find_branch(branch, BranchType::Local).is_ok();
        }

        self.branches.local.values().any(|branches| branches.iter().any(|local| local.as_str() == branch))
    }

    pub(crate) fn graph_local_branch_choices(&self, alias: u32) -> Vec<String> {
        self.graph_branch_choices(alias).into_iter().filter(|branch| self.is_local_branch_name(branch)).collect()
    }

    pub(crate) fn modal_branch_action_choices(&self, alias: u32) -> Vec<String> {
        match self.modal_branch_action {
            BranchModalAction::Rename => self.graph_local_branch_choices(alias),
            BranchModalAction::Solo | BranchModalAction::Toggle => self.graph_branch_choices(alias),
        }
    }

    fn apply_graph_branch_action(&mut self, action: BranchModalAction) {
        if self.viewport != Viewport::Graph || self.graph_selected == 0 {
            return;
        }

        let Some(alias) = self.graph_alias_at(self.graph_selected) else {
            return;
        };
        let branch_names = self.graph_branch_choices(alias);

        match branch_names.as_slice() {
            [] => {},
            [branch] => {
                let mut should_reload = true;
                match action {
                    BranchModalAction::Solo => self.solo_branch_name(branch),
                    BranchModalAction::Toggle => self.toggle_branch_name(branch),
                    BranchModalAction::Rename => {
                        self.open_branch_rename_modal(branch.clone());
                        should_reload = false;
                    },
                }
                if should_reload {
                    self.reload(None);
                }
            },
            _ => {
                self.modal_branch_action = action;
                self.modal_solo_selected = 0;
                self.focus = Focus::ModalSolo;
            },
        }
    }

    pub fn on_toggle_branch(&mut self) {
        match self.focus {
            Focus::Branches => {
                if let Some(branch) = self.branch_name_at_pane_selection() {
                    self.toggle_branch_name(&branch);
                    self.reload(None);
                }
            },
            Focus::Viewport => self.apply_graph_branch_action(BranchModalAction::Toggle),
            _ => {},
        }
    }

    pub fn on_solo_branch(&mut self) {
        match self.focus {
            Focus::Branches => {
                if let Some(branch) = self.branch_name_at_pane_selection() {
                    self.solo_branch_name(&branch);
                    self.reload(None);
                }
            },
            Focus::Viewport => self.apply_graph_branch_action(BranchModalAction::Solo),
            _ => {},
        }
    }

    pub fn on_back(&mut self) {
        match self.focus {
            Focus::ModalCommit => {
                self.modal_input.clear();
                self.focus = Focus::Viewport;
            },
            Focus::ModalCherrypick => {
                self.modal_input.clear();
                self.pending_cherrypick_oid = None;
                self.focus = Focus::Viewport;
            },
            Focus::ModalRevert => {
                self.modal_input.clear();
                self.pending_revert_oid = None;
                self.focus = Focus::Viewport;
            },
            Focus::ModalCreateBranch => {
                self.modal_input.clear();
                self.clear_pending_branch_target();
                self.focus = Focus::Viewport;
            },
            Focus::ModalRenameBranch => {
                self.modal_input.clear();
                self.modal_rename_branch_source = None;
                self.focus = Focus::Viewport;
            },
            Focus::ModalCreateWorktreeName | Focus::ModalCreateWorktreePath => {
                self.modal_input.clear();
                self.modal_worktree_name.clear();
                self.focus = Focus::Viewport;
            },
            Focus::ModalWorktreeChooser | Focus::ModalRemoveWorktree => {
                self.close_worktree_modal();
            },
            Focus::ModalSolo => {
                self.modal_solo_selected = 0;
                self.modal_branch_action = BranchModalAction::Solo;
                self.focus = Focus::Viewport;
            },
            Focus::ModalDeleteBranch => {
                self.modal_delete_branch_selected = 0;
                self.focus = Focus::Viewport;
            },
            Focus::ModalDeleteTag => {
                self.modal_delete_tag_selected = 0;
                self.focus = Focus::Viewport;
            },
            Focus::ModalLockWorktree => {
                self.modal_input.clear();
                self.focus = Focus::Worktrees;
            },
            Focus::ModalRemoteAction | Focus::ModalRemoteDelete | Focus::ModalRemoteName | Focus::ModalRemoteUrl => {
                self.close_remote_modal();
            },
            Focus::ModalGraphLaneLimit => {
                self.modal_input.clear();
                self.focus = Focus::Viewport;
            },
            Focus::ModalFileSearch => {
                self.modal_input.clear();
                self.modal_file_search_results.clear();
                self.modal_file_search_selected = 0;
                self.modal_file_search_scroll.set(0);
                self.focus = self.modal_file_search_return_focus;
                self.modal_file_search_return_focus = Focus::Viewport;
            },
            Focus::ModalKeyCapture => {
                self.close_key_capture();
            },
            Focus::ModalAuth => {
                self.cancel_auth_prompt();
            },
            Focus::ModalNetworkProgress => {},
            Focus::ModalCheckout => {
                self.modal_checkout_selected = 0;
                self.focus = Focus::Viewport;
            },
            Focus::ModalGrep | Focus::ModalTag => {
                self.modal_input.clear();
                self.focus = Focus::Viewport;
            },
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    if self.spinner.is_running() {
                        return;
                    }
                    self.viewport = Viewport::Splash;
                    self.focus = Focus::Viewport;

                    // Highlight the current repository in the recent list when possible.
                    let mut selected = 0;

                    if let Some(path) = &self.path
                        && let Some(pos) = self.recent.iter().position(|p| p == path)
                    {
                        selected = pos;
                    }

                    self.splash_selected = selected;
                },
                Viewport::Splash => {
                    if self.spinner.is_running() {
                        return;
                    }
                    self.viewer_selected = 0;
                    self.viewport = Viewport::Graph;
                    self.focus = Focus::Viewport;
                    self.file_name = None;
                },
                _ => {
                    self.viewer_selected = 0;
                    self.viewport = Viewport::Graph;
                    self.focus = Focus::Viewport;
                    self.file_name = None;
                },
            },
            _ => {
                self.viewer_selected = 0;
                self.viewport = Viewport::Graph;
                self.focus = Focus::Viewport;
                self.file_name = None;
            },
        };
    }

    pub fn on_reload(&mut self) {
        self.reload(None);
        self.cleanup_after_reload();
    }

    pub fn on_reload_all_branches(&mut self) {
        self.branches.hidden_branch_names.clear();
        self.save_hidden_branch_names();
        self.reload(None);
        self.cleanup_after_reload();
    }

    fn cleanup_after_reload(&mut self) {
        match self.focus {
            Focus::ModalWorktreeChooser | Focus::ModalRemoveWorktree => {
                self.clear_worktree_modal_state();
                self.focus = Focus::Viewport;
            },
            Focus::ModalCherrypick => {
                self.modal_input.clear();
                self.pending_cherrypick_oid = None;
                self.focus = Focus::Viewport;
            },
            Focus::ModalRevert => {
                self.modal_input.clear();
                self.pending_revert_oid = None;
                self.focus = Focus::Viewport;
            },
            Focus::ModalCommit
            | Focus::ModalCreateBranch
            | Focus::ModalRenameBranch
            | Focus::ModalCreateWorktreeName
            | Focus::ModalCreateWorktreePath
            | Focus::ModalLockWorktree
            | Focus::ModalRemoteName
            | Focus::ModalRemoteUrl
            | Focus::ModalGraphLaneLimit
            | Focus::ModalFileSearch => {
                self.modal_input.clear();
                self.modal_file_search_results.clear();
                self.modal_file_search_selected = 0;
                self.modal_file_search_scroll.set(0);
                self.clear_pending_branch_target();
                self.modal_rename_branch_source = None;
                self.focus = Focus::Viewport;
            },
            Focus::ModalRemoteAction | Focus::ModalRemoteDelete => {
                self.close_remote_modal();
            },
            Focus::ModalAuth | Focus::ModalNetworkProgress => {
                self.pending_auth_prompt = None;
                self.auth_username_input.clear();
                self.auth_secret_input.clear();
                self.pending_network_request = None;
                self.network_handle = None;
                self.network_auth_attempts = 0;
                self.focus = Focus::Viewport;
            },
            Focus::ModalCheckout => {
                self.focus = Focus::Viewport;
            },
            _ => {},
        }
    }

    pub fn on_minimize(&mut self) {
        self.layout_config.is_minimal = !self.layout_config.is_minimal;
        self.mark_viewer_layout_dirty();
        self.save_layout();
    }

    pub fn on_reset_layout(&mut self) {
        let config = LayoutConfig::default();
        let should_reload = self.repo.is_some() && self.layout_config.is_graph_reflogs != config.is_graph_reflogs;
        self.layout_config = config;
        self.mouse_drag = None;
        self.mark_viewer_layout_dirty();
        self.viewport = Viewport::Graph;
        self.focus = Focus::Viewport;
        self.file_name = None;
        self.save_layout();
        if should_reload {
            self.reload(None);
        }
    }

    fn set_graph_lane_limit_from_shortcut(&mut self, limit: usize) {
        let limit = limit.max(1);
        if self.layout_config.graph_lane_limit == limit {
            return;
        }

        let was_settings = self.viewport == Viewport::Settings;
        let settings_selected = self.settings_selected;
        let settings_scroll = self.settings_scroll.get();

        self.layout_config.graph_lane_limit = limit;
        self.save_layout();

        if self.repo.is_some() {
            self.reload(None);
        }

        if was_settings {
            self.viewport = Viewport::Settings;
            self.focus = Focus::Viewport;
            self.settings_selected = settings_selected;
            self.settings_scroll.set(settings_scroll);
        }
    }

    pub fn on_shrink_graph_lane_limit(&mut self) {
        let current = self.layout_config.graph_lane_limit.max(1);
        self.set_graph_lane_limit_from_shortcut(current.saturating_sub(1).max(1));
    }

    pub fn on_grow_graph_lane_limit(&mut self) {
        let current = self.layout_config.graph_lane_limit.max(1);
        self.set_graph_lane_limit_from_shortcut(current.saturating_add(1));
    }

    pub fn on_toggle_shas(&mut self) {
        if self.viewport != Viewport::Splash {
            self.layout_config.is_shas = !self.layout_config.is_shas;
            self.save_layout();
        }
    }

    pub fn on_toggle_zen_mode(&mut self) {
        self.layout_config.is_zen = !self.layout_config.is_zen;
        self.mark_viewer_layout_dirty();
        self.save_layout();
    }

    pub fn on_toggle_branches(&mut self) {
        self.layout_config.is_branches = !self.layout_config.is_branches;
        self.mark_viewer_layout_dirty();
        if self.viewport == Viewport::Settings {
            return;
        }
        if self.layout_config.is_branches {
            self.focus = Focus::Branches;
        } else {
            self.focus = Focus::Viewport;
        }
        self.save_layout();
    }

    pub fn on_toggle_tags(&mut self) {
        self.layout_config.is_tags = !self.layout_config.is_tags;
        self.mark_viewer_layout_dirty();
        if self.viewport == Viewport::Settings {
            return;
        }
        if self.layout_config.is_tags {
            self.focus = Focus::Tags;
        } else {
            self.focus = Focus::Viewport;
        }
        self.save_layout();
    }

    pub fn on_toggle_stashes(&mut self) {
        self.layout_config.is_stashes = !self.layout_config.is_stashes;
        self.mark_viewer_layout_dirty();
        if self.viewport == Viewport::Settings {
            return;
        }
        if self.layout_config.is_stashes {
            self.focus = Focus::Stashes;
        } else {
            self.focus = Focus::Viewport;
        }
        self.save_layout();
    }

    pub fn on_toggle_reflogs(&mut self) {
        self.layout_config.is_reflogs = !self.layout_config.is_reflogs;
        self.mark_viewer_layout_dirty();
        if self.viewport == Viewport::Settings {
            return;
        }
        if self.layout_config.is_reflogs {
            self.focus = Focus::Reflogs;
        } else {
            self.focus = Focus::Viewport;
        }
        self.save_layout();
    }

    pub fn on_toggle_graph_reflogs(&mut self) {
        self.layout_config.is_graph_reflogs = !self.layout_config.is_graph_reflogs;
        self.mark_viewer_layout_dirty();
        self.save_layout();
        if self.repo.is_some() {
            self.reload(None);
            self.focus = Focus::Viewport;
            self.viewport = Viewport::Graph;
        }
    }

    pub fn on_toggle_graph_dates(&mut self) {
        if self.viewport != Viewport::Splash {
            self.layout_config.is_graph_dates = !self.layout_config.is_graph_dates;
            self.save_layout();
        }
    }

    pub fn on_toggle_graph_committers(&mut self) {
        if self.viewport != Viewport::Splash {
            self.layout_config.is_graph_committers = !self.layout_config.is_graph_committers;
            self.save_layout();
        }
    }

    pub fn on_toggle_graph_refs(&mut self) {
        if self.viewport != Viewport::Splash {
            self.layout_config.is_graph_refs = !self.layout_config.is_graph_refs;
            self.save_layout();
        }
    }

    pub fn on_toggle_worktrees(&mut self) {
        self.layout_config.is_worktrees = !self.layout_config.is_worktrees;
        self.mark_viewer_layout_dirty();
        if self.viewport == Viewport::Settings {
            return;
        }
        if self.layout_config.is_worktrees {
            self.focus = Focus::Worktrees;
        } else {
            self.focus = Focus::Viewport;
        }
        self.save_layout();
    }

    pub fn on_toggle_submodules(&mut self) {
        self.layout_config.is_submodules = !self.layout_config.is_submodules;
        self.mark_viewer_layout_dirty();
        if self.viewport == Viewport::Settings {
            return;
        }
        if self.layout_config.is_submodules {
            self.focus = Focus::Submodules;
        } else {
            self.focus = Focus::Viewport;
        }
        self.save_layout();
    }

    pub fn on_toggle_search(&mut self) {
        self.layout_config.is_search = !self.layout_config.is_search;
        self.mark_viewer_layout_dirty();
        if self.viewport == Viewport::Settings {
            return;
        }
        if self.layout_config.is_search {
            self.focus = Focus::Search;
        } else {
            self.focus = Focus::Viewport;
        }
        self.save_layout();
    }

    pub fn on_toggle_status(&mut self) {
        self.layout_config.is_status = !self.layout_config.is_status;
        self.mark_viewer_layout_dirty();
        if !self.layout_config.is_status && (self.focus == Focus::StatusTop || self.focus == Focus::StatusBottom) {
            self.focus = Focus::Viewport;
        }
        self.save_layout();
    }

    pub fn on_toggle_inspector(&mut self) {
        self.layout_config.is_inspector = !self.layout_config.is_inspector;
        self.mark_viewer_layout_dirty();
        if !self.layout_config.is_inspector && self.focus == Focus::Inspector {
            if self.layout_config.is_status {
                self.focus = Focus::StatusTop;
            } else {
                self.focus = Focus::Viewport;
            }
        }
        self.save_layout();
    }

    pub fn on_toggle_help(&mut self) {
        match self.viewport {
            Viewport::Graph => {
                self.viewport = Viewport::Settings;
                self.focus = Focus::Viewport;
                self.settings_tab = SettingsTab::General;
                self.settings_selected = 0;
                self.settings_scroll.set(0);
                self.last_input_direction = None;
            },
            _ => {
                self.viewport = Viewport::Graph;
                self.focus = Focus::Viewport;
            },
        };
    }

    pub fn on_exit(&mut self) {
        self.exit();
    }
}

#[cfg(test)]
#[path = "../../tests/app/input/navigation.rs"]
mod tests;
