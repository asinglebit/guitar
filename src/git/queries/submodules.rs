use crate::{core::submodules::SubmoduleEntry, git::queries::commits::get_current_branch};
use git2::{Repository, Submodule, SubmoduleIgnore, SubmoduleStatus};
use std::path::{Path, PathBuf};

fn status_for(repo: &Repository, name: &str, path: &Path) -> SubmoduleStatus {
    repo.submodule_status(name, SubmoduleIgnore::None)
        .or_else(|_| path.to_str().map(|path| repo.submodule_status(path, SubmoduleIgnore::None)).unwrap_or_else(|| Err(git2::Error::from_str("invalid submodule path"))))
        .unwrap_or_else(|_| SubmoduleStatus::empty())
}

pub fn has_submodule_metadata(repo: &Repository) -> bool {
    let gitmodules = Path::new(".gitmodules");
    repo.workdir().is_some_and(|workdir| workdir.join(gitmodules).exists())
        || repo.head().ok().and_then(|head| head.peel_to_tree().ok()).is_some_and(|tree| tree.get_path(gitmodules).is_ok())
        || repo.index().ok().is_some_and(|index| index.get_path(gitmodules, 0).is_some())
}

pub fn submodules_if_present(repo: &Repository) -> Result<Vec<Submodule<'_>>, git2::Error> {
    if !has_submodule_metadata(repo) {
        return Ok(Vec::new());
    }

    repo.submodules()
}

pub fn list_submodules(repo: &Repository) -> Result<Vec<SubmoduleEntry>, git2::Error> {
    let workdir = repo.workdir().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let mut entries = Vec::new();

    for submodule in submodules_if_present(repo)? {
        let path = submodule.path().to_path_buf();
        let name = submodule.name().map(str::to_string).unwrap_or_else(|| path.display().to_string());
        let status = status_for(repo, &name, &path);
        let is_initialized_in_workdir = status.is_in_wd() && !status.is_wd_uninitialized();
        let sub_repo = is_initialized_in_workdir.then(|| submodule.open().ok()).flatten();
        let branch = sub_repo.as_ref().and_then(get_current_branch).or_else(|| submodule.branch().map(str::to_string));
        let absolute_path = workdir.join(&path);

        let is_index_modified = status.is_index_added() || status.is_index_deleted() || status.is_index_modified();
        let has_new_commits = status.is_wd_modified() || submodule.workdir_id().zip(submodule.index_id()).is_some_and(|(workdir, index)| workdir != index);
        let has_modified_content = status.contains(SubmoduleStatus::WD_INDEX_MODIFIED) || status.is_wd_wd_modified();
        let has_untracked_content = status.is_wd_untracked();
        let is_workdir_modified =
            status.is_wd_added() || status.is_wd_deleted() || status.is_wd_modified() || status.contains(SubmoduleStatus::WD_INDEX_MODIFIED) || status.is_wd_wd_modified() || status.is_wd_untracked();

        entries.push(SubmoduleEntry {
            name,
            path,
            absolute_path,
            url: submodule.url().map(str::to_string),
            branch,
            head: submodule.head_id(),
            index: submodule.index_id(),
            workdir: submodule.workdir_id(),
            is_open: sub_repo.is_some(),
            is_uninitialized: status.is_wd_uninitialized(),
            is_in_head: status.is_in_head(),
            is_in_index: status.is_in_index(),
            is_in_config: status.is_in_config(),
            is_in_workdir: status.is_in_wd(),
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
