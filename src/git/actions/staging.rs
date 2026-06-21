use crate::git::queries::submodules::submodules_if_present;
use git2::{Error, Repository, ResetType, StatusOptions, Submodule, SubmoduleIgnore, SubmoduleStatus};
use std::path::{Path, PathBuf};

pub fn stage_all(repo: &Repository) -> Result<(), Error> {
    let mut index = repo.index()?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true).include_ignored(false).include_unmodified(false).exclude_submodules(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let submodules = submodules_if_present(repo).unwrap_or_default();
    let submodule_paths = submodules.iter().map(|entry| entry.path().to_path_buf()).collect::<Vec<_>>();

    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            if is_submodule_status_path(path, &submodule_paths) {
                continue;
            }

            let path = Path::new(path);

            match entry.status() {
                s if s.is_wt_deleted() || s.is_index_deleted() => {
                    // A deleted tracked file is staged by removing its index entry.
                    if index.get_path(path, 0).is_some() {
                        index.remove_path(path)?;
                    }
                },
                _ => {
                    // New and modified files both enter the index through add_path.
                    index.add_path(path)?;
                },
            }
        }
    }

    index.write()?;
    drop(index);

    for submodule in submodules {
        stage_submodule_pointer_change(repo, submodule)?;
    }

    Ok(())
}

fn stage_submodule_pointer_change(repo: &Repository, mut submodule: Submodule<'_>) -> Result<(), Error> {
    let path = submodule.path().to_path_buf();
    let path_text = path.to_string_lossy().to_string();
    let name = submodule.name().unwrap_or(path_text.as_str());
    let status = submodule_status_for(repo, name, path.as_path());
    let has_pointer_change =
        status.is_wd_added() || status.is_wd_deleted() || status.is_wd_modified() || submodule.workdir_id().zip(submodule.index_id()).is_some_and(|(workdir, index)| workdir != index);

    if !has_pointer_change {
        return Ok(());
    }

    if status.is_wd_deleted() {
        let mut index = repo.index()?;
        if index.get_path(path.as_path(), 0).is_some() {
            index.remove_path(path.as_path())?;
            index.write()?;
        }
        return Ok(());
    }

    submodule.add_to_index(true)
}

fn submodule_status_for(repo: &Repository, name: &str, path: &Path) -> SubmoduleStatus {
    repo.submodule_status(name, SubmoduleIgnore::None)
        .or_else(|_| path.to_str().map(|path| repo.submodule_status(path, SubmoduleIgnore::None)).unwrap_or_else(|| Err(Error::from_str("invalid submodule path"))))
        .unwrap_or_else(|_| SubmoduleStatus::empty())
}

fn is_submodule_status_path(path: &str, submodule_paths: &[PathBuf]) -> bool {
    if path.is_empty() {
        return false;
    }

    let normalized = path.trim_end_matches('/');
    let path = Path::new(normalized);
    submodule_paths.iter().any(|submodule_path| path == submodule_path || path.starts_with(submodule_path))
}

pub fn unstage_all(repo: &Repository) -> Result<(), git2::Error> {
    let head = match repo.head() {
        Ok(head) => head.peel_to_commit()?,
        Err(_) => {
            // A fresh repository has no HEAD to reset back to.
            return Ok(());
        },
    };

    // Mixed reset keeps workdir changes while returning the whole index to HEAD.
    repo.reset(&head.into_object(), ResetType::Mixed, None)?;

    Ok(())
}

pub fn stage_file(repo: &Repository, path: &Path) -> Result<(), git2::Error> {
    let mut index = repo.index()?;

    if path.exists() {
        // Existing paths represent new or modified files.
        index.add_path(path)?;
    } else {
        // Missing paths represent deletes, which are staged by removing index entries.
        index.remove_path(path)?;
    }

    index.write()?;
    Ok(())
}

pub fn unstage_file(repo: &Repository, path: &Path) -> Result<(), git2::Error> {
    let head = match repo.head() {
        Ok(h) => h.peel_to_commit()?,
        Err(_) => {
            // Without HEAD, unstage means remove the path from the initial index.
            let mut index = repo.index()?;
            index.remove_path(path)?;
            index.write()?;
            return Ok(());
        },
    };

    // reset_default updates the index pathspec without touching working tree contents.
    repo.reset_default(Some(&head.into_object()), [path])?;
    Ok(())
}
