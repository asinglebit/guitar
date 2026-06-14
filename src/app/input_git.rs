use crate::{
    app::app::{App, Focus, PendingRebaseAction, Viewport},
    git::{
        actions::{
            branching::delete_branch,
            checkout::{checkout_branch, checkout_head},
            fetching::fetch_over_ssh,
            pushing::{push_over_ssh, push_tags_over_ssh},
            rebasing::{RebaseOutcome, abort_rebase, continue_rebase, is_rebase_in_progress, start_rebase},
            resetting::{reset_file, reset_to_commit},
            staging::{stage_all, stage_file, unstage_all, unstage_file},
            stashing::{pop, stash},
            tagging::untag,
        },
        queries::commits::get_current_branch,
    },
};
use git2::Repository;
use std::path::Path;

impl App {
    pub fn run_pending_rebase_action(&mut self) {
        let Some(action) = self.pending_rebase_action.take() else {
            return;
        };
        let Some(path) = self.repo.as_ref().map(|repo| repo.path().to_path_buf()) else {
            self.focus = Focus::Viewport;
            self.show_error("Rebase failed: no repository is open");
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

        let result = match action {
            PendingRebaseAction::Start(oid) => start_rebase(&repo, oid),
            PendingRebaseAction::Continue => continue_rebase(&repo),
            PendingRebaseAction::Abort => abort_rebase(&repo),
        };

        match result {
            Ok(RebaseOutcome::Completed { applied }) => {
                self.modal_rebase_message = if applied == 1 { "Rebase completed after applying 1 commit.".to_string() } else { format!("Rebase completed after applying {applied} commits.") };
                self.focus = Focus::ModalRebaseSuccess;
                self.reload(None);
            },
            Ok(RebaseOutcome::Conflict) => {
                self.modal_rebase_message = "Rebase stopped because conflicts need to be resolved.".to_string();
                self.focus = Focus::ModalRebaseConflict;
                self.reload(None);
            },
            Ok(RebaseOutcome::Aborted) => {
                self.modal_rebase_message = "Rebase aborted.".to_string();
                self.focus = Focus::ModalRebaseSuccess;
                self.reload(None);
            },
            Err(error) => {
                self.modal_rebase_message.clear();
                self.focus = Focus::Viewport;
                self.show_error(format!("Rebase failed: {error}"));
                self.reload(None);
            },
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
                                let conflicts = &self.uncommitted.conflicts;
                                if idx < conflicts.len() {
                                    self.show_error("Unstage file failed: resolve conflicts in your editor, then continue the rebase");
                                    return;
                                }
                                idx -= conflicts.len();
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
                                let conflicts = &self.uncommitted.conflicts;
                                if idx < conflicts.len() {
                                    self.show_error("Stage file failed: resolve conflicts in your editor, then continue the rebase");
                                    return;
                                }
                                idx -= conflicts.len();
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
            let oid = *self.oids.get_oid_by_idx(idx);

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

    pub fn on_rebase(&mut self) {
        let Some(repo) = &self.repo else { return };
        if matches!(self.viewport, Viewport::Settings | Viewport::Viewer) || self.focus != Focus::Viewport {
            return;
        }

        if is_rebase_in_progress(repo) {
            self.pending_rebase_action = Some(PendingRebaseAction::Continue);
            self.modal_rebase_message = "Continuing rebase...".to_string();
            self.focus = Focus::ModalRebaseProgress;
            return;
        }

        if self.viewport != Viewport::Graph || self.graph_selected == 0 {
            return;
        }

        let oid = *self.oids.get_oid_by_idx(self.graph_selected);
        self.pending_rebase_action = Some(PendingRebaseAction::Start(oid));
        self.modal_rebase_message = "Rebasing the current branch onto the selected commit...".to_string();
        self.focus = Focus::ModalRebaseProgress;
    }

    pub fn on_abort_rebase(&mut self) {
        let Some(repo) = &self.repo else { return };
        if matches!(self.viewport, Viewport::Settings | Viewport::Viewer) || self.focus != Focus::Viewport || !is_rebase_in_progress(repo) {
            return;
        }

        self.pending_rebase_action = Some(PendingRebaseAction::Abort);
        self.modal_rebase_message = "Aborting rebase...".to_string();
        self.focus = Focus::ModalRebaseProgress;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chunk::NONE;
    use git2::Signature;
    use std::{
        fs,
        path::Path,
        rc::Rc,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_repo(name: &str) -> (std::path::PathBuf, Repository) {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-input-git-{name}-{id}"));
        fs::create_dir_all(&path).unwrap();
        let repo = Repository::init(&path).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        (path, repo)
    }

    fn commit(repo: &Repository, file: &str, message: &str) -> git2::Oid {
        let workdir = repo.workdir().unwrap().to_path_buf();
        fs::write(workdir.join(file), "content\n").unwrap();

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

    #[test]
    fn cherrypick_opens_message_modal_with_prefilled_summary() {
        let (_path, repo) = temp_repo("cherrypick-modal");
        let oid = commit(&repo, "file.txt", "original summary\n\nbody");

        let mut app = App { repo: Some(Rc::new(repo)), viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };
        let alias = app.oids.get_alias_by_oid(oid);
        app.oids.sorted_aliases = vec![NONE, alias];

        app.on_cherrypick();

        assert_eq!(app.focus, Focus::ModalCherrypick);
        assert_eq!(app.pending_cherrypick_oid, Some(oid));
        assert_eq!(app.modal_input.value(), "cherrypicked: original summary");
    }
}
