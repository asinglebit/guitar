use crate::helpers::keymap::InputMode;
use crate::{
    app::input::TextInput, core::stashes::Stashes, git::queries::helpers::commits_per_day,
    helpers::heatmap::build_heatmap,
};
use crate::{
    app::{
        app::{App, Focus, Viewport},
        app_layout::Layout,
    },
    core::{branches::Branches, buffer::Buffer, oids::Oids, tags::Tags},
    git::os::path::try_into_git_repo_root,
    git::queries::helpers::UncommittedChanges,
    helpers::{colors::ColorPicker, palette::*, spinner::Spinner},
};
use chrono::Utc;
use git2::Repository;
use indexmap::IndexMap;
use ratatui::{style::Style, text::Span};
use std::{cell::RefCell, env, path::PathBuf, rc::Rc};

impl Default for App {
    fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let path = if args.len() > 1 {
            &args[1]
        } else {
            &".".to_string()
        };
        let theme = Theme::default();
        let color = Rc::new(RefCell::new(ColorPicker::from_theme(&theme)));
        let canonical_path = std::fs::canonicalize(path).expect("Invalid repo path");
        let absolute_path: PathBuf =
            try_into_git_repo_root(canonical_path).unwrap_or_else(|| PathBuf::from(path));
        let repo = Rc::new(Repository::open(absolute_path.clone()).expect("Could not open repo"));
        let logo = vec![
            Span::styled("  g", Style::default().fg(theme.COLOR_GRASS)),
            Span::styled("u", Style::default().fg(theme.COLOR_GRASS)),
            Span::styled("i", Style::default().fg(theme.COLOR_GRASS)),
            Span::styled("t", Style::default().fg(theme.COLOR_GRASS)),
            Span::styled("a", Style::default().fg(theme.COLOR_GRASS)),
            Span::styled("╭", Style::default().fg(theme.COLOR_GREEN)),
        ];
        let heatmap = {
            let counts = commits_per_day(&repo);
            let today = Utc::now().date_naive();
            build_heatmap(&counts, today)
        };

        App {
            // General
            logo,
            path: absolute_path.display().to_string(),
            repo,
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

            // Interface
            layout: Layout::default(),

            // Focus
            is_shas: false,
            is_minimal: false,
            is_branches: false,
            is_tags: false,
            is_stashes: false,
            is_status: false,
            is_inspector: false,
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
