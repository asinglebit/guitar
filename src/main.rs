use std::{env, fs, io, path::PathBuf};
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
    pub mod app_draw_modal_error;
    pub mod app_draw_modal_input;
    pub mod app_draw_modal_rebase;
    pub mod app_draw_modal_remove_worktree;
    pub mod app_draw_modal_solo;
    pub mod app_draw_modal_worktree_chooser;
    pub mod app_draw_settings;
    pub mod app_draw_splash;
    pub mod app_draw_stashes;
    pub mod app_draw_status;
    pub mod app_draw_statusbar;
    pub mod app_draw_tags;
    pub mod app_draw_title;
    pub mod app_draw_viewer;
    pub mod app_draw_worktrees;
    pub mod app_input;
    pub mod app_layout;
    pub mod input;
    pub mod input_events;
    pub mod input_git;
    pub mod input_modals;
    pub mod input_navigation;
    pub mod input_worktrees;
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
    pub mod worktrees;
}
pub mod git {
    pub mod actions {
        pub mod branching;
        pub mod checkout;
        pub mod cherrypicking;
        pub mod committing;
        pub mod conflicts;
        pub mod fetching;
        pub mod pushing;
        pub mod rebasing;
        pub mod resetting;
        pub mod staging;
        pub mod stashing;
        pub mod tagging;
        pub mod worktrees;
    }
    pub mod os {
        pub mod path;
    }
    pub mod queries {
        pub mod commits;
        pub mod diffs;
        pub mod helpers;
        pub mod worktrees;
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

use crate::{app::app::App, helpers::version::VERSION};

const RESET_CONFIG: &str = "--reset";
const VERSION_LONG: &str = "--version";
const VERSION_SHORT: &str = "-v";

fn guitar_config_dir() -> io::Result<PathBuf> {
    let mut path = dirs::config_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find config directory"))?;
    path.push("guitar");
    Ok(path)
}

fn reset_saved_config() -> io::Result<()> {
    let path = guitar_config_dir()?;
    if path.is_dir() {
        fs::remove_dir_all(&path)?;
    } else if path.exists() {
        fs::remove_file(&path)?;
    }
    println!("Reset saved guitar config at {}", path.display());
    Ok(())
}

fn main() -> io::Result<()> {
    // Meta flags are handled before ratatui takes over the terminal.
    let args: Vec<String> = env::args().collect();

    // Version output must stay plain so scripts can consume it.
    if args.iter().any(|a| a == VERSION_LONG || a == VERSION_SHORT) {
        println!("{VERSION}");
        return Ok(());
    }

    if args.iter().any(|a| a == RESET_CONFIG) {
        reset_saved_config()?;
    }

    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}
