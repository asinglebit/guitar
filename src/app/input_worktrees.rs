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

        self.oids.get_sorted_aliases().get(self.graph_selected).and_then(|alias| self.worktrees.by_alias.get(alias)).cloned().unwrap_or_default()
    }

    fn graph_open_worktree_indices(&self) -> Vec<usize> {
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
mod tests {
    use super::*;
    use crate::core::worktrees::{WorktreeEntry, WorktreeKind, Worktrees};
    use git2::Oid;

    fn test_oid(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; 20]).unwrap()
    }

    fn worktree_entry(name: &str, head: Oid) -> WorktreeEntry {
        WorktreeEntry {
            name: name.into(),
            path: PathBuf::from(format!("/tmp/{name}")),
            branch: Some(name.into()),
            head: Some(head),
            alias: None,
            kind: WorktreeKind::Linked,
            is_current: false,
            is_valid: true,
            is_prunable: false,
            locked_reason: None,
            is_dirty: false,
        }
    }

    fn app_with_graph_worktrees(entries: Vec<WorktreeEntry>) -> App {
        let mut app = App { viewport: Viewport::Graph, focus: Focus::Viewport, graph_selected: 1, ..Default::default() };

        let head = entries.first().and_then(|entry| entry.head).unwrap_or_else(|| test_oid(1));
        let alias = app.oids.get_alias_by_oid(head);
        let uncommitted = app.oids.sorted_aliases[0];
        app.oids.sorted_aliases = vec![uncommitted, alias];
        app.worktrees = Worktrees::from_entries(entries);
        app.worktrees.refresh_aliases(&app.oids);
        app
    }

    #[test]
    fn graph_worktree_target_resolution_handles_zero_one_and_multiple_rows() {
        let empty = app_with_graph_worktrees(Vec::new());
        assert!(empty.graph_worktree_indices().is_empty());

        let head = test_oid(2);
        let one = app_with_graph_worktrees(vec![worktree_entry("feature", head)]);
        assert_eq!(one.graph_worktree_indices(), vec![0]);

        let multiple = app_with_graph_worktrees(vec![worktree_entry("feature", head), worktree_entry("review", head)]);
        assert_eq!(multiple.graph_worktree_indices(), vec![0, 1]);
    }

    #[test]
    fn graph_enter_opens_chooser_for_multiple_valid_worktrees() {
        let head = test_oid(3);
        let mut app = app_with_graph_worktrees(vec![worktree_entry("feature", head), worktree_entry("review", head)]);

        app.on_select();

        assert_eq!(app.focus, Focus::ModalWorktreeChooser);
        assert_eq!(app.modal_worktree_action, WorktreeModalAction::Open);
        assert_eq!(app.modal_worktree_candidates, vec![0, 1]);
        assert_eq!(app.modal_worktree_return_focus, Focus::Viewport);
    }

    #[test]
    fn graph_remove_uses_existing_worktree_removal_guards() {
        let head = test_oid(4);
        let mut current = worktree_entry("current", head);
        current.is_current = true;
        let mut main = worktree_entry("main", head);
        main.kind = WorktreeKind::Main;
        let mut locked = worktree_entry("locked", head);
        locked.locked_reason = Some("keep".into());
        let mut app = app_with_graph_worktrees(vec![current, main, locked]);

        app.on_remove_worktree();

        assert_eq!(app.focus, Focus::ModalError);
        assert!(app.modal_error_message.contains("cannot remove current, main, or locked worktrees"));
    }

    #[test]
    fn graph_remove_opens_confirmation_or_chooser_for_removable_worktrees() {
        let head = test_oid(5);
        let mut one = app_with_graph_worktrees(vec![worktree_entry("feature", head)]);
        one.on_remove_worktree();
        assert_eq!(one.focus, Focus::ModalRemoveWorktree);
        assert_eq!(one.modal_worktree_target, Some(0));
        assert_eq!(one.modal_worktree_return_focus, Focus::Viewport);

        let mut multiple = app_with_graph_worktrees(vec![worktree_entry("feature", head), worktree_entry("review", head)]);
        multiple.on_remove_worktree();
        assert_eq!(multiple.focus, Focus::ModalWorktreeChooser);
        assert_eq!(multiple.modal_worktree_action, WorktreeModalAction::Remove);
        assert_eq!(multiple.modal_worktree_candidates, vec![0, 1]);
    }

    #[test]
    fn worktree_chooser_confirmation_routes_open_and_remove_actions() {
        let head = test_oid(6);
        let mut invalid = worktree_entry("invalid", head);
        invalid.is_valid = false;
        let mut open = app_with_graph_worktrees(vec![invalid]);
        open.open_worktree_chooser(WorktreeModalAction::Open, vec![0], Focus::Viewport);
        open.confirm_worktree_chooser();
        assert_eq!(open.focus, Focus::ModalError);
        assert!(open.modal_error_message.contains("path is invalid"));

        let mut remove = app_with_graph_worktrees(vec![worktree_entry("feature", head)]);
        remove.open_worktree_chooser(WorktreeModalAction::Remove, vec![0], Focus::Viewport);
        remove.confirm_worktree_chooser();
        assert_eq!(remove.focus, Focus::ModalRemoveWorktree);
        assert_eq!(remove.modal_worktree_target, Some(0));
    }
}
