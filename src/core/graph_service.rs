use crate::{
    core::{
        chunk::{Chunk, LaneRef, NONE},
        reflogs::HeadReflogAliasEntry,
        walker::Walker,
        worktrees::{WorktreeEntry, Worktrees},
    },
    git::queries::{file_history::changed_file_status_at_commit, helpers::FileStatus, reflogs::HeadReflogEntry},
    helpers::{
        heatmap::{DAYS, WEEKS, build_heatmap},
        localisation::{common, empty, errors, status as status_text},
        symbols::SymbolTheme,
        time::timestamp_to_utc_date_time,
    },
};
use git2::Oid;
use im::{HashSet, Vector};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
    },
    thread,
    time::Duration,
};

pub type RequestId = u64;
pub type Generation = u64;
pub type GraphVersion = u64;
pub type GraphHistory = Vector<Vector<Chunk>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphPane {
    Branches,
    Tags,
    Stashes,
    Reflogs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphBranchJumpDirection {
    Previous,
    Next,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphLookupKind {
    GraphRowAt { index: usize },
    PaneRowAt { pane: GraphPane, index: usize },
    BranchIndex { from: usize, direction: GraphBranchJumpDirection },
    ShaPrefix { prefix: String },
    Oid { oid: Oid },
    ParentIndex { index: usize },
    ChildIndex { index: usize },
}

#[derive(Clone, Debug)]
pub enum GraphCommand {
    QueryGraphWindow { generation: Generation, request_id: RequestId, start: usize, end: usize },
    QueryPaneWindow { generation: Generation, pane: GraphPane, start: usize, end: usize },
    QueryFileHistory { generation: Generation, request_id: RequestId, path: String },
    Lookup { generation: Generation, request_id: RequestId, kind: GraphLookupKind },
    Shutdown,
}

#[derive(Clone, Debug)]
pub struct GraphBranchLabel {
    pub name: String,
    pub is_local: bool,
    pub lane: Option<LaneRef>,
}

#[derive(Clone, Debug)]
pub struct GraphTagLabel {
    pub name: String,
    pub lane: Option<LaneRef>,
}

#[derive(Clone, Debug)]
pub struct GraphReflogLabel {
    pub selector: String,
    pub message: String,
    pub lane: Option<LaneRef>,
}

#[derive(Clone, Debug)]
pub struct GraphRow {
    pub index: usize,
    pub alias: u32,
    pub oid: Oid,
    pub summary: String,
    pub committer_date: String,
    pub committer_name: String,
    pub has_any_branch: bool,
    pub branches: Vec<GraphBranchLabel>,
    pub tags: Vec<GraphTagLabel>,
    pub is_stash: bool,
    pub stash_lane: Option<LaneRef>,
    pub worktrees: Vec<WorktreeEntry>,
    pub reflog: Option<GraphReflogLabel>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CommitMetadata {
    summary: String,
    committer_date: String,
    committer_name: String,
}

#[derive(Debug, Default)]
struct CommitMetadataCache {
    entries: HashMap<u32, CommitMetadata>,
}

impl CommitMetadataCache {
    fn get_or_insert_with(&mut self, alias: u32, load: impl FnOnce() -> CommitMetadata) -> CommitMetadata {
        if let Some(metadata) = self.entries.get(&alias).cloned() {
            return metadata;
        }

        let metadata = load();
        self.entries.insert(alias, metadata.clone());
        metadata
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Clone, Debug)]
pub enum GraphPaneRow {
    Branch { alias: u32, name: String, is_local: bool, lane: Option<LaneRef>, graph_index: Option<usize> },
    Tag { alias: u32, name: String, lane: Option<LaneRef>, graph_index: Option<usize> },
    Stash { alias: u32, summary: String, lane: Option<LaneRef>, graph_index: Option<usize> },
    Reflog { alias: u32, selector: String, message: String, lane: Option<LaneRef>, graph_index: Option<usize> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphFileHistoryRow {
    pub graph_index: usize,
    pub oid: Oid,
    pub short_oid: String,
    pub summary: String,
    pub status: FileStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphIndexIdentity {
    pub index: usize,
    pub alias: u32,
    pub oid: Oid,
}

#[derive(Clone, Debug)]
pub enum GraphLookupResult {
    GraphRow(Option<GraphRow>),
    Index(Option<usize>),
    PaneRow(Option<GraphPaneRow>),
}

#[derive(Clone, Debug)]
pub enum GraphEvent {
    Progress { generation: Generation, version: GraphVersion, total: usize, is_first: bool, is_complete: bool },
    GraphWindow { generation: Generation, request_id: RequestId, version: GraphVersion, start: usize, end: usize, total: usize, head_alias: u32, rows: Vec<GraphRow>, history: GraphHistory },
    PaneWindow { generation: Generation, version: GraphVersion, pane: GraphPane, start: usize, end: usize, total: usize, rows: Vec<GraphPaneRow> },
    FileHistory { generation: Generation, request_id: RequestId, path: String, rows: Vec<GraphFileHistoryRow>, error: Option<String> },
    LookupResult { generation: Generation, request_id: RequestId, result: GraphLookupResult },
    Heatmap { generation: Generation, heatmap: [[usize; WEEKS]; DAYS] },
    Error { generation: Generation, message: String },
}

pub struct GraphServiceConfig {
    pub generation: Generation,
    pub path: String,
    pub amount: usize,
    pub hidden_branch_names: HashSet<String>,
    pub include_head_reflog_roots: bool,
    pub graph_lane_limit: usize,
    pub worktrees: Vec<WorktreeEntry>,
    pub symbols: SymbolTheme,
}

pub fn spawn_graph_service(config: GraphServiceConfig, rx: Receiver<GraphCommand>, tx: Sender<GraphEvent>, cancel: Arc<AtomicBool>) -> thread::JoinHandle<()> {
    thread::spawn(move || run_graph_service(config, rx, tx, cancel))
}

fn run_graph_service(config: GraphServiceConfig, rx: Receiver<GraphCommand>, tx: Sender<GraphEvent>, cancel: Arc<AtomicBool>) {
    let generation = config.generation;
    let mut walk_ctx = match Walker::new(config.path, config.amount, config.hidden_branch_names.clone(), config.include_head_reflog_roots, config.graph_lane_limit) {
        Ok(walker) => walker,
        Err(error) => {
            let _ = tx.send(GraphEvent::Error { generation, message: errors::walker_failed(error) });
            return;
        },
    };

    let mut worktrees = Worktrees::from_entries(config.worktrees);
    let mut version: GraphVersion = 0;
    let mut is_first = true;
    let mut is_complete = false;
    let mut pending_graph: Option<(RequestId, usize, usize)> = None;
    let mut pending_file_history: Option<(RequestId, String)> = None;
    let mut commit_metadata = CommitMetadataCache::default();

    loop {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        if !drain_commands(generation, version, &rx, &tx, &walk_ctx, &mut worktrees, &mut pending_graph, &mut pending_file_history, &mut commit_metadata, &config.hidden_branch_names, &config.symbols)
        {
            break;
        }

        if let Some((request_id, start, end)) = pending_graph.take() {
            send_graph_window(generation, request_id, version, start, end, &tx, &walk_ctx, &worktrees, &mut commit_metadata, &config.hidden_branch_names, &config.symbols);
        }

        if is_complete && let Some((request_id, path)) = pending_file_history.take() {
            send_file_history(generation, request_id, path, &tx, &walk_ctx, &config.symbols);
        }

        if is_complete {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(GraphCommand::Shutdown) => break,
                Ok(command) => {
                    if !handle_command(
                        generation,
                        version,
                        command,
                        &tx,
                        &walk_ctx,
                        &mut worktrees,
                        &mut pending_graph,
                        &mut pending_file_history,
                        &mut commit_metadata,
                        &config.hidden_branch_names,
                        &config.symbols,
                    ) {
                        break;
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {},
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
            continue;
        }

        let is_again = walk_ctx.walk();
        version = version.saturating_add(1);
        is_complete = !is_again;
        let total = walk_ctx.oids.get_commit_count();

        let _ = tx.send(GraphEvent::Progress { generation, version, total, is_first, is_complete });
        is_first = false;

        if is_complete {
            let repo = walk_ctx.repo.borrow();
            let heatmap = build_heatmap(&repo, &walk_ctx.oids.oids);
            let _ = tx.send(GraphEvent::Heatmap { generation, heatmap });

            if let Some((request_id, path)) = pending_file_history.take() {
                send_file_history(generation, request_id, path, &tx, &walk_ctx, &config.symbols);
            }
        }
    }
}

fn drain_commands(
    generation: Generation, version: GraphVersion, rx: &Receiver<GraphCommand>, tx: &Sender<GraphEvent>, walk_ctx: &Walker, worktrees: &mut Worktrees,
    pending_graph: &mut Option<(RequestId, usize, usize)>, pending_file_history: &mut Option<(RequestId, String)>, commit_metadata: &mut CommitMetadataCache, hidden_branch_names: &HashSet<String>,
    symbols: &SymbolTheme,
) -> bool {
    while let Ok(command) = rx.try_recv() {
        if !handle_command(generation, version, command, tx, walk_ctx, worktrees, pending_graph, pending_file_history, commit_metadata, hidden_branch_names, symbols) {
            return false;
        }
    }
    true
}

fn handle_command(
    generation: Generation, version: GraphVersion, command: GraphCommand, tx: &Sender<GraphEvent>, walk_ctx: &Walker, worktrees: &mut Worktrees, pending_graph: &mut Option<(RequestId, usize, usize)>,
    pending_file_history: &mut Option<(RequestId, String)>, commit_metadata: &mut CommitMetadataCache, hidden_branch_names: &HashSet<String>, symbols: &SymbolTheme,
) -> bool {
    match command {
        GraphCommand::Shutdown => false,
        GraphCommand::QueryGraphWindow { generation: cmd_generation, request_id, start, end } => {
            if cmd_generation == generation {
                *pending_graph = Some((request_id, start, end));
            }
            true
        },
        GraphCommand::QueryPaneWindow { generation: cmd_generation, pane, start, end } => {
            if cmd_generation == generation {
                send_pane_window(generation, version, pane, start, end, tx, walk_ctx);
            }
            true
        },
        GraphCommand::QueryFileHistory { generation: cmd_generation, request_id, path } => {
            if cmd_generation == generation {
                *pending_file_history = Some((request_id, path));
            }
            true
        },
        GraphCommand::Lookup { generation: cmd_generation, request_id, kind } => {
            if cmd_generation == generation {
                let result = lookup(kind, walk_ctx, worktrees, commit_metadata, hidden_branch_names, symbols);
                let _ = tx.send(GraphEvent::LookupResult { generation, request_id, result });
            }
            true
        },
    }
}

fn send_graph_window(
    generation: Generation, request_id: RequestId, version: GraphVersion, start: usize, end: usize, tx: &Sender<GraphEvent>, walk_ctx: &Walker, worktrees: &Worktrees,
    commit_metadata: &mut CommitMetadataCache, hidden_branch_names: &HashSet<String>, symbols: &SymbolTheme,
) {
    let total = walk_ctx.oids.get_commit_count();
    let start = start.min(total);
    let end = end.min(total);
    let history = walk_ctx.buffer.borrow().window(start, end.saturating_add(1));
    let rows = graph_rows(walk_ctx, worktrees, commit_metadata, hidden_branch_names, symbols, start, end);
    let head_alias = head_alias(walk_ctx);

    let _ = tx.send(GraphEvent::GraphWindow { generation, request_id, version, start, end, total, head_alias, rows, history });
}

fn send_pane_window(generation: Generation, version: GraphVersion, pane: GraphPane, start: usize, end: usize, tx: &Sender<GraphEvent>, walk_ctx: &Walker) {
    let all_rows = pane_rows(pane, walk_ctx);
    let total = all_rows.len();
    let start = start.min(total);
    let end = end.min(total);
    let rows = all_rows.into_iter().skip(start).take(end.saturating_sub(start)).collect();

    let _ = tx.send(GraphEvent::PaneWindow { generation, version, pane, start, end, total, rows });
}

fn send_file_history(generation: Generation, request_id: RequestId, path: String, tx: &Sender<GraphEvent>, walk_ctx: &Walker, symbols: &SymbolTheme) {
    let result = file_history_rows(walk_ctx, &path, symbols);
    match result {
        Ok(rows) => {
            let _ = tx.send(GraphEvent::FileHistory { generation, request_id, path, rows, error: None });
        },
        Err(error) => {
            let _ = tx.send(GraphEvent::FileHistory { generation, request_id, path, rows: Vec::new(), error: Some(error.to_string()) });
        },
    }
}

fn file_history_rows(walk_ctx: &Walker, path: &str, symbols: &SymbolTheme) -> Result<Vec<GraphFileHistoryRow>, git2::Error> {
    let repo = walk_ctx.repo.borrow();
    let mut rows = Vec::new();

    for (graph_index, &alias) in walk_ctx.oids.get_sorted_aliases().iter().enumerate() {
        let oid = *walk_ctx.oids.get_oid_by_alias(alias);
        if walk_ctx.oids.is_zero(&oid) {
            continue;
        }

        let Some(status) = changed_file_status_at_commit(&repo, oid, path)? else {
            continue;
        };

        let summary = repo.find_commit(oid).ok().and_then(|commit| commit.summary().map(str::to_string)).unwrap_or_else(|| no_message(symbols));
        let short_oid = oid.to_string().chars().take(8).collect();
        rows.push(GraphFileHistoryRow { graph_index, oid, short_oid, summary, status });
    }

    Ok(rows)
}

fn no_message(symbols: &SymbolTheme) -> String {
    format!("{} {}", symbols.empty_state.mark, empty::NO_MESSAGE())
}

fn graph_rows(
    walk_ctx: &Walker, worktrees: &Worktrees, commit_metadata: &mut CommitMetadataCache, hidden_branch_names: &HashSet<String>, symbols: &SymbolTheme, start: usize, end: usize,
) -> Vec<GraphRow> {
    let latest_reflogs = latest_reflogs_by_alias(walk_ctx);
    let mut rows = Vec::with_capacity(end.saturating_sub(start));

    for index in start..end {
        let alias = walk_ctx.oids.get_sorted_aliases().get(index).copied().unwrap_or(NONE);
        let oid = *walk_ctx.oids.get_oid_by_alias(alias);
        let is_uncommitted = alias == NONE || walk_ctx.oids.is_zero(&oid);
        let metadata = if is_uncommitted { CommitMetadata::default() } else { load_commit_metadata(walk_ctx, commit_metadata, alias, oid, symbols) };

        let local = walk_ctx.branches_local.get(&alias).cloned().unwrap_or_default();
        let remote = walk_ctx.branches_remote.get(&alias).cloned().unwrap_or_default();
        let has_any_branch = !local.is_empty() || !remote.is_empty();
        let branch_lane = walk_ctx.branches_lanes.get(&alias).copied();
        let branches = local
            .into_iter()
            .map(|name| (name, true))
            .chain(remote.into_iter().map(|name| (name, false)))
            .filter(|(name, _)| !hidden_branch_names.contains(name))
            .map(|(name, is_local)| GraphBranchLabel { name, is_local, lane: branch_lane })
            .collect();

        let tag_lane = walk_ctx.tags_lanes.get(&alias).copied();
        let tags = walk_ctx.tags_local.get(&alias).cloned().unwrap_or_default().into_iter().map(|name| GraphTagLabel { name, lane: tag_lane }).collect();

        let is_stash = walk_ctx.oids.stashes.contains(&alias);
        let stash_lane = walk_ctx.stashes_lanes.get(&alias).copied();
        let worktrees = worktrees_for_alias(worktrees, walk_ctx, alias);
        let reflog = latest_reflogs.get(&alias).map(|entry| GraphReflogLabel { selector: entry.selector.clone(), message: entry.message.clone(), lane: walk_ctx.reflogs_lanes.get(&alias).copied() });

        rows.push(GraphRow {
            index,
            alias,
            oid,
            summary: metadata.summary,
            committer_date: metadata.committer_date,
            committer_name: metadata.committer_name,
            has_any_branch,
            branches,
            tags,
            is_stash,
            stash_lane,
            worktrees,
            reflog,
        });
    }

    rows
}

fn load_commit_metadata(walk_ctx: &Walker, cache: &mut CommitMetadataCache, alias: u32, oid: Oid, symbols: &SymbolTheme) -> CommitMetadata {
    cache.get_or_insert_with(alias, || {
        if let Ok(commit) = walk_ctx.repo.borrow().find_commit(oid) {
            let summary = commit.summary().map(str::to_string).unwrap_or_else(|| no_message(symbols));
            let committer = commit.committer();
            let committer_date = timestamp_to_utc_date_time(committer.when());
            let committer_name = committer.name().unwrap_or(common::UNKNOWN()).to_string();
            CommitMetadata { summary, committer_date, committer_name }
        } else {
            CommitMetadata { summary: no_message(symbols), ..CommitMetadata::default() }
        }
    })
}

fn graph_row_at(walk_ctx: &Walker, worktrees: &Worktrees, commit_metadata: &mut CommitMetadataCache, hidden_branch_names: &HashSet<String>, symbols: &SymbolTheme, index: usize) -> Option<GraphRow> {
    if index >= walk_ctx.oids.get_commit_count() {
        return None;
    }

    graph_rows(walk_ctx, worktrees, commit_metadata, hidden_branch_names, symbols, index, index.saturating_add(1)).into_iter().next()
}

fn pane_rows(pane: GraphPane, walk_ctx: &Walker) -> Vec<GraphPaneRow> {
    let index_map = alias_index_map(walk_ctx);
    match pane {
        GraphPane::Branches => {
            let mut local: Vec<_> = walk_ctx.branches_local.iter().flat_map(|(&alias, branches)| branches.iter().map(move |branch| (alias, branch.clone(), true))).collect();
            let mut remote: Vec<_> = walk_ctx.branches_remote.iter().flat_map(|(&alias, branches)| branches.iter().map(move |branch| (alias, branch.clone(), false))).collect();
            local.sort_by(|a, b| a.1.cmp(&b.1));
            remote.sort_by(|a, b| a.1.cmp(&b.1));
            local
                .into_iter()
                .chain(remote)
                .map(|(alias, name, is_local)| GraphPaneRow::Branch { alias, name, is_local, lane: walk_ctx.branches_lanes.get(&alias).copied(), graph_index: index_map.get(&alias).copied() })
                .collect()
        },
        GraphPane::Tags => {
            let mut rows: Vec<_> = walk_ctx.tags_local.iter().flat_map(|(&alias, tags)| tags.iter().map(move |tag| (alias, tag.clone()))).collect();
            rows.sort_by(|a, b| a.1.cmp(&b.1));
            rows.into_iter().map(|(alias, name)| GraphPaneRow::Tag { alias, name, lane: walk_ctx.tags_lanes.get(&alias).copied(), graph_index: index_map.get(&alias).copied() }).collect()
        },
        GraphPane::Stashes => {
            let repo = walk_ctx.repo.borrow();
            walk_ctx
                .oids
                .stashes
                .iter()
                .map(|&alias| {
                    let oid = *walk_ctx.oids.get_oid_by_alias(alias);
                    let summary = repo.find_commit(oid).ok().and_then(|commit| commit.summary().map(str::to_string)).unwrap_or_else(|| status_text::STASH().to_string());
                    GraphPaneRow::Stash { alias, summary, lane: walk_ctx.stashes_lanes.get(&alias).copied(), graph_index: index_map.get(&alias).copied() }
                })
                .collect()
        },
        GraphPane::Reflogs => walk_ctx
            .head_reflog_entries
            .iter()
            .filter_map(|entry| {
                let alias = walk_ctx.oids.aliases.get(&entry.new_oid).copied()?;
                Some(GraphPaneRow::Reflog {
                    alias,
                    selector: entry.selector.clone(),
                    message: entry.message.clone(),
                    lane: walk_ctx.reflogs_lanes.get(&alias).copied(),
                    graph_index: index_map.get(&alias).copied(),
                })
            })
            .collect(),
    }
}

fn lookup(
    kind: GraphLookupKind, walk_ctx: &Walker, worktrees: &Worktrees, commit_metadata: &mut CommitMetadataCache, hidden_branch_names: &HashSet<String>, symbols: &SymbolTheme,
) -> GraphLookupResult {
    match kind {
        GraphLookupKind::GraphRowAt { index } => GraphLookupResult::GraphRow(graph_row_at(walk_ctx, worktrees, commit_metadata, hidden_branch_names, symbols, index)),
        GraphLookupKind::PaneRowAt { pane, index } => GraphLookupResult::PaneRow(pane_rows(pane, walk_ctx).get(index).cloned()),
        GraphLookupKind::BranchIndex { from, direction } => GraphLookupResult::Index(branch_index(walk_ctx, hidden_branch_names, from, direction)),
        GraphLookupKind::ShaPrefix { prefix } => {
            let oid = walk_ctx.oids.oids.iter().find(|oid| oid.to_string().starts_with(&prefix)).copied();
            let index = oid.and_then(|oid| walk_ctx.oids.aliases.get(&oid).copied()).and_then(|alias| walk_ctx.oids.get_sorted_aliases().iter().position(|&current| current == alias));
            GraphLookupResult::Index(index)
        },
        GraphLookupKind::Oid { oid } => {
            let index = walk_ctx.oids.aliases.get(&oid).copied().and_then(|alias| walk_ctx.oids.get_sorted_aliases().iter().position(|&current| current == alias));
            GraphLookupResult::Index(index)
        },
        GraphLookupKind::ParentIndex { index } => GraphLookupResult::Index(parent_index(walk_ctx, index)),
        GraphLookupKind::ChildIndex { index } => GraphLookupResult::Index(child_index(walk_ctx, index)),
    }
}

fn branch_index(walk_ctx: &Walker, hidden_branch_names: &HashSet<String>, from: usize, direction: GraphBranchJumpDirection) -> Option<usize> {
    let mut indices: Vec<usize> = pane_rows(GraphPane::Branches, walk_ctx)
        .into_iter()
        .filter_map(|row| match row {
            GraphPaneRow::Branch { name, graph_index: Some(index), .. } if !hidden_branch_names.contains(&name) => Some(index),
            _ => None,
        })
        .collect();
    indices.sort_unstable();
    indices.dedup();

    match direction {
        GraphBranchJumpDirection::Previous => indices.into_iter().rev().find(|&index| index < from),
        GraphBranchJumpDirection::Next => indices.into_iter().find(|&index| index > from),
    }
}

fn parent_index(walk_ctx: &Walker, index: usize) -> Option<usize> {
    let oid = walk_ctx.oids.get_sorted_aliases().get(index).map(|&alias| *walk_ctx.oids.get_oid_by_alias(alias))?;
    if walk_ctx.oids.is_zero(&oid) {
        return Some(1).filter(|idx| *idx < walk_ctx.oids.get_commit_count());
    }

    let repo = walk_ctx.repo.borrow();
    let commit = repo.find_commit(oid).ok()?;
    let parent_oid = commit.parent_ids().next()?;
    let parent_alias = walk_ctx.oids.aliases.get(&parent_oid).copied()?;
    walk_ctx.oids.get_sorted_aliases().iter().position(|&alias| alias == parent_alias)
}

fn child_index(walk_ctx: &Walker, index: usize) -> Option<usize> {
    let oid = walk_ctx.oids.get_sorted_aliases().get(index).map(|&alias| *walk_ctx.oids.get_oid_by_alias(alias))?;
    if walk_ctx.oids.is_zero(&oid) {
        return None;
    }

    let repo = walk_ctx.repo.borrow();
    walk_ctx.oids.get_sorted_aliases().iter().enumerate().take(index).find_map(|(idx, &alias)| {
        let child_oid = *walk_ctx.oids.get_oid_by_alias(alias);
        let child = repo.find_commit(child_oid).ok()?;
        child.parent_ids().any(|parent_oid| parent_oid == oid).then_some(idx)
    })
}

fn head_alias(walk_ctx: &Walker) -> u32 {
    let repo = walk_ctx.repo.borrow();
    repo.head().ok().and_then(|head| head.target()).and_then(|oid| walk_ctx.oids.aliases.get(&oid).copied()).unwrap_or(NONE)
}

fn alias_index_map(walk_ctx: &Walker) -> HashMap<u32, usize> {
    walk_ctx.oids.get_sorted_aliases().iter().enumerate().map(|(idx, &alias)| (alias, idx)).collect()
}

fn latest_reflogs_by_alias(walk_ctx: &Walker) -> HashMap<u32, HeadReflogAliasEntry> {
    let mut latest = HashMap::new();
    for entry in &walk_ctx.head_reflog_entries {
        let Some(&new_alias) = walk_ctx.oids.aliases.get(&entry.new_oid) else {
            continue;
        };
        let alias_entry = alias_reflog_entry(entry, new_alias);
        latest.entry(new_alias).or_insert(alias_entry);
    }
    latest
}

fn alias_reflog_entry(entry: &HeadReflogEntry, new_alias: u32) -> HeadReflogAliasEntry {
    HeadReflogAliasEntry { selector: entry.selector.clone(), old_oid: entry.old_oid, new_oid: entry.new_oid, new_alias, message: entry.message.clone(), time: entry.time }
}

fn worktrees_for_alias(worktrees: &Worktrees, walk_ctx: &Walker, alias: u32) -> Vec<WorktreeEntry> {
    worktrees
        .entries
        .iter()
        .filter_map(|entry| {
            let entry_alias = entry.head.and_then(|oid| walk_ctx.oids.aliases.get(&oid).copied());
            (entry_alias == Some(alias)).then(|| {
                let mut entry = entry.clone();
                entry.alias = Some(alias);
                entry
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/core/graph_service.rs"]
mod tests;
