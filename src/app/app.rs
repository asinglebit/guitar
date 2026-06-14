use crate::{
    app::input::TextInput,
    core::stashes::Stashes,
    core::worktrees::Worktrees,
    git::{
        os::path::try_into_git_repo_root,
        queries::{diffs::get_filenames_diff_at_oid, worktrees::list_worktrees},
    },
    helpers::{
        copy::{STR_CHERRYPICK_COMMIT, STR_CREATE_BRANCH, STR_CREATE_COMMIT, STR_CREATE_TAG, STR_CREATE_WORKTREE_NAME, STR_CREATE_WORKTREE_PATH, STR_FIND_SHA, STR_LOCK_WORKTREE},
        heatmap::{DAYS, WEEKS, empty_heatmap},
        keymap::{Command, KeyBinding},
        layout::LayoutConfig,
        recent::{load_recent, save_recent},
    },
};
use crate::{
    app::{
        app_default::{SplitViewerRow, ViewerMode},
        app_layout::Layout,
    },
    core::{
        branches::Branches,
        buffer::Buffer,
        oids::Oids,
        tags::Tags,
        walker::{Walker, WalkerOutput},
    },
    git::queries::{
        commits::get_git_user_info,
        diffs::get_filenames_diff_at_workdir,
        helpers::{FileChange, UncommittedChanges},
    },
    helpers::{colors::ColorPicker, heatmap::build_heatmap, keymap::InputMode, palette::*, spinner::Spinner},
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
    io,
    rc::Rc,
    sync::{Arc, atomic::AtomicBool, mpsc::channel},
    thread,
    time::Duration,
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
    ModalRebaseProgress,
    ModalRebaseConflict,
    ModalRebaseSuccess,
    ModalError,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PendingRebaseAction {
    Start(Oid),
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

#[derive(PartialEq, Eq)]
pub enum Direction {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDrag {
    LeftPane,
    RightPane,
    ViewerSplit,
    BranchesTags,
    BranchesStashes,
    BranchesWorktrees,
    TagsStashes,
    TagsWorktrees,
    StashesWorktrees,
    InspectorStatus,
    StatusFiles,
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
    pub buffer: RefCell<Buffer>,
    pub walker_rx: Option<std::sync::mpsc::Receiver<WalkerOutput>>,
    pub walker_cancel: Option<Arc<AtomicBool>>,
    pub walker_handle: Option<std::thread::JoinHandle<()>>,

    // Repository metadata consumed by graph, branch, tag, and stash panes.
    pub oids: Oids,
    pub branches: Branches,
    pub tags: Tags,
    pub stashes: Stashes,
    pub worktrees: Worktrees,
    pub uncommitted: UncommittedChanges,

    // Cached file and diff data for the currently selected graph or status row.
    pub current_diff: Vec<FileChange>,
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
    pub layout_drag: Option<LayoutDrag>,
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

    // Settings
    pub settings_selected: usize,
    pub settings_selections: Vec<usize>,

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

    // Modal editor
    pub modal_input: TextInput,
    pub pending_cherrypick_oid: Option<Oid>,
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

    // Modal rebase
    pub modal_rebase_message: String,
    pub pending_rebase_action: Option<PendingRebaseAction>,

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

                terminal.draw(|frame| self.draw(frame))?;
                self.run_pending_rebase_action();
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
                    if self.layout_config.is_worktrees {
                        self.draw_worktrees(frame);
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
                Focus::ModalDeleteTag => {
                    self.draw_modal_delete_tag(frame);
                },
                Focus::ModalRebaseProgress | Focus::ModalRebaseConflict | Focus::ModalRebaseSuccess => {
                    self.draw_modal_rebase(frame);
                },
                Focus::ModalError => {
                    self.draw_modal_error(frame);
                },
                Focus::ModalCommit => {
                    self.draw_modal_input(frame, STR_CREATE_COMMIT);
                },
                Focus::ModalCherrypick => {
                    self.draw_modal_input(frame, STR_CHERRYPICK_COMMIT);
                },
                Focus::ModalCreateBranch => {
                    self.draw_modal_input(frame, STR_CREATE_BRANCH);
                },
                Focus::ModalCreateWorktreeName => {
                    self.draw_modal_input(frame, STR_CREATE_WORKTREE_NAME);
                },
                Focus::ModalCreateWorktreePath => {
                    self.draw_modal_input(frame, STR_CREATE_WORKTREE_PATH);
                },
                Focus::ModalLockWorktree => {
                    self.draw_modal_input(frame, STR_LOCK_WORKTREE);
                },
                Focus::ModalGrep => {
                    self.draw_modal_input(frame, STR_FIND_SHA);
                },
                Focus::ModalTag => {
                    self.draw_modal_input(frame, STR_CREATE_TAG);
                },
                _ => {},
            }
        } else {
            self.draw_splash(frame);
        }
    }

    pub fn reload(&mut self, override_path: Option<String>) {
        // Preserve branch filters across reloads because reload also follows git actions.
        let visible_branch_names = self.branches.visible_branch_names.clone();

        // Clear derived data; the walker will repopulate it asynchronously.
        self.heatmap = empty_heatmap();
        self.current_diff = Vec::new();
        self.viewer_lines = Vec::new();
        self.viewer_split_rows = Vec::new();
        self.viewer_edges = Vec::new();
        self.viewer_hunks = Vec::new();
        self.branches = Branches::default();
        self.tags = Tags::default();
        self.stashes = Stashes::default();
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

        // Repository-specific state starts only after Repository::open succeeds.
        if let Some(repo) = &self.repo {
            let current_path = PathBuf::from(&absolute_path);
            self.worktrees = Worktrees::from_entries(list_worktrees(repo, Some(current_path.as_path())).unwrap_or_default());

            // Recent paths are append-only here; the splash screen controls selection.
            if !self.recent.iter().any(|v| v == &absolute_path) {
                self.recent.push(absolute_path.clone());
            }

            save_recent(&self.recent);

            // ColorPicker owns lane color rotation, so reset it with every fresh graph.
            self.color = Rc::new(RefCell::new(ColorPicker::from_theme(&self.theme)));

            // The logo uses theme colors and is rebuilt when the theme changes.
            self.logo = vec![Span::styled("  guita", Style::default().fg(self.theme.COLOR_GRASS)), Span::styled("╭", Style::default().fg(self.theme.COLOR_GREEN))];

            // Cancel the previous walker before spawning a new one for this repository state.
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

            let (tx, rx) = channel();
            self.walker_rx = Some(rx);

            // Move only serializable state into the worker thread.
            let visible_branch_names = self.branches.visible_branch_names.clone();

            // The worker streams partial graph state so large repositories become usable quickly.
            let handle = thread::spawn(move || {
                let mut walk_ctx = Walker::new(absolute_path, 10000, visible_branch_names).expect("Error");
                let mut is_first = true;

                loop {
                    if cancel_clone.load(std::sync::atomic::Ordering::SeqCst) {
                        break;
                    }

                    let is_again = walk_ctx.walk();

                    if tx
                        .send(WalkerOutput {
                            oids: walk_ctx.oids.clone(),
                            branches_lanes: walk_ctx.branches_lanes.clone(),
                            branches_local: walk_ctx.branches_local.clone(),
                            branches_remote: walk_ctx.branches_remote.clone(),
                            tags_lanes: walk_ctx.tags_lanes.clone(),
                            tags_local: walk_ctx.tags_local.clone(),
                            stashes_lanes: walk_ctx.stashes_lanes.clone(),
                            buffer: walk_ctx.buffer.clone(),
                            is_first,
                            is_again,
                        })
                        .is_err()
                    {
                        break;
                    }

                    if !is_again {
                        break;
                    } else {
                        is_first = false;
                    }
                }
            });

            self.walker_handle = Some(handle);
        }
    }

    pub fn sync(&mut self, repo: &git2::Repository) {
        if let Some(rx) = &self.walker_rx
            && let Ok(result) = rx.try_recv()
        {
            // First batch is the moment the graph can replace the splash screen.
            if result.is_first {
                if self.viewport == Viewport::Splash {
                    self.viewport = Viewport::Graph;
                }

                // Workdir state is cheap enough to refresh once at the start of a reload.
                self.uncommitted = get_filenames_diff_at_workdir(repo).expect("Couldn't get the file diff");
            }

            // Swap all walker-owned data together so panes stay in sync.
            self.oids = result.oids;
            self.worktrees.refresh_aliases(&self.oids);

            self.buffer = result.buffer;

            self.branches.feed(&self.oids, &self.color, &result.branches_lanes, result.branches_local, result.branches_remote);

            self.tags.feed(&self.oids, &self.color, &result.tags_lanes, result.tags_local);

            self.stashes.feed(&self.color, &result.stashes_lanes);

            // Keep the selected commit's file list fresh as more history arrives.
            if self.graph_selected != 0 && self.graph_selected < self.oids.get_commit_count() {
                let oid = self.oids.get_oid_by_idx(self.graph_selected);
                self.current_diff = get_filenames_diff_at_oid(repo, *oid);
            }

            // Heatmap waits for the full walk so its counts include every loaded commit.
            if !result.is_again {
                self.spinner.stop();

                self.heatmap = build_heatmap(repo, &self.oids.oids);
            }
        }
    }

    pub fn load_recent(&mut self) {
        self.recent = load_recent();
    }

    pub fn exit(&mut self) {
        self.is_exit = true;
    }
}
