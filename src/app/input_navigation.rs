use crate::{
    app::{
        app::{App, BranchModalAction, Direction, Focus, Viewport},
        app_default::ViewerMode,
    },
    git::{
        actions::{branching::delete_branch, checkout::checkout_branch, tagging::untag},
        queries::{commits::get_current_branch, diffs::get_filenames_diff_at_oid},
    },
    helpers::{keymap::InputMode, layout::LayoutConfig, palette::Theme},
};

const SETTINGS_THEME_SELECTION_START: usize = 7;

impl App {
    fn last_index(len: usize) -> usize {
        len.saturating_sub(1)
    }

    fn clamp_selection(selection: usize, len: usize) -> usize {
        selection.min(Self::last_index(len))
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

    fn refresh_current_diff_for_graph_selection(&mut self) {
        self.current_diff.clear();
        if self.graph_selected == 0 || self.graph_selected >= self.oids.get_commit_count() {
            return;
        }

        if let Some(repo) = self.repo.clone() {
            let oid = self.oids.get_oid_by_idx(self.graph_selected);
            self.current_diff = get_filenames_diff_at_oid(&repo, *oid);
        }
    }

    fn select_graph_index(&mut self, idx: usize) {
        self.graph_selected = Self::clamp_selection(idx, self.oids.get_commit_count());
        self.refresh_current_diff_for_graph_selection();
    }

    fn select_graph_alias(&mut self, alias: u32) -> bool {
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
        true
    }

    fn branch_alias_at_pane_selection(&self) -> Option<u32> {
        self.branches.sorted.get(self.branches_selected).map(|(alias, _)| *alias)
    }

    fn graph_visible_branch_indices(&self) -> Vec<usize> {
        let mut visible_indices: Vec<usize> = self
            .branches
            .all
            .iter()
            .filter_map(|(&alias, all_branches)| {
                let has_visible_branch = all_branches.iter().any(|branch| self.branches.visible_branch_names.is_empty() || self.branches.visible_branch_names.contains(branch));
                if has_visible_branch { self.branches.indices.get(alias as usize).copied() } else { None }
            })
            .collect();
        visible_indices.sort_unstable();
        visible_indices
    }

    pub(crate) fn graph_deletable_branch_choices(&self, alias: u32, current: Option<&str>) -> Vec<String> {
        self.graph_branch_choices(alias).into_iter().filter(|branch| current != Some(branch.as_str())).collect()
    }

    fn get_focusable_panes(&self) -> Vec<Focus> {
        let mut order = Vec::new();
        if self.viewport == Viewport::Settings || self.viewport == Viewport::Splash {
            return order;
        }
        for focus in &[Focus::Viewport, Focus::Inspector, Focus::StatusTop, Focus::StatusBottom, Focus::Worktrees, Focus::Reflogs, Focus::Stashes, Focus::Tags, Focus::Branches] {
            match focus {
                Focus::Viewport => order.push(Focus::Viewport),
                Focus::Inspector if self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) => order.push(Focus::Inspector),
                Focus::StatusTop if self.layout_config.is_status => order.push(*focus),
                Focus::StatusBottom if self.layout_config.is_status && self.graph_selected == 0 => order.push(*focus),
                Focus::Branches if self.layout_config.is_branches => order.push(Focus::Branches),
                Focus::Tags if self.layout_config.is_tags => order.push(Focus::Tags),
                Focus::Stashes if self.layout_config.is_stashes => order.push(Focus::Stashes),
                Focus::Reflogs if self.layout_config.is_reflogs => order.push(Focus::Reflogs),
                Focus::Worktrees if self.layout_config.is_worktrees => order.push(Focus::Worktrees),
                _ => {},
            }
        }
        order
    }

    pub fn on_action_mode(&mut self) {
        self.mode = InputMode::Action;
    }

    pub fn on_select(&mut self) {
        match self.focus {
            Focus::Viewport => match self.viewport {
                Viewport::Settings => {
                    if let Some(position) = self.settings_selections.iter().position(|&x| x == self.settings_selected) {
                        let Some(theme_idx) = position.checked_sub(SETTINGS_THEME_SELECTION_START) else {
                            return;
                        };
                        let Some(preset) = Theme::presets().get(theme_idx) else {
                            return;
                        };
                        self.set_theme(preset.theme);
                        self.save_theme_config();
                        self.reload(None);
                    }
                },
                Viewport::Splash => {
                    if let Some(path) = self.recent.get(self.splash_selected) {
                        self.reload(Some(path.to_string()));
                        self.graph_selected = 0;
                    }
                },
                Viewport::Graph => {
                    self.open_graph_worktree();
                },
                _ => {},
            },
            Focus::Branches => {
                if self.repo.is_some()
                    && let Some(alias) = self.branch_alias_at_pane_selection()
                {
                    self.open_graph_at_alias(alias);
                }
            },
            Focus::Tags => {
                if self.repo.is_some()
                    && let Some((alias, _)) = self.tags.sorted.get(self.tags_selected)
                {
                    self.open_graph_at_alias(*alias);
                }
            },
            Focus::Stashes => {
                if self.repo.is_some()
                    && let Some(alias) = self.oids.stashes.get(self.stashes_selected)
                {
                    self.open_graph_at_alias(*alias);
                }
            },
            Focus::Reflogs => {
                if self.repo.is_some()
                    && let Some(entry) = self.reflogs.entries.get(self.reflogs_selected)
                {
                    if !self.open_graph_at_alias(entry.new_alias) {
                        self.show_error("Reflog commit is hidden from the graph. Press 9 to show graph reflogs.");
                        return;
                    }
                }
            },
            Focus::Worktrees => {
                self.open_selected_worktree();
            },
            Focus::ModalWorktreeChooser => {
                self.confirm_worktree_chooser();
            },
            Focus::StatusTop | Focus::StatusBottom => {
                if let Some(repo) = &self.repo.clone() {
                    self.open_viewer(repo);
                    self.focus = Focus::Viewport;
                }
            },
            Focus::ModalCheckout => {
                if let Some(repo) = &self.repo {
                    let alias = self.oids.get_alias_by_idx(self.graph_selected);
                    let visible_branch_names = self.graph_branch_choices(alias);

                    if let Some(branch_name) = visible_branch_names.get(self.modal_checkout_selected as usize) {
                        match checkout_branch(repo, &mut self.branches.visible_branch_names, &mut self.branches.local, alias, branch_name) {
                            Ok(_) => {
                                self.modal_checkout_selected = 0;
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(format!("Checkout failed: {error}")),
                        }
                    }
                }
            },
            Focus::ModalSolo => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branch_names = self.graph_branch_choices(alias);
                let mut should_reload = false;

                if let Some(branch) = branch_names.get(self.modal_solo_selected as usize) {
                    match self.modal_branch_action {
                        BranchModalAction::Solo => self.solo_branch_name(branch),
                        BranchModalAction::Toggle => self.toggle_branch_name(branch),
                    }
                    should_reload = true;
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
                    let alias = self.oids.get_alias_by_idx(self.graph_selected);
                    let current = get_current_branch(repo);
                    let visible_branch_names = self.graph_deletable_branch_choices(alias, current.as_deref());

                    if let Some(branch) = visible_branch_names.get(self.modal_delete_branch_selected as usize) {
                        match delete_branch(repo, branch) {
                            Ok(_) => {
                                self.branches.visible_branch_names.remove(branch);
                                self.modal_delete_branch_selected = 0;
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(format!("Delete branch failed: {error}")),
                        }
                    }
                }
            },
            Focus::ModalDeleteTag => {
                if let Some(repo) = &self.repo {
                    let alias = self.oids.get_alias_by_idx(self.graph_selected);
                    let tags = self.tags.local.get(&alias).cloned().unwrap_or_default();
                    if let Some(tag) = tags.get(self.modal_delete_tag_selected as usize) {
                        match untag(repo, tag) {
                            Ok(_) => {
                                self.modal_delete_tag_selected = 0;
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(format!("Delete tag failed: {error}")),
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
                        self.focus = Focus::Inspector;
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
                    if let Some(path) = self.recent.get(self.splash_selected) {
                        self.reload(Some(path.to_string()));
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
                if self.repo.is_some()
                    && let Some(alias) = self.branch_alias_at_pane_selection()
                {
                    self.open_graph_at_alias(alias);
                }
            },
            Focus::Tags => {
                if self.repo.is_some()
                    && let Some((alias, _)) = self.tags.sorted.get(self.tags_selected)
                {
                    self.open_graph_at_alias(*alias);
                }
            },
            Focus::Stashes => {
                if self.repo.is_some()
                    && let Some(alias) = self.oids.stashes.get(self.stashes_selected)
                {
                    self.open_graph_at_alias(*alias);
                }
            },
            Focus::Reflogs => {
                if self.repo.is_some()
                    && let Some(entry) = self.reflogs.entries.get(self.reflogs_selected)
                {
                    if !self.open_graph_at_alias(entry.new_alias) {
                        self.show_error("Reflog commit is hidden from the graph. Press 9 to show graph reflogs.");
                        return;
                    }
                }
            },
            Focus::Worktrees => {
                self.open_selected_worktree();
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
        let active = self.get_focusable_panes();
        if active.is_empty() {
            return;
        }
        let idx = active.iter().position(|&f| f == self.focus).unwrap_or(0);
        self.focus = active[(idx + 1) % active.len()];
    }

    pub fn on_focus_prev_pane(&mut self) {
        let active = self.get_focusable_panes();
        if active.is_empty() {
            return;
        }
        let idx = active.iter().position(|&f| f == self.focus).unwrap_or(0);
        self.focus = active[(idx + active.len() - 1) % active.len()];
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
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branch_names = self.graph_branch_choices(alias);
                Self::wrap_modal_selection(&mut self.modal_checkout_selected, branch_names.len(), Direction::Up);
            },
            Focus::ModalSolo => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branch_names = self.graph_branch_choices(alias);
                Self::wrap_modal_selection(&mut self.modal_solo_selected, branch_names.len(), Direction::Up);
            },
            Focus::ModalDeleteBranch => {
                if let Some(repo) = &self.repo {
                    let alias = self.oids.get_alias_by_idx(self.graph_selected);
                    let current = get_current_branch(repo);
                    let branch_names = self.graph_deletable_branch_choices(alias, current.as_deref());
                    Self::wrap_modal_selection(&mut self.modal_delete_branch_selected, branch_names.len(), Direction::Up);
                }
            },
            Focus::ModalDeleteTag => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let tags = self.tags.local.get(&alias).cloned().unwrap_or_default();
                Self::wrap_modal_selection(&mut self.modal_delete_tag_selected, tags.len(), Direction::Up);
            },
            Focus::ModalWorktreeChooser => {
                Self::wrap_modal_selection(&mut self.modal_worktree_selected, self.modal_worktree_candidates.len(), Direction::Up);
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
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branch_names = self.graph_branch_choices(alias);
                Self::wrap_modal_selection(&mut self.modal_checkout_selected, branch_names.len(), Direction::Down);
            },
            Focus::ModalSolo => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branch_names = self.graph_branch_choices(alias);
                Self::wrap_modal_selection(&mut self.modal_solo_selected, branch_names.len(), Direction::Down);
            },
            Focus::ModalDeleteBranch => {
                if let Some(repo) = &self.repo {
                    let alias = self.oids.get_alias_by_idx(self.graph_selected);
                    let current = get_current_branch(repo);
                    let branch_names = self.graph_deletable_branch_choices(alias, current.as_deref());
                    Self::wrap_modal_selection(&mut self.modal_delete_branch_selected, branch_names.len(), Direction::Down);
                }
            },
            Focus::ModalDeleteTag => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let tags = self.tags.local.get(&alias).cloned().unwrap_or_default();
                Self::wrap_modal_selection(&mut self.modal_delete_tag_selected, tags.len(), Direction::Down);
            },
            Focus::ModalWorktreeChooser => {
                Self::wrap_modal_selection(&mut self.modal_worktree_selected, self.modal_worktree_candidates.len(), Direction::Down);
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
            _ => {},
        };
    }

    pub fn on_scroll_down_half(&mut self) {
        match self.focus {
            Focus::Viewport => {
                if self.viewport == Viewport::Graph {
                    let total = self.oids.get_commit_count();
                    let next = self.graph_selected.saturating_add(total.saturating_sub(self.graph_selected) / 2);
                    self.select_graph_index(next);
                }
            },
            Focus::Branches => {
                let total = self.branches.sorted.len();
                self.branches_selected = self.branches_selected + (total - self.branches_selected) / 2
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

        if let Some(&next) = self.graph_visible_branch_indices().iter().rev().find(|&&idx| idx < self.graph_selected) {
            self.select_graph_index(next);
        }
    }

    pub fn on_scroll_down_branch(&mut self) {
        if self.focus != Focus::Viewport || self.viewport != Viewport::Graph {
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
            let oid = self.oids.get_oid_by_idx(self.graph_selected);

            if self.oids.is_zero(oid) {
                return;
            }

            let child_positions: Vec<usize> = self
                .oids
                .get_sorted_aliases()
                .iter()
                .enumerate()
                .filter_map(|(idx, &alias)| {
                    let child_oid = self.oids.get_oid_by_alias(alias);
                    let commit = repo.find_commit(*child_oid).ok()?;
                    if commit.parent_ids().any(|parent_oid| parent_oid == *oid) { Some(idx) } else { None }
                })
                .collect();

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
            let oid = self.oids.get_oid_by_idx(self.graph_selected);

            if self.oids.is_zero(oid) {
                self.select_graph_index(1);
                return;
            }

            let next = {
                let Ok(commit) = repo.find_commit(*oid) else {
                    return;
                };
                let Some(parent_oid) = commit.parent_ids().next() else {
                    return;
                };
                let parent_alias = self.oids.get_alias_by_oid(parent_oid);
                self.oids.get_sorted_aliases().iter().position(|&alias| alias == parent_alias)
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
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    self.select_graph_index(Self::last_index(self.oids.get_commit_count()));
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

    fn branch_name_at_pane_selection(&self) -> Option<String> {
        self.branches.sorted.get(self.branches_selected).map(|(_, branch)| branch.clone())
    }

    fn all_branch_names(&self) -> im::HashSet<String> {
        self.branches.sorted.iter().map(|(_, branch)| branch.clone()).collect()
    }

    fn solo_branch_name(&mut self, branch: &str) {
        self.branches.visible_branch_names.clear();
        self.branches.visible_branch_names.insert(branch.to_string());
    }

    fn toggle_branch_name(&mut self, branch: &str) {
        if self.branches.visible_branch_names.is_empty() {
            self.branches.visible_branch_names = self.all_branch_names();
            self.branches.visible_branch_names.remove(branch);
        } else if self.branches.visible_branch_names.contains(branch) {
            self.branches.visible_branch_names.remove(branch);
        } else {
            self.branches.visible_branch_names.insert(branch.to_string());
        }

        if self.branches.visible_branch_names.is_empty() {
            return;
        }

        let has_visible_branch = self.branches.sorted.iter().any(|(_, branch)| self.branches.visible_branch_names.contains(branch));
        if !has_visible_branch {
            self.branches.visible_branch_names.clear();
        }
    }

    pub(crate) fn graph_branch_choices(&self, alias: u32) -> Vec<String> {
        self.branches
            .sorted
            .iter()
            .filter(|(branch_alias, branch)| *branch_alias == alias && (self.branches.visible_branch_names.is_empty() || self.branches.visible_branch_names.contains(branch)))
            .map(|(_, branch)| branch.clone())
            .collect()
    }

    fn apply_graph_branch_action(&mut self, action: BranchModalAction) {
        if self.viewport != Viewport::Graph || self.graph_selected == 0 {
            return;
        }

        let alias = self.oids.get_alias_by_idx(self.graph_selected);
        let branch_names = self.graph_branch_choices(alias);

        match branch_names.as_slice() {
            [] => {},
            [branch] => {
                match action {
                    BranchModalAction::Solo => self.solo_branch_name(branch),
                    BranchModalAction::Toggle => self.toggle_branch_name(branch),
                }
                self.reload(None);
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
            Focus::ModalCreateBranch => {
                self.modal_input.clear();
                self.clear_pending_branch_target();
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
            Focus::ModalLockWorktree => {
                self.modal_input.clear();
                self.focus = Focus::Worktrees;
            },
            Focus::ModalCheckout => {
                self.modal_checkout_selected = 0;
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
            Focus::ModalCommit | Focus::ModalCreateBranch | Focus::ModalCreateWorktreeName | Focus::ModalCreateWorktreePath | Focus::ModalLockWorktree => {
                self.modal_input.clear();
                self.clear_pending_branch_target();
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
        self.layout_drag = None;
        self.mark_viewer_layout_dirty();
        self.viewport = Viewport::Graph;
        self.focus = Focus::Viewport;
        self.file_name = None;
        self.save_layout();
        if should_reload {
            self.reload(None);
        }
    }

    pub fn on_toggle_shas(&mut self) {
        if self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            self.layout_config.is_shas = !self.layout_config.is_shas;
        }
        self.save_layout();
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
mod tests {
    use super::*;
    use crate::{core::chunk::NONE, core::reflogs::HeadReflogAliasEntry};
    use git2::{Repository, Signature};
    use std::{
        fs,
        path::Path,
        rc::Rc,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_non_repo_path(name: &str) -> String {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-input-navigation-{name}-{id}"));
        fs::create_dir_all(&path).unwrap();
        path.display().to_string()
    }

    fn temp_repo(name: &str) -> (std::path::PathBuf, Repository) {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-input-navigation-{name}-{id}"));
        fs::create_dir_all(&path).unwrap();
        let repo = Repository::init(&path).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        (path, repo)
    }

    fn commit_file(repo: &Repository, file: &str, message: &str) -> git2::Oid {
        let workdir = repo.workdir().unwrap().to_path_buf();
        fs::write(workdir.join(file), format!("{message}\n")).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
    }

    fn graph_app_with_history() -> (App, git2::Oid, git2::Oid, git2::Oid) {
        let (path, repo) = temp_repo("history");
        let root_oid = commit_file(&repo, "root.txt", "root");
        let parent_oid = commit_file(&repo, "parent.txt", "parent");
        let child_oid = commit_file(&repo, "child.txt", "child");

        let mut app = App { path: Some(path.display().to_string()), repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, ..Default::default() };
        let root_alias = app.oids.get_alias_by_oid(root_oid);
        let parent_alias = app.oids.get_alias_by_oid(parent_oid);
        let child_alias = app.oids.get_alias_by_oid(child_oid);
        app.oids.sorted_aliases = vec![NONE, child_alias, parent_alias, root_alias];

        let max_alias = [root_alias, parent_alias, child_alias].into_iter().max().unwrap() as usize;
        app.branches.indices = vec![usize::MAX; max_alias + 1];
        app.branches.indices[root_alias as usize] = 3;
        app.branches.indices[parent_alias as usize] = 2;
        app.branches.indices[child_alias as usize] = 1;
        app.branches.all.insert(parent_alias, vec!["parent".to_string()]);
        app.branches.sorted.push((parent_alias, "parent".to_string()));

        (app, root_oid, parent_oid, child_oid)
    }

    fn diff_filenames(app: &App) -> Vec<String> {
        app.current_diff.iter().map(|change| change.filename.clone()).collect()
    }

    fn branch_app() -> App {
        let mut app = App { path: Some(temp_non_repo_path("branches")), viewport: Viewport::Graph, ..Default::default() };
        app.branches.sorted = vec![(0, "feature".to_string()), (1, "main".to_string())];
        app.branches.all.insert(0, vec!["feature".to_string()]);
        app.branches.all.insert(1, vec!["main".to_string()]);
        app.oids.sorted_aliases = vec![NONE, 1];
        app
    }

    fn visible_branches(app: &App) -> Vec<String> {
        let mut branches: Vec<String> = app.branches.visible_branch_names.iter().cloned().collect();
        branches.sort();
        branches
    }

    #[test]
    fn solo_branch_from_pane_keeps_selected_as_only_visible() {
        let mut app = branch_app();
        app.focus = Focus::Branches;
        app.branches_selected = 1;
        app.branches.visible_branch_names.insert("main".to_string());

        app.on_solo_branch();

        assert_eq!(visible_branches(&app), vec!["main"]);
    }

    #[test]
    fn toggle_branch_from_all_visible_hides_selected_branch() {
        let mut app = branch_app();
        app.focus = Focus::Branches;
        app.branches_selected = 1;

        app.on_toggle_branch();

        assert_eq!(visible_branches(&app), vec!["feature"]);
    }

    #[test]
    fn toggle_last_visible_branch_returns_to_all_visible() {
        let mut app = branch_app();
        app.focus = Focus::Branches;
        app.branches_selected = 1;
        app.branches.visible_branch_names.insert("main".to_string());

        app.on_toggle_branch();

        assert!(app.branches.visible_branch_names.is_empty());
    }

    #[test]
    fn graph_solo_uses_selected_commit_branch() {
        let mut app = branch_app();
        app.focus = Focus::Viewport;
        app.graph_selected = 1;

        app.on_solo_branch();

        assert_eq!(visible_branches(&app), vec!["main"]);
    }

    #[test]
    fn graph_toggle_uses_selected_commit_branch() {
        let mut app = branch_app();
        app.focus = Focus::Viewport;
        app.graph_selected = 1;

        app.on_toggle_branch();

        assert_eq!(visible_branches(&app), vec!["feature"]);
    }

    #[test]
    fn graph_toggle_multiple_branch_commit_opens_toggle_modal() {
        let mut app = branch_app();
        app.focus = Focus::Viewport;
        app.graph_selected = 1;
        app.branches.sorted.push((1, "release".to_string()));

        app.on_toggle_branch();

        assert_eq!(app.focus, Focus::ModalSolo);
        assert_eq!(app.modal_branch_action, BranchModalAction::Toggle);
        assert_eq!(app.modal_solo_selected, 0);
    }

    #[test]
    fn empty_recent_splash_scrolls_keep_selection_at_zero() {
        let mut app = App { viewport: Viewport::Splash, focus: Focus::Viewport, ..Default::default() };
        app.layout.graph.height = 10;
        app.splash_selected = 3;

        app.on_scroll_down();
        assert_eq!(app.splash_selected, 0);

        app.splash_selected = 3;
        app.on_scroll_page_down();
        assert_eq!(app.splash_selected, 0);

        app.splash_selected = 3;
        app.on_scroll_half_page_down();
        assert_eq!(app.splash_selected, 0);

        app.splash_selected = 3;
        app.on_scroll_to_end();
        assert_eq!(app.splash_selected, 0);
    }

    #[test]
    fn empty_branch_pane_select_and_narrow_are_noops() {
        let (_path, repo) = temp_repo("empty-branches");
        let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Branches, ..Default::default() };

        app.on_select();
        assert_eq!(app.focus, Focus::Branches);

        app.on_narrow_scope();
        assert_eq!(app.focus, Focus::Branches);
    }

    #[test]
    fn empty_delete_tag_modal_navigation_stays_at_zero() {
        let mut app = App { focus: Focus::ModalDeleteTag, modal_delete_tag_selected: 4, ..Default::default() };

        app.on_scroll_up();
        assert_eq!(app.modal_delete_tag_selected, 0);

        app.modal_delete_tag_selected = 4;
        app.on_scroll_down();
        assert_eq!(app.modal_delete_tag_selected, 0);
    }

    #[test]
    fn graph_branch_and_commit_jumps_refresh_current_diff() {
        let (mut app, _root_oid, _parent_oid, _child_oid) = graph_app_with_history();

        app.graph_selected = 3;
        app.current_diff.clear();
        app.on_scroll_up_branch();
        assert_eq!(app.graph_selected, 2);
        assert_eq!(diff_filenames(&app), vec!["parent.txt"]);

        app.current_diff.clear();
        app.on_scroll_up_commit();
        assert_eq!(app.graph_selected, 1);
        assert_eq!(diff_filenames(&app), vec!["child.txt"]);

        app.current_diff.clear();
        app.on_scroll_down_commit();
        assert_eq!(app.graph_selected, 2);
        assert_eq!(diff_filenames(&app), vec!["parent.txt"]);
    }

    #[test]
    fn reflog_selection_refreshes_current_diff() {
        let (mut app, root_oid, _parent_oid, child_oid) = graph_app_with_history();
        let child_alias = app.oids.get_alias_by_oid(child_oid);
        let root_alias = app.oids.get_alias_by_oid(root_oid);
        app.focus = Focus::Reflogs;
        app.reflogs.entries.push(HeadReflogAliasEntry {
            selector: "HEAD@{0}".to_string(),
            old_oid: root_oid,
            new_oid: child_oid,
            new_alias: child_alias,
            message: "commit: child".to_string(),
            time: git2::Time::new(1, 0),
        });
        app.graph_selected = 3;
        app.current_diff.clear();

        app.on_select();

        assert_eq!(app.graph_selected, 1);
        assert_eq!(app.oids.get_alias_by_idx(3), root_alias);
        assert_eq!(diff_filenames(&app), vec!["child.txt"]);
    }
}
