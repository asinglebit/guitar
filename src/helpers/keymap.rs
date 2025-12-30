use std::fs;
use std::path::Path;
#[rustfmt::skip]
use indexmap::IndexMap;
#[rustfmt::skip]
use ratatui::{
    crossterm::event::{
        KeyCode,
        KeyModifiers,
        KeyCode::*,
    }
};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    Drop,
    Pop,
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
    ToggleTags,
    ToggleStashes,
    ToggleStatus,
    ToggleInspector,
    ToggleSettings,
    Exit,
}

fn default_keymap() -> IndexMap<KeyBinding, Command> {
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
    map.insert(KeyBinding::new(Char('y'), KeyModifiers::NONE), Command::Drop);
    map.insert(KeyBinding::new(Char('t'), KeyModifiers::NONE), Command::Pop);
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
    map.insert(KeyBinding::new(Char('3'), KeyModifiers::NONE), Command::ToggleTags);
    map.insert(KeyBinding::new(Char('4'), KeyModifiers::NONE), Command::ToggleStashes);
    map.insert(KeyBinding::new(Char('2'), KeyModifiers::NONE), Command::ToggleStatus);
    map.insert(KeyBinding::new(Char('1'), KeyModifiers::NONE), Command::ToggleInspector);
    map.insert(KeyBinding::new(F(1), KeyModifiers::NONE), Command::ToggleSettings);
    map.insert(KeyBinding::new(Char('c'), KeyModifiers::CONTROL), Command::Exit);

    map
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, )]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
}

#[derive(Serialize, Deserialize)]
struct KeymapConfig {
    bindings: Vec<KeyBindingEntry>,
}

#[derive(Serialize, Deserialize)]
struct KeyBindingEntry {
    key: String,
    modifiers: Vec<String>,
    command: Command,
}

pub fn keycode_to_string(code: KeyCode) -> String {
    match code {
        KeyCode::Backspace => "Backspace".into(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Left => "Left".into(),
        KeyCode::Right => "Right".into(),
        KeyCode::Up => "Up".into(),
        KeyCode::Down => "Down".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::PageUp => "PageUp".into(),
        KeyCode::PageDown => "PageDown".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::BackTab => "BackTab".into(),
        KeyCode::Delete => "Delete".into(),
        KeyCode::Insert => "Insert".into(),
        KeyCode::F(n) => format!("F({})", n),
        KeyCode::Char(c) => format!("Char({})", c),
        KeyCode::Null => "Null".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::CapsLock => "CapsLock".into(),
        KeyCode::ScrollLock => "ScrollLock".into(),
        KeyCode::NumLock => "NumLock".into(),
        KeyCode::PrintScreen => "PrintScreen".into(),
        KeyCode::Pause => "Pause".into(),
        _ => "Unsupported".into(),
    }
}

pub fn modifiers_to_vec(mods: KeyModifiers) -> Vec<String> {
    let mut vec = Vec::new();

    if mods.contains(KeyModifiers::SHIFT) {
        vec.push("Shift".to_string());
    }
    if mods.contains(KeyModifiers::CONTROL) {
        vec.push("Control".to_string());
    }
    if mods.contains(KeyModifiers::ALT) {
        vec.push("Alt".to_string());
    }
    if mods.contains(KeyModifiers::SUPER) {
        vec.push("Command".to_string()); // SUPER maps to Command/Meta
    }

    vec
}

pub fn parse_key(s: &str) -> Result<KeyCode, String> {
    match s {
        "Backspace" => Ok(KeyCode::Backspace),
        "Enter" => Ok(KeyCode::Enter),
        "Left" => Ok(KeyCode::Left),
        "Right" => Ok(KeyCode::Right),
        "Up" => Ok(KeyCode::Up),
        "Down" => Ok(KeyCode::Down),
        "Home" => Ok(KeyCode::Home),
        "End" => Ok(KeyCode::End),
        "PageUp" => Ok(KeyCode::PageUp),
        "PageDown" => Ok(KeyCode::PageDown),
        "Tab" => Ok(KeyCode::Tab),
        "BackTab" => Ok(KeyCode::BackTab),
        "Delete" => Ok(KeyCode::Delete),
        "Insert" => Ok(KeyCode::Insert),
        "Null" => Ok(KeyCode::Null),
        "Esc" => Ok(KeyCode::Esc),
        "CapsLock" => Ok(KeyCode::CapsLock),
        "ScrollLock" => Ok(KeyCode::ScrollLock),
        "NumLock" => Ok(KeyCode::NumLock),
        "PrintScreen" => Ok(KeyCode::PrintScreen),
        "Pause" => Ok(KeyCode::Pause),
        s if s.starts_with("F(") && s.ends_with(")") => {
            let inner = &s[2..s.chars().count()-1];
            inner.parse::<u8>()
                .map(KeyCode::F)
                .map_err(|_| format!("Invalid F-key string: {}", s))
        }
        s if s.starts_with("Char(") && s.ends_with(")") => {
            let inner = &s[5..s.chars().count() - 1];
            let ch = inner
                .chars()
                .next()
                .ok_or_else(|| format!("Empty Char key: {}", s))?;
        
            Ok(KeyCode::Char(ch))
        }      
        _ => Err(format!("Unsupported key string: {}", s)),
    }
}

pub fn parse_modifiers(mods: &[String]) -> Result<KeyModifiers, String> {
    let mut km = KeyModifiers::empty();

    for m in mods {
        match m.as_str() {
            "Shift" => km |= KeyModifiers::SHIFT,
            "Control" | "Ctrl" => km |= KeyModifiers::CONTROL,
            "Alt" => km |= KeyModifiers::ALT,
            "Command" | "Meta" => km |= KeyModifiers::SUPER, // SUPER is used for Command on macOS
            "" => (), // ignore empty strings
            other => return Err(format!("Unknown modifier: {}", other)),
        }
    }

    Ok(km)
}

fn keymap_to_config(map: &IndexMap<KeyBinding, Command>) -> KeymapConfig {
    KeymapConfig {
        bindings: map.iter().map(|(kb, cmd)| {
            KeyBindingEntry {
                key: keycode_to_string(kb.code),
                modifiers: modifiers_to_vec(kb.modifiers),
                command: cmd.clone(),
            }
        }).collect(),
    }
}

fn config_to_keymap(cfg: KeymapConfig) -> Result<IndexMap<KeyBinding, Command>, String> {
    let mut map = IndexMap::new();

    for entry in cfg.bindings {
        let key = parse_key(&entry.key)?;
        let mods = parse_modifiers(&entry.modifiers)?;
        map.insert(KeyBinding::new(key, mods), entry.command);
    }

    Ok(map)
}

fn load_keymap_from_disk(path: &Path) -> Result<IndexMap<KeyBinding, Command>, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let cfg: KeymapConfig = toml::from_str(&text)?;
    Ok(config_to_keymap(cfg)?)
}

fn save_keymap_to_disk(
    path: &Path,
    map: &IndexMap<KeyBinding, Command>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = keymap_to_config(map);
    let toml = toml::to_string_pretty(&cfg)?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, toml)?;
    Ok(())
}

pub fn load_or_init_keymap() -> IndexMap<KeyBinding, Command> {
    
    let mut pathbuf = dirs::config_dir().unwrap();
    pathbuf.push("guitar");
    pathbuf.push("keymap.toml");
    let path = pathbuf.as_path();

    match load_keymap_from_disk(path) {
        Ok(map) => map,
        Err(_) => {
            let defaults = default_keymap();
            let _ = save_keymap_to_disk(path, &defaults);
            defaults
        }
    }
}
