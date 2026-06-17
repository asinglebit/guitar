#[allow(clippy::module_inception)]
pub mod app;

pub mod draw {
    pub mod branches;
    pub mod context_menu;
    pub mod graph;
    pub mod inspector;
    pub mod modals {
        pub mod auth;
        pub mod checkout;
        pub mod delete_branch;
        pub mod delete_tag;
        pub mod error;
        pub mod file_search;
        pub mod input;
        pub mod key_capture;
        pub mod rebase;
        pub mod remotes;
        pub mod remove_worktree;
        pub(crate) mod shared;
        pub mod solo;
        pub mod worktree_chooser;
    }
    pub(super) mod pane_window;
    pub mod reflogs;
    pub mod search;
    pub mod settings;
    pub mod splash;
    pub mod stashes;
    pub mod status;
    pub mod statusbar;
    pub mod submodules;
    pub mod tags;
    pub mod title;
    pub mod viewer;
    pub mod worktrees;
}

pub mod input {
    pub mod context_menu;
    pub mod events;
    pub mod git;
    pub mod handler;
    pub mod modals;
    pub mod navigation;
    pub mod remotes;
    pub mod submodules;
    pub mod text;
    pub mod worktrees;

    pub use text::TextInput;
}

pub mod state {
    pub mod defaults;
    pub mod layout;
}
