use crate::{
    app::app::{App, Focus, RemoteInputAction},
    git::{
        actions::{
            network::NetworkRequest,
            remotes::{add_remote, delete_remote, rename_remote, set_default_remote, set_remote_push_url, set_remote_url},
        },
        queries::remotes::list_remotes,
    },
    helpers::branch_visibility::save_branch_visibility,
};

pub(crate) const REMOTE_ACTIONS: [&str; 6] = ["fetch", "set as default", "rename", "edit fetch URL", "edit push URL", "delete"];

impl App {
    pub(crate) fn begin_add_remote(&mut self) {
        self.modal_remote_selected = 0;
        self.modal_remote_target = None;
        self.modal_remote_name.clear();
        self.modal_remote_input_action = RemoteInputAction::AddName;
        self.modal_input.clear();
        self.focus = Focus::ModalRemoteName;
    }

    pub(crate) fn begin_remote_action(&mut self, remote_name: String) {
        self.modal_remote_selected = 0;
        self.modal_remote_target = Some(remote_name);
        self.modal_input.clear();
        self.focus = Focus::ModalRemoteAction;
    }

    pub(crate) fn close_remote_modal(&mut self) {
        self.modal_remote_selected = 0;
        self.modal_remote_target = None;
        self.modal_remote_name.clear();
        self.modal_input.clear();
        self.focus = Focus::Viewport;
    }

    pub(crate) fn remote_input_title(&self) -> &'static str {
        match self.modal_remote_input_action {
            RemoteInputAction::AddName => "Enter new remote name",
            RemoteInputAction::AddUrl => "Enter new remote URL",
            RemoteInputAction::Rename => "Enter renamed remote name",
            RemoteInputAction::EditUrl => "Enter remote fetch URL",
            RemoteInputAction::EditPushUrl => "Enter remote push URL",
        }
    }

    pub(crate) fn move_remote_action_selection(&mut self, direction: crate::app::app::Direction) {
        if REMOTE_ACTIONS.is_empty() {
            self.modal_remote_selected = 0;
            return;
        }

        let len = REMOTE_ACTIONS.len() as i32;
        let current = self.modal_remote_selected.rem_euclid(len);
        self.modal_remote_selected = match direction {
            crate::app::app::Direction::Up => (current - 1).rem_euclid(len),
            crate::app::app::Direction::Down => (current + 1).rem_euclid(len),
        };
    }

    pub(crate) fn confirm_remote_action(&mut self) {
        let Some(remote_name) = self.modal_remote_target.clone() else {
            self.close_remote_modal();
            return;
        };

        match self.modal_remote_selected.rem_euclid(REMOTE_ACTIONS.len() as i32) {
            0 => {
                let repo_path = self.path.as_deref().unwrap_or(".");
                self.modal_remote_selected = 0;
                self.modal_remote_target = None;
                self.start_network_request(NetworkRequest::Fetch { repo_path: repo_path.to_string(), remote_name });
            },
            1 => {
                let Some(repo) = self.repo.clone() else {
                    self.close_remote_modal();
                    return;
                };
                match set_default_remote(&repo, &remote_name) {
                    Ok(_) => {
                        self.close_remote_modal();
                        self.viewport = crate::app::app::Viewport::Settings;
                        self.reload(None);
                    },
                    Err(error) => self.show_error(format!("Set default remote failed: {error}")),
                }
            },
            2 => {
                self.modal_remote_input_action = RemoteInputAction::Rename;
                self.modal_input.set_value(remote_name);
                self.focus = Focus::ModalRemoteName;
            },
            3 => {
                self.modal_remote_input_action = RemoteInputAction::EditUrl;
                self.prefill_remote_url(false);
                self.focus = Focus::ModalRemoteUrl;
            },
            4 => {
                self.modal_remote_input_action = RemoteInputAction::EditPushUrl;
                self.prefill_remote_url(true);
                self.focus = Focus::ModalRemoteUrl;
            },
            5 => {
                self.focus = Focus::ModalRemoteDelete;
            },
            _ => {},
        }
    }

    fn prefill_remote_url(&mut self, push_url: bool) {
        let Some(repo) = self.repo.clone() else {
            self.modal_input.clear();
            return;
        };
        let Some(target) = self.modal_remote_target.as_deref() else {
            self.modal_input.clear();
            return;
        };

        let value = list_remotes(&repo)
            .ok()
            .and_then(|remotes| remotes.into_iter().find(|remote| remote.name == target))
            .map(|remote| if push_url { remote.push_url.unwrap_or_default() } else { remote.url })
            .unwrap_or_default();
        self.modal_input.set_value(value);
    }

    pub(crate) fn confirm_remote_name_input(&mut self) {
        match self.modal_remote_input_action {
            RemoteInputAction::AddName => {
                let name = self.modal_input.value().trim().to_string();
                if name.is_empty() {
                    return;
                }
                if !git2::Remote::is_valid_name(&name) {
                    self.show_error("Add remote failed: remote name is invalid");
                    return;
                }
                self.modal_remote_name = name;
                self.modal_input.clear();
                self.modal_remote_input_action = RemoteInputAction::AddUrl;
                self.focus = Focus::ModalRemoteUrl;
            },
            RemoteInputAction::Rename => {
                let Some(repo) = self.repo.clone() else {
                    self.close_remote_modal();
                    return;
                };
                let Some(old_name) = self.modal_remote_target.clone() else {
                    self.show_error("Rename remote failed: no remote is pending");
                    return;
                };
                let new_name = self.modal_input.value().trim().to_string();
                match rename_remote(&repo, &old_name, &new_name) {
                    Ok(_) => {
                        self.rewrite_hidden_remote_prefix(&old_name, Some(&new_name));
                        self.close_remote_modal();
                        self.viewport = crate::app::app::Viewport::Settings;
                        self.reload(None);
                    },
                    Err(error) => self.show_error(format!("Rename remote failed: {error}")),
                }
            },
            _ => {},
        }
    }

    pub(crate) fn confirm_remote_url_input(&mut self) {
        let Some(repo) = self.repo.clone() else {
            self.close_remote_modal();
            return;
        };

        match self.modal_remote_input_action {
            RemoteInputAction::AddUrl => {
                let name = self.modal_remote_name.clone();
                let url = self.modal_input.value().trim().to_string();
                match add_remote(&repo, &name, &url) {
                    Ok(_) => {
                        self.close_remote_modal();
                        self.viewport = crate::app::app::Viewport::Settings;
                        self.reload(None);
                    },
                    Err(error) => self.show_error(format!("Add remote failed: {error}")),
                }
            },
            RemoteInputAction::EditUrl => {
                let Some(remote_name) = self.modal_remote_target.clone() else {
                    self.show_error("Edit remote failed: no remote is pending");
                    return;
                };
                let url = self.modal_input.value().trim().to_string();
                match set_remote_url(&repo, &remote_name, &url) {
                    Ok(_) => {
                        self.close_remote_modal();
                        self.viewport = crate::app::app::Viewport::Settings;
                        self.reload(None);
                    },
                    Err(error) => self.show_error(format!("Edit remote failed: {error}")),
                }
            },
            RemoteInputAction::EditPushUrl => {
                let Some(remote_name) = self.modal_remote_target.clone() else {
                    self.show_error("Edit remote failed: no remote is pending");
                    return;
                };
                let push_url = self.modal_input.value().trim().to_string();
                match set_remote_push_url(&repo, &remote_name, Some(push_url.as_str())) {
                    Ok(_) => {
                        self.close_remote_modal();
                        self.viewport = crate::app::app::Viewport::Settings;
                        self.reload(None);
                    },
                    Err(error) => self.show_error(format!("Edit remote failed: {error}")),
                }
            },
            _ => {},
        }
    }

    pub(crate) fn confirm_delete_remote(&mut self) {
        let Some(repo) = self.repo.clone() else {
            self.close_remote_modal();
            return;
        };
        let Some(remote_name) = self.modal_remote_target.clone() else {
            self.show_error("Delete remote failed: no remote is pending");
            return;
        };

        match delete_remote(&repo, &remote_name) {
            Ok(_) => {
                self.rewrite_hidden_remote_prefix(&remote_name, None);
                self.close_remote_modal();
                self.viewport = crate::app::app::Viewport::Settings;
                self.reload(None);
            },
            Err(error) => self.show_error(format!("Delete remote failed: {error}")),
        }
    }

    fn rewrite_hidden_remote_prefix(&mut self, old_remote: &str, new_remote: Option<&str>) {
        let prefix = format!("{old_remote}/");
        let updated = self
            .branches
            .hidden_branch_names
            .iter()
            .filter_map(|name| if let Some(suffix) = name.strip_prefix(&prefix) { new_remote.map(|remote| format!("{remote}/{suffix}")) } else { Some(name.clone()) })
            .collect();

        self.branches.hidden_branch_names = updated;
        if let Some(path) = &self.path {
            save_branch_visibility(path, &self.branches.hidden_branch_names);
        }
    }
}
