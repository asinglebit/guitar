pub const HISTORY_OBJECT_CACHE_BYTES: usize = 64 * 1024 * 1024;

pub fn enable_history_object_cache(repo: &mut gix::Repository) {
    repo.object_cache_size_if_unset(HISTORY_OBJECT_CACHE_BYTES);
}

pub fn commit_graph_if_available(repo: &gix::Repository) -> Option<gix::commitgraph::Graph> {
    repo.commit_graph_if_enabled().ok().flatten()
}
