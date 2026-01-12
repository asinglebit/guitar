use std::io;
mod app {
    #[allow(clippy::module_inception)]
    pub mod app;
    pub mod app_default;
    pub mod app_draw_branches;
    pub mod app_draw_graph;
    pub mod app_draw_inspector;
    pub mod app_draw_modal_checkout;
    pub mod app_draw_modal_delete_branch;
    pub mod app_draw_modal_delete_tag;
    pub mod app_draw_modal_input;
    pub mod app_draw_modal_solo;
    pub mod app_draw_settings;
    pub mod app_draw_splash;
    pub mod app_draw_stashes;
    pub mod app_draw_status;
    pub mod app_draw_statusbar;
    pub mod app_draw_tags;
    pub mod app_draw_title;
    pub mod app_draw_viewer;
    pub mod app_input;
    pub mod app_layout;
    pub mod input;
}
mod core {
    pub mod batcher;
    pub mod branches;
    pub mod buffer;
    pub mod chunk;
    pub mod layers;
    pub mod oids;
    pub mod renderers;
    pub mod stashes;
    pub mod tags;
    pub mod walker;
}
pub mod git {
    pub mod actions {
        pub mod commits;
    }
    pub mod os {
        pub mod path;
    }
    pub mod queries {
        pub mod commits;
        pub mod diffs;
        pub mod helpers;
    }
}
pub mod helpers {
    pub mod colors;
    pub mod copy;
    pub mod heatmap;
    pub mod keymap;
    pub mod layout;
    pub mod logger;
    pub mod palette;
    pub mod recent;
    pub mod spinner;
    pub mod symbols;
    pub mod text;
    pub mod time;
    pub mod version;
}

use crate::app::app::App;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}
