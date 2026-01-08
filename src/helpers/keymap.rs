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

    // Vim-style scope navigation: h=widen scope (back), l=narrow scope (select)
    // This creates a horizontal mental model where h goes "out" and l goes "in"
    map.insert(
        KeyBinding::new(Char('h'), KeyModifiers::NONE),
        Command::Back,
    );
    map.insert(
        KeyBinding::new(Char('l'), KeyModifiers::NONE),
        Command::Select,
    );

    // Enter as action key -- select
    map.insert(KeyBinding::new(Enter, KeyModifiers::NONE), Command::Select);

    map.insert(KeyBinding::new(Esc, KeyModifiers::NONE), Command::Back);

    // Ctrl-P and Ctrl-N for navigating between adjacent hierarchy layers (panes)
    // Think of this as moving between sibling views rather than parent/child
    map.insert(
        KeyBinding::new(Char('p'), KeyModifiers::CONTROL),
        Command::FocusPreviousPane,
    );
    map.insert(
        KeyBinding::new(Char('n'), KeyModifiers::CONTROL),
        Command::FocusNextPane,
    );

    // Keep Tab/BackTab as fallback for muscle memory and accessibility
    map.insert(
        KeyBinding::new(Tab, KeyModifiers::NONE),
        Command::FocusNextPane,
    );
    map.insert(
        KeyBinding::new(BackTab, KeyModifiers::SHIFT),
        Command::FocusPreviousPane,
    );

    // Vim-style vertical navigation
    map.insert(
        KeyBinding::new(Char('j'), KeyModifiers::NONE),
        Command::ScrollDown,
    );
    map.insert(
        KeyBinding::new(Char('k'), KeyModifiers::NONE),
        Command::ScrollUp,
    );

    map.insert(
        KeyBinding::new(Down, KeyModifiers::NONE),
        Command::ScrollDown,
    );
    map.insert(KeyBinding::new(Up, KeyModifiers::NONE), Command::ScrollUp);

    // Vim-style half-page scroll: Ctrl-D (down half page), Ctrl-U (up half page)
    map.insert(
        KeyBinding::new(Char('d'), KeyModifiers::CONTROL),
        Command::ScrollDownHalf,
    );
    map.insert(
        KeyBinding::new(Char('u'), KeyModifiers::CONTROL),
        Command::ScrollUpHalf,
    );

    // Regular page up and page down
    map.insert(KeyBinding::new(PageUp, KeyModifiers::NONE), Command::PageUp);
    map.insert(
        KeyBinding::new(PageDown, KeyModifiers::NONE),
        Command::PageDown,
    );

    // 'g' for beginning (in vim it's 'gg', but that requires stateful key handling)
    // 'G' (Shift-g) for end
    map.insert(
        KeyBinding::new(Char('g'), KeyModifiers::NONE),
        Command::GoToBeginning,
    );
    map.insert(
        KeyBinding::new(Char('G'), KeyModifiers::SHIFT),
        Command::GoToEnd,
    );

    // The semantic obviousness here
    map.insert(
        KeyBinding::new(Home, KeyModifiers::NONE),
        Command::GoToBeginning,
    );
    map.insert(KeyBinding::new(End, KeyModifiers::NONE), Command::GoToEnd);

    // Vim-style search: '/' for find/search
    map.insert(
        KeyBinding::new(Char('/'), KeyModifiers::NONE),
        Command::Find,
    );

    // Graph-specific navigation using vim-style bracket motions
    // '{' and '}' for branch navigation (larger conceptual jumps between branches)
    map.insert(
        KeyBinding::new(Char('{'), KeyModifiers::SHIFT),
        Command::ScrollUpBranch,
    );
    map.insert(
        KeyBinding::new(Char('}'), KeyModifiers::SHIFT),
        Command::ScrollDownBranch,
    );

    // '[' and ']' for commit navigation (smaller jumps between commits)
    // Note: Using Ctrl modifier since '[' and ']' alone are hard to distinguish as Shift-[ and Shift-]
    map.insert(
        KeyBinding::new(Char('['), KeyModifiers::CONTROL),
        Command::ScrollUpCommit,
    );
    map.insert(
        KeyBinding::new(Char(']'), KeyModifiers::CONTROL),
        Command::ScrollDownCommit,
    );

    // UI panel toggles: numbered 1-6
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

    // '.' for minimize (vim uses '.' to repeat last command; here it minimizes panels)
    map.insert(
        KeyBinding::new(Char('.'), KeyModifiers::NONE),
        Command::Minimize,
    );

    // Safe git operations (non-destructive)

    // 's' for stage (git add)
    map.insert(
        KeyBinding::new(Char('s'), KeyModifiers::NONE),
        Command::Stage,
    );

    // 'u' for unstage (undo staging)
    map.insert(
        KeyBinding::new(Char('u'), KeyModifiers::NONE),
        Command::Unstage,
    );

    // 'c' for commit (git commit)
    map.insert(
        KeyBinding::new(Char('c'), KeyModifiers::NONE),
        Command::Commit,
    );

    // 'f' for fetch (git fetch)
    map.insert(
        KeyBinding::new(Char('f'), KeyModifiers::NONE),
        Command::FetchAll,
    );

    // 'b' for branch (create new branch)
    map.insert(
        KeyBinding::new(Char('b'), KeyModifiers::NONE),
        Command::CreateBranch,
    );

    // 't' for tag (create tag)
    map.insert(KeyBinding::new(Char('t'), KeyModifiers::NONE), Command::Tag);

    // 'T' (Shift-t) for toggle branch visibility in graph
    map.insert(
        KeyBinding::new(Char('T'), KeyModifiers::SHIFT),
        Command::ToggleBranch,
    );

    // Space for solo branch (similar to vim's fold toggle, shows only one branch)
    map.insert(
        KeyBinding::new(Char(' '), KeyModifiers::NONE),
        Command::SoloBranch,
    );

    // 'r' for reload (similar to vim's :e to reload file)
    map.insert(
        KeyBinding::new(Char('r'), KeyModifiers::NONE),
        Command::Reload,
    );

    // Ctrl-A to enter action mode (mnemonic: 'A' for Action)
    // This is where dangerous/destructive operations live
    map.insert(
        KeyBinding::new(Char('a'), KeyModifiers::CONTROL),
        Command::ActionMode,
    );

    // 'q' for quit (vim standard)
    map.insert(
        KeyBinding::new(Char('q'), KeyModifiers::NONE),
        Command::Exit,
    );

    map
}

fn default_action_keymap() -> IndexMap<KeyBinding, Command> {
    let mut map = IndexMap::new();

    // Keep all basic navigation in action mode
    map.insert(
        KeyBinding::new(Char('h'), KeyModifiers::NONE),
        Command::Back,
    );
    map.insert(
        KeyBinding::new(Char('l'), KeyModifiers::NONE),
        Command::Select,
    );
    map.insert(KeyBinding::new(Enter, KeyModifiers::NONE), Command::Select);
    map.insert(KeyBinding::new(Esc, KeyModifiers::NONE), Command::Back);

    map.insert(
        KeyBinding::new(Char('p'), KeyModifiers::CONTROL),
        Command::FocusPreviousPane,
    );
    map.insert(
        KeyBinding::new(Char('n'), KeyModifiers::CONTROL),
        Command::FocusNextPane,
    );
    map.insert(
        KeyBinding::new(Tab, KeyModifiers::NONE),
        Command::FocusNextPane,
    );
    map.insert(
        KeyBinding::new(BackTab, KeyModifiers::SHIFT),
        Command::FocusPreviousPane,
    );

    map.insert(
        KeyBinding::new(Char('j'), KeyModifiers::NONE),
        Command::ScrollDown,
    );
    map.insert(
        KeyBinding::new(Char('k'), KeyModifiers::NONE),
        Command::ScrollUp,
    );
    map.insert(
        KeyBinding::new(Down, KeyModifiers::NONE),
        Command::ScrollDown,
    );
    map.insert(KeyBinding::new(Up, KeyModifiers::NONE), Command::ScrollUp);

    map.insert(
        KeyBinding::new(Char('d'), KeyModifiers::CONTROL),
        Command::ScrollDownHalf,
    );
    map.insert(
        KeyBinding::new(Char('u'), KeyModifiers::CONTROL),
        Command::ScrollUpHalf,
    );

    map.insert(
        KeyBinding::new(Char('b'), KeyModifiers::CONTROL),
        Command::PageUp,
    );
    map.insert(
        KeyBinding::new(PageDown, KeyModifiers::NONE),
        Command::PageDown,
    );
    map.insert(KeyBinding::new(PageUp, KeyModifiers::NONE), Command::PageUp);

    map.insert(
        KeyBinding::new(Char('g'), KeyModifiers::NONE),
        Command::GoToBeginning,
    );
    map.insert(
        KeyBinding::new(Char('G'), KeyModifiers::SHIFT),
        Command::GoToEnd,
    );
    map.insert(
        KeyBinding::new(Home, KeyModifiers::NONE),
        Command::GoToBeginning,
    );
    map.insert(KeyBinding::new(End, KeyModifiers::NONE), Command::GoToEnd);

    // UI toggles (same as normal mode)
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
        KeyBinding::new(Char('.'), KeyModifiers::NONE),
        Command::Minimize,
    );

    // Safe git operations (same as normal mode)
    map.insert(
        KeyBinding::new(Char('s'), KeyModifiers::NONE),
        Command::Stage,
    );
    map.insert(
        KeyBinding::new(Char('u'), KeyModifiers::NONE),
        Command::Unstage,
    );
    map.insert(
        KeyBinding::new(Char('c'), KeyModifiers::NONE),
        Command::Commit,
    );
    map.insert(
        KeyBinding::new(Char('f'), KeyModifiers::NONE),
        Command::FetchAll,
    );
    map.insert(
        KeyBinding::new(Char('b'), KeyModifiers::NONE),
        Command::CreateBranch,
    );
    map.insert(KeyBinding::new(Char('t'), KeyModifiers::NONE), Command::Tag);

    // Dangerous/destructive git operations (action mode only)
    // 'x' for drop (vim uses 'x' to delete character; here delete/drop stash or commit)
    map.insert(
        KeyBinding::new(Char('x'), KeyModifiers::NONE),
        Command::Drop,
    );

    // 'p' for pop stash (vim uses 'p' for put/paste; contextually pop from stash here)
    map.insert(KeyBinding::new(Char('p'), KeyModifiers::NONE), Command::Pop);

    // 'S' (Shift-s) for stash (capital to emphasize it's a state-changing operation)
    map.insert(
        KeyBinding::new(Char('S'), KeyModifiers::SHIFT),
        Command::Stash,
    );

    // 'o' for checkout (vim uses 'o' to open line below; here "open" a different branch)
    map.insert(
        KeyBinding::new(Char('o'), KeyModifiers::NONE),
        Command::Checkout,
    );

    // 'H' (Shift-h) for hard reset (capital to indicate DANGER - destructive operation)
    map.insert(
        KeyBinding::new(Char('H'), KeyModifiers::SHIFT),
        Command::HardReset,
    );

    // 'M' (Shift-m) for mixed reset (capital to indicate caution)
    map.insert(
        KeyBinding::new(Char('M'), KeyModifiers::SHIFT),
        Command::MixedReset,
    );

    // 'P' (Shift-p) for force push (capital P to indicate DANGER)
    map.insert(
        KeyBinding::new(Char('P'), KeyModifiers::SHIFT),
        Command::ForcePush,
    );

    // 'D' (Shift-d) for delete branch (vim uses 'D' to delete to end of line)
    map.insert(
        KeyBinding::new(Char('D'), KeyModifiers::SHIFT),
        Command::DeleteBranch,
    );

    // 'U' (Shift-u) for untag (capital U to match vim's "undo whole line" conceptually)
    map.insert(
        KeyBinding::new(Char('U'), KeyModifiers::SHIFT),
        Command::Untag,
    );

    // 'y' for cherrypick (vim uses 'y' for yank; here yank/copy a commit to current branch)
    map.insert(
        KeyBinding::new(Char('y'), KeyModifiers::NONE),
        Command::Cherrypick,
    );

    map.insert(
        KeyBinding::new(Char('r'), KeyModifiers::NONE),
        Command::Reload,
    );

    map.insert(
        KeyBinding::new(Char('q'), KeyModifiers::NONE),
        Command::Exit,
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
        vec.push("Command".to_string());
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
            "Command" | "Meta" => km |= KeyModifiers::SUPER,
            "" => (),
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
