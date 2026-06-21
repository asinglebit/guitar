use crate::git::queries::{commits::get_stashed_commits, reflogs::HeadReflogEntry};
use crate::{
    core::{
        batcher::{Batcher, WalkCommit},
        buffer::Buffer,
        chunk::{Chunk, LaneRef, NONE},
        oids::{Oids, git2_to_gix_oid, gix_to_git2_oid},
    },
    git::queries::commits::{get_sorted_oids, get_tag_oids, get_tip_oids},
    git::queries::reflogs::get_head_reflog_entries,
    git::repository::open,
};
use git2::Repository;
use im::{HashSet, Vector};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet as StdHashSet},
    rc::Rc,
};

// Walks git history into lane snapshots and ref lookup tables.
pub struct Walker {
    // Repository state shared with the batcher and stash query.
    pub repo: Rc<RefCell<Repository>>,

    // gitoxide repository shared with the batcher and commit metadata lookups.
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
    stash_aliases: StdHashSet<u32>,
    reflog_aliases: StdHashSet<u32>,
    stash_parent_aliases: Vec<(u32, u32)>,
    oid_batch: Vec<WalkCommit>,
    sorted_batch: Vec<u32>,

    // Number of commits requested per walk iteration.
    pub amount: usize,
}

impl Walker {
    // Open the repository and seed all metadata that does not depend on walking commits.
    pub fn new(path: String, amount: usize, hidden_branch_names: HashSet<String>, include_head_reflog_roots: bool, graph_lane_limit: usize) -> Result<Self, git2::Error> {
        let repo_path = path.clone();
        let repo = Rc::new(RefCell::new(open(path)?));
        let gix_repo = gix::open(repo_path.clone()).map_err(|error| git2::Error::from_str(&error.to_string()))?;

        let buffer = RefCell::new(Buffer::with_lane_limit(graph_lane_limit));

        let mut oids = Oids::default();

        // Branch and tag tips are registered before walking so aliases are stable.
        let branches_lanes = HashMap::new();
        let (branches_local, branches_remote) = get_tip_oids(&repo.borrow(), &mut oids);

        let tags_lanes = HashMap::new();
        let tags_local = get_tag_oids(&repo.borrow(), &mut oids);

        let stashes_lanes = HashMap::new();
        let reflogs_lanes = HashMap::new();

        // Stashes are collected up front so they can be inserted near their parents later.
        {
            let mut repo_mut = repo.borrow_mut();
            oids.stashes = get_stashed_commits(&mut repo_mut, &mut oids);
        }
        let stash_aliases: StdHashSet<u32> = oids.stashes.iter().copied().collect();
        let mut stash_parent_aliases = Vec::with_capacity(oids.stashes.len());
        for stash_alias in oids.stashes.clone() {
            let parent_oid =
                gix_repo.find_commit(git2_to_gix_oid(*oids.get_oid_by_alias(stash_alias))).ok().and_then(|commit| commit.parent_ids().next().map(|parent| gix_to_git2_oid(parent.detach())));
            if let Some(parent_oid) = parent_oid {
                let parent_alias = oids.get_alias_by_oid(parent_oid);
                stash_parent_aliases.push((stash_alias, parent_alias));
            }
        }

        let head_reflog_entries = get_head_reflog_entries(&repo.borrow()).unwrap_or_default();
        let mut head_reflog_roots = Vec::new();
        let mut reflog_aliases = StdHashSet::new();
        for entry in &head_reflog_entries {
            let alias = oids.get_alias_by_oid(entry.new_oid);
            reflog_aliases.insert(alias);
            if include_head_reflog_roots && !head_reflog_roots.contains(&entry.new_oid) {
                head_reflog_roots.push(entry.new_oid);
            }
        }

        let batcher = Batcher::new(repo.clone(), repo_path, &hidden_branch_names, &head_reflog_roots)?;
        let sorted_batch_capacity = amount.saturating_add(oids.stashes.len());

        Ok(Self {
            repo,
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
        let repo = self.repo.borrow();

        // Without HEAD there is no stable parent for the uncommitted pseudo-row.
        let head_oid = match repo.head().ok().and_then(|h| h.target()) {
            Some(oid) => oid,
            None => {
                return false;
            },
        };

        let head_alias = self.oids.get_alias_by_oid(head_oid);

        self.sorted_batch.clear();
        get_sorted_oids(&mut self.batcher, &mut self.oids, &mut self.sorted_batch, self.amount, &mut self.oid_batch);

        // Alias NONE is rendered as the uncommitted row above HEAD.
        if self.oids.get_commit_count() == 1 {
            self.buffer.borrow_mut().update(Chunk::uncommitted(head_alias, NONE));
        }

        // Place each stash near its first parent so it reads as a side snapshot.
        for &(stash_alias, parent_alias) in &self.stash_parent_aliases {
            if let Some(pos) = self.sorted_batch.iter().position(|&a| a == parent_alias) {
                self.sorted_batch.insert(if pos == 0 { 0 } else { pos - 1 }, stash_alias);
            }
        }

        // Hold one mutable buffer borrow while the page updates topology.
        let mut buffer = self.buffer.borrow_mut();

        let mut walked_commits = self.oid_batch.iter().peekable();

        for &alias in self.sorted_batch.iter() {
            let mut merger_alias: u32 = NONE;
            let mut transient_lane: Option<usize> = None;

            let (parent_a, parent_b) = if self.stash_aliases.contains(&alias) {
                if walked_commits.peek().and_then(|commit| self.oids.get_existing_alias(commit.oid)) == Some(alias) {
                    walked_commits.next();
                }
                let parent = self.stash_parent_aliases.iter().find_map(|&(stash_alias, parent_alias)| (stash_alias == alias).then_some(parent_alias)).unwrap_or(NONE);
                (parent, NONE)
            } else {
                let commit = walked_commits.next().expect("walked commit metadata matches sorted aliases");
                debug_assert_eq!(self.oids.get_existing_alias(commit.oid), Some(alias));

                // Only two parents are modeled because the renderer draws one merge edge.
                let parent_a_oid = commit.parent_ids.first().map(|parent| gix_to_git2_oid(*parent));
                let parent_b_oid = commit.parent_ids.get(1).map(|parent| gix_to_git2_oid(*parent));
                (parent_a_oid.map(|p| self.oids.get_alias_by_oid(p)).unwrap_or(NONE), parent_b_oid.map(|p| self.oids.get_alias_by_oid(p)).unwrap_or(NONE))
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

                if chunk.parent_a != NONE && chunk.parent_b != NONE {
                    // If the second parent is not already visible as a lane, mark a deferred merge.
                    let is_merger_found = buffer.curr.iter().any(|chunk_nested| chunk_nested.parent_a != NONE && chunk_nested.parent_b == NONE && chunk.parent_b == chunk_nested.parent_a);
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

fn parent_is_on_prior_lane(lanes: &Vector<Chunk>, parent: u32, before_lane: usize) -> bool {
    parent != NONE && lanes.iter().take(before_lane).any(|chunk| is_single_parent_lane_for(chunk, parent))
}

fn is_single_parent_lane_for(chunk: &Chunk, parent: u32) -> bool {
    (chunk.parent_a == parent && chunk.parent_b == NONE) || (chunk.parent_a == NONE && chunk.parent_b == parent)
}

#[cfg(test)]
#[path = "../tests/core/walker.rs"]
mod tests;
