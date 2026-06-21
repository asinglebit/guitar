use crate::core::{oids::gix_to_git2_oid, submodules::SubmoduleEntry};
use git2::Repository;
use gix::bstr::ByteSlice;
use std::path::{Path, PathBuf};

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

pub fn has_submodule_metadata(repo: &Repository) -> bool {
    let gitmodules = Path::new(".gitmodules");
    repo.workdir().is_some_and(|workdir| workdir.join(gitmodules).exists())
        || repo.head().ok().and_then(|head| head.peel_to_tree().ok()).is_some_and(|tree| tree.get_path(gitmodules).is_ok())
        || repo.index().ok().is_some_and(|index| index.get_path(gitmodules, 0).is_some())
}

pub fn submodules_if_present(repo: &Repository) -> Result<Vec<git2::Submodule<'_>>, git2::Error> {
    if !has_submodule_metadata(repo) {
        return Ok(Vec::new());
    }

    repo.submodules()
}

pub fn list_submodules(repo: &Repository) -> Result<Vec<SubmoduleEntry>, git2::Error> {
    if !has_submodule_metadata(repo) {
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
