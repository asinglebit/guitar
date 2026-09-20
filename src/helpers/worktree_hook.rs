use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread::{self, JoinHandle},
};

/// The command to run when a worktree appears, when anything is listening.
/// atrium reads the same variable and spells the call the same way, so one
/// hook serves both tools.
pub const HOOK_ENV: &str = "WORKTREE_HOOK";

/// What happened, as the hook's first argument. The path says the rest.
pub const CREATED: &str = "created";

/// Say that a worktree appeared. Nothing listening is the ordinary case, and a
/// hook that fails is never worth interrupting what guitar was doing.
pub fn worktree_created(path: &Path) {
    let _ = worktree_created_with(std::env::var_os(HOOK_ENV), path);
}

/// Split out so a test can choose the hook without writing to the one
/// environment every test in the process shares.
pub fn worktree_created_with(hook: Option<OsString>, path: &Path) -> Option<JoinHandle<()>> {
    // Nothing to announce to, and that is fine.
    let hook = hook.filter(|hook| !hook.is_empty())?;
    Some(announce(hook, path.to_path_buf()))
}

/// On a thread of its own, so a hook that sleeps cannot hold up a frame -- and
/// waited on there, so it leaves no zombie behind. This is the only process
/// guitar starts.
fn announce(hook: OsString, path: PathBuf) -> JoinHandle<()> {
    thread::spawn(move || {
        let _ = Command::new(hook).arg(CREATED).arg(path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    })
}

#[cfg(test)]
#[path = "../tests/helpers/worktree_hook.rs"]
mod tests;
