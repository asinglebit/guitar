use git2::{Error, Repository, StatusOptions};
use std::fs;
use std::path::{Path, PathBuf};

fn conflict_path(entry: &git2::IndexEntry) -> Option<PathBuf> {
    std::str::from_utf8(&entry.path).ok().map(PathBuf::from)
}

pub fn ensure_clean_workdir(repo: &Repository, operation: &str) -> Result<(), Error> {
    let index = repo.index()?;
    if index.has_conflicts() {
        return Err(Error::from_str("repository has unresolved conflicts"));
    }

    let mut options = StatusOptions::new();
    options.include_untracked(true).show(git2::StatusShow::IndexAndWorkdir).renames_head_to_index(false).renames_index_to_workdir(false);
    let statuses = repo.statuses(Some(&mut options))?;
    if statuses.is_empty() { Ok(()) } else { Err(Error::from_str(&format!("working tree must be clean before {operation}"))) }
}

fn collect_conflict_paths(repo: &Repository) -> Result<Vec<PathBuf>, Error> {
    let index = repo.index()?;
    let mut paths = Vec::new();

    for conflict in index.conflicts()? {
        let conflict = conflict?;
        let path = conflict.our.as_ref().and_then(conflict_path).or_else(|| conflict.their.as_ref().and_then(conflict_path)).or_else(|| conflict.ancestor.as_ref().and_then(conflict_path));
        if let Some(path) = path
            && !paths.iter().any(|existing| existing == &path)
        {
            paths.push(path);
        }
    }

    Ok(paths)
}

pub fn mark_conflicts_resolved_from_workdir(repo: &Repository) -> Result<(), Error> {
    let workdir = repo.workdir().ok_or_else(|| Error::from_str("bare repositories are not supported"))?.to_path_buf();
    let paths = collect_conflict_paths(repo)?;
    let mut index = repo.index()?;

    for path in paths {
        let full_path = workdir.join(&path);
        if full_path.exists() {
            if has_conflict_markers(&full_path) {
                continue;
            }
            index.add_path(Path::new(&path))?;
        } else {
            index.remove_path(Path::new(&path))?;
        }
    }

    index.write()?;
    Ok(())
}

fn has_conflict_markers(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|content| {
            content.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("<<<<<<<") || line.starts_with("=======") || line.starts_with(">>>>>>>")
            })
        })
        .unwrap_or(false)
}
