use std::path::{Path, PathBuf};

pub fn try_into_git_repo_root(start_path: impl AsRef<Path>) -> Option<PathBuf> {
    let mut current_path = start_path.as_ref();

    while let Some(parent) = current_path.parent() {
        let git_path = parent.join(".git");
        if git_path.exists() && git_path.is_dir() {
            return Some(parent.to_path_buf());
        }
        current_path = parent;
    }

    None
}
