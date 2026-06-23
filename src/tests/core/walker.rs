use super::*;
use crate::{
    core::{graph_service::GraphRow, oids::Oids, renderers::render_graph_projection},
    git::actions::worktrees::create_worktree,
    git::queries::{
        commits::{get_stashed_commits, get_tag_oids, get_tip_oids},
        reflogs::get_head_reflog_entries,
    },
    helpers::{
        palette::Theme,
        symbols::{SymbolTheme, graph},
    },
};
use git2::{ObjectType, Oid, Repository, ResetType, Signature, Time};
use gix::traverse::commit::topo::{Builder as GixTopoBuilder, Sorting as GixTopoSorting};
use ratatui::text::Line;
use std::{
    collections::{HashMap, HashSet as StdHashSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-walker-reflog-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn commit(repo: &Repository, file: &str, message: &str) -> Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), message).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
}

fn stash_tracked_change(repo: &mut Repository, file: &str, message: &str) -> Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), message).unwrap();
    let sig = repo.signature().unwrap();
    repo.stash_save(&sig, message, None).unwrap()
}

fn commit_with_parents(repo: &Repository, file: &str, message: &str, parents: &[Oid], time: i64) -> Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join(file), message).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::new("Test User", "test@example.com", &Time::new(time, 0)).unwrap();
    let parent_commits: Vec<_> = parents.iter().map(|oid| repo.find_commit(*oid).unwrap()).collect();
    let parent_refs: Vec<&git2::Commit<'_>> = parent_commits.iter().collect();
    repo.commit(None, &sig, &sig, message, &tree, &parent_refs).unwrap()
}

fn graph_row(index: usize, alias: u32, oid: Oid) -> GraphRow {
    GraphRow {
        index,
        alias,
        oid,
        short_oid: oid.to_string()[..9].to_string(),
        summary: String::new(),
        committer_date: String::new(),
        committer_name: String::new(),
        has_any_branch: false,
        branches: Vec::new(),
        tags: Vec::new(),
        is_stash: false,
        stash_lane: None,
        worktrees: Vec::new(),
        has_current_worktree: false,
        reflog: None,
    }
}

fn line_text(line: &Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommitMetadata {
    summary: String,
    committer_name: String,
    committer_seconds: i64,
}

fn commit_metadata_from_repo(repo: &gix::Repository, oid: gix::ObjectId) -> CommitMetadata {
    let commit = repo.find_commit(oid).unwrap();
    let message = commit.message().unwrap();
    let committer = commit.committer().unwrap();

    CommitMetadata {
        summary: String::from_utf8_lossy(message.summary().as_ref()).into_owned(),
        committer_name: String::from_utf8_lossy(committer.name.as_ref()).into_owned(),
        committer_seconds: committer.time().unwrap().seconds,
    }
}

fn collect_root_oids(repo: &mut Repository, include_head_reflog_roots: bool) -> Vec<Oid> {
    let mut oids = Oids::default();
    let gix_repo = gix::open(repo.workdir().unwrap_or(repo.path())).unwrap();
    let (branches_local, branches_remote) = get_tip_oids(&gix_repo, &mut oids);
    let tags_local = tag_oids_via_libgit2(repo, &mut oids);
    let stashes = get_stashed_commits(&gix_repo, &mut oids);
    let mut aliases: StdHashSet<u32> = branches_local.keys().copied().chain(branches_remote.keys().copied()).chain(tags_local.keys().copied()).chain(stashes).collect();

    if include_head_reflog_roots {
        for entry in get_head_reflog_entries(&gix_repo).unwrap_or_default() {
            aliases.insert(oids.get_alias_by_oid(entry.new_oid));
        }
    }

    let mut roots: Vec<_> = aliases.into_iter().map(|alias| *oids.get_oid_by_alias(alias)).collect();
    roots.sort_unstable_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    roots
}

fn tag_oids_via_libgit2(repo: &Repository, oids: &mut Oids) -> HashMap<u32, Vec<String>> {
    repo.references()
        .unwrap()
        .flatten()
        .filter_map(|reference| {
            let tag = reference.name()?.strip_prefix("refs/tags/")?;
            let commit_oid = reference.peel(ObjectType::Commit).ok()?.id();
            Some((oids.get_alias_by_oid(commit_oid), tag.to_string()))
        })
        .fold(HashMap::new(), |mut tags, (alias, tag)| {
            tags.entry(alias).or_default().push(tag);
            tags
        })
}

fn git2_stash_root_oids(repo: &mut Repository) -> StdHashSet<Oid> {
    let mut oids = Oids::default();
    let gix_repo = gix::open(repo.workdir().unwrap_or(repo.path())).unwrap();
    get_stashed_commits(&gix_repo, &mut oids).into_iter().map(|alias| *oids.get_oid_by_alias(alias)).collect()
}

fn git2_head_reflog_root_oids(repo: &Repository) -> StdHashSet<Oid> {
    let gix_repo = gix::open(repo.workdir().unwrap_or(repo.path())).unwrap();
    get_head_reflog_entries(&gix_repo).unwrap_or_default().into_iter().map(|entry| entry.new_oid).collect()
}

fn gitoxide_stash_root_oids(repo: &gix::Repository) -> StdHashSet<Oid> {
    repo.try_find_reference("refs/stash").ok().flatten().and_then(|reference| reference.try_id().map(|id| Oid::from_bytes(id.as_bytes()).unwrap())).into_iter().collect()
}

fn gitoxide_head_reflog_root_oids(repo: &gix::Repository) -> StdHashSet<Oid> {
    repo.head().unwrap().log_iter().all().unwrap().into_iter().flat_map(|logs| logs.filter_map(Result::ok)).map(|line| Oid::from_bytes(line.new_oid().as_bytes()).unwrap()).collect()
}

fn special_roots_fixture(name: &str) -> (PathBuf, Repository) {
    let (path, mut repo) = temp_repo(name);
    let base = commit(&repo, "file.txt", "base");
    let tip = commit(&repo, "file.txt", "tip");
    let base_commit = repo.find_commit(base).unwrap();
    repo.reset(base_commit.as_object(), ResetType::Hard, None).unwrap();
    drop(base_commit);
    let _stash = stash_tracked_change(&mut repo, "file.txt", "stashed change");

    assert_ne!(base, tip);
    (path, repo)
}

fn graph_metadata_from_walker(walker: &Walker) -> HashMap<Oid, CommitMetadata> {
    walker
        .oids
        .get_sorted_aliases()
        .iter()
        .filter_map(|&alias| {
            let oid = *walker.oids.get_oid_by_alias(alias);
            (!walker.oids.is_zero(&oid)).then(|| {
                let oid = gix::ObjectId::from_bytes_or_panic(oid.as_bytes());
                (gix_to_git2_oid(oid), commit_metadata_from_repo(&walker.gix_repo, oid))
            })
        })
        .collect()
}

fn graph_metadata_from_current_backend(path: &Path) -> HashMap<Oid, CommitMetadata> {
    let mut walker = Walker::new(path.display().to_string(), 10_000, HashSet::new(), true, 20).unwrap();
    while walker.walk() {}
    graph_metadata_from_walker(&walker)
}

fn graph_metadata_from_gitoxide(path: &Path, roots: &[Oid]) -> HashMap<Oid, CommitMetadata> {
    let repo = gix::open(path).unwrap();
    let tips = roots.iter().map(|oid| gix::ObjectId::from_bytes_or_panic(oid.as_bytes())).collect::<Vec<_>>();
    let topo = GixTopoBuilder::new(&repo.objects).with_tips(tips).sorting(GixTopoSorting::TopoOrder).build().unwrap();

    topo.map(|result| {
        let info = result.unwrap();
        let oid = Oid::from_bytes(info.id.as_slice()).unwrap();
        (oid, commit_metadata_from_repo(&repo, info.id))
    })
    .collect()
}

fn representative_graph_fixture(name: &str, tail_commits: usize) -> (PathBuf, Repository) {
    let (path, repo) = temp_repo(name);
    let root = commit(&repo, "root.txt", "root");
    let main_1 = commit_with_parents(&repo, "main.txt", "main-1", &[root], 1);
    let side_1 = commit_with_parents(&repo, "side.txt", "side-1", &[root], 2);
    let main_2 = commit_with_parents(&repo, "main.txt", "main-2", &[main_1], 3);
    let side_2 = commit_with_parents(&repo, "side.txt", "side-2", &[side_1], 4);
    let merge = commit_with_parents(&repo, "merge.txt", "merge", &[main_2, side_2], 5);

    let mut tip = commit_with_parents(&repo, "tail.txt", "tail-0", &[merge], 6);
    for idx in 1..tail_commits {
        tip = commit_with_parents(&repo, "tail.txt", &format!("tail-{idx}"), &[tip], 6 + idx as i64);
    }

    repo.reference("refs/heads/main", tip, true, "test").unwrap();
    repo.reference("refs/heads/feature", side_2, true, "test").unwrap();
    repo.reference("refs/tags/v-main", main_1, true, "test").unwrap();
    repo.set_head("refs/heads/main").unwrap();

    (path, repo)
}

fn linked_worktree_startup_fixture(name: &str) -> (PathBuf, PathBuf, Repository) {
    let (path, repo) = representative_graph_fixture(name, 32);
    let head = repo.head().unwrap().target().unwrap();
    let linked_path = path.parent().unwrap_or_else(|| Path::new(".")).join(format!("{}-linked", path.file_name().and_then(|name| name.to_str()).unwrap_or("repo")));
    create_worktree(&repo, "linked", &linked_path, head).unwrap();
    (path, linked_path, repo)
}

#[test]
fn walker_loads_commit_reachable_only_from_head_reflog() {
    let (path, repo) = temp_repo("lost-root");
    let base = commit(&repo, "file.txt", "base");
    let lost = commit(&repo, "file.txt", "lost");
    let base_commit = repo.find_commit(base).unwrap();
    repo.reset(base_commit.as_object(), ResetType::Hard, None).unwrap();

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), true, 20).unwrap();
    walker.walk();
    let lost_alias = walker.oids.get_existing_alias(lost).unwrap();

    assert!(walker.oids.get_sorted_aliases().contains(&lost_alias));
}

#[test]
fn walker_can_hide_commit_reachable_only_from_head_reflog() {
    let (path, repo) = temp_repo("hidden-lost-root");
    let base = commit(&repo, "file.txt", "base");
    let lost = commit(&repo, "file.txt", "lost");
    let base_commit = repo.find_commit(base).unwrap();
    repo.reset(base_commit.as_object(), ResetType::Hard, None).unwrap();

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), false, 20).unwrap();
    walker.walk();
    let lost_alias = walker.oids.get_existing_alias(lost).unwrap();

    assert!(!walker.oids.get_sorted_aliases().contains(&lost_alias));
    assert!(walker.head_reflog_entries.iter().any(|entry| entry.new_oid == lost));
}

#[test]
fn walker_expires_new_right_merge_lane_before_next_rendered_row() {
    let (path, repo) = temp_repo("transient-merge-lane");
    let root = commit_with_parents(&repo, "root.txt", "root", &[], 1);
    let left_parent = commit_with_parents(&repo, "left-parent.txt", "left parent", &[root], 2);
    let right_parent = commit_with_parents(&repo, "right-parent.txt", "right parent", &[root], 3);
    let merge = commit_with_parents(&repo, "merge.txt", "merge", &[left_parent, right_parent], 4);
    let right_tip = commit_with_parents(&repo, "right-tip.txt", "right tip", &[right_parent], 5);
    let left_tip = commit_with_parents(&repo, "left-tip.txt", "left tip", &[left_parent], 6);

    repo.reference("refs/heads/main", left_tip, true, "test").unwrap();
    repo.reference("refs/heads/right", right_tip, true, "test").unwrap();
    repo.reference("refs/heads/merge", merge, true, "test").unwrap();
    repo.set_head("refs/heads/main").unwrap();

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), false, 20).unwrap();
    while walker.walk() {}

    let merge_alias = walker.oids.get_existing_alias(merge).unwrap();
    let head_alias = walker.oids.get_existing_alias(left_tip).unwrap();
    let aliases = walker.oids.get_sorted_aliases().clone();
    let merge_idx = aliases.iter().position(|alias| *alias == merge_alias).unwrap();
    assert!(merge_idx + 1 < aliases.len());

    let history = walker.buffer.borrow().window(0, aliases.len().saturating_add(1));
    let merge_history_idx = merge_idx;
    let merge_lane = history.get(merge_history_idx).unwrap().iter().position(|chunk| chunk.alias == merge_alias).unwrap();

    assert_eq!(merge_lane + 1, history.get(merge_history_idx).unwrap().len());
    assert!(history.get(merge_history_idx + 1).unwrap().get(merge_lane).is_none());

    let rows: Vec<_> = aliases.iter().enumerate().map(|(index, &alias)| graph_row(index, alias, *walker.oids.get_oid_by_alias(alias))).collect();
    let symbols = SymbolTheme::main();
    let lines = render_graph_projection(&Theme::classic(), &symbols, &rows, &history, head_alias, 0, aliases.len(), true);
    let merge_text = line_text(&lines[merge_idx]);
    let next_text = line_text(&lines[merge_idx + 1]);
    let merge_col = merge_text.chars().position(|ch| ch == graph::MERGE.chars().next().unwrap()).unwrap();

    assert_ne!(next_text.chars().nth(merge_col), graph::VERTICAL.chars().next());
}

#[test]
fn walker_records_ref_stash_and_reflog_lanes_from_update_lane() {
    let (path, mut repo) = temp_repo("cached-lanes");
    let base = commit(&repo, "file.txt", "base");
    {
        let base_commit = repo.find_commit(base).unwrap();
        repo.tag_lightweight("v-base", base_commit.as_object(), false).unwrap();
    }
    let stash = stash_tracked_change(&mut repo, "file.txt", "stashed change");

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), true, 20).unwrap();
    while walker.walk() {}

    let base_alias = walker.oids.get_existing_alias(base).unwrap();
    let stash_alias = walker.oids.get_existing_alias(stash).unwrap();

    assert!(walker.branches_lanes.contains_key(&base_alias));
    assert!(walker.tags_lanes.contains_key(&base_alias));
    assert!(walker.reflogs_lanes.contains_key(&base_alias));
    assert!(walker.stashes_lanes.contains_key(&stash_alias));
}

#[test]
fn walker_new_collects_startup_metadata_before_walking() {
    let (path, mut repo) = representative_graph_fixture("startup-metadata", 32);
    let stash = stash_tracked_change(&mut repo, "tail.txt", "stashed change");

    let walker = Walker::new(path.display().to_string(), 100, HashSet::new(), true, 20).unwrap();

    assert!(walker.branches_local.values().flatten().any(|name| name == "main"));
    assert!(walker.branches_local.values().flatten().any(|name| name == "feature"));
    assert!(walker.tags_local.values().flatten().any(|name| name == "v-main"));
    assert!(!walker.oids.stashes.is_empty());
    assert!(walker.oids.get_existing_alias(stash).is_some());
    assert!(!walker.head_reflog_entries.is_empty());
}

#[test]
fn walker_matches_rev_list_all_by_walking_tag_only_commits() {
    let (path, repo) = temp_repo("tag-only-root");
    let root = commit(&repo, "root.txt", "root");
    let branch_tip = commit(&repo, "branch.txt", "branch");
    let tag_only = commit_with_parents(&repo, "tag-only.txt", "tag-only", &[], 99);
    let tag_only_commit = repo.find_commit(tag_only).unwrap();
    repo.tag_lightweight("tag-only", tag_only_commit.as_object(), false).unwrap();
    repo.reference("refs/heads/main", branch_tip, true, "test").unwrap();
    repo.set_head("refs/heads/main").unwrap();

    let mut walker = Walker::new(path.display().to_string(), 1, HashSet::new(), false, 20).unwrap();
    while walker.walk() {}

    let sorted_oids: StdHashSet<Oid> = walker.oids.get_sorted_aliases().iter().map(|alias| *walker.oids.get_oid_by_alias(*alias)).filter(|oid| !walker.oids.is_zero(oid)).collect();

    assert_eq!(sorted_oids, StdHashSet::from([root, branch_tip, tag_only]));
}

#[test]
fn walker_matches_rev_list_all_for_annotated_tags() {
    let (path, repo) = temp_repo("annotated-tag-root");
    let root = commit(&repo, "root.txt", "root");
    let branch_tip = commit(&repo, "branch.txt", "branch");
    let tagged = commit_with_parents(&repo, "tagged.txt", "tagged", &[], 100);
    let tagged_commit = repo.find_commit(tagged).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    repo.tag("annotated", tagged_commit.as_object(), &sig, "annotated", false).unwrap();
    repo.reference("refs/heads/main", branch_tip, true, "test").unwrap();
    repo.set_head("refs/heads/main").unwrap();

    let mut walker = Walker::new(path.display().to_string(), 1, HashSet::new(), false, 20).unwrap();
    while walker.walk() {}

    let sorted_oids: StdHashSet<Oid> = walker.oids.get_sorted_aliases().iter().map(|alias| *walker.oids.get_oid_by_alias(*alias)).filter(|oid| !walker.oids.is_zero(oid)).collect();

    assert_eq!(sorted_oids, StdHashSet::from([root, branch_tip, tagged]));
}

#[test]
fn gix_tag_oids_resolves_tags_without_collecting_other_refs() {
    let (path, repo) = temp_repo("gix-tag-oids");
    let base = commit(&repo, "base.txt", "base");
    let tagged = commit_with_parents(&repo, "tagged.txt", "tagged", &[], 100);
    let base_commit = repo.find_commit(base).unwrap();
    let tagged_commit = repo.find_commit(tagged).unwrap();
    let tree = base_commit.tree().unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();

    repo.tag_lightweight("lightweight", base_commit.as_object(), false).unwrap();
    repo.tag("annotated", tagged_commit.as_object(), &sig, "annotated", false).unwrap();
    repo.tag_lightweight("tree-tag", tree.as_object(), false).unwrap();
    repo.reference("refs/notes/not-a-tag", tagged, true, "test").unwrap();

    let mut oids = Oids::default();
    let gix_repo = gix::open(path).unwrap();
    let tags = get_tag_oids(&gix_repo, &mut oids);

    let base_alias = oids.get_existing_alias(base).unwrap();
    let tagged_alias = oids.get_existing_alias(tagged).unwrap();
    let names = tags.values().flatten().cloned().collect::<StdHashSet<_>>();

    assert_eq!(tags.get(&base_alias).unwrap(), &vec!["lightweight".to_string()]);
    assert_eq!(tags.get(&tagged_alias).unwrap(), &vec!["annotated".to_string()]);
    assert!(!names.contains("tree-tag"));
    assert!(!names.contains("not-a-tag"));
}

#[test]
fn walker_new_handles_linked_worktree_startup_paths() {
    let (_repo_path, linked_path, repo) = linked_worktree_startup_fixture("startup-linked-worktree");

    let walker = Walker::new(linked_path.display().to_string(), 100, HashSet::new(), true, 20).unwrap();

    assert!(walker.gix_repo.worktree().is_some());
    assert_eq!(walker.gix_repo.common_dir(), repo.commondir());
    assert!(walker.branches_local.values().flatten().any(|name| name == "main"));
    assert!(walker.tags_local.values().flatten().any(|name| name == "v-main"));
}

#[test]
fn walker_keeps_stash_adjacent_to_its_base_parent() {
    let (path, mut repo) = temp_repo("stash-order");
    let base = commit(&repo, "file.txt", "base");
    let stash = stash_tracked_change(&mut repo, "file.txt", "stashed change");

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), false, 20).unwrap();
    while walker.walk() {}

    let aliases = walker.oids.get_sorted_aliases();
    let base_alias = walker.oids.get_existing_alias(base).unwrap();
    let stash_alias = walker.oids.get_existing_alias(stash).unwrap();
    let base_idx = aliases.iter().position(|alias| *alias == base_alias).unwrap();
    let stash_idx = aliases.iter().position(|alias| *alias == stash_alias).unwrap();

    assert_eq!(stash_idx + 1, base_idx);
}

#[test]
fn gitoxide_reproduces_stash_and_head_reflog_roots() {
    let (path, mut repo) = special_roots_fixture("special-root-parity");
    let git2_stash_roots = git2_stash_root_oids(&mut repo);
    let git2_head_reflog_roots = git2_head_reflog_root_oids(&repo);

    let gix_repo = gix::open(&path).unwrap();
    let gix_stash_roots = gitoxide_stash_root_oids(&gix_repo);
    let gix_head_reflog_roots = gitoxide_head_reflog_root_oids(&gix_repo);

    assert!(!git2_stash_roots.is_empty());
    assert!(!git2_head_reflog_roots.is_empty());
    assert_eq!(git2_stash_roots, gix_stash_roots);
    assert_eq!(git2_head_reflog_roots, gix_head_reflog_roots);
}

#[test]
fn gitoxide_topo_walk_matches_current_graph_metadata_on_representative_history() {
    let (path, mut repo) = representative_graph_fixture("gitoxide-parity", 32);
    let roots = collect_root_oids(&mut repo, true);
    let current = graph_metadata_from_current_backend(&path);
    let gitoxide = graph_metadata_from_gitoxide(&path, &roots);

    assert_eq!(current, gitoxide);
}
