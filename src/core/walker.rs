use crate::{
    core::{
        batcher::{Batcher, WalkedCommit},
        buffer::Buffer,
        chunk::{Chunk, LaneRef, NONE},
        oids::Oids,
    },
    git::gix::{enable_history_object_cache, gix_error, history_commit_count_hint},
    git::queries::commits::{get_sorted_oids, get_stashed_commits, get_tag_oids, get_tip_oids},
    git::queries::reflogs::{HeadReflogEntry, get_head_reflog_entries},
    helpers::heatmap::HeatmapCounts,
};
use im::HashSet;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet as StdHashSet},
};

// Walks git history into lane snapshots and ref lookup tables.
pub struct Walker {
    // Repository handle shared with the batcher and commit metadata lookups.
    pub gix_repo: gix::Repository,

    // Revwalk cursor for incremental history loading.
    pub batcher: Batcher,

    // Mutable lane buffer that records topology deltas.
    pub buffer: RefCell<Buffer>,

    // Alias and ref metadata accumulated during the walk.
    pub oids: Oids,

    pub branches_lanes: HashMap<u32, LaneRef>,
    pub branches_local: HashMap<u32, Vec<String>>,
    pub branches_remote: HashMap<u32, Vec<String>>,

    pub tags_lanes: HashMap<u32, LaneRef>,
    pub tags_local: HashMap<u32, Vec<String>>,

    pub stashes_lanes: HashMap<u32, LaneRef>,
    pub reflogs_lanes: HashMap<u32, LaneRef>,
    pub head_reflog_entries: Vec<HeadReflogEntry>,
    pub heatmap_counts: HeatmapCounts,
    head_alias: Option<u32>,
    stash_aliases: StdHashSet<u32>,
    reflog_aliases: StdHashSet<u32>,
    stash_parent_aliases: Vec<(u32, u32)>,
    oid_batch: Vec<WalkedCommit>,
    sorted_batch: Vec<u32>,

    // Number of commits requested per walk iteration.
    pub amount: usize,
}

impl Walker {
    // Open the repository and seed all metadata that does not depend on walking commits.
    pub fn new(path: String, amount: usize, hidden_branch_names: HashSet<String>, include_head_reflog_roots: bool, graph_lane_limit: usize) -> Result<Self, git2::Error> {
        let mut gix_repo = gix::open(path).map_err(gix_error)?;
        enable_history_object_cache(&mut gix_repo);
        let history_commit_hint = history_commit_count_hint(&gix_repo);

        let buffer = RefCell::new(Buffer::with_lane_limit(graph_lane_limit));

        let mut oids = Oids::default();

        // Branch and tag tips are registered before walking so aliases are stable.
        let branches_lanes = HashMap::new();
        let (branches_local, branches_remote, mut branch_tips) = get_tip_oids(&gix_repo, &mut oids, &hidden_branch_names);

        let tags_lanes = HashMap::new();
        let tags_local = get_tag_oids(&gix_repo, &mut oids);

        let stashes_lanes = HashMap::new();
        let reflogs_lanes = HashMap::new();

        // Stashes are collected up front so they can be inserted near their parents later.
        oids.stashes = get_stashed_commits(&gix_repo, &mut oids);
        let stash_aliases = std::mem::take(&mut oids.stashes);
        let mut stash_parent_aliases = Vec::with_capacity(stash_aliases.len());
        for stash_alias in stash_aliases.iter().copied() {
            let parent_oid = gix_repo.find_commit(*oids.get_gix_oid_by_alias(stash_alias)).ok().and_then(|commit| commit.parent_ids().next().map(|parent| parent.detach()));
            if let Some(parent_oid) = parent_oid {
                let parent_alias = oids.get_alias_by_oid(parent_oid);
                stash_parent_aliases.push((stash_alias, parent_alias));
            }
        }
        oids.stashes = stash_aliases;
        let stash_aliases: StdHashSet<u32> = oids.stashes.iter().copied().collect();

        let head_reflog_entries = get_head_reflog_entries(&gix_repo).unwrap_or_default();
        if let Some(commit_hint) = history_commit_hint {
            let alias_hint = commit_hint.saturating_add(tags_local.len()).saturating_add(oids.stashes.len()).saturating_add(head_reflog_entries.len());
            oids.reserve_total_aliases(alias_hint);
            buffer.borrow_mut().reserve_history(commit_hint);
        }
        let mut head_reflog_roots = Vec::new();
        let mut reflog_aliases = StdHashSet::new();
        for entry in &head_reflog_entries {
            let alias = oids.get_alias_by_oid(entry.new_oid);
            reflog_aliases.insert(alias);
            if include_head_reflog_roots && !head_reflog_roots.contains(&entry.new_oid) {
                head_reflog_roots.push(entry.new_oid);
            }
        }

        // Seed the cursor with every ref-backed root we render, plus optional head reflog roots.
        let mut extra_roots = Vec::with_capacity(tags_local.len().saturating_add(oids.stashes.len()).saturating_add(head_reflog_roots.len()));
        extra_roots.extend(tags_local.keys().copied().map(|alias| *oids.get_gix_oid_by_alias(alias)));
        extra_roots.extend(oids.stashes.iter().copied().map(|alias| *oids.get_gix_oid_by_alias(alias)));
        extra_roots.extend(head_reflog_roots);

        let mut pushed: StdHashSet<_> = branch_tips.iter().copied().collect();
        branch_tips.extend(extra_roots.into_iter().filter(|oid| pushed.insert(*oid)));
        let batcher = Batcher::from_tips_with_oids(&gix_repo, branch_tips, &mut oids)?;
        let head_alias = gix_repo.head_id().ok().map(|oid| oids.get_alias_by_oid(oid.detach()));
        let sorted_batch_capacity = amount.saturating_add(oids.stashes.len());

        Ok(Self {
            gix_repo,
            batcher,
            buffer,
            oids,
            branches_lanes,
            branches_local,
            branches_remote,
            tags_lanes,
            tags_local,
            stashes_lanes,
            reflogs_lanes,
            head_reflog_entries,
            heatmap_counts: HeatmapCounts::default(),
            head_alias,
            stash_aliases,
            reflog_aliases,
            stash_parent_aliases,
            oid_batch: Vec::with_capacity(amount),
            sorted_batch: Vec::with_capacity(sorted_batch_capacity),
            amount,
        })
    }

    // Process one revwalk page and update lane snapshots for the renderer.
    pub fn walk(&mut self) -> bool {
        // Without HEAD there is no stable parent for the uncommitted pseudo-row.
        let Some(head_alias) = self.head_alias else {
            return false;
        };

        self.sorted_batch.clear();
        get_sorted_oids(&mut self.batcher, &mut self.oids, &mut self.sorted_batch, self.amount, &mut self.oid_batch);
        for commit in &self.oid_batch {
            if let Some(seconds) = commit.commit_time {
                self.heatmap_counts.add_commit_seconds(seconds);
            }
        }

        // Alias NONE is rendered as the uncommitted row above HEAD.
        if self.oids.get_commit_count() == 1 {
            self.buffer.borrow_mut().update(Chunk::uncommitted(head_alias, NONE));
        }

        place_stashes_by_parent(&mut self.sorted_batch, &mut self.oid_batch, &self.stash_parent_aliases);

        // Hold one mutable buffer borrow while the page updates topology.
        let mut buffer = self.buffer.borrow_mut();

        let mut walked_commits = self.oid_batch.iter().peekable();

        for &alias in self.sorted_batch.iter() {
            let mut merger_alias: u32 = NONE;
            let mut transient_lane: Option<usize> = None;

            let (parent_a, parent_b) = if self.stash_aliases.contains(&alias) {
                if walked_commits.peek().is_some_and(|commit| commit.alias == alias) {
                    walked_commits.next();
                }
                let parent = self.stash_parent_aliases.iter().find_map(|&(stash_alias, parent_alias)| (stash_alias == alias).then_some(parent_alias)).unwrap_or(NONE);
                (parent, NONE)
            } else {
                let commit = walked_commits.next().expect("walked commit metadata matches sorted aliases");
                debug_assert_eq!(commit.alias, alias);

                // Only two parents are modeled because the renderer draws one merge edge.
                (commit.first_parent_alias(), commit.second_parent_alias())
            };

            let chunk = Chunk::commit(alias, parent_a, parent_b);

            let update = buffer.update(chunk);

            if let Some(chunk) = buffer.curr.get(update.lane.index)
                && !chunk.is_dummy()
                && alias == chunk.alias
            {
                let lane = update.lane;
                let lane_idx = lane.index;

                // Ref lanes are captured after the buffer decides where this alias sits.
                if self.branches_local.contains_key(&alias) || self.branches_remote.contains_key(&alias) {
                    self.branches_lanes.insert(alias, lane);
                }

                if self.tags_local.contains_key(&alias) {
                    self.tags_lanes.insert(alias, lane);
                }

                if self.stash_aliases.contains(&alias) {
                    self.stashes_lanes.insert(alias, lane);
                }

                if self.reflog_aliases.contains(&alias) {
                    self.reflogs_lanes.insert(alias, lane);
                }

                if !chunk.is_flattened && chunk.parent_a != NONE && chunk.parent_b != NONE {
                    // If the second parent is not already visible as a lane, mark a deferred merge.
                    let is_merger_found = buffer.curr.iter().enumerate().any(|(idx, chunk_nested)| idx != lane_idx && chunk_nested.has_parent(chunk.parent_b));
                    if !is_merger_found {
                        merger_alias = chunk.alias;
                    } else if update.started_lane
                        && !lane.is_flattened
                        && lane_idx + 1 == buffer.curr.len()
                        && parent_is_on_prior_lane(&buffer.curr, chunk.parent_a, lane_idx)
                        && parent_is_on_prior_lane(&buffer.curr, chunk.parent_b, lane_idx)
                    {
                        transient_lane = Some(lane_idx);
                    }
                }
            }

            if merger_alias != NONE {
                buffer.merger(merger_alias);
            } else if let Some(lane_idx) = transient_lane {
                buffer.expire_lane_after_snapshot(lane_idx);
            }

            // Preserve the rendered order separately from first-seen alias assignment.
            self.oids.append_sorted_alias(alias);
        }

        // Empty pages mean the worker is done; emit one backup so lane-window reconstruction has a final delta.
        if self.sorted_batch.is_empty() {
            buffer.backup();
            return false;
        }

        true
    }
}

fn place_stashes_by_parent(sorted_batch: &mut Vec<u32>, oid_batch: &mut Vec<WalkedCommit>, stash_parent_aliases: &[(u32, u32)]) {
    for &(stash_alias, parent_alias) in stash_parent_aliases {
        let Some(parent_pos) = sorted_batch.iter().position(|&alias| alias == parent_alias) else {
            continue;
        };

        if let Some(stash_pos) = sorted_batch.iter().position(|&alias| alias == stash_alias) {
            let stash = sorted_batch.remove(stash_pos);
            let commit = oid_batch.iter().position(|commit| commit.alias == stash_alias).map(|commit_pos| oid_batch.remove(commit_pos));
            let parent_pos = if stash_pos < parent_pos { parent_pos.saturating_sub(1) } else { parent_pos };
            let insert_pos = parent_pos.saturating_sub(1).min(sorted_batch.len());
            sorted_batch.insert(insert_pos, stash);
            if let Some(commit) = commit {
                oid_batch.insert(insert_pos.min(oid_batch.len()), commit);
            }
        } else {
            sorted_batch.insert(parent_pos.saturating_sub(1), stash_alias);
        }
    }
}

fn parent_is_on_prior_lane(lanes: &[Chunk], parent: u32, before_lane: usize) -> bool {
    parent != NONE && lanes.iter().take(before_lane).any(|chunk| is_single_parent_lane_for(chunk, parent))
}

fn is_single_parent_lane_for(chunk: &Chunk, parent: u32) -> bool {
    chunk.has_parent(parent)
}

#[cfg(test)]
#[path = "../tests/core/walker.rs"]
mod tests;
