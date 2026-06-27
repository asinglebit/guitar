use std::{env, fs, io, path::PathBuf};

use guitar::{App, VERSION};

const RESET_CONFIG: &str = "--reset";
const EXIT_WHEN_GRAPH_COMPLETE: &str = "--exit-when-graph-complete";
const SKIP_WORKDIR_STATUS: &str = "--skip-workdir-status";
const VERSION_LONG: &str = "--version";
const VERSION_SHORT: &str = "-v";

fn repo_path_from_args(args: &[String]) -> Option<String> {
    args.iter().skip(1).find(|arg| !arg.starts_with('-')).cloned()
}

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

    let exit_when_graph_complete = args.iter().any(|a| a == EXIT_WHEN_GRAPH_COMPLETE);
    let skip_workdir_status = args.iter().any(|a| a == SKIP_WORKDIR_STATUS);
    let repo_path = repo_path_from_args(&args);

    if exit_when_graph_complete {
        let mut app = App::default();
        app.skip_workdir_status = true;
        app.bootstrap(repo_path);
        let result = app.wait_until_graph_complete(std::time::Duration::from_secs(600));
        app.shutdown_background_tasks();
        println!("{}", result?);
        return Ok(());
    }

    let mut app = App::default();
    app.skip_workdir_status = skip_workdir_status;
    if let Some(path) = repo_path {
        app.path = Some(path);
    }

    let mut terminal = ratatui::init();
    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}
