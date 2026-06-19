use std::fmt::Display;

pub mod common {
    pub const DEFAULT_REMOTE: &str = "default";
    pub const LOADING: &str = "loading";
    pub const NONE: &str = "none";
    pub const NO_HEAD: &str = "no head";
    pub const NOT_INITIALIZED: &str = "not initialized";
    pub const UNKNOWN: &str = "-";
    pub const WORKING: &str = "working...";
}

pub mod empty {
    pub const NO_BODY: &str = "no body";
    pub const NO_BRANCHES: &str = "no branches";
    pub const NO_COMMITS: &str = "no commits";
    pub const NO_HEAD_REFLOG: &str = "no HEAD reflog";
    pub const NO_MESSAGE: &str = "no message";
    pub const NO_RECENT_REPOSITORIES: &str = "no recent repositories";
    pub const NO_REMOTES: &str = "no remotes";
    pub const NO_STAGED_CHANGES: &str = "no staged changes";
    pub const NO_STASHES: &str = "no stashes";
    pub const NO_SUBMODULES: &str = "no submodules";
    pub const NO_SUMMARY: &str = "no summary";
    pub const NO_TAGS: &str = "no tags";
    pub const NO_UNSTAGED_CHANGES: &str = "no unstaged changes";
    pub const NO_WORKTREES: &str = "no worktrees";
    pub const SEARCH: &str = "search";
}

pub mod errors {
    use super::Display;

    pub const ABORT_NO_OPERATION: &str = "Abort failed: no rebase, cherry-pick, revert, or merge in progress";
    pub const ADD_REMOTE_INVALID_NAME: &str = "Add remote failed: remote name is invalid";
    pub const ADD_REMOTE: &str = "Add remote failed";
    pub const CHECKOUT: &str = "Checkout failed";
    pub const CHERRYPICK: &str = "Cherry-pick failed";
    pub const CHERRYPICK_NO_MESSAGE: &str = "Cherry-pick failed: no commit message was provided";
    pub const CHERRYPICK_NO_PENDING: &str = "Cherry-pick failed: no commit is pending";
    pub const COMMIT: &str = "Commit failed";
    pub const CONTINUE_NO_OPERATION: &str = "Continue failed: no rebase, cherry-pick, revert, or merge in progress";
    pub const CREATE_BRANCH: &str = "Create branch failed";
    pub const CREATE_BRANCH_NO_COMMIT: &str = "Create branch failed: no commit is selected";
    pub const CREATE_TAG: &str = "Create tag failed";
    pub const CREATE_TAG_NO_COMMIT: &str = "Create tag failed: no commit is selected";
    pub const CREATE_WORKTREE: &str = "Create worktree failed";
    pub const CREATE_WORKTREE_INVALID_NAME: &str = "Create worktree failed: names cannot be empty or contain path separators";
    pub const CREATE_WORKTREE_EMPTY_PATH: &str = "Create worktree failed: path cannot be empty";
    pub const CREATE_WORKTREE_NO_COMMIT: &str = "Create worktree failed: no commit is selected";
    pub const DELETE_BRANCH: &str = "Delete branch failed";
    pub const DELETE_BRANCH_CURRENT: &str = "Delete branch failed: cannot delete the current branch";
    pub const DELETE_BRANCH_INVALID_REMOTE: &str = "Delete branch failed: remote branch name is invalid";
    pub const DELETE_REMOTE: &str = "Delete remote failed";
    pub const DELETE_REMOTE_NO_PENDING: &str = "Delete remote failed: no remote is pending";
    pub const DELETE_TAG: &str = "Delete tag failed";
    pub const DROP_STASH: &str = "Drop stash failed";
    pub const EDIT_REMOTE: &str = "Edit remote failed";
    pub const EDIT_REMOTE_NO_PENDING: &str = "Edit remote failed: no remote is pending";
    pub const FILE_DIFF: &str = "Couldn't get the file diff";
    pub const FILE_HISTORY_WORKER_UNAVAILABLE: &str = "File history failed: graph worker is unavailable";
    pub const GIT_NETWORK_ALREADY_RUNNING: &str = "Git network operation failed: another network operation is already running";
    pub const GIT_NETWORK_PANICKED: &str = "Git network operation failed: worker thread panicked";
    pub const GIT_OPERATION_NO_REPOSITORY: &str = "Git operation failed: no repository is open";
    pub const HARD_RESET: &str = "Hard reset failed";
    pub const LOCK_WORKTREE: &str = "Lock worktree failed";
    pub const LOCK_WORKTREE_INVALID: &str = "Lock worktree failed: only valid linked worktrees can be locked";
    pub const MERGE: &str = "Merge failed";
    pub const MIXED_RESET: &str = "Mixed reset failed";
    pub const OPEN_REPOSITORY: &str = "Open repository failed";
    pub const OPEN_SUBMODULE_NOT_INITIALIZED: &str = "Open submodule failed: submodule is not initialized. Run update/init first.";
    pub const OPEN_WORKTREE_INVALID_PATH: &str = "Open worktree failed: worktree path is invalid";
    pub const POP_STASH: &str = "Pop stash failed";
    pub const PUSH_DETACHED_HEAD: &str = "Push failed: detached HEAD has no current branch";
    pub const REBASE: &str = "Rebase failed";
    pub const REMOVE_WORKTREE: &str = "Remove worktree failed";
    pub const REMOVE_WORKTREE_FORBIDDEN: &str = "Remove worktree failed: cannot remove current, main, or locked worktrees";
    pub const RENAME_BRANCH: &str = "Rename branch failed";
    pub const RENAME_BRANCH_LOCAL_ONLY: &str = "Rename branch failed: only local branches can be renamed";
    pub const RENAME_BRANCH_NO_PENDING: &str = "Rename branch failed: no branch is pending";
    pub const RENAME_REMOTE: &str = "Rename remote failed";
    pub const RENAME_REMOTE_NO_PENDING: &str = "Rename remote failed: no remote is pending";
    pub const REFLOG_COMMIT_HIDDEN: &str = "Reflog commit is hidden from the graph. Press 9 to show graph reflogs.";
    pub const RESET_FILE: &str = "Reset file failed";
    pub const REVERT: &str = "Revert failed";
    pub const REVERT_MERGE_UNSUPPORTED: &str = "Revert failed: reverting merge commits is not supported";
    pub const REVERT_NO_MESSAGE: &str = "Revert failed: no commit message was provided";
    pub const REVERT_NO_PENDING: &str = "Revert failed: no commit is pending";
    pub const SAVE_KEYMAP: &str = "Save keymap failed";
    pub const SET_DEFAULT_REMOTE: &str = "Set default remote failed";
    pub const STAGE_ALL: &str = "Stage all failed";
    pub const STAGE_FILE: &str = "Stage file failed";
    pub const STAGE_FILE_CONFLICT: &str = "Stage file failed: resolve conflicts in your editor, then continue the active operation";
    pub const STAGE_SUBMODULE: &str = "Stage submodule failed";
    pub const STASH: &str = "Stash failed";
    pub const SYNC_SUBMODULE: &str = "Sync submodule failed";
    pub const UNSTAGE_ALL: &str = "Unstage all failed";
    pub const UNSTAGE_FILE: &str = "Unstage file failed";
    pub const UNSTAGE_FILE_CONFLICT: &str = "Unstage file failed: resolve conflicts in your editor, then continue the active operation";
    pub const UNSTAGE_SUBMODULE: &str = "Unstage submodule failed";
    pub const UNLOCK_WORKTREE: &str = "Unlock worktree failed";

    pub fn with_error(prefix: &str, error: impl Display) -> String {
        format!("{prefix}: {error}")
    }

    pub fn authentication_failed(operation: &str, attempts: usize) -> String {
        format!("{operation} failed: authentication failed after {attempts} attempts")
    }

    pub fn auth_cancelled(operation: &str) -> String {
        format!("{operation} cancelled: authentication was not provided")
    }

    pub fn no_remotes_configured(operation: &str) -> String {
        format!("{operation} failed: no remotes configured")
    }

    pub fn operation_failed(operation: &str, error: impl Display) -> String {
        format!("{operation} failed: {error}")
    }

    pub fn walker_failed(error: impl Display) -> String {
        format!("Walker failed: {error}")
    }
}

pub mod inspector {
    pub const AUTHORED_BY: &str = "authored by:";
    pub const COMMIT_SHA: &str = "commit sha:";
    pub const COMMITTED_BY: &str = "committed by:";
    pub const CONFLICTED_FILES: &str = "conflicted files:";
    pub const FEATURED_BRANCHES: &str = "featured branches:";
    pub const HEAD_REFLOG: &str = "head reflog:";
    pub const MESSAGE_BODY: &str = "message body:";
    pub const MESSAGE_SUMMARY: &str = "message summary:";
    pub const NEXT_ACTION: &str = "next action:";
    pub const OPERATION_CONFLICTS: &str = "operation conflicts";
    pub const PARENT_SHAS: &str = "parent shas:";
    pub const REPOSITORY_STATE: &str = "repository state:";
    pub const RESOLVE_CONFLICTS_ACTION: &str = "resolve files externally, then action+Shift+C";
}

pub mod keymap {
    pub const ACTION_MODE: &str = "action";
    pub const ALT: &str = "Alt";
    pub const BACK_TAB: &str = "BackTab";
    pub const BACKSPACE: &str = "Backspace";
    pub const CAPS_LOCK: &str = "CapsLock";
    pub const CHAR: &str = "Char";
    pub const COMMAND: &str = "Command";
    pub const CONTROL: &str = "Control";
    pub const CTRL: &str = "Ctrl";
    pub const DELETE: &str = "Delete";
    pub const DOWN: &str = "Down";
    pub const END: &str = "End";
    pub const ENTER: &str = "Enter";
    pub const ESC: &str = "Esc";
    pub const HOME: &str = "Home";
    pub const INSERT: &str = "Insert";
    pub const LEFT: &str = "Left";
    pub const META: &str = "Meta";
    pub const NORMAL_MODE: &str = "normal";
    pub const NULL: &str = "Null";
    pub const NUM_LOCK: &str = "NumLock";
    pub const PAGE_DOWN: &str = "PageDown";
    pub const PAGE_UP: &str = "PageUp";
    pub const PAUSE: &str = "Pause";
    pub const PRINT_SCREEN: &str = "PrintScreen";
    pub const RIGHT: &str = "Right";
    pub const SCROLL_LOCK: &str = "ScrollLock";
    pub const SHIFT: &str = "Shift";
    pub const SPACE: &str = "Space";
    pub const TAB: &str = "Tab";
    pub const UNSUPPORTED: &str = "Unsupported";
    pub const UP: &str = "Up";
}

pub mod menu {
    pub const ABORT_OPERATION: &str = "Abort operation";
    pub const ADD_REMOTE: &str = "Add remote";
    pub const APPLY_THEME: &str = "Apply theme";
    pub const BACK: &str = "Back";
    pub const BACK_TO_GRAPH: &str = "Back to graph";
    pub const CHECKOUT: &str = "Checkout";
    pub const CHECKOUT_BRANCH: &str = "Checkout branch";
    pub const CHERRYPICK: &str = "Cherry-pick";
    pub const COMMIT: &str = "Commit";
    pub const CONTINUE_OPERATION: &str = "Continue operation";
    pub const CREATE_BRANCH: &str = "Create branch";
    pub const CREATE_BRANCH_HERE: &str = "Create branch here";
    pub const CREATE_TAG: &str = "Create tag";
    pub const CREATE_WORKTREE: &str = "Create worktree";
    pub const DELETE_BRANCH: &str = "Delete branch";
    pub const DELETE_REMOTE: &str = "Delete remote";
    pub const DELETE_TAG: &str = "Delete tag";
    pub const DISCARD_FILE_CHANGES: &str = "Discard file changes";
    pub const DROP_STASH: &str = "Drop stash";
    pub const EDIT_FETCH_URL: &str = "Edit fetch URL";
    pub const EDIT_PUSH_URL: &str = "Edit push URL";
    pub const EXIT: &str = "Exit";
    pub const FETCH: &str = "Fetch";
    pub const FIND: &str = "Find";
    pub const FIND_FILE: &str = "Find file";
    pub const HARD_RESET: &str = "Hard reset";
    pub const LOCK_WORKTREE: &str = "Lock worktree";
    pub const MERGE: &str = "Merge";
    pub const MIXED_RESET: &str = "Mixed reset";
    pub const MOVE_DOWN: &str = "Move down";
    pub const MOVE_UP: &str = "Move up";
    pub const OPEN_COMMIT: &str = "Open commit";
    pub const OPEN_FILE: &str = "Open file";
    pub const OPEN_REPOSITORY: &str = "Open repository";
    pub const OPEN_STASH_COMMIT: &str = "Open stash commit";
    pub const OPEN_SUBMODULE: &str = "Open submodule";
    pub const OPEN_WORKTREE: &str = "Open worktree";
    pub const POP_STASH: &str = "Pop stash";
    pub const PUSH: &str = "Push";
    pub const REBASE: &str = "Rebase";
    pub const REBIND_SHORTCUT: &str = "Rebind shortcut";
    pub const RELOAD: &str = "Reload";
    pub const REMOVE: &str = "Remove";
    pub const REMOVE_WORKTREE: &str = "Remove worktree";
    pub const RENAME_BRANCH: &str = "Rename branch";
    pub const RENAME_REMOTE: &str = "Rename remote";
    pub const RETURN_TO_PARENT_REPOSITORY: &str = "Return to parent repository";
    pub const REVERT: &str = "Revert";
    pub const SET_AS_DEFAULT: &str = "Set as default";
    pub const SETTINGS: &str = "Settings";
    pub const SHOW_DETAILS: &str = "Show details";
    pub const SHOW_FILES_STATUS: &str = "Show files/status";
    pub const SHOW_FULL_DIFF: &str = "Show full diff";
    pub const SHOW_HUNK_ROWS: &str = "Show hunk rows";
    pub const SHOW_SPLIT_DIFF: &str = "Show split diff";
    pub const SHOW_UNIFIED_DIFF: &str = "Show unified diff";
    pub const SOLO_BRANCH: &str = "Solo branch";
    pub const SPLASH_SCREEN: &str = "Splash screen";
    pub const STAGE_ALL: &str = "Stage all";
    pub const STAGE_FILE: &str = "Stage file";
    pub const STAGE_SUBMODULE: &str = "Stage submodule";
    pub const STASH_CHANGES: &str = "Stash changes";
    pub const SYNC_URL: &str = "Sync URL";
    pub const TOGGLE_BRANCH: &str = "Toggle branch";
    pub const UNLOCK_WORKTREE: &str = "Unlock worktree";
    pub const UNSTAGE_ALL: &str = "Unstage all";
    pub const UNSTAGE_FILE: &str = "Unstage file";
    pub const UNSTAGE_SUBMODULE: &str = "Unstage submodule";
    pub const UPDATE_INIT_SUBMODULE: &str = "Update/init submodule";

    pub fn open_settings_tab(tab: &str) -> String {
        format!("Open {tab}")
    }

    pub fn run_command(command: &str) -> String {
        format!("Run {command}")
    }
}

pub mod modal {
    pub const ACTION_CHOOSE: &str = "choose";
    pub const ACTION_CONFIRM: &str = "confirm";
    pub const ACTION_MOVE: &str = "move";
    pub const ACTION_OK: &str = "ok";
    pub const ACTION_SAVE: &str = "save";
    pub const ACTION_SUBMIT: &str = "submit";
    pub const ACTION_SWITCH_FIELD: &str = "switch field";
    pub const AUTH_KEY: &str = "key:";
    pub const AUTH_PASSPHRASE: &str = "passphrase";
    pub const AUTH_PASSWORD_TOKEN: &str = "password / token";
    pub const AUTH_USER: &str = "user:";
    pub const AUTH_USERNAME: &str = "username";
    pub const CURRENT_SHORTCUT: &str = "current:";
    pub const DELETE_SELECTED_REMOTE: &str = "delete selected remote?";
    pub const ERROR_TITLE: &str = "error";
    pub const KEY_ENTER: &str = "enter";
    pub const KEY_TAB: &str = "tab";
    pub const KEY_CTRL_J_K: &str = "ctrl+j/k";
    pub const NAME_LABEL: &str = "name:";
    pub const NEW_SHORTCUT: &str = "new:";
    pub const NEW_SHORTCUT_WAITING: &str = "new: waiting for key";
    pub const PATH_LABEL: &str = "path:";
    pub const PRESS_KEY: &str = "press key";
    pub const PROMPT_CHERRYPICK_COMMIT: &str = "Enter cherry-pick commit message";
    pub const PROMPT_CREATE_BRANCH: &str = "Enter new branch name";
    pub const PROMPT_CREATE_COMMIT: &str = "Enter commit message";
    pub const PROMPT_CREATE_TAG: &str = "Enter new tag name";
    pub const PROMPT_CREATE_WORKTREE_NAME: &str = "Enter new worktree name";
    pub const PROMPT_CREATE_WORKTREE_PATH: &str = "Enter new worktree path";
    pub const PROMPT_FIND_FILE: &str = "Search repository files";
    pub const PROMPT_FIND_SHA: &str = "Enter commit SHA to search for";
    pub const PROMPT_LOCK_WORKTREE: &str = "Enter lock reason";
    pub const PROMPT_REMOTE_ADD_NAME: &str = "Enter new remote name";
    pub const PROMPT_REMOTE_ADD_URL: &str = "Enter new remote URL";
    pub const PROMPT_REMOTE_EDIT_PUSH_URL: &str = "Enter remote push URL";
    pub const PROMPT_REMOTE_EDIT_URL: &str = "Enter remote fetch URL";
    pub const PROMPT_REMOTE_RENAME: &str = "Enter renamed remote name";
    pub const PROMPT_RENAME_BRANCH: &str = "Enter renamed branch name";
    pub const PROMPT_REVERT_COMMIT: &str = "Enter revert commit message";
    pub const REMOTE_FALLBACK: &str = "remote";
    pub const REMOTE_LABEL: &str = "remote:";
    pub const REMOVE_SELECTED_WORKTREE: &str = "remove selected worktree?";
    pub const SELECT_BRANCH_CHECKOUT: &str = "select a branch to checkout";
    pub const SELECT_BRANCH_DELETE: &str = "select a branch to delete";
    pub const SELECT_BRANCH_RENAME: &str = "select a branch to rename";
    pub const SELECT_BRANCH_SOLO: &str = "select a branch to solo";
    pub const SELECT_BRANCH_TOGGLE: &str = "select a branch to toggle";
    pub const SELECT_TAG_DELETE: &str = "select a tag to delete";
    pub const SELECT_WORKTREE_OPEN: &str = "select a worktree to open";
    pub const SELECT_WORKTREE_REMOVE: &str = "select a worktree to remove";
    pub const SET_SHORTCUT: &str = "set shortcut";
    pub const TYPE_TO_SEARCH: &str = " type to search";
    pub const NO_MATCHES: &str = " no matches";

    pub fn auth_title(protocol: &str) -> String {
        format!("{protocol} authentication")
    }

    pub fn keymap_conflict(mode: &str, key: &str, command: &str) -> String {
        format!("conflict: {mode} {key} already runs {command}")
    }

    pub fn keymap_missing_mode(mode: &str) -> String {
        format!("missing keymap mode: {mode}")
    }

    pub fn keymap_missing_binding(mode: &str, key: &str) -> String {
        format!("missing binding: {mode} {key}")
    }

    pub fn keymap_binding_changed(mode: &str, key: &str, expected: &str, actual: &str) -> String {
        format!("binding changed: {mode} {key} was {expected}, now {actual}")
    }
}

pub mod network {
    pub const DELETE_REMOTE_BRANCH: &str = "Delete remote branch";
    pub const FETCH: &str = "Fetch";
    pub const GIT_NETWORK_OPERATION: &str = "Git network operation";
    pub const PROTOCOL_HTTP: &str = "HTTP";
    pub const PROTOCOL_HTTPS: &str = "HTTPS";
    pub const PROTOCOL_LOCAL: &str = "local";
    pub const PROTOCOL_REMOTE: &str = "remote";
    pub const PROTOCOL_SSH: &str = "SSH";
    pub const PUSH: &str = "Push";
    pub const PUSH_TAGS: &str = "Push tags";
    pub const UPDATE_SUBMODULE: &str = "Update submodule";

    pub fn deleting_remote_branch(remote_name: &str, branch: &str) -> String {
        format!("Deleting {remote_name}/{branch}...")
    }

    pub fn fetching(remote_name: &str) -> String {
        format!("Fetching {remote_name}...")
    }

    pub fn force_pushing(branch: &str, remote_name: &str) -> String {
        format!("Force pushing {branch} to {remote_name}...")
    }

    pub fn pushing(branch: &str, remote_name: &str) -> String {
        format!("Pushing {branch} to {remote_name}...")
    }

    pub fn pushing_tags(remote_name: &str) -> String {
        format!("Pushing local tags to {remote_name}...")
    }

    pub fn updating_submodule(name: &str) -> String {
        format!("Updating submodule {name}...")
    }
}

pub mod operations {
    pub const ABORTED: &str = "aborted";
    pub const CHERRYPICK: &str = "cherrypick";
    pub const CHERRYPICK_ABORTED: &str = "Cherry-pick aborted.";
    pub const CHERRYPICK_COMMIT_FALLBACK: &str = "Cherry-pick commit";
    pub const CHERRYPICK_COMPLETED: &str = "Cherry-pick completed.";
    pub const CHERRYPICK_CONFLICT: &str = "Cherry-pick stopped because conflicts need to be resolved.";
    pub const COMPLETE: &str = "complete";
    pub const CONFLICT: &str = "conflict";
    pub const MERGE: &str = "merge";
    pub const MERGE_ALREADY_UP_TO_DATE: &str = "Merge already up to date.";
    pub const MERGE_ABORTED: &str = "Merge aborted.";
    pub const MERGE_COMPLETED: &str = "Merge completed.";
    pub const MERGE_CONFLICT: &str = "Merge stopped because conflicts need to be resolved.";
    pub const MERGE_FAST_FORWARDED: &str = "Merge fast-forwarded.";
    pub const REBASE: &str = "rebase";
    pub const REBASE_ABORTED: &str = "Rebase aborted.";
    pub const REBASE_CONFLICT: &str = "Rebase stopped because conflicts need to be resolved.";
    pub const REVERT: &str = "revert";
    pub const REVERT_ABORTED: &str = "Revert aborted.";
    pub const REVERT_COMMIT_FALLBACK: &str = "Revert commit";
    pub const REVERT_COMPLETED: &str = "Revert completed.";
    pub const REVERT_CONFLICT: &str = "Revert stopped because conflicts need to be resolved.";
    pub const RESOLVE_CONFLICTS: &str = "resolve conflicts in your editor, then action+Shift+C";

    pub fn aborted(operation: &str) -> String {
        format!("{operation} {ABORTED}.")
    }

    pub fn aborting(operation: &str) -> String {
        format!("Aborting {operation}...")
    }

    pub fn continuing(operation: &str) -> String {
        format!("Continuing {operation}...")
    }

    pub fn cherrypicked(original_message: &str) -> String {
        format!("cherrypicked: {original_message}")
    }

    pub fn rebase_completed(applied: usize) -> String {
        if applied == 1 { "Rebase completed after applying 1 commit.".to_string() } else { format!("Rebase completed after applying {applied} commits.") }
    }

    pub fn reverted(original_message: &str) -> String {
        format!("reverted: {original_message}")
    }

    pub fn rebasing_selected_commit() -> String {
        "Rebasing the current branch onto the selected commit...".to_string()
    }

    pub fn merging_selected_commit() -> String {
        "Merging the selected commit into the current branch...".to_string()
    }
}

pub mod settings {
    pub const ACTIONS: &str = " actions:";
    pub const ACTIVE_CUSTOM: &str = " active custom:";
    pub const ACTIVE_CUSTOM_SYMBOLS: &str = " active custom symbols:";
    pub const AUTH: &str = "auth";
    pub const AUTHORIZATION: &str = " authorization:";
    pub const BRANCHES: &str = "branches";
    pub const COMMITTER_DATE_TIME: &str = "committer date/time";
    pub const COMMITTERS: &str = "committers";
    pub const CREDENTIALS: &str = " credentials:";
    pub const DEFAULT_REMOTE: &str = " default remote:";
    pub const DISPLAY: &str = "display";
    pub const EMAIL: &str = " email:";
    pub const ENTER_ACTION: &str = "(enter)";
    pub const GRAPH_METADATA: &str = " graph metadata:";
    pub const GRAPH_REFLOG_COMMITS: &str = "graph reflog commits";
    pub const HTTPS: &str = " https:";
    pub const HTTPS_DETAIL: &str = "username/password or token prompt ";
    pub const INSPECTOR: &str = "inspector";
    pub const KEYMAP: &str = " keymap:";
    pub const LAYOUT: &str = " layout:";
    pub const NAME: &str = " name:";
    pub const PANE_VISIBILITY: &str = " pane visibility:";
    pub const PATHS: &str = "paths";
    pub const PATHS_SECTION: &str = " paths:";
    pub const RECENT_FILE: &str = " recent file:";
    pub const RECENT_REPOSITORIES: &str = " recent repositories:";
    pub const REFLOG: &str = "reflog";
    pub const REFS: &str = "refs";
    pub const REMOTE_ERROR: &str = " remote error:";
    pub const REMOTES: &str = " remotes:";
    pub const REMOTES_ACTIONS_DETAIL: &str = "select remote to manage | + add remote to create ";
    pub const REPO: &str = "repo";
    pub const RESET_LAYOUT: &str = "reset layout";
    pub const SECRETS: &str = " secrets:";
    pub const SECRETS_DETAIL: &str = "session only ";
    pub const SETTINGS: &str = "settings";
    pub const SEARCH: &str = "search";
    pub const SHAS: &str = "SHAs";
    pub const SHORTCUTS: &str = "shortcuts";
    pub const SHORTCUTS_ACTION_MODE: &str = " shortcuts / action mode:";
    pub const SHORTCUTS_NORMAL_MODE: &str = " shortcuts / normal mode:";
    pub const SSH_FALLBACK: &str = " ssh fallback:";
    pub const SSH_FALLBACK_DETAIL: &str = "key passphrase prompt ";
    pub const SSH_AGENT_DETAIL: &str = "ssh-agent when available ";
    pub const STASHES: &str = "stashes";
    pub const STATUS: &str = "status";
    pub const SUBMODULES: &str = "submodules";
    pub const SYMBOLS: &str = " symbols:";
    pub const TAGS: &str = "tags";
    pub const THEME: &str = " theme:";
    pub const THEMES: &str = " themes:";
    pub const SYMBOL_THEME: &str = " symbol theme:";
    pub const SYMBOL_THEMES: &str = " symbol themes:";
    pub const VERSION: &str = " version:";
    pub const WORKTREES: &str = "worktrees";
    pub const ADD_REMOTE: &str = " + add remote";
    pub const FETCH_SUFFIX: &str = "fetch:";
    pub const PUSH_SUFFIX: &str = "push:";
}

pub mod splash {
    pub const ACTIONS: &str = "actions:";
    pub const KEY_MOVE_DOWN_FALLBACK: &str = "Shift + J";
    pub const KEY_MOVE_UP_FALLBACK: &str = "Shift + K";
    pub const KEY_REMOVE_FALLBACK: &str = "d";
    pub const LOADING: &str = "loading...";
    pub const MADE_WITH: &str = "made with ♡";
    pub const MOVE_DOWN: &str = "move down";
    pub const MOVE_UP: &str = "move up";
    pub const NOT_A_VALID_GIT_REPOSITORY: &str = "! not a valid git repository !";
    pub const RECENT_REPOSITORIES: &str = "recent repositories:";
    pub const REMOVE: &str = "remove";
    pub const REPOSITORY_URL: &str = "https://github.com/asinglebit/guitar";

    pub fn recent_actions(remove: &str, move_up: &str, move_down: &str) -> String {
        format!("{REMOVE} ({remove}) | {MOVE_UP} ({move_up}) | {MOVE_DOWN} ({move_down})")
    }

    pub fn actions(text: &str) -> String {
        format!("{ACTIONS} {text}")
    }
}

pub mod status {
    pub const DETACHED: &str = "detached";
    pub const DETACHED_HEAD: &str = "detached head:";
    pub const GRAPH: &str = "graph";
    pub const INSPECTOR: &str = "inspector";
    pub const MODAL: &str = "modal";
    pub const NOT_INITIALIZED: &str = "not initialized";
    pub const NO_HEAD_NO_COMMITS: &str = "no head (no commits yet)";
    pub const SEARCH: &str = "search";
    pub const STAGED: &str = "staged";
    pub const STASH: &str = "stash";
    pub const UNSTAGED: &str = "unstaged";
    pub const VIEWER: &str = "viewer";
    pub const MODIFIED: &str = "modified";
    pub const NEW_COMMITS: &str = "new commits";
    pub const UNTRACKED: &str = "untracked";
}
