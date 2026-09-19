use std::{
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender, channel},
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind};

// Directories that churn without changing what the app renders. Keeping them out of the watch set
// matters more than filtering their events: a recursive watch registers one OS handle per
// directory, and `.git/objects` alone can hold tens of thousands on a large repository.
const EXCLUDED_DIRECTORIES: &[&str] = &["target", "node_modules"];

// `.git` entries that are pure storage or append-only logs. Ref updates land in `.git/refs`,
// `.git/packed-refs` and the various `*_HEAD` files, all of which stay watched.
const EXCLUDED_GIT_ENTRIES: &[&str] = &["objects", "logs", "lfs", "modules"];

pub struct RepoWatcher {
    // Held only for its Drop: releasing the watcher unregisters every OS watch it owns.
    _watcher: RecommendedWatcher,
    pub rx: Receiver<()>,
    // The repository this watcher is bound to, so the app can tell when it needs re-targeting.
    pub path: String,
}

fn is_excluded(path: &Path, repo_root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(repo_root) else {
        return false;
    };
    let mut components = relative.components().map(|component| component.as_os_str().to_string_lossy().into_owned());
    match components.next() {
        Some(first) if first == ".git" => components.next().is_some_and(|second| EXCLUDED_GIT_ENTRIES.contains(&second.as_str())),
        Some(first) => EXCLUDED_DIRECTORIES.contains(&first.as_str()),
        None => false,
    }
}

// Reading a file produces Access events, and the app reads the repository constantly: the graph
// walker alone opens refs and objects on every reload. Treating those as changes would keep the
// debounce window permanently open, so a reload would be owed forever and never actually run.
// Metadata modifications are dropped for the same reason, since a read can bump an access time.
fn is_change(kind: &EventKind) -> bool {
    match kind {
        EventKind::Create(_) | EventKind::Remove(_) => true,
        EventKind::Modify(ModifyKind::Metadata(_)) => false,
        EventKind::Modify(_) => true,
        // Backends that cannot classify an event still report a real change.
        EventKind::Any => true,
        EventKind::Access(_) | EventKind::Other => false,
    }
}

// Watch the working tree one top-level entry at a time instead of recursing from the root, because
// a recursive root watch would pull `.git/objects` and build directories back in.
fn watch_paths(watcher: &mut RecommendedWatcher, repo_root: &Path) {
    let mut watch = |path: PathBuf, mode: RecursiveMode| {
        if path.exists() {
            let _ = watcher.watch(&path, mode);
        }
    };

    // The root itself, so new top-level entries are noticed. A directory created after this point
    // is not itself watched until the watcher is respawned, but its creation still reports.
    watch(repo_root.to_path_buf(), RecursiveMode::NonRecursive);

    let git_dir = repo_root.join(".git");
    watch(git_dir.clone(), RecursiveMode::NonRecursive);
    watch(git_dir.join("refs"), RecursiveMode::Recursive);
    watch(git_dir.join("worktrees"), RecursiveMode::Recursive);

    let Ok(entries) = std::fs::read_dir(repo_root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || is_excluded(&path, repo_root) || path == git_dir {
            continue;
        }
        watch(path, RecursiveMode::Recursive);
    }
}

// Returns None when the repository cannot be watched at all, for example when the OS watch limit is
// exhausted. That is a silent no-op by design: the feature degrades to manual reload.
pub fn spawn_repo_watcher(repo_path: &str) -> Option<RepoWatcher> {
    let repo_root = std::fs::canonicalize(repo_path).ok()?;
    let (tx, rx): (Sender<()>, Receiver<()>) = channel();

    let filter_root = repo_root.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
        let Ok(event) = result else {
            return;
        };
        // Coalescing happens in the main loop, so every interesting event is just a bare ping.
        if is_change(&event.kind) && event.paths.iter().any(|path| !is_excluded(path, &filter_root)) {
            let _ = tx.send(());
        }
    })
    .ok()?;

    watch_paths(&mut watcher, &repo_root);

    Some(RepoWatcher { _watcher: watcher, rx, path: repo_path.to_string() })
}

#[cfg(test)]
#[path = "../tests/helpers/watcher.rs"]
mod tests;
