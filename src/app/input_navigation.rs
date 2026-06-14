use crate::{
    app::{
        app::{App, Direction, Focus, Viewport},
        app_default::ViewerMode,
    },
    git::{
        actions::{branching::delete_branch, checkout::checkout_branch, tagging::untag},
        queries::{commits::get_current_branch, diffs::get_filenames_diff_at_oid},
    },
    helpers::{keymap::InputMode, palette::Theme},
};

impl App {
    fn get_focusable_panes(&self) -> Vec<Focus> {
        let mut order = Vec::new();
        if self.viewport == Viewport::Settings || self.viewport == Viewport::Splash {
            return order;
        }
        for focus in &[Focus::Viewport, Focus::Inspector, Focus::StatusTop, Focus::StatusBottom, Focus::Worktrees, Focus::Stashes, Focus::Tags, Focus::Branches] {
            match focus {
                Focus::Viewport => order.push(Focus::Viewport),
                Focus::Inspector if self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) => order.push(Focus::Inspector),
                Focus::StatusTop if self.layout_config.is_status => order.push(*focus),
                Focus::StatusBottom if self.layout_config.is_status && self.graph_selected == 0 => order.push(*focus),
                Focus::Branches if self.layout_config.is_branches => order.push(Focus::Branches),
                Focus::Tags if self.layout_config.is_tags => order.push(Focus::Tags),
                Focus::Stashes if self.layout_config.is_stashes => order.push(Focus::Stashes),
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
                        match position {
                            7 => self.theme = Theme::classic(),
                            8 => self.theme = Theme::ansi(),
                            9 => self.theme = Theme::monochrome(),
                            _ => {
                                return;
                            },
                        }
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
                if let Some(repo) = &self.repo {
                    self.viewport = Viewport::Graph;
                    self.focus = Focus::Viewport;
                    let alias = self.branches.sorted.get(self.branches_selected).unwrap().0;
                    self.graph_selected = self.oids.get_sorted_aliases().iter().position(|o| o == &alias).unwrap_or(0);
                    if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                        let oid = self.oids.get_oid_by_idx(self.graph_selected);
                        self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                    }
                }
            },
            Focus::Tags => {
                if let Some(repo) = &self.repo {
                    self.viewport = Viewport::Graph;
                    self.focus = Focus::Viewport;
                    if let Some(alias) = self.tags.sorted.get(self.tags_selected) {
                        self.graph_selected = self.oids.get_sorted_aliases().iter().position(|o| o == &alias.0).unwrap_or(0);
                        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                            let oid = self.oids.get_oid_by_idx(self.graph_selected);
                            self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                        }
                    }
                }
            },
            Focus::Stashes => {
                if let Some(repo) = &self.repo {
                    self.viewport = Viewport::Graph;
                    self.focus = Focus::Viewport;
                    if let Some(alias) = self.oids.stashes.get(self.stashes_selected) {
                        self.graph_selected = self.oids.get_sorted_aliases().iter().position(|o| o == alias).unwrap_or(0);
                        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                            let oid = self.oids.get_oid_by_idx(self.graph_selected);
                            self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                        }
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

                    // Restrict choices to the active branch filter unless no filter is active.
                    let visible_branch_names: Vec<String> = if self.branches.visible_branch_names.is_empty() {
                        self.branches.all.get(&alias).cloned().unwrap_or_default()
                    } else {
                        self.branches.visible_branch_names.iter().filter(|b| self.branches.all.get(&alias).is_some_and(|all| all.contains(b))).cloned().collect()
                    };

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

                // Solo choices mirror what the graph currently considers visible.
                let visible_branch_names: Vec<String> = if self.branches.visible_branch_names.is_empty() {
                    self.branches.all.get(&alias).cloned().unwrap_or_default()
                } else {
                    self.branches.visible_branch_names.iter().filter(|b| self.branches.all.get(&alias).is_some_and(|all| all.contains(b))).cloned().collect()
                };

                // Selecting an already soloed branch clears the filter back to all branches.
                if let Some(branch) = visible_branch_names.get(self.modal_solo_selected as usize) {
                    let is_already_solo = self.branches.visible_branch_names.len() == 1 && self.branches.visible_branch_names.contains(branch);

                    if is_already_solo {
                        self.branches.visible_branch_names.clear();
                    } else {
                        self.branches.visible_branch_names.clear();
                        self.branches.visible_branch_names.insert(branch.clone());
                    }
                }

                self.modal_solo_selected = 0;
                self.focus = Focus::Viewport;
                self.reload(None);
            },

            Focus::ModalDeleteBranch => {
                if let Some(repo) = &self.repo {
                    let alias = self.oids.get_alias_by_idx(self.graph_selected);

                    // Deletion choices mirror graph visibility so hidden branches stay untouched.
                    let visible_branch_names: Vec<String> = if self.branches.visible_branch_names.is_empty() {
                        self.branches.all.get(&alias).cloned().unwrap_or_default()
                    } else {
                        self.branches.visible_branch_names.iter().filter(|b| self.branches.all.get(&alias).is_some_and(|all| all.contains(b))).cloned().collect()
                    };

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
                if let Some(repo) = &self.repo {
                    self.viewport = Viewport::Graph;
                    self.focus = Focus::Viewport;
                    let alias = self.branches.sorted.get(self.branches_selected).unwrap().0;
                    self.graph_selected = self.oids.get_sorted_aliases().iter().position(|o| o == &alias).unwrap_or(0);
                    if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                        let oid = self.oids.get_oid_by_idx(self.graph_selected);
                        self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                    }
                }
            },
            Focus::Tags => {
                if let Some(repo) = &self.repo {
                    self.viewport = Viewport::Graph;
                    self.focus = Focus::Viewport;
                    if let Some(alias) = self.tags.sorted.get(self.tags_selected) {
                        self.graph_selected = self.oids.get_sorted_aliases().iter().position(|o| o == &alias.0).unwrap_or(0);
                        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                            let oid = self.oids.get_oid_by_idx(self.graph_selected);
                            self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                        }
                    }
                }
            },
            Focus::Stashes => {
                if let Some(repo) = &self.repo {
                    self.viewport = Viewport::Graph;
                    self.focus = Focus::Viewport;
                    if let Some(alias) = self.oids.stashes.get(self.stashes_selected) {
                        self.graph_selected = self.oids.get_sorted_aliases().iter().position(|o| o == alias).unwrap_or(0);
                        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                            let oid = self.oids.get_oid_by_idx(self.graph_selected);
                            self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                        }
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
            Focus::Worktrees => {
                let page = self.layout.worktrees.height as usize - 1;
                self.worktrees_selected = self.worktrees_selected.saturating_sub(page);
            },
            Focus::Viewport => {
                let page = self.layout.graph.height as usize - 1;
                match self.viewport {
                    Viewport::Graph => {
                        if let Some(repo) = &self.repo {
                            self.graph_selected = self.graph_selected.saturating_sub(page);
                            if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                                let oid = self.oids.get_oid_by_idx(self.graph_selected);
                                self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                            }
                        }
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
            Focus::Worktrees => {
                let page = self.layout.worktrees.height as usize - 1;
                self.worktrees_selected += page;
            },
            Focus::Viewport => {
                let page = self.layout.graph.height as usize - 1;
                match self.viewport {
                    Viewport::Graph => {
                        if let Some(repo) = &self.repo {
                            if self.graph_selected + page < self.oids.get_commit_count() {
                                self.graph_selected += page;
                            } else {
                                self.graph_selected = self.oids.get_commit_count() - 1;
                            }
                            if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                                let oid = self.oids.get_oid_by_idx(self.graph_selected);
                                self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                            }
                        }
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
                        self.splash_selected += page;
                        if self.splash_selected >= self.recent.len() {
                            self.splash_selected = self.recent.len() - 1;
                        };
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
            Focus::Worktrees => {
                self.worktrees_selected = self.worktrees_selected.saturating_sub(1);
            },
            Focus::Viewport => {
                match self.viewport {
                    Viewport::Graph => {
                        if let Some(repo) = &self.repo {
                            if self.graph_selected > 0 {
                                self.graph_selected -= 1;
                                if self.graph_selected == 0 && self.focus == Focus::Inspector && !self.uncommitted.has_conflicts {
                                    self.focus = Focus::Viewport;
                                }
                            }
                            if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                                let oid = self.oids.get_oid_by_idx(self.graph_selected);
                                self.current_diff = get_filenames_diff_at_oid(repo, *oid);
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

                // Modal navigation wraps over the same branch list the modal displays.
                let branch_names: Vec<String> = if self.branches.visible_branch_names.is_empty() {
                    self.branches.all.get(&alias).cloned().unwrap_or_default()
                } else {
                    self.branches.visible_branch_names.iter().filter(|name| self.branches.all.get(&alias).is_some_and(|branches| branches.contains(name))).cloned().collect()
                };

                let len = branch_names.len() as i32;
                if len > 0 {
                    self.modal_checkout_selected = if self.modal_checkout_selected - 1 < 0 { len - 1 } else { self.modal_checkout_selected - 1 };
                } else {
                    self.modal_checkout_selected = 0;
                }
            },
            Focus::ModalSolo => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);

                // Modal navigation wraps over the same branch list the modal displays.
                let branch_names: Vec<String> = if self.branches.visible_branch_names.is_empty() {
                    self.branches.all.get(&alias).cloned().unwrap_or_default()
                } else {
                    self.branches.visible_branch_names.iter().filter(|name| self.branches.all.get(&alias).is_some_and(|branches| branches.contains(name))).cloned().collect()
                };

                let len = branch_names.len() as i32;
                if len > 0 {
                    self.modal_solo_selected = if self.modal_solo_selected - 1 < 0 { len - 1 } else { self.modal_solo_selected - 1 };
                } else {
                    self.modal_solo_selected = 0;
                }
            },
            Focus::ModalDeleteBranch => {
                if let Some(repo) = &self.repo {
                    let alias = self.oids.get_alias_by_idx(self.graph_selected);

                    // Current branch is filtered out after visibility so it cannot be deleted.
                    let mut branch_names: Vec<String> = if self.branches.visible_branch_names.is_empty() {
                        self.branches.all.get(&alias).cloned().unwrap_or_default()
                    } else {
                        self.branches.visible_branch_names.iter().filter(|name| self.branches.all.get(&alias).is_some_and(|branches| branches.contains(name))).cloned().collect()
                    };

                    if let Some(current) = get_current_branch(repo) {
                        branch_names.retain(|branch| branch != &current);
                    }

                    let length = branch_names.len() as i32;
                    if length > 0 {
                        self.modal_delete_branch_selected = if self.modal_delete_branch_selected - 1 < 0 { length - 1 } else { self.modal_delete_branch_selected - 1 };
                    } else {
                        self.modal_delete_branch_selected = 0;
                    }
                }
            },
            Focus::ModalDeleteTag => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let tags = self.tags.local.get(&alias).cloned().unwrap_or_default();
                self.modal_delete_tag_selected = if self.modal_delete_tag_selected - 1 < 0 { tags.len() as i32 - 1 } else { self.modal_delete_tag_selected - 1 };
            },
            Focus::ModalWorktreeChooser => {
                let len = self.modal_worktree_candidates.len() as i32;
                if len > 0 {
                    self.modal_worktree_selected = if self.modal_worktree_selected - 1 < 0 { len - 1 } else { self.modal_worktree_selected - 1 };
                } else {
                    self.modal_worktree_selected = 0;
                }
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
            Focus::Worktrees => {
                self.worktrees_selected += 1;
            },
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    if let Some(repo) = &self.repo {
                        if self.graph_selected + 1 < self.oids.get_commit_count() {
                            self.graph_selected += 1;
                        }
                        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                            let oid = self.oids.get_oid_by_idx(self.graph_selected);
                            self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                        }
                    }
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
                    self.splash_selected += 1;
                    if self.splash_selected >= self.recent.len() {
                        self.splash_selected = self.recent.len() - 1;
                    };
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

                // Modal navigation wraps over the same branch list the modal displays.
                let branch_names: Vec<String> = if self.branches.visible_branch_names.is_empty() {
                    self.branches.all.get(&alias).cloned().unwrap_or_default()
                } else {
                    self.branches.visible_branch_names.iter().filter(|name| self.branches.all.get(&alias).is_some_and(|branches| branches.contains(name))).cloned().collect()
                };

                let len = branch_names.len() as i32;
                if len > 0 {
                    self.modal_checkout_selected = (self.modal_checkout_selected + 1) % len;
                } else {
                    self.modal_checkout_selected = 0;
                }
            },
            Focus::ModalSolo => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);

                // Modal navigation wraps over the same branch list the modal displays.
                let branch_names: Vec<String> = if self.branches.visible_branch_names.is_empty() {
                    self.branches.all.get(&alias).cloned().unwrap_or_default()
                } else {
                    self.branches.visible_branch_names.iter().filter(|name| self.branches.all.get(&alias).is_some_and(|branches| branches.contains(name))).cloned().collect()
                };

                let len = branch_names.len() as i32;
                if len > 0 {
                    self.modal_solo_selected = (self.modal_solo_selected + 1) % len;
                } else {
                    self.modal_solo_selected = 0;
                }
            },
            Focus::ModalDeleteBranch => {
                if let Some(repo) = &self.repo {
                    let alias = self.oids.get_alias_by_idx(self.graph_selected);

                    // Current branch is filtered out after visibility so it cannot be deleted.
                    let mut branch_names: Vec<String> = if self.branches.visible_branch_names.is_empty() {
                        self.branches.all.get(&alias).cloned().unwrap_or_default()
                    } else {
                        self.branches.visible_branch_names.iter().filter(|name| self.branches.all.get(&alias).is_some_and(|branches| branches.contains(name))).cloned().collect()
                    };

                    if let Some(current) = get_current_branch(repo) {
                        branch_names.retain(|branch| branch != &current);
                    }

                    let length = branch_names.len() as i32;
                    if length > 0 {
                        self.modal_delete_branch_selected = (self.modal_delete_branch_selected + 1) % length;
                    } else {
                        self.modal_delete_branch_selected = 0;
                    }
                }
            },
            Focus::ModalDeleteTag => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let tags = self.tags.local.get(&alias).cloned().unwrap_or_default();
                self.modal_delete_tag_selected = if self.modal_delete_tag_selected + 1 > tags.len() as i32 - 1 { 0 } else { self.modal_delete_tag_selected + 1 };
            },
            Focus::ModalWorktreeChooser => {
                let len = self.modal_worktree_candidates.len() as i32;
                if len > 0 {
                    self.modal_worktree_selected = (self.modal_worktree_selected + 1) % len;
                } else {
                    self.modal_worktree_selected = 0;
                }
            },
            _ => {},
        }
    }

    pub fn on_scroll_up_half(&mut self) {
        match self.focus {
            Focus::Viewport => {
                if self.viewport == Viewport::Graph
                    && let Some(repo) = &self.repo
                {
                    self.graph_selected /= 2;
                    if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                        let oid = self.oids.get_oid_by_idx(self.graph_selected);
                        self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                    }
                }
            },
            Focus::Branches => self.branches_selected /= 2,
            Focus::Tags => self.tags_selected /= 2,
            Focus::Stashes => self.stashes_selected /= 2,
            Focus::Worktrees => self.worktrees_selected /= 2,
            _ => {},
        };
    }

    pub fn on_scroll_down_half(&mut self) {
        match self.focus {
            Focus::Viewport => {
                if let Some(repo) = &self.repo
                    && self.viewport == Viewport::Graph
                {
                    self.graph_selected = (self.oids.get_commit_count() - 1).min(self.graph_selected + (self.oids.get_commit_count() - self.graph_selected) / 2);
                    if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                        let oid = self.oids.get_oid_by_idx(self.graph_selected);
                        self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                    }
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
            Focus::Worktrees => {
                let half = (self.layout.worktrees.height as usize - 1) / 2;
                self.worktrees_selected = self.worktrees_selected.saturating_sub(half);
            },
            Focus::Viewport => {
                let half = (self.layout.graph.height as usize - 1) / 2;
                match self.viewport {
                    Viewport::Graph => {
                        if let Some(repo) = &self.repo {
                            self.graph_selected = self.graph_selected.saturating_sub(half);
                            if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                                let oid = self.oids.get_oid_by_idx(self.graph_selected);
                                self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                            }
                        }
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
            Focus::Worktrees => {
                let half = (self.layout.worktrees.height.saturating_sub(1) as usize) / 2;
                self.worktrees_selected += half;
            },
            Focus::Viewport => {
                let half = (self.layout.graph.height.saturating_sub(1) as usize) / 2;
                match self.viewport {
                    Viewport::Graph => {
                        if let Some(repo) = &self.repo {
                            let max = self.oids.get_commit_count().saturating_sub(1);
                            self.graph_selected = (self.graph_selected + half).min(max);

                            if self.graph_selected < self.oids.get_commit_count() {
                                let oid = self.oids.get_oid_by_idx(self.graph_selected);
                                self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                            }
                        }
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
                        self.splash_selected += half;
                        if self.splash_selected >= self.recent.len() {
                            self.splash_selected = self.recent.len() - 1;
                        };
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

        // Build candidate rows from branch labels currently visible in the graph.
        let mut visible_indices: Vec<usize> = self
            .branches
            .all
            .iter()
            .filter_map(|(&alias, all_branches)| {
                let relevant_branches: Vec<&String> = all_branches.iter().filter(|b| self.branches.visible_branch_names.is_empty() || self.branches.visible_branch_names.contains(*b)).collect();
                if relevant_branches.is_empty() { None } else { self.branches.indices.get(alias as usize).copied() }
            })
            .collect();

        // Sorting by row makes "up" choose the nearest newer visible branch.
        visible_indices.sort_unstable();

        if let Some(&next) = visible_indices.iter().rev().find(|&&idx| idx < self.graph_selected) {
            self.graph_selected = next;
        }
    }

    pub fn on_scroll_down_branch(&mut self) {
        if self.focus != Focus::Viewport || self.viewport != Viewport::Graph {
            return;
        }

        // Build candidate rows from branch labels currently visible in the graph.
        let mut visible_indices: Vec<usize> = self
            .branches
            .all
            .iter()
            .filter_map(|(&alias, all_branches)| {
                let relevant_branches: Vec<&String> = all_branches.iter().filter(|b| self.branches.visible_branch_names.is_empty() || self.branches.visible_branch_names.contains(*b)).collect();
                if relevant_branches.is_empty() { None } else { self.branches.indices.get(alias as usize).copied() }
            })
            .collect();

        // Sorting by row makes "down" choose the nearest older visible branch.
        visible_indices.sort_unstable();

        if let Some(&next) = visible_indices.iter().find(|&&idx| idx > self.graph_selected) {
            self.graph_selected = next;
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

            if child_positions.is_empty() {
            } else {
                self.graph_selected = child_positions[0];
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
                self.graph_selected = 1;
                return;
            }

            let commit = repo.find_commit(*oid).unwrap();
            let mut parents = commit.parent_ids();

            if parents.len() == 0 {
            } else {
                let parent_oid = parents.next().unwrap();
                let parent_alias = self.oids.get_alias_by_oid(parent_oid);
                let next = self.oids.get_sorted_aliases().iter().position(|&alias| alias == parent_alias).unwrap();
                self.graph_selected = next;
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
            Focus::Worktrees => {
                self.worktrees_selected = 0;
            },
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    self.graph_selected = 0;
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
            Focus::Worktrees => {
                self.worktrees_selected = usize::MAX;
            },
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    if let Some(repo) = &self.repo {
                        self.graph_selected = self.oids.get_commit_count().saturating_sub(1);
                        if self.graph_selected != 0 {
                            let oid = self.oids.get_oid_by_idx(self.graph_selected);
                            self.current_diff = get_filenames_diff_at_oid(repo, *oid);
                        }
                    }
                },
                Viewport::Viewer => {
                    self.viewer_selected = usize::MAX;
                },
                Viewport::Settings => {
                    self.settings_selected = usize::MAX;
                },
                Viewport::Splash => {
                    self.splash_selected = self.recent.len() - 1;
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

    pub fn on_toggle_branch(&mut self) {
        if self.focus == Focus::Branches {
            let (_, branch) = self.branches.sorted.get(self.branches_selected).unwrap();
            let branch = branch.clone();

            if self.branches.visible_branch_names.contains(&branch) {
                self.branches.visible_branch_names.remove(&branch);
            } else {
                self.branches.visible_branch_names.insert(branch);
            }

            self.reload(None);
        }
    }

    pub fn on_solo_branch(&mut self) {
        match self.focus {
            Focus::Branches => {
                let (_, branch) = self.branches.sorted.get(self.branches_selected).unwrap();
                let branch = branch.clone();

                // Repeating solo on the same branch clears the filter back to all branches.
                if self.branches.visible_branch_names.len() == 1 && self.branches.visible_branch_names.contains(&branch) {
                    self.branches.visible_branch_names.clear();
                } else {
                    self.branches.visible_branch_names.clear();
                    self.branches.visible_branch_names.insert(branch);
                }

                self.reload(None);
            },

            Focus::Viewport => {
                if self.viewport != Viewport::Graph || self.graph_selected == 0 {
                    return;
                }

                let alias = self.oids.get_alias_by_idx(self.graph_selected);

                // Commits with multiple branch labels need a modal before choosing the solo branch.
                let branches_for_alias: Vec<String> = self.branches.sorted.iter().filter(|(b_alias, _)| *b_alias == alias).map(|(_, name)| name.clone()).collect();

                if branches_for_alias.is_empty() {
                    return;
                }

                if branches_for_alias.len() == 1 {
                    let branch = &branches_for_alias[0];

                    if self.branches.visible_branch_names.len() == 1 && self.branches.visible_branch_names.contains(branch) {
                        self.branches.visible_branch_names.clear();
                    } else {
                        self.branches.visible_branch_names.clear();
                        self.branches.visible_branch_names.insert(branch.clone());
                    }

                    self.reload(None);
                } else {
                    self.focus = Focus::ModalSolo;
                }
            },

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
            Focus::ModalCommit | Focus::ModalCreateWorktreeName | Focus::ModalCreateWorktreePath | Focus::ModalLockWorktree => {
                self.modal_input.clear();
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
