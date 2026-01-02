#[rustfmt::skip]
use std::{
    cell::{
        Cell,
        RefCell
    },
    rc::Rc,
    sync::{
        mpsc::{
            channel
        },
        Arc,
        atomic::{
            AtomicBool
        }
    },
    time::{
        Duration
    },
    thread,
    io,
};
#[rustfmt::skip]
use edtui::{
    EditorEventHandler,
    EditorState
};
#[rustfmt::skip]
use git2::{
    Repository
};
#[rustfmt::skip]
use indexmap::{
    IndexMap
};
#[rustfmt::skip]
use ratatui::{
    DefaultTerminal,
    Frame,
    style::{
        Style
    },
    crossterm::event,
    widgets::{
        ListItem,
        Block,
        Borders
    },
    text::{
        Span
    },
};
use crate::{core::stashes::Stashes, helpers::{copy::{STR_CREATE_BRANCH, STR_CREATE_COMMIT, STR_CREATE_TAG, STR_FIND_SHA}, heatmap::{DAYS, WEEKS}, keymap::{Command, KeyBinding}}};
#[rustfmt::skip]
use crate::{
    app::{
        app_layout::{
            Layout
        }
    },
    core::{
        oids::{
            Oids
        },
        branches::{
            Branches
        },
        walker::{
            Walker,
            WalkerOutput
        },
        buffer::{
            Buffer
        },
        tags::{
            Tags
        }
    },
    helpers::{
        palette::*,
        colors::{
            ColorPicker
        },
        spinner::{
            Spinner
        }
    },
    git::{
        queries::{
            diffs::{
                get_filenames_diff_at_workdir
            },
            commits::{
                get_git_user_info
            },
            helpers::{
                FileChange,
                UncommittedChanges
            }
        }
    },
};

#[derive(PartialEq, Eq, Debug)]
pub enum Viewport {
    Graph,
    Viewer,
    Editor,
    Splash,
    Settings
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
    ModalDeleteTag
}

#[derive(PartialEq, Eq)]
pub enum Direction {
    Down,
    Up,
}
pub struct App {

    // General
    pub logo: Vec<Span<'static>>,
    pub path: String,
    pub repo: Rc<Repository>,
    pub hint: String,
    pub spinner: Spinner,
    pub keymap: IndexMap<KeyBinding, Command>,
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

    // Interface
    pub layout: Layout,

    // Focus
    pub is_shas: bool,
    pub is_minimal: bool,
    pub is_branches: bool,
    pub is_status: bool,
    pub is_tags: bool,
    pub is_stashes: bool,
    pub is_inspector: bool,
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

    // Settings
    pub settings_selected: usize,
    pub settings_selections: Vec<usize>,

    // Editor
    pub file_editor: EditorState,
    pub file_editor_event_handler: EditorEventHandler,

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
    pub modal_editor: EditorState,
    pub modal_editor_event_handler: EditorEventHandler,

    // Modal delete a branch
    pub modal_delete_branch_selected: i32,

    // Modal delete a tag
    pub modal_delete_tag_selected: i32,

    // Exit
    pub is_exit: bool,
}

impl App  {
    
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {

        self.load_layout();
        self.load_keymap();
        self.reload();

        // Main loop
        while !self.is_exit {

            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                self.handle_events()?;
            }

            // Handle background processes
            self.sync();

            // Draw the user interface
            terminal.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        
        let minimal_horizontal_space = if (self.is_branches || self.is_tags || self.is_stashes) && (self.is_inspector || self.is_status) { 100 } else { 50 };
        let is_zen = frame.area().width < minimal_horizontal_space;

        // Compute the layout
        self.layout(frame);
        
        let is_splash = self.viewport == Viewport::Splash || is_zen;

        frame.render_widget( Block::default()
            .borders(if is_splash { Borders::NONE } else { Borders::ALL })
            .border_style(Style::default().fg(self.theme.COLOR_BORDER))
            .border_type(ratatui::widgets::BorderType::Rounded), self.layout.app);

        if is_zen {
            self.draw_splash(frame);
            return;
        }
        
        // Viewport
        match self.viewport {
            Viewport::Graph => {
                self.draw_graph(frame);
            }
            Viewport::Viewer => {
                self.draw_viewer(frame);
            }
            Viewport::Editor => {
                self.draw_editor(frame);
            }
            Viewport::Splash => {
                self.draw_splash(frame);
            }
            Viewport::Settings => {
                self.draw_settings(frame);
            }
        }
                
        // Main layout
        if !is_splash {
            self.draw_title(frame);
        }

        // Panes
        match self.viewport {
            Viewport::Splash => {}
            Viewport::Settings => {}
            _ => {
                if self.is_branches {
                    self.draw_branches(frame);
                }
                if self.is_tags {
                    self.draw_tags(frame);
                }
                if self.is_stashes {
                    self.draw_stashes(frame);
                }
                if self.is_status {
                    self.draw_status(frame);
                }
                if self.is_inspector && self.graph_selected != 0 {
                    self.draw_inspector(frame);
                }
            }
        }

        // Status bar
        if !is_splash {
            self.draw_statusbar(frame);
        }

        // Modals
        match self.focus {
            Focus::ModalCheckout => {
                self.draw_modal_checkout(frame);
            }
            Focus::ModalSolo => {
                self.draw_modal_solo(frame);
            }
            Focus::ModalDeleteBranch => {
                self.draw_modal_delete_branch(frame);
            }
            Focus::ModalDeleteTag => {
                self.draw_modal_delete_tag(frame);
            }
            Focus::ModalCommit => {
                self.draw_modal_input(frame, STR_CREATE_COMMIT);
            }
            Focus::ModalCreateBranch => {
                self.draw_modal_input(frame, STR_CREATE_BRANCH);
            }
            Focus::ModalGrep => {
                self.draw_modal_input(frame, STR_FIND_SHA);
            }
            Focus::ModalTag => {
                self.draw_modal_input(frame, STR_CREATE_TAG);
            }
            _ => {}
        }
    }

    pub fn reload(&mut self) {

        // Update colors        
        self.color = Rc::new(RefCell::new(ColorPicker::from_theme(&self.theme)));

        // Update logo
        self.logo = vec![
            Span::styled("  g", Style::default().fg(self.theme.COLOR_GRASS)),
            Span::styled("u", Style::default().fg(self.theme.COLOR_GRASS)),
            Span::styled("i", Style::default().fg(self.theme.COLOR_GRASS)),
            Span::styled("t", Style::default().fg(self.theme.COLOR_GRASS)),
            Span::styled("a", Style::default().fg(self.theme.COLOR_GRASS)),
            Span::styled("╭", Style::default().fg(self.theme.COLOR_GREEN))
        ];

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
        let (name, email) = get_git_user_info(&self.repo).expect("Error");
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
        let path = self.path.clone();
        let visible = self.branches.visible.clone();

        // Spawn a thread that computes something; it will check cancel flag between iterations
        let handle = thread::spawn(move || {
            // Create the walker
            let mut walk_ctx = Walker::new(path, 10000, visible).expect("Error");
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
                if tx.send(WalkerOutput {
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
                }).is_err() {
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

    pub fn sync(&mut self) {
        if let Some(rx) = &self.walker_rx && let Ok(result) = rx.try_recv() {

            // Crude check to see if this is a first iteration
            if result.is_first {

                // Transition from the splash screen on startup
                if self.viewport == Viewport::Splash {
                    self.viewport = Viewport::Graph;
                }

                // Get uncomitted changes info
                self.uncommitted = get_filenames_diff_at_workdir(&self.repo).expect("Error");
            }
            
            
            // Lookup tables
            self.oids = result.oids;

            // Buffer
            self.buffer = result.buffer;

            // Update branches
            self.branches.feed(
                &self.oids,
                &self.color,
                &result.branches_lanes,
                result.branches_local,
                result.branches_remote
            );

            // Update tags
            self.tags.feed(
                &self.oids,
                &self.color,
                &result.tags_lanes,
                result.tags_local
            );

            // Update stashes
            self.stashes.feed(
                &self.color,
                &result.stashes_lanes,
            );

            if !result.is_again {
                self.spinner.stop();
            }
        }
    }

    pub fn exit(&mut self) {
        self.is_exit = true;
    }
}
