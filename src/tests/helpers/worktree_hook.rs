use super::*;
use std::fs;

/// A hook that records what it was handed, so a test can read it back. Written
/// rather than fixtured because it has to be executable.
fn recording_hook(dir: &Path, log: &Path) -> PathBuf {
    let hook = dir.join("hook.sh");
    fs::write(&hook, format!("#!/bin/sh\nprintf '%s %s' \"$1\" \"$2\" > '{}'\n", log.display())).unwrap();
    let mut mode = fs::metadata(&hook).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
    fs::set_permissions(&hook, mode).unwrap();
    hook
}

#[test]
fn an_unset_hook_announces_nothing() {
    assert!(worktree_created_with(None, Path::new("/tmp/anywhere")).is_none());
}

#[test]
fn an_empty_hook_announces_nothing() {
    assert!(worktree_created_with(Some(OsString::from("")), Path::new("/tmp/anywhere")).is_none());
}

#[test]
fn the_hook_is_handed_the_event_and_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("log");
    let hook = recording_hook(dir.path(), &log);

    worktree_created_with(Some(hook.into_os_string()), Path::new("/projects/guitar-scratch")).unwrap().join().unwrap();

    assert_eq!(fs::read_to_string(&log).unwrap(), "created /projects/guitar-scratch");
}

#[test]
fn a_hook_that_is_not_there_is_not_an_error() {
    worktree_created_with(Some(OsString::from("/nonexistent/hook")), Path::new("/tmp/anywhere")).unwrap().join().unwrap();
}
