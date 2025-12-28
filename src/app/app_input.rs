#[rustfmt::skip]
use std::{
    io
};
use git2::{Oid, Repository};
#[rustfmt::skip]
use indexmap::IndexMap;
#[rustfmt::skip]
use ratatui::{
    crossterm::event::{
        self,
        Event,
        KeyCode,
        KeyEvent,
        KeyEventKind,
        KeyModifiers,
        KeyCode::*,
    }
};
#[rustfmt::skip]
use edtui::{
    EditorMode,
};
use crate::git::actions::commits::stash;
#[rustfmt::skip]
use crate::{
    app::app::{
        App,
        Focus,
        Viewport,
        Direction
    },
    git::{
        actions::{
            commits::{
                checkout_head,
                checkout_branch,
                commit_staged,
                git_add_all,
                reset_to_commit,
                unstage_all,
                fetch_over_ssh,
                push_over_ssh,
                create_branch,
                delete_branch
            }
        },
        queries::{
            diffs::{
                get_filenames_diff_at_oid,
            },
            commits::{
                get_current_branch
            }
        }
    },
    helpers::{
        palette::Theme,
        text::{
            editor_state_to_string,
        }
    }
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {

    // List navigation
    Select,
    NextPane,
    PreviousPane,
    PageUp,
    PageDown,    
    ScrollUp,
    ScrollDown,    
    ScrollUpHalf,
    ScrollDownHalf,
    ScrollUpBranch,
    ScrollDownBranch,
    ScrollUpCommit,
    ScrollDownCommit,
    GoToBeginning,
    GoToEnd,
    
    // Branches
    JumpToBranch,
    SoloBranch,
    
    // Git
    Stash,
    Grep,
    Fetch,
    Checkout,
    HardReset,
    MixedReset,
    UnstageAll,
    StageAll,
    Commit,
    Push,
    CreateANewBranch,
    DeleteABranch,
    
    // Layout
    GoBack,
    Reload,
    Minimize,
    ToggleShas,
    ToggleBranches,
    ToggleStatus,
    ToggleInspector,
    ToggleSettings,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
}

impl App {

    pub fn load_keymap(&mut self) {
        let mut map = IndexMap::new();
        
        // List navigation
        map.insert(KeyBinding::new(Enter, KeyModifiers::NONE), Command::Select);
        map.insert(KeyBinding::new(Tab, KeyModifiers::NONE), Command::NextPane);
        map.insert(KeyBinding::new(BackTab, KeyModifiers::SHIFT), Command::PreviousPane);
        map.insert(KeyBinding::new(PageUp, KeyModifiers::NONE), Command::PageUp);
        map.insert(KeyBinding::new(PageDown, KeyModifiers::NONE), Command::PageDown);
        map.insert(KeyBinding::new(Up, KeyModifiers::NONE), Command::ScrollUp);
        map.insert(KeyBinding::new(Down, KeyModifiers::NONE), Command::ScrollDown);
        map.insert(KeyBinding::new(Up, KeyModifiers::SHIFT), Command::ScrollUpHalf);
        map.insert(KeyBinding::new(Down, KeyModifiers::SHIFT), Command::ScrollDownHalf);
        map.insert(KeyBinding::new(Up, KeyModifiers::CONTROL), Command::ScrollUpBranch);
        map.insert(KeyBinding::new(Down, KeyModifiers::CONTROL), Command::ScrollDownBranch);
        map.insert(KeyBinding::new(Up, KeyModifiers::ALT), Command::ScrollUpCommit);
        map.insert(KeyBinding::new(Down, KeyModifiers::ALT), Command::ScrollDownCommit);
        map.insert(KeyBinding::new(Home, KeyModifiers::NONE), Command::GoToBeginning);
        map.insert(KeyBinding::new(End, KeyModifiers::NONE), Command::GoToEnd);

        // Branches
        map.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::JumpToBranch);
        map.insert(KeyBinding::new(Char('o'), KeyModifiers::NONE), Command::SoloBranch);
        
        // Git
        map.insert(KeyBinding::new(Char('e'), KeyModifiers::NONE), Command::Stash);
        map.insert(KeyBinding::new(Char('g'), KeyModifiers::NONE), Command::Grep);
        map.insert(KeyBinding::new(Char('f'), KeyModifiers::NONE), Command::Fetch);
        map.insert(KeyBinding::new(Char('c'), KeyModifiers::NONE), Command::Checkout);
        map.insert(KeyBinding::new(Char('h'), KeyModifiers::NONE), Command::HardReset);
        map.insert(KeyBinding::new(Char('m'), KeyModifiers::NONE), Command::MixedReset);
        map.insert(KeyBinding::new(Char('u'), KeyModifiers::NONE), Command::UnstageAll);
        map.insert(KeyBinding::new(Char('s'), KeyModifiers::NONE), Command::StageAll);
        map.insert(KeyBinding::new(Char('a'), KeyModifiers::NONE), Command::Commit);
        map.insert(KeyBinding::new(Char('p'), KeyModifiers::NONE), Command::Push);
        map.insert(KeyBinding::new(Char('b'), KeyModifiers::NONE), Command::CreateANewBranch);
        map.insert(KeyBinding::new(Char('d'), KeyModifiers::NONE), Command::DeleteABranch);

        // Layout
        map.insert(KeyBinding::new(Esc, KeyModifiers::NONE), Command::GoBack);
        map.insert(KeyBinding::new(Char('r'), KeyModifiers::NONE), Command::Reload);
        map.insert(KeyBinding::new(Char('.'), KeyModifiers::NONE), Command::Minimize);
        map.insert(KeyBinding::new(Char('w'), KeyModifiers::NONE), Command::ToggleShas);
        map.insert(KeyBinding::new(Char('`'), KeyModifiers::NONE), Command::ToggleBranches);
        map.insert(KeyBinding::new(Char('2'), KeyModifiers::NONE), Command::ToggleStatus);
        map.insert(KeyBinding::new(Char('1'), KeyModifiers::NONE), Command::ToggleInspector);
        map.insert(KeyBinding::new(F(1), KeyModifiers::NONE), Command::ToggleSettings);
        map.insert(KeyBinding::new(Char('c'), KeyModifiers::CONTROL), Command::Exit);

        self.keymap = map;
    }

    pub fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if matches!(key_event.kind, KeyEventKind::Press) => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {

        let key_binding = KeyBinding::new(key_event.code, key_event.modifiers);
        
        // Handle text editing
        match self.focus {
            Focus::ModalCommit => {
                if self.commit_editor.mode == EditorMode::Normal {
                    match key_event.code {
                        KeyCode::Esc => {
                            self.focus = Focus::Viewport;
                        }
                        KeyCode::Enter => {
                            commit_staged(
                                &self.repo,
                                &editor_state_to_string(&self.commit_editor),
                                &self.name,
                                &self.email,
                            )
                            .expect("Error");
                            self.commit_editor = edtui::EditorState::default();
                            self.branches.visible.clear();
                            self.reload();
                            self.focus = Focus::Viewport;
                        }
                        _ => {
                            self.commit_editor_event_handler
                                .on_key_event(key_event, &mut self.commit_editor);
                        }
                    }
                } else {
                    self.commit_editor_event_handler
                        .on_key_event(key_event, &mut self.commit_editor);
                }
                return;
            }
            Focus::ModalCreateBranch => {
                if self.create_branch_editor.mode == EditorMode::Normal {
                    match key_event.code {
                        KeyCode::Esc => {
                            self.focus = Focus::Viewport;
                        }
                        KeyCode::Enter => {
                            let oid = self.oids.get_oid_by_idx(if self.graph_selected == 0 { 1 } else { self.graph_selected });
                            match create_branch(&self.repo, &editor_state_to_string(&self.create_branch_editor), *oid) {
                                Ok(_) => {
                                    self.branches.visible.clear();
                                    self.create_branch_editor = edtui::EditorState::default();
                                    self.reload();
                                    self.focus = Focus::Viewport;
                                }
                                Err(_) => {
                                    // TODO
                                }
                            }
                        }
                        _ => {
                            self.create_branch_editor_event_handler
                                .on_key_event(key_event, &mut self.create_branch_editor);
                        }
                    }
                } else {
                    self.create_branch_editor_event_handler
                        .on_key_event(key_event, &mut self.create_branch_editor);
                }
                return;
            }
            Focus::ModalGrep => {
                if self.grep_editor.mode == EditorMode::Normal {
                    match key_event.code {
                        KeyCode::Esc => {
                            self.focus = Focus::Viewport;
                        }
                        KeyCode::Enter => {
                            let sha = editor_state_to_string(&self.grep_editor);

                            // Reject obviously invalid prefixes early
                            if sha.is_empty() || sha.len() > 40 {
                                return;
                            }
                            
                            // Find the correpsonding oid
                            let oid: Option<Oid> = self.oids.oids.iter()
                                .find(|oid| oid.to_string().starts_with(&sha))
                                .copied();

                            // In case oid exists
                            if let Some(oid) = oid {

                                // Get the alias
                                let oid_alias = self.oids.get_alias_by_oid(oid);

                                // Find the position in the sorted alias vector
                                let next = self.oids.get_sorted_aliases()
                                    .iter()
                                    .position(|&alias| alias == oid_alias)
                                    .unwrap();

                                // Scroll to line number
                                self.graph_selected = next;
                                self.grep_editor = edtui::EditorState::default();
                                self.focus = Focus::Viewport;
                            }

                        }
                        _ => {
                            self.grep_editor_event_handler
                                .on_key_event(key_event, &mut self.grep_editor);
                        }
                    }
                } else {
                    self.grep_editor_event_handler
                        .on_key_event(key_event, &mut self.grep_editor);
                }
                return;
            }
            Focus::Viewport => {
                if self.viewport == Viewport::Editor {
                    if self.file_editor.mode == EditorMode::Normal {
                        match key_event.code {
                            KeyCode::Char('c')
                                if key_event.modifiers.contains(KeyModifiers::CONTROL) => {}
                            KeyCode::Esc => {
                                self.viewport = Viewport::Graph;
                            }
                            _ => {
                                self.file_editor_event_handler
                                    .on_key_event(key_event, &mut self.file_editor);
                            }
                        }
                    } else {
                        self.file_editor_event_handler
                            .on_key_event(key_event, &mut self.file_editor);
                        return;
                    }
                }
            }
            _ => {}
        }

        // Handle the application
        if let Some(cmd) = self.keymap.get(&key_binding) {
            match cmd {
                // List navigation
                Command::Select => self.on_select(),
                Command::NextPane => self.on_next_pane(),
                Command::PreviousPane => self.on_previous_pane(),
                Command::PageUp => self.on_scroll_page_up(),
                Command::PageDown => self.on_scroll_page_down(),
                Command::ScrollUp => self.on_scroll_up(),
                Command::ScrollDown => self.on_scroll_down(),
                Command::ScrollUpHalf => self.on_scroll_up_half(),
                Command::ScrollDownHalf => self.on_scroll_down_half(),
                Command::ScrollUpBranch => self.on_scroll_up_branch(),
                Command::ScrollDownBranch => self.on_scroll_down_branch(),
                Command::ScrollUpCommit => self.on_scroll_up_commit(),
                Command::ScrollDownCommit => self.on_scroll_down_commit(),
                Command::GoToBeginning => self.on_scroll_to_beginning(),
                Command::GoToEnd => self.on_scroll_to_end(),

                // Branches
                Command::JumpToBranch => self.on_jump_to_branch(),
                Command::SoloBranch => self.on_solo_branch(),

                // Git
                Command::Stash => self.on_stash(),
                Command::Grep => self.on_grep(),
                Command::Fetch => self.on_fetch(),
                Command::Checkout => self.on_checkout(),
                Command::HardReset => self.on_hard_reset(),
                Command::MixedReset => self.on_mixed_reset(),
                Command::UnstageAll => self.on_unstage_all(),
                Command::StageAll => self.on_stage_all(),
                Command::Commit => self.on_commit(),
                Command::Push => self.on_push(),
                Command::CreateANewBranch => self.on_create_branch(),
                Command::DeleteABranch => self.on_delete_branch(),
                
                // Layout
                Command::GoBack => self.on_go_back(),
                Command::Reload => self.on_reload(),
                Command::Minimize => self.on_minimize(),
                Command::ToggleShas => self.on_toggle_shas(),
                Command::ToggleBranches => self.on_toggle_branches(),
                Command::ToggleStatus => self.on_toggle_status(),
                Command::ToggleInspector => self.on_toggle_inspector(),
                Command::ToggleSettings => self.on_toggle_settings(),
                Command::Exit => self.on_exit(),
            }
        }
    }

    pub fn on_select(&mut self) {
        match self.focus {
            Focus::Branches => {
                let (oid, branch) = self.branches.sorted.get(self.branches_selected).unwrap();

                let branch = branch.clone(); // clone because we may insert/remove it

                self.branches.visible
                    .entry(*oid)
                    .and_modify(|branches| {
                        if let Some(pos) = branches.iter().position(|b| b == &branch) {
                            branches.remove(pos);
                        } else {
                            branches.push(branch.clone());
                        }

                        // remove oid entirely if empty
                        if branches.is_empty() {
                            // can't remove while borrowing, so mark later
                        }
                    })
                    .or_insert_with(|| vec![branch]);

                // cleanup pass (safe because we can't mutate while borrowed above)
                if let Some(branches) = self.branches.visible.get(oid)
                    && branches.is_empty() {
                        self.branches.visible.remove(oid);
                    }

                self.reload();
            }
            Focus::Viewport => {
                if self.viewport == Viewport::Settings
                    && let Some(position) = self.settings_selections.iter().position(|&x| x == self.settings_selected) {
                        match position {
                            3 => self.theme = Theme::classic(),
                            4 => self.theme = Theme::ansi(),
                            5 => self.theme = Theme::monochrome(),
                            _ => {}
                        }
                        self.reload();
                    }
            }
            Focus::ModalCheckout => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branches = self.branches.visible.get(&alias).cloned().unwrap_or_default();
                checkout_branch(
                    &self.repo,
                    &mut self.branches.visible,
                    &mut self.branches.local,
                    alias,
                    branches.get(self.modal_checkout_selected as usize).unwrap(),
                )
                .expect("Error");
                self.modal_checkout_selected = 0;
                self.focus = Focus::Viewport;
                self.reload();
            }
            Focus::ModalSolo => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branches = self
                    .branches.visible
                    .get(&alias)
                    .cloned()
                    .unwrap_or_default();
                let branch = branches.get(self.modal_solo_selected as usize).unwrap();
                
                // Check if the same branch is already the only one visible
                let already_visible = 
                    self.branches.visible.len() == 1 &&
                    self.branches.visible.entry(alias).or_default().len() == 1 &&
                    self.branches.visible.entry(alias).or_default().contains(branch);

                if already_visible {
                    self.branches.visible.clear();
                } else {
                    self.branches.visible.clear();
                    self.branches.visible
                        .entry(alias)
                        .and_modify(|branches| branches.push(branch.clone()))
                        .or_insert_with(|| vec![branch.clone()]);
                }
                self.modal_solo_selected = 0;
                self.focus = Focus::Viewport;
                self.reload();
            }
            Focus::ModalDeleteBranch => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branches = self.branches.visible.get(&alias).cloned().unwrap_or_default();
                let branch = branches.get(self.modal_delete_branch_selected as usize).unwrap();
                match delete_branch(&self.repo, branch) {
                    Ok(_) => {
                        self.branches.visible.clear();
                        self.modal_delete_branch_selected = 0;
                        self.focus = Focus::Viewport;
                        self.reload();
                    }
                    Err(_) => {
                        // TODO
                    }
                }
            }
            Focus::StatusTop | Focus::StatusBottom => {
                self.open_viewer();
                self.focus = Focus::Viewport;
            }
            _ => {}
        };
    }

    pub fn on_next_pane(&mut self) {
        self.focus = match self.focus {
            Focus::Branches => Focus::Viewport,
            Focus::Viewport => {
                if self.focus == Focus::Viewport && (self.viewport == Viewport::Editor || self.viewport == Viewport::Settings) {
                    return;
                }
                if self.is_inspector && self.graph_selected != 0 {
                    Focus::Inspector
                } else if self.is_status {
                    Focus::StatusTop
                } else if self.is_branches {
                    Focus::Branches
                } else {
                    Focus::Viewport
                }
            }
            Focus::Inspector => {
                if self.is_status {
                    Focus::StatusTop
                } else if self.is_branches {
                    Focus::Branches
                } else {
                    Focus::Viewport
                }
            }
            Focus::StatusTop => {
                if self.graph_selected == 0 {
                    Focus::StatusBottom
                } else if self.is_branches {
                    Focus::Branches
                } else {
                    Focus::Viewport
                }
            }
            Focus::StatusBottom => {
                if self.is_branches {
                    Focus::Branches
                } else {
                    Focus::Viewport
                }
            }
            _ => Focus::Viewport,
        };
    }
    
    pub fn on_previous_pane(&mut self) {
        self.focus = match self.focus {
            Focus::Branches => {
                if self.is_status && self.graph_selected == 0 {
                    Focus::StatusBottom
                } else if self.is_status {
                    Focus::StatusTop
                } else if self.is_inspector && self.graph_selected != 0 {
                    Focus::Inspector
                } else {
                    Focus::Viewport
                }
            }
            Focus::Viewport => {
                if self.focus == Focus::Viewport && (self.viewport == Viewport::Editor || self.viewport == Viewport::Settings) {
                    return;
                }
                if self.is_branches {
                    Focus::Branches
                } else if self.is_status && self.graph_selected == 0 {
                    Focus::StatusBottom
                } else if self.is_status {
                    Focus::StatusTop
                } else {
                    Focus::Inspector
                }
            }
            Focus::Inspector => {
                Focus::Viewport
            }
            Focus::StatusTop => {
                if self.is_inspector && self.graph_selected != 0 {
                    Focus::Inspector
                } else {
                    Focus::Viewport
                }
            }
            Focus::StatusBottom => {
                Focus::StatusTop
            }
            _ => Focus::Viewport,
        };
    }

    pub fn on_scroll_page_up(&mut self) {
        match self.focus {
            Focus::Branches => {
                let page = self.layout.branches.height as usize - 1;
                self.branches_selected = self.branches_selected.saturating_sub(page);
            }
            Focus::Viewport => {
                let page = self.layout.graph.height as usize - 1;
                match self.viewport {
                    Viewport::Graph => {
                        if self.graph_selected >= page {
                            self.graph_selected -= page;
                        } else {
                            self.graph_selected = 0;
                        }
                        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                            let oid = self.oids.get_oid_by_idx(self.graph_selected);
                            self.current_diff = get_filenames_diff_at_oid(&self.repo, *oid);
                        }
                    }
                    Viewport::Viewer => {
                        if self.viewer_selected >= page {
                            self.viewer_selected -= page;
                        } else {
                            self.viewer_selected = 0;
                        }
                    }
                    Viewport::Settings => {
                        self.settings_selected = self.settings_selected.saturating_sub(page);
                        self.last_input_direction = Some(Direction::Up);
                    }
                    _ => {}
                }
            }
            Focus::Inspector => {
                let page = self.layout.inspector.height as usize - 3;
                self.inspector_selected = self.inspector_selected.saturating_sub(page);
            }
            Focus::StatusTop => {
                let page = self.layout.status_top.height as usize - 3;
                self.status_top_selected = self.status_top_selected.saturating_sub(page);
            }
            Focus::StatusBottom => {
                let page = self.layout.status_bottom.height as usize - 3;
                self.status_bottom_selected = self.status_bottom_selected.saturating_sub(page);
            }
            _ => {}
        };

        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
            let oid = self.oids.get_oid_by_idx(self.graph_selected);
            self.current_diff = get_filenames_diff_at_oid(&self.repo, *oid);
        }
    }

    pub fn on_scroll_page_down(&mut self) {
        match self.focus {
            Focus::Branches => {
                let page = self.layout.branches.height as usize - 1;
                self.branches_selected += page;
            }
            Focus::Viewport => {
                let page = self.layout.graph.height as usize - 1;
                match self.viewport {
                    Viewport::Graph => {
                        if self.graph_selected + page < self.oids.get_commit_count() {
                            self.graph_selected += page;
                        } else {
                            self.graph_selected = self.oids.get_commit_count() - 1;
                        }
                        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                            let oid = self.oids.get_oid_by_idx(self.graph_selected);
                            self.current_diff = get_filenames_diff_at_oid(&self.repo, *oid);
                        }
                    }
                    Viewport::Viewer => {
                        if self.viewer_selected + page < self.viewer_lines.len() {
                            self.viewer_selected += page;
                        } else {
                            self.viewer_selected = self.viewer_lines.len() - 1;
                        }
                    }
                    Viewport::Settings => {
                        self.settings_selected += page;
                        self.last_input_direction = Some(Direction::Down);
                    }
                    _ => {}
                }
            }
            Focus::Inspector => {
                let page = self.layout.inspector.height as usize - 3;
                self.inspector_selected += page;
            }
            Focus::StatusTop => {
                let page = self.layout.status_top.height as usize - 3;
                self.status_top_selected += page;
            }
            Focus::StatusBottom => {
                let page = self.layout.status_bottom.height as usize - 3;
                self.status_bottom_selected += page;
            }
            _ => {}
        };
        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
            let oid = self.oids.get_oid_by_idx(self.graph_selected);
            self.current_diff = get_filenames_diff_at_oid(&self.repo, *oid);
        }
    }

    pub fn on_scroll_up(&mut self) {
        match self.focus {
            Focus::Branches => {
                self.branches_selected = self.branches_selected.saturating_sub(1);
            }
            Focus::Viewport => {
                match self.viewport {
                    Viewport::Graph => {
                        if self.graph_selected > 0 {
                            self.graph_selected -= 1;
                            if self.graph_selected == 0 && self.focus == Focus::Inspector {
                                self.focus = Focus::Viewport;
                            }
                        }
                        if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                            let oid = self.oids.get_oid_by_idx(self.graph_selected);
                            self.current_diff = get_filenames_diff_at_oid(&self.repo, *oid);
                        }
                    }
                    Viewport::Viewer => {
                        if self.viewer_selected > 0 {
                            self.viewer_selected -= 1;
                        }
                    }
                    Viewport::Settings => {
                        self.settings_selected = self.settings_selected.saturating_sub(1);
                        self.last_input_direction = Some(Direction::Up);
                    }
                    _ => {}
                }
                if self.viewport == Viewport::Graph {}
            }
            Focus::Inspector => {
                self.inspector_selected = self.inspector_selected.saturating_sub(1);
            }
            Focus::StatusTop => {
                self.status_top_selected = self.status_top_selected.saturating_sub(1);
            }
            Focus::StatusBottom => {
                self.status_bottom_selected = self.status_bottom_selected.saturating_sub(1);
            }
            Focus::ModalCheckout => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branches = self.branches.visible.entry(alias).or_default();
                self.modal_checkout_selected = if self.modal_checkout_selected - 1 < 0 {
                    branches.len() as i32 - 1
                } else {
                    self.modal_checkout_selected - 1
                };
            }
            Focus::ModalSolo => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branches = self.branches.visible.entry(alias).or_default();
                self.modal_solo_selected = if self.modal_solo_selected - 1 < 0 {
                    branches.len() as i32 - 1
                } else {
                    self.modal_solo_selected - 1
                };
            }
            Focus::ModalDeleteBranch => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branches = self.branches.visible.entry(alias).or_default();
                let length = match get_current_branch(&self.repo) {
                    Some(current) => branches.iter().filter(|branch| current != **branch).count(),
                    None => branches.len()
                };
                self.modal_delete_branch_selected = if self.modal_delete_branch_selected - 1 < 0 {
                    length as i32 - 1
                } else {
                    self.modal_delete_branch_selected - 1
                };
            }
            _ => {}
        }
    }

    pub fn on_scroll_down(&mut self) {
        match self.focus {
            Focus::Branches => {
                self.branches_selected += 1;
            }
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    if self.graph_selected + 1 < self.oids.get_commit_count() {
                        self.graph_selected += 1;
                    }
                    if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                        let oid = self.oids.get_oid_by_idx(self.graph_selected);
                        self.current_diff = get_filenames_diff_at_oid(&self.repo, *oid);
                    }
                }
                Viewport::Viewer => {
                    if self.viewer_selected + 1 < self.viewer_lines.len() {
                        self.viewer_selected += 1;
                    }
                }
                Viewport::Settings => {
                    self.settings_selected += 1;
                    self.last_input_direction = Some(Direction::Down);
                }
                _ => {}
            },
            Focus::Inspector => {
                self.inspector_selected += 1;
            }
            Focus::StatusTop => {
                self.status_top_selected += 1;
            }
            Focus::StatusBottom => {
                self.status_bottom_selected += 1;
            }
            Focus::ModalCheckout => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branches = self.branches.visible.entry(alias).or_default();
                self.modal_checkout_selected = if self.modal_checkout_selected + 1 > branches.len() as i32 - 1 {
                    0
                } else {
                    self.modal_checkout_selected + 1
                };
            }
            Focus::ModalSolo => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branches = self.branches.visible.entry(alias).or_default();
                self.modal_solo_selected = if self.modal_solo_selected + 1 > branches.len() as i32 - 1 {
                    0
                } else {
                    self.modal_solo_selected + 1
                };
            }
            Focus::ModalDeleteBranch => {
                let alias = self.oids.get_alias_by_idx(self.graph_selected);
                let branches = self.branches.visible.entry(alias).or_default();
                let length = match get_current_branch(&self.repo) {
                    Some(current) => branches.iter().filter(|branch| current != **branch).count(),
                    None => branches.len()
                };                
                self.modal_delete_branch_selected = if self.modal_delete_branch_selected + 1 > length as i32 - 1 {
                    0
                } else {
                    self.modal_delete_branch_selected + 1
                };
            }
            _ => {}
        }
    }

    pub fn on_scroll_up_half(&mut self) {
        match self.focus {
            Focus::Viewport => if self.viewport == Viewport::Graph {
                self.graph_selected /= 2;
            },
            Focus::Branches => {
                self.branches_selected /= 2
            },
            _ => {}
        };
    }

    pub fn on_scroll_down_half(&mut self) {
        match self.focus {
            Focus::Viewport => if self.viewport == Viewport::Graph {
                self.graph_selected = (self.oids.get_commit_count() - 1)
                    .min(self.graph_selected + (self.oids.get_commit_count() - self.graph_selected) / 2);
            },
            Focus::Branches => {
                let total = self.branches.sorted.len();
                self.branches_selected = self.branches_selected + (total - self.branches_selected) / 2
            },
            _ => {}
        };
    }

    pub fn on_scroll_up_branch(&mut self) {
        if self.focus == Focus::Viewport && self.viewport == Viewport::Graph {
            let next = *self
                .branches.indices
                .iter()
                .filter(|&k| k < &self.graph_selected)
                .max()
                .unwrap_or(&self.graph_selected);
            self.graph_selected = next;
        };
    }

    pub fn on_scroll_down_branch(&mut self) {
        if self.focus == Focus::Viewport && self.viewport == Viewport::Graph {
            let next = *self
                .branches.indices
                .iter()
                .find(|&k| k > &self.graph_selected)
                .unwrap_or(&self.graph_selected);
            self.graph_selected = next;
        };
    }

    pub fn on_scroll_up_commit(&mut self) {
        if self.focus == Focus::Viewport && self.viewport == Viewport::Graph {
            let oid = self.oids.get_oid_by_idx(self.graph_selected);

            if self.oids.is_zero(oid) {
                return;
            }
            
            let child_positions: Vec<usize> = self.oids.get_sorted_aliases()
                .iter()
                .enumerate()
                .filter_map(|(idx, &alias)| {
                    let child_oid = self.oids.get_oid_by_alias(alias);
                    let commit = self.repo.find_commit(*child_oid).ok()?;
                    if commit.parent_ids().any(|parent_oid| parent_oid == *oid) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect();

            if child_positions.is_empty() {
            } else {
                self.graph_selected = child_positions[0];
            }
        }
    }

    pub fn on_scroll_down_commit(&mut self) {
        if self.focus == Focus::Viewport && self.viewport == Viewport::Graph {
            let oid = self.oids.get_oid_by_idx(self.graph_selected);

            if self.oids.is_zero(oid) {
                self.graph_selected = 1;
                return;
            }

            let commit = self.repo.find_commit(*oid).unwrap();
            let mut parents = commit.parent_ids();

            if parents.len() == 0 {
            } else {
                let parent_oid = parents.next().unwrap();
                let parent_alias = self.oids.get_alias_by_oid(parent_oid);
                let next = self.oids.get_sorted_aliases()
                    .iter()
                    .position(|&alias| alias == parent_alias)
                    .unwrap();
                self.graph_selected = next;
            }
        }
    }

    pub fn on_scroll_to_beginning(&mut self) {
        match self.focus {
            Focus::Branches => {
                self.branches_selected = 0;
            }
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    self.graph_selected = 0;
                }
                Viewport::Viewer => {
                    self.viewer_selected = 0;
                }
                Viewport::Settings => {
                    self.settings_selected = 0;
                }
                _ => {}
            },
            Focus::Inspector => {
                self.inspector_selected = 0;
            }
            Focus::StatusTop => {
                self.status_top_selected = 0;
            }
            Focus::StatusBottom => {
                self.status_bottom_selected = 0;
            }
            _ => {}
        };
    }

    pub fn on_scroll_to_end(&mut self) {
        match self.focus {
            Focus::Branches => {
                self.branches_selected = usize::MAX;
            }
            Focus::Viewport => match self.viewport {
                Viewport::Graph => {
                    self.graph_selected = usize::MAX;
                }
                Viewport::Viewer => {
                    self.viewer_selected = usize::MAX;
                }
                Viewport::Settings => {
                    self.settings_selected = usize::MAX;
                }
                _ => {}
            },
            Focus::Inspector => {
                self.inspector_selected = usize::MAX;
            }
            Focus::StatusTop => {
                self.status_top_selected = usize::MAX;
            }
            Focus::StatusBottom => {
                self.status_bottom_selected = usize::MAX;
            }
            _ => {}
        };
    }

    pub fn on_jump_to_branch(&mut self) {
        if self.focus == Focus::Branches {
            self.viewport = Viewport::Graph;
            let oidi = self.branches.sorted.get(self.branches_selected).unwrap().0;
            self.graph_selected = self.oids.get_sorted_aliases().iter().position(|o| o == &oidi).unwrap_or(0);
        };
    }
    
    pub fn on_solo_branch(&mut self) {
        match self.focus {
            Focus::Branches => {
                let (oid, branch) = self.branches.sorted.get(self.branches_selected).unwrap();

                // Check if the same branch is already the only one visible
                let already_visible = 
                    self.branches.visible.len() == 1 &&
                    self.branches.visible.entry(*oid).or_default().len() == 1 &&
                    self.branches.visible.entry(*oid).or_default().contains(branch);

                if already_visible {
                    self.branches.visible.clear();
                } else {
                    self.branches.visible.clear();
                    self.branches.visible
                        .entry(*oid)
                        .and_modify(|branches| branches.push(branch.clone()))
                        .or_insert_with(|| vec![branch.clone()]);
                }

                self.reload();
            }
            Focus::Viewport => {
                if self.focus == Focus::Viewport && self.viewport != Viewport::Graph || self.graph_selected == 0 {
                    return;
                }

                let alias = self.oids.get_alias_by_idx(self.graph_selected);

                let branches = self
                    .branches.visible
                    .get(&alias)
                    .cloned()
                    .unwrap_or_default();
                if branches.is_empty() {
                    return;
                }
                if branches.len() == 1 {
                    let branch = branches.first().unwrap();
                    if self.branches.visible.len() == 1 && self.branches.visible.entry(alias).or_default().len() == 1 && self.branches.visible.entry(alias).or_default().contains(branch) {
                        self.branches.visible.clear();
                    } else {
                        self.branches.visible.clear();
                        self.branches.visible
                            .entry(alias)
                            .and_modify(|branches| branches.push(branch.clone()))
                            .or_insert_with(|| vec![branch.clone()]);
                    }self.reload();
                } else {
                    self.focus = Focus::ModalSolo;
                }
            }
            _ => {}
        };
    }

    pub fn on_stash(&mut self) {
        if self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            let path = self.repo.path().to_path_buf();
            let mut repo = Repository::open(path).unwrap();

            // Due to incosistnent git2 api, stashing requires mutalbe repo reference, im too lazy 
            stash(&mut repo).unwrap();
            self.reload();
        }
    }

    pub fn on_grep(&mut self) {
        if self.viewport == Viewport::Graph {
            if self.focus == Focus::Viewport {
                self.focus = Focus::ModalGrep;
                self.grep_editor.mode = EditorMode::Insert;
            }
        }
    }

    pub fn on_fetch(&mut self) {
        if self.viewport != Viewport::Settings {
            let handle = fetch_over_ssh(&self.path, "origin");
            match handle.join().expect("Thread panicked") {
                Ok(_) => {
                    self.branches.visible.clear();
                    self.reload();
                }
                Err(e) => eprintln!("Fetch failed: {}", e),
            }
        }
    }

    pub fn on_checkout(&mut self) {
        if self.focus == Focus::Viewport {
            if self.focus == Focus::Viewport && self.viewport != Viewport::Graph {
                return;
            }
                        
            if self.graph_selected == 0 {
                self.focus = Focus::Viewport;
                return;
            }

            let alias = self.oids.get_alias_by_idx(self.graph_selected);
            let oid = self.oids.get_oid_by_alias(alias);
            let branches = self.branches.all.entry(alias).or_default();

            if branches.is_empty() {
                checkout_head(&self.repo, *oid);
                self.focus = Focus::Viewport;
                self.branches.visible.clear();
                self.reload();
            } else if branches.len() == 1 {
                checkout_branch(
                    &self.repo,
                    &mut self.branches.visible,
                    &mut self.branches.local,
                    alias,
                    branches.first().unwrap(),
                )
                .expect("Error");
                self.focus = Focus::Viewport;
                self.branches.visible.clear();
                self.reload();
            } else {
                self.focus = Focus::ModalCheckout;
            }
        }
    }

    pub fn on_hard_reset(&mut self) {
        if self.focus == Focus::Viewport {
            if self.focus == Focus::Viewport && self.viewport != Viewport::Graph {
                return;
            }
            let oid = self.oids.get_oid_by_idx(self.graph_selected);
            reset_to_commit(&self.repo, *oid, git2::ResetType::Hard).expect("Error");
            self.branches.visible.clear();
            self.reload();
            self.focus = Focus::Viewport;
        }
    }

    pub fn on_mixed_reset(&mut self) {
        if self.focus == Focus::Viewport {
            if self.focus == Focus::Viewport && self.viewport != Viewport::Graph {
                return;
            }
            let oid = self.oids.get_oid_by_idx(self.graph_selected);
            reset_to_commit(&self.repo, *oid, git2::ResetType::Mixed).expect("Error");
            self.branches.visible.clear();
            self.reload();
            self.focus = Focus::Viewport;
        }
    }

    pub fn on_unstage_all(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer | Viewport::Editor => {}
            _ => {
                if self.uncommitted.is_staged {
                    unstage_all(&self.repo).expect("Error");
                    self.reload();
                }
            }
        }
    }

    pub fn on_stage_all(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer | Viewport::Editor => {}
            _ => {
                if self.uncommitted.is_unstaged {
                    git_add_all(&self.repo).expect("Error");
                    self.reload();
                }
            }
        }
    }

    pub fn on_commit(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer | Viewport::Editor => {}
            _ => {
                if self.uncommitted.is_staged {
                    self.focus = Focus::ModalCommit;
                    self.commit_editor.mode = EditorMode::Insert;
                }
            }
        }
    }

    pub fn on_push(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer | Viewport::Editor => {}
            _ => {
                let handle = push_over_ssh(
                    &self.path,
                    "origin",
                    get_current_branch(&self.repo).unwrap().as_str(),
                    true,
                );
                match handle.join().expect("Thread panicked") {
                    Ok(_) => {
                        self.branches.visible.clear();
                        self.reload();
                    }
                    Err(e) => eprintln!("Fetch failed: {}", e),
                }
            }
        }
    }

    pub fn on_create_branch(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer | Viewport::Editor => {}
            _ => {
                if self.graph_selected != 0 {
                    self.focus = Focus::ModalCreateBranch;
                    self.create_branch_editor.mode = EditorMode::Insert;
                }
            }
        }
    }

    pub fn on_delete_branch(&mut self) {
        match self.viewport {
            Viewport::Settings | Viewport::Viewer | Viewport::Editor => {}
            _ => {
                match self.focus {
                    Focus::Branches => {
                        let branch = &self.branches.sorted.get(self.branches_selected).unwrap().1;
                        let proceed = if let Some(current) = get_current_branch(&self.repo) {
                            current != *branch
                        } else {
                            true
                        };
                        if proceed
                            && delete_branch(&self.repo, branch).is_ok() {
                                self.branches.visible.clear();
                                self.reload();
                            };
                    }
                    Focus::Viewport => {
                        if self.graph_selected != 0 {
                            
                            let alias = self.oids.get_alias_by_idx(if self.graph_selected == 0 { 1 } else { self.graph_selected });
                            let current = get_current_branch(&self.repo);

                            if let Some(branches) = self.branches.visible.get(&alias) {
                                // Filter out the current branch, if any
                                let filtered_branches: Vec<_> = branches
                                    .iter()
                                    .filter(|branch| current.as_ref() != Some(*branch))
                                    .collect();

                                match filtered_branches.len() {
                                    0 => {
                                    }
                                    1 => {
                                        if delete_branch(&self.repo, filtered_branches[0]).is_ok() {
                                            self.branches.visible.clear();
                                            self.reload();
                                        };
                                    }
                                    _ => {
                                        self.focus = Focus::ModalDeleteBranch;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn on_go_back(&mut self) {
        match self.focus {
            Focus::ModalCommit => {
                self.focus = Focus::Viewport;
            }
            Focus::ModalCheckout => {
                self.modal_checkout_selected = 0;
                self.focus = Focus::Viewport;
            }
            _ => {
                self.viewer_selected = 0;
                self.viewport = Viewport::Graph;
                self.focus = Focus::Viewport;
                self.file_name = None;
            },
        };
    }

    pub fn on_reload(&mut self) {
        self.reload();
        match self.focus {
            Focus::ModalCheckout | Focus::ModalCommit => {
                self.focus = Focus::Viewport;
            }
            _ => {}
        }
    }

    pub fn on_minimize(&mut self) {
        self.is_minimal = !self.is_minimal;
    }

    pub fn on_toggle_shas(&mut self) {
        if self.viewport == Viewport::Graph && self.focus == Focus::Viewport {
            self.is_shas = !self.is_shas;
        }
    }

    pub fn on_toggle_branches(&mut self) {
        self.is_branches = !self.is_branches;
        if self.viewport == Viewport::Editor || self.viewport == Viewport::Settings {
            return;
        }
        if self.is_branches {
            self.focus = Focus::Branches;
        } else {
            self.focus = Focus::Viewport;
        }
    }

    pub fn on_toggle_status(&mut self) {
        self.is_status = !self.is_status;
        if !self.is_status && (self.focus == Focus::StatusTop || self.focus == Focus::StatusBottom)
        {
            self.focus = Focus::Viewport;
        }
    }

    pub fn on_toggle_inspector(&mut self) {
        self.is_inspector = !self.is_inspector;
        if !self.is_inspector && self.focus == Focus::Inspector {
            if self.is_status {
                self.focus = Focus::StatusTop;
            } else {
                self.focus = Focus::Viewport;
            }
        }
    }

    pub fn on_toggle_settings(&mut self) {
        match self.viewport {
            Viewport::Graph => {
                self.viewport = Viewport::Settings;
                self.focus = Focus::Viewport;
            }
            _ => {
                self.viewport = Viewport::Graph;
                self.focus = Focus::Viewport;
            }
        };
    }

    pub fn on_exit(&mut self) {
        self.exit();
    }
}
