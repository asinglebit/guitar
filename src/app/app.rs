use crate::{
    app::input::TextInput,
    core::stashes::Stashes,
    git::os::path::try_into_git_repo_root,
    helpers::{
        copy::{STR_CREATE_BRANCH, STR_CREATE_COMMIT, STR_CREATE_TAG, STR_FIND_SHA},
        heatmap::{DAYS, WEEKS, empty_heatmap},
        keymap::{Command, KeyBinding},
        layout::LayoutConfig,
        recent::{load_recent, save_recent},
    },
};
use crate::{
    app::{app_default::ViewerMode, app_layout::Layout},
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
    event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::enable_raw_mode,
};
use git2::Repository;
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

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    Viewport,
    Inspector,
    StatusTop,
    StatusBottom,
    Branches,
    Tags,
    Stashes,
    ModalCheckout,
    ModalSolo,
    ModalCommit,
    ModalCreateBranch,
    ModalDeleteBranch,
    ModalGrep,
    ModalTag,
    ModalDeleteTag,
}

#[derive(PartialEq, Eq)]
pub enum Direction {
    Down,
    Up,
}
pub struct App {
    // General
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

    // User
    pub name: String,
    pub email: String,

    // Walker utilities
    pub color: Rc<RefCell<ColorPicker>>,
    pub buffer: RefCell<Buffer>,
    pub walker_rx: Option<std::sync::mpsc::Receiver<WalkerOutput>>,
    pub walker_cancel: Option<Arc<AtomicBool>>,
    pub walker_handle: Option<std::thread::JoinHandle<()>>,

    // Walker data
    pub oids: Oids,
    pub branches: Branches,
    pub tags: Tags,
    pub stashes: Stashes,
    pub uncommitted: UncommittedChanges,

    // Cache
    pub current_diff: Vec<FileChange>,
    pub file_name: Option<String>,
    pub viewer_lines: Vec<ListItem<'static>>,
    pub viewer_edges: Vec<usize>,
    pub viewer_hunks: Vec<usize>,
    pub viewer_mode: ViewerMode,

    // Interface
    pub layout: Layout,

    // Focus
    pub layout_config: LayoutConfig,
    pub viewport: Viewport,
    pub focus: Focus,

    // Branches
    pub branches_selected: usize,
    pub branches_scroll: Cell<usize>,

    // Tags
    pub tags_selected: usize,
    pub tags_scroll: Cell<usize>,

    // Stashes
    pub stashes_selected: usize,
    pub stashes_scroll: Cell<usize>,

    // Graph
    pub graph_selected: usize,
    pub graph_scroll: Cell<usize>,

    // Viewer
    pub viewer_selected: usize,
    pub viewer_scroll: Cell<usize>,

    // Splash
    pub splash_selected: usize,
    pub splash_selections: Vec<usize>,

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

    // Modal checkout
    pub modal_checkout_selected: i32,

    // Modal solo
    pub modal_solo_selected: i32,

    // Modal editor
    pub modal_input: TextInput,

    // Modal delete a branch
    pub modal_delete_branch_selected: i32,

    // Modal delete a tag
    pub modal_delete_tag_selected: i32,

    // Exit
    pub is_exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // Enable faster Escape detection in supported terminals
        enable_raw_mode()?;
        execute!(stdout(), PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES))?;

        // Load the app and initialize state
        self.load_recent();
        self.load_layout();
        self.load_keymap();
        self.reload(None);

        // Main loop
        while !self.is_exit {
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                self.handle_events()?;
            }

            // Handle background processes
            if let Some(repo) = &self.repo.clone() {
                self.sync(repo);
            }

            // Draw the user interface
            terminal.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        // Compute the layout
        self.layout(frame);

        let is_splash = self.viewport == Viewport::Splash;

        frame.render_widget(
            Block::default()
                .borders(if is_splash { Borders::NONE } else { Borders::ALL })
                .border_style(Style::default().fg(self.theme.COLOR_BORDER))
                .border_type(ratatui::widgets::BorderType::Rounded),
            self.layout.app,
        );

        // Only proceed with repo-dependent views if we have a repo
        if let Some(repo) = &self.repo.clone() {
            // Viewport
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

            // Main layout
            if !is_splash {
                self.draw_title(frame);
            }

            // Panes
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
                    if self.layout_config.is_status {
                        self.draw_status(frame);
                    }
                    if self.layout_config.is_inspector && self.graph_selected != 0 {
                        self.draw_inspector(frame, repo);
                    }
                },
            }

            // Status bar
            if !is_splash {
                self.draw_statusbar(frame, repo);
            }

            // Modals
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
                Focus::ModalDeleteTag => {
                    self.draw_modal_delete_tag(frame);
                },
                Focus::ModalCommit => {
                    self.draw_modal_input(frame, STR_CREATE_COMMIT);
                },
                Focus::ModalCreateBranch => {
                    self.draw_modal_input(frame, STR_CREATE_BRANCH);
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
        // Reset
        self.heatmap = empty_heatmap();
        self.current_diff = Vec::new();
        self.viewer_lines = Vec::new();
        self.viewer_edges = Vec::new();
        self.viewer_hunks = Vec::new();
        self.branches = Branches::default();
        self.tags = Tags::default();
        self.stashes = Stashes::default();

        // Determine repo path
        let path = if let Some(path) = override_path {
            path
        } else if let Some(path) = self.path.clone() {
            path
        } else {
            let args: Vec<String> = env::args().collect();
            let path = if args.len() > 1 { &args[1] } else { &".".to_string() };
            path.to_string()
        };
        let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from("."));
        let absolute_path: PathBuf = try_into_git_repo_root(&canonical_path).unwrap_or(canonical_path.clone());

        // Try to open the repository, fallback to None
        let repo = match Repository::open(&absolute_path) {
            Ok(r) => Some(Rc::new(r)),
            Err(_) => None,
        };

        let absolute_path = absolute_path.display().to_string();
        self.path = Some(absolute_path.clone());
        self.repo = repo;

        // Only proceed if we successfully opened a repo
        if let Some(repo) = &self.repo {
            // Add the repo path to the list of recent repositories if not present already
            if !self.recent.iter().any(|v| v == &absolute_path) {
                self.recent.push(absolute_path.clone());
            }

            save_recent(&self.recent);

            // Update colors
            self.color = Rc::new(RefCell::new(ColorPicker::from_theme(&self.theme)));

            // Update logo
            self.logo = vec![Span::styled("  guita", Style::default().fg(self.theme.COLOR_GRASS)), Span::styled("╭", Style::default().fg(self.theme.COLOR_GREEN))];

            // Cancel any existing walker thread immediately
            if let Some(cancel_flag) = &self.walker_cancel {
                cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }

            // Try to join previous walker handle if present (best-effort, non-blocking)
            if let Some(handle) = self.walker_handle.take() {
                // detach by spawning a thread that joins to avoid blocking reload
                std::thread::spawn(move || {
                    let _ = handle.join();
                });
            }

            // Get user credentials
            let (name, email) = get_git_user_info(repo).expect("Couldn't get user credentials");
            self.name = name.unwrap();
            self.email = email.unwrap();

            // Restart the spinner
            self.spinner.start();

            // Create a new cancellation flag and channel
            let cancel = Arc::new(AtomicBool::new(false));
            let cancel_clone = cancel.clone();
            self.walker_cancel = Some(cancel);

            let (tx, rx) = channel();
            self.walker_rx = Some(rx);

            // Copy the repo path and visible branches
            let visible = self.branches.visible.clone();

            // Spawn a thread that computes something; it will check cancel flag between iterations
            let handle = thread::spawn(move || {
                // Create the walker
                let mut walk_ctx = Walker::new(absolute_path, 10000, visible).expect("Error");
                let mut is_first = true;

                // Walker loop
                loop {
                    // Breaker
                    if cancel_clone.load(std::sync::atomic::Ordering::SeqCst) {
                        break;
                    }

                    // Parse a chunk
                    let is_again = walk_ctx.walk();

                    // Send the message to the main thread
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
                        // Receiver dropped, stop
                        break;
                    }

                    // Break the loop if walker finished
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
            // Crude check to see if this is a first iteration
            if result.is_first {
                // Transition from the splash screen on startup
                if self.viewport == Viewport::Splash {
                    self.viewport = Viewport::Graph;
                }

                // Get uncomitted changes info
                self.uncommitted = get_filenames_diff_at_workdir(repo).expect("Couldn't get the file diff");
            }

            // Lookup tables
            self.oids = result.oids;

            // Buffer
            self.buffer = result.buffer;

            // Update branches
            self.branches.feed(&self.oids, &self.color, &result.branches_lanes, result.branches_local, result.branches_remote);

            // Update tags
            self.tags.feed(&self.oids, &self.color, &result.tags_lanes, result.tags_local);

            // Update stashes
            self.stashes.feed(&self.color, &result.stashes_lanes);

            // We parsed the entire repository
            if !result.is_again {
                // Stop the spinner
                self.spinner.stop();

                // Build the heatmap
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
