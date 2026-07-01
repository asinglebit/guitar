use crate::helpers::branch_visibility::branch_name_from_ref;

pub const HISTORY_OBJECT_CACHE_BYTES: usize = 64 * 1024 * 1024;

pub fn enable_history_object_cache(repo: &mut gix::Repository) {
    repo.object_cache_size_if_unset(HISTORY_OBJECT_CACHE_BYTES);
}

pub fn commit_graph_if_available(repo: &gix::Repository) -> Option<gix::commitgraph::Graph> {
    repo.commit_graph_if_enabled().ok().flatten()
}

pub fn history_commit_count_hint(repo: &gix::Repository) -> Option<usize> {
    commit_graph_if_available(repo).map(|graph| graph.num_commits() as usize)
}

pub fn for_each_branch_tip(repo: &gix::Repository, mut visit: impl FnMut(bool, &str, gix::ObjectId)) -> Result<(), git2::Error> {
    let references = repo.references().map_err(gix_error)?;
    for (is_local, references) in [(true, references.local_branches()), (false, references.remote_branches())] {
        for reference in references.map_err(gix_error)? {
            let reference = reference.map_err(gix_error)?;
            let Some(name) = branch_name_from_ref(reference.name().as_bstr()) else {
                continue;
            };
            let Some(oid) = reference.try_id().map(|id| id.detach()) else {
                continue;
            };
            visit(is_local, name, oid);
        }
    }
    Ok(())
}

pub(crate) fn gix_error(error: impl std::fmt::Display) -> git2::Error {
    git2::Error::from_str(&error.to_string())
}
