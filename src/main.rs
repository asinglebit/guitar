use std::{env, fs, io, path::PathBuf};

use guitar::{App, VERSION};

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
