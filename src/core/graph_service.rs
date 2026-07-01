use crate::{
    core::{
        chunk::{Chunk, LaneRef, NONE},
        oids::gix_to_git2_oid,
        reflogs::HeadReflogAliasEntry,
        walker::Walker,
        worktrees::{WorktreeEntry, Worktrees},
    },
    git::queries::{file_history::changed_file_status_at_commit_from_repo, helpers::FileStatus, reflogs::HeadReflogEntry},
    helpers::{
        heatmap::{DAYS, WEEKS},
        localisation::{empty, errors, status as status_text},
        symbols::SymbolTheme,
        time::gix_timestamp_to_utc_date_time,
    },
};
use git2::Oid;
use im::HashSet;
use smallvec::SmallVec;
use std::{
    collections::{HashMap, HashSet as StdHashSet},
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
pub type LaneSnapshot = SmallVec<[Chunk; 32]>;
pub type GraphSnapshot = LaneSnapshot;
pub type GraphBranchLabels = SmallVec<[GraphBranchLabel; 2]>;
pub type GraphTagLabels = SmallVec<[GraphTagLabel; 2]>;
pub type GraphWorktrees = SmallVec<[WorktreeEntry; 1]>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphHistory {
    rows: Vec<LaneSnapshot>,
}

impl GraphHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn from_rows(rows: impl IntoIterator<Item = impl IntoIterator<Item = Chunk>>) -> Self {
        Self { rows: rows.into_iter().map(|row| row.into_iter().collect()).collect() }
    }

    pub fn get(&self, index: usize) -> Option<&[Chunk]> {
        self.rows.get(index).map(|row| row.as_slice())
    }

    pub fn last(&self) -> Option<&[Chunk]> {
        self.rows.last().map(|row| row.as_slice())
    }

    pub fn push(&mut self, row: LaneSnapshot) {
        self.rows.push(row);
    }

    pub fn rows(&self) -> &[LaneSnapshot] {
        &self.rows
    }
}

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
    UpdateWorktrees { generation: Generation, worktrees: Vec<WorktreeEntry> },
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
    pub is_merge: bool,
    pub has_any_branch: bool,
    pub branches: GraphBranchLabels,
    pub tags: GraphTagLabels,
    pub is_stash: bool,
    pub stash_lane: Option<LaneRef>,
    pub worktrees: GraphWorktrees,
    pub has_current_worktree: bool,
    pub reflog: Option<GraphReflogLabel>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CommitMetadata {
    summary: String,
    committer_date: String,
    committer_name: String,
    is_merge_commit: bool,
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
    Worktrees { generation: Generation, version: GraphVersion, worktrees: Vec<WorktreeEntry> },
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

        let mut command_context = GraphCommandContext {
            generation,
            version: &mut version,
            tx: &tx,
            walk_ctx: &walk_ctx,
            worktrees: &mut worktrees,
            pending_graph: &mut pending_graph,
            pending_file_history: &mut pending_file_history,
            commit_metadata: &mut commit_metadata,
            hidden_branch_names: &config.hidden_branch_names,
            symbols: &config.symbols,
        };
        if !drain_commands(&rx, &mut command_context) {
            break;
        }

        if let Some((request_id, start, end)) = pending_graph.take() {
            let mut window_context = GraphWindowContext {
                generation,
                request_id,
                version,
                start,
                end,
                tx: &tx,
                walk_ctx: &walk_ctx,
                worktrees: &worktrees,
                commit_metadata: &mut commit_metadata,
                hidden_branch_names: &config.hidden_branch_names,
                symbols: &config.symbols,
            };
            send_graph_window(&mut window_context);
        }

        if is_complete && let Some((request_id, path)) = pending_file_history.take() {
            send_file_history(generation, request_id, path, &tx, &walk_ctx, &config.symbols);
        }

        if is_complete {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(GraphCommand::Shutdown) => break,
                Ok(command) => {
                    let mut command_context = GraphCommandContext {
                        generation,
                        version: &mut version,
                        tx: &tx,
                        walk_ctx: &walk_ctx,
                        worktrees: &mut worktrees,
                        pending_graph: &mut pending_graph,
                        pending_file_history: &mut pending_file_history,
                        commit_metadata: &mut commit_metadata,
                        hidden_branch_names: &config.hidden_branch_names,
                        symbols: &config.symbols,
                    };
                    if !handle_command(command, &mut command_context) {
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
            walk_ctx.oids.shrink_to_fit();
            walk_ctx.buffer.borrow_mut().shrink_to_fit();
            let heatmap = walk_ctx.heatmap_counts.build();
            let _ = tx.send(GraphEvent::Heatmap { generation, heatmap });

            if let Some((request_id, path)) = pending_file_history.take() {
                send_file_history(generation, request_id, path, &tx, &walk_ctx, &config.symbols);
            }
        }
    }
}

struct GraphCommandContext<'a> {
    generation: Generation,
    version: &'a mut GraphVersion,
    tx: &'a Sender<GraphEvent>,
    walk_ctx: &'a Walker,
    worktrees: &'a mut Worktrees,
    pending_graph: &'a mut Option<(RequestId, usize, usize)>,
    pending_file_history: &'a mut Option<(RequestId, String)>,
    commit_metadata: &'a mut CommitMetadataCache,
    hidden_branch_names: &'a HashSet<String>,
    symbols: &'a SymbolTheme,
}

fn drain_commands(rx: &Receiver<GraphCommand>, context: &mut GraphCommandContext<'_>) -> bool {
    while let Ok(command) = rx.try_recv() {
        if !handle_command(command, context) {
            return false;
        }
    }
    true
}

fn handle_command(command: GraphCommand, context: &mut GraphCommandContext<'_>) -> bool {
    match command {
        GraphCommand::Shutdown => false,
        GraphCommand::QueryGraphWindow { generation: cmd_generation, request_id, start, end } => {
            if cmd_generation == context.generation {
                *context.pending_graph = Some((request_id, start, end));
            }
            true
        },
        GraphCommand::QueryPaneWindow { generation: cmd_generation, pane, start, end } => {
            if cmd_generation == context.generation {
                send_pane_window(context.generation, *context.version, pane, start, end, context.tx, context.walk_ctx);
            }
            true
        },
        GraphCommand::QueryFileHistory { generation: cmd_generation, request_id, path } => {
            if cmd_generation == context.generation {
                *context.pending_file_history = Some((request_id, path));
            }
            true
        },
        GraphCommand::Lookup { generation: cmd_generation, request_id, kind } => {
            if cmd_generation == context.generation {
                let result = lookup(kind, context.walk_ctx, context.worktrees, context.commit_metadata, context.hidden_branch_names, context.symbols);
                let _ = context.tx.send(GraphEvent::LookupResult { generation: context.generation, request_id, result });
            }
            true
        },
        GraphCommand::UpdateWorktrees { generation: cmd_generation, worktrees: updated_worktrees } => {
            if cmd_generation == context.generation {
                context.worktrees.entries = updated_worktrees.clone();
                *context.version = (*context.version).saturating_add(1);
                let _ = context.tx.send(GraphEvent::Worktrees { generation: context.generation, version: *context.version, worktrees: updated_worktrees });
            }
            true
        },
    }
}

struct GraphWindowContext<'a> {
    generation: Generation,
    request_id: RequestId,
    version: GraphVersion,
    start: usize,
    end: usize,
    tx: &'a Sender<GraphEvent>,
    walk_ctx: &'a Walker,
    worktrees: &'a Worktrees,
    commit_metadata: &'a mut CommitMetadataCache,
    hidden_branch_names: &'a HashSet<String>,
    symbols: &'a SymbolTheme,
}

fn send_graph_window(context: &mut GraphWindowContext<'_>) {
    let total = context.walk_ctx.oids.get_commit_count();
    let start = context.start.min(total);
    let end = context.end.min(total);
    let history = context.walk_ctx.buffer.borrow().window(start, end.saturating_add(1));
    let rows = graph_rows(context.walk_ctx, context.worktrees, context.commit_metadata, context.hidden_branch_names, context.symbols, start, end);
    let head_alias = head_alias(context.walk_ctx);

    let _ = context.tx.send(GraphEvent::GraphWindow { generation: context.generation, request_id: context.request_id, version: context.version, start, end, total, head_alias, rows, history });
}

fn send_pane_window(generation: Generation, version: GraphVersion, pane: GraphPane, start: usize, end: usize, tx: &Sender<GraphEvent>, walk_ctx: &Walker) {
    let (total, rows) = pane_window_rows(pane, walk_ctx, start, end);
    let start = start.min(total);
    let end = end.min(total);

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
    let mut rows = Vec::new();

    for (graph_index, &alias) in walk_ctx.oids.get_sorted_aliases().iter().enumerate() {
        let oid = *walk_ctx.oids.get_gix_oid_by_alias(alias);
        if walk_ctx.oids.is_zero(&oid) {
            continue;
        }

        let Some(status) = changed_file_status_at_commit_from_repo(&walk_ctx.gix_repo, oid, path)? else {
            continue;
        };

        let summary = commit_summary_from_repo(&walk_ctx.gix_repo, oid, symbols);
        let git2_oid = gix_to_git2_oid(oid);
        rows.push(GraphFileHistoryRow { graph_index, oid: git2_oid, summary, status });
    }

    Ok(rows)
}

fn no_message(symbols: &SymbolTheme) -> String {
    format!("{} {}", symbols.empty_state.mark, empty::NO_MESSAGE())
}

fn commit_metadata_from_repo(repo: &gix::Repository, oid: gix::ObjectId, symbols: &SymbolTheme) -> CommitMetadata {
    repo.find_commit(oid)
        .ok()
        .and_then(|commit| {
            let summary = commit.message().ok().map(|message| String::from_utf8_lossy(message.summary().as_ref()).into_owned()).unwrap_or_else(|| no_message(symbols));
            let committer = commit.committer().ok()?;
            let committer_date = gix_timestamp_to_utc_date_time(committer.time().ok()?);
            let committer_name = String::from_utf8_lossy(committer.name.as_ref()).into_owned();
            let is_merge_commit = commit.parent_ids().take(2).count() > 1;
            Some(CommitMetadata { summary, committer_date, committer_name, is_merge_commit })
        })
        .unwrap_or_else(|| CommitMetadata { summary: no_message(symbols), ..CommitMetadata::default() })
}

fn commit_summary_from_repo(repo: &gix::Repository, oid: gix::ObjectId, symbols: &SymbolTheme) -> String {
    repo.find_commit(oid).ok().and_then(|commit| commit.message().ok().map(|message| String::from_utf8_lossy(message.summary().as_ref()).into_owned())).unwrap_or_else(|| no_message(symbols))
}

fn first_parent_oid_from_repo(repo: &gix::Repository, oid: gix::ObjectId) -> Option<gix::ObjectId> {
    repo.find_commit(oid).ok()?.parent_ids().next().map(|parent| parent.detach())
}

fn has_parent_oid(repo: &gix::Repository, child_oid: gix::ObjectId, parent_oid: gix::ObjectId) -> bool {
    repo.find_commit(child_oid).ok().is_some_and(|commit| commit.parent_ids().any(|parent| parent == parent_oid))
}

fn graph_rows(
    walk_ctx: &Walker, worktrees: &Worktrees, commit_metadata: &mut CommitMetadataCache, hidden_branch_names: &HashSet<String>, symbols: &SymbolTheme, start: usize, end: usize,
) -> Vec<GraphRow> {
    let latest_reflogs = latest_reflogs_by_alias(walk_ctx);
    let mut rows = Vec::with_capacity(end.saturating_sub(start));

    for index in start..end {
        let alias = walk_ctx.oids.get_sorted_aliases().get(index).copied().unwrap_or(NONE);
        let oid = *walk_ctx.oids.get_gix_oid_by_alias(alias);
        let is_uncommitted = alias == NONE || walk_ctx.oids.is_zero(&oid);
        let metadata = if is_uncommitted { CommitMetadata::default() } else { load_commit_metadata(walk_ctx, commit_metadata, alias, oid, symbols) };
        let git2_oid = gix_to_git2_oid(oid);

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
        let is_merge = metadata.is_merge_commit && !is_stash;
        let stash_lane = walk_ctx.stashes_lanes.get(&alias).copied();
        let worktrees = worktrees_for_alias(worktrees, walk_ctx, alias);
        let has_current_worktree = worktrees.iter().any(|entry| entry.is_current);
        let reflog = latest_reflogs.get(&alias).map(|entry| GraphReflogLabel { selector: entry.selector.clone(), message: entry.message.clone(), lane: walk_ctx.reflogs_lanes.get(&alias).copied() });

        rows.push(GraphRow {
            index,
            alias,
            oid: git2_oid,
            summary: metadata.summary,
            committer_date: metadata.committer_date,
            committer_name: metadata.committer_name,
            is_merge,
            has_any_branch,
            branches,
            tags,
            is_stash,
            stash_lane,
            worktrees,
            has_current_worktree,
            reflog,
        });
    }

    rows
}

fn load_commit_metadata(walk_ctx: &Walker, cache: &mut CommitMetadataCache, alias: u32, oid: gix::ObjectId, symbols: &SymbolTheme) -> CommitMetadata {
    cache.get_or_insert_with(alias, || commit_metadata_from_repo(&walk_ctx.gix_repo, oid, symbols))
}

fn graph_row_at(walk_ctx: &Walker, worktrees: &Worktrees, commit_metadata: &mut CommitMetadataCache, hidden_branch_names: &HashSet<String>, symbols: &SymbolTheme, index: usize) -> Option<GraphRow> {
    if index >= walk_ctx.oids.get_commit_count() {
        return None;
    }

    graph_rows(walk_ctx, worktrees, commit_metadata, hidden_branch_names, symbols, index, index.saturating_add(1)).into_iter().next()
}

fn pane_rows(pane: GraphPane, walk_ctx: &Walker) -> Vec<GraphPaneRow> {
    pane_window_rows(pane, walk_ctx, 0, usize::MAX).1
}

fn index_for_alias(index_map: &HashMap<u32, usize>, alias: u32) -> Option<usize> {
    index_map.get(&alias).copied()
}

fn stash_pane_row(walk_ctx: &Walker, index_map: &HashMap<u32, usize>, alias: u32) -> GraphPaneRow {
    let oid = *walk_ctx.oids.get_gix_oid_by_alias(alias);
    let summary = walk_ctx
        .gix_repo
        .find_commit(oid)
        .ok()
        .and_then(|commit| {
            let message = commit.message().ok()?;
            Some(String::from_utf8_lossy(message.summary().as_ref()).into_owned())
        })
        .unwrap_or_else(|| status_text::STASH().to_string());
    let lane = walk_ctx.stashes_lanes.get(&alias).copied();
    let graph_index = index_for_alias(index_map, alias);

    GraphPaneRow::Stash { alias, summary, lane, graph_index }
}

fn reflog_pane_row(walk_ctx: &Walker, index_map: &HashMap<u32, usize>, alias: u32, entry: &HeadReflogEntry) -> GraphPaneRow {
    let lane = walk_ctx.reflogs_lanes.get(&alias).copied();
    let graph_index = index_for_alias(index_map, alias);

    GraphPaneRow::Reflog { alias, selector: entry.selector.clone(), message: entry.message.clone(), lane, graph_index }
}

fn branch_pane_row(walk_ctx: &Walker, index_map: &HashMap<u32, usize>, alias: u32, name: &str, is_local: bool) -> GraphPaneRow {
    let lane = walk_ctx.branches_lanes.get(&alias).copied();
    let graph_index = index_for_alias(index_map, alias);

    GraphPaneRow::Branch { alias, name: name.to_string(), is_local, lane, graph_index }
}

fn tag_pane_row(walk_ctx: &Walker, index_map: &HashMap<u32, usize>, alias: u32, name: &str) -> GraphPaneRow {
    let lane = walk_ctx.tags_lanes.get(&alias).copied();
    let graph_index = index_for_alias(index_map, alias);

    GraphPaneRow::Tag { alias, name: name.to_string(), lane, graph_index }
}

fn pane_window_rows(pane: GraphPane, walk_ctx: &Walker, start: usize, end: usize) -> (usize, Vec<GraphPaneRow>) {
    match pane {
        GraphPane::Branches => branch_pane_rows(walk_ctx, start, end),
        GraphPane::Tags => tag_pane_rows(walk_ctx, start, end),
        GraphPane::Stashes => {
            let total = walk_ctx.oids.stashes.len();
            let window = pane_window(start, end, total);
            let visible_aliases = || walk_ctx.oids.stashes.iter().skip(window.start).take(window.len()).copied();
            let index_map = alias_indices_for(walk_ctx, visible_aliases());
            let rows = visible_aliases().map(|alias| stash_pane_row(walk_ctx, &index_map, alias)).collect();
            (total, rows)
        },
        GraphPane::Reflogs => {
            let visible_reflogs = || {
                let entries = walk_ctx.head_reflog_entries.iter();
                entries.filter_map(|entry| walk_ctx.oids.get_existing_alias(entry.new_oid).map(|alias| (alias, entry)))
            };
            let total = visible_reflogs().count();
            let window = pane_window(start, end, total);
            let selected_reflogs = || visible_reflogs().skip(window.start).take(window.len());
            let index_map = alias_indices_for(walk_ctx, selected_reflogs().map(|(alias, _)| alias));
            let rows = selected_reflogs().map(|(alias, entry)| reflog_pane_row(walk_ctx, &index_map, alias, entry)).collect();
            (total, rows)
        },
    }
}

fn branch_pane_rows(walk_ctx: &Walker, start: usize, end: usize) -> (usize, Vec<GraphPaneRow>) {
    let mut branches: Vec<_> = walk_ctx.branches_local.iter().flat_map(|(&alias, branches)| branches.iter().map(move |branch| (alias, branch, true))).collect();
    let mut remotes: Vec<_> = walk_ctx.branches_remote.iter().flat_map(|(&alias, branches)| branches.iter().map(move |branch| (alias, branch, false))).collect();
    branches.sort_by(|a, b| a.1.cmp(b.1));
    remotes.sort_by(|a, b| a.1.cmp(b.1));

    let total = branches.len() + remotes.len();
    let window = pane_window(start, end, total);
    let visible_rows = || branches.iter().chain(remotes.iter()).skip(window.start).take(window.len()).copied();
    let index_map = alias_indices_for(walk_ctx, visible_rows().map(|(alias, _, _)| alias));
    let rows = visible_rows().map(|(alias, name, is_local)| branch_pane_row(walk_ctx, &index_map, alias, name, is_local)).collect();

    (total, rows)
}

fn tag_pane_rows(walk_ctx: &Walker, start: usize, end: usize) -> (usize, Vec<GraphPaneRow>) {
    let mut tags: Vec<_> = walk_ctx.tags_local.iter().flat_map(|(&alias, tags)| tags.iter().map(move |tag| (alias, tag))).collect();
    tags.sort_by(|a, b| a.1.cmp(b.1));

    let total = tags.len();
    let window = pane_window(start, end, total);
    let visible_rows = || tags.iter().skip(window.start).take(window.len()).copied();
    let index_map = alias_indices_for(walk_ctx, visible_rows().map(|(alias, _)| alias));
    let rows = visible_rows().map(|(alias, name)| tag_pane_row(walk_ctx, &index_map, alias, name)).collect();

    (total, rows)
}

fn pane_window(start: usize, end: usize, total: usize) -> std::ops::Range<usize> {
    let start = start.min(total);
    start..end.min(total).max(start)
}

fn lookup(
    kind: GraphLookupKind, walk_ctx: &Walker, worktrees: &Worktrees, commit_metadata: &mut CommitMetadataCache, hidden_branch_names: &HashSet<String>, symbols: &SymbolTheme,
) -> GraphLookupResult {
    match kind {
        GraphLookupKind::GraphRowAt { index } => GraphLookupResult::GraphRow(graph_row_at(walk_ctx, worktrees, commit_metadata, hidden_branch_names, symbols, index)),
        GraphLookupKind::PaneRowAt { pane, index } => GraphLookupResult::PaneRow(pane_rows(pane, walk_ctx).get(index).cloned()),
        GraphLookupKind::BranchIndex { from, direction } => GraphLookupResult::Index(branch_index(walk_ctx, hidden_branch_names, from, direction)),
        GraphLookupKind::ShaPrefix { prefix } => {
            let index = walk_ctx.oids.get_alias_by_prefix(&prefix).and_then(|alias| walk_ctx.oids.get_sorted_aliases().iter().position(|&current| current == alias));
            GraphLookupResult::Index(index)
        },
        GraphLookupKind::Oid { oid } => {
            let index = walk_ctx.oids.get_existing_alias(oid).and_then(|alias| walk_ctx.oids.get_sorted_aliases().iter().position(|&current| current == alias));
            GraphLookupResult::Index(index)
        },
        GraphLookupKind::ParentIndex { index } => GraphLookupResult::Index(parent_index(walk_ctx, index)),
        GraphLookupKind::ChildIndex { index } => GraphLookupResult::Index(child_index(walk_ctx, index)),
    }
}

fn branch_index(walk_ctx: &Walker, hidden_branch_names: &HashSet<String>, from: usize, direction: GraphBranchJumpDirection) -> Option<usize> {
    let visible_aliases =
        walk_ctx.branches_local.iter().chain(&walk_ctx.branches_remote).filter(|(_, branches)| branches.iter().any(|name| !hidden_branch_names.contains(name))).map(|(&alias, _)| alias);
    let mut indices: Vec<_> = alias_indices_for(walk_ctx, visible_aliases).into_values().collect();
    indices.sort_unstable();

    match direction {
        GraphBranchJumpDirection::Previous => indices.into_iter().rev().find(|&index| index < from),
        GraphBranchJumpDirection::Next => indices.into_iter().find(|&index| index > from),
    }
}

fn parent_index(walk_ctx: &Walker, index: usize) -> Option<usize> {
    let oid = walk_ctx.oids.get_sorted_aliases().get(index).map(|&alias| *walk_ctx.oids.get_gix_oid_by_alias(alias))?;
    if walk_ctx.oids.is_zero(&oid) {
        return Some(1).filter(|idx| *idx < walk_ctx.oids.get_commit_count());
    }

    let parent_oid = first_parent_oid_from_repo(&walk_ctx.gix_repo, oid)?;
    let parent_alias = walk_ctx.oids.get_existing_alias(parent_oid)?;
    walk_ctx.oids.get_sorted_aliases().iter().position(|&alias| alias == parent_alias)
}

fn child_index(walk_ctx: &Walker, index: usize) -> Option<usize> {
    let oid = walk_ctx.oids.get_sorted_aliases().get(index).map(|&alias| *walk_ctx.oids.get_gix_oid_by_alias(alias))?;
    if walk_ctx.oids.is_zero(&oid) {
        return None;
    }

    walk_ctx.oids.get_sorted_aliases().iter().enumerate().take(index).find_map(|(idx, &alias)| {
        let child_oid = *walk_ctx.oids.get_gix_oid_by_alias(alias);
        has_parent_oid(&walk_ctx.gix_repo, child_oid, oid).then_some(idx)
    })
}

fn head_alias(walk_ctx: &Walker) -> u32 {
    walk_ctx.gix_repo.head_id().ok().and_then(|oid| walk_ctx.oids.get_existing_alias(oid.detach())).unwrap_or(NONE)
}

fn alias_indices_for<I>(walk_ctx: &Walker, aliases: I) -> HashMap<u32, usize>
where
    I: IntoIterator<Item = u32>,
{
    let mut wanted: StdHashSet<u32> = aliases.into_iter().collect();
    let mut indices = HashMap::with_capacity(wanted.len());

    if wanted.is_empty() {
        return indices;
    }

    for (idx, &alias) in walk_ctx.oids.get_sorted_aliases().iter().enumerate() {
        if wanted.remove(&alias) {
            indices.insert(alias, idx);
            if wanted.is_empty() {
                break;
            }
        }
    }

    indices
}

fn latest_reflogs_by_alias(walk_ctx: &Walker) -> HashMap<u32, HeadReflogAliasEntry> {
    let mut latest = HashMap::new();
    for entry in &walk_ctx.head_reflog_entries {
        let Some(new_alias) = walk_ctx.oids.get_existing_alias(entry.new_oid) else {
            continue;
        };
        let alias_entry = alias_reflog_entry(entry, new_alias);
        latest.entry(new_alias).or_insert(alias_entry);
    }
    latest
}

fn alias_reflog_entry(entry: &HeadReflogEntry, new_alias: u32) -> HeadReflogAliasEntry {
    HeadReflogAliasEntry {
        selector: entry.selector.clone(),
        old_oid: gix_to_git2_oid(entry.old_oid),
        new_oid: gix_to_git2_oid(entry.new_oid),
        new_alias,
        message: entry.message.clone(),
        time: entry.time,
    }
}

fn worktrees_for_alias(worktrees: &Worktrees, walk_ctx: &Walker, alias: u32) -> GraphWorktrees {
    worktrees
        .entries
        .iter()
        .filter_map(|entry| {
            let entry_alias = entry.head.and_then(|oid| walk_ctx.oids.get_existing_alias(oid));
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
