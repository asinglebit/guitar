use crate::{
    app::app::{App, Focus},
    git::actions::{
        branching::create_branch,
        cherrypicking::cherry_pick_commit,
        committing::commit_staged,
        tagging::tag,
        worktrees::{create_worktree, is_valid_worktree_name, lock_worktree},
    },
};
use git2::Oid;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use std::path::PathBuf;

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

    pub(super) fn handle_modal_key_event(&mut self, key_event: KeyEvent) -> bool {
        if self.focus == Focus::ModalError {
            if matches!(key_event.code, KeyCode::Enter | KeyCode::Esc) {
                self.close_error_modal();
            }
            return true;
        }

        if matches!(self.focus, Focus::ModalRebaseConflict | Focus::ModalRebaseSuccess) {
            if matches!(key_event.code, KeyCode::Enter | KeyCode::Esc) {
                self.modal_rebase_message.clear();
                self.focus = Focus::Viewport;
                self.reload(None);
            }
            return true;
        }

        if self.focus == Focus::ModalRebaseProgress {
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
                                Err(error) => self.show_error(format!("Commit failed: {error}")),
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
                            self.show_error("Cherry-pick failed: no commit is pending");
                            return true;
                        };
                        let message = self.modal_input.value().trim().to_string();
                        if message.is_empty() {
                            return true;
                        }

                        // Cherry-pick reloads because it creates a new HEAD commit.
                        match cherry_pick_commit(repo, oid, Some(&message), true) {
                            Ok(_) => {
                                self.modal_input.clear();
                                self.pending_cherrypick_oid = None;
                                self.reload(None);
                                self.focus = Focus::Viewport;
                            },
                            Err(error) => self.show_error(format!("Cherry-pick failed: {error}")),
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
                            self.show_error("Create worktree failed: names cannot be empty or contain path separators");
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
                            self.show_error("Create worktree failed: path cannot be empty");
                            return true;
                        }

                        let oid = self.oids.get_oid_by_idx(self.graph_selected);
                        match create_worktree(&repo, &name, &path, *oid) {
                            Ok(_) => {
                                self.modal_input.clear();
                                self.modal_worktree_name.clear();
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(format!("Create worktree failed: {error}")),
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
                            Err(error) => self.show_error(format!("Lock worktree failed: {error}")),
                        }
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

                        let oid: Option<Oid> = self.oids.oids.iter().find(|oid| oid.to_string().starts_with(sha)).copied();

                        if let Some(oid) = oid {
                            let oid_alias = self.oids.get_alias_by_oid(oid);
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
                true
            },
            _ => false,
        }
    }
}
