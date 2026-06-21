pub mod app;
pub mod core {
    pub mod batcher;
    pub mod branches;
    pub mod buffer;
    pub mod chunk;
    pub mod graph_service;
    pub mod layers;
    pub mod oids;
    pub mod reflogs;
    pub mod renderers;
    pub mod stashes;
    pub mod submodules;
    pub mod tags;
    pub mod walker;
    pub mod worktrees;
}
pub mod git {
    pub mod auth;
    pub mod actions {
        pub mod branching;
        pub mod checkout;
        pub mod cherrypicking;
        pub mod committing;
        pub mod conflicts;
        pub mod fetching;
        pub mod merging;
        pub mod network;
        pub mod pushing;
        pub mod rebasing;
        pub mod remotes;
        pub mod resetting;
        pub mod reverting;
        pub mod staging;
        pub mod stashing;
        pub mod submodules;
        pub mod tagging;
        pub mod worktrees;
    }
    pub mod os {
        pub mod path;
    }
    pub mod queries {
        pub mod commits;
        pub mod diffs;
        pub mod file_history;
        pub mod files;
        pub mod helpers;
        pub mod reflogs;
        pub mod remotes;
        pub mod submodules;
        pub mod worktrees;
    }
}
pub mod helpers {
    pub mod branch_visibility;
    pub mod colors;
    pub mod heatmap;
    pub mod keymap;
    pub mod layout;
    pub mod localisation;
    pub mod logger;
    pub mod palette;
    pub mod recent;
    pub mod spinner;
    pub mod symbols;
    pub mod text;
    pub mod time;
    pub mod version;
}

pub use app::app::App;
pub use helpers::version::VERSION;
