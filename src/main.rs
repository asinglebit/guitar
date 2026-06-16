use std::{env, fs, io, path::PathBuf};

mod app;
mod core {
    pub mod batcher;
    pub mod branches;
    pub mod buffer;
    pub mod chunk;
    pub mod graph_service;
    pub mod layers;
    pub mod oids;
    pub mod reflogs;
    pub mod renderers;
    pub mod stashes;
    pub mod tags;
    pub mod walker;
    pub mod worktrees;
}
pub mod git {
    pub mod auth;
    pub mod actions {
        pub mod branching;
        pub mod checkout;
        pub mod cherrypicking;
        pub mod committing;
        pub mod conflicts;
        pub mod fetching;
        pub mod merging;
        pub mod network;
        pub mod pushing;
        pub mod rebasing;
        pub mod resetting;
        pub mod reverting;
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
        pub mod reflogs;
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
