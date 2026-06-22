pub const HISTORY_OBJECT_CACHE_BYTES: usize = 64 * 1024 * 1024;

pub fn enable_history_object_cache(repo: &mut gix::Repository) {
    repo.object_cache_size_if_unset(HISTORY_OBJECT_CACHE_BYTES);
}

pub fn commit_graph_if_available(repo: &gix::Repository) -> Option<gix::commitgraph::Graph> {
    repo.commit_graph_if_enabled().ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::{fs, path::Path, process::Command};

    fn commit_file(repo: &Repository, path: &str, contents: &str, message: &str) {
        let workdir = repo.workdir().unwrap();
        fs::write(workdir.join(path), contents).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(path)).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents = parent.iter().collect::<Vec<_>>();

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap();
    }

    #[test]
    fn commit_graph_helper_uses_written_commit_graph() {
        let temp = tempfile::Builder::new().prefix("guitar-gix-commit-graph-").tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        commit_file(&repo, "one.txt", "one\n", "one");
        commit_file(&repo, "two.txt", "two\n", "two");

        let status = Command::new("git").arg("-C").arg(temp.path()).args(["commit-graph", "write", "--reachable"]).status().unwrap();
        assert!(status.success());

        let gix_repo = gix::open(temp.path()).unwrap();
        assert!(commit_graph_if_available(&gix_repo).is_some());
    }
}
