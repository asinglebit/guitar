use crate::git::actions::resetting::reset_file;
use crate::helpers::{
    keymap::{Command, KeyBinding, load_or_init_keymaps},
    layout::{LAYOUT_HEIGHT_MIN_STACKED_PANE, LAYOUT_WIDTH_MIN_CENTER, LAYOUT_WIDTH_MIN_SIDE_PANE},
};
use crate::{
    app::{
        app::{App, Direction, Focus, LayoutDrag, Viewport},
        app_default::ViewerMode,
    },
    git::{
        actions::{
            branching::{create_branch, delete_branch},
            checkout::{checkout_branch, checkout_head},
            cherrypicking::cherry_pick_commit,
            committing::commit_staged,
            fetching::fetch_over_ssh,
            pushing::{push_over_ssh, push_tags_over_ssh},
            resetting::reset_to_commit,
            staging::{stage_all, stage_file, unstage_all, unstage_file},
            stashing::{pop, stash},
            tagging::{tag, untag},
        },
        queries::{commits::get_current_branch, diffs::get_filenames_diff_at_oid},
    },
    helpers::{keymap::InputMode, palette::Theme},
};
use git2::{Oid, Repository};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
};
use std::io;
use std::path::Path;

impl App {
    pub fn load_keymap(&mut self) {
        self.keymaps = load_or_init_keymaps();
    }

    fn get_focusable_panes(&self) -> Vec<Focus> {
        let mut order = Vec::new();
        if self.viewport == Viewport::Settings || self.viewport == Viewport::Splash {
            return order;
        }
        for focus in &[Focus::Viewport, Focus::Inspector, Focus::StatusTop, Focus::StatusBottom, Focus::Stashes, Focus::Tags, Focus::Branches] {
            match focus {
                Focus::Viewport => order.push(Focus::Viewport),
                Focus::Inspector if self.layout_config.is_inspector && self.graph_selected != 0 => order.push(Focus::Inspector),
                Focus::StatusTop if self.layout_config.is_status => order.push(*focus),
                Focus::StatusBottom if self.layout_config.is_status && self.graph_selected == 0 => order.push(*focus),
                Focus::Branches if self.layout_config.is_branches => order.push(Focus::Branches),
                Focus::Tags if self.layout_config.is_tags => order.push(Focus::Tags),
                Focus::Stashes if self.layout_config.is_stashes => order.push(Focus::Stashes),
                _ => {},
            }
        }
        order
    }

    pub fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if matches!(key_event.kind, KeyEventKind::Press) => {
                self.handle_key_event(key_event);
            },
            Event::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event);
            },
            _ => {},
        };
        Ok(())
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.layout_drag = self.layout_drag_at(mouse_event.column, mouse_event.row);
            },
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(drag) = self.layout_drag {
                    self.apply_layout_drag(drag, mouse_event.column, mouse_event.row);
                }
            },
            MouseEventKind::Up(MouseButton::Left) => {
                if self.layout_drag.take().is_some() {
                    self.save_layout();
                }
            },
            MouseEventKind::ScrollUp => {
                self.handle_mouse_scroll(mouse_event.column, mouse_event.row, Direction::Up);
            },
            MouseEventKind::ScrollDown => {
                self.handle_mouse_scroll(mouse_event.column, mouse_event.row, Direction::Down);
            },
            _ => {},
        }
    }

    fn handle_mouse_scroll(&mut self, column: u16, row: u16, direction: Direction) {
        if self.layout_drag.is_some() {
            return;
        }

        if self.is_modal_focus() {
            match direction {
                Direction::Up => self.on_scroll_up(),
                Direction::Down => self.on_scroll_down(),
            }
            return;
        }

        if let Some(focus) = self.scroll_focus_at(column, row) {
            self.focus = focus;
            match direction {
                Direction::Up => self.on_scroll_up(),
                Direction::Down => self.on_scroll_down(),
            }
        }
    }

    fn scroll_focus_at(&self, column: u16, row: u16) -> Option<Focus> {
        if self.layout_config.is_zen {
            return Self::rect_contains(self.layout.app, column, row).then_some(self.focus);
        }

        if matches!(self.viewport, Viewport::Splash | Viewport::Settings) {
            return Self::rect_contains(self.layout.app, column, row).then_some(Focus::Viewport);
        }

        if self.layout_config.is_branches && Self::rect_contains(self.layout.pane_branches, column, row) {
            return Some(Focus::Branches);
        }
        if self.layout_config.is_tags && Self::rect_contains(self.layout.pane_tags, column, row) {
            return Some(Focus::Tags);
        }
        if self.layout_config.is_stashes && Self::rect_contains(self.layout.pane_stashes, column, row) {
            return Some(Focus::Stashes);
        }
        if self.layout_config.is_inspector && self.graph_selected != 0 && Self::rect_contains(self.layout.pane_inspector, column, row) {
            return Some(Focus::Inspector);
        }
        if self.layout_config.is_status && self.graph_selected == 0 && Self::rect_contains(self.layout.pane_status_bottom, column, row) {
            return Some(Focus::StatusBottom);
        }
        if self.layout_config.is_status && Self::rect_contains(self.layout.pane_status_top, column, row) {
            return Some(Focus::StatusTop);
        }
        if Self::rect_contains(self.layout.graph_scrollbar, column, row) {
            return Some(Focus::Viewport);
        }

        None
    }

    fn layout_drag_at(&self, column: u16, row: u16) -> Option<LayoutDrag> {
        if self.layout_config.is_zen || matches!(self.viewport, Viewport::Splash | Viewport::Settings) || self.is_modal_focus() {
            return None;
        }

        if Self::rect_contains(self.layout.divider_left, column, row) {
            return Some(LayoutDrag::LeftPane);
        }
        if Self::rect_contains(self.layout.divider_right, column, row) {
            return Some(LayoutDrag::RightPane);
        }
        if Self::rect_contains(self.layout.divider_branches_tags, column, row) {
            return Some(LayoutDrag::BranchesTags);
        }
        if Self::rect_contains(self.layout.divider_branches_stashes, column, row) {
            return Some(LayoutDrag::BranchesStashes);
        }
        if Self::rect_contains(self.layout.divider_tags_stashes, column, row) {
            return Some(LayoutDrag::TagsStashes);
        }
        if Self::rect_contains(self.layout.divider_inspector_status, column, row) {
            return Some(LayoutDrag::InspectorStatus);
        }
        if Self::rect_contains(self.layout.divider_status_files, column, row) {
            return Some(LayoutDrag::StatusFiles);
        }

        None
    }

    fn is_modal_focus(&self) -> bool {
        matches!(
            self.focus,
            Focus::ModalCheckout
                | Focus::ModalSolo
                | Focus::ModalCommit
                | Focus::ModalCreateBranch
                | Focus::ModalDeleteBranch
                | Focus::ModalGrep
                | Focus::ModalTag
                | Focus::ModalDeleteTag
                | Focus::ModalError
        )
    }

    fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
        rect.width > 0 && rect.height > 0 && column >= rect.x && column < rect.x.saturating_add(rect.width) && row >= rect.y && row < rect.y.saturating_add(rect.height)
    }

    fn apply_layout_drag(&mut self, drag: LayoutDrag, column: u16, row: u16) {
        match drag {
            LayoutDrag::LeftPane => self.resize_left_pane(column),
            LayoutDrag::RightPane => self.resize_right_pane(column),
            LayoutDrag::BranchesTags => {
                if let Some((branches, tags)) = Self::resized_pair_weights(row, self.layout.pane_branches, self.layout.pane_tags, self.layout_config.weight_branches, self.layout_config.weight_tags) {
                    self.layout_config.weight_branches = branches;
                    self.layout_config.weight_tags = tags;
                }
            },
            LayoutDrag::BranchesStashes => {
                if let Some((branches, stashes)) =
                    Self::resized_pair_weights(row, self.layout.pane_branches, self.layout.pane_stashes, self.layout_config.weight_branches, self.layout_config.weight_stashes)
                {
                    self.layout_config.weight_branches = branches;
                    self.layout_config.weight_stashes = stashes;
                }
            },
            LayoutDrag::TagsStashes => {
                if let Some((tags, stashes)) = Self::resized_pair_weights(row, self.layout.pane_tags, self.layout.pane_stashes, self.layout_config.weight_tags, self.layout_config.weight_stashes) {
                    self.layout_config.weight_tags = tags;
                    self.layout_config.weight_stashes = stashes;
                }
            },
            LayoutDrag::InspectorStatus => {
                if let Some((inspector, status)) =
                    Self::resized_pair_weights(row, self.layout.pane_inspector, self.layout.pane_status, self.layout_config.weight_inspector, self.layout_config.weight_status)
                {
                    self.layout_config.weight_inspector = inspector;
                    self.layout_config.weight_status = status;
                }
            },
            LayoutDrag::StatusFiles => {
                if let Some((top, bottom)) =
                    Self::resized_pair_weights(row, self.layout.pane_status_top, self.layout.pane_status_bottom, self.layout_config.weight_status_top, self.layout_config.weight_status_bottom)
                {
                    self.layout_config.weight_status_top = top;
                    self.layout_config.weight_status_bottom = bottom;
                }
            },
        }
    }

    fn resize_left_pane(&mut self, column: u16) {
        let total_width = self.layout.app.width;
        let other_width = self.layout.pane_right.width;
        let max_width = total_width.saturating_sub(other_width).saturating_sub(LAYOUT_WIDTH_MIN_CENTER);
        if max_width < LAYOUT_WIDTH_MIN_SIDE_PANE {
            return;
        }

        let desired_width = column.saturating_sub(self.layout.app.x);
        self.layout_config.width_left_pane = desired_width.clamp(LAYOUT_WIDTH_MIN_SIDE_PANE, max_width);
    }

    fn resize_right_pane(&mut self, column: u16) {
        let total_width = self.layout.app.width;
        let other_width = self.layout.pane_left.width;
        let max_width = total_width.saturating_sub(other_width).saturating_sub(LAYOUT_WIDTH_MIN_CENTER);
        if max_width < LAYOUT_WIDTH_MIN_SIDE_PANE {
            return;
        }

        let app_right = self.layout.app.x.saturating_add(self.layout.app.width);
        let desired_width = app_right.saturating_sub(column.saturating_add(1));
        self.layout_config.width_right_pane = desired_width.clamp(LAYOUT_WIDTH_MIN_SIDE_PANE, max_width);
    }

    fn resized_pair_weights(row: u16, first: Rect, second: Rect, first_weight: u16, second_weight: u16) -> Option<(u16, u16)> {
        if first.height == 0 || second.height == 0 {
            return None;
        }

        let pair_top = first.y;
        let pair_bottom = second.y.saturating_add(second.height);
        let pair_height = pair_bottom.saturating_sub(pair_top);
        if pair_height < 2 {
            return None;
        }

        let min_height = LAYOUT_HEIGHT_MIN_STACKED_PANE.min(pair_height / 2).max(1);
        let max_first_height = pair_height.saturating_sub(min_height);
        let first_height = row.saturating_sub(pair_top).saturating_add(1).clamp(min_height, max_first_height);
        let total_weight = first_weight.max(1).saturating_add(second_weight.max(1));
        if total_weight < 2 {
            return None;
        }

        let first_weight = ((first_height as u32 * total_weight as u32) / pair_height as u32).clamp(1, total_weight.saturating_sub(1) as u32) as u16;
        Some((first_weight, total_weight.saturating_sub(first_weight)))
    }

    pub fn show_error(&mut self, message: impl Into<String>) {
        if self.focus != Focus::ModalError {
            self.modal_error_return_focus = self.focus;
        }
        self.modal_error_message = message.into();
        self.focus = Focus::ModalError;
    }

    fn close_error_modal(&mut self) {
        self.focus = self.modal_error_return_focus;
        self.modal_error_message.clear();
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        // Commands are looked up by the keymap for the current input mode.
        let key_binding = KeyBinding::new(key_event.code, key_event.modifiers);
        let current_mode = self.mode;

        if self.focus == Focus::ModalError {
            if matches!(key_event.code, KeyCode::Enter | KeyCode::Esc) {
                self.close_error_modal();
            }
            self.mode = InputMode::Normal;
            return;
        }

        // Text-entry modals own all key handling until they submit or cancel.
        match self.focus {
            Focus::ModalCommit => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_input.clear();
                    },
                    KeyCode::Enter => {
                        if let Some(repo) = &self.repo {
                            match commit_staged(repo, self.modal_input.value(), &self.name, &self.email) {
                                Ok(_) => {
                                    self.modal_input.clear();
                                    self.reload(None);
                                    self.focus = Focus::Viewport;
                                },
                                Err(error) => self.show_error(format!("Commit failed: {error}")),
                            }
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                return;
            },
            Focus::ModalCreateBranch => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_input.clear();
                    },
                    KeyCode::Enter => {
                        if let Some(repo) = &self.repo {
                            let oid = self.oids.get_oid_by_idx(if self.graph_selected == 0 { 1 } else { self.graph_selected });
                            match create_branch(repo, self.modal_input.value(), *oid) {
                                Ok(_) => {
                                    self.modal_input.clear();
                                    self.reload(None);
                                    self.focus = Focus::Viewport;
                                },
                                Err(error) => self.show_error(format!("Create branch failed: {error}")),
                            }
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                return;
            },
            Focus::ModalGrep => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_input.clear();
                    },
                    KeyCode::Enter => {
                        let sha = self.modal_input.value();

                        // Git object prefixes cannot be empty or longer than a full SHA-1.
                        if sha.is_empty() || sha.len() > 40 {
                            return;
                        }

                        // Match against already loaded OIDs; unloaded history cannot be jumped to yet.
                        let oid: Option<Oid> = self.oids.oids.iter().find(|oid| oid.to_string().starts_with(sha)).copied();

                        if let Some(oid) = oid {
                            let oid_alias = self.oids.get_alias_by_oid(oid);

                            // Jump the graph selection to the row for the matched alias.
                            let next = self.oids.get_sorted_aliases().iter().position(|&alias| alias == oid_alias).unwrap();

                            self.graph_selected = next;
                            self.modal_input.clear();
                            self.focus = Focus::Viewport;
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                return;
            },
            Focus::ModalTag => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_input.clear();
                    },
                    KeyCode::Enter => {
                        if let Some(repo) = &self.repo {
                            let tag_name = self.modal_input.value();

                            // Empty tag names would fail in git anyway and leave the modal unchanged.
                            if tag_name.is_empty() {
                                return;
                            }

                            let oid = self.oids.get_oid_by_idx(if self.graph_selected == 0 { 1 } else { self.graph_selected });

                            match tag(repo, *oid, tag_name) {
                                Ok(_) => {
                                    self.reload(None);
                                    self.modal_input.clear();
                                    self.focus = Focus::Viewport;
                                },
                                Err(error) => self.show_error(format!("Create tag failed: {error}")),
                            }
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                return;
            },
            _ => {},
        }

        if let Some(mode_map) = self.keymaps.get(&self.mode)
            && let Some(cmd) = mode_map.get(&key_binding)
        {
            if !(self.viewport == Viewport::Splash && self.focus == Focus::Viewport) {
                match cmd {
                    // User interface commands change focus, panes, modes, and top-level views.
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
                    Command::ToggleStatus => self.on_toggle_status(),
                    Command::ToggleInspector => self.on_toggle_inspector(),
                    Command::ToggleShas => self.on_toggle_shas(),
                    Command::ToggleHelp => self.on_toggle_help(),
                    Command::ActionMode => self.on_action_mode(),
                    Command::Exit => self.on_exit(),

                    // List commands share selection semantics across panes.
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

                    // Graph commands understand commit topology and branch filters.
                    Command::ScrollUpBranch => self.on_scroll_up_branch(),
                    Command::ScrollDownBranch => self.on_scroll_down_branch(),
                    Command::ScrollUpCommit => self.on_scroll_up_commit(),
                    Command::ScrollDownCommit => self.on_scroll_down_commit(),
                    Command::Find => self.on_find(),
                    Command::SoloBranch => self.on_solo_branch(),
                    Command::ToggleBranch => self.on_toggle_branch(),

                    // Viewer commands change how diffs are navigated.
                    Command::ToggleHunkMode => self.on_toggle_hunk_mode(),

                    // Git commands mutate repository state and usually reload afterward.
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
                    Command::Tag => self.on_tag(),
                    Command::Untag => self.on_untag(),
                    Command::Cherrypick => self.on_cherrypick(),
                    Command::Reload => self.on_reload(),
                }
            } else {
                match cmd {
                    // Splash allows only navigation and opening a selected recent repo.
                    Command::NarrowScope => self.on_narrow_scope(),
                    Command::Select => self.on_select(),
                    Command::Back => self.on_back(),
                    Command::Exit => self.on_exit(),

                    // List navigation keeps the recent-repository picker usable.
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

        // Action mode is single-shot so destructive commands require a fresh prefix.
        if current_mode == InputMode::Action {
            self.mode = InputMode::Normal;
        }
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
                if self.graph_selected != 0 {
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
                        if self.viewer_selected + page < self.viewer_lines.len() {
                            self.viewer_selected += page;
                        } else {
                            self.viewer_selected = self.viewer_lines.len() - 1;
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
            Focus::Viewport => {
                match self.viewport {
                    Viewport::Graph => {
                        if let Some(repo) = &self.repo {
                            if self.graph_selected > 0 {
                                self.graph_selected -= 1;
                                if self.graph_selected == 0 && self.focus == Focus::Inspector {
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
                    if self.viewer_selected + 1 < self.viewer_lines.len() {
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
        }

        self.viewer_scroll.set(self.viewer_selected);
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

    pub fn on_drop(&mut self) {
        if self.repo.is_some() && self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            let alias = self.oids.get_alias_by_idx(self.graph_selected);
            if !self.oids.stashes.contains(&alias) {
                return;
            }

            let Some(path) = self.repo.as_ref().map(|repo| repo.path().to_path_buf()) else {
                return;
            };
            let mut repo = match Repository::open(path) {
                Ok(repo) => repo,
                Err(error) => {
                    self.show_error(format!("Open repository failed: {error}"));
                    return;
                },
            };
            let oid = self.oids.get_oid_by_alias(alias);

            match pop(&mut repo, oid, false) {
                Ok(_) => self.reload(None),
                Err(error) => self.show_error(format!("Drop stash failed: {error}")),
            }
        }
    }

    pub fn on_pop(&mut self) {
        if self.repo.is_some() && self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            let alias = self.oids.get_alias_by_idx(self.graph_selected);
            if !self.oids.stashes.contains(&alias) {
                return;
            }

            let Some(path) = self.repo.as_ref().map(|repo| repo.path().to_path_buf()) else {
                return;
            };
            let mut repo = match Repository::open(path) {
                Ok(repo) => repo,
                Err(error) => {
                    self.show_error(format!("Open repository failed: {error}"));
                    return;
                },
            };
            let oid = self.oids.get_oid_by_alias(alias);

            match pop(&mut repo, oid, true) {
                Ok(_) => self.reload(None),
                Err(error) => self.show_error(format!("Pop stash failed: {error}")),
            }
        }
    }

    pub fn on_stash(&mut self) {
        if self.repo.is_some() && self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            let Some(path) = self.repo.as_ref().map(|repo| repo.path().to_path_buf()) else {
                return;
            };
            let mut repo = match Repository::open(path) {
                Ok(repo) => repo,
                Err(error) => {
                    self.show_error(format!("Open repository failed: {error}"));
                    return;
                },
            };

            match stash(&mut repo) {
                Ok(_) => self.reload(None),
                Err(error) => self.show_error(format!("Stash failed: {error}")),
            }
        }
    }

    pub fn on_find(&mut self) {
        if self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            self.focus = Focus::ModalGrep;
        }
    }

    pub fn on_fetch_all(&mut self) {
        if self.viewport != Viewport::Settings {
            let repo_path = self.path.as_deref().unwrap_or(".");
            let handle = fetch_over_ssh(repo_path, "origin");
            match handle.join() {
                Ok(Ok(_)) => {
                    self.reload(None);
                },
                Ok(Err(error)) => self.show_error(format!("Fetch failed: {error}")),
                Err(_) => self.show_error("Fetch failed: worker thread panicked"),
            }
        }
    }

    pub fn on_checkout(&mut self) {
        let Some(repo) = &self.repo else { return };

        match self.focus {
            Focus::Branches => {
                // Branch pane checkout uses the selected row directly.
                let Some((alias, branch)) = self.branches.sorted.get(self.branches_selected).cloned() else {
                    return;
                };

                match checkout_branch(repo, &mut self.branches.visible_branch_names, &mut self.branches.local, alias, &branch) {
                    Ok(_) => {
                        // Keep graph selection on the commit that owns the checked-out branch.
                        self.graph_selected = self.oids.get_sorted_aliases().iter().position(|o| o == &alias).unwrap_or(0);

                        self.focus = Focus::Viewport;
                        self.reload(None);
                    },
                    Err(error) => self.show_error(format!("Checkout failed: {error}")),
                }
            },

            Focus::Viewport => {
                // The uncommitted pseudo-row has no standalone commit to checkout.
                if self.viewport != Viewport::Graph || self.graph_selected == 0 {
                    return;
                }

                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let oid = self.oids.get_oid_by_alias(alias);

                // Ambiguous commits are checked out through a branch-selection modal.
                let branches_for_alias: Vec<String> = self.branches.all.get(&alias).cloned().unwrap_or_default();

                match branches_for_alias.len() {
                    0 => {
                        // No branch label means detached checkout is the only option.
                        match checkout_head(repo, *oid) {
                            Ok(_) => {
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(format!("Checkout failed: {error}")),
                        }
                    },
                    1 => {
                        // A single label can be checked out without another prompt.
                        match checkout_branch(repo, &mut self.branches.visible_branch_names, &mut self.branches.local, alias, &branches_for_alias[0]) {
                            Ok(_) => {
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(format!("Checkout failed: {error}")),
                        }
                    },
                    _ => {
                        self.focus = Focus::ModalCheckout;
                    },
                }
            },

            _ => (),
        }
    }

    pub fn on_hard_reset(&mut self) {
        if let Some(repo) = &self.repo {
            match self.focus {
                Focus::Viewport => {
                    if self.viewport != Viewport::Graph {
                        return;
                    }
                    let oid = self.oids.get_oid_by_idx(self.graph_selected);
                    match reset_to_commit(repo, *oid, git2::ResetType::Hard) {
                        Ok(_) => {
                            self.reload(None);
                            self.focus = Focus::Viewport;
                        },
                        Err(error) => self.show_error(format!("Hard reset failed: {error}")),
                    }
                },
                Focus::StatusTop | Focus::StatusBottom => {
                    if let Some(file_name) = self.get_selected_file_name() {
                        let path = Path::new(&file_name);
                        match reset_file(repo, path) {
                            Ok(_) => self.reload(None),
                            Err(error) => self.show_error(format!("Reset file failed: {error}")),
                        }
                    }
                },
                _ => {},
            }
        }
    }

    pub fn on_mixed_reset(&mut self) {
        if let Some(repo) = &self.repo
            && self.focus == Focus::Viewport
        {
            if self.focus == Focus::Viewport && self.viewport != Viewport::Graph {
                return;
            }
            let oid = self.oids.get_oid_by_idx(self.graph_selected);
            match reset_to_commit(repo, *oid, git2::ResetType::Mixed) {
                Ok(_) => {
                    self.reload(None);
                    self.focus = Focus::Viewport;
                },
                Err(error) => self.show_error(format!("Mixed reset failed: {error}")),
            }
        }
    }

    pub fn on_unstage(&mut self) {
        if let Some(repo) = &self.repo {
            match self.viewport {
                Viewport::Settings => {},
                _ => {
                    match self.focus {
                        Focus::Viewport => {
                            if self.uncommitted.is_staged {
                                match unstage_all(repo) {
                                    Ok(_) => self.reload(None),
                                    Err(error) => self.show_error(format!("Unstage all failed: {error}")),
                                }
                            }
                        },
                        Focus::StatusTop => {
                            let file: String = {
                                let mut idx = self.status_top_selected;
                                let modified = &self.uncommitted.staged.modified;
                                let added = &self.uncommitted.staged.added;
                                let deleted = &self.uncommitted.staged.deleted;
                                if idx < modified.len() {
                                    modified[idx].clone()
                                } else {
                                    idx -= modified.len();
                                    if idx < added.len() {
                                        added[idx].clone()
                                    } else {
                                        idx -= added.len();
                                        if idx < deleted.len() {
                                            deleted[idx].clone()
                                        } else {
                                            // Selection drifted beyond the staged lists; ignore the command.
                                            return;
                                        }
                                    }
                                }
                            };
                            match unstage_file(repo, Path::new(&file)) {
                                Ok(_) => self.reload(None),
                                Err(error) => self.show_error(format!("Unstage file failed: {error}")),
                            }
                        },
                        _ => {},
                    }
                },
            }
        }
    }

    pub fn on_stage(&mut self) {
        if let Some(repo) = &self.repo {
            match self.viewport {
                Viewport::Settings => {},
                _ => {
                    match self.focus {
                        Focus::Viewport => {
                            if self.uncommitted.is_unstaged {
                                match stage_all(repo) {
                                    Ok(_) => self.reload(None),
                                    Err(error) => self.show_error(format!("Stage all failed: {error}")),
                                }
                            }
                        },
                        Focus::StatusBottom => {
                            let file: String = {
                                let mut idx = self.status_bottom_selected;
                                let modified = &self.uncommitted.unstaged.modified;
                                let added = &self.uncommitted.unstaged.added;
                                let deleted = &self.uncommitted.unstaged.deleted;
                                if idx < modified.len() {
                                    modified[idx].clone()
                                } else {
                                    idx -= modified.len();
                                    if idx < added.len() {
                                        added[idx].clone()
                                    } else {
                                        idx -= added.len();
                                        if idx < deleted.len() {
                                            deleted[idx].clone()
                                        } else {
                                            // Selection drifted beyond the unstaged lists; ignore the command.
                                            return;
                                        }
                                    }
                                }
                            };
                            match stage_file(repo, Path::new(&file)) {
                                Ok(_) => self.reload(None),
                                Err(error) => self.show_error(format!("Stage file failed: {error}")),
                            }
                        },
                        _ => {},
                    }
                },
            }
        }
    }

    pub fn on_commit(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer => {},
            _ => {
                if self.uncommitted.is_staged {
                    self.focus = Focus::ModalCommit;
                }
            },
        }
    }

    pub fn on_force_push(&mut self) {
        if let Some(repo) = &self.repo {
            match self.viewport {
                Viewport::Settings | Viewport::Viewer => {},
                _ => {
                    let repo_path = self.path.as_deref().unwrap_or(".");
                    let Some(branch) = get_current_branch(repo) else {
                        self.show_error("Push failed: detached HEAD has no current branch");
                        return;
                    };
                    let handle = push_over_ssh(repo_path, "origin", branch.as_str(), true);
                    match handle.join() {
                        Ok(Ok(_)) => {
                            self.reload(None);
                        },
                        Ok(Err(error)) => self.show_error(format!("Push failed: {error}")),
                        Err(_) => self.show_error("Push failed: worker thread panicked"),
                    }
                },
            }
        }
    }

    pub fn on_push_tags(&mut self) {
        if self.repo.is_some() {
            match self.viewport {
                Viewport::Settings | Viewport::Viewer => {},
                _ => {
                    let repo_path = self.path.as_deref().unwrap_or(".");
                    let handle = push_tags_over_ssh(repo_path, "origin");
                    match handle.join() {
                        Ok(Ok(_)) => {
                            self.reload(None);
                        },
                        Ok(Err(error)) => self.show_error(format!("Push tags failed: {error}")),
                        Err(_) => self.show_error("Push tags failed: worker thread panicked"),
                    }
                },
            }
        }
    }

    pub fn on_create_branch(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer => {},
            _ => {
                if self.graph_selected != 0 {
                    self.focus = Focus::ModalCreateBranch;
                }
            },
        }
    }

    pub fn on_delete_branch(&mut self) {
        let Some(repo) = &self.repo else { return };

        match self.viewport {
            Viewport::Settings | Viewport::Viewer => return,
            _ => {},
        }

        match self.focus {
            Focus::Branches => {
                let Some((_, branch)) = self.branches.sorted.get(self.branches_selected).cloned() else {
                    return;
                };

                // Deleting the currently checked-out branch would leave HEAD invalid.
                let proceed = match get_current_branch(repo) {
                    Some(current) => current != branch,
                    None => true,
                };

                if proceed {
                    match delete_branch(repo, &branch) {
                        Ok(_) => {
                            // Remove stale filter entries before reload repopulates branches.
                            self.branches.visible_branch_names.remove(&branch);
                            self.reload(None);
                        },
                        Err(error) => self.show_error(format!("Delete branch failed: {error}")),
                    }
                } else {
                    self.show_error("Delete branch failed: cannot delete the current branch");
                }
            },

            Focus::Viewport => {
                if self.graph_selected == 0 {
                    return;
                }

                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let current = get_current_branch(repo);

                // Current branch is excluded so graph deletion cannot remove checked-out HEAD.
                let visible_branches: Vec<_> = self.branches.all.get(&alias).cloned().unwrap_or_default().into_iter().filter(|b| current.as_ref() != Some(b)).collect();

                match visible_branches.len() {
                    0 => {},
                    1 => match delete_branch(repo, &visible_branches[0]) {
                        Ok(_) => {
                            self.branches.visible_branch_names.remove(&visible_branches[0]);
                            self.reload(None);
                        },
                        Err(error) => self.show_error(format!("Delete branch failed: {error}")),
                    },
                    _ => {
                        self.focus = Focus::ModalDeleteBranch;
                    },
                }
            },

            _ => {},
        }
    }

    pub fn on_tag(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer => {},
            _ => {
                if self.focus == Focus::Viewport && self.graph_selected != 0 {
                    self.focus = Focus::ModalTag;
                }
            },
        }
    }

    pub fn on_untag(&mut self) {
        if let Some(repo) = &self.repo {
            match self.viewport {
                Viewport::Settings | Viewport::Viewer => {},
                _ => match self.focus {
                    Focus::Tags => {
                        let Some((_, tag)) = self.tags.sorted.get(self.tags_selected).cloned() else {
                            return;
                        };
                        match untag(repo, &tag) {
                            Ok(_) => self.reload(None),
                            Err(error) => self.show_error(format!("Delete tag failed: {error}")),
                        }
                    },
                    Focus::Viewport => {
                        if self.graph_selected != 0 {
                            let alias = self.oids.get_alias_by_idx(if self.graph_selected == 0 { 1 } else { self.graph_selected });
                            if let Some(tag_names) = self.tags.local.get(&alias) {
                                match tag_names.len() {
                                    0 => {},
                                    1 => match untag(repo, tag_names[0].as_str()) {
                                        Ok(_) => self.reload(None),
                                        Err(error) => self.show_error(format!("Delete tag failed: {error}")),
                                    },
                                    _ => {
                                        self.focus = Focus::ModalDeleteTag;
                                    },
                                }
                            }
                        }
                    },
                    _ => {},
                },
            }
        }
    }

    pub fn on_cherrypick(&mut self) {
        if self.viewport == Viewport::Graph
            && self.focus == Focus::Viewport
            && self.graph_selected != 0
            && let Some(repo) = &self.repo
        {
            let idx = if self.graph_selected == 0 { 1 } else { self.graph_selected };
            let oid = self.oids.get_oid_by_idx(idx);

            // Cherry-pick reloads because it creates a new HEAD commit.
            match cherry_pick_commit(repo, *oid, Some("message"), true) {
                Ok(_) => self.reload(None),
                Err(error) => self.show_error(format!("Cherry-pick failed: {error}")),
            }
        }
    }

    pub fn on_back(&mut self) {
        match self.focus {
            Focus::ModalCommit => {
                self.focus = Focus::Viewport;
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
            Focus::ModalCheckout | Focus::ModalCommit => {
                self.focus = Focus::Viewport;
            },
            _ => {},
        }
    }

    pub fn on_minimize(&mut self) {
        self.layout_config.is_minimal = !self.layout_config.is_minimal;
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
        self.save_layout();
    }

    pub fn on_toggle_branches(&mut self) {
        self.layout_config.is_branches = !self.layout_config.is_branches;
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

    pub fn on_toggle_status(&mut self) {
        self.layout_config.is_status = !self.layout_config.is_status;
        if !self.layout_config.is_status && (self.focus == Focus::StatusTop || self.focus == Focus::StatusBottom) {
            self.focus = Focus::Viewport;
        }
        self.save_layout();
    }

    pub fn on_toggle_inspector(&mut self) {
        self.layout_config.is_inspector = !self.layout_config.is_inspector;
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
