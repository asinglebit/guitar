use crate::{
    app::app::{App, AuthInputField, Focus, OperationKind, PendingGraphLookup, Viewport},
    core::graph_service::GraphLookupKind,
    git::actions::{
        branching::{create_branch, rename_branch},
        cherrypicking::{CherrypickOutcome, start_cherrypick},
        committing::commit_staged,
        reverting::{RevertOutcome, start_revert},
        tagging::tag,
        worktrees::{create_worktree, is_valid_worktree_name, lock_worktree},
    },
    git::queries::{diffs::get_filenames_diff_at_oid, files::search_tracked_files},
    helpers::{
        branch_visibility::save_branch_visibility,
        keymap::{KeyBinding, rebind_keymap_selection, save_keymaps, save_keymaps_to_path},
        localisation::{errors, operations},
    },
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

const FILE_SEARCH_RESULT_LIMIT: usize = 50;

impl App {
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

    pub(crate) fn close_key_capture(&mut self) {
        self.modal_key_capture_selection = None;
        self.modal_key_capture_candidate = None;
        self.modal_key_capture_error = None;
        self.focus = Focus::Viewport;
    }

    fn is_key_capture_cancel(key_event: &KeyEvent) -> bool {
        key_event.code == KeyCode::Esc || (matches!(key_event.code, KeyCode::Char('c') | KeyCode::Char('C')) && key_event.modifiers.contains(KeyModifiers::CONTROL))
    }

    fn preview_key_capture_candidate(&mut self, key_binding: KeyBinding) {
        self.modal_key_capture_candidate = Some(key_binding.clone());
        self.modal_key_capture_error = self.modal_key_capture_selection.as_ref().and_then(|selection| {
            let mut preview = self.keymaps.clone();
            rebind_keymap_selection(&mut preview, selection, key_binding).err()
        });
    }

    fn save_keymaps_for_app(&self, keymaps: &crate::helpers::keymap::Keymaps) -> Result<(), Box<dyn std::error::Error>> {
        self.keymap_save_path.as_ref().map_or_else(|| save_keymaps(keymaps), |path| save_keymaps_to_path(path.as_path(), keymaps))
    }

    fn confirm_key_capture(&mut self) {
        if self.modal_key_capture_error.is_some() {
            return;
        }

        let Some(selection) = self.modal_key_capture_selection.clone() else {
            return;
        };
        let Some(candidate) = self.modal_key_capture_candidate.clone() else {
            return;
        };

        let mut updated = self.keymaps.clone();
        match rebind_keymap_selection(&mut updated, &selection, candidate) {
            Ok(_) => match self.save_keymaps_for_app(&updated) {
                Ok(_) => {
                    self.keymaps = updated;
                    self.close_key_capture();
                },
                Err(error) => self.show_error(errors::with_error(errors::SAVE_KEYMAP(), error)),
            },
            Err(error) => {
                self.modal_key_capture_error = Some(error);
            },
        }
    }

    fn handle_key_capture_event(&mut self, key_event: KeyEvent) -> bool {
        if Self::is_key_capture_cancel(&key_event) {
            self.close_key_capture();
            return true;
        }

        if self.modal_key_capture_candidate.is_some() && self.modal_key_capture_error.is_none() && key_event.code == KeyCode::Enter && key_event.modifiers == KeyModifiers::NONE {
            self.confirm_key_capture();
            return true;
        }

        self.preview_key_capture_candidate(KeyBinding::new(key_event.code, key_event.modifiers));
        true
    }

    fn toggle_auth_field(&mut self) {
        if self.pending_auth_prompt.as_ref().is_some_and(|challenge| !challenge.protocol.is_http()) {
            self.auth_input_field = AuthInputField::Secret;
            return;
        }

        self.auth_input_field = match self.auth_input_field {
            AuthInputField::Username => AuthInputField::Secret,
            AuthInputField::Secret => AuthInputField::Username,
        };
    }

    fn handle_auth_event(&mut self, key_event: KeyEvent) -> bool {
        match key_event.code {
            KeyCode::Esc => {
                self.cancel_auth_prompt();
            },
            KeyCode::Enter => {
                self.submit_auth_prompt();
            },
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down => {
                self.toggle_auth_field();
            },
            _ => {
                let input = if self.auth_input_field == AuthInputField::Username { &mut self.auth_username_input } else { &mut self.auth_secret_input };
                input.on_key(key_event);
            },
        }
        true
    }

    fn refresh_file_search_results(&mut self) {
        let Some(repo) = &self.repo else {
            self.modal_file_search_results.clear();
            self.modal_file_search_selected = 0;
            self.modal_file_search_scroll.set(0);
            return;
        };

        self.modal_file_search_results = search_tracked_files(repo, self.modal_input.value(), FILE_SEARCH_RESULT_LIMIT).unwrap_or_default();
        self.modal_file_search_selected = 0;
        self.modal_file_search_scroll.set(0);
    }

    fn close_file_search_modal(&mut self) {
        self.modal_input.clear();
        self.modal_file_search_results.clear();
        self.modal_file_search_selected = 0;
        self.modal_file_search_scroll.set(0);
        self.focus = self.modal_file_search_return_focus;
        self.modal_file_search_return_focus = Focus::Viewport;
    }

    fn move_file_search_selection(&mut self, direction: crate::app::app::Direction) {
        let len = self.modal_file_search_results.len();
        if len == 0 {
            self.modal_file_search_selected = 0;
            return;
        }

        let len = len as i32;
        let current = self.modal_file_search_selected.rem_euclid(len);
        self.modal_file_search_selected = match direction {
            crate::app::app::Direction::Up => (current - 1).rem_euclid(len),
            crate::app::app::Direction::Down => (current + 1).rem_euclid(len),
        };
    }

    fn select_file_search_result(&mut self) {
        let Some(path) = self.modal_file_search_results.get(self.modal_file_search_selected as usize).map(|result| result.path.clone()) else {
            return;
        };

        self.modal_input.clear();
        self.modal_file_search_results.clear();
        self.modal_file_search_selected = 0;
        self.modal_file_search_scroll.set(0);
        self.modal_file_search_return_focus = Focus::Viewport;

        self.layout_config.is_search = true;
        self.mark_viewer_layout_dirty();
        self.save_layout();
        self.focus = Focus::Search;
        self.request_file_history_search(path);
    }

    fn handle_file_search_event(&mut self, key_event: KeyEvent) -> bool {
        match key_event.code {
            KeyCode::Esc => {
                self.close_file_search_modal();
            },
            KeyCode::Enter => {
                self.select_file_search_result();
            },
            KeyCode::Down => {
                self.move_file_search_selection(crate::app::app::Direction::Down);
            },
            KeyCode::Up => {
                self.move_file_search_selection(crate::app::app::Direction::Up);
            },
            KeyCode::Char('j') | KeyCode::Char('J') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_file_search_selection(crate::app::app::Direction::Down);
            },
            KeyCode::Char('k') | KeyCode::Char('K') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_file_search_selection(crate::app::app::Direction::Up);
            },
            _ => {
                self.modal_input.on_key(key_event);
                self.refresh_file_search_results();
            },
        }

        true
    }

    fn confirm_graph_lane_limit_input(&mut self) {
        let Ok(limit) = self.modal_input.value().trim().parse::<usize>() else {
            return;
        };
        if limit == 0 {
            return;
        }

        let selected = self.settings_selected;
        let scroll = self.settings_scroll.get();
        let should_reload = self.repo.is_some() && self.layout_config.graph_lane_limit != limit;
        self.layout_config.graph_lane_limit = limit;
        self.save_layout();
        self.modal_input.clear();
        self.focus = Focus::Viewport;
        if should_reload {
            self.reload(None);
            self.viewport = Viewport::Settings;
            self.focus = Focus::Viewport;
            self.settings_selected = selected;
            self.settings_scroll.set(scroll);
        }
    }

    pub(super) fn handle_modal_key_event(&mut self, key_event: KeyEvent) -> bool {
        if key_event.code == KeyCode::Esc && key_event.modifiers == KeyModifiers::NONE && self.is_dismissible_modal_focus() {
            self.on_back();
            return true;
        }

        if self.focus == Focus::ModalKeyCapture {
            return self.handle_key_capture_event(key_event);
        }

        if self.focus == Focus::ModalAuth {
            return self.handle_auth_event(key_event);
        }

        if self.focus == Focus::ModalError {
            if matches!(key_event.code, KeyCode::Enter | KeyCode::Esc) {
                self.close_error_modal();
            }
            return true;
        }

        if matches!(self.focus, Focus::ModalOperationConflict | Focus::ModalOperationSuccess) {
            if matches!(key_event.code, KeyCode::Enter | KeyCode::Esc) {
                self.modal_operation_message.clear();
                self.focus = Focus::Viewport;
                self.reload(None);
            }
            return true;
        }

        if self.focus == Focus::ModalOperationProgress {
            return true;
        }

        if self.focus == Focus::ModalNetworkProgress {
            return true;
        }

        if self.focus == Focus::ModalFileSearch {
            return self.handle_file_search_event(key_event);
        }

        if self.focus == Focus::ModalGraphLaneLimit {
            match key_event.code {
                KeyCode::Enter => self.confirm_graph_lane_limit_input(),
                _ => self.modal_input.on_key(key_event),
            }
            return true;
        }

        if self.focus == Focus::ModalRemoveWorktree {
            match key_event.code {
                KeyCode::Esc => {
                    self.close_worktree_modal();
                },
                KeyCode::Enter => self.confirm_remove_worktree(),
                _ => {},
            }
            return true;
        }

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
                                Err(error) => self.show_error(errors::with_error(errors::COMMIT(), error)),
                            }
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalCherrypick => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_input.clear();
                        self.pending_cherrypick_oid = None;
                    },
                    KeyCode::Enter => {
                        let Some(repo) = &self.repo else {
                            return true;
                        };
                        let Some(oid) = self.pending_cherrypick_oid else {
                            self.show_error(errors::CHERRYPICK_NO_PENDING());
                            return true;
                        };
                        let message = self.modal_input.value().trim().to_string();
                        if message.is_empty() {
                            return true;
                        }

                        match start_cherrypick(repo, oid, &message) {
                            Ok(CherrypickOutcome::Committed { .. }) => {
                                self.modal_input.clear();
                                self.pending_cherrypick_oid = None;
                                self.reload(None);
                                self.focus = Focus::Viewport;
                            },
                            Ok(CherrypickOutcome::Conflict) => {
                                self.modal_input.clear();
                                self.pending_cherrypick_oid = None;
                                self.show_operation_conflict(crate::app::app::OperationKind::Cherrypick, operations::CHERRYPICK_CONFLICT());
                            },
                            Ok(CherrypickOutcome::Aborted) => {},
                            Err(error) => self.show_error(errors::with_error(errors::CHERRYPICK(), error)),
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalRevert => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_input.clear();
                        self.pending_revert_oid = None;
                    },
                    KeyCode::Enter => {
                        let Some(repo) = &self.repo else {
                            return true;
                        };
                        let Some(oid) = self.pending_revert_oid else {
                            self.show_error(errors::REVERT_NO_PENDING());
                            return true;
                        };
                        let message = self.modal_input.value().trim().to_string();
                        if message.is_empty() {
                            return true;
                        }

                        match start_revert(repo, oid, &message) {
                            Ok(RevertOutcome::Committed { .. }) => {
                                self.modal_input.clear();
                                self.pending_revert_oid = None;
                                self.reload(None);
                                self.focus = Focus::Viewport;
                            },
                            Ok(RevertOutcome::Conflict) => {
                                self.modal_input.clear();
                                self.pending_revert_oid = None;
                                self.show_operation_conflict(OperationKind::Revert, operations::REVERT_CONFLICT());
                            },
                            Ok(RevertOutcome::Aborted) => {},
                            Err(error) => self.show_error(errors::with_error(errors::REVERT(), error)),
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalCreateBranch => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.clear_pending_branch_target();
                        self.modal_input.clear();
                    },
                    KeyCode::Enter => {
                        if let Some(repo) = &self.repo {
                            let Some(oid) = self.selected_branch_target_oid() else {
                                self.show_error(errors::CREATE_BRANCH_NO_COMMIT());
                                return true;
                            };
                            match create_branch(repo, self.modal_input.value(), oid) {
                                Ok(_) => {
                                    self.modal_input.clear();
                                    self.clear_pending_branch_target();
                                    self.reload(None);
                                    self.focus = Focus::Viewport;
                                },
                                Err(error) => self.show_error(errors::with_error(errors::CREATE_BRANCH(), error)),
                            }
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalRenameBranch => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_rename_branch_source = None;
                        self.modal_input.clear();
                    },
                    KeyCode::Enter => {
                        if let Some(repo) = self.repo.clone() {
                            let Some(source) = self.modal_rename_branch_source.clone() else {
                                self.show_error(errors::RENAME_BRANCH_NO_PENDING());
                                return true;
                            };
                            let new_name = self.modal_input.value().trim().to_string();
                            match rename_branch(&repo, &source, &new_name) {
                                Ok(_) => {
                                    if self.branches.hidden_branch_names.contains(source.as_str()) {
                                        self.branches.hidden_branch_names.remove(source.as_str());
                                        self.branches.hidden_branch_names.insert(new_name);
                                        if let Some(path) = &self.path {
                                            save_branch_visibility(path, &self.branches.hidden_branch_names);
                                        }
                                    }
                                    self.modal_input.clear();
                                    self.modal_rename_branch_source = None;
                                    self.reload(None);
                                    self.focus = Focus::Viewport;
                                },
                                Err(error) => self.show_error(errors::with_error(errors::RENAME_BRANCH(), error)),
                            }
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalCreateWorktreeName => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_input.clear();
                        self.modal_worktree_name.clear();
                    },
                    KeyCode::Enter => {
                        let name = self.modal_input.value().trim().to_string();
                        if !is_valid_worktree_name(&name) {
                            self.show_error(errors::CREATE_WORKTREE_INVALID_NAME());
                            return true;
                        }

                        let path = self.default_worktree_path(&name);
                        self.modal_worktree_name = name;
                        self.modal_input.set_value(path);
                        self.focus = Focus::ModalCreateWorktreePath;
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalCreateWorktreePath => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_input.clear();
                        self.modal_worktree_name.clear();
                    },
                    KeyCode::Enter => {
                        let Some(repo) = self.repo.clone() else {
                            return true;
                        };
                        let name = self.modal_worktree_name.clone();
                        let path = PathBuf::from(self.modal_input.value().trim());
                        if path.as_os_str().is_empty() {
                            self.show_error(errors::CREATE_WORKTREE_EMPTY_PATH());
                            return true;
                        }

                        let Some(oid) = self.graph_oid_at(self.graph_selected) else {
                            self.show_error(errors::CREATE_WORKTREE_NO_COMMIT());
                            return true;
                        };
                        match create_worktree(&repo, &name, &path, oid) {
                            Ok(_) => {
                                self.modal_input.clear();
                                self.modal_worktree_name.clear();
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(errors::with_error(errors::CREATE_WORKTREE(), error)),
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalLockWorktree => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Worktrees;
                        self.modal_input.clear();
                    },
                    KeyCode::Enter => {
                        let Some(repo) = self.repo.clone() else {
                            return true;
                        };
                        let Some(entry) = self.worktrees.entries.get(self.worktrees_selected).cloned() else {
                            return true;
                        };
                        let reason = self.modal_input.value().to_string();
                        match lock_worktree(&repo, &entry.name, Some(reason.as_str())) {
                            Ok(_) => {
                                self.modal_input.clear();
                                self.focus = Focus::Worktrees;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(errors::with_error(errors::LOCK_WORKTREE(), error)),
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalRemoteName => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.close_remote_modal();
                    },
                    KeyCode::Enter => {
                        self.confirm_remote_name_input();
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalRemoteUrl => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.close_remote_modal();
                    },
                    KeyCode::Enter => {
                        self.confirm_remote_url_input();
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            Focus::ModalGrep => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Focus::Viewport;
                        self.modal_input.clear();
                    },
                    KeyCode::Enter => {
                        let sha = self.modal_input.value();

                        if sha.is_empty() || sha.len() > 40 {
                            return true;
                        }

                        if self.graph_tx.is_some() {
                            self.request_graph_lookup(GraphLookupKind::ShaPrefix { prefix: sha.to_string() }, PendingGraphLookup::SelectIndex);
                            return true;
                        }

                        if let Some(oid_alias) = self.oids.get_alias_by_prefix(sha) {
                            let next = self.oids.get_sorted_aliases().iter().position(|&alias| alias == oid_alias).unwrap();

                            self.graph_selected = next;
                            self.current_diff.clear();
                            self.current_diff_identity = None;
                            if let Some(repo) = self.repo.clone()
                                && let Some(identity) = self.graph_identity_at(self.graph_selected)
                                && let Some(oid) = self.graph_oid_for_identity(identity)
                            {
                                self.current_diff = get_filenames_diff_at_oid(&repo, oid);
                                self.current_diff_identity = Some(identity);
                            }
                            self.modal_input.clear();
                            self.focus = Focus::Viewport;
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
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

                            if tag_name.is_empty() {
                                return true;
                            }

                            let Some(oid) = self.graph_oid_at(if self.graph_selected == 0 { 1 } else { self.graph_selected }) else {
                                self.show_error(errors::CREATE_TAG_NO_COMMIT());
                                return true;
                            };

                            match tag(repo, oid, tag_name) {
                                Ok(_) => {
                                    self.reload(None);
                                    self.modal_input.clear();
                                    self.focus = Focus::Viewport;
                                },
                                Err(error) => self.show_error(errors::with_error(errors::CREATE_TAG(), error)),
                            }
                        }
                    },
                    _ => {
                        self.modal_input.on_key(key_event);
                    },
                }
                true
            },
            _ => false,
        }
    }

    fn is_dismissible_modal_focus(&self) -> bool {
        matches!(
            self.focus,
            Focus::ModalCheckout
                | Focus::ModalSolo
                | Focus::ModalCommit
                | Focus::ModalCherrypick
                | Focus::ModalRevert
                | Focus::ModalCreateBranch
                | Focus::ModalRenameBranch
                | Focus::ModalCreateWorktreeName
                | Focus::ModalCreateWorktreePath
                | Focus::ModalDeleteBranch
                | Focus::ModalWorktreeChooser
                | Focus::ModalRemoveWorktree
                | Focus::ModalLockWorktree
                | Focus::ModalRemoteAction
                | Focus::ModalRemoteDelete
                | Focus::ModalRemoteName
                | Focus::ModalRemoteUrl
                | Focus::ModalGraphLaneLimit
                | Focus::ModalGrep
                | Focus::ModalFileSearch
                | Focus::ModalTag
                | Focus::ModalDeleteTag
                | Focus::ModalKeyCapture
                | Focus::ModalAuth
        )
    }
}

#[cfg(test)]
#[path = "../../tests/app/input/modals.rs"]
mod tests;
