use crate::{
    app::app::{App, AuthInputField, BranchModalAction, Focus, OperationKind, PendingOperationAction, Viewport},
    core::graph_service::GraphPaneRow,
    git::{
        actions::{
            branching::delete_branch,
            checkout::{checkout_branch, checkout_head},
            cherrypicking::{CherrypickOutcome, abort_cherrypick, continue_cherrypick, is_cherrypick_in_progress},
            merging::{MergeOutcome, abort_merge, continue_merge, is_merge_in_progress, start_merge},
            network::NetworkRequest,
            rebasing::{RebaseOutcome, abort_rebase, continue_rebase, is_rebase_in_progress, start_rebase},
            resetting::{reset_file, reset_to_commit},
            reverting::{RevertOutcome, abort_revert, continue_revert, is_revert_in_progress},
            staging::{stage_all, stage_file, unstage_all, unstage_file},
            stashing::{pop, stash},
            submodules::{stage_submodule_head, unstage_submodule},
            tagging::untag,
        },
        auth::{AuthRequired, AuthSecret, NetworkResult},
        queries::{commits::get_current_branch, remotes::effective_default_remote, submodules::submodules_if_present},
    },
    helpers::{
        branch_visibility::save_branch_visibility,
        localisation::{errors, network, operations},
    },
};
use git2::{BranchType, Repository, RepositoryState};
use std::path::Path;

impl App {
    const MAX_AUTH_ATTEMPTS: usize = 3;

    fn submodule_name_for_status_path(repo: &Repository, path: &str) -> Option<String> {
        let target = Path::new(path);
        submodules_if_present(repo).ok()?.into_iter().find(|submodule| submodule.path() == target).map(|submodule| submodule.name().map(str::to_string).unwrap_or_else(|| path.to_string()))
    }

    pub(crate) fn start_network_request(&mut self, request: NetworkRequest) {
        if self.network_handle.is_some() {
            self.show_error(errors::GIT_NETWORK_ALREADY_RUNNING());
            return;
        }

        self.pending_network_request = Some(request);
        self.network_auth_attempts = 0;
        self.spawn_pending_network_request();
    }

    pub(crate) fn retry_pending_network_request(&mut self) {
        self.network_auth_attempts = self.network_auth_attempts.saturating_add(1);
        self.spawn_pending_network_request();
    }

    fn spawn_pending_network_request(&mut self) {
        let Some(request) = self.pending_network_request.clone() else {
            return;
        };
        self.modal_network_title = request.label().to_string();
        self.modal_network_message = request.progress_message();
        self.focus = Focus::ModalNetworkProgress;
        self.network_handle = Some(request.spawn(self.auth_session.clone()));
    }

    pub fn poll_network_request(&mut self) {
        let is_finished = self.network_handle.as_ref().is_some_and(|handle| handle.is_finished());
        if !is_finished {
            return;
        }

        let Some(handle) = self.network_handle.take() else {
            return;
        };

        match handle.join() {
            Ok(result) => self.handle_network_result(result),
            Err(_) => self.finish_network_failure(errors::GIT_NETWORK_PANICKED().to_string()),
        }
    }

    pub(crate) fn handle_network_result(&mut self, result: NetworkResult) {
        match result {
            NetworkResult::Success => {
                let completed_request = self.pending_network_request.take();
                self.network_auth_attempts = 0;
                self.pending_auth_prompt = None;
                self.auth_username_input.clear();
                self.auth_secret_input.clear();
                self.modal_network_title.clear();
                self.modal_network_message.clear();
                if let Some(NetworkRequest::DeleteRemoteBranch { remote_name, branch, .. }) = completed_request {
                    let hidden_name = format!("{remote_name}/{branch}");
                    if self.branches.hidden_branch_names.contains(hidden_name.as_str()) {
                        self.branches.hidden_branch_names.remove(hidden_name.as_str());
                        if let Some(path) = &self.path {
                            save_branch_visibility(path, &self.branches.hidden_branch_names);
                        }
                    }
                }
                self.focus = Focus::Viewport;
                self.reload(None);
            },
            NetworkResult::AuthRequired(AuthRequired { challenge, rejected }) => {
                self.auth_session.evict(&rejected);
                if self.network_auth_attempts >= Self::MAX_AUTH_ATTEMPTS {
                    self.finish_network_failure(errors::authentication_failed(&challenge.operation, Self::MAX_AUTH_ATTEMPTS));
                    return;
                }

                self.pending_auth_prompt = Some(challenge.clone());
                self.auth_username_input.clear();
                self.auth_secret_input.clear();
                if let Some(username) = challenge.username {
                    self.auth_username_input.set_value(username);
                }
                self.auth_input_field = if challenge.protocol.is_http() && self.auth_username_input.value().is_empty() { AuthInputField::Username } else { AuthInputField::Secret };
                self.focus = Focus::ModalAuth;
            },
            NetworkResult::Failure(message) => self.finish_network_failure(message),
        }
    }

    fn finish_network_failure(&mut self, message: String) {
        self.pending_network_request = None;
        self.network_auth_attempts = 0;
        self.pending_auth_prompt = None;
        self.auth_username_input.clear();
        self.auth_secret_input.clear();
        self.modal_network_title.clear();
        self.modal_network_message.clear();
        self.focus = Focus::Viewport;
        self.show_error(message);
    }

    pub(crate) fn cancel_auth_prompt(&mut self) {
        let operation = self.pending_auth_prompt.as_ref().map(|challenge| challenge.operation.clone()).unwrap_or_else(|| network::GIT_NETWORK_OPERATION().to_string());
        self.pending_network_request = None;
        self.network_auth_attempts = 0;
        self.pending_auth_prompt = None;
        self.auth_username_input.clear();
        self.auth_secret_input.clear();
        self.modal_network_title.clear();
        self.modal_network_message.clear();
        self.focus = Focus::Viewport;
        self.show_error(errors::auth_cancelled(&operation));
    }

    pub(crate) fn submit_auth_prompt(&mut self) {
        let Some(challenge) = self.pending_auth_prompt.clone() else {
            return;
        };

        let secret = if challenge.protocol.is_http() {
            let username = self.auth_username_input.value().trim().to_string();
            let password = self.auth_secret_input.value().to_string();
            if username.is_empty() || password.is_empty() {
                return;
            }
            AuthSecret::Https { username, password }
        } else {
            let passphrase = self.auth_secret_input.value().to_string();
            if passphrase.is_empty() {
                return;
            }
            AuthSecret::SshKeyPassphrase { passphrase }
        };

        self.auth_session.store(&challenge, secret);
        self.pending_auth_prompt = None;
        self.auth_username_input.clear();
        self.auth_secret_input.clear();
        self.retry_pending_network_request();
    }

    pub fn run_pending_operation_action(&mut self) {
        let Some(action) = self.pending_operation_action.take() else {
            return;
        };
        let Some(path) = self.repo.as_ref().map(|repo| repo.path().to_path_buf()) else {
            self.focus = Focus::Viewport;
            self.show_error(errors::GIT_OPERATION_NO_REPOSITORY());
            return;
        };

        let repo = match Repository::open(path) {
            Ok(repo) => repo,
            Err(error) => {
                self.focus = Focus::Viewport;
                self.show_error(errors::with_error(errors::OPEN_REPOSITORY(), error));
                return;
            },
        };

        match action {
            PendingOperationAction::Start { kind: OperationKind::Rebase, oid } => self.handle_rebase_result(start_rebase(&repo, oid)),
            PendingOperationAction::Start { kind: OperationKind::Merge, oid } => self.handle_merge_result(start_merge(&repo, oid)),
            PendingOperationAction::Start { kind: OperationKind::Cherrypick, .. } => {
                self.focus = Focus::Viewport;
                self.show_error(errors::CHERRYPICK_NO_MESSAGE());
            },
            PendingOperationAction::Start { kind: OperationKind::Revert, .. } => {
                self.focus = Focus::Viewport;
                self.show_error(errors::REVERT_NO_MESSAGE());
            },
            PendingOperationAction::Continue => self.continue_active_operation(&repo),
            PendingOperationAction::Abort => self.abort_active_operation(&repo),
        }
    }

    fn handle_rebase_result(&mut self, result: Result<RebaseOutcome, git2::Error>) {
        self.modal_operation_kind = OperationKind::Rebase;
        match result {
            Ok(RebaseOutcome::Completed { applied }) => {
                self.modal_operation_message = operations::rebase_completed(applied);
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(RebaseOutcome::Conflict) => {
                self.show_operation_conflict(OperationKind::Rebase, operations::REBASE_CONFLICT());
            },
            Ok(RebaseOutcome::Aborted) => {
                self.modal_operation_message = operations::REBASE_ABORTED().to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Err(error) => {
                self.modal_operation_message.clear();
                self.focus = Focus::Viewport;
                self.show_error(errors::with_error(errors::REBASE(), error));
                self.reload(None);
            },
        }
    }

    fn handle_cherrypick_result(&mut self, result: Result<CherrypickOutcome, git2::Error>) {
        self.modal_operation_kind = OperationKind::Cherrypick;
        match result {
            Ok(CherrypickOutcome::Committed { .. }) => {
                self.modal_operation_message = operations::CHERRYPICK_COMPLETED().to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(CherrypickOutcome::Conflict) => {
                self.show_operation_conflict(OperationKind::Cherrypick, operations::CHERRYPICK_CONFLICT());
            },
            Ok(CherrypickOutcome::Aborted) => {
                self.modal_operation_message = operations::CHERRYPICK_ABORTED().to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Err(error) => {
                self.modal_operation_message.clear();
                self.focus = Focus::Viewport;
                self.show_error(errors::with_error(errors::CHERRYPICK(), error));
                self.reload(None);
            },
        }
    }

    pub(crate) fn handle_revert_result(&mut self, result: Result<RevertOutcome, git2::Error>) {
        self.modal_operation_kind = OperationKind::Revert;
        match result {
            Ok(RevertOutcome::Committed { .. }) => {
                self.modal_operation_message = operations::REVERT_COMPLETED().to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(RevertOutcome::Conflict) => {
                self.show_operation_conflict(OperationKind::Revert, operations::REVERT_CONFLICT());
            },
            Ok(RevertOutcome::Aborted) => {
                self.modal_operation_message = operations::REVERT_ABORTED().to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Err(error) => {
                self.modal_operation_message.clear();
                self.focus = Focus::Viewport;
                self.show_error(errors::with_error(errors::REVERT(), error));
                self.reload(None);
            },
        }
    }

    fn handle_merge_result(&mut self, result: Result<MergeOutcome, git2::Error>) {
        self.modal_operation_kind = OperationKind::Merge;
        match result {
            Ok(MergeOutcome::Completed { .. }) => {
                self.modal_operation_message = operations::MERGE_COMPLETED().to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(MergeOutcome::FastForward { .. }) => {
                self.modal_operation_message = operations::MERGE_FAST_FORWARDED().to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(MergeOutcome::UpToDate) => {
                self.modal_operation_message = operations::MERGE_ALREADY_UP_TO_DATE().to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(MergeOutcome::Conflict) => {
                self.show_operation_conflict(OperationKind::Merge, operations::MERGE_CONFLICT());
            },
            Ok(MergeOutcome::Aborted) => {
                self.modal_operation_message = operations::MERGE_ABORTED().to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Err(error) => {
                self.modal_operation_message.clear();
                self.focus = Focus::Viewport;
                self.show_error(errors::with_error(errors::MERGE(), error));
                self.reload(None);
            },
        }
    }

    pub fn show_operation_conflict(&mut self, kind: OperationKind, message: impl Into<String>) {
        self.modal_operation_kind = kind;
        self.modal_operation_message = message.into();
        self.focus = Focus::ModalOperationConflict;
        self.reload(None);
    }

    pub(crate) fn active_operation_kind(repo: &Repository) -> Option<OperationKind> {
        match repo.state() {
            RepositoryState::Rebase | RepositoryState::RebaseInteractive | RepositoryState::RebaseMerge | RepositoryState::ApplyMailboxOrRebase => Some(OperationKind::Rebase),
            RepositoryState::CherryPick | RepositoryState::CherryPickSequence => Some(OperationKind::Cherrypick),
            RepositoryState::Revert | RepositoryState::RevertSequence => Some(OperationKind::Revert),
            RepositoryState::Merge => Some(OperationKind::Merge),
            _ => None,
        }
    }

    fn continue_active_operation(&mut self, repo: &Repository) {
        match Self::active_operation_kind(repo) {
            Some(OperationKind::Rebase) => self.handle_rebase_result(continue_rebase(repo)),
            Some(OperationKind::Cherrypick) => self.handle_cherrypick_result(continue_cherrypick(repo)),
            Some(OperationKind::Revert) => self.handle_revert_result(continue_revert(repo)),
            Some(OperationKind::Merge) => self.handle_merge_result(continue_merge(repo)),
            None => {
                self.focus = Focus::Viewport;
                self.show_error(errors::CONTINUE_NO_OPERATION());
            },
        }
    }

    fn abort_active_operation(&mut self, repo: &Repository) {
        match Self::active_operation_kind(repo) {
            Some(OperationKind::Rebase) => self.handle_rebase_result(abort_rebase(repo)),
            Some(OperationKind::Cherrypick) => self.handle_cherrypick_result(abort_cherrypick(repo)),
            Some(OperationKind::Revert) => self.handle_revert_result(abort_revert(repo)),
            Some(OperationKind::Merge) => self.handle_merge_result(abort_merge(repo)),
            None => {
                self.focus = Focus::Viewport;
                self.show_error(errors::ABORT_NO_OPERATION());
            },
        }
    }

    pub fn on_drop(&mut self) {
        if self.repo.is_none() || self.viewport != Viewport::Graph {
            return;
        }

        let oid = match self.focus {
            Focus::Viewport => {
                let Some(row) = self.graph_row_at(self.graph_selected) else {
                    return;
                };
                if !row.is_stash {
                    return;
                }
                row.oid
            },
            Focus::Stashes => {
                let Some(alias) = self.stash_alias_at_pane_selection() else {
                    return;
                };
                *self.oids.get_oid_by_alias(alias)
            },
            _ => return,
        };

        let Some(path) = self.repo.as_ref().map(|repo| repo.path().to_path_buf()) else {
            return;
        };
        let mut repo = match Repository::open(path) {
            Ok(repo) => repo,
            Err(error) => {
                self.show_error(errors::with_error(errors::OPEN_REPOSITORY(), error));
                return;
            },
        };
        match pop(&mut repo, &oid, false) {
            Ok(_) => self.reload(None),
            Err(error) => self.show_error(errors::with_error(errors::DROP_STASH(), error)),
        }
    }

    pub fn on_pop(&mut self) {
        if self.repo.is_none() || self.viewport != Viewport::Graph {
            return;
        }

        let oid = match self.focus {
            Focus::Viewport => {
                let Some(row) = self.graph_row_at(self.graph_selected) else {
                    return;
                };
                if !row.is_stash {
                    return;
                }
                row.oid
            },
            Focus::Stashes => {
                let Some(alias) = self.stash_alias_at_pane_selection() else {
                    return;
                };
                *self.oids.get_oid_by_alias(alias)
            },
            _ => return,
        };

        let Some(path) = self.repo.as_ref().map(|repo| repo.path().to_path_buf()) else {
            return;
        };
        let mut repo = match Repository::open(path) {
            Ok(repo) => repo,
            Err(error) => {
                self.show_error(errors::with_error(errors::OPEN_REPOSITORY(), error));
                return;
            },
        };
        match pop(&mut repo, &oid, true) {
            Ok(_) => self.reload(None),
            Err(error) => self.show_error(errors::with_error(errors::POP_STASH(), error)),
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
                    self.show_error(errors::with_error(errors::OPEN_REPOSITORY(), error));
                    return;
                },
            };

            match stash(&mut repo) {
                Ok(_) => self.reload(None),
                Err(error) => self.show_error(errors::with_error(errors::STASH(), error)),
            }
        }
    }

    pub fn on_find(&mut self) {
        if self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            self.focus = Focus::ModalGrep;
        }
    }

    pub fn on_find_file(&mut self) {
        if self.repo.is_none() || matches!(self.viewport, Viewport::Splash | Viewport::Settings) {
            return;
        }

        if !matches!(
            self.focus,
            Focus::Viewport
                | Focus::Inspector
                | Focus::StatusTop
                | Focus::StatusBottom
                | Focus::Search
                | Focus::Branches
                | Focus::Tags
                | Focus::Stashes
                | Focus::Reflogs
                | Focus::Worktrees
                | Focus::Submodules
        ) {
            return;
        }

        self.modal_file_search_return_focus = self.focus;
        self.modal_input.clear();
        self.modal_file_search_results.clear();
        self.modal_file_search_selected = 0;
        self.modal_file_search_scroll.set(0);
        self.focus = Focus::ModalFileSearch;
    }

    pub fn on_fetch_all(&mut self) {
        if self.viewport != Viewport::Settings {
            let Some(remote_name) = self.default_remote_for_network(network::FETCH()) else {
                return;
            };
            let repo_path = self.path.as_deref().unwrap_or(".");
            self.start_network_request(NetworkRequest::Fetch { repo_path: repo_path.to_string(), remote_name });
        }
    }

    pub fn on_checkout(&mut self) {
        let Some(repo) = &self.repo else { return };

        match self.focus {
            Focus::Branches => {
                // Branch pane checkout uses the selected row directly.
                let projected = self.graph.branches_window.as_ref().and_then(|window| {
                    if self.branches_selected >= window.start
                        && self.branches_selected < window.end
                        && let Some(GraphPaneRow::Branch { alias, name, graph_index, .. }) = window.rows.get(self.branches_selected - window.start)
                    {
                        Some((*alias, name.clone(), *graph_index))
                    } else {
                        None
                    }
                });
                let Some((alias, branch, graph_index)) = projected.or_else(|| self.branches.sorted.get(self.branches_selected).cloned().map(|(alias, branch)| (alias, branch, None))) else {
                    return;
                };

                match checkout_branch(repo, &mut self.branches.hidden_branch_names, &mut self.branches.local, alias, &branch) {
                    Ok(_) => {
                        // Keep graph selection on the commit that owns the checked-out branch.
                        self.graph_selected = graph_index.or_else(|| self.oids.get_sorted_aliases().iter().position(|o| o == &alias)).unwrap_or(0);

                        if let Some(path) = &self.path {
                            save_branch_visibility(path, &self.branches.hidden_branch_names);
                        }
                        self.focus = Focus::Viewport;
                        self.reload(None);
                    },
                    Err(error) => self.show_error(errors::with_error(errors::CHECKOUT(), error)),
                }
            },

            Focus::Viewport => {
                // The uncommitted pseudo-row has no standalone commit to checkout.
                if self.viewport != Viewport::Graph || self.graph_selected == 0 {
                    return;
                }

                let Some(alias) = self.graph_alias_at(self.graph_selected) else {
                    return;
                };
                let Some(oid) = self.graph_oid_at(self.graph_selected) else {
                    return;
                };

                // Ambiguous commits are checked out through a branch-selection modal.
                let branches_for_alias = self.graph_branch_choices(alias);

                match branches_for_alias.len() {
                    0 => {
                        // No branch label means detached checkout is the only option.
                        match checkout_head(repo, oid) {
                            Ok(_) => {
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(errors::with_error(errors::CHECKOUT(), error)),
                        }
                    },
                    1 => {
                        // A single label can be checked out without another prompt.
                        match checkout_branch(repo, &mut self.branches.hidden_branch_names, &mut self.branches.local, alias, &branches_for_alias[0]) {
                            Ok(_) => {
                                if let Some(path) = &self.path {
                                    save_branch_visibility(path, &self.branches.hidden_branch_names);
                                }
                                self.focus = Focus::Viewport;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(errors::with_error(errors::CHECKOUT(), error)),
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
                    let Some(oid) = self.graph_oid_at(self.graph_selected) else {
                        return;
                    };
                    match reset_to_commit(repo, oid, git2::ResetType::Hard) {
                        Ok(_) => {
                            self.reload(None);
                            self.focus = Focus::Viewport;
                        },
                        Err(error) => self.show_error(errors::with_error(errors::HARD_RESET(), error)),
                    }
                },
                Focus::StatusTop | Focus::StatusBottom => {
                    if let Some(file_name) = self.get_selected_file_name() {
                        let path = Path::new(&file_name);
                        match reset_file(repo, path) {
                            Ok(_) => self.reload(None),
                            Err(error) => self.show_error(errors::with_error(errors::RESET_FILE(), error)),
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
            let Some(oid) = self.graph_oid_at(self.graph_selected) else {
                return;
            };
            match reset_to_commit(repo, oid, git2::ResetType::Mixed) {
                Ok(_) => {
                    self.reload(None);
                    self.focus = Focus::Viewport;
                },
                Err(error) => self.show_error(errors::with_error(errors::MIXED_RESET(), error)),
            }
        }
    }

    pub fn on_unstage(&mut self) {
        if let Some(repo) = &self.repo {
            match self.viewport {
                Viewport::Settings => {},
                _ => match self.focus {
                    Focus::Viewport => {
                        if self.uncommitted.is_staged {
                            match unstage_all(repo) {
                                Ok(_) => self.reload(None),
                                Err(error) => self.show_error(errors::with_error(errors::UNSTAGE_ALL(), error)),
                            }
                        }
                    },
                    Focus::StatusTop => {
                        if self.selected_staged_status_file_is_conflict() {
                            self.show_error(errors::UNSTAGE_FILE_CONFLICT());
                            return;
                        }
                        let Some(file) = self.selected_staged_status_file_name() else {
                            return;
                        };
                        if let Some(name) = Self::submodule_name_for_status_path(repo, &file) {
                            match unstage_submodule(repo, &name) {
                                Ok(_) => self.reload(None),
                                Err(error) => self.show_error(errors::with_error(errors::UNSTAGE_SUBMODULE(), error)),
                            }
                            return;
                        }
                        match unstage_file(repo, Path::new(&file)) {
                            Ok(_) => self.reload(None),
                            Err(error) => self.show_error(errors::with_error(errors::UNSTAGE_FILE(), error)),
                        }
                    },
                    Focus::Submodules => {
                        let Some(name) = self.selected_submodule_name() else {
                            return;
                        };
                        match unstage_submodule(repo, &name) {
                            Ok(_) => {
                                self.focus = Focus::Submodules;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(errors::with_error(errors::UNSTAGE_SUBMODULE(), error)),
                        }
                    },
                    _ => {},
                },
            }
        }
    }

    pub fn on_stage(&mut self) {
        if let Some(repo) = &self.repo {
            match self.viewport {
                Viewport::Settings => {},
                _ => match self.focus {
                    Focus::Viewport => {
                        if self.uncommitted.is_unstaged {
                            match stage_all(repo) {
                                Ok(_) => self.reload(None),
                                Err(error) => self.show_error(errors::with_error(errors::STAGE_ALL(), error)),
                            }
                        }
                    },
                    Focus::StatusBottom => {
                        if self.selected_unstaged_status_file_is_conflict() {
                            self.show_error(errors::STAGE_FILE_CONFLICT());
                            return;
                        }
                        let Some(file) = self.selected_unstaged_status_file_name() else {
                            return;
                        };
                        if let Some(name) = Self::submodule_name_for_status_path(repo, &file) {
                            match stage_submodule_head(repo, &name) {
                                Ok(_) => self.reload(None),
                                Err(error) => self.show_error(errors::with_error(errors::STAGE_SUBMODULE(), error)),
                            }
                            return;
                        }
                        match stage_file(repo, Path::new(&file)) {
                            Ok(_) => self.reload(None),
                            Err(error) => self.show_error(errors::with_error(errors::STAGE_FILE(), error)),
                        }
                    },
                    Focus::Submodules => {
                        let Some(name) = self.selected_submodule_name() else {
                            return;
                        };
                        match stage_submodule_head(repo, &name) {
                            Ok(_) => {
                                self.focus = Focus::Submodules;
                                self.reload(None);
                            },
                            Err(error) => self.show_error(errors::with_error(errors::STAGE_SUBMODULE(), error)),
                        }
                    },
                    _ => {},
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
        if let Some(repo) = self.repo.clone() {
            match self.viewport {
                Viewport::Settings | Viewport::Viewer => {},
                _ => {
                    let repo_path = self.path.as_deref().unwrap_or(".").to_string();
                    let Some(branch) = get_current_branch(&repo) else {
                        self.show_error(errors::PUSH_DETACHED_HEAD());
                        return;
                    };
                    let Some(remote_name) = self.default_remote_for_network(network::PUSH()) else {
                        return;
                    };
                    self.start_network_request(NetworkRequest::PushBranch { repo_path, remote_name, branch, force: true });
                },
            }
        }
    }

    pub fn on_push_tags(&mut self) {
        if self.repo.is_some() {
            match self.viewport {
                Viewport::Settings | Viewport::Viewer => {},
                _ => {
                    let Some(remote_name) = self.default_remote_for_network(network::PUSH_TAGS()) else {
                        return;
                    };
                    let repo_path = self.path.as_deref().unwrap_or(".");
                    self.start_network_request(NetworkRequest::PushTags { repo_path: repo_path.to_string(), remote_name });
                },
            }
        }
    }

    fn default_remote_for_network(&mut self, operation: &str) -> Option<String> {
        let Some(repo) = self.repo.clone() else {
            return None;
        };

        match effective_default_remote(&repo) {
            Some(remote_name) => Some(remote_name),
            None => {
                self.show_error(errors::no_remotes_configured(operation));
                None
            },
        }
    }

    pub fn on_create_branch(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer => {},
            _ => match self.focus {
                Focus::Reflogs => {
                    if let Some(entry) = self.reflogs.entries.get(self.reflogs_selected) {
                        self.pending_branch_target_oid = Some(entry.new_oid);
                        self.focus = Focus::ModalCreateBranch;
                    }
                },
                _ => {
                    if self.graph_selected != 0 {
                        self.pending_branch_target_oid = None;
                        self.focus = Focus::ModalCreateBranch;
                    }
                },
            },
        }
    }

    pub fn selected_branch_target_oid(&self) -> Option<git2::Oid> {
        if let Some(oid) = self.pending_branch_target_oid {
            return Some(oid);
        }

        if self.graph_selected != 0 { self.graph_oid_at(self.graph_selected) } else { None }
    }

    pub fn clear_pending_branch_target(&mut self) {
        self.pending_branch_target_oid = None;
    }

    pub(crate) fn open_branch_rename_modal(&mut self, branch: String) {
        self.modal_input.set_value(branch.clone());
        self.modal_rename_branch_source = Some(branch);
        self.focus = Focus::ModalRenameBranch;
    }

    pub fn on_rename_branch(&mut self) {
        let Some(repo) = self.repo.clone() else { return };

        match self.viewport {
            Viewport::Settings | Viewport::Viewer => return,
            _ => {},
        }

        match self.focus {
            Focus::Branches => {
                let Some(branch) = self.branch_name_at_pane_selection() else {
                    return;
                };

                if repo.find_branch(&branch, BranchType::Local).is_ok() {
                    self.open_branch_rename_modal(branch);
                } else {
                    self.show_error(errors::RENAME_BRANCH_LOCAL_ONLY());
                }
            },
            Focus::Viewport => {
                if self.viewport != Viewport::Graph || self.graph_selected == 0 {
                    return;
                }

                let Some(alias) = self.graph_alias_at(self.graph_selected) else {
                    return;
                };
                let branch_names = self.graph_branch_choices(alias);
                if branch_names.is_empty() {
                    return;
                }

                let local_branch_names = self.graph_local_branch_choices(alias);
                match local_branch_names.as_slice() {
                    [] => self.show_error(errors::RENAME_BRANCH_LOCAL_ONLY()),
                    [branch] => self.open_branch_rename_modal(branch.clone()),
                    _ => {
                        self.modal_branch_action = BranchModalAction::Rename;
                        self.modal_solo_selected = 0;
                        self.focus = Focus::ModalSolo;
                    },
                }
            },
            _ => {},
        }
    }

    pub(crate) fn delete_branch_from_ui(&mut self, branch: &str) {
        let Some(repo) = self.repo.clone() else {
            return;
        };

        if repo.find_branch(branch, BranchType::Local).is_ok() {
            match delete_branch(&repo, branch) {
                Ok(_) => {
                    if self.branches.hidden_branch_names.contains(branch) {
                        self.branches.hidden_branch_names.remove(branch);
                        if let Some(path) = &self.path {
                            save_branch_visibility(path, &self.branches.hidden_branch_names);
                        }
                    }
                    self.modal_delete_branch_selected = 0;
                    self.focus = Focus::Viewport;
                    self.reload(None);
                },
                Err(error) => self.show_error(errors::with_error(errors::DELETE_BRANCH(), error)),
            }
            return;
        }

        let (remote_name, remote_branch) = branch.split_once('/').unwrap_or(("origin", branch));
        if remote_name.is_empty() || remote_branch.is_empty() {
            self.show_error(errors::DELETE_BRANCH_INVALID_REMOTE());
            return;
        }

        let repo_path = self.path.as_deref().unwrap_or(".");
        self.modal_delete_branch_selected = 0;
        self.start_network_request(NetworkRequest::DeleteRemoteBranch { repo_path: repo_path.to_string(), remote_name: remote_name.to_string(), branch: remote_branch.to_string() });
    }

    pub fn on_delete_branch(&mut self) {
        let Some(repo) = &self.repo else { return };

        match self.viewport {
            Viewport::Settings | Viewport::Viewer => return,
            _ => {},
        }

        match self.focus {
            Focus::Branches => {
                let projected = self.graph.branches_window.as_ref().and_then(|window| {
                    if self.branches_selected >= window.start
                        && self.branches_selected < window.end
                        && let Some(GraphPaneRow::Branch { name, .. }) = window.rows.get(self.branches_selected - window.start)
                    {
                        Some(name.clone())
                    } else {
                        None
                    }
                });
                let Some(branch) = projected.or_else(|| self.branches.sorted.get(self.branches_selected).map(|(_, branch)| branch.clone())) else {
                    return;
                };

                // Deleting the currently checked-out branch would leave HEAD invalid.
                let proceed = match get_current_branch(repo) {
                    Some(current) => current != branch,
                    None => true,
                };

                if proceed {
                    self.delete_branch_from_ui(&branch);
                } else {
                    self.show_error(errors::DELETE_BRANCH_CURRENT());
                }
            },

            Focus::Viewport => {
                if self.graph_selected == 0 {
                    return;
                }

                let Some(alias) = self.graph_alias_at(self.graph_selected) else {
                    return;
                };
                let current = get_current_branch(repo);

                // Current branch is excluded so graph deletion cannot remove checked-out HEAD.
                let branch_names = self.graph_deletable_branch_choices(alias, current.as_deref());

                match branch_names.len() {
                    0 => {},
                    1 => self.delete_branch_from_ui(&branch_names[0]),
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
                        let projected = self.graph.tags_window.as_ref().and_then(|window| {
                            if self.tags_selected >= window.start
                                && self.tags_selected < window.end
                                && let Some(GraphPaneRow::Tag { name, .. }) = window.rows.get(self.tags_selected - window.start)
                            {
                                Some(name.clone())
                            } else {
                                None
                            }
                        });
                        let Some(tag) = projected.or_else(|| self.tags.sorted.get(self.tags_selected).map(|(_, tag)| tag.clone())) else {
                            return;
                        };
                        match untag(repo, &tag) {
                            Ok(_) => self.reload(None),
                            Err(error) => self.show_error(errors::with_error(errors::DELETE_TAG(), error)),
                        }
                    },
                    Focus::Viewport => {
                        if self.graph_selected != 0 {
                            let tag_names: Vec<String> = self
                                .graph_row_at(if self.graph_selected == 0 { 1 } else { self.graph_selected })
                                .map(|row| row.tags.iter().map(|tag| tag.name.clone()).collect())
                                .or_else(|| self.graph_alias_at(if self.graph_selected == 0 { 1 } else { self.graph_selected }).map(|alias| self.tags.local.get(&alias).cloned().unwrap_or_default()))
                                .unwrap_or_default();
                            match tag_names.len() {
                                0 => {},
                                1 => match untag(repo, tag_names[0].as_str()) {
                                    Ok(_) => self.reload(None),
                                    Err(error) => self.show_error(errors::with_error(errors::DELETE_TAG(), error)),
                                },
                                _ => {
                                    self.focus = Focus::ModalDeleteTag;
                                },
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
            let Some(oid) = self.graph_oid_at(idx) else {
                return;
            };

            let original_message = match repo.find_commit(oid) {
                Ok(commit) => Ok(commit.summary().unwrap_or(operations::CHERRYPICK_COMMIT_FALLBACK()).to_string()),
                Err(error) => Err(error),
            };

            match original_message {
                Ok(original_message) => {
                    self.pending_cherrypick_oid = Some(oid);
                    self.modal_input.set_value(operations::cherrypicked(&original_message));
                    self.focus = Focus::ModalCherrypick;
                },
                Err(error) => self.show_error(errors::with_error(errors::CHERRYPICK(), error)),
            }
        }
    }

    pub fn on_revert(&mut self) {
        let Some(repo) = &self.repo else { return };
        if matches!(self.viewport, Viewport::Settings | Viewport::Viewer) || self.focus != Focus::Viewport {
            return;
        }

        if Self::active_operation_kind(repo).is_some() {
            self.on_continue_operation();
            return;
        }

        if self.viewport != Viewport::Graph || self.graph_selected == 0 {
            return;
        }

        let Some(oid) = self.graph_oid_at(self.graph_selected) else {
            return;
        };

        let original_message = match repo.find_commit(oid) {
            Ok(commit) if commit.parent_count() > 1 => None,
            Ok(commit) => Some(Ok(commit.summary().unwrap_or(operations::REVERT_COMMIT_FALLBACK()).to_string())),
            Err(error) => Some(Err(error)),
        };

        match original_message {
            None => self.show_error(errors::REVERT_MERGE_UNSUPPORTED()),
            Some(Ok(original_message)) => {
                self.pending_revert_oid = Some(oid);
                self.modal_input.set_value(operations::reverted(&original_message));
                self.focus = Focus::ModalRevert;
            },
            Some(Err(error)) => self.show_error(errors::with_error(errors::REVERT(), error)),
        }
    }

    pub fn on_rebase(&mut self) {
        let Some(repo) = &self.repo else { return };
        if matches!(self.viewport, Viewport::Settings | Viewport::Viewer) || self.focus != Focus::Viewport {
            return;
        }

        if is_rebase_in_progress(repo) || is_cherrypick_in_progress(repo) || is_revert_in_progress(repo) || is_merge_in_progress(repo) {
            self.on_continue_operation();
            return;
        }

        if self.viewport != Viewport::Graph || self.graph_selected == 0 {
            return;
        }

        let Some(oid) = self.graph_oid_at(self.graph_selected) else {
            return;
        };
        self.pending_operation_action = Some(PendingOperationAction::Start { kind: OperationKind::Rebase, oid });
        self.modal_operation_kind = OperationKind::Rebase;
        self.modal_operation_message = operations::rebasing_selected_commit();
        self.focus = Focus::ModalOperationProgress;
    }

    pub fn on_merge(&mut self) {
        let Some(repo) = &self.repo else { return };
        if matches!(self.viewport, Viewport::Settings | Viewport::Viewer) || self.focus != Focus::Viewport {
            return;
        }

        if is_rebase_in_progress(repo) || is_cherrypick_in_progress(repo) || is_revert_in_progress(repo) || is_merge_in_progress(repo) {
            self.on_continue_operation();
            return;
        }

        if self.viewport != Viewport::Graph || self.graph_selected == 0 {
            return;
        }

        let Some(oid) = self.graph_oid_at(self.graph_selected) else {
            return;
        };
        self.pending_operation_action = Some(PendingOperationAction::Start { kind: OperationKind::Merge, oid });
        self.modal_operation_kind = OperationKind::Merge;
        self.modal_operation_message = operations::merging_selected_commit();
        self.focus = Focus::ModalOperationProgress;
    }

    pub fn on_continue_operation(&mut self) {
        let Some(repo) = &self.repo else { return };
        if matches!(self.viewport, Viewport::Settings | Viewport::Viewer) || self.focus != Focus::Viewport || Self::active_operation_kind(repo).is_none() {
            return;
        }

        let kind = Self::active_operation_kind(repo).unwrap();
        self.pending_operation_action = Some(PendingOperationAction::Continue);
        self.modal_operation_kind = kind;
        self.modal_operation_message = operations::continuing(kind.label());
        self.focus = Focus::ModalOperationProgress;
    }

    pub fn on_abort_operation(&mut self) {
        let Some(repo) = &self.repo else { return };
        if matches!(self.viewport, Viewport::Settings | Viewport::Viewer) || self.focus != Focus::Viewport || Self::active_operation_kind(repo).is_none() {
            return;
        }

        let kind = Self::active_operation_kind(repo).unwrap();
        self.pending_operation_action = Some(PendingOperationAction::Abort);
        self.modal_operation_kind = kind;
        self.modal_operation_message = operations::aborting(kind.label());
        self.focus = Focus::ModalOperationProgress;
    }
}

#[cfg(test)]
#[path = "../../tests/app/input/git.rs"]
mod tests;
