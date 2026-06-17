use crate::{
    app::app::{App, Focus, Viewport, WorktreeModalAction},
    git::actions::worktrees::{remove_worktree, unlock_worktree},
};
use std::path::{Path, PathBuf};

impl App {
    pub(super) fn default_worktree_path(&self, name: &str) -> String {
        let current = PathBuf::from(self.path.as_deref().unwrap_or("."));
        let parent = current.parent().unwrap_or_else(|| Path::new("."));
        let repo_name = current.file_name().and_then(|value| value.to_str()).unwrap_or("worktree");
        parent.join(format!("{repo_name}-{name}")).display().to_string()
    }

    fn graph_worktree_indices(&self) -> Vec<usize> {
        if self.viewport != Viewport::Graph || self.graph_selected == 0 {
            return Vec::new();
        }

        if let Some(alias) = self.graph_alias_at(self.graph_selected) {
            if let Some(row) = self.graph_row_at(self.graph_selected) {
                let indices: Vec<usize> =
                    row.worktrees.iter().filter_map(|projected| self.worktrees.entries.iter().position(|entry| entry.path == projected.path && entry.name == projected.name)).collect();
                if !indices.is_empty() {
                    return indices;
                }
            }

            return self
                .worktrees
                .entries
                .iter()
                .enumerate()
                .filter_map(|(idx, entry)| entry.head.and_then(|oid| self.oids.aliases.get(&oid)).is_some_and(|current| *current == alias).then_some(idx))
                .collect();
        }

        Vec::new()
    }

    pub(crate) fn graph_open_worktree_indices(&self) -> Vec<usize> {
        self.graph_worktree_indices().into_iter().filter(|idx| self.worktrees.entries.get(*idx).is_some_and(|entry| entry.is_valid)).collect()
    }

    fn graph_remove_worktree_indices(&self) -> Vec<usize> {
        self.graph_worktree_indices().into_iter().filter(|idx| self.worktrees.entries.get(*idx).is_some_and(|entry| entry.can_remove())).collect()
    }

    pub(super) fn clear_worktree_modal_state(&mut self) {
        self.modal_worktree_selected = 0;
        self.modal_worktree_candidates.clear();
        self.modal_worktree_target = None;
        self.modal_worktree_action = WorktreeModalAction::Open;
        self.modal_worktree_return_focus = Focus::Viewport;
    }

    pub(super) fn close_worktree_modal(&mut self) {
        let return_focus = self.modal_worktree_return_focus;
        self.clear_worktree_modal_state();
        self.focus = return_focus;
    }

    fn open_worktree_chooser(&mut self, action: WorktreeModalAction, candidates: Vec<usize>, return_focus: Focus) {
        self.modal_worktree_selected = 0;
        self.modal_worktree_candidates = candidates;
        self.modal_worktree_target = None;
        self.modal_worktree_action = action;
        self.modal_worktree_return_focus = return_focus;
        self.focus = Focus::ModalWorktreeChooser;
    }

    fn open_worktree_by_index(&mut self, index: usize) {
        let Some(entry) = self.worktrees.entries.get(index).cloned() else {
            return;
        };

        if !entry.is_valid {
            self.show_error("Open worktree failed: worktree path is invalid");
            return;
        }

        self.clear_worktree_modal_state();
        self.submodule_stack.clear();
        self.reload(Some(entry.path.display().to_string()));
        self.viewport = Viewport::Graph;
        self.focus = Focus::Viewport;
        self.graph_selected = 0;
    }

    pub(super) fn open_selected_worktree(&mut self) {
        self.open_worktree_by_index(self.worktrees_selected);
    }

    pub(super) fn open_graph_worktree(&mut self) {
        let candidates = self.graph_open_worktree_indices();

        match candidates.len() {
            0 => {},
            1 => self.open_worktree_by_index(candidates[0]),
            _ => self.open_worktree_chooser(WorktreeModalAction::Open, candidates, Focus::Viewport),
        }
    }

    fn selected_modal_worktree_index(&self) -> Option<usize> {
        let selected = usize::try_from(self.modal_worktree_selected).ok()?;
        self.modal_worktree_candidates.get(selected).copied()
    }

    pub(super) fn confirm_worktree_chooser(&mut self) {
        let Some(index) = self.selected_modal_worktree_index() else {
            return;
        };

        match self.modal_worktree_action {
            WorktreeModalAction::Open => {
                self.open_worktree_by_index(index);
            },
            WorktreeModalAction::Remove => {
                self.modal_worktree_candidates.clear();
                self.modal_worktree_selected = 0;
                self.modal_worktree_target = Some(index);
                self.focus = Focus::ModalRemoveWorktree;
            },
        }
    }

    fn open_remove_worktree_confirmation(&mut self, index: usize, return_focus: Focus) {
        self.modal_worktree_selected = 0;
        self.modal_worktree_candidates.clear();
        self.modal_worktree_target = Some(index);
        self.modal_worktree_action = WorktreeModalAction::Remove;
        self.modal_worktree_return_focus = return_focus;
        self.focus = Focus::ModalRemoveWorktree;
    }

    pub(super) fn confirm_remove_worktree(&mut self) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let Some(index) = self.modal_worktree_target else {
            return;
        };
        let Some(entry) = self.worktrees.entries.get(index).cloned() else {
            return;
        };

        if !entry.can_remove() {
            self.show_error("Remove worktree failed: cannot remove current, main, or locked worktrees");
            return;
        }

        let return_focus = self.modal_worktree_return_focus;
        match remove_worktree(&repo, &entry.name) {
            Ok(_) => {
                self.clear_worktree_modal_state();
                self.focus = return_focus;
                self.reload(None);
            },
            Err(error) => self.show_error(format!("Remove worktree failed: {error}")),
        }
    }

    pub fn on_create_worktree(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer => {},
            _ => {
                if self.focus == Focus::Viewport && self.viewport == Viewport::Graph && self.graph_selected != 0 {
                    self.modal_input.clear();
                    self.modal_worktree_name.clear();
                    self.focus = Focus::ModalCreateWorktreeName;
                }
            },
        }
    }

    pub fn on_remove_worktree(&mut self) {
        if self.viewport == Viewport::Settings || self.viewport == Viewport::Viewer {
            return;
        }

        match self.focus {
            Focus::Worktrees => {
                let Some(entry) = self.worktrees.entries.get(self.worktrees_selected) else {
                    return;
                };

                if !entry.can_remove() {
                    self.show_error("Remove worktree failed: cannot remove current, main, or locked worktrees");
                    return;
                }

                self.open_remove_worktree_confirmation(self.worktrees_selected, Focus::Worktrees);
            },
            Focus::Viewport if self.viewport == Viewport::Graph => {
                let all = self.graph_worktree_indices();
                let removable = self.graph_remove_worktree_indices();

                match removable.len() {
                    0 if !all.is_empty() => self.show_error("Remove worktree failed: cannot remove current, main, or locked worktrees"),
                    0 => {},
                    1 => self.open_remove_worktree_confirmation(removable[0], Focus::Viewport),
                    _ => self.open_worktree_chooser(WorktreeModalAction::Remove, removable, Focus::Viewport),
                }
            },
            _ => {},
        }
    }

    pub fn on_toggle_worktree_lock(&mut self) {
        if self.viewport == Viewport::Settings || self.viewport == Viewport::Viewer || self.focus != Focus::Worktrees {
            return;
        }

        let Some(repo) = self.repo.clone() else {
            return;
        };
        let Some(entry) = self.worktrees.entries.get(self.worktrees_selected).cloned() else {
            return;
        };

        if !entry.can_lock() {
            self.show_error("Lock worktree failed: only valid linked worktrees can be locked");
            return;
        }

        if entry.locked_reason.is_some() {
            match unlock_worktree(&repo, &entry.name) {
                Ok(_) => {
                    self.focus = Focus::Worktrees;
                    self.reload(None);
                },
                Err(error) => self.show_error(format!("Unlock worktree failed: {error}")),
            }
            return;
        }

        self.modal_input.clear();
        self.focus = Focus::ModalLockWorktree;
    }
}

#[cfg(test)]
#[path = "../../tests/app/input/worktrees.rs"]
mod tests;
