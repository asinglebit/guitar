use crate::{
    core::{
        chunk::NONE,
        oids::{IntoGixOid, Oids},
    },
    git::gix::{commit_graph_if_available, for_each_branch_tip, gix_error},
};
use gix::traverse::commit::{Either, find};
use im::HashSet;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashSet as StdHashSet},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkedCommit {
    pub oid: gix::ObjectId,
    pub alias: u32,
    parents: ParentAliases,
    pub commit_time: Option<i64>,
}

impl WalkedCommit {
    #[cfg(test)]
    fn new(oid: gix::ObjectId, alias: u32, parent_aliases: [u32; 2], parent_len: u8, commit_time: Option<i64>) -> Self {
        Self { oid, alias, parents: ParentAliases { aliases: parent_aliases, len: parent_len }, commit_time }
    }

    fn from_parents(oid: gix::ObjectId, alias: u32, parents: ParentAliases, commit_time: Option<i64>) -> Self {
        Self { oid, alias, parents, commit_time }
    }

    pub fn is_parentless(&self) -> bool {
        self.parents.is_empty()
    }

    pub fn first_parent_alias(&self) -> u32 {
        self.parents.get(0)
    }

    pub fn second_parent_alias(&self) -> u32 {
        self.parents.get(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParentAliases {
    aliases: [u32; 2],
    len: u8,
}

impl Default for ParentAliases {
    fn default() -> Self {
        Self { aliases: [NONE; 2], len: 0 }
    }
}

impl ParentAliases {
    fn push(&mut self, alias: u32) {
        if let Some(slot) = self.aliases.get_mut(self.len as usize) {
            *slot = alias;
            self.len += 1;
        }
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn get(&self, index: usize) -> u32 {
        (index < self.len as usize).then_some(self.aliases[index]).unwrap_or(NONE)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingCommit {
    commit_time: i64,
    oid: gix::ObjectId,
    alias: u32,
    graph_pos: Option<gix::commitgraph::Position>,
}

impl Ord for PendingCommit {
    fn cmp(&self, other: &Self) -> Ordering {
        self.commit_time.cmp(&other.commit_time).then_with(|| self.graph_pos.cmp(&other.graph_pos)).then_with(|| self.alias.cmp(&other.alias))
    }
}

impl PartialOrd for PendingCommit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct CommitCursor {
    queue: BinaryHeap<PendingCommit>,
    seen: SeenCommits,
    objects: gix::OdbHandle,
    commit_graph: Option<gix::commitgraph::Graph>,
    // gix object APIs require Vec<u8> scratch buffers; these are reused for the whole cursor.
    commit_buf: Vec<u8>,
    time_buf: Vec<u8>,
}

#[derive(Default)]
struct SeenCommits {
    graph_positions: Vec<u64>,
    loose_oids: StdHashSet<gix::ObjectId>,
}

impl SeenCommits {
    fn new(commit_graph: Option<&gix::commitgraph::Graph>, loose_capacity: usize) -> Self {
        let graph_positions = commit_graph.map_or_else(Vec::new, |graph| vec![0; bit_words(graph.num_commits() as usize)]);
        Self { graph_positions, loose_oids: StdHashSet::with_capacity(loose_capacity) }
    }

    fn insert_graph_pos(&mut self, pos: gix::commitgraph::Position) -> bool {
        let pos = pos.0 as usize;
        let word = pos / u64::BITS as usize;
        let mask = 1u64 << (pos % u64::BITS as usize);
        let Some(word_bits) = self.graph_positions.get_mut(word) else {
            return false;
        };
        let was_seen = *word_bits & mask != 0;
        *word_bits |= mask;
        !was_seen
    }

    fn insert_loose_oid(&mut self, oid: gix::ObjectId) -> bool {
        self.loose_oids.insert(oid)
    }
}

fn bit_words(bits: usize) -> usize {
    bits.div_ceil(u64::BITS as usize)
}

impl CommitCursor {
    fn new(repo: &gix::Repository, tips: Vec<(gix::ObjectId, u32)>) -> Result<Self, git2::Error> {
        let commit_graph = commit_graph_if_available(repo);
        let mut cursor = Self {
            queue: BinaryHeap::with_capacity(tips.len()),
            seen: SeenCommits::new(commit_graph.as_ref(), tips.len()),
            objects: repo.objects.clone(),
            commit_graph,
            commit_buf: Vec::new(),
            time_buf: Vec::new(),
        };

        for (tip, alias) in tips {
            cursor.enqueue(tip, alias)?;
        }

        Ok(cursor)
    }

    fn enqueue(&mut self, oid: gix::ObjectId, alias: u32) -> Result<(), git2::Error> {
        enqueue_commit(&mut self.queue, &mut self.seen, &self.objects, self.commit_graph.as_ref(), &mut self.time_buf, oid, alias)
    }

    fn next_commit(&mut self) -> Option<WalkedCommit> {
        self.next_commit_with_aliases(None)
    }

    fn next_commit_aliased(&mut self, oids: &mut Oids) -> Option<WalkedCommit> {
        self.next_commit_with_aliases(Some(oids))
    }

    fn next_commit_with_aliases(&mut self, aliases: Option<&mut Oids>) -> Option<WalkedCommit> {
        let mut aliases = aliases;
        loop {
            let entry = self.queue.pop()?;
            if let Some(commit) = self.commit_from_entry(entry, aliases.as_deref_mut()) {
                return Some(commit);
            }
        }
    }

    fn commit_from_entry(&mut self, entry: PendingCommit, aliases: Option<&mut Oids>) -> Option<WalkedCommit> {
        let PendingCommit { commit_time, oid, alias, graph_pos } = entry;

        if let (Some(graph), Some(pos)) = (self.commit_graph.as_ref(), graph_pos) {
            let commit = graph.commit_at(pos);
            let parents = graph_parent_aliases(&mut self.queue, &mut self.seen, graph, commit, aliases)?;
            return Some(WalkedCommit::from_parents(oid, alias, parents, Some(commit_time)));
        }

        match find(self.commit_graph.as_ref(), &self.objects, oid.as_ref(), &mut self.commit_buf).map_err(gix_error).ok()? {
            Either::CachedCommit(commit) => {
                let graph = self.commit_graph.as_ref()?;
                let parents = graph_parent_aliases(&mut self.queue, &mut self.seen, graph, commit, aliases)?;
                Some(WalkedCommit::from_parents(oid, alias, parents, Some(commit_time)))
            },
            Either::CommitRefIter(iter) => {
                let parents = loose_parent_aliases(&mut self.queue, &mut self.seen, &self.objects, self.commit_graph.as_ref(), &mut self.time_buf, iter.parent_ids(), aliases);
                Some(WalkedCommit::from_parents(oid, alias, parents, Some(commit_time)))
            },
        }
    }
}

fn graph_parent_aliases(
    queue: &mut BinaryHeap<PendingCommit>, seen: &mut SeenCommits, graph: &gix::commitgraph::Graph, commit: gix::commitgraph::file::Commit<'_>, mut aliases: Option<&mut Oids>,
) -> Option<ParentAliases> {
    commit.iter_parents().try_fold(ParentAliases::default(), |mut parents, parent_pos| {
        let parent_pos = parent_pos.ok()?;
        let parent = graph.commit_at(parent_pos);
        let parent_oid = parent.id().to_owned();
        let parent_alias = alias_for(aliases.as_deref_mut(), parent_oid);
        parents.push(parent_alias);
        let _ = enqueue_graph_position(queue, seen, graph, parent_pos, parent_oid, parent_alias);
        Some(parents)
    })
}

fn loose_parent_aliases(
    queue: &mut BinaryHeap<PendingCommit>, seen: &mut SeenCommits, objects: &gix::OdbHandle, commit_graph: Option<&gix::commitgraph::Graph>, time_buf: &mut Vec<u8>,
    parent_ids: impl IntoIterator<Item = gix::ObjectId>, mut aliases: Option<&mut Oids>,
) -> ParentAliases {
    parent_ids.into_iter().fold(ParentAliases::default(), |mut parents, parent_oid| {
        let parent_alias = alias_for(aliases.as_deref_mut(), parent_oid);
        parents.push(parent_alias);
        let _ = enqueue_commit(queue, seen, objects, commit_graph, time_buf, parent_oid, parent_alias);
        parents
    })
}

fn alias_for(aliases: Option<&mut Oids>, oid: gix::ObjectId) -> u32 {
    aliases.map_or(NONE, |oids| oids.get_alias_by_oid(oid))
}

fn enqueue_commit(
    queue: &mut BinaryHeap<PendingCommit>, seen: &mut SeenCommits, objects: &gix::OdbHandle, commit_graph: Option<&gix::commitgraph::Graph>, time_buf: &mut Vec<u8>, oid: gix::ObjectId, alias: u32,
) -> Result<(), git2::Error> {
    if let Some(graph) = commit_graph
        && let Some(pos) = graph.lookup(oid.as_ref())
    {
        return enqueue_graph_position(queue, seen, graph, pos, oid, alias);
    }

    if !seen.insert_loose_oid(oid) {
        return Ok(());
    }
    let commit_time = match find(commit_graph, objects, oid.as_ref(), time_buf).map_err(gix_error)? {
        Either::CachedCommit(commit) => commit.committer_timestamp() as i64,
        Either::CommitRefIter(iter) => iter.committer().map_err(gix_error)?.seconds(),
    };

    queue.push(PendingCommit { commit_time, oid, alias, graph_pos: None });
    Ok(())
}

fn enqueue_graph_position(
    queue: &mut BinaryHeap<PendingCommit>, seen: &mut SeenCommits, graph: &gix::commitgraph::Graph, pos: gix::commitgraph::Position, oid: gix::ObjectId, alias: u32,
) -> Result<(), git2::Error> {
    if !seen.insert_graph_pos(pos) {
        return Ok(());
    }
    let commit_time = graph.commit_at(pos).committer_timestamp() as i64;
    queue.push(PendingCommit { commit_time, oid, alias, graph_pos: Some(pos) });
    Ok(())
}

// Own a lazy commit cursor so history pages don't precompute the entire graph.
pub struct Batcher {
    cursor: Option<CommitCursor>,
}

impl Batcher {
    pub fn from_tips(repo: &gix::Repository, tips: Vec<gix::ObjectId>) -> Result<Self, git2::Error> {
        Ok(Self { cursor: Some(Self::cursor_from_tips(repo, tips)?) })
    }

    pub fn from_tips_with_oids(repo: &gix::Repository, tips: Vec<gix::ObjectId>, oids: &mut Oids) -> Result<Self, git2::Error> {
        oids.reserve_aliases(tips.len());
        Ok(Self { cursor: Some(Self::cursor_from_aliased_tips(repo, tips.into_iter().map(|oid| (oid, oids.get_alias_by_oid(oid))).collect())?) })
    }

    // Build the initial commit cursor from all visible local and remote branch tips.
    pub fn new<I: IntoIterator<Item = O>, O: IntoGixOid>(repo: &gix::Repository, hidden_branch_names: &HashSet<String>, extra_roots: I) -> Result<Self, git2::Error> {
        let cursor = Self::build(repo, hidden_branch_names, extra_roots)?;
        Ok(Self { cursor: Some(cursor) })
    }

    // Recreate the cursor after branch filters, fetches, or repository state changes.
    pub fn reset<I: IntoIterator<Item = O>, O: IntoGixOid>(&mut self, repo: &gix::Repository, hidden_branch_names: &HashSet<String>, extra_roots: I) -> Result<(), git2::Error> {
        self.cursor = Some(Self::build(repo, hidden_branch_names, extra_roots)?);
        Ok(())
    }

    // Pull the next page, dropping commits the object database cannot resolve.
    pub fn next(&mut self, count: usize) -> Vec<WalkedCommit> {
        let mut page = Vec::with_capacity(count);
        self.next_into(count, &mut page);
        page
    }

    // Pull the next page into an existing output buffer to avoid a temporary page allocation.
    pub fn next_into(&mut self, count: usize, out: &mut Vec<WalkedCommit>) -> usize {
        self.take_into(count, out, CommitCursor::next_commit)
    }

    pub fn next_aliased_into(&mut self, count: usize, out: &mut Vec<WalkedCommit>, oids: &mut Oids) -> usize {
        oids.reserve_aliases(count.saturating_mul(2));
        self.take_into(count, out, |cursor| cursor.next_commit_aliased(oids))
    }

    fn take_into(&mut self, count: usize, out: &mut Vec<WalkedCommit>, mut next: impl FnMut(&mut CommitCursor) -> Option<WalkedCommit>) -> usize {
        let before = out.len();
        let Some(cursor) = self.cursor.as_mut() else {
            return 0;
        };

        out.extend(std::iter::from_fn(|| next(cursor)).take(count));
        let fetched = out.len() - before;
        if fetched < count {
            self.cursor = None;
        }
        fetched
    }

    fn build<I: IntoIterator<Item = O>, O: IntoGixOid>(repo: &gix::Repository, hidden_branch_names: &HashSet<String>, extra_roots: I) -> Result<CommitCursor, git2::Error> {
        let mut pushed: StdHashSet<gix::ObjectId> = StdHashSet::new();
        let mut tips: Vec<gix::ObjectId> = Vec::new();

        for_each_branch_tip(repo, |_, name, oid| {
            // Hidden branch names are a deny-list; new branches are visible by default.
            if !hidden_branch_names.contains(name) && pushed.insert(oid) {
                tips.push(oid);
            }
        })?;

        tips.extend(extra_roots.into_iter().map(IntoGixOid::into_gix_oid).filter(|oid| pushed.insert(*oid)));

        Self::cursor_from_tips(repo, tips)
    }

    fn cursor_from_tips(repo: &gix::Repository, tips: Vec<gix::ObjectId>) -> Result<CommitCursor, git2::Error> {
        Self::cursor_from_aliased_tips(repo, tips.into_iter().map(|oid| (oid, NONE)).collect())
    }

    fn cursor_from_aliased_tips(repo: &gix::Repository, tips: Vec<(gix::ObjectId, u32)>) -> Result<CommitCursor, git2::Error> {
        CommitCursor::new(repo, tips)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::oids::{IntoGixOid, Oids, git2_to_gix_oid},
        git::test_support::{TestDir, commit_file, create_branch, init_repo_at},
    };
    use git2::{BranchType, Commit, Oid, Repository};
    use im::HashSet;
    use std::{collections::HashSet as StdHashSet, fs, path::Path};

    fn temp_repo(name: &str) -> (TestDir, Repository) {
        let dir = TestDir::new(name);
        let repo = init_repo_at(&dir.join("repo"));
        (dir, repo)
    }

    fn head_refname(repo: &Repository) -> String {
        repo.head().unwrap().name().unwrap().to_string()
    }

    fn checkout(repo: &Repository, reference: &str) {
        repo.set_head(reference).unwrap();
        repo.checkout_head(None).unwrap();
    }

    fn commit_with_parents(repo: &Repository, file: &str, message: &str, parents: &[&Commit<'_>]) -> Oid {
        let workdir = repo.workdir().unwrap();
        fs::write(workdir.join(file), message).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let signature = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, parents).unwrap()
    }

    fn hidden_names(names: &[&str]) -> HashSet<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    fn walk_oids<I, O>(dir: &TestDir, hidden_branch_names: HashSet<String>, extra_roots: I) -> Vec<gix::ObjectId>
    where
        I: IntoIterator<Item = O>,
        O: IntoGixOid,
    {
        let gix_repo = gix::open(dir.join("repo")).unwrap();
        let mut batcher = Batcher::new(&gix_repo, &hidden_branch_names, extra_roots).unwrap();
        batcher.next(10).into_iter().map(|commit| commit.oid).collect()
    }

    #[test]
    fn next_into_appends_pages_without_replacing_existing_output() {
        let (dir, repo) = temp_repo("next-into");
        let first = commit_file(&repo, "first.txt", "first", "first");
        let second = commit_file(&repo, "second.txt", "second", "second");
        let third = commit_file(&repo, "third.txt", "third", "third");
        let sentinel = WalkedCommit::new(git2_to_gix_oid(Oid::zero()), NONE, [NONE; 2], 0, None);
        let gix_repo = gix::open(dir.join("repo")).unwrap();
        let mut batcher = Batcher::new(&gix_repo, &HashSet::new(), vec![third]).unwrap();
        let mut out = vec![sentinel.clone()];

        assert_eq!(batcher.next_into(2, &mut out), 2);
        assert_eq!(batcher.next_into(2, &mut out), 1);
        assert_eq!(batcher.next_into(2, &mut out), 0);

        let oids = out.iter().map(|commit| commit.oid).collect::<Vec<_>>();
        let expected = [Oid::zero(), third, second, first].into_iter().map(git2_to_gix_oid).collect::<Vec<_>>();
        assert_eq!(oids, expected);
        assert!(out[3].is_parentless());
    }

    #[test]
    fn merge_commit_carries_commit_and_parent_aliases() {
        let (dir, repo) = temp_repo("merge-parents");
        let base = commit_file(&repo, "base.txt", "base", "base");
        create_branch(&repo, "side", base);
        let main_ref = head_refname(&repo);
        let main = commit_file(&repo, "main.txt", "main", "main");
        checkout(&repo, "refs/heads/side");
        let side = commit_file(&repo, "side.txt", "side", "side");
        checkout(&repo, &main_ref);
        let merge = {
            let main_commit = repo.find_commit(main).unwrap();
            let side_commit = repo.find_commit(side).unwrap();
            commit_with_parents(&repo, "merge.txt", "merge", &[&main_commit, &side_commit])
        };
        let gix_repo = gix::open(dir.join("repo")).unwrap();
        let mut oids = Oids::default();
        let mut batcher = Batcher::from_tips_with_oids(&gix_repo, vec![git2_to_gix_oid(merge)], &mut oids).unwrap();
        let mut page = Vec::new();

        assert_eq!(batcher.next_aliased_into(1, &mut page, &mut oids), 1);
        let merge_commit = &page[0];

        assert_eq!(merge_commit.oid, git2_to_gix_oid(merge));
        assert_eq!(merge_commit.alias, oids.get_existing_alias(merge).unwrap());
        assert_eq!(merge_commit.first_parent_alias(), oids.get_existing_alias(main).unwrap());
        assert_eq!(merge_commit.second_parent_alias(), oids.get_existing_alias(side).unwrap());
    }

    #[test]
    fn root_filter_cases_use_visible_tips_once() {
        for (name, actual, expected) in [
            {
                let (dir, repo) = temp_repo("duplicate-tips");
                let first = commit_file(&repo, "first.txt", "first", "first");
                let second = commit_file(&repo, "second.txt", "second", "second");
                create_branch(&repo, "duplicate", second);
                (
                    "duplicate branch tips are returned once",
                    walk_oids(&dir, HashSet::new(), vec![second]).into_iter().collect::<StdHashSet<_>>(),
                    StdHashSet::from([git2_to_gix_oid(second), git2_to_gix_oid(first)]),
                )
            },
            {
                let (dir, repo) = temp_repo("hidden-tip");
                let main = commit_file(&repo, "main.txt", "main", "main");
                let main_ref = head_refname(&repo);
                create_branch(&repo, "hidden", main);
                checkout(&repo, "refs/heads/hidden");
                let _hidden = commit_file(&repo, "hidden.txt", "hidden", "hidden");
                checkout(&repo, &main_ref);
                (
                    "hidden branch tip is not used as a walk root",
                    walk_oids(&dir, hidden_names(&["hidden"]), Vec::<Oid>::new()).into_iter().collect::<StdHashSet<_>>(),
                    StdHashSet::from([git2_to_gix_oid(main)]),
                )
            },
            {
                let (dir, repo) = temp_repo("remote-tip");
                let base = commit_file(&repo, "base.txt", "base", "base");
                let main_ref = head_refname(&repo);
                create_branch(&repo, "side", base);
                checkout(&repo, "refs/heads/side");
                let side = commit_file(&repo, "side.txt", "side", "side");
                checkout(&repo, &main_ref);
                repo.find_branch("side", BranchType::Local).unwrap().delete().unwrap();
                repo.reference("refs/remotes/origin/side", side, true, "test").unwrap();
                (
                    "remote branch tip is used and can be hidden",
                    walk_oids(&dir, HashSet::new(), Vec::<Oid>::new()).into_iter().collect::<StdHashSet<_>>(),
                    StdHashSet::from([git2_to_gix_oid(side), git2_to_gix_oid(base)]),
                )
            },
            {
                let (dir, repo) = temp_repo("remote-tip-hidden");
                let base = commit_file(&repo, "base.txt", "base", "base");
                let main_ref = head_refname(&repo);
                create_branch(&repo, "side", base);
                checkout(&repo, "refs/heads/side");
                let side = commit_file(&repo, "side.txt", "side", "side");
                checkout(&repo, &main_ref);
                repo.find_branch("side", BranchType::Local).unwrap().delete().unwrap();
                repo.reference("refs/remotes/origin/side", side, true, "test").unwrap();
                (
                    "hidden remote branch tip is not used as a walk root",
                    walk_oids(&dir, hidden_names(&["origin/side"]), Vec::<Oid>::new()).into_iter().collect::<StdHashSet<_>>(),
                    StdHashSet::from([git2_to_gix_oid(base)]),
                )
            },
        ] {
            assert_eq!(actual, expected, "{name}");
        }
    }

    #[test]
    fn bit_words_and_seen_commits_track_storage_boundaries() {
        for (bits, expected) in [(0, 0), (1, 1), (64, 1), (65, 2)] {
            assert_eq!(bit_words(bits), expected);
        }

        let mut seen = SeenCommits::default();
        seen.graph_positions.resize(2, 0);

        assert!(seen.insert_graph_pos(gix::commitgraph::Position(64)));
        assert!(!seen.insert_graph_pos(gix::commitgraph::Position(64)));
        assert!(seen.loose_oids.is_empty());

        let oid = git2_to_gix_oid(Oid::zero());
        let mut seen = SeenCommits::default();

        assert!(seen.insert_loose_oid(oid));
        assert!(!seen.insert_loose_oid(oid));
    }
}
