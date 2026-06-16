use crate::{
    app::app::{App, AuthInputField, Focus, OperationKind, PendingOperationAction, Viewport},
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
            tagging::untag,
        },
        auth::{AuthRequired, AuthSecret, NetworkResult},
        queries::commits::get_current_branch,
    },
};
use git2::{BranchType, Repository, RepositoryState};
use std::path::Path;

impl App {
    const MAX_AUTH_ATTEMPTS: usize = 3;

    fn start_network_request(&mut self, request: NetworkRequest) {
        if self.network_handle.is_some() {
            self.show_error("Git network operation failed: another network operation is already running");
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
            Err(_) => self.finish_network_failure("Git network operation failed: worker thread panicked".to_string()),
        }
    }

    pub(crate) fn handle_network_result(&mut self, result: NetworkResult) {
        match result {
            NetworkResult::Success => {
                self.pending_network_request = None;
                self.network_auth_attempts = 0;
                self.pending_auth_prompt = None;
                self.auth_username_input.clear();
                self.auth_secret_input.clear();
                self.modal_network_title.clear();
                self.modal_network_message.clear();
                self.focus = Focus::Viewport;
                self.reload(None);
            },
            NetworkResult::AuthRequired(AuthRequired { challenge, rejected }) => {
                self.auth_session.evict(&rejected);
                if self.network_auth_attempts >= Self::MAX_AUTH_ATTEMPTS {
                    self.finish_network_failure(format!("{} failed: authentication failed after {} attempts", challenge.operation, Self::MAX_AUTH_ATTEMPTS));
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
        let operation = self.pending_auth_prompt.as_ref().map(|challenge| challenge.operation.clone()).unwrap_or_else(|| "Git network operation".to_string());
        self.pending_network_request = None;
        self.network_auth_attempts = 0;
        self.pending_auth_prompt = None;
        self.auth_username_input.clear();
        self.auth_secret_input.clear();
        self.modal_network_title.clear();
        self.modal_network_message.clear();
        self.focus = Focus::Viewport;
        self.show_error(format!("{operation} cancelled: authentication was not provided"));
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
            self.show_error("Git operation failed: no repository is open");
            return;
        };

        let repo = match Repository::open(path) {
            Ok(repo) => repo,
            Err(error) => {
                self.focus = Focus::Viewport;
                self.show_error(format!("Open repository failed: {error}"));
                return;
            },
        };

        match action {
            PendingOperationAction::Start { kind: OperationKind::Rebase, oid } => self.handle_rebase_result(start_rebase(&repo, oid)),
            PendingOperationAction::Start { kind: OperationKind::Merge, oid } => self.handle_merge_result(start_merge(&repo, oid)),
            PendingOperationAction::Start { kind: OperationKind::Cherrypick, .. } => {
                self.focus = Focus::Viewport;
                self.show_error("Cherry-pick failed: no commit message was provided");
            },
            PendingOperationAction::Start { kind: OperationKind::Revert, .. } => {
                self.focus = Focus::Viewport;
                self.show_error("Revert failed: no commit message was provided");
            },
            PendingOperationAction::Continue => self.continue_active_operation(&repo),
            PendingOperationAction::Abort => self.abort_active_operation(&repo),
        }
    }

    fn handle_rebase_result(&mut self, result: Result<RebaseOutcome, git2::Error>) {
        self.modal_operation_kind = OperationKind::Rebase;
        match result {
            Ok(RebaseOutcome::Completed { applied }) => {
                self.modal_operation_message = if applied == 1 { "Rebase completed after applying 1 commit.".to_string() } else { format!("Rebase completed after applying {applied} commits.") };
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(RebaseOutcome::Conflict) => {
                self.show_operation_conflict(OperationKind::Rebase, "Rebase stopped because conflicts need to be resolved.");
            },
            Ok(RebaseOutcome::Aborted) => {
                self.modal_operation_message = "Rebase aborted.".to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Err(error) => {
                self.modal_operation_message.clear();
                self.focus = Focus::Viewport;
                self.show_error(format!("Rebase failed: {error}"));
                self.reload(None);
            },
        }
    }

    fn handle_cherrypick_result(&mut self, result: Result<CherrypickOutcome, git2::Error>) {
        self.modal_operation_kind = OperationKind::Cherrypick;
        match result {
            Ok(CherrypickOutcome::Committed { .. }) => {
                self.modal_operation_message = "Cherry-pick completed.".to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(CherrypickOutcome::Conflict) => {
                self.show_operation_conflict(OperationKind::Cherrypick, "Cherry-pick stopped because conflicts need to be resolved.");
            },
            Ok(CherrypickOutcome::Aborted) => {
                self.modal_operation_message = "Cherry-pick aborted.".to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Err(error) => {
                self.modal_operation_message.clear();
                self.focus = Focus::Viewport;
                self.show_error(format!("Cherry-pick failed: {error}"));
                self.reload(None);
            },
        }
    }

    pub(crate) fn handle_revert_result(&mut self, result: Result<RevertOutcome, git2::Error>) {
        self.modal_operation_kind = OperationKind::Revert;
        match result {
            Ok(RevertOutcome::Committed { .. }) => {
                self.modal_operation_message = "Revert completed.".to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(RevertOutcome::Conflict) => {
                self.show_operation_conflict(OperationKind::Revert, "Revert stopped because conflicts need to be resolved.");
            },
            Ok(RevertOutcome::Aborted) => {
                self.modal_operation_message = "Revert aborted.".to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Err(error) => {
                self.modal_operation_message.clear();
                self.focus = Focus::Viewport;
                self.show_error(format!("Revert failed: {error}"));
                self.reload(None);
            },
        }
    }

    fn handle_merge_result(&mut self, result: Result<MergeOutcome, git2::Error>) {
        self.modal_operation_kind = OperationKind::Merge;
        match result {
            Ok(MergeOutcome::Completed { .. }) => {
                self.modal_operation_message = "Merge completed.".to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(MergeOutcome::FastForward { .. }) => {
                self.modal_operation_message = "Merge fast-forwarded.".to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(MergeOutcome::UpToDate) => {
                self.modal_operation_message = "Merge already up to date.".to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Ok(MergeOutcome::Conflict) => {
                self.show_operation_conflict(OperationKind::Merge, "Merge stopped because conflicts need to be resolved.");
            },
            Ok(MergeOutcome::Aborted) => {
                self.modal_operation_message = "Merge aborted.".to_string();
                self.focus = Focus::ModalOperationSuccess;
                self.reload(None);
            },
            Err(error) => {
                self.modal_operation_message.clear();
                self.focus = Focus::Viewport;
                self.show_error(format!("Merge failed: {error}"));
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

    fn active_operation_kind(repo: &Repository) -> Option<OperationKind> {
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
                self.show_error("Continue failed: no rebase, cherry-pick, revert, or merge in progress");
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
                self.show_error("Abort failed: no rebase, cherry-pick, revert, or merge in progress");
            },
        }
    }

    pub fn on_drop(&mut self) {
        if self.repo.is_some() && self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            let Some(row) = self.graph_row_at(self.graph_selected) else {
                return;
            };
            if !row.is_stash {
                return;
            }
            let oid = row.oid;

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
            match pop(&mut repo, &oid, false) {
                Ok(_) => self.reload(None),
                Err(error) => self.show_error(format!("Drop stash failed: {error}")),
            }
        }
    }

    pub fn on_pop(&mut self) {
        if self.repo.is_some() && self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            let Some(row) = self.graph_row_at(self.graph_selected) else {
                return;
            };
            if !row.is_stash {
                return;
            }
            let oid = row.oid;

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
            match pop(&mut repo, &oid, true) {
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
            self.start_network_request(NetworkRequest::Fetch { repo_path: repo_path.to_string(), remote_name: "origin".to_string() });
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

                match checkout_branch(repo, &mut self.branches.visible_branch_names, &mut self.branches.local, alias, &branch) {
                    Ok(_) => {
                        // Keep graph selection on the commit that owns the checked-out branch.
                        self.graph_selected = graph_index.or_else(|| self.oids.get_sorted_aliases().iter().position(|o| o == &alias)).unwrap_or(0);

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
                    let Some(oid) = self.graph_oid_at(self.graph_selected) else {
                        return;
                    };
                    match reset_to_commit(repo, oid, git2::ResetType::Hard) {
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
            let Some(oid) = self.graph_oid_at(self.graph_selected) else {
                return;
            };
            match reset_to_commit(repo, oid, git2::ResetType::Mixed) {
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
                _ => match self.focus {
                    Focus::Viewport => {
                        if self.uncommitted.is_staged {
                            match unstage_all(repo) {
                                Ok(_) => self.reload(None),
                                Err(error) => self.show_error(format!("Unstage all failed: {error}")),
                            }
                        }
                    },
                    Focus::StatusTop => {
                        if self.selected_staged_status_file_is_conflict() {
                            self.show_error("Unstage file failed: resolve conflicts in your editor, then continue the active operation");
                            return;
                        }
                        let Some(file) = self.selected_staged_status_file_name() else {
                            return;
                        };
                        match unstage_file(repo, Path::new(&file)) {
                            Ok(_) => self.reload(None),
                            Err(error) => self.show_error(format!("Unstage file failed: {error}")),
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
                                Err(error) => self.show_error(format!("Stage all failed: {error}")),
                            }
                        }
                    },
                    Focus::StatusBottom => {
                        if self.selected_unstaged_status_file_is_conflict() {
                            self.show_error("Stage file failed: resolve conflicts in your editor, then continue the active operation");
                            return;
                        }
                        let Some(file) = self.selected_unstaged_status_file_name() else {
                            return;
                        };
                        match stage_file(repo, Path::new(&file)) {
                            Ok(_) => self.reload(None),
                            Err(error) => self.show_error(format!("Stage file failed: {error}")),
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
        if let Some(repo) = &self.repo {
            match self.viewport {
                Viewport::Settings | Viewport::Viewer => {},
                _ => {
                    let repo_path = self.path.as_deref().unwrap_or(".");
                    let Some(branch) = get_current_branch(repo) else {
                        self.show_error("Push failed: detached HEAD has no current branch");
                        return;
                    };
                    self.start_network_request(NetworkRequest::PushBranch { repo_path: repo_path.to_string(), remote_name: "origin".to_string(), branch, force: true });
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
                    self.start_network_request(NetworkRequest::PushTags { repo_path: repo_path.to_string(), remote_name: "origin".to_string() });
                },
            }
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

    pub(crate) fn delete_branch_from_ui(&mut self, branch: &str) {
        let Some(repo) = self.repo.clone() else {
            return;
        };

        if repo.find_branch(branch, BranchType::Local).is_ok() {
            match delete_branch(&repo, branch) {
                Ok(_) => {
                    self.branches.visible_branch_names.remove(branch);
                    self.modal_delete_branch_selected = 0;
                    self.focus = Focus::Viewport;
                    self.reload(None);
                },
                Err(error) => self.show_error(format!("Delete branch failed: {error}")),
            }
            return;
        }

        let (remote_name, remote_branch) = branch.split_once('/').unwrap_or(("origin", branch));
        if remote_name.is_empty() || remote_branch.is_empty() {
            self.show_error("Delete branch failed: remote branch name is invalid");
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
                    self.show_error("Delete branch failed: cannot delete the current branch");
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
                let visible_branches = self.graph_deletable_branch_choices(alias, current.as_deref());

                match visible_branches.len() {
                    0 => {},
                    1 => self.delete_branch_from_ui(&visible_branches[0]),
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
                            Err(error) => self.show_error(format!("Delete tag failed: {error}")),
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
                                    Err(error) => self.show_error(format!("Delete tag failed: {error}")),
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
                Ok(commit) => Ok(commit.summary().unwrap_or("Cherry-pick commit").to_string()),
                Err(error) => Err(error),
            };

            match original_message {
                Ok(original_message) => {
                    self.pending_cherrypick_oid = Some(oid);
                    self.modal_input.set_value(format!("cherrypicked: {original_message}"));
                    self.focus = Focus::ModalCherrypick;
                },
                Err(error) => self.show_error(format!("Cherry-pick failed: {error}")),
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
            Ok(commit) => Some(Ok(commit.summary().unwrap_or("Revert commit").to_string())),
            Err(error) => Some(Err(error)),
        };

        match original_message {
            None => self.show_error("Revert failed: reverting merge commits is not supported"),
            Some(Ok(original_message)) => {
                self.pending_revert_oid = Some(oid);
                self.modal_input.set_value(format!("reverted: {original_message}"));
                self.focus = Focus::ModalRevert;
            },
            Some(Err(error)) => self.show_error(format!("Revert failed: {error}")),
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
        self.modal_operation_message = "Rebasing the current branch onto the selected commit...".to_string();
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
        self.modal_operation_message = "Merging the selected commit into the current branch...".to_string();
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
        self.modal_operation_message = format!("Continuing {}...", kind.label());
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
        self.modal_operation_message = format!("Aborting {}...", kind.label());
        self.focus = Focus::ModalOperationProgress;
    }
}

#[cfg(test)]
#[path = "../../tests/app/input/git.rs"]
mod tests;
