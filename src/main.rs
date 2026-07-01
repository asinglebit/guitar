use std::{env, fs, io, path::PathBuf};

use guitar::{App, VERSION, helpers::symbols::load_symbol_theme};

const RESET_CONFIG: &str = "--reset";
const EXIT_WHEN_GRAPH_COMPLETE: &str = "--exit-when-graph-complete";
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
    let mut repo_arg = None;
    let mut print_version = false;
    let mut reset_config = false;
    let mut exit_when_graph_complete = false;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            VERSION_LONG | VERSION_SHORT => print_version = true,
            RESET_CONFIG => reset_config = true,
            EXIT_WHEN_GRAPH_COMPLETE => exit_when_graph_complete = true,
            _ if repo_arg.is_none() && !arg.starts_with('-') => repo_arg = Some(arg),
            _ => {},
        }
    }

    // Version output must stay plain so scripts can consume it.
    if print_version {
        println!("{VERSION}");
        return Ok(());
    }

    if reset_config {
        reset_saved_config()?;
    }

    if exit_when_graph_complete {
        let mut app = App::with_symbol_theme(load_symbol_theme());
        app.bootstrap(repo_arg);
        let result = app.wait_until_graph_complete(std::time::Duration::from_secs(600));
        app.shutdown_background_tasks();
        println!("{}", result?);
        return Ok(());
    }

    let mut app = App::with_symbol_theme(load_symbol_theme());
    if let Some(path) = repo_arg {
        app.path = Some(path);
    }

    let mut terminal = ratatui::init();
    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}
