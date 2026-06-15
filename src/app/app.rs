use crate::{
    app::draw::buffered::{DrawSurface, SurfaceBuffers},
    app::input::TextInput,
    core::reflogs::HeadReflogs,
    core::stashes::Stashes,
    core::worktrees::Worktrees,
    git::{
        auth::{AuthChallenge, AuthSession, NetworkResult},
        os::path::try_into_git_repo_root,
        queries::{diffs::get_filenames_diff_at_oid, worktrees::list_worktrees},
    },
    helpers::{
        copy::{STR_CHERRYPICK_COMMIT, STR_CREATE_BRANCH, STR_CREATE_COMMIT, STR_CREATE_TAG, STR_CREATE_WORKTREE_NAME, STR_CREATE_WORKTREE_PATH, STR_FIND_SHA, STR_LOCK_WORKTREE},
        heatmap::{DAYS, WEEKS, empty_heatmap},
        keymap::{Command, KeyBinding, KeymapEditError, KeymapSelection},
        layout::LayoutConfig,
        recent::{load_recent, save_recent, save_recent_to_path},
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
            Generation, GraphCommand, GraphEvent, GraphHistory, GraphIndexIdentity, GraphLookupKind, GraphLookupResult, GraphPane, GraphPaneRow, GraphRow, GraphServiceConfig, GraphVersion, RequestId,
            spawn_graph_service,
        },
        oids::Oids,
        tags::Tags,
    },
    git::{
        actions::network::NetworkRequest,
        queries::{
            commits::get_git_user_info,
            diffs::get_filenames_diff_at_workdir,
            helpers::{FileChange, UncommittedChanges},
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
    style::Style,
    text::Span,
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
    Branches,
    Tags,
    Stashes,
    Reflogs,
    Worktrees,
    ModalCheckout,
    ModalSolo,
    ModalCommit,
    ModalCherrypick,
    ModalCreateBranch,
    ModalCreateWorktreeName,
    ModalCreateWorktreePath,
    ModalDeleteBranch,
    ModalWorktreeChooser,
    ModalRemoveWorktree,
    ModalLockWorktree,
    ModalGrep,
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
    Merge,
}

impl OperationKind {
    pub fn label(self) -> &'static str {
        match self {
            OperationKind::Rebase => "rebase",
            OperationKind::Cherrypick => "cherrypick",
            OperationKind::Merge => "merge",
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
}

#[derive(PartialEq, Eq)]
pub enum Direction {
    Down,
    Up,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsSelectionKind {
    Info,
    Theme(usize),
    KeyBinding(KeymapSelection),
    LayoutCommand(Command),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSelection {
    pub line: usize,
    pub kind: SettingsSelectionKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDrag {
    LeftPane,
    RightPane,
    BranchesTags,
    BranchesStashes,
    BranchesWorktrees,
    BranchesReflogs,
    TagsStashes,
    TagsWorktrees,
    StashesWorktrees,
    TagsReflogs,
    StashesReflogs,
    ReflogsWorktrees,
    InspectorStatus,
    StatusFiles,
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
    Inspector(usize),
    StatusTop(usize),
    StatusBottom(usize),
    Splash(usize),
    Settings(usize),
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
    pub heatmap: [[usize; WEEKS]; DAYS],

    // Git identity used when creating commits.
    pub name: String,
    pub email: String,

    // Background history walker and graph rendering helpers.
    pub color: Rc<RefCell<ColorPicker>>,
    pub graph: GraphClientCache,
    pub graph_tx: Option<std::sync::mpsc::Sender<GraphCommand>>,
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
    pub uncommitted: UncommittedChanges,

    // Cached file and diff data for the currently selected graph or status row.
    pub current_diff: Vec<FileChange>,
    pub current_diff_identity: Option<GraphIndexIdentity>,
    pub is_uncommitted_loaded: bool,
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
    pub surface_buffers: SurfaceBuffers,

    // Persistent layout switches and current interaction target.
    pub layout_config: LayoutConfig,
    pub layout_drag: Option<LayoutDrag>,
    pub last_mouse_click: Option<(MouseSelectionTarget, Instant)>,
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
    pub settings_selected: usize,
    pub settings_scroll: Cell<usize>,
    pub settings_selections: Vec<SettingsSelection>,
    pub modal_key_capture_selection: Option<KeymapSelection>,
    pub modal_key_capture_candidate: Option<KeyBinding>,
    pub modal_key_capture_error: Option<KeymapEditError>,
    pub keymap_save_path: Option<PathBuf>,

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
    pub pending_branch_target_oid: Option<Oid>,
    pub modal_worktree_name: String,
    pub modal_worktree_selected: i32,
    pub modal_worktree_candidates: Vec<usize>,
    pub modal_worktree_target: Option<usize>,
    pub modal_worktree_action: WorktreeModalAction,
    pub modal_worktree_return_focus: Focus,

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
            execute!(stdout(), PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES))?;
        }
        execute!(stdout(), EnableMouseCapture)?;

        let run_result = (|| {
            // Load persisted state before the first repository scan.
            self.load_recent();
            self.load_layout();
            self.load_theme_config();
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
        let previous_layout = self.layout;
        self.layout(frame);
        if self.layout != previous_layout {
            self.surface_buffers.clear();
        }
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
            Block::default()
                .borders(if is_splash { Borders::NONE } else { Borders::ALL })
                .border_style(Style::default().fg(self.theme.COLOR_BORDER))
                .border_type(ratatui::widgets::BorderType::Rounded),
            self.layout.app,
        );

        // Repo-dependent panes render only after a repository has opened successfully.
        if let Some(repo) = &self.repo.clone() {
            // The central viewport is mutually exclusive, while side panes can be toggled.
            match self.viewport {
                Viewport::Graph => {
                    self.draw_surface(frame, DrawSurface::Graph, |app, surface| app.draw_graph(surface, repo));
                },
                Viewport::Viewer => {
                    self.draw_surface(frame, DrawSurface::Viewer, |app, surface| app.draw_viewer(surface));
                },
                Viewport::Splash => {
                    self.draw_surface(frame, DrawSurface::Splash, |app, surface| app.draw_splash(surface));
                },
                Viewport::Settings => {
                    self.draw_surface(frame, DrawSurface::Settings, |app, surface| app.draw_settings(surface, repo));
                },
            }

            if !is_splash {
                self.draw_surface(frame, DrawSurface::Title, |app, surface| app.draw_title(surface));
            }

            // Side panes are hidden on splash/settings because those views own the frame.
            match self.viewport {
                Viewport::Splash => {},
                Viewport::Settings => {},
                _ => {
                    if self.layout_config.is_branches {
                        self.draw_surface(frame, DrawSurface::Branches, |app, surface| app.draw_branches(surface));
                    }
                    if self.layout_config.is_tags {
                        self.draw_surface(frame, DrawSurface::Tags, |app, surface| app.draw_tags(surface));
                    }
                    if self.layout_config.is_stashes {
                        self.draw_surface(frame, DrawSurface::Stashes, |app, surface| app.draw_stashes(surface, repo));
                    }
                    if self.layout_config.is_reflogs {
                        self.draw_surface(frame, DrawSurface::Reflogs, |app, surface| app.draw_reflogs(surface));
                    }
                    if self.layout_config.is_worktrees {
                        self.draw_surface(frame, DrawSurface::Worktrees, |app, surface| app.draw_worktrees(surface));
                    }
                    if self.layout_config.is_status {
                        self.draw_surface(frame, DrawSurface::Status, |app, surface| app.draw_status(surface));
                    }
                    if self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) {
                        self.draw_surface(frame, DrawSurface::Inspector, |app, surface| app.draw_inspector(surface, repo));
                    }
                },
            }

            if !is_splash {
                self.draw_surface(frame, DrawSurface::Statusbar, |app, surface| app.draw_statusbar(surface, repo));
            }

            // Modals render last so they overlay panes without changing pane layout.
            match self.focus {
                Focus::ModalCheckout => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_checkout(surface));
                },
                Focus::ModalSolo => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_solo(surface));
                },
                Focus::ModalDeleteBranch => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_delete_branch(surface, repo));
                },
                Focus::ModalWorktreeChooser => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_worktree_chooser(surface));
                },
                Focus::ModalRemoveWorktree => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_remove_worktree(surface));
                },
                Focus::ModalDeleteTag => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_delete_tag(surface));
                },
                Focus::ModalOperationProgress | Focus::ModalOperationConflict | Focus::ModalOperationSuccess => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_rebase(surface));
                },
                Focus::ModalError => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_error(surface));
                },
                Focus::ModalCommit => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_input(surface, STR_CREATE_COMMIT));
                },
                Focus::ModalCherrypick => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_input(surface, STR_CHERRYPICK_COMMIT));
                },
                Focus::ModalCreateBranch => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_input(surface, STR_CREATE_BRANCH));
                },
                Focus::ModalCreateWorktreeName => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_input(surface, STR_CREATE_WORKTREE_NAME));
                },
                Focus::ModalCreateWorktreePath => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_input(surface, STR_CREATE_WORKTREE_PATH));
                },
                Focus::ModalLockWorktree => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_input(surface, STR_LOCK_WORKTREE));
                },
                Focus::ModalGrep => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_input(surface, STR_FIND_SHA));
                },
                Focus::ModalTag => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_input(surface, STR_CREATE_TAG));
                },
                Focus::ModalKeyCapture => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_key_capture(surface));
                },
                Focus::ModalAuth => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_auth(surface));
                },
                Focus::ModalNetworkProgress => {
                    self.draw_surface(frame, DrawSurface::Modal, |app, surface| app.draw_modal_network_progress(surface));
                },
                _ => {},
            }
        } else {
            self.draw_surface(frame, DrawSurface::Splash, |app, surface| app.draw_splash(surface));
        }
    }

    pub fn reload(&mut self, override_path: Option<String>) {
        self.surface_buffers.clear();

        // Preserve branch filters across reloads because reload also follows git actions.
        let visible_branch_names = self.branches.visible_branch_names.clone();
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

        self.branches.visible_branch_names = visible_branch_names;

        // Prefer an explicit path, then the current path, then the first non-flag CLI arg.
        let path = if let Some(path) = override_path {
            path
        } else if let Some(path) = self.path.clone() {
            path
        } else {
            env::args().skip(1).find(|arg| !arg.starts_with('-')).unwrap_or_else(|| ".".to_string())
        };
        let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from("."));
        let absolute_path: PathBuf = try_into_git_repo_root(&canonical_path).unwrap_or(canonical_path.clone());

        // Failure keeps the app usable by falling back to the splash screen.
        let repo = match Repository::open(&absolute_path) {
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
            self.worktrees = Worktrees::from_entries(list_worktrees(repo, Some(current_path.as_path())).unwrap_or_default());

            // Recent paths are append-only here; the splash screen controls selection.
            if !self.recent.iter().any(|v| v == &absolute_path) {
                self.recent.push(absolute_path.clone());
                self.save_recent();
            }

            // Cancel the previous walker before spawning a new one for this repository state.
            if let Some(tx) = self.graph_tx.take() {
                let _ = tx.send(GraphCommand::Shutdown);
            }
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
            self.graph_rx = Some(event_rx);

            // Move only serializable state into the worker thread.
            let visible_branch_names = self.branches.visible_branch_names.clone();
            let include_head_reflog_roots = self.layout_config.is_graph_reflogs;
            let worktrees = self.worktrees.entries.clone();

            // The worker streams partial graph state so large repositories become usable quickly.
            let handle = spawn_graph_service(
                GraphServiceConfig { generation, path: absolute_path, amount: 10000, visible_branch_names, include_head_reflog_roots, worktrees },
                command_rx,
                event_tx,
                cancel_clone,
            );

            self.walker_handle = Some(handle);
        }
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

                if is_first {
                    if self.viewport == Viewport::Splash {
                        self.viewport = Viewport::Graph;
                    }

                    self.uncommitted = get_filenames_diff_at_workdir(repo).expect("Couldn't get the file diff");
                    self.is_uncommitted_loaded = true;
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
                self.graph.requested_graph = None;

                if self.graph_selected != 0
                    && let Some(identity) = self.graph_identity_at(self.graph_selected)
                {
                    self.current_diff = get_filenames_diff_at_oid(repo, identity.oid);
                    self.current_diff_identity = Some(identity);
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
                        let oid = row.oid;
                        self.cache_graph_row(row);
                        if index == self.graph_selected && index != 0 {
                            self.current_diff = get_filenames_diff_at_oid(repo, oid);
                            self.current_diff_identity = self.graph_identity_at(index);
                        }
                    },
                    (PendingGraphLookup::OpenInspector, GraphLookupResult::GraphRow(Some(row))) => {
                        let index = row.index;
                        let oid = row.oid;
                        self.cache_graph_row(row);
                        if index == self.graph_selected {
                            self.current_diff = get_filenames_diff_at_oid(repo, oid);
                            self.current_diff_identity = self.graph_identity_at(index);
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
                    self.heatmap = heatmap;
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
            self.current_diff = get_filenames_diff_at_oid(repo, identity.oid);
            self.current_diff_identity = Some(identity);
        }
    }

    fn refresh_theme_assets(&mut self) {
        self.color = Rc::new(RefCell::new(ColorPicker::from_theme(&self.theme)));
        self.logo = vec![Span::styled("  guita", Style::default().fg(self.theme.COLOR_GRASS)), Span::styled("╭", Style::default().fg(self.theme.COLOR_GREEN))];
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.surface_buffers.clear();
        self.refresh_theme_assets();
    }

    pub fn load_theme_config(&mut self) {
        self.set_theme(load_theme());
    }

    pub fn save_theme_config(&self) {
        save_theme(&self.theme);
    }

    pub fn exit(&mut self) {
        self.is_exit = true;
    }
}

#[cfg(test)]
#[path = "../tests/app/state/app.rs"]
mod tests;
