use crate::helpers::heatmap::empty_heatmap;
use crate::helpers::keymap::InputMode;
use crate::helpers::layout::load_layout_config;
use crate::{app::input::TextInput, core::stashes::Stashes};
use crate::{
    app::{
        app::{App, Focus, Viewport},
        app_layout::Layout,
    },
    core::{branches::Branches, buffer::Buffer, oids::Oids, tags::Tags},
    git::queries::helpers::UncommittedChanges,
    helpers::{colors::ColorPicker, palette::*, spinner::Spinner},
};
use indexmap::IndexMap;
use ratatui::{style::Style, text::Span};
use std::{cell::RefCell, rc::Rc};

pub enum ViewerMode {
    Full,
    Hunks,
}

impl Default for App {
    fn default() -> Self {
        let theme = Theme::default();
        let color = Rc::new(RefCell::new(ColorPicker::from_theme(&theme)));
        let heatmap = empty_heatmap();
        let logo = vec![Span::styled("  guita", Style::default().fg(theme.COLOR_GRASS)), Span::styled("╭", Style::default().fg(theme.COLOR_GREEN))];

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
            heatmap,

            // User
            name: String::new(),
            email: String::new(),

            // Walker utilities
            color,
            buffer: RefCell::new(Buffer::default()),
            walker_rx: None,
            walker_cancel: None,
            walker_handle: None,

            // Walker data
            oids: Oids::default(),
            branches: Branches::default(),
            tags: Tags::default(),
            stashes: Stashes::default(),
            uncommitted: UncommittedChanges::default(),

            // Cache
            current_diff: Vec::new(),
            file_name: None,
            viewer_lines: Vec::new(),
            viewer_edges: Vec::new(),      // line numbers where hunks start and end
            viewer_hunks: Vec::new(),      // indices of changed lines the belong to hunks
            viewer_mode: ViewerMode::Full, // Viewer mode: Full or Hunks

            // Interface
            layout: Layout::default(),

            // Focus
            layout_config: load_layout_config(),
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

            // Graph
            graph_selected: 0,
            graph_scroll: 0.into(),

            // Splash
            splash_selected: 0,
            splash_selections: Vec::new(),

            // Settings
            settings_selected: 0,
            settings_selections: Vec::new(),

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

            // Modal editor
            modal_input: TextInput::default(),

            // Modal delete branch
            modal_delete_branch_selected: 0,

            // Modal delete tag
            modal_delete_tag_selected: 0,

            // Exit
            is_exit: false,
        }
    }
}
