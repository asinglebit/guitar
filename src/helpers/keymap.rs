use indexmap::IndexMap;
use ratatui::crossterm::event::{KeyCode, KeyCode::*, KeyModifiers};
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputMode {
    Normal,
    Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Command {
    // User Interface
    FocusNextPane,
    FocusPreviousPane,
    Select,
    Back,
    Minimize,
    ToggleBranches,
    ToggleTags,
    ToggleStashes,
    ToggleStatus,
    ToggleInspector,
    ToggleShas,
    ToggleSettings,
    ActionMode,
    Exit,

    // Lists
    PageUp,
    PageDown,
    ScrollUp,
    ScrollDown,
    ScrollUpHalf,
    ScrollDownHalf,
    GoToBeginning,
    GoToEnd,

    // Graph
    ScrollUpBranch,
    ScrollDownBranch,
    ScrollUpCommit,
    ScrollDownCommit,
    Find,

    // Git
    Drop,
    Pop,
    Stash,
    FetchAll,
    Checkout,
    HardReset,
    MixedReset,
    Unstage,
    Stage,
    Commit,
    ForcePush,
    SoloBranch,
    ToggleBranch,
    CreateBranch,
    DeleteBranch,
    Tag,
    Untag,
    Cherrypick,
    Reload,
}

pub type ModeKeymap = IndexMap<KeyBinding, Command>;
pub type Keymaps = IndexMap<InputMode, ModeKeymap>;

fn default_normal_keymap() -> IndexMap<KeyBinding, Command> {
    let mut map = IndexMap::new();

    // User Interface
    map.insert(
        KeyBinding::new(Tab, KeyModifiers::NONE),
        Command::FocusNextPane,
    );
    map.insert(
        KeyBinding::new(Right, KeyModifiers::NONE),
        Command::FocusNextPane,
    );
    map.insert(
        KeyBinding::new(Char('l'), KeyModifiers::NONE),
        Command::FocusNextPane,
    );
    map.insert(
        KeyBinding::new(BackTab, KeyModifiers::SHIFT),
        Command::FocusPreviousPane,
    );
    map.insert(
        KeyBinding::new(Left, KeyModifiers::NONE),
        Command::FocusPreviousPane,
    );
    map.insert(
        KeyBinding::new(Char('h'), KeyModifiers::NONE),
        Command::FocusPreviousPane,
    );
    map.insert(KeyBinding::new(Enter, KeyModifiers::NONE), Command::Select);
    map.insert(KeyBinding::new(Esc, KeyModifiers::NONE), Command::Back);
    map.insert(
        KeyBinding::new(Char('.'), KeyModifiers::NONE),
        Command::Minimize,
    );
    map.insert(
        KeyBinding::new(Char('1'), KeyModifiers::NONE),
        Command::ToggleBranches,
    );
    map.insert(
        KeyBinding::new(Char('2'), KeyModifiers::NONE),
        Command::ToggleTags,
    );
    map.insert(
        KeyBinding::new(Char('3'), KeyModifiers::NONE),
        Command::ToggleStashes,
    );
    map.insert(
        KeyBinding::new(Char('4'), KeyModifiers::NONE),
        Command::ToggleStatus,
    );
    map.insert(
        KeyBinding::new(Char('5'), KeyModifiers::NONE),
        Command::ToggleInspector,
    );
    map.insert(
        KeyBinding::new(Char('6'), KeyModifiers::NONE),
        Command::ToggleShas,
    );
    map.insert(
        KeyBinding::new(F(1), KeyModifiers::NONE),
        Command::ToggleSettings,
    );
    map.insert(
        KeyBinding::new(Char(' '), KeyModifiers::CONTROL),
        Command::ActionMode,
    );
    map.insert(
        KeyBinding::new(Char('q'), KeyModifiers::NONE),
        Command::Exit,
    );
    map.insert(
        KeyBinding::new(Char('c'), KeyModifiers::CONTROL),
        Command::Exit,
    );
    map.insert(
        KeyBinding::new(Char('r'), KeyModifiers::NONE),
        Command::Reload,
    );

    // Lists
    map.insert(KeyBinding::new(PageUp, KeyModifiers::NONE), Command::PageUp);
    map.insert(
        KeyBinding::new(PageDown, KeyModifiers::NONE),
        Command::PageDown,
    );
    map.insert(
        KeyBinding::new(Char('k'), KeyModifiers::NONE),
        Command::ScrollUp,
    );
    map.insert(
        KeyBinding::new(Char('j'), KeyModifiers::NONE),
        Command::ScrollDown,
    );
    map.insert(KeyBinding::new(Up, KeyModifiers::NONE), Command::ScrollUp);
    map.insert(
        KeyBinding::new(Down, KeyModifiers::NONE),
        Command::ScrollDown,
    );
    map.insert(
        KeyBinding::new(Char('k'), KeyModifiers::CONTROL | KeyModifiers::ALT),
        Command::ScrollUpHalf,
    );
    map.insert(
        KeyBinding::new(Char('j'), KeyModifiers::CONTROL | KeyModifiers::ALT),
        Command::ScrollDownHalf,
    );
    map.insert(
        KeyBinding::new(Up, KeyModifiers::CONTROL | KeyModifiers::ALT),
        Command::ScrollUpHalf,
    );
    map.insert(
        KeyBinding::new(Down, KeyModifiers::CONTROL | KeyModifiers::ALT),
        Command::ScrollDownHalf,
    );
    map.insert(
        KeyBinding::new(Home, KeyModifiers::NONE),
        Command::GoToBeginning,
    );
    map.insert(KeyBinding::new(End, KeyModifiers::NONE), Command::GoToEnd);

    // Graph
    map.insert(
        KeyBinding::new(Char('k'), KeyModifiers::CONTROL),
        Command::ScrollUpBranch,
    );
    map.insert(
        KeyBinding::new(Char('j'), KeyModifiers::CONTROL),
        Command::ScrollDownBranch,
    );
    map.insert(
        KeyBinding::new(Up, KeyModifiers::CONTROL),
        Command::ScrollUpBranch,
    );
    map.insert(
        KeyBinding::new(Down, KeyModifiers::CONTROL),
        Command::ScrollDownBranch,
    );
    map.insert(
        KeyBinding::new(Char('k'), KeyModifiers::ALT),
        Command::ScrollUpCommit,
    );
    map.insert(
        KeyBinding::new(Char('j'), KeyModifiers::ALT),
        Command::ScrollDownCommit,
    );
    map.insert(
        KeyBinding::new(Up, KeyModifiers::ALT),
        Command::ScrollUpCommit,
    );
    map.insert(
        KeyBinding::new(Down, KeyModifiers::ALT),
        Command::ScrollDownCommit,
    );
    map.insert(
        KeyBinding::new(Char('f'), KeyModifiers::CONTROL),
        Command::Find,
    );
    map.insert(
        KeyBinding::new(Char(' '), KeyModifiers::NONE),
        Command::SoloBranch,
    );
    map.insert(
        KeyBinding::new(Char('t'), KeyModifiers::NONE),
        Command::ToggleBranch,
    );

    // Git
    map.insert(
        KeyBinding::new(Char('f'), KeyModifiers::NONE),
        Command::FetchAll,
    );
    map.insert(
        KeyBinding::new(Char('u'), KeyModifiers::NONE),
        Command::Unstage,
    );
    map.insert(
        KeyBinding::new(Char('s'), KeyModifiers::NONE),
        Command::Stage,
    );
    map.insert(
        KeyBinding::new(Char('a'), KeyModifiers::NONE),
        Command::Commit,
    );
    map.insert(
        KeyBinding::new(Char('b'), KeyModifiers::NONE),
        Command::CreateBranch,
    );
    map.insert(KeyBinding::new(Char('/'), KeyModifiers::NONE), Command::Tag);

    map
}

fn default_action_keymap() -> IndexMap<KeyBinding, Command> {
    let mut map = IndexMap::new();

    // User Interface
    map.insert(
        KeyBinding::new(Tab, KeyModifiers::NONE),
        Command::FocusNextPane,
    );
    map.insert(
        KeyBinding::new(BackTab, KeyModifiers::SHIFT),
        Command::FocusPreviousPane,
    );
    map.insert(
        KeyBinding::new(Right, KeyModifiers::NONE),
        Command::FocusNextPane,
    );
    map.insert(
        KeyBinding::new(Left, KeyModifiers::NONE),
        Command::FocusPreviousPane,
    );
    map.insert(KeyBinding::new(Enter, KeyModifiers::NONE), Command::Select);
    map.insert(KeyBinding::new(Esc, KeyModifiers::NONE), Command::Back);
    map.insert(
        KeyBinding::new(Char('.'), KeyModifiers::NONE),
        Command::Minimize,
    );
    map.insert(
        KeyBinding::new(Char('1'), KeyModifiers::NONE),
        Command::ToggleBranches,
    );
    map.insert(
        KeyBinding::new(Char('2'), KeyModifiers::NONE),
        Command::ToggleTags,
    );
    map.insert(
        KeyBinding::new(Char('3'), KeyModifiers::NONE),
        Command::ToggleStashes,
    );
    map.insert(
        KeyBinding::new(Char('4'), KeyModifiers::NONE),
        Command::ToggleStatus,
    );
    map.insert(
        KeyBinding::new(Char('5'), KeyModifiers::NONE),
        Command::ToggleInspector,
    );
    map.insert(
        KeyBinding::new(Char('6'), KeyModifiers::NONE),
        Command::ToggleShas,
    );
    map.insert(
        KeyBinding::new(F(1), KeyModifiers::NONE),
        Command::ToggleSettings,
    );
    map.insert(
        KeyBinding::new(Char('q'), KeyModifiers::NONE),
        Command::Exit,
    );
    map.insert(
        KeyBinding::new(Char('c'), KeyModifiers::CONTROL),
        Command::Exit,
    );
    map.insert(
        KeyBinding::new(Char('r'), KeyModifiers::NONE),
        Command::Reload,
    );

    // Lists
    map.insert(KeyBinding::new(PageUp, KeyModifiers::NONE), Command::PageUp);
    map.insert(
        KeyBinding::new(PageDown, KeyModifiers::NONE),
        Command::PageDown,
    );
    map.insert(KeyBinding::new(Up, KeyModifiers::NONE), Command::ScrollUp);
    map.insert(
        KeyBinding::new(Down, KeyModifiers::NONE),
        Command::ScrollDown,
    );
    map.insert(
        KeyBinding::new(Up, KeyModifiers::CONTROL | KeyModifiers::ALT),
        Command::ScrollUpHalf,
    );
    map.insert(
        KeyBinding::new(Down, KeyModifiers::CONTROL | KeyModifiers::ALT),
        Command::ScrollDownHalf,
    );
    map.insert(
        KeyBinding::new(Home, KeyModifiers::NONE),
        Command::GoToBeginning,
    );
    map.insert(KeyBinding::new(End, KeyModifiers::NONE), Command::GoToEnd);

    // Git
    map.insert(
        KeyBinding::new(Char('f'), KeyModifiers::NONE),
        Command::FetchAll,
    );
    map.insert(
        KeyBinding::new(Char('u'), KeyModifiers::NONE),
        Command::Unstage,
    );
    map.insert(
        KeyBinding::new(Char('s'), KeyModifiers::NONE),
        Command::Stage,
    );
    map.insert(
        KeyBinding::new(Char('a'), KeyModifiers::NONE),
        Command::Commit,
    );
    map.insert(
        KeyBinding::new(Char('b'), KeyModifiers::NONE),
        Command::CreateBranch,
    );
    map.insert(KeyBinding::new(Char('/'), KeyModifiers::NONE), Command::Tag);

    // Git, but dangerous
    map.insert(
        KeyBinding::new(Char('y'), KeyModifiers::NONE),
        Command::Drop,
    );
    map.insert(KeyBinding::new(Char('t'), KeyModifiers::NONE), Command::Pop);
    map.insert(
        KeyBinding::new(Char('e'), KeyModifiers::NONE),
        Command::Stash,
    );
    map.insert(
        KeyBinding::new(Char('c'), KeyModifiers::NONE),
        Command::Checkout,
    );
    map.insert(
        KeyBinding::new(Char('h'), KeyModifiers::NONE),
        Command::HardReset,
    );
    map.insert(
        KeyBinding::new(Char('m'), KeyModifiers::NONE),
        Command::MixedReset,
    );
    map.insert(
        KeyBinding::new(Char('p'), KeyModifiers::NONE),
        Command::ForcePush,
    );
    map.insert(
        KeyBinding::new(Char('d'), KeyModifiers::NONE),
        Command::DeleteBranch,
    );
    map.insert(
        KeyBinding::new(Char('?'), KeyModifiers::NONE),
        Command::Untag,
    );
    map.insert(
        KeyBinding::new(Char(']'), KeyModifiers::NONE),
        Command::Cherrypick,
    );

    map
}

fn default_keymaps() -> Keymaps {
    let mut maps = IndexMap::new();

    maps.insert(InputMode::Normal, default_normal_keymap());
    maps.insert(InputMode::Action, default_action_keymap());

    maps
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    normal: Vec<KeyBindingEntry>,
    action: Vec<KeyBindingEntry>,
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

pub fn keycode_to_visual_string(code: KeyCode) -> String {
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
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) => format!("{c}"),
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
            let inner = &s[2..s.chars().count() - 1];
            inner
                .parse::<u8>()
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
            "" => (),                                        // ignore empty strings
            other => return Err(format!("Unknown modifier: {}", other)),
        }
    }

    Ok(km)
}

fn keymap_entries(map: &ModeKeymap) -> Vec<KeyBindingEntry> {
    map.iter()
        .map(|(kb, cmd)| KeyBindingEntry {
            key: keycode_to_string(kb.code),
            modifiers: modifiers_to_vec(kb.modifiers),
            command: cmd.clone(),
        })
        .collect()
}

fn keymaps_to_config(maps: &Keymaps) -> KeymapConfig {
    let normal = maps
        .get(&InputMode::Normal)
        .map(|m| keymap_entries(m))
        .unwrap_or_default();

    let action = maps
        .get(&InputMode::Action)
        .map(|m| keymap_entries(m))
        .unwrap_or_default();

    KeymapConfig { normal, action }
}

fn entries_to_keymap(entries: Vec<KeyBindingEntry>) -> Result<ModeKeymap, String> {
    let mut map = IndexMap::new();

    for entry in entries {
        let key = parse_key(&entry.key)?;
        let mods = parse_modifiers(&entry.modifiers)?;
        map.insert(KeyBinding::new(key, mods), entry.command);
    }

    Ok(map)
}

fn config_to_keymaps(cfg: KeymapConfig) -> Result<Keymaps, String> {
    let mut maps = IndexMap::new();

    maps.insert(InputMode::Normal, entries_to_keymap(cfg.normal)?);
    maps.insert(InputMode::Action, entries_to_keymap(cfg.action)?);

    Ok(maps)
}

fn load_keymaps_from_disk(path: &Path) -> Result<Keymaps, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let cfg: KeymapConfig = serde_json::from_str(&text)?;
    Ok(config_to_keymaps(cfg)?)
}

fn save_keymaps_to_disk(path: &Path, maps: &Keymaps) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = keymaps_to_config(maps);
    let json = serde_json::to_string_pretty(&cfg)?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_or_init_keymaps() -> Keymaps {
    let mut pathbuf = dirs::config_dir().unwrap();
    pathbuf.push("guitar");
    pathbuf.push("keymap.json");
    let path = pathbuf.as_path();

    match load_keymaps_from_disk(path) {
        Ok(maps) => maps,
        Err(_) => {
            let defaults = default_keymaps();
            let _ = save_keymaps_to_disk(path, &defaults);
            defaults
        }
    }
}
