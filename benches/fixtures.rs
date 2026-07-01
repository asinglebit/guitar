use git2::{Oid, Repository, Signature, StashFlags, build::CheckoutBuilder};
use guitar::{
    core::{
        buffer::Buffer,
        chunk::{Chunk, NONE},
        graph_service::{GraphBranchLabel, GraphBranchLabels, GraphHistory, GraphRow, GraphTagLabel, GraphTagLabels, GraphWorktrees},
        oids::git2_to_gix_oid,
        worktrees::{WorktreeEntry, WorktreeKind},
    },
    helpers::{palette::Theme, symbols::SymbolTheme},
};
use im::HashSet;
use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};

#[allow(dead_code)]
pub struct TempFixture {
    root: tempfile::TempDir,
}

impl TempFixture {
    pub fn path(&self) -> &Path {
        self.root.path()
    }

    pub fn to_path_buf(&self) -> PathBuf {
        self.path().to_path_buf()
    }
}

impl Deref for TempFixture {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}

#[allow(dead_code)]
pub fn temp_dir(name: &str) -> TempFixture {
    let prefix = format!("guitar-{name}-");
    TempFixture { root: tempfile::Builder::new().prefix(&prefix).tempdir().unwrap() }
}

#[allow(dead_code)]
pub fn temp_repo(name: &str) -> (TempFixture, Repository) {
    let path = temp_dir(name);
    let repo = Repository::init(path.path()).unwrap();
    configure_repo(&repo);
    (path, repo)
}

#[allow(dead_code)]
pub fn write_text(root: &Path, file: &str, contents: &str) {
    write_bytes(root, file, contents.as_bytes());
}

#[allow(dead_code)]
pub fn write_bytes(root: &Path, file: &str, contents: &[u8]) {
    let path = root.join(file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[allow(dead_code)]
pub fn add_path(repo: &Repository, path: &str) {
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(path)).unwrap();
    index.write().unwrap();
}

#[allow(dead_code)]
pub fn commit_index(repo: &Repository, message: &str) -> Oid {
    let mut index = repo.index().unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = signature();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<_> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}

#[allow(dead_code)]
pub fn commit_file(repo: &Repository, file: &str, contents: &str, message: &str) -> Oid {
    let workdir = repo.workdir().unwrap();
    write_text(workdir, file, contents);
    add_path(repo, file);
    commit_index(repo, message)
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum BufferOp {
    Update(Chunk),
    Merger(u32),
}

#[allow(dead_code)]
pub struct BufferFixture {
    pub ops: Vec<BufferOp>,
    pub buffer: Buffer,
    pub window_start: usize,
    pub window_end: usize,
}

#[allow(dead_code)]
pub struct GraphFixture {
    pub buffer: Buffer,
    pub rows: Vec<GraphRow>,
    pub history: GraphHistory,
    pub head_alias: u32,
    pub theme: Theme,
    pub symbols: SymbolTheme,
}

#[allow(dead_code)]
pub struct GraphServiceFixture {
    pub _temp: TempFixture,
    pub path: PathBuf,
    pub amount: usize,
    pub hidden_branch_names: HashSet<String>,
    pub include_head_reflog_roots: bool,
    pub graph_lane_limit: usize,
    pub worktrees: Vec<WorktreeEntry>,
    pub symbols: SymbolTheme,
}

#[allow(dead_code)]
pub struct RepoWalkFixture {
    pub _temp: TempFixture,
    pub path: PathBuf,
    pub amount: usize,
    pub hidden_branch_names: HashSet<String>,
    pub include_head_reflog_roots: bool,
    pub graph_lane_limit: usize,
    pub expected_commits: usize,
    pub expected_walker_rows: usize,
}

#[allow(dead_code)]
pub fn graph_fixture(cycles: usize) -> GraphFixture {
    let mut buffer = Buffer::with_lane_limit(8);
    let mut rows = Vec::with_capacity(1 + cycles * 3);
    let mut next_alias = 1u32;

    let root = next_alias;
    next_alias += 1;
    let mut left_tip = root;
    let mut right_tip = root;

    push_commit(&mut buffer, &mut rows, 0, root, NONE, NONE, "root".to_string());

    for cycle in 0..cycles {
        let left_alias = next_alias;
        next_alias += 1;
        let left_index = rows.len();
        push_commit(&mut buffer, &mut rows, left_index, left_alias, left_tip, NONE, format!("left branch {cycle}"));

        let right_alias = next_alias;
        next_alias += 1;
        let right_index = rows.len();
        push_commit(&mut buffer, &mut rows, right_index, right_alias, right_tip, NONE, format!("right branch {cycle}"));

        let merge_alias = next_alias;
        next_alias += 1;
        let merge_index = rows.len();
        push_commit(&mut buffer, &mut rows, merge_index, merge_alias, left_alias, right_alias, format!("merge cycle {cycle}"));

        left_tip = merge_alias;
        right_tip = merge_alias;
    }

    buffer.backup();
    let history = buffer.window(1, buffer.deltas.len());
    let head_alias = rows.last().map(|row| row.alias).unwrap_or(NONE);

    GraphFixture { buffer, rows, history, head_alias, theme: Theme::classic(), symbols: SymbolTheme::main() }
}

#[allow(dead_code)]
pub fn buffer_linear_fixture(commits: usize) -> BufferFixture {
    let mut buffer = Buffer::default();
    let mut ops = Vec::with_capacity(commits);
    let mut parent = NONE;

    for alias in 1..=commits as u32 {
        let chunk = Chunk::commit(alias, parent, NONE);
        ops.push(BufferOp::Update(chunk));
        buffer.update(chunk);
        parent = alias;
    }

    buffer.backup();
    BufferFixture { ops, window_start: 1, window_end: buffer.deltas.len(), buffer }
}

#[allow(dead_code)]
pub fn buffer_merge_fixture(rounds: usize) -> BufferFixture {
    let mut buffer = Buffer::default();
    let mut ops = Vec::with_capacity(1 + rounds * 5);
    let mut next_alias = 1u32;

    let root = next_alias;
    next_alias += 1;
    let root_chunk = Chunk::commit(root, NONE, NONE);
    ops.push(BufferOp::Update(root_chunk));
    buffer.update(root_chunk);
    let mut parent = root;

    for _ in 0..rounds {
        let left = next_alias;
        next_alias += 1;
        let left_chunk = Chunk::commit(left, parent, NONE);
        ops.push(BufferOp::Update(left_chunk));
        buffer.update(left_chunk);

        let right = next_alias;
        next_alias += 1;
        let right_chunk = Chunk::commit(right, parent, NONE);
        ops.push(BufferOp::Update(right_chunk));
        buffer.update(right_chunk);

        let merge = next_alias;
        next_alias += 1;
        let merge_chunk = Chunk::commit(merge, left, right);
        ops.push(BufferOp::Update(merge_chunk));
        buffer.update(merge_chunk);

        ops.push(BufferOp::Merger(merge));
        buffer.merger(merge);

        let replay = next_alias;
        next_alias += 1;
        let replay_chunk = Chunk::commit(replay, merge, NONE);
        ops.push(BufferOp::Update(replay_chunk));
        buffer.update(replay_chunk);

        parent = replay;
    }

    buffer.backup();
    BufferFixture { ops, window_start: 1, window_end: buffer.deltas.len(), buffer }
}

#[allow(dead_code)]
pub fn buffer_checkpoint_fixture(commits: usize) -> BufferFixture {
    buffer_linear_fixture(commits)
}

#[allow(dead_code)]
pub fn graph_service_fixture(rounds: usize) -> GraphServiceFixture {
    let temp = temp_dir("graph-service");
    let path = temp.to_path_buf();
    let mut repo = Repository::init(&path).unwrap();
    configure_repo(&repo);

    let sig = signature();
    let root = commit_worktree(&mut repo, "root.txt", "root", &sig, &[]);
    repo.branch("topic", &repo.find_commit(root).unwrap(), true).unwrap();

    let mut current_base = root;
    let mut last_left = root;
    let mut last_right = root;
    let mut last_merge = root;

    for round in 0..rounds {
        let left_name = format!("left-{round}");
        let right_name = format!("right-{round}");

        repo.branch(&left_name, &repo.find_commit(current_base).unwrap(), true).unwrap();
        checkout_branch(&mut repo, &left_name);
        last_left = commit_worktree(&mut repo, &format!("left-{round}.txt"), &format!("left {round}"), &sig, &[current_base]);

        repo.branch(&right_name, &repo.find_commit(current_base).unwrap(), true).unwrap();
        checkout_branch(&mut repo, &right_name);
        last_right = commit_worktree(&mut repo, &format!("right-{round}.txt"), &format!("right {round}"), &sig, &[current_base]);

        checkout_branch(&mut repo, &left_name);
        last_merge = commit_worktree(&mut repo, &format!("merge-{round}.txt"), &format!("merge {round}"), &sig, &[last_left, last_right]);
        repo.tag_lightweight(&format!("v{round}"), &repo.find_commit(last_merge).unwrap().into_object(), false).unwrap();

        current_base = last_merge;
    }

    fs::write(path.join("stash.txt"), "stash me").unwrap();
    let _ = repo.stash_save(&sig, "bench stash", Some(StashFlags::INCLUDE_UNTRACKED));

    let worktrees = vec![
        WorktreeEntry {
            name: "main".to_string(),
            path: path.clone(),
            branch: Some("topic".to_string()),
            head: Some(git2_to_gix_oid(current_base)),
            alias: None,
            kind: WorktreeKind::Main,
            is_current: true,
            is_valid: true,
            is_prunable: false,
            locked_reason: None,
            is_dirty: false,
        },
        WorktreeEntry {
            name: "left".to_string(),
            path: path.join("wt-left"),
            branch: Some(format!("left-{}", rounds.saturating_sub(1))),
            head: Some(git2_to_gix_oid(last_left)),
            alias: None,
            kind: WorktreeKind::Linked,
            is_current: false,
            is_valid: true,
            is_prunable: false,
            locked_reason: None,
            is_dirty: false,
        },
        WorktreeEntry {
            name: "right".to_string(),
            path: path.join("wt-right"),
            branch: Some(format!("right-{}", rounds.saturating_sub(1))),
            head: Some(git2_to_gix_oid(last_right)),
            alias: None,
            kind: WorktreeKind::Linked,
            is_current: false,
            is_valid: true,
            is_prunable: false,
            locked_reason: None,
            is_dirty: false,
        },
        WorktreeEntry {
            name: "merge".to_string(),
            path: path.join("wt-merge"),
            branch: Some(format!("left-{}", rounds.saturating_sub(1))),
            head: Some(git2_to_gix_oid(last_merge)),
            alias: None,
            kind: WorktreeKind::Linked,
            is_current: false,
            is_valid: true,
            is_prunable: false,
            locked_reason: None,
            is_dirty: false,
        },
    ];

    GraphServiceFixture {
        _temp: temp,
        path,
        amount: rounds.saturating_mul(8).max(8),
        hidden_branch_names: HashSet::new(),
        include_head_reflog_roots: true,
        graph_lane_limit: 20,
        worktrees,
        symbols: SymbolTheme::main(),
    }
}

#[allow(dead_code)]
pub fn repo_walk_linear_fixture(commits: usize, amount: usize) -> RepoWalkFixture {
    let (temp, mut repo) = repo_walk_repo("repo-walk-linear");
    let sig = signature();
    let mut parent = commit_worktree(&mut repo, "root.txt", "root", &sig, &[]);

    for index in 1..commits {
        parent = commit_worktree(&mut repo, &format!("linear-{index}.txt"), &format!("linear {index}"), &sig, &[parent]);
    }

    repo_walk_fixture(temp, amount, HashSet::new(), commits)
}

#[allow(dead_code)]
pub fn repo_walk_many_refs_fixture(commits: usize, refs: usize, amount: usize) -> RepoWalkFixture {
    let (temp, mut repo) = repo_walk_repo("repo-walk-many-refs");
    let sig = signature();
    let mut tips = Vec::with_capacity(commits);
    let mut parent = commit_worktree(&mut repo, "root.txt", "root", &sig, &[]);
    tips.push(parent);

    for index in 1..commits {
        parent = commit_worktree(&mut repo, &format!("linear-{index}.txt"), &format!("linear {index}"), &sig, &[parent]);
        tips.push(parent);
    }

    for index in 0..refs {
        let oid = tips[index % tips.len()];
        repo.branch(&format!("bench/{index:03}"), &repo.find_commit(oid).unwrap(), true).unwrap();
    }

    repo_walk_fixture(temp, amount, HashSet::new(), commits)
}

#[allow(dead_code)]
pub fn repo_walk_hidden_branches_fixture(visible_commits: usize, hidden_branches: usize, hidden_commits: usize, amount: usize) -> RepoWalkFixture {
    let (temp, mut repo) = repo_walk_repo("repo-walk-hidden");
    let sig = signature();
    let root = commit_worktree(&mut repo, "root.txt", "root", &sig, &[]);
    let mut visible_parent = root;

    for index in 1..visible_commits {
        visible_parent = commit_worktree(&mut repo, &format!("visible-{index}.txt"), &format!("visible {index}"), &sig, &[visible_parent]);
    }
    let visible_branch = repo.head().unwrap().shorthand().unwrap().to_string();

    let mut hidden_branch_names = HashSet::new();
    for branch in 0..hidden_branches {
        let branch_name = format!("hidden/{branch:03}");
        hidden_branch_names.insert(branch_name.clone());
        repo.branch(&branch_name, &repo.find_commit(root).unwrap(), true).unwrap();
        checkout_branch(&mut repo, &branch_name);

        let mut hidden_parent = root;
        for index in 0..hidden_commits {
            hidden_parent = commit_worktree(&mut repo, &format!("hidden-{branch}-{index}.txt"), &format!("hidden {branch} {index}"), &sig, &[hidden_parent]);
        }
    }

    checkout_branch(&mut repo, &visible_branch);
    repo_walk_fixture(temp, amount, hidden_branch_names, visible_commits)
}

#[allow(dead_code)]
pub fn repo_walk_merge_fixture(rounds: usize, amount: usize) -> RepoWalkFixture {
    let fixture = graph_service_fixture(rounds);

    RepoWalkFixture {
        _temp: fixture._temp,
        path: fixture.path,
        amount,
        hidden_branch_names: fixture.hidden_branch_names,
        include_head_reflog_roots: fixture.include_head_reflog_roots,
        graph_lane_limit: fixture.graph_lane_limit,
        expected_commits: rounds.saturating_mul(3).saturating_add(1),
        expected_walker_rows: rounds.saturating_mul(3).saturating_add(3),
    }
}

#[allow(dead_code)]
pub fn apply_buffer_ops(ops: &[BufferOp]) -> Buffer {
    let mut buffer = Buffer::default();

    for op in ops {
        match op {
            BufferOp::Update(chunk) => {
                buffer.update(*chunk);
            },
            BufferOp::Merger(alias) => buffer.merger(*alias),
        }
    }

    buffer
}

#[allow(dead_code)]
fn push_commit(buffer: &mut Buffer, rows: &mut Vec<GraphRow>, index: usize, alias: u32, parent_a: u32, parent_b: u32, summary: String) {
    buffer.update(Chunk::commit(alias, parent_a, parent_b));

    let mut row = GraphRow {
        index,
        alias,
        oid: Oid::zero(),
        summary,
        committer_date: "2026-06-20 12:34".to_string(),
        committer_name: "Benchmark Runner".to_string(),
        is_merge: false,
        has_any_branch: false,
        branches: GraphBranchLabels::new(),
        tags: GraphTagLabels::new(),
        is_stash: false,
        stash_lane: None,
        worktrees: GraphWorktrees::new(),
        has_current_worktree: false,
        reflog: None,
    };

    if index % 5 == 0 {
        row.has_any_branch = true;
        row.branches.push(GraphBranchLabel { name: format!("bench/branch-{alias}"), is_local: true, lane: None });
    }

    if index % 8 == 0 {
        row.tags.push(GraphTagLabel { name: format!("v{alias}"), lane: None });
    }

    rows.push(row);
}

#[allow(dead_code)]
fn configure_repo(repo: &Repository) {
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Benchmark Runner").unwrap();
    config.set_str("user.email", "bench@example.com").unwrap();
}

#[allow(dead_code)]
fn signature() -> Signature<'static> {
    Signature::now("Benchmark Runner", "bench@example.com").unwrap()
}

#[allow(dead_code)]
fn checkout_branch(repo: &mut Repository, branch: &str) {
    repo.set_head(&format!("refs/heads/{branch}")).unwrap();
    repo.checkout_head(Some(CheckoutBuilder::default().force())).unwrap();
}

#[allow(dead_code)]
fn repo_walk_repo(name: &str) -> (TempFixture, Repository) {
    let temp = temp_dir(name);
    let repo = Repository::init(temp.path()).unwrap();
    configure_repo(&repo);
    (temp, repo)
}

#[allow(dead_code)]
fn repo_walk_fixture(temp: TempFixture, amount: usize, hidden_branch_names: HashSet<String>, expected_commits: usize) -> RepoWalkFixture {
    RepoWalkFixture {
        path: temp.to_path_buf(),
        _temp: temp,
        amount,
        hidden_branch_names,
        include_head_reflog_roots: false,
        graph_lane_limit: 20,
        expected_commits,
        expected_walker_rows: expected_commits.saturating_add(1),
    }
}

#[allow(dead_code)]
fn commit_worktree(repo: &mut Repository, filename: &str, contents: &str, sig: &Signature<'_>, parents: &[Oid]) -> Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(filename), contents).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new(filename)).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let parent_commits: Vec<_> = parents.iter().map(|oid| repo.find_commit(*oid).unwrap()).collect();
    let parent_refs: Vec<_> = parent_commits.iter().collect();
    repo.commit(Some("HEAD"), sig, sig, contents, &tree, &parent_refs).unwrap()
}
