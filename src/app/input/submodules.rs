use crate::{
    app::app::{App, Focus, Viewport},
    core::submodules::SubmoduleStackEntry,
    git::actions::{network::NetworkRequest, submodules::sync_submodule},
    helpers::localisation::errors,
};
use std::path::PathBuf;

impl App {
    pub(crate) fn selected_submodule_name(&self) -> Option<String> {
        self.submodules.entries.get(self.submodules_selected).map(|entry| entry.name.clone())
    }

    pub(super) fn open_selected_submodule(&mut self) {
        let Some(entry) = self.submodules.entries.get(self.submodules_selected).cloned() else {
            return;
        };

        if !entry.can_open() {
            self.show_error(errors::OPEN_SUBMODULE_NOT_INITIALIZED());
            return;
        }

        let parent_path = PathBuf::from(self.path.as_deref().unwrap_or("."));
        self.submodule_stack.push(SubmoduleStackEntry::new(parent_path, entry.path.clone(), entry.name.clone()));
        self.reload(Some(entry.absolute_path.display().to_string()));
        self.viewport = Viewport::Graph;
        self.focus = Focus::Viewport;
        self.graph_selected = 0;
    }

    pub fn on_return_to_parent_repository(&mut self) {
        if self.is_modal_focus() {
            return;
        }

        let Some(entry) = self.submodule_stack.pop() else {
            return;
        };

        self.reload(Some(entry.parent_path.display().to_string()));
        self.viewport = Viewport::Graph;
        self.focus = Focus::Viewport;
        self.graph_selected = 0;
        self.viewer_selected = 0;
        self.file_name = None;
    }

    pub fn on_update_submodule(&mut self) {
        if self.viewport == Viewport::Settings || self.viewport == Viewport::Viewer || self.focus != Focus::Submodules {
            return;
        }

        let Some(name) = self.selected_submodule_name() else {
            return;
        };
        let repo_path = self.path.as_deref().unwrap_or(".");
        self.start_network_request(NetworkRequest::UpdateSubmodule { repo_path: repo_path.to_string(), name });
    }

    pub fn on_sync_submodule(&mut self) {
        if self.viewport == Viewport::Settings || self.viewport == Viewport::Viewer || self.focus != Focus::Submodules {
            return;
        }

        let Some(repo) = self.repo.clone() else {
            return;
        };
        let Some(name) = self.selected_submodule_name() else {
            return;
        };

        match sync_submodule(&repo, &name) {
            Ok(_) => {
                self.focus = Focus::Submodules;
                self.reload(None);
            },
            Err(error) => self.show_error(errors::with_error(errors::SYNC_SUBMODULE(), error)),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/input/submodules.rs"]
mod tests;
