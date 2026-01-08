use crate::git::queries::commits::get_stashed_commits;
use crate::{
    core::{
        batcher::Batcher,
        buffer::Buffer,
        chunk::{Chunk, NONE},
        oids::Oids,
    },
    git::queries::commits::{get_sorted_oids, get_tag_oids, get_tip_oids},
};
use git2::{Oid, Repository};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

// Context for walking and rendering commits
pub struct Walker {
    // General
    pub repo: Rc<RefCell<Repository>>,

    // Batcher
    pub batcher: Batcher,

    // Walker utilities
    pub buffer: RefCell<Buffer>,

    // Walker data
    pub oids: Oids,

    pub branches_lanes: HashMap<u32, usize>,
    pub branches_local: HashMap<u32, Vec<String>>,
    pub branches_remote: HashMap<u32, Vec<String>>,

    pub tags_lanes: HashMap<u32, usize>,
    pub tags_local: HashMap<u32, Vec<String>>,

    pub stashes_lanes: HashMap<u32, usize>,

    // Batching
    pub amount: usize,
}

// Output structure for walk results
pub struct WalkerOutput {
    // Walker utilities
    pub buffer: RefCell<Buffer>,

    // Walker data
    pub oids: Oids,

    pub branches_lanes: HashMap<u32, usize>,
    pub branches_local: HashMap<u32, Vec<String>>,
    pub branches_remote: HashMap<u32, Vec<String>>,

    pub tags_lanes: HashMap<u32, usize>,
    pub tags_local: HashMap<u32, Vec<String>>,

    pub stashes_lanes: HashMap<u32, usize>,

    // Batching
    pub is_again: bool,
    pub is_first: bool,
}

impl Walker {
    // Creates a new walker
    pub fn new(
        path: String,
        amount: usize,
        visible: HashMap<u32, Vec<String>>,
    ) -> Result<Self, git2::Error> {
        let path = path.clone();
        let repo = Rc::new(RefCell::new(
            Repository::open(path).expect("Failed to open repo"),
        ));

        // Walker utilities
        let buffer = RefCell::new(Buffer::default());

        // Walker data
        let mut oids = Oids::default();
        let branches_lanes = HashMap::new();
        let (branches_local, branches_remote) = get_tip_oids(&repo.borrow(), &mut oids);

        let tags_lanes = HashMap::new();
        let tags_local = get_tag_oids(&repo.borrow(), &mut oids);

        let stashes_lanes = HashMap::new();

        // Get stashed commits and store them in oids
        {
            let mut repo_mut = repo.borrow_mut();
            oids.stashes = get_stashed_commits(&mut repo_mut, &mut oids);
        }

        // Batcher
        let batcher = Batcher::new(repo.clone(), visible, &mut oids).expect("Error");

        Ok(Self {
            repo,

            // Batcher
            batcher,

            // Walker utilities
            buffer,

            // Walker data
            oids,
            branches_lanes,
            branches_local,
            branches_remote,
            tags_lanes,
            tags_local,
            stashes_lanes,

            // Pagination
            amount,
        })
    }

    // Walk through "amount" commits, update buffers and render lines
    pub fn walk(&mut self) -> bool {
        // Determine current HEAD oid
        let head_oid = self.repo.borrow().head().unwrap().target().unwrap();

        // Get the alias
        let head_alias = self.oids.get_alias_by_oid(head_oid);

        // Sort commits
        let mut sorted_batch: Vec<u32> = Vec::new();
        get_sorted_oids(
            &self.batcher,
            &mut self.oids,
            &mut sorted_batch,
            self.amount,
        );

        // Make a fake commit for unstaged changes
        if self.oids.get_commit_count() == 1 {
            self.buffer
                .borrow_mut()
                .update(Chunk::uncommitted(head_alias, NONE));
        }

        // Get all the stashed commits here
        let stashes: Vec<u32> = self.oids.stashes.clone();

        // Insert stashes into sorted_batch right after their parent commit
        for &stash_alias in &stashes {
            let stash_oid = self.oids.get_oid_by_alias(stash_alias);
            let repo = self.repo.borrow();
            let stash_commit = repo.find_commit(*stash_oid).unwrap();

            if let Some(parent_oid) = stash_commit.parent_ids().next() {
                let parent_alias = self.oids.get_alias_by_oid(parent_oid);

                // Find the position of the parent in the current sorted batch
                if let Some(pos) = sorted_batch.iter().position(|&a| a == parent_alias) {
                    // Insert the stash alias right after the parent
                    sorted_batch.insert(if pos == 0 { 0 } else { pos - 1 }, stash_alias);
                }
            }
        }

        // Go through the commits, inferring the graph
        for &alias in sorted_batch.iter() {
            let mut merger_alias: u32 = NONE;
            let oid = self.oids.get_oid_by_alias(alias);
            let repo = self.repo.borrow();
            let commit = repo.find_commit(*oid).unwrap();
            let parents: Vec<Oid> = commit.parent_ids().collect();

            // Get parent aliases
            let (parent_a, parent_b) = if stashes.contains(&alias) {
                (
                    // For stash commits, only the first parent is used
                    parents
                        .first()
                        .map(|p| self.oids.get_alias_by_oid(*p))
                        .unwrap_or(NONE),
                    NONE,
                )
            } else {
                (
                    // For normal commits, use both parents if they exist
                    parents
                        .first()
                        .map(|p| self.oids.get_alias_by_oid(*p))
                        .unwrap_or(NONE),
                    parents
                        .get(1)
                        .map(|p| self.oids.get_alias_by_oid(*p))
                        .unwrap_or(NONE),
                )
            };

            // Create commit chunk for the current commit with its parents
            let chunk = Chunk::commit(alias, parent_a, parent_b);

            // Update
            self.buffer.borrow_mut().update(chunk);

            for (lane_idx, chunk) in (&self.buffer.borrow().curr).into_iter().enumerate() {
                if !chunk.is_dummy() && alias == chunk.alias {
                    if self.branches_local.contains_key(&alias)
                        || self.branches_remote.contains_key(&alias)
                    {
                        self.branches_lanes.insert(alias, lane_idx);
                    }

                    if self.tags_local.contains_key(&alias) {
                        self.tags_lanes.insert(alias, lane_idx);
                    }

                    if stashes.contains(&alias) {
                        self.stashes_lanes.insert(alias, lane_idx);
                    }

                    if chunk.parent_a != NONE && chunk.parent_b != NONE {
                        let mut is_merger_found = false;
                        for chunk_nested in &self.buffer.borrow().curr {
                            if chunk_nested.parent_a != NONE
                                && chunk_nested.parent_b == NONE
                                && chunk.parent_b == chunk_nested.parent_a
                            {
                                is_merger_found = true;
                                break;
                            }
                        }
                        if !is_merger_found {
                            merger_alias = chunk.alias;
                        }
                    }
                }
            }

            // Now we can borrow mutably
            if merger_alias != NONE {
                self.buffer.borrow_mut().merger(merger_alias);
            }

            // Serialize
            self.oids.append_sorted_alias(alias);
        }

        // Indicate whether repeats are needed
        // Too lazy to make an off by one mistake here, zero is fine
        if sorted_batch.is_empty() {
            self.buffer.borrow_mut().backup();
            return false;
        }

        true
    }
}
