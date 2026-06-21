use crate::core::worktrees::{WorktreeEntry, WorktreeKind};
use git2::{Diff, DiffOptions, Oid, Repository, Worktree, WorktreeLockStatus, WorktreePruneOptions};
use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
};

fn canonical_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn has_diff_delta(diff: &Diff<'_>) -> bool {
    diff.deltas().next().is_some()
}

fn dirty_diff_options() -> DiffOptions {
    let mut options = DiffOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(false).ignore_submodules(true);
    options
}

fn repo_dirty(repo: &Repository, head: Option<Oid>) -> bool {
    let mut workdir_options = dirty_diff_options();
    if repo.diff_index_to_workdir(None, Some(&mut workdir_options)).map(|diff| has_diff_delta(&diff)).unwrap_or(false) {
        return true;
    }

    let index = repo.index().ok();
    let head_tree = head.and_then(|oid| repo.find_commit(oid).ok()).and_then(|commit| commit.tree().ok());
    let mut staged_options = dirty_diff_options();
    repo.diff_tree_to_index(head_tree.as_ref(), index.as_ref(), Some(&mut staged_options)).map(|diff| has_diff_delta(&diff)).unwrap_or(false)
}

fn parallel_worktree_status_enabled() -> bool {
    env::var("GUITAR_PARALLEL_WORKTREE_STATUS").map(|value| value == "1" || ["true", "yes", "on"].iter().any(|enabled| value.eq_ignore_ascii_case(enabled))).unwrap_or(false)
}

fn repo_head_info(repo: &Repository) -> (Option<String>, Option<Oid>) {
    let Ok(head) = repo.head() else {
        return (None, None);
    };
    let branch = head.is_branch().then(|| head.shorthand().map(str::to_string)).flatten();
    (branch, head.target())
}

fn main_worktree_path(repo: &Repository) -> Option<PathBuf> {
    repo.commondir().parent().map(Path::to_path_buf)
}

fn entry_from_repository(repo: Option<&Repository>, name: String, path: PathBuf, kind: WorktreeKind, current_path: &Path) -> WorktreeEntry {
    let (branch, head) = repo.map(repo_head_info).unwrap_or((None, None));
    let is_dirty = repo.is_some_and(|repo| repo_dirty(repo, head));

    WorktreeEntry {
        name,
        path: path.clone(),
        branch,
        head,
        alias: None,
        kind,
        is_current: canonical_path(&path) == current_path,
        is_valid: repo.is_some(),
        is_prunable: false,
        locked_reason: None,
        is_dirty,
    }
}

struct LinkedWorktreeDescriptor {
    name: String,
    path: PathBuf,
    is_valid: bool,
    is_prunable: bool,
    locked_reason: Option<String>,
}

fn linked_descriptor(repo: &Repository, worktree_name: &str) -> Option<LinkedWorktreeDescriptor> {
    let worktree = repo.find_worktree(worktree_name).ok()?;
    let path = worktree.path().to_path_buf();
    let is_valid = worktree.validate().is_ok();
    let locked_reason = match worktree.is_locked() {
        Ok(WorktreeLockStatus::Unlocked) => None,
        Ok(WorktreeLockStatus::Locked(reason)) => Some(reason.unwrap_or_default()),
        Err(_) => None,
    };
    let is_prunable = !is_valid && is_prunable(&worktree);

    Some(LinkedWorktreeDescriptor { name: worktree_name.to_string(), path, is_valid, is_prunable, locked_reason })
}

fn linked_entry_from_descriptor(descriptor: LinkedWorktreeDescriptor, current_path: &Path) -> WorktreeEntry {
    let linked_repo = Repository::open(&descriptor.path).ok();

    let mut entry = entry_from_repository(linked_repo.as_ref(), descriptor.name, descriptor.path, WorktreeKind::Linked, current_path);
    entry.is_valid = descriptor.is_valid;
    entry.is_prunable = descriptor.is_prunable;
    entry.locked_reason = descriptor.locked_reason;

    entry
}

fn linked_entries(repo: &Repository, worktree_names: Vec<String>, current_path: &Path) -> Vec<WorktreeEntry> {
    let descriptors = worktree_names.iter().filter_map(|name| linked_descriptor(repo, name)).collect::<Vec<_>>();
    let mut entries = if descriptors.len() < 2 || !parallel_worktree_status_enabled() {
        descriptors.into_iter().map(|descriptor| linked_entry_from_descriptor(descriptor, current_path)).collect::<Vec<_>>()
    } else {
        thread::scope(|scope| {
            let handles = descriptors
                .into_iter()
                .map(|descriptor| {
                    let current = current_path.to_path_buf();
                    scope.spawn(move || linked_entry_from_descriptor(descriptor, &current))
                })
                .collect::<Vec<_>>();

            handles.into_iter().filter_map(|handle| handle.join().ok()).collect::<Vec<_>>()
        })
    };

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn is_prunable(worktree: &Worktree) -> bool {
    let mut opts = WorktreePruneOptions::new();
    worktree.is_prunable(Some(&mut opts)).unwrap_or(false)
}

pub fn list_worktrees(repo: &Repository, current_path: Option<&Path>) -> Result<Vec<WorktreeEntry>, git2::Error> {
    let owner = Repository::open(repo.commondir()).ok();
    let worktree_repo = owner.as_ref().unwrap_or(repo);
    let current =
        current_path.map(canonical_path).or_else(|| repo.workdir().map(canonical_path)).or_else(|| main_worktree_path(repo).map(|path| canonical_path(&path))).unwrap_or_else(|| PathBuf::from("."));

    let mut entries = Vec::new();

    if let Some(main_path) = main_worktree_path(worktree_repo) {
        let main_name = main_path.file_name().and_then(|name| name.to_str()).unwrap_or("main").to_string();
        entries.push(entry_from_repository(Some(worktree_repo), main_name, main_path, WorktreeKind::Main, &current));
    }

    let worktree_names = worktree_repo.worktrees()?.iter().flatten().map(str::to_string).collect::<Vec<_>>();
    entries.extend(linked_entries(worktree_repo, worktree_names, &current));

    Ok(entries)
}

#[cfg(test)]
#[path = "../../tests/git/queries/worktrees.rs"]
mod tests;
