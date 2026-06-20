use crate::helpers::{
    localisation::{self, keymap as keymap_text},
    text::modifiers_to_string,
};
use facet::Facet;
use indexmap::IndexMap;
use ratatui::crossterm::event::{KeyCode, KeyCode::*, KeyModifiers};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Facet)]
#[repr(C)]
pub enum InputMode {
    Normal,
    Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
#[repr(C)]
pub enum Command {
    // User Interface
    WidenScope,
    NarrowScope,
    FocusNextPane,
    FocusPreviousPane,
    FocusPaneLeft,
    FocusPaneDown,
    FocusPaneUp,
    FocusPaneRight,
    Select,
    Back,
    Minimize,
    ResetLayout,
    ShrinkGraphLaneLimit,
    GrowGraphLaneLimit,
    ResizePaneLeft,
    ResizePaneDown,
    ResizePaneUp,
    ResizePaneRight,
    ToggleZenMode,
    ToggleBranches,
    ToggleTags,
    ToggleStashes,
    ToggleReflogs,
    ToggleGraphReflogs,
    ToggleGraphDates,
    ToggleGraphCommitters,
    ToggleGraphRefs,
    ToggleWorktrees,
    ToggleSubmodules,
    ToggleSearch,
    ToggleStatus,
    ToggleInspector,
    ToggleShas,
    ToggleHelp,
    ActionMode,
    Exit,
    RemoveRecentRepository,
    MoveRecentRepositoryUp,
    MoveRecentRepositoryDown,
    ReturnToParentRepository,

    // Lists
    ScrollPageUp,
    ScrollPageDown,
    ScrollHalfPageUp,
    ScrollHalfPageDown,
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
    FindFile,

    // Viewer
    ToggleHunkMode,
    ToggleSplitDiffMode,

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
    PushTags,
    SoloBranch,
    ToggleBranch,
    CreateBranch,
    DeleteBranch,
    RenameBranch,
    Tag,
    Untag,
    Cherrypick,
    Revert,
    Rebase,
    Merge,
    ContinueOperation,
    AbortOperation,
    CreateWorktree,
    RemoveWorktree,
    ToggleWorktreeLock,
    UpdateSubmodule,
    SyncSubmodule,
    Reload,
    ReloadAllBranches,
}

pub type ModeKeymap = IndexMap<KeyBinding, Command>;
pub type Keymaps = IndexMap<InputMode, ModeKeymap>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapSelection {
    pub mode: InputMode,
    pub key: KeyBinding,
    pub command: Command,
}

impl KeymapSelection {
    pub fn new(mode: InputMode, key: KeyBinding, command: Command) -> Self {
        Self { mode, key, command }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapEditError {
    MissingMode(InputMode),
    MissingBinding { mode: InputMode, key: KeyBinding },
    CommandChanged { mode: InputMode, key: KeyBinding, expected: Command, actual: Command },
    Conflict { mode: InputMode, key: KeyBinding, command: Command },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapEditOutcome {
    pub synced_action: bool,
}

pub fn action_keymap_visible_entries(normal: Option<&ModeKeymap>, action: &ModeKeymap) -> ModeKeymap {
    action.iter().filter(|(kb, cmd)| normal.and_then(|normal| normal.get(*kb)) != Some(*cmd)).map(|(kb, cmd)| (kb.clone(), cmd.clone())).collect()
}

pub fn input_mode_to_visual_string(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Normal => keymap_text::NORMAL_MODE(),
        InputMode::Action => keymap_text::ACTION_MODE(),
    }
}

pub fn command_to_visual_string(command: &Command) -> String {
    let label = match command {
        Command::WidenScope => "Widen scope",
        Command::NarrowScope => "Narrow scope",
        Command::FocusNextPane => "Focus next pane",
        Command::FocusPreviousPane => "Focus previous pane",
        Command::FocusPaneLeft => "Focus pane left",
        Command::FocusPaneDown => "Focus pane down",
        Command::FocusPaneUp => "Focus pane up",
        Command::FocusPaneRight => "Focus pane right",
        Command::Select => "Select",
        Command::Back => "Back",
        Command::Minimize => "Minimize",
        Command::ResetLayout => "Reset layout",
        Command::ShrinkGraphLaneLimit => "Shrink graph lane limit",
        Command::GrowGraphLaneLimit => "Grow graph lane limit",
        Command::ResizePaneLeft => "Resize pane left",
        Command::ResizePaneDown => "Resize pane down",
        Command::ResizePaneUp => "Resize pane up",
        Command::ResizePaneRight => "Resize pane right",
        Command::ToggleZenMode => "Toggle zen mode",
        Command::ToggleBranches => "Toggle branches",
        Command::ToggleTags => "Toggle tags",
        Command::ToggleStashes => "Toggle stashes",
        Command::ToggleReflogs => "Toggle reflogs",
        Command::ToggleGraphReflogs => "Toggle graph reflogs",
        Command::ToggleGraphDates => "Toggle graph dates",
        Command::ToggleGraphCommitters => "Toggle graph committers",
        Command::ToggleGraphRefs => "Toggle graph refs",
        Command::ToggleWorktrees => "Toggle worktrees",
        Command::ToggleSubmodules => "Toggle submodules",
        Command::ToggleSearch => "Toggle search",
        Command::ToggleStatus => "Toggle status",
        Command::ToggleInspector => "Toggle inspector",
        Command::ToggleShas => "Toggle SHAs",
        Command::ToggleHelp => "Toggle help",
        Command::ActionMode => "Action mode",
        Command::Exit => "Exit",
        Command::RemoveRecentRepository => "Remove recent repository",
        Command::MoveRecentRepositoryUp => "Move recent repository up",
        Command::MoveRecentRepositoryDown => "Move recent repository down",
        Command::ReturnToParentRepository => "Return to parent repository",
        Command::ScrollPageUp => "Scroll page up",
        Command::ScrollPageDown => "Scroll page down",
        Command::ScrollHalfPageUp => "Scroll half page up",
        Command::ScrollHalfPageDown => "Scroll half page down",
        Command::ScrollUp => "Scroll up",
        Command::ScrollDown => "Scroll down",
        Command::ScrollUpHalf => "Scroll up half",
        Command::ScrollDownHalf => "Scroll down half",
        Command::GoToBeginning => "Go to beginning",
        Command::GoToEnd => "Go to end",
        Command::ScrollUpBranch => "Scroll up branch",
        Command::ScrollDownBranch => "Scroll down branch",
        Command::ScrollUpCommit => "Scroll up commit",
        Command::ScrollDownCommit => "Scroll down commit",
        Command::Find => "Find",
        Command::FindFile => "Find file",
        Command::ToggleHunkMode => "Toggle hunk mode",
        Command::ToggleSplitDiffMode => "Toggle split diff mode",
        Command::Drop => "Drop stash",
        Command::Pop => "Pop stash",
        Command::Stash => "Stash changes",
        Command::FetchAll => "Fetch all",
        Command::Checkout => "Checkout",
        Command::HardReset => "Hard reset",
        Command::MixedReset => "Mixed reset",
        Command::Unstage => "Unstage file",
        Command::Stage => "Stage file",
        Command::Commit => "Commit",
        Command::ForcePush => "Push",
        Command::PushTags => "Push tags",
        Command::SoloBranch => "Solo branch",
        Command::ToggleBranch => "Toggle branch",
        Command::CreateBranch => "Create branch",
        Command::DeleteBranch => "Delete branch",
        Command::RenameBranch => "Rename branch",
        Command::Tag => "Create tag",
        Command::Untag => "Delete tag",
        Command::Cherrypick => "Cherry-pick",
        Command::Revert => "Revert",
        Command::Rebase => "Rebase",
        Command::Merge => "Merge",
        Command::ContinueOperation => "Continue operation",
        Command::AbortOperation => "Abort operation",
        Command::CreateWorktree => "Create worktree",
        Command::RemoveWorktree => "Remove worktree",
        Command::ToggleWorktreeLock => "Toggle worktree lock",
        Command::UpdateSubmodule => "Update submodule",
        Command::SyncSubmodule => "Sync URL",
        Command::Reload => "Reload",
        Command::ReloadAllBranches => "Reload all branches",
    };
    localisation::command_label(label).to_string()
}

fn shifted_digit_visual_char(c: char) -> Option<char> {
    match c {
        '0' => Some(')'),
        '1' => Some('!'),
        '2' => Some('@'),
        '3' => Some('#'),
        '4' => Some('$'),
        '5' => Some('%'),
        '6' => Some('^'),
        '7' => Some('&'),
        '8' => Some('*'),
        '9' => Some('('),
        _ => None,
    }
}

fn shifted_digit_visual_binding(binding: &KeyBinding) -> Option<KeyBinding> {
    let KeyCode::Char(c) = binding.code else {
        return None;
    };
    let shifted = shifted_digit_visual_char(c)?;
    if !binding.modifiers.contains(KeyModifiers::SHIFT) {
        return None;
    }

    let mut modifiers = binding.modifiers;
    modifiers.remove(KeyModifiers::SHIFT);
    Some(KeyBinding::new(KeyCode::Char(shifted), modifiers))
}

pub fn keybinding_to_visual_string(binding: &KeyBinding) -> String {
    if let Some(visual_binding) = shifted_digit_visual_binding(binding) {
        return keybinding_to_visual_string(&visual_binding);
    }

    let mut key_string = modifiers_to_string(binding.modifiers);
    if !key_string.is_empty() {
        key_string = format!("{} + ", key_string);
    }
    key_string.push_str(&keycode_to_visual_string(binding.code));
    key_string
}

fn shifted_digit_alias(binding: &KeyBinding) -> Option<KeyBinding> {
    let KeyCode::Char(c) = binding.code else {
        return None;
    };

    let digit = match c {
        ')' | '=' => '0',
        '!' => '1',
        '@' | '"' => '2',
        '#' | '§' => '3',
        '$' => '4',
        _ => return None,
    };

    let mut modifiers = binding.modifiers;
    modifiers.remove(KeyModifiers::SHIFT);
    Some(KeyBinding::new(KeyCode::Char(digit), modifiers | KeyModifiers::SHIFT))
}

fn legacy_ctrl_digit_alias(binding: &KeyBinding) -> Option<KeyBinding> {
    match binding {
        KeyBinding { code: KeyCode::Char('0'..='4'), modifiers } if *modifiers == KeyModifiers::CONTROL => Some(KeyBinding::new(binding.code, KeyModifiers::SHIFT)),
        _ => None,
    }
}

fn legacy_ctrl_binding_for_shift_digit(binding: &KeyBinding) -> Option<KeyBinding> {
    match binding {
        KeyBinding { code: KeyCode::Char('0'..='4'), modifiers } if *modifiers == KeyModifiers::SHIFT => Some(KeyBinding::new(binding.code, KeyModifiers::CONTROL)),
        _ => None,
    }
}

pub fn command_for_key_binding(map: &ModeKeymap, binding: &KeyBinding) -> Option<Command> {
    map.get(binding)
        .cloned()
        .or_else(|| {
            shifted_digit_alias(binding).and_then(|alias| map.get(&alias).cloned().or_else(|| legacy_ctrl_binding_for_shift_digit(&alias).and_then(|legacy_alias| map.get(&legacy_alias).cloned())))
        })
        .or_else(|| legacy_ctrl_digit_alias(binding).and_then(|alias| map.get(&alias).cloned()))
        .or_else(|| legacy_ctrl_binding_for_shift_digit(binding).and_then(|alias| map.get(&alias).cloned()))
}

fn rebind_key_in_mode(maps: &mut Keymaps, mode: InputMode, old_key: &KeyBinding, command: &Command, new_key: &KeyBinding) -> Result<(), KeymapEditError> {
    let Some(mode_map) = maps.get_mut(&mode) else {
        return Err(KeymapEditError::MissingMode(mode));
    };

    let Some(actual) = mode_map.get(old_key).cloned() else {
        return Err(KeymapEditError::MissingBinding { mode, key: old_key.clone() });
    };

    if actual != *command {
        return Err(KeymapEditError::CommandChanged { mode, key: old_key.clone(), expected: command.clone(), actual });
    }

    if old_key == new_key {
        return Ok(());
    }

    match mode_map.get(new_key) {
        Some(existing) if existing != command => {
            return Err(KeymapEditError::Conflict { mode, key: new_key.clone(), command: existing.clone() });
        },
        Some(_) => {
            let mut updated = IndexMap::with_capacity(mode_map.len().saturating_sub(1));
            for (key, value) in mode_map.iter() {
                if key != old_key {
                    updated.insert(key.clone(), value.clone());
                }
            }
            *mode_map = updated;
            return Ok(());
        },
        None => {},
    }

    let mut updated = IndexMap::with_capacity(mode_map.len());
    for (key, value) in mode_map.iter() {
        if key == old_key {
            updated.insert(new_key.clone(), value.clone());
        } else {
            updated.insert(key.clone(), value.clone());
        }
    }
    *mode_map = updated;
    Ok(())
}

fn should_sync_action_binding(maps: &Keymaps, selection: &KeymapSelection) -> bool {
    selection.mode == InputMode::Normal && maps.get(&InputMode::Action).and_then(|action| action.get(&selection.key)) == Some(&selection.command)
}

pub fn rebind_keymap_selection(maps: &mut Keymaps, selection: &KeymapSelection, new_key: KeyBinding) -> Result<KeymapEditOutcome, KeymapEditError> {
    let synced_action = should_sync_action_binding(maps, selection);
    let mut updated = maps.clone();

    rebind_key_in_mode(&mut updated, selection.mode, &selection.key, &selection.command, &new_key)?;

    if synced_action {
        rebind_key_in_mode(&mut updated, InputMode::Action, &selection.key, &selection.command, &new_key)?;
    }

    *maps = updated;
    Ok(KeymapEditOutcome { synced_action })
}

fn default_navigation_keymap() -> IndexMap<KeyBinding, Command> {
    let mut map = IndexMap::new();

    // Scope navigation (Vim-style)
    // This creates a horizontal mental model where h goes "out" and l goes "in"

    // 'h' = widen scope
    map.insert(KeyBinding::new(Char('h'), KeyModifiers::NONE), Command::WidenScope);

    // 'l' = narrow scope
    map.insert(KeyBinding::new(Char('l'), KeyModifiers::NONE), Command::NarrowScope);

    // [Left] = widen scope
    map.insert(KeyBinding::new(Left, KeyModifiers::NONE), Command::WidenScope);

    // [Right] = narrow scope
    map.insert(KeyBinding::new(Right, KeyModifiers::NONE), Command::NarrowScope);

    // Primary action keys

    // [Enter] = select
    map.insert(KeyBinding::new(Enter, KeyModifiers::NONE), Command::Select);

    // [Esc] = back
    map.insert(KeyBinding::new(Esc, KeyModifiers::NONE), Command::Back);

    // Navigating between adjacent hierarchy layers (panes)
    // Think of this as moving between sibling views rather than parent / child

    // [Ctrl] + 'p' = previous pane
    map.insert(KeyBinding::new(Char('p'), KeyModifiers::CONTROL), Command::FocusPreviousPane);

    // [Ctrl] + 'n' = next pane
    map.insert(KeyBinding::new(Char('n'), KeyModifiers::CONTROL), Command::FocusNextPane);

    // Alternative pane navigation for muscle memory and accessibility

    // [Tab] = next pane
    map.insert(KeyBinding::new(Tab, KeyModifiers::NONE), Command::FocusNextPane);

    // [Shift] + [Tab] = previous pane
    map.insert(KeyBinding::new(BackTab, KeyModifiers::SHIFT), Command::FocusPreviousPane);

    // Directional pane focus (Vim-style direction keys)
    map.insert(KeyBinding::new(Char('h'), KeyModifiers::CONTROL), Command::FocusPaneLeft);
    map.insert(KeyBinding::new(Char('j'), KeyModifiers::CONTROL), Command::FocusPaneDown);
    map.insert(KeyBinding::new(Char('k'), KeyModifiers::CONTROL), Command::FocusPaneUp);
    map.insert(KeyBinding::new(Char('l'), KeyModifiers::CONTROL), Command::FocusPaneRight);
    map.insert(KeyBinding::new(Left, KeyModifiers::CONTROL), Command::FocusPaneLeft);
    map.insert(KeyBinding::new(Down, KeyModifiers::CONTROL), Command::FocusPaneDown);
    map.insert(KeyBinding::new(Up, KeyModifiers::CONTROL), Command::FocusPaneUp);
    map.insert(KeyBinding::new(Right, KeyModifiers::CONTROL), Command::FocusPaneRight);

    // Pane resizing (Vim-style direction keys)
    map.insert(KeyBinding::new(Char('h'), KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneLeft);
    map.insert(KeyBinding::new(Char('j'), KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneDown);
    map.insert(KeyBinding::new(Char('k'), KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneUp);
    map.insert(KeyBinding::new(Char('l'), KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneRight);
    map.insert(KeyBinding::new(Left, KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneLeft);
    map.insert(KeyBinding::new(Down, KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneDown);
    map.insert(KeyBinding::new(Up, KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneUp);
    map.insert(KeyBinding::new(Right, KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneRight);

    // Vertical navigation (Vim-style)

    // 'j' = scroll down
    map.insert(KeyBinding::new(Char('j'), KeyModifiers::NONE), Command::ScrollDown);

    // 'k' = scroll up
    map.insert(KeyBinding::new(Char('k'), KeyModifiers::NONE), Command::ScrollUp);

    // Vertical navigation (arrows)

    // [Down] = scroll down
    map.insert(KeyBinding::new(Down, KeyModifiers::NONE), Command::ScrollDown);

    // [Up] = scroll up
    map.insert(KeyBinding::new(Up, KeyModifiers::NONE), Command::ScrollUp);

    // [Ctrl] + [Alt] + 'd' = down
    map.insert(KeyBinding::new(Char('d'), KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ScrollDownHalf);

    // [Ctrl] + [Alt] + 'u' = up
    map.insert(KeyBinding::new(Char('u'), KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ScrollUpHalf);

    // Half-page scrolling (Vim-style)

    // [Ctrl] + 'd' = down
    map.insert(KeyBinding::new(Char('d'), KeyModifiers::CONTROL), Command::ScrollHalfPageDown);

    // [Ctrl] + 'u' = up
    map.insert(KeyBinding::new(Char('u'), KeyModifiers::CONTROL), Command::ScrollHalfPageUp);

    // Regular page up and page down

    // [Page Up] = up
    map.insert(KeyBinding::new(PageUp, KeyModifiers::NONE), Command::ScrollPageUp);

    // [Page Down] = down
    map.insert(KeyBinding::new(PageDown, KeyModifiers::NONE), Command::ScrollPageDown);

    // Full-page scrolling (Vim-style)

    // 'g' goes to beginning because single-key handling cannot distinguish vim's 'gg' yet.
    map.insert(KeyBinding::new(Char('g'), KeyModifiers::NONE), Command::GoToBeginning);

    // 'G' for end
    map.insert(KeyBinding::new(Char('G'), KeyModifiers::SHIFT), Command::GoToEnd);

    // Semantically obvious full-page scrolling

    // [HOME] for beginning of the list
    map.insert(KeyBinding::new(Home, KeyModifiers::NONE), Command::GoToBeginning);

    // [END] for end of the list
    map.insert(KeyBinding::new(End, KeyModifiers::NONE), Command::GoToEnd);

    // Search (Vim-style)

    // '/' Open the search modal
    map.insert(KeyBinding::new(Char('/'), KeyModifiers::NONE), Command::Find);

    // [Shift] + 'F' opens repository file search.
    map.insert(KeyBinding::new(Char('F'), KeyModifiers::SHIFT), Command::FindFile);

    // Graph-specific navigation (Vim-style)

    // '{' for branch navigation, larger conceptual jumps to a newer branch
    map.insert(KeyBinding::new(Char('{'), KeyModifiers::NONE), Command::ScrollUpBranch);

    // '}' for branch navigation, larger conceptual jumps to an older branch
    map.insert(KeyBinding::new(Char('}'), KeyModifiers::NONE), Command::ScrollDownBranch);

    // '[' for commit navigation, smaller jumps to a newer commit in the topology
    map.insert(KeyBinding::new(Char('['), KeyModifiers::NONE), Command::ScrollUpCommit);

    // ']' for commit navigation, smaller jumps to an older commit in the topology
    map.insert(KeyBinding::new(Char(']'), KeyModifiers::NONE), Command::ScrollDownCommit);

    // Viewer specific navigation

    // 'm' to toggle viewer mode, between full and hunks only view
    map.insert(KeyBinding::new(Char('m'), KeyModifiers::NONE), Command::ToggleHunkMode);

    // 'v' toggles the side-by-side diff viewer
    map.insert(KeyBinding::new(Char('v'), KeyModifiers::NONE), Command::ToggleSplitDiffMode);

    // UI toggles

    // 'z' for zen mode
    map.insert(KeyBinding::new(Char('z'), KeyModifiers::NONE), Command::ToggleZenMode);
    map.insert(KeyBinding::new(Char('0'), KeyModifiers::NONE), Command::ResetLayout);
    map.insert(KeyBinding::new(Char('1'), KeyModifiers::NONE), Command::ToggleBranches);
    map.insert(KeyBinding::new(Char('2'), KeyModifiers::NONE), Command::ToggleTags);
    map.insert(KeyBinding::new(Char('3'), KeyModifiers::NONE), Command::ToggleStashes);
    map.insert(KeyBinding::new(Char('4'), KeyModifiers::NONE), Command::ToggleReflogs);
    map.insert(KeyBinding::new(Char('5'), KeyModifiers::NONE), Command::ToggleWorktrees);
    map.insert(KeyBinding::new(Char('6'), KeyModifiers::NONE), Command::ToggleSubmodules);
    map.insert(KeyBinding::new(Char('7'), KeyModifiers::NONE), Command::ToggleSearch);
    map.insert(KeyBinding::new(Char('8'), KeyModifiers::NONE), Command::ToggleInspector);
    map.insert(KeyBinding::new(Char('9'), KeyModifiers::NONE), Command::ToggleStatus);
    map.insert(KeyBinding::new(Char('0'), KeyModifiers::SHIFT), Command::ToggleGraphReflogs);
    map.insert(KeyBinding::new(Char('1'), KeyModifiers::SHIFT), Command::ToggleShas);
    map.insert(KeyBinding::new(Char('2'), KeyModifiers::SHIFT), Command::ToggleGraphDates);
    map.insert(KeyBinding::new(Char('3'), KeyModifiers::SHIFT), Command::ToggleGraphCommitters);
    map.insert(KeyBinding::new(Char('4'), KeyModifiers::SHIFT), Command::ToggleGraphRefs);

    // Help and settings
    map.insert(KeyBinding::new(Char('?'), KeyModifiers::NONE), Command::ToggleHelp);

    // Ctrl-A to enter action mode (mnemonic: 'A' for Action)
    // This is where dangerous/destructive operations live
    map.insert(KeyBinding::new(Char('a'), KeyModifiers::CONTROL), Command::ActionMode);

    // '.' minimizes panels, trading vim repeat behavior for a compact UI command.
    map.insert(KeyBinding::new(Char('.'), KeyModifiers::NONE), Command::Minimize);

    // 'r' for reload (similar to vim's :e to reload file)
    map.insert(KeyBinding::new(Char('r'), KeyModifiers::NONE), Command::Reload);

    // 'R' reloads and clears branch visibility filters.
    map.insert(KeyBinding::new(Char('R'), KeyModifiers::SHIFT), Command::ReloadAllBranches);

    // 'q' for quit (vim standard)
    map.insert(KeyBinding::new(Char('q'), KeyModifiers::NONE), Command::Exit);

    map
}

fn default_normal_keymap() -> IndexMap<KeyBinding, Command> {
    let mut map = default_navigation_keymap();

    // Safe git operations (non-destructive)

    // 's' for stage (git add)
    map.insert(KeyBinding::new(Char('s'), KeyModifiers::NONE), Command::Stage);

    // 'u' for unstage (undo staging)
    map.insert(KeyBinding::new(Char('u'), KeyModifiers::NONE), Command::Unstage);

    // 'c' for commit (git commit)
    map.insert(KeyBinding::new(Char('c'), KeyModifiers::NONE), Command::Commit);

    // 'f' for fetch (git fetch)
    map.insert(KeyBinding::new(Char('f'), KeyModifiers::NONE), Command::FetchAll);

    // 'b' for branch (create new branch)
    map.insert(KeyBinding::new(Char('b'), KeyModifiers::NONE), Command::CreateBranch);

    // 't' for tag (create tag)
    map.insert(KeyBinding::new(Char('t'), KeyModifiers::NONE), Command::Tag);

    // 'w' for worktree creation
    map.insert(KeyBinding::new(Char('w'), KeyModifiers::NONE), Command::CreateWorktree);

    // 'T' for toggling selected branch visibility.
    map.insert(KeyBinding::new(Char('T'), KeyModifiers::SHIFT), Command::ToggleBranch);

    // Space for solo branch, keeping only the selected branch visible.
    map.insert(KeyBinding::new(Char(' '), KeyModifiers::NONE), Command::SoloBranch);

    // 'd' removes the selected splash recent repository entry.
    map.insert(KeyBinding::new(Char('d'), KeyModifiers::NONE), Command::RemoveRecentRepository);

    // Shift+K/J reorder the selected recent repository without stealing normal list navigation.
    map.insert(KeyBinding::new(Char('K'), KeyModifiers::SHIFT), Command::MoveRecentRepositoryUp);
    map.insert(KeyBinding::new(Char('J'), KeyModifiers::SHIFT), Command::MoveRecentRepositoryDown);

    // Backspace returns from a submodule to its parent repository when there is a session stack.
    map.insert(KeyBinding::new(Backspace, KeyModifiers::NONE), Command::ReturnToParentRepository);

    // '-' and '+' adjust the graph lane cap without entering settings.
    map.insert(KeyBinding::new(Char('-'), KeyModifiers::NONE), Command::ShrinkGraphLaneLimit);
    map.insert(KeyBinding::new(Char('+'), KeyModifiers::NONE), Command::GrowGraphLaneLimit);

    map
}

fn default_action_keymap() -> IndexMap<KeyBinding, Command> {
    // Keep all basic navigation in action mode

    let mut map = default_normal_keymap();
    map.retain(|_, command| !matches!(command, Command::ShrinkGraphLaneLimit | Command::GrowGraphLaneLimit));

    // Dangerous/destructive git operations (action mode only)

    // 'x' for drop (vim uses 'x' to delete character; here delete/drop stash or commit)
    map.insert(KeyBinding::new(Char('x'), KeyModifiers::NONE), Command::Drop);

    // 'p' for pop stash (vim uses 'p' for put/paste; contextually pop from stash here)
    map.insert(KeyBinding::new(Char('p'), KeyModifiers::NONE), Command::Pop);

    // 'S' for stash (capital to emphasize it's a state-changing operation)
    map.insert(KeyBinding::new(Char('S'), KeyModifiers::SHIFT), Command::Stash);

    // 'o' for checkout (vim uses 'o' to open line below; here "open" a different branch)
    map.insert(KeyBinding::new(Char('o'), KeyModifiers::NONE), Command::Checkout);

    // 'H' for hard reset (capital to indicate DANGER - destructive operation)
    map.insert(KeyBinding::new(Char('H'), KeyModifiers::SHIFT), Command::HardReset);

    // 'M' for mixed reset (capital to indicate caution)
    map.insert(KeyBinding::new(Char('M'), KeyModifiers::SHIFT), Command::MixedReset);

    // 'P' for force push (capital P to indicate DANGER)
    map.insert(KeyBinding::new(Char('P'), KeyModifiers::SHIFT), Command::ForcePush);

    // 'V' for push tags ("versions"), guarded by action mode because tags are shared refs
    map.insert(KeyBinding::new(Char('V'), KeyModifiers::SHIFT), Command::PushTags);

    // 'D' for delete branch (vim uses 'D' to delete to end of line)
    map.insert(KeyBinding::new(Char('D'), KeyModifiers::SHIFT), Command::DeleteBranch);

    // 'B' renames a local branch; paired with normal-mode 'b' branch creation.
    map.insert(KeyBinding::new(Char('B'), KeyModifiers::SHIFT), Command::RenameBranch);

    // 'U' for untag (capital U to match vim's "undo whole line" conceptually)
    map.insert(KeyBinding::new(Char('U'), KeyModifiers::SHIFT), Command::Untag);

    // 'y' for cherrypick (vim uses 'y' for yank here yank/copy a commit to current branch)
    map.insert(KeyBinding::new(Char('y'), KeyModifiers::NONE), Command::Cherrypick);

    // 'R' for revert (capital to indicate caution)
    map.insert(KeyBinding::new(Char('R'), KeyModifiers::SHIFT), Command::Revert);

    // 'r' starts a rebase from action mode.
    map.insert(KeyBinding::new(Char('r'), KeyModifiers::NONE), Command::Rebase);

    // 'm' merges the selected commit into the current branch.
    map.insert(KeyBinding::new(Char('m'), KeyModifiers::NONE), Command::Merge);

    // 'C' continues an in-progress rebase, cherry-pick, or merge from action mode.
    map.insert(KeyBinding::new(Char('C'), KeyModifiers::SHIFT), Command::ContinueOperation);

    // 'A' aborts an in-progress rebase, cherry-pick, or merge from action mode.
    map.insert(KeyBinding::new(Char('A'), KeyModifiers::SHIFT), Command::AbortOperation);

    // 'W' removes/prunes a selected worktree.
    map.insert(KeyBinding::new(Char('W'), KeyModifiers::SHIFT), Command::RemoveWorktree);

    // 'L' toggles a selected linked worktree lock.
    map.insert(KeyBinding::new(Char('L'), KeyModifiers::SHIFT), Command::ToggleWorktreeLock);

    // 'i' updates/initializes the selected submodule.
    map.insert(KeyBinding::new(Char('i'), KeyModifiers::NONE), Command::UpdateSubmodule);

    // 'I' syncs the selected submodule URL into its local config.
    map.insert(KeyBinding::new(Char('I'), KeyModifiers::SHIFT), Command::SyncSubmodule);

    map
}

fn default_keymaps() -> Keymaps {
    let mut maps = IndexMap::new();

    maps.insert(InputMode::Normal, default_normal_keymap());
    maps.insert(InputMode::Action, default_action_keymap());

    maps
}

fn old_numeric_ui_defaults() -> Vec<(KeyBinding, Command)> {
    vec![
        (KeyBinding::new(Char('0'), KeyModifiers::NONE), Command::ResetLayout),
        (KeyBinding::new(Char('1'), KeyModifiers::NONE), Command::ToggleBranches),
        (KeyBinding::new(Char('2'), KeyModifiers::NONE), Command::ToggleTags),
        (KeyBinding::new(Char('3'), KeyModifiers::NONE), Command::ToggleStashes),
        (KeyBinding::new(Char('4'), KeyModifiers::NONE), Command::ToggleStatus),
        (KeyBinding::new(Char('5'), KeyModifiers::NONE), Command::ToggleInspector),
        (KeyBinding::new(Char('6'), KeyModifiers::NONE), Command::ToggleWorktrees),
        (KeyBinding::new(Char('7'), KeyModifiers::NONE), Command::ToggleReflogs),
        (KeyBinding::new(Char('8'), KeyModifiers::NONE), Command::ToggleShas),
        (KeyBinding::new(Char('9'), KeyModifiers::NONE), Command::ToggleGraphReflogs),
        (KeyBinding::new(Char('\\'), KeyModifiers::NONE), Command::ToggleSubmodules),
        (KeyBinding::new(Char('`'), KeyModifiers::NONE), Command::ToggleSearch),
    ]
}

fn new_numeric_ui_defaults() -> Vec<(KeyBinding, Command)> {
    vec![
        (KeyBinding::new(Char('0'), KeyModifiers::NONE), Command::ResetLayout),
        (KeyBinding::new(Char('1'), KeyModifiers::NONE), Command::ToggleBranches),
        (KeyBinding::new(Char('2'), KeyModifiers::NONE), Command::ToggleTags),
        (KeyBinding::new(Char('3'), KeyModifiers::NONE), Command::ToggleStashes),
        (KeyBinding::new(Char('4'), KeyModifiers::NONE), Command::ToggleReflogs),
        (KeyBinding::new(Char('5'), KeyModifiers::NONE), Command::ToggleWorktrees),
        (KeyBinding::new(Char('6'), KeyModifiers::NONE), Command::ToggleSubmodules),
        (KeyBinding::new(Char('7'), KeyModifiers::NONE), Command::ToggleSearch),
        (KeyBinding::new(Char('8'), KeyModifiers::NONE), Command::ToggleInspector),
        (KeyBinding::new(Char('9'), KeyModifiers::NONE), Command::ToggleStatus),
    ]
    .into_iter()
    .chain(shift_graph_metadata_defaults())
    .collect()
}

fn ctrl_graph_metadata_defaults() -> Vec<(KeyBinding, Command)> {
    vec![
        (KeyBinding::new(Char('0'), KeyModifiers::CONTROL), Command::ToggleGraphReflogs),
        (KeyBinding::new(Char('1'), KeyModifiers::CONTROL), Command::ToggleShas),
        (KeyBinding::new(Char('2'), KeyModifiers::CONTROL), Command::ToggleGraphDates),
        (KeyBinding::new(Char('3'), KeyModifiers::CONTROL), Command::ToggleGraphCommitters),
        (KeyBinding::new(Char('4'), KeyModifiers::CONTROL), Command::ToggleGraphRefs),
    ]
}

fn shift_graph_metadata_defaults() -> Vec<(KeyBinding, Command)> {
    vec![
        (KeyBinding::new(Char('0'), KeyModifiers::SHIFT), Command::ToggleGraphReflogs),
        (KeyBinding::new(Char('1'), KeyModifiers::SHIFT), Command::ToggleShas),
        (KeyBinding::new(Char('2'), KeyModifiers::SHIFT), Command::ToggleGraphDates),
        (KeyBinding::new(Char('3'), KeyModifiers::SHIFT), Command::ToggleGraphCommitters),
        (KeyBinding::new(Char('4'), KeyModifiers::SHIFT), Command::ToggleGraphRefs),
    ]
}

fn keymap_can_rewrite_default_group(map: &ModeKeymap, old_defaults: &[(KeyBinding, Command)], new_defaults: &[(KeyBinding, Command)]) -> bool {
    old_defaults.iter().all(|(key, command)| map.get(key) == Some(command))
        && new_defaults.iter().all(|(new_key, new_command)| old_defaults.iter().any(|(old_key, _)| old_key == new_key) || !map.contains_key(new_key) || map.get(new_key) == Some(new_command))
}

fn rewrite_default_group(map: &mut ModeKeymap, old_defaults: &[(KeyBinding, Command)], new_defaults: &[(KeyBinding, Command)]) -> bool {
    if !keymap_can_rewrite_default_group(map, old_defaults, new_defaults) {
        return false;
    }

    let mut updated = IndexMap::with_capacity(map.len().saturating_add(new_defaults.len()));
    let mut inserted = false;

    for (key, command) in map.iter() {
        if old_defaults.iter().any(|(old_key, _)| old_key == key) {
            if !inserted {
                for (new_key, new_command) in new_defaults {
                    updated.insert(new_key.clone(), new_command.clone());
                }
                inserted = true;
            }
        } else {
            updated.insert(key.clone(), command.clone());
        }
    }

    *map = updated;
    true
}

fn rewrite_untouched_old_numeric_ui_defaults(map: &mut ModeKeymap) -> bool {
    rewrite_default_group(map, &old_numeric_ui_defaults(), &new_numeric_ui_defaults())
}

fn rewrite_untouched_ctrl_graph_metadata_defaults(map: &mut ModeKeymap) -> bool {
    rewrite_default_group(map, &ctrl_graph_metadata_defaults(), &shift_graph_metadata_defaults())
}

fn insert_default_binding_if_available(map: &mut ModeKeymap, key: KeyBinding, command: Command) -> bool {
    if map.values().any(|existing| existing == &command) || map.contains_key(&key) {
        false
    } else {
        map.insert(key, command);
        true
    }
}

fn ensure_default_keymap_bindings(maps: &mut Keymaps) -> bool {
    let mut changed = false;
    let shared_defaults = [
        (KeyBinding::new(Char('F'), KeyModifiers::SHIFT), Command::FindFile),
        (KeyBinding::new(Char('4'), KeyModifiers::NONE), Command::ToggleReflogs),
        (KeyBinding::new(Char('5'), KeyModifiers::NONE), Command::ToggleWorktrees),
        (KeyBinding::new(Char('6'), KeyModifiers::NONE), Command::ToggleSubmodules),
        (KeyBinding::new(Char('7'), KeyModifiers::NONE), Command::ToggleSearch),
        (KeyBinding::new(Char('8'), KeyModifiers::NONE), Command::ToggleInspector),
        (KeyBinding::new(Char('9'), KeyModifiers::NONE), Command::ToggleStatus),
        (KeyBinding::new(Char('0'), KeyModifiers::SHIFT), Command::ToggleGraphReflogs),
        (KeyBinding::new(Char('1'), KeyModifiers::SHIFT), Command::ToggleShas),
        (KeyBinding::new(Char('2'), KeyModifiers::SHIFT), Command::ToggleGraphDates),
        (KeyBinding::new(Char('3'), KeyModifiers::SHIFT), Command::ToggleGraphCommitters),
        (KeyBinding::new(Char('4'), KeyModifiers::SHIFT), Command::ToggleGraphRefs),
        (KeyBinding::new(Left, KeyModifiers::CONTROL), Command::FocusPaneLeft),
        (KeyBinding::new(Down, KeyModifiers::CONTROL), Command::FocusPaneDown),
        (KeyBinding::new(Up, KeyModifiers::CONTROL), Command::FocusPaneUp),
        (KeyBinding::new(Right, KeyModifiers::CONTROL), Command::FocusPaneRight),
        (KeyBinding::new(Left, KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneLeft),
        (KeyBinding::new(Down, KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneDown),
        (KeyBinding::new(Up, KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneUp),
        (KeyBinding::new(Right, KeyModifiers::CONTROL | KeyModifiers::ALT), Command::ResizePaneRight),
    ];
    for mode in [InputMode::Normal, InputMode::Action] {
        let mode_map = maps.entry(mode).or_default();
        if rewrite_untouched_old_numeric_ui_defaults(mode_map) {
            changed = true;
        }
        if rewrite_untouched_ctrl_graph_metadata_defaults(mode_map) {
            changed = true;
        }
        for (key, command) in shared_defaults.iter() {
            if insert_default_binding_if_available(mode_map, key.clone(), command.clone()) {
                changed = true;
            }
        }
    }
    let normal_map = maps.entry(InputMode::Normal).or_default();
    let reload_all_key = KeyBinding::new(Char('R'), KeyModifiers::SHIFT);
    if !normal_map.values().any(|existing| existing == &Command::ReloadAllBranches) && !normal_map.contains_key(&reload_all_key) {
        normal_map.insert(reload_all_key, Command::ReloadAllBranches);
        changed = true;
    }
    let return_parent_key = KeyBinding::new(Backspace, KeyModifiers::NONE);
    if !normal_map.values().any(|existing| existing == &Command::ReturnToParentRepository) && !normal_map.contains_key(&return_parent_key) {
        normal_map.insert(return_parent_key, Command::ReturnToParentRepository);
        changed = true;
    }
    let normal_only_defaults = [(KeyBinding::new(Char('-'), KeyModifiers::NONE), Command::ShrinkGraphLaneLimit), (KeyBinding::new(Char('+'), KeyModifiers::NONE), Command::GrowGraphLaneLimit)];
    for (key, command) in normal_only_defaults {
        if insert_default_binding_if_available(normal_map, key, command) {
            changed = true;
        }
    }
    let action_map = maps.entry(InputMode::Action).or_default();
    let rename_branch_key = KeyBinding::new(Char('B'), KeyModifiers::SHIFT);
    if !action_map.values().any(|existing| existing == &Command::RenameBranch) && !action_map.contains_key(&rename_branch_key) {
        action_map.insert(rename_branch_key, Command::RenameBranch);
        changed = true;
    }
    let update_submodule_key = KeyBinding::new(Char('i'), KeyModifiers::NONE);
    if !action_map.values().any(|existing| existing == &Command::UpdateSubmodule) && !action_map.contains_key(&update_submodule_key) {
        action_map.insert(update_submodule_key, Command::UpdateSubmodule);
        changed = true;
    }
    let sync_submodule_key = KeyBinding::new(Char('I'), KeyModifiers::SHIFT);
    if !action_map.values().any(|existing| existing == &Command::SyncSubmodule) && !action_map.contains_key(&sync_submodule_key) {
        action_map.insert(sync_submodule_key, Command::SyncSubmodule);
        changed = true;
    }
    changed
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

#[derive(Facet)]
struct KeymapConfig {
    normal: Vec<KeyBindingEntry>,
    action: Vec<KeyBindingEntry>,
}

#[derive(Facet)]
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
        KeyCode::Backspace => keymap_text::BACKSPACE().into(),
        KeyCode::Enter => keymap_text::ENTER().into(),
        KeyCode::Left => keymap_text::LEFT().into(),
        KeyCode::Right => keymap_text::RIGHT().into(),
        KeyCode::Up => keymap_text::UP().into(),
        KeyCode::Down => keymap_text::DOWN().into(),
        KeyCode::Home => keymap_text::HOME().into(),
        KeyCode::End => keymap_text::END().into(),
        KeyCode::PageUp => keymap_text::PAGE_UP().into(),
        KeyCode::PageDown => keymap_text::PAGE_DOWN().into(),
        KeyCode::Tab => keymap_text::TAB().into(),
        KeyCode::BackTab => keymap_text::BACK_TAB().into(),
        KeyCode::Delete => keymap_text::DELETE().into(),
        KeyCode::Insert => keymap_text::INSERT().into(),
        KeyCode::F(n) => format!("F({})", n),
        KeyCode::Char(' ') => keymap_text::SPACE().into(),
        KeyCode::Char(c) => format!("{c}"),
        KeyCode::Null => keymap_text::NULL().into(),
        KeyCode::Esc => keymap_text::ESC().into(),
        KeyCode::CapsLock => keymap_text::CAPS_LOCK().into(),
        KeyCode::ScrollLock => keymap_text::SCROLL_LOCK().into(),
        KeyCode::NumLock => keymap_text::NUM_LOCK().into(),
        KeyCode::PrintScreen => keymap_text::PRINT_SCREEN().into(),
        KeyCode::Pause => keymap_text::PAUSE().into(),
        _ => keymap_text::UNSUPPORTED().into(),
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
            inner.parse::<u8>().map(KeyCode::F).map_err(|_| format!("Invalid F-key string: {}", s))
        },
        s if s.starts_with("Char(") && s.ends_with(")") => {
            let inner = &s[5..s.chars().count() - 1];
            let ch = inner.chars().next().ok_or_else(|| format!("Empty Char key: {}", s))?;

            Ok(KeyCode::Char(ch))
        },
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
    map.iter().map(|(kb, cmd)| KeyBindingEntry { key: keycode_to_string(kb.code), modifiers: modifiers_to_vec(kb.modifiers), command: cmd.clone() }).collect()
}

fn keymaps_to_config(maps: &Keymaps) -> KeymapConfig {
    let normal = maps.get(&InputMode::Normal).map(keymap_entries).unwrap_or_default();

    let action = maps.get(&InputMode::Action).map(keymap_entries).unwrap_or_default();

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

pub fn load_keymaps_from_path(path: &Path) -> Result<Keymaps, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let cfg: KeymapConfig = facet_json::from_str(&text)?;
    Ok(config_to_keymaps(cfg)?)
}

pub fn save_keymaps_to_path(path: &Path, maps: &Keymaps) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = keymaps_to_config(maps);
    let json = facet_json::to_string_pretty(&cfg)?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, json)?;
    Ok(())
}

pub fn keymap_config_path() -> PathBuf {
    let mut pathbuf = dirs::config_dir().unwrap();
    pathbuf.push("guitar");
    pathbuf.push("keymap.json");
    pathbuf
}

pub fn save_keymaps(maps: &Keymaps) -> Result<(), Box<dyn std::error::Error>> {
    let path = keymap_config_path();
    save_keymaps_to_path(path.as_path(), maps)
}

pub fn load_or_init_keymaps() -> Keymaps {
    let pathbuf = keymap_config_path();
    let path = pathbuf.as_path();

    match load_keymaps_from_path(path) {
        Ok(mut maps) => {
            if ensure_default_keymap_bindings(&mut maps) {
                let _ = save_keymaps_to_path(path, &maps);
            }
            maps
        },
        Err(_) => {
            let defaults = default_keymaps();
            let _ = save_keymaps_to_path(path, &defaults);
            defaults
        },
    }
}

#[cfg(test)]
#[path = "../tests/helpers/keymap.rs"]
mod tests;
