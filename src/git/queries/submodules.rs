use crate::core::{oids::gix_to_git2_oid, submodules::SubmoduleEntry};
use git2::Repository;
use gix::bstr::ByteSlice;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

const GITMODULES_PATH: &[u8] = b".gitmodules";
const INDEX_SCAN_BUFFER: usize = 64 * 1024;
const GITMODULES_OVERLAP: usize = 10;

fn open_gix_repo(repo: &Repository) -> Result<gix::Repository, git2::Error> {
    let path = repo.workdir().unwrap_or(repo.path());
    gix::open(path).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn current_branch(repo: &gix::Repository) -> Option<String> {
    let head = repo.head_name().ok().flatten()?;
    head.shorten().to_str().ok().map(str::to_string)
}

fn configured_branch(branch: Option<gix::submodule::config::Branch>) -> Option<String> {
    match branch? {
        gix::submodule::config::Branch::CurrentInSuperproject => Some(".".to_string()),
        gix::submodule::config::Branch::Name(name) => name.to_str().ok().map(str::to_string),
    }
}

fn changed_content_flags(changes: &[gix::status::Item]) -> (bool, bool) {
    let mut has_modified_content = false;
    let mut has_untracked_content = false;

    for change in changes {
        match change {
            gix::status::Item::IndexWorktree(item) => match item {
                gix::status::index_worktree::Item::DirectoryContents { entry, .. } if matches!(entry.status, gix::dir::entry::Status::Untracked) => {
                    has_untracked_content = true;
                },
                gix::status::index_worktree::Item::DirectoryContents { .. } => {
                    has_modified_content = true;
                },
                gix::status::index_worktree::Item::Rewrite { .. } => {
                    has_modified_content = true;
                },
                gix::status::index_worktree::Item::Modification { status, .. } => match status {
                    gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                        gix::status::plumbing::index_as_worktree::Change::Removed
                        | gix::status::plumbing::index_as_worktree::Change::Type { .. }
                        | gix::status::plumbing::index_as_worktree::Change::Modification { .. }
                        | gix::status::plumbing::index_as_worktree::Change::SubmoduleModification(_),
                    )
                    | gix::status::plumbing::index_as_worktree::EntryStatus::Conflict { .. }
                    | gix::status::plumbing::index_as_worktree::EntryStatus::IntentToAdd => {
                        has_modified_content = true;
                    },
                    gix::status::plumbing::index_as_worktree::EntryStatus::NeedsUpdate(_) => {},
                },
            },
            gix::status::Item::TreeIndex(_) => {
                has_modified_content = true;
            },
        }
    }

    (has_modified_content, has_untracked_content)
}

fn has_committed_or_workdir_submodule_metadata(repo: &Repository) -> bool {
    let gitmodules = Path::new(".gitmodules");
    if repo.workdir().is_some_and(|workdir| workdir.join(gitmodules).exists()) {
        return true;
    }

    let Ok(gix_repo) = open_gix_repo(repo) else {
        return false;
    };
    let Ok(tree_id) = gix_repo.head_tree_id_or_empty() else {
        return false;
    };
    let Ok(tree) = gix_repo.find_tree(tree_id) else {
        return false;
    };

    tree.lookup_entry_by_path(gitmodules).ok().flatten().is_some()
}

pub fn has_submodule_metadata(repo: &Repository) -> bool {
    has_committed_or_workdir_submodule_metadata(repo) || index_contains_gitmodules_path(repo)
}

fn index_contains_gitmodules_path(repo: &Repository) -> bool {
    let path = repo.path().join("index");
    file_contains_gitmodules_path(&path)
}

fn file_contains_gitmodules_path(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };

    let mut buffer = [0u8; INDEX_SCAN_BUFFER];
    let mut overlap = [0u8; GITMODULES_OVERLAP];
    let mut overlap_len = 0;

    loop {
        let Ok(read) = file.read(&mut buffer) else {
            return false;
        };
        if read == 0 {
            return false;
        }

        if overlap_len > 0 {
            let prefix_len = (GITMODULES_PATH.len() - 1).min(read);
            let mut boundary = [0u8; GITMODULES_OVERLAP * 2 + 1];
            boundary[..overlap_len].copy_from_slice(&overlap[..overlap_len]);
            boundary[overlap_len..overlap_len + prefix_len].copy_from_slice(&buffer[..prefix_len]);
            if boundary[..overlap_len + prefix_len].windows(GITMODULES_PATH.len()).any(|window| window == GITMODULES_PATH) {
                return true;
            }
        }

        if buffer[..read].windows(GITMODULES_PATH.len()).any(|window| window == GITMODULES_PATH) {
            return true;
        }

        let keep = GITMODULES_PATH.len().saturating_sub(1).min(read);
        overlap[..keep].copy_from_slice(&buffer[read - keep..read]);
        overlap_len = keep;
    }
}

pub fn list_submodules(repo: &Repository) -> Result<Vec<SubmoduleEntry>, git2::Error> {
    if !has_committed_or_workdir_submodule_metadata(repo) {
        return Ok(Vec::new());
    }

    let gix_repo = open_gix_repo(repo)?;
    let workdir = repo.workdir().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let mut entries = Vec::new();

    let Some(submodules) = gix_repo.submodules().map_err(|error| git2::Error::from_str(&error.to_string()))? else {
        return Ok(entries);
    };

    for submodule in submodules {
        let Ok(path) = submodule.path() else {
            continue;
        };
        let path = gix::path::from_bstring(path.into_owned());
        let name = submodule.name().to_str().ok().map(str::to_string).unwrap_or_else(|| path.display().to_string());
        let opened = submodule.open().ok().flatten();
        let status = submodule.status(gix::submodule::config::Ignore::None, false).ok();
        let state = status.as_ref().map(|status| status.state).unwrap_or_else(|| submodule.state().unwrap_or_default());
        let branch = opened.as_ref().and_then(current_branch).or_else(|| configured_branch(submodule.branch().ok().flatten()));
        let head = submodule.head_id().ok().flatten().map(gix_to_git2_oid);
        let index = submodule.index_id().ok().flatten().map(gix_to_git2_oid);
        let workdir_id = opened.as_ref().and_then(|repo| repo.head_id().ok().map(|id| gix_to_git2_oid(id.detach())));
        let absolute_path = workdir.join(&path);

        let (has_modified_content, has_untracked_content) = status.as_ref().and_then(|status| status.changes.as_ref().map(|changes| changed_content_flags(changes))).unwrap_or((false, false));
        let is_index_modified = head != index;
        let has_new_commits = workdir_id.zip(index).is_some_and(|(workdir, index)| workdir != index);
        let is_workdir_modified = has_new_commits || has_modified_content || has_untracked_content;

        entries.push(SubmoduleEntry {
            name,
            path,
            absolute_path,
            url: submodule.url().ok().map(|url| url.to_string()),
            branch,
            head,
            index,
            workdir: workdir_id,
            is_open: opened.is_some(),
            is_uninitialized: !state.repository_exists,
            is_in_head: head.is_some(),
            is_in_index: index.is_some(),
            is_in_config: state.superproject_configuration,
            is_in_workdir: state.worktree_checkout,
            is_index_modified,
            is_workdir_modified,
            has_new_commits,
            has_modified_content,
            has_untracked_content,
        });
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

#[cfg(test)]
#[path = "../../tests/git/queries/submodules.rs"]
mod tests;
