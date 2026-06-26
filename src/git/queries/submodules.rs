use crate::{core::submodules::SubmoduleEntry, git::gix::gix_error};
use git2::Repository;
use gix::bstr::ByteSlice;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

const GITMODULES_PATH: &[u8] = b".gitmodules";
const INDEX_SCAN_BUFFER: usize = 64 * 1024;
const GITMODULES_OVERLAP: usize = 10;

fn open_repo(repo: &Repository) -> Result<gix::Repository, git2::Error> {
    let path = repo.workdir().unwrap_or(repo.path());
    let options = if repo.workdir().is_some() { gix::open::Options::default() } else { gix::open::Options::default().open_path_as_is(true) };
    gix::open_opts(path, options).map_err(gix_error)
}

fn open_repo_path(path: &Path) -> Result<gix::Repository, git2::Error> {
    gix::open(path).or_else(|_| gix::open_opts(path, gix::open::Options::default().open_path_as_is(true))).map_err(gix_error)
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
    changes.iter().fold((false, false), |(modified, untracked), change| match change {
        gix::status::Item::TreeIndex(_) => (true, untracked),
        gix::status::Item::IndexWorktree(gix::status::index_worktree::Item::DirectoryContents { entry, .. }) => {
            let is_untracked = matches!(entry.status, gix::dir::entry::Status::Untracked);
            (modified || !is_untracked, untracked || is_untracked)
        },
        gix::status::Item::IndexWorktree(gix::status::index_worktree::Item::Rewrite { .. }) => (true, untracked),
        gix::status::Item::IndexWorktree(gix::status::index_worktree::Item::Modification { status, .. }) => {
            let is_modified = matches!(
                status,
                gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                    gix::status::plumbing::index_as_worktree::Change::Removed
                        | gix::status::plumbing::index_as_worktree::Change::Type { .. }
                        | gix::status::plumbing::index_as_worktree::Change::Modification { .. }
                        | gix::status::plumbing::index_as_worktree::Change::SubmoduleModification(_),
                ) | gix::status::plumbing::index_as_worktree::EntryStatus::Conflict { .. }
                    | gix::status::plumbing::index_as_worktree::EntryStatus::IntentToAdd
            );
            (modified || is_modified, untracked)
        },
    })
}

fn head_contains_gitmodules(gix_repo: &gix::Repository, gitmodules: &Path) -> bool {
    gix_repo.head_tree_id_or_empty().ok().and_then(|tree_id| gix_repo.find_tree(tree_id).ok()).and_then(|tree| tree.lookup_entry_by_path(gitmodules).ok().flatten()).is_some()
}

fn has_committed_or_workdir_submodule_metadata(repo: &Repository) -> bool {
    let gitmodules = Path::new(".gitmodules");
    let workdir_has_gitmodules = repo.workdir().is_some_and(|workdir| workdir.join(gitmodules).exists());
    let head_has_gitmodules = open_repo(repo).ok().is_some_and(|gix_repo| head_contains_gitmodules(&gix_repo, gitmodules));

    workdir_has_gitmodules || head_has_gitmodules
}

fn has_committed_or_workdir_submodule_metadata_from_path(path: &Path) -> bool {
    let gitmodules = Path::new(".gitmodules");
    let workdir_has_gitmodules = path.join(gitmodules).exists();
    let head_has_gitmodules = open_repo_path(path).ok().is_some_and(|gix_repo| head_contains_gitmodules(&gix_repo, gitmodules));

    workdir_has_gitmodules || head_has_gitmodules
}

pub fn has_submodule_metadata(repo: &Repository) -> bool {
    has_committed_or_workdir_submodule_metadata(repo) || index_contains_gitmodules_path(repo)
}

fn index_contains_gitmodules_path(repo: &Repository) -> bool {
    let path = repo.path().join("index");
    file_contains_gitmodules_path(&path)
}

fn file_contains_gitmodules_path(path: &Path) -> bool {
    File::open(path).ok().is_some_and(|mut file| {
        let mut buffer = [0u8; INDEX_SCAN_BUFFER];
        let mut overlap = [0u8; GITMODULES_OVERLAP];
        let mut overlap_len = 0;

        loop {
            let Ok(read) = file.read(&mut buffer) else {
                break false;
            };
            if read == 0 {
                break false;
            }

            let boundary_match = if overlap_len == 0 {
                false
            } else {
                let prefix_len = (GITMODULES_PATH.len() - 1).min(read);
                let mut boundary = [0u8; GITMODULES_OVERLAP * 2 + 1];
                boundary[..overlap_len].copy_from_slice(&overlap[..overlap_len]);
                boundary[overlap_len..overlap_len + prefix_len].copy_from_slice(&buffer[..prefix_len]);
                boundary[..overlap_len + prefix_len].windows(GITMODULES_PATH.len()).any(|window| window == GITMODULES_PATH)
            };

            if boundary_match || buffer[..read].windows(GITMODULES_PATH.len()).any(|window| window == GITMODULES_PATH) {
                break true;
            }

            let keep = GITMODULES_PATH.len().saturating_sub(1).min(read);
            overlap[..keep].copy_from_slice(&buffer[read - keep..read]);
            overlap_len = keep;
        }
    })
}

pub fn list_submodules(repo: &Repository) -> Result<Vec<SubmoduleEntry>, git2::Error> {
    if !has_committed_or_workdir_submodule_metadata(repo) {
        return Ok(Vec::new());
    }

    let gix_repo = open_repo(repo)?;
    let workdir = repo.workdir().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    list_submodules_from_gix_repo(&gix_repo, workdir)
}

pub fn list_submodules_from_path(path: impl AsRef<Path>) -> Result<Vec<SubmoduleEntry>, git2::Error> {
    let path = path.as_ref();
    if !has_committed_or_workdir_submodule_metadata_from_path(path) {
        return Ok(Vec::new());
    }

    let gix_repo = open_repo_path(path)?;
    list_submodules_from_gix_repo(&gix_repo, path.to_path_buf())
}

fn list_submodules_from_gix_repo(gix_repo: &gix::Repository, workdir: PathBuf) -> Result<Vec<SubmoduleEntry>, git2::Error> {
    let mut entries = gix_repo
        .submodules()
        .map_err(gix_error)?
        .into_iter()
        .flatten()
        .filter_map(|submodule| {
            let path = gix::path::from_bstring(submodule.path().ok()?.into_owned());
            let name = submodule.name().to_str().ok().map(str::to_string).unwrap_or_else(|| path.display().to_string());
            let opened = submodule.open().ok().flatten();
            let status = submodule.status(gix::submodule::config::Ignore::None, false).ok();
            let state = status.as_ref().map(|status| status.state).unwrap_or_else(|| submodule.state().unwrap_or_default());
            let branch = opened.as_ref().and_then(current_branch).or_else(|| configured_branch(submodule.branch().ok().flatten()));
            let head = submodule.head_id().ok().flatten();
            let index = submodule.index_id().ok().flatten();
            let workdir_id = opened.as_ref().and_then(|repo| repo.head_id().ok().map(|id| id.detach()));
            let absolute_path = workdir.join(&path);

            let (has_modified_content, has_untracked_content) = status.as_ref().and_then(|status| status.changes.as_ref().map(|changes| changed_content_flags(changes))).unwrap_or((false, false));
            let is_index_modified = head != index;
            let has_new_commits = workdir_id.zip(index).is_some_and(|(workdir, index)| workdir != index);
            let is_workdir_modified = has_new_commits || has_modified_content || has_untracked_content;

            Some(SubmoduleEntry {
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
            })
        })
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

#[cfg(test)]
#[path = "../../tests/git/queries/submodules.rs"]
mod tests;
