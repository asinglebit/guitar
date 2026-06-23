use crate::{
    app::input::TextInput,
    core::reflogs::HeadReflogs,
    core::stashes::Stashes,
    core::{
        submodules::{SubmoduleStackEntry, Submodules},
        worktrees::Worktrees,
    },
    git::{
        auth::{AuthChallenge, AuthSession, NetworkResult},
        os::path::try_into_git_repo_root,
        queries::{
            diffs::get_filenames_diff_at_oid,
            files::FileSearchResult,
            submodules::list_submodules,
            worktrees::{list_worktrees_metadata, list_worktrees_metadata_with_current_dirty},
        },
        repository::open,
    },
    helpers::{
        branch_visibility::{current_branch_names, load_branch_visibility, prune_hidden_branches, save_branch_visibility},
        heatmap::{DAYS, WEEKS, empty_heatmap},
        keymap::{Command, KeyBinding, KeymapEditError, KeymapSelection},
        layout::LayoutConfig,
        localisation::{Language, errors, load_language, load_language_from_path, modal, operations, save_language, save_language_to_path, set_active_language, settings},
        recent::{load_recent, save_recent, save_recent_to_path},
        symbols::{SymbolTheme, load_symbol_theme, load_symbol_theme_from_path, save_symbol_theme, save_symbol_theme_to_path},
    },
};
use crate::{
    app::state::{
        defaults::{SplitViewerRow, ViewerMode},
        layout::Layout,
    },
    core::{
        branches::Branches,
        graph_service::{
            Generation, GraphCommand, GraphEvent, GraphFileHistoryRow, GraphHistory, GraphIndexIdentity, GraphLookupKind, GraphLookupResult, GraphPane, GraphPaneRow, GraphRow, GraphServiceConfig,
            GraphVersion, RequestId, spawn_graph_service,
        },
        oids::Oids,
        tags::Tags,
    },
    git::{
        actions::network::NetworkRequest,
        queries::{
            commits::get_git_user_info,
            diffs::{get_filenames_diff_at_workdir, get_staged_filenames_diff},
            helpers::{FileChange, UncommittedChanges},
            remotes::list_remotes,
        },
    },
    helpers::{colors::ColorPicker, keymap::InputMode, palette::*, spinner::Spinner},
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{enable_raw_mode, supports_keyboard_enhancement},
};
use git2::{Oid, Repository};
use indexmap::IndexMap;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, ListItem},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    io,
    rc::Rc,
    sync::{Arc, atomic::AtomicBool, mpsc::channel},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use std::{env, io::stdout, path::PathBuf};

#[derive(PartialEq, Eq, Debug)]
pub enum Viewport {
    Graph,
    Viewer,
    Splash,
    Settings,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    Viewport,
    Inspector,
    StatusTop,
    StatusBottom,
    Search,
    Branches,
    Tags,
    Stashes,
    Reflogs,
    Worktrees,
    Submodules,
    ModalCheckout,
    ModalSolo,
    ModalCommit,
    ModalCherrypick,
    ModalRevert,
    ModalCreateBranch,
    ModalRenameBranch,
    ModalCreateWorktreeName,
    ModalCreateWorktreePath,
    ModalDeleteBranch,
    ModalWorktreeChooser,
    ModalRemoveWorktree,
    ModalLockWorktree,
    ModalRemoteAction,
    ModalRemoteDelete,
    ModalRemoteName,
    ModalRemoteUrl,
    ModalGraphLaneLimit,
    ModalGrep,
    ModalFileSearch,
    ModalTag,
    ModalDeleteTag,
    ModalKeyCapture,
    ModalAuth,
    ModalNetworkProgress,
    ModalOperationProgress,
    ModalOperationConflict,
    ModalOperationSuccess,
    ModalError,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OperationKind {
    Rebase,
    Cherrypick,
    Revert,
    Merge,
}

impl OperationKind {
    pub fn label(self) -> &'static str {
        match self {
            OperationKind::Rebase => operations::REBASE(),
            OperationKind::Cherrypick => operations::CHERRYPICK(),
            OperationKind::Revert => operations::REVERT(),
            OperationKind::Merge => operations::MERGE(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PendingOperationAction {
    Start { kind: OperationKind, oid: Oid },
    Continue,
    Abort,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ViewerLayoutSignature {
    pub graph_width: u16,
    pub split_left_width: u16,
    pub split_right_width: u16,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WorktreeModalAction {
    Open,
    Remove,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AuthInputField {
    Username,
    Secret,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BranchModalAction {
    Solo,
    Toggle,
    Rename,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RemoteInputAction {
    AddName,
    AddUrl,
    Rename,
    EditUrl,
    EditPushUrl,
}

#[derive(Default)]
pub struct GraphWindowCache {
    pub version: GraphVersion,
    pub start: usize,
    pub end: usize,
    pub head_alias: u32,
    pub rows: Vec<GraphRow>,
    pub history: GraphHistory,
}

#[derive(Default)]
pub struct PaneWindowCache {
    pub version: GraphVersion,
    pub start: usize,
    pub end: usize,
    pub total: usize,
    pub rows: Vec<GraphPaneRow>,
}

#[derive(Clone, Copy)]
pub enum PendingGraphLookup {
    SelectIndex,
    SelectPaneRow,
    CacheGraphRow,
    OpenInspector,
    RestoreSelection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphSelectionRestore {
    pub oid: Oid,
    pub selected_offset: usize,
}

#[derive(Default)]
pub struct GraphProjectionCache {
    pub key: Option<GraphProjectionKey>,
    pub message_lines: Vec<Line<'static>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphProjectionKey {
    pub version: GraphVersion,
    pub start: usize,
    pub end: usize,
    pub head_alias: u32,
    pub selected: usize,
    pub show_reflog_labels: bool,
    pub show_ref_labels: bool,
    pub render_uncommitted_row: bool,
    pub conflict_count: usize,
    pub modified_count: usize,
    pub added_count: usize,
    pub deleted_count: usize,
}

#[derive(Default)]
pub struct GraphClientCache {
    pub generation: Generation,
    pub version: GraphVersion,
    pub total: usize,
    pub is_complete: bool,
    pub next_request_id: RequestId,
    pub requested_graph: Option<(RequestId, usize, usize)>,
    pub pending_lookup: Option<(RequestId, PendingGraphLookup)>,
    pub pending_selection_restore: Option<GraphSelectionRestore>,
    pub index_rows: HashMap<usize, GraphRow>,
    pub graph_window: Option<GraphWindowCache>,
    pub graph_projection: GraphProjectionCache,
    pub branches_window: Option<PaneWindowCache>,
    pub tags_window: Option<PaneWindowCache>,
    pub stashes_window: Option<PaneWindowCache>,
    pub reflogs_window: Option<PaneWindowCache>,
}

impl GraphClientCache {
    pub fn next_request_id(&mut self) -> RequestId {
        self.next_request_id = self.next_request_id.saturating_add(1);
        self.next_request_id
    }

    pub fn row_at(&self, index: usize) -> Option<&GraphRow> {
        self.graph_window.as_ref().and_then(|window| window.rows.iter().find(|row| row.index == index)).or_else(|| self.index_rows.get(&index))
    }

    pub fn clear_graph_projection(&mut self) {
        self.graph_projection = GraphProjectionCache::default();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Display,
    Auth,
    Repo,
    Shortcuts,
}

impl SettingsTab {
    pub const ALL: [SettingsTab; 5] = [SettingsTab::General, SettingsTab::Display, SettingsTab::Auth, SettingsTab::Repo, SettingsTab::Shortcuts];

    pub fn label(self) -> &'static str {
        match self {
            SettingsTab::General => settings::GENERAL(),
            SettingsTab::Display => settings::DISPLAY(),
            SettingsTab::Auth => settings::AUTH(),
            SettingsTab::Repo => settings::REPO(),
            SettingsTab::Shortcuts => settings::SHORTCUTS(),
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|&tab| tab == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL.iter().position(|&tab| tab == self).unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsSelectionKind {
    Info,
    RecentRepository(usize),
    RemoteAdd,
    Remote(String),
    Language(usize),
    Theme(usize),
    SymbolTheme(usize),
    KeyBinding(KeymapSelection),
    LayoutCommand(Command),
    GraphLaneLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSelection {
    pub line: usize,
    pub kind: SettingsSelectionKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsTabHitbox {
    pub tab: SettingsTab,
    pub line: usize,
    pub start: u16,
    pub end: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextMenuAction {
    Divider,
    Spacer,
    Command(Command),
    GraphCommand(Command),
    OpenRecentRepository(usize),
    RemoteAction { name: String, index: usize },
    SwitchSettingsTab(SettingsTab),
    Settings,
    Splash,
    Exit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenuItem {
    pub label: String,
    pub action: ContextMenuAction,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenuState {
    pub column: u16,
    pub row: u16,
    pub selected: usize,
    pub items: Vec<ContextMenuItem>,
}

impl ContextMenuState {
    pub fn label_width(&self) -> usize {
        self.items.iter().map(|item| item.label.chars().count()).max().unwrap_or(0)
    }

    pub fn width(&self) -> u16 {
        let width = self.label_width().saturating_add(7);
        width.min(u16::MAX as usize) as u16
    }

    pub fn height(&self) -> u16 {
        let height = self.items.len().saturating_add(4);
        height.min(u16::MAX as usize) as u16
    }

    pub fn area(&self, bounds: Rect) -> Rect {
        if bounds.width == 0 || bounds.height == 0 {
            return Rect::new(bounds.x, bounds.y, 0, 0);
        }

        let width = self.width().min(bounds.width);
        let height = self.height().min(bounds.height);
        let max_x = bounds.x.saturating_add(bounds.width.saturating_sub(width));
        let max_y = bounds.y.saturating_add(bounds.height.saturating_sub(height));
        let x = self.column.clamp(bounds.x, max_x);
        let y = self.row.clamp(bounds.y, max_y);

        Rect::new(x, y, width, height)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDrag {
    LeftPane,
    RightPane,
    BranchesTags,
    BranchesStashes,
    BranchesWorktrees,
    BranchesSubmodules,
    BranchesReflogs,
    BranchesSearch,
    TagsStashes,
    TagsWorktrees,
    TagsSubmodules,
    StashesWorktrees,
    StashesSubmodules,
    TagsReflogs,
    StashesReflogs,
    ReflogsWorktrees,
    ReflogsSubmodules,
    WorktreesSubmodules,
    TagsSearch,
    StashesSearch,
    ReflogsSearch,
    WorktreesSearch,
    SubmodulesSearch,
    InspectorStatus,
    StatusFiles,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollbarTarget {
    Graph,
    Viewer,
    Settings,
    Branches,
    Tags,
    Stashes,
    Reflogs,
    Worktrees,
    Submodules,
    Search,
    Inspector,
    StatusTop,
    StatusBottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollbarDrag {
    pub target: ScrollbarTarget,
    pub grab_offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedMouseDrag {
    pub layout: LayoutDrag,
    pub scrollbar: ScrollbarDrag,
    pub start_column: u16,
    pub start_row: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseDrag {
    Layout(LayoutDrag),
    Scrollbar(ScrollbarDrag),
    Shared(SharedMouseDrag),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseSelectionTarget {
    Graph(usize),
    Viewer(usize),
    Branches(usize),
    Tags(usize),
    Stashes(usize),
    Reflogs(usize),
    Worktrees(usize),
    Submodules(usize),
    Inspector(usize),
    StatusTop(usize),
    StatusBottom(usize),
    Search(usize),
    Splash(usize),
    Settings(usize),
    SettingsTab(SettingsTab),
}

pub struct App {
    // Global application state and user-facing configuration.
    pub logo: Vec<Span<'static>>,
    pub path: Option<String>,
    pub recent: Vec<String>,
    pub repo: Option<Rc<Repository>>,
    pub spinner: Spinner,
    pub keymaps: IndexMap<InputMode, IndexMap<KeyBinding, Command>>,
    pub mode: InputMode,
    pub last_input_direction: Option<Direction>,
    pub theme: Theme,
    pub symbols: SymbolTheme,
    pub language: Language,
    pub heatmap: [[usize; WEEKS]; DAYS],
    pub remotes: Vec<crate::git::queries::remotes::RemoteEntry>,

    // Git identity used when creating commits.
    pub name: String,
    pub email: String,

    // Background history walker and graph rendering helpers.
    pub color: Rc<RefCell<ColorPicker>>,
    pub graph: GraphClientCache,
    pub graph_tx: Option<std::sync::mpsc::Sender<GraphCommand>>,
    pub graph_event_tx: Option<std::sync::mpsc::Sender<GraphEvent>>,
    pub graph_rx: Option<std::sync::mpsc::Receiver<GraphEvent>>,
    pub walker_cancel: Option<Arc<AtomicBool>>,
    pub walker_handle: Option<std::thread::JoinHandle<()>>,

    // Repository metadata consumed by graph, branch, tag, and stash panes.
    pub oids: Oids,
    pub branches: Branches,
    pub tags: Tags,
    pub stashes: Stashes,
    pub reflogs: HeadReflogs,
    pub worktrees: Worktrees,
    pub submodules: Submodules,
    pub submodule_stack: Vec<SubmoduleStackEntry>,
    pub uncommitted: UncommittedChanges,

    // Cached file and diff data for the currently selected graph or status row.
    pub current_diff: Vec<FileChange>,
    pub current_diff_identity: Option<GraphIndexIdentity>,
    pub is_uncommitted_loaded: bool,
    pub is_uncommitted_detail_loaded: bool,
    pub is_uncommitted_detail_loading: bool,
    pub file_name: Option<String>,
    pub viewer_lines: Vec<ListItem<'static>>,
    pub viewer_split_rows: Vec<SplitViewerRow>,
    pub viewer_edges: Vec<usize>,
    pub viewer_hunks: Vec<usize>,
    pub viewer_mode: ViewerMode,
    pub is_viewer_layout_dirty: bool,
    pub viewer_layout_signature: Option<ViewerLayoutSignature>,

    // Last computed terminal rectangles.
    pub layout: Layout,

    // Persistent layout switches and current interaction target.
    pub layout_config: LayoutConfig,
    pub mouse_drag: Option<MouseDrag>,
    pub last_mouse_click: Option<(MouseSelectionTarget, Instant)>,
    pub context_menu: Option<ContextMenuState>,
    pub modal_area: Option<Rect>,
    pub viewport: Viewport,
    pub focus: Focus,

    // Pane selections and scroll offsets.
    pub branches_selected: usize,
    pub branches_scroll: Cell<usize>,

    // Tags
    pub tags_selected: usize,
    pub tags_scroll: Cell<usize>,

    // Stashes
    pub stashes_selected: usize,
    pub stashes_scroll: Cell<usize>,

    // Reflogs
    pub reflogs_selected: usize,
    pub reflogs_scroll: Cell<usize>,

    // Worktrees
    pub worktrees_selected: usize,
    pub worktrees_scroll: Cell<usize>,

    // Submodules
    pub submodules_selected: usize,
    pub submodules_scroll: Cell<usize>,

    // Search
    pub search_path: Option<String>,
    pub search_rows: Vec<GraphFileHistoryRow>,
    pub search_is_loading: bool,
    pub search_error: Option<String>,
    pub search_request_id: Option<RequestId>,
    pub search_selected: usize,
    pub search_scroll: Cell<usize>,

    // Graph
    pub graph_selected: usize,
    pub graph_scroll: Cell<usize>,

    // Viewer
    pub viewer_selected: usize,
    pub viewer_scroll: Cell<usize>,

    // Splash
    pub splash_selected: usize,
    pub recent_save_path: Option<PathBuf>,

    // Settings
    pub settings_tab: SettingsTab,
    pub settings_selected: usize,
    pub settings_scroll: Cell<usize>,
    pub settings_selections: Vec<SettingsSelection>,
    pub settings_tab_hitboxes: Vec<SettingsTabHitbox>,
    pub modal_key_capture_selection: Option<KeymapSelection>,
    pub modal_key_capture_candidate: Option<KeyBinding>,
    pub modal_key_capture_error: Option<KeymapEditError>,
    pub keymap_save_path: Option<PathBuf>,
    pub symbol_theme_save_path: Option<PathBuf>,
    pub language_save_path: Option<PathBuf>,

    // Inspector
    pub inspector_selected: usize,
    pub inspector_scroll: Cell<usize>,

    // Status top
    pub status_top_selected: usize,
    pub status_top_scroll: Cell<usize>,

    // Status bottom
    pub status_bottom_selected: usize,
    pub status_bottom_scroll: Cell<usize>,

    // Modal selections and text input buffers.
    pub modal_checkout_selected: i32,

    // Modal solo
    pub modal_solo_selected: i32,
    pub modal_branch_action: BranchModalAction,

    // Modal editor
    pub modal_input: TextInput,
    pub pending_cherrypick_oid: Option<Oid>,
    pub pending_revert_oid: Option<Oid>,
    pub pending_branch_target_oid: Option<Oid>,
    pub modal_rename_branch_source: Option<String>,
    pub modal_worktree_name: String,
    pub modal_worktree_selected: i32,
    pub modal_worktree_candidates: Vec<usize>,
    pub modal_worktree_target: Option<usize>,
    pub modal_worktree_action: WorktreeModalAction,
    pub modal_worktree_return_focus: Focus,
    pub modal_remote_selected: i32,
    pub modal_remote_target: Option<String>,
    pub modal_remote_input_action: RemoteInputAction,
    pub modal_remote_name: String,
    pub modal_file_search_results: Vec<FileSearchResult>,
    pub modal_file_search_selected: i32,
    pub modal_file_search_scroll: Cell<usize>,
    pub modal_file_search_return_focus: Focus,

    // Modal delete a branch
    pub modal_delete_branch_selected: i32,

    // Modal delete a tag
    pub modal_delete_tag_selected: i32,

    // Modal error
    pub modal_error_message: String,
    pub modal_error_return_focus: Focus,

    // Modal git operation
    pub modal_operation_kind: OperationKind,
    pub modal_operation_message: String,
    pub pending_operation_action: Option<PendingOperationAction>,

    // Modal network operation and authentication prompts.
    pub auth_session: AuthSession,
    pub pending_network_request: Option<NetworkRequest>,
    pub network_handle: Option<JoinHandle<NetworkResult>>,
    pub network_auth_attempts: usize,
    pub pending_auth_prompt: Option<AuthChallenge>,
    pub auth_username_input: TextInput,
    pub auth_secret_input: TextInput,
    pub auth_input_field: AuthInputField,
    pub modal_network_title: String,
    pub modal_network_message: String,

    // Main loop shutdown flag.
    pub is_exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // Ask supported terminals to distinguish Esc from modified key sequences.
        enable_raw_mode()?;
        let has_keyboard_enhancement = matches!(supports_keyboard_enhancement(), Ok(true));

        if has_keyboard_enhancement {
            execute!(stdout(), PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES))?;
        }
        execute!(stdout(), EnableMouseCapture)?;

        let run_result = (|| {
            // Load persisted state before the first repository scan.
            self.load_language_config();
            self.load_recent();
            self.load_layout();
            self.load_theme_config();
            self.load_symbol_theme_config();
            self.load_keymap();
            self.reload(None);

            while !self.is_exit {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    self.handle_events()?;
                }

                // Pull at most one walker update per tick to keep redraws responsive.
                if let Some(repo) = &self.repo.clone() {
                    self.sync(repo);
                }
                self.poll_network_request();

                terminal.draw(|frame| self.draw(frame))?;
                self.run_pending_operation_action();
            }

            Ok(())
        })();

        let pop_result = if has_keyboard_enhancement { execute!(stdout(), PopKeyboardEnhancementFlags) } else { Ok(()) };
        let mouse_result = execute!(stdout(), DisableMouseCapture);
        if run_result.is_ok() {
            pop_result?;
            mouse_result?;
        }

        run_result
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        // Layout must be recomputed every frame because terminal size and focus can change.
        self.layout(frame);
        if self.viewport == Viewport::Viewer {
            let signature = self.current_viewer_layout_signature();
            if self.viewer_layout_signature != Some(signature) {
                self.is_viewer_layout_dirty = true;
            }
            self.viewer_layout_signature = Some(signature);
        }
        if self.is_viewer_layout_dirty {
            self.refresh_viewer_for_layout_change();
            self.is_viewer_layout_dirty = false;
        }

        frame.render_widget(Block::default().style(self.theme.background_style()), frame.area());

        let is_splash = self.viewport == Viewport::Splash;

        frame.render_widget(
            Block::default().borders(if is_splash { Borders::NONE } else { Borders::ALL }).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_set(self.symbols.border.block_set()),
            self.layout.app,
        );

        // Repo-dependent panes render only after a repository has opened successfully.
        if let Some(repo) = &self.repo.clone() {
            // The central viewport is mutually exclusive, while side panes can be toggled.
            match self.viewport {
                Viewport::Graph => {
                    self.draw_graph(frame, repo);
                },
                Viewport::Viewer => {
                    self.draw_viewer(frame);
                },
                Viewport::Splash => {
                    self.draw_splash(frame);
                },
                Viewport::Settings => {
                    self.draw_settings(frame, repo);
                },
            }

            if !is_splash {
                self.draw_title(frame);
            }

            // Side panes are hidden on splash/settings because those views own the frame.
            match self.viewport {
                Viewport::Splash => {},
                Viewport::Settings => {},
                _ => {
                    if self.layout_config.is_branches {
                        self.draw_branches(frame);
                    }
                    if self.layout_config.is_tags {
                        self.draw_tags(frame);
                    }
                    if self.layout_config.is_stashes {
                        self.draw_stashes(frame, repo);
                    }
                    if self.layout_config.is_reflogs {
                        self.draw_reflogs(frame);
                    }
                    if self.layout_config.is_worktrees {
                        self.draw_worktrees(frame);
                    }
                    if self.layout_config.is_submodules {
                        self.draw_submodules(frame);
                    }
                    if self.layout_config.is_search {
                        self.draw_search(frame);
                    }
                    if self.layout_config.is_status {
                        self.draw_status(frame);
                    }
                    if self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) {
                        self.draw_inspector(frame, repo);
                    }
                },
            }

            if !is_splash {
                self.draw_statusbar(frame, repo);
            }

            // Modals render last so they overlay panes without changing pane layout.
            self.modal_area = None;
            match self.focus {
                Focus::ModalCheckout => {
                    self.draw_modal_checkout(frame);
                },
                Focus::ModalSolo => {
                    self.draw_modal_solo(frame);
                },
                Focus::ModalDeleteBranch => {
                    self.draw_modal_delete_branch(frame, repo);
                },
                Focus::ModalWorktreeChooser => {
                    self.draw_modal_worktree_chooser(frame);
                },
                Focus::ModalRemoveWorktree => {
                    self.draw_modal_remove_worktree(frame);
                },
                Focus::ModalRemoteAction => {
                    self.draw_modal_remote_action(frame);
                },
                Focus::ModalRemoteDelete => {
                    self.draw_modal_delete_remote(frame);
                },
                Focus::ModalDeleteTag => {
                    self.draw_modal_delete_tag(frame);
                },
                Focus::ModalOperationProgress | Focus::ModalOperationConflict | Focus::ModalOperationSuccess => {
                    self.draw_modal_rebase(frame);
                },
                Focus::ModalError => {
                    self.draw_modal_error(frame);
                },
                Focus::ModalCommit => {
                    self.draw_modal_input(frame, modal::PROMPT_CREATE_COMMIT());
                },
                Focus::ModalCherrypick => {
                    self.draw_modal_input(frame, modal::PROMPT_CHERRYPICK_COMMIT());
                },
                Focus::ModalRevert => {
                    self.draw_modal_input(frame, modal::PROMPT_REVERT_COMMIT());
                },
                Focus::ModalCreateBranch => {
                    self.draw_modal_input(frame, modal::PROMPT_CREATE_BRANCH());
                },
                Focus::ModalRenameBranch => {
                    self.draw_modal_input(frame, modal::PROMPT_RENAME_BRANCH());
                },
                Focus::ModalCreateWorktreeName => {
                    self.draw_modal_input(frame, modal::PROMPT_CREATE_WORKTREE_NAME());
                },
                Focus::ModalCreateWorktreePath => {
                    self.draw_modal_input(frame, modal::PROMPT_CREATE_WORKTREE_PATH());
                },
                Focus::ModalLockWorktree => {
                    self.draw_modal_input(frame, modal::PROMPT_LOCK_WORKTREE());
                },
                Focus::ModalRemoteName | Focus::ModalRemoteUrl => {
                    self.draw_modal_input(frame, self.remote_input_title());
                },
                Focus::ModalGraphLaneLimit => {
                    self.draw_modal_input(frame, modal::PROMPT_GRAPH_LANE_LIMIT());
                },
                Focus::ModalGrep => {
                    self.draw_modal_input(frame, modal::PROMPT_FIND_SHA());
                },
                Focus::ModalFileSearch => {
                    self.draw_modal_file_search(frame, modal::PROMPT_FIND_FILE());
                },
                Focus::ModalTag => {
                    self.draw_modal_input(frame, modal::PROMPT_CREATE_TAG());
                },
                Focus::ModalKeyCapture => {
                    self.draw_modal_key_capture(frame);
                },
                Focus::ModalAuth => {
                    self.draw_modal_auth(frame);
                },
                Focus::ModalNetworkProgress => {
                    self.draw_modal_network_progress(frame);
                },
                _ => {},
            }
        } else {
            self.draw_splash(frame);
        }

        if self.context_menu.is_some() && !self.is_modal_focus() {
            self.draw_context_menu(frame);
        }
    }

    pub fn reload(&mut self, override_path: Option<String>) {
        let existing_hidden_branch_names = self.branches.hidden_branch_names.clone();
        let previous_path = self.path.clone();
        let has_override_path = override_path.is_some();
        let pending_selection_restore = if override_path.is_none() && self.graph_selected != 0 {
            self.graph_identity_at(self.graph_selected)
                .map(|identity| GraphSelectionRestore { oid: identity.oid, selected_offset: self.graph_selected.saturating_sub(self.graph_scroll.get()) })
                .filter(|restore| restore.oid != Oid::zero())
        } else {
            None
        };

        // Clear derived data; the walker will repopulate it asynchronously.
        self.heatmap = empty_heatmap();
        self.current_diff = Vec::new();
        self.current_diff_identity = None;
        self.is_uncommitted_loaded = false;
        self.is_uncommitted_detail_loaded = false;
        self.is_uncommitted_detail_loading = false;
        self.uncommitted = UncommittedChanges::default();
        self.viewer_lines = Vec::new();
        self.viewer_split_rows = Vec::new();
        self.viewer_edges = Vec::new();
        self.viewer_hunks = Vec::new();
        self.branches = Branches::default();
        self.tags = Tags::default();
        self.stashes = Stashes::default();
        self.reflogs = HeadReflogs::default();
        self.worktrees = Worktrees::default();
        self.submodules = Submodules::default();
        self.remotes = Vec::new();
        self.clear_file_history_search();
        self.branches.hidden_branch_names = existing_hidden_branch_names.clone();

        // Prefer an explicit path, then the current path, then the first non-flag CLI arg.
        let path = if let Some(path) = override_path {
            path
        } else if let Some(path) = self.path.clone() {
            path
        } else {
            env::args().skip(1).find(|arg| !arg.starts_with('-')).unwrap_or_else(|| ".".to_string())
        };
        let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from("."));
        let absolute_path: PathBuf = try_into_git_repo_root(&canonical_path).unwrap_or(canonical_path);

        // Failure keeps the app usable by falling back to the splash screen.
        let repo = match open(&absolute_path) {
            Ok(r) => Some(Rc::new(r)),
            Err(_) => None,
        };

        let absolute_path = absolute_path.display().to_string();
        self.path = Some(absolute_path.clone());
        self.repo = repo;
        self.refresh_theme_assets();

        // Repository-specific state starts only after Repository::open succeeds.
        if let Some(repo) = &self.repo {
            let current_path = PathBuf::from(&absolute_path);
            self.remotes = list_remotes(current_path.as_path()).unwrap_or_default();
            self.worktrees = Worktrees::from_entries(list_worktrees_metadata(repo, Some(current_path.as_path())).unwrap_or_default());
            self.submodules = Submodules::from_entries(list_submodules(repo).unwrap_or_default());

            let same_repo_reload = !has_override_path && previous_path.as_deref() == Some(absolute_path.as_str());
            let mut hidden_branch_names = if same_repo_reload { existing_hidden_branch_names } else { load_branch_visibility(&absolute_path) };
            if !hidden_branch_names.is_empty() && prune_hidden_branches(&mut hidden_branch_names, &current_branch_names(repo)) {
                save_branch_visibility(&absolute_path, &hidden_branch_names);
            }
            self.branches.hidden_branch_names = hidden_branch_names;

            // Recent paths are append-only here; the splash screen controls selection.
            if !self.recent.iter().any(|v| v == &absolute_path) {
                self.recent.push(absolute_path.clone());
                self.save_recent();
            }

            // Cancel the previous walker before spawning a new one for this repository state.
            if let Some(tx) = self.graph_tx.take() {
                let _ = tx.send(GraphCommand::Shutdown);
            }
            self.graph_event_tx = None;
            self.graph_rx = None;

            if let Some(cancel_flag) = &self.walker_cancel {
                cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }

            // Join the old worker off-thread so reload never stalls the UI loop.
            if let Some(handle) = self.walker_handle.take() {
                std::thread::spawn(move || {
                    let _ = handle.join();
                });
            }

            // Commit actions require a concrete identity, so missing config is treated as fatal.
            let (name, email) = get_git_user_info(repo).expect("Couldn't get user credentials");
            self.name = name.unwrap();
            self.email = email.unwrap();

            // The spinner reflects walker activity, not individual git network commands.
            self.spinner.start();

            // Each reload gets a fresh channel so stale walker results cannot be received.
            let cancel = Arc::new(AtomicBool::new(false));
            let cancel_clone = cancel.clone();
            self.walker_cancel = Some(cancel);

            let generation = self.graph.generation.saturating_add(1);
            self.graph = GraphClientCache { generation, pending_selection_restore, ..Default::default() };

            let (command_tx, command_rx) = channel();
            let (event_tx, event_rx) = channel();
            self.graph_tx = Some(command_tx);
            self.graph_event_tx = Some(event_tx.clone());
            self.graph_rx = Some(event_rx);

            // Move only serializable state into the worker thread.
            let hidden_branch_names = self.branches.hidden_branch_names.clone();
            let include_head_reflog_roots = self.layout_config.is_graph_reflogs;
            let graph_lane_limit = self.layout_config.graph_lane_limit;
            let worktrees = self.worktrees.entries.clone();

            // The worker streams partial graph state so large repositories become usable quickly.
            let handle = spawn_graph_service(
                GraphServiceConfig { generation, path: absolute_path, amount: 10000, hidden_branch_names, include_head_reflog_roots, graph_lane_limit, worktrees, symbols: self.symbols.clone() },
                command_rx,
                event_tx,
                cancel_clone,
            );

            self.walker_handle = Some(handle);
        }
    }

    fn spawn_reload_metadata(&self, generation: Generation) {
        let Some(path) = self.path.as_ref().map(PathBuf::from) else {
            return;
        };
        let Some(reload_metadata_tx) = self.graph_tx.clone() else {
            return;
        };
        let Some(reload_uncommitted_tx) = self.graph_event_tx.clone() else {
            return;
        };

        std::thread::spawn(move || {
            let result = open(&path).map_err(|error| error.to_string()).and_then(|repo| {
                let staged = get_staged_filenames_diff(&repo).map_err(|error| error.to_string())?;
                if let Ok(worktrees) = list_worktrees_metadata_with_current_dirty(&repo, Some(path.as_path()), &staged) {
                    let _ = reload_metadata_tx.send(GraphCommand::UpdateWorktrees { generation, worktrees });
                }
                Ok(staged)
            });
            let _ = reload_uncommitted_tx.send(GraphEvent::Uncommitted { generation, result });
        });
    }

    pub(crate) fn refresh_current_diff_for_identity(&mut self, repo: &git2::Repository, identity: GraphIndexIdentity) {
        if self.current_diff_identity == Some(identity) {
            return;
        }
        self.current_diff = get_filenames_diff_at_oid(repo, identity.oid);
        self.current_diff_identity = Some(identity);
    }

    pub fn ensure_uncommitted_details_loaded(&mut self) {
        if self.is_uncommitted_detail_loaded || self.is_uncommitted_detail_loading {
            return;
        }
        let generation = self.graph.generation;
        let Some(path) = self.path.as_ref().map(PathBuf::from) else {
            return;
        };
        let Some(reload_metadata_tx) = self.graph_tx.clone() else {
            return;
        };
        let Some(reload_uncommitted_tx) = self.graph_event_tx.clone() else {
            return;
        };
        self.is_uncommitted_detail_loading = true;

        std::thread::spawn(move || {
            let result = open(&path).map_err(|error| error.to_string()).and_then(|repo| {
                let uncommitted = get_filenames_diff_at_workdir(&repo).map_err(|error| error.to_string())?;
                if let Ok(worktrees) = list_worktrees_metadata_with_current_dirty(&repo, Some(path.as_path()), &uncommitted) {
                    let _ = reload_metadata_tx.send(GraphCommand::UpdateWorktrees { generation, worktrees });
                }
                Ok(uncommitted)
            });
            let _ = reload_uncommitted_tx.send(GraphEvent::UncommittedDetails { generation, result });
        });
    }

    pub fn sync(&mut self, repo: &git2::Repository) {
        let mut events = Vec::new();
        if let Some(rx) = &self.graph_rx {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }

        for event in events {
            self.handle_graph_event(repo, event);
        }
    }

    fn handle_graph_event(&mut self, repo: &git2::Repository, event: GraphEvent) {
        match event {
            GraphEvent::Progress { generation, version, total, is_first, is_complete } => {
                if generation != self.graph.generation {
                    return;
                }
                self.graph.version = version;
                self.graph.total = total;
                self.graph.is_complete = is_complete;

                if is_complete && !self.is_uncommitted_loaded {
                    self.spawn_reload_metadata(generation);
                }

                if is_first && self.viewport == Viewport::Splash {
                    self.viewport = Viewport::Graph;
                }

                if is_complete {
                    self.spinner.stop();
                }
                self.request_pending_graph_selection_restore_lookup();
            },
            GraphEvent::GraphWindow { generation, request_id, version, start, end, total, head_alias, rows, history } => {
                if generation != self.graph.generation {
                    return;
                }
                let Some((pending_id, pending_start, pending_end)) = self.graph.requested_graph else {
                    return;
                };
                if request_id < pending_id || start != pending_start || end != pending_end {
                    return;
                }
                self.graph.version = self.graph.version.max(version);
                self.graph.total = total;
                self.graph.graph_window = Some(GraphWindowCache { version, start, end, head_alias, rows, history });
                self.graph.clear_graph_projection();
                self.graph.requested_graph = None;

                if self.graph_selected != 0
                    && let Some(identity) = self.graph_identity_at(self.graph_selected)
                {
                    self.refresh_current_diff_for_identity(repo, identity);
                }
            },
            GraphEvent::PaneWindow { generation, version, pane, start, end, total, rows } => {
                if generation != self.graph.generation {
                    return;
                }
                let cache = PaneWindowCache { version, start, end, total, rows };
                match pane {
                    GraphPane::Branches => self.graph.branches_window = Some(cache),
                    GraphPane::Tags => self.graph.tags_window = Some(cache),
                    GraphPane::Stashes => self.graph.stashes_window = Some(cache),
                    GraphPane::Reflogs => self.graph.reflogs_window = Some(cache),
                }
            },
            GraphEvent::FileHistory { generation, request_id, path, rows, error } => {
                if generation != self.graph.generation || self.search_request_id != Some(request_id) || self.search_path.as_deref() != Some(path.as_str()) {
                    return;
                }

                self.search_is_loading = false;
                self.search_request_id = None;
                self.search_error = error;
                self.search_rows = rows;
                self.search_selected = self.search_selected.min(self.search_rows.len().saturating_sub(1));
                self.search_scroll.set(0);
            },
            GraphEvent::LookupResult { generation, request_id, result, .. } => {
                if generation != self.graph.generation {
                    return;
                }
                let Some((pending_id, action)) = self.graph.pending_lookup.take() else {
                    return;
                };
                if request_id != pending_id {
                    self.graph.pending_lookup = Some((pending_id, action));
                    return;
                }
                let was_restore_lookup = matches!(action, PendingGraphLookup::RestoreSelection);
                match (action, result) {
                    (PendingGraphLookup::SelectIndex, GraphLookupResult::Index(Some(index))) => {
                        self.select_graph_index_from_lookup(repo, index);
                        self.modal_input.clear();
                        self.focus = Focus::Viewport;
                    },
                    (PendingGraphLookup::RestoreSelection, GraphLookupResult::Index(Some(index))) => {
                        let selected_offset = self.graph.pending_selection_restore.map(|restore| restore.selected_offset).unwrap_or_default();
                        self.graph.pending_selection_restore = None;
                        self.restore_graph_index_from_lookup(repo, index, selected_offset);
                    },
                    (PendingGraphLookup::RestoreSelection, GraphLookupResult::Index(None)) => {
                        if self.graph.is_complete {
                            self.graph.pending_selection_restore = None;
                        }
                    },
                    (PendingGraphLookup::SelectPaneRow, GraphLookupResult::PaneRow(Some(row))) => {
                        self.open_graph_pane_row(row);
                        self.modal_input.clear();
                    },
                    (PendingGraphLookup::CacheGraphRow, GraphLookupResult::GraphRow(Some(row))) => {
                        let index = row.index;
                        self.cache_graph_row(row);
                        if index == self.graph_selected
                            && index != 0
                            && let Some(identity) = self.graph_identity_at(index)
                        {
                            self.refresh_current_diff_for_identity(repo, identity);
                        }
                    },
                    (PendingGraphLookup::OpenInspector, GraphLookupResult::GraphRow(Some(row))) => {
                        let index = row.index;
                        self.cache_graph_row(row);
                        if index == self.graph_selected {
                            if let Some(identity) = self.graph_identity_at(index) {
                                self.refresh_current_diff_for_identity(repo, identity);
                            }
                            self.layout_config.is_inspector = true;
                            self.focus = Focus::Inspector;
                        }
                    },
                    _ => {},
                }
                if !was_restore_lookup {
                    self.request_pending_graph_selection_restore_lookup();
                }
            },
            GraphEvent::Heatmap { generation, heatmap } => {
                if generation == self.graph.generation {
                    self.heatmap = *heatmap;
                }
            },
            GraphEvent::Worktrees { generation, version, worktrees } => {
                if generation == self.graph.generation {
                    self.graph.version = self.graph.version.max(version);
                    self.worktrees = Worktrees::from_entries(worktrees);
                }
            },
            GraphEvent::Uncommitted { generation, result } => {
                if generation == self.graph.generation {
                    match result {
                        Ok(uncommitted) => {
                            self.uncommitted = uncommitted;
                        },
                        Err(error) => {
                            self.uncommitted = UncommittedChanges::default();
                            self.show_error(errors::with_error(errors::FILE_DIFF(), error));
                        },
                    }
                    self.is_uncommitted_loaded = true;
                }
            },
            GraphEvent::UncommittedDetails { generation, result } => {
                if generation == self.graph.generation {
                    match result {
                        Ok(uncommitted) => {
                            self.uncommitted = uncommitted;
                            self.is_uncommitted_detail_loaded = true;
                            self.is_uncommitted_detail_loading = false;
                        },
                        Err(error) => {
                            self.uncommitted = UncommittedChanges::default();
                            self.is_uncommitted_detail_loaded = true;
                            self.is_uncommitted_detail_loading = false;
                            self.show_error(errors::with_error(errors::FILE_DIFF(), error));
                        },
                    }
                    self.is_uncommitted_loaded = true;
                }
            },
            GraphEvent::Error { generation, message } => {
                if generation == self.graph.generation {
                    self.show_error(message);
                    self.spinner.stop();
                }
            },
        }
    }

    pub fn load_recent(&mut self) {
        self.recent = load_recent();
    }

    pub fn save_recent(&self) {
        if let Some(path) = &self.recent_save_path {
            save_recent_to_path(path.as_path(), &self.recent);
        } else {
            save_recent(&self.recent);
        }
    }

    pub(crate) fn graph_commit_count(&self) -> usize {
        self.graph.total.max(self.oids.get_commit_count())
    }

    pub(crate) fn graph_row_at(&self, index: usize) -> Option<&GraphRow> {
        self.graph.row_at(index)
    }

    pub(crate) fn graph_identity_at(&self, index: usize) -> Option<GraphIndexIdentity> {
        if let Some(row) = self.graph_row_at(index) {
            return Some(GraphIndexIdentity { index: row.index, alias: row.alias, oid: row.oid });
        }

        if self.graph_tx.is_some() {
            return None;
        }

        self.oids.get_sorted_aliases().get(index).map(|&alias| GraphIndexIdentity { index, alias, oid: *self.oids.get_oid_by_alias(alias) })
    }

    pub(crate) fn graph_alias_at(&self, index: usize) -> Option<u32> {
        self.graph_identity_at(index).map(|identity| identity.alias)
    }

    pub(crate) fn graph_oid_at(&self, index: usize) -> Option<Oid> {
        self.graph_identity_at(index).map(|identity| identity.oid)
    }

    pub(crate) fn selected_commit_diff_is_loaded(&self) -> bool {
        self.graph_selected != 0 && self.graph_identity_at(self.graph_selected).is_some_and(|identity| self.current_diff_identity == Some(identity))
    }

    pub(crate) fn request_graph_window(&mut self, start: usize, end: usize) {
        let Some(tx) = self.graph_tx.clone() else {
            return;
        };

        if self.graph.graph_window.as_ref().is_some_and(|window| window.start <= start && end <= window.end && window.version >= self.graph.version) {
            return;
        }

        if self.graph.requested_graph.is_some_and(|(_, requested_start, requested_end)| requested_start <= start && end <= requested_end) {
            return;
        }

        let request_id = self.graph.next_request_id();
        self.graph.requested_graph = Some((request_id, start, end));
        let _ = tx.send(GraphCommand::QueryGraphWindow { generation: self.graph.generation, request_id, start, end });
    }

    pub(crate) fn request_pane_window(&mut self, pane: GraphPane, start: usize, end: usize) {
        let Some(tx) = self.graph_tx.clone() else {
            return;
        };

        let cache = match pane {
            GraphPane::Branches => self.graph.branches_window.as_ref(),
            GraphPane::Tags => self.graph.tags_window.as_ref(),
            GraphPane::Stashes => self.graph.stashes_window.as_ref(),
            GraphPane::Reflogs => self.graph.reflogs_window.as_ref(),
        };

        if cache.is_some_and(|window| window.start <= start && end <= window.end && window.version >= self.graph.version) {
            return;
        }

        let _ = tx.send(GraphCommand::QueryPaneWindow { generation: self.graph.generation, pane, start, end });
    }

    pub(crate) fn request_graph_lookup(&mut self, kind: GraphLookupKind, action: PendingGraphLookup) {
        let Some(tx) = self.graph_tx.clone() else {
            return;
        };

        if matches!(action, PendingGraphLookup::SelectIndex | PendingGraphLookup::SelectPaneRow) {
            self.graph.pending_selection_restore = None;
        }

        let request_id = self.graph.next_request_id();
        self.graph.pending_lookup = Some((request_id, action));
        let _ = tx.send(GraphCommand::Lookup { generation: self.graph.generation, request_id, kind });
    }

    fn request_pending_graph_selection_restore_lookup(&mut self) {
        let Some(restore) = self.graph.pending_selection_restore else {
            return;
        };
        if self.graph.pending_lookup.is_some() {
            return;
        }
        self.request_graph_lookup(GraphLookupKind::Oid { oid: restore.oid }, PendingGraphLookup::RestoreSelection);
    }

    pub(crate) fn request_graph_row_lookup(&mut self, index: usize, action: PendingGraphLookup) {
        if self.graph_row_at(index).is_some() {
            return;
        }
        self.request_graph_lookup(GraphLookupKind::GraphRowAt { index }, action);
    }

    pub(crate) fn clear_file_history_search(&mut self) {
        self.search_path = None;
        self.search_rows.clear();
        self.search_is_loading = false;
        self.search_error = None;
        self.search_request_id = None;
        self.search_selected = 0;
        self.search_scroll.set(0);
    }

    pub(crate) fn request_file_history_search(&mut self, path: String) {
        self.search_path = Some(path.clone());
        self.search_rows.clear();
        self.search_is_loading = true;
        self.search_error = None;
        self.search_selected = 0;
        self.search_scroll.set(0);

        let Some(tx) = self.graph_tx.clone() else {
            self.search_is_loading = false;
            self.search_error = Some(errors::FILE_HISTORY_WORKER_UNAVAILABLE().to_string());
            self.search_request_id = None;
            return;
        };

        let request_id = self.graph.next_request_id();
        self.search_request_id = Some(request_id);
        if tx.send(GraphCommand::QueryFileHistory { generation: self.graph.generation, request_id, path }).is_err() {
            self.search_is_loading = false;
            self.search_error = Some(errors::FILE_HISTORY_WORKER_UNAVAILABLE().to_string());
            self.search_request_id = None;
        }
    }

    pub(crate) fn cache_graph_row(&mut self, row: GraphRow) {
        self.graph.index_rows.insert(row.index, row);
    }

    fn select_graph_index_from_lookup(&mut self, repo: &git2::Repository, index: usize) {
        self.graph.pending_selection_restore = None;
        self.set_graph_index_from_lookup(repo, index);
    }

    fn restore_graph_index_from_lookup(&mut self, repo: &git2::Repository, index: usize, selected_offset: usize) {
        self.set_graph_index_from_lookup(repo, index);
        self.graph_scroll.set(self.graph_selected.saturating_sub(selected_offset));
    }

    fn set_graph_index_from_lookup(&mut self, repo: &git2::Repository, index: usize) {
        self.graph_selected = index.min(self.graph_commit_count().saturating_sub(1));
        self.graph_scroll.set(self.graph_selected);
        self.current_diff.clear();
        self.current_diff_identity = None;

        if self.graph_selected != 0
            && let Some(identity) = self.graph_identity_at(self.graph_selected)
        {
            self.refresh_current_diff_for_identity(repo, identity);
        }
    }

    fn refresh_theme_assets(&mut self) {
        self.color = Rc::new(RefCell::new(ColorPicker::from_theme(&self.theme)));
        self.logo = vec![
            Span::styled(self.symbols.splash.logo_word_prefix.clone(), Style::default().fg(self.theme.COLOR_GRASS)),
            Span::styled(self.symbols.splash.logo_corner.clone(), Style::default().fg(self.theme.COLOR_GREEN)),
        ];
        self.graph.clear_graph_projection();
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.refresh_theme_assets();
    }

    pub fn load_theme_config(&mut self) {
        self.set_theme(load_theme());
    }

    pub fn save_theme_config(&self) {
        save_theme(&self.theme);
    }

    pub fn set_language(&mut self, language: Language) {
        self.language = language;
        set_active_language(language);
        self.mark_viewer_layout_dirty();
    }

    pub fn load_language_config(&mut self) {
        let language = if let Some(path) = &self.language_save_path { load_language_from_path(path.as_path()) } else { load_language() };
        self.set_language(language);
    }

    pub fn save_language_config(&self) {
        let result = if let Some(path) = &self.language_save_path { save_language_to_path(path.as_path(), self.language) } else { save_language(self.language) };
        let _ = result;
    }

    pub fn set_symbol_theme(&mut self, symbols: SymbolTheme) {
        self.symbols = symbols;
        self.refresh_theme_assets();
    }

    pub fn load_symbol_theme_config(&mut self) {
        let symbols = if let Some(path) = &self.symbol_theme_save_path { load_symbol_theme_from_path(path.as_path()) } else { load_symbol_theme() };
        self.set_symbol_theme(symbols);
    }

    pub fn save_symbol_theme_config(&self) {
        if let Some(path) = &self.symbol_theme_save_path {
            save_symbol_theme_to_path(path.as_path(), &self.symbols);
        } else {
            save_symbol_theme(&self.symbols);
        }
    }

    pub fn exit(&mut self) {
        self.is_exit = true;
    }
}

#[cfg(test)]
#[path = "../tests/app/state/app.rs"]
mod tests;
