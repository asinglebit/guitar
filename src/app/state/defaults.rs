use crate::helpers::heatmap::empty_heatmap;
use crate::helpers::keymap::InputMode;
use crate::helpers::layout::load_layout_config;
use crate::{
    app::input::TextInput,
    core::{reflogs::HeadReflogs, stashes::Stashes, submodules::Submodules, worktrees::Worktrees},
};
use crate::{
    app::{
        app::{App, AuthInputField, BranchModalAction, Focus, OperationKind, RemoteInputAction, SettingsTab, Viewport, WorktreeModalAction},
        state::layout::Layout,
    },
    core::{branches::Branches, oids::Oids, tags::Tags},
    git::queries::helpers::UncommittedChanges,
    helpers::{
        colors::ColorPicker,
        localisation::{Language, set_active_language},
        palette::*,
        spinner::Spinner,
        symbols::SymbolTheme,
    },
};
use indexmap::IndexMap;
use ratatui::{style::Style, text::Span, widgets::ListItem};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub struct SplitViewerRow {
    pub left: ListItem<'static>,
    pub right: ListItem<'static>,
    pub unified_indices: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewerMode {
    Full,
    Hunks,
    Split,
}

impl Default for App {
    fn default() -> Self {
        let theme = Theme::default();
        let symbols = SymbolTheme::default();
        let language = Language::English;
        set_active_language(language);
        let color = Rc::new(RefCell::new(ColorPicker::from_theme(&theme)));
        let heatmap = empty_heatmap();
        let logo = vec![
            Span::styled(symbols.splash.logo_word_prefix.clone(), Style::default().fg(theme.COLOR_GRASS)),
            Span::styled(symbols.splash.logo_corner.clone(), Style::default().fg(theme.COLOR_GREEN)),
        ];

        App {
            // General
            logo,
            path: None,
            recent: Vec::new(),
            repo: None,
            spinner: Spinner::new(),
            keymaps: IndexMap::new(),
            mode: InputMode::Normal,
            last_input_direction: None,
            theme,
            symbols,
            language,
            heatmap,

            // User
            name: String::new(),
            email: String::new(),

            // Walker utilities
            color,
            graph: Default::default(),
            graph_tx: None,
            graph_rx: None,
            walker_cancel: None,
            walker_handle: None,

            // Walker data
            oids: Oids::default(),
            branches: Branches::default(),
            tags: Tags::default(),
            stashes: Stashes::default(),
            reflogs: HeadReflogs::default(),
            worktrees: Worktrees::default(),
            submodules: Submodules::default(),
            submodule_stack: Vec::new(),
            uncommitted: UncommittedChanges::default(),

            // Cache
            current_diff: Vec::new(),
            current_diff_identity: None,
            is_uncommitted_loaded: false,
            file_name: None,
            viewer_lines: Vec::new(),
            viewer_split_rows: Vec::new(),
            viewer_edges: Vec::new(),      // line numbers where hunks start and end
            viewer_hunks: Vec::new(),      // indices of changed lines the belong to hunks
            viewer_mode: ViewerMode::Full, // Viewer mode: Full, Hunks, or Split
            is_viewer_layout_dirty: false,
            viewer_layout_signature: None,

            // Interface
            layout: Layout::default(),

            // Focus
            layout_config: load_layout_config(),
            mouse_drag: None,
            last_mouse_click: None,
            context_menu: None,
            modal_area: None,
            viewport: Viewport::Splash,
            focus: Focus::Viewport,

            // Branches
            branches_selected: 0,
            branches_scroll: 0.into(),

            // Tags
            tags_selected: 0,
            tags_scroll: 0.into(),

            // Stashes
            stashes_selected: 0,
            stashes_scroll: 0.into(),

            // Reflogs
            reflogs_selected: 0,
            reflogs_scroll: 0.into(),

            // Worktrees
            worktrees_selected: 0,
            worktrees_scroll: 0.into(),

            // Submodules
            submodules_selected: 0,
            submodules_scroll: 0.into(),

            // Search
            search_path: None,
            search_rows: Vec::new(),
            search_is_loading: false,
            search_error: None,
            search_request_id: None,
            search_selected: 0,
            search_scroll: 0.into(),

            // Graph
            graph_selected: 0,
            graph_scroll: 0.into(),

            // Splash
            splash_selected: 0,
            recent_save_path: None,

            // Settings
            settings_tab: SettingsTab::General,
            settings_selected: 0,
            settings_scroll: 0.into(),
            settings_selections: Vec::new(),
            settings_tab_hitboxes: Vec::new(),
            modal_key_capture_selection: None,
            modal_key_capture_candidate: None,
            modal_key_capture_error: None,
            keymap_save_path: None,
            symbol_theme_save_path: None,
            language_save_path: None,

            // Viewer
            viewer_selected: 0,
            viewer_scroll: 0.into(),

            // Inspector
            inspector_selected: 0,
            inspector_scroll: 0.into(),

            // Status top
            status_top_selected: 0,
            status_top_scroll: 0.into(),

            // Status bottom
            status_bottom_selected: 0,
            status_bottom_scroll: 0.into(),

            // Modal checkout
            modal_checkout_selected: 0,

            // Modal solo
            modal_solo_selected: 0,
            modal_branch_action: BranchModalAction::Solo,

            // Modal editor
            modal_input: TextInput::default(),
            pending_cherrypick_oid: None,
            pending_revert_oid: None,
            pending_branch_target_oid: None,
            modal_rename_branch_source: None,
            modal_worktree_name: String::new(),
            modal_worktree_selected: 0,
            modal_worktree_candidates: Vec::new(),
            modal_worktree_target: None,
            modal_worktree_action: WorktreeModalAction::Open,
            modal_worktree_return_focus: Focus::Viewport,
            modal_remote_selected: 0,
            modal_remote_target: None,
            modal_remote_input_action: RemoteInputAction::AddName,
            modal_remote_name: String::new(),
            modal_file_search_results: Vec::new(),
            modal_file_search_selected: 0,
            modal_file_search_scroll: 0.into(),
            modal_file_search_return_focus: Focus::Viewport,

            // Modal delete branch
            modal_delete_branch_selected: 0,

            // Modal delete tag
            modal_delete_tag_selected: 0,

            // Modal error
            modal_error_message: String::new(),
            modal_error_return_focus: Focus::Viewport,
            modal_operation_kind: OperationKind::Rebase,
            modal_operation_message: String::new(),
            pending_operation_action: None,

            // Modal network operation and authentication prompts.
            auth_session: Default::default(),
            pending_network_request: None,
            network_handle: None,
            network_auth_attempts: 0,
            pending_auth_prompt: None,
            auth_username_input: TextInput::default(),
            auth_secret_input: TextInput::default(),
            auth_input_field: AuthInputField::Username,
            modal_network_title: String::new(),
            modal_network_message: String::new(),

            // Exit
            is_exit: false,
        }
    }
}
