use super::*;
use crate::{
    core::{graph_service::GraphRow, renderers::render_graph_projection},
    helpers::{palette::Theme, symbols::graph},
};
use git2::{Oid, ResetType, Signature, Time};
use ratatui::text::Line;
use std::{
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
        summary: String::new(),
        committer_date: String::new(),
        committer_name: String::new(),
        has_any_branch: false,
        branches: Vec::new(),
        tags: Vec::new(),
        is_stash: false,
        stash_lane: None,
        worktrees: Vec::new(),
        reflog: None,
    }
}

fn line_text(line: &Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect()
}

#[test]
fn walker_loads_commit_reachable_only_from_head_reflog() {
    let (path, repo) = temp_repo("lost-root");
    let base = commit(&repo, "file.txt", "base");
    let lost = commit(&repo, "file.txt", "lost");
    let base_commit = repo.find_commit(base).unwrap();
    repo.reset(base_commit.as_object(), ResetType::Hard, None).unwrap();

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), true).unwrap();
    walker.walk();
    let lost_alias = walker.oids.aliases.get(&lost).copied().unwrap();

    assert!(walker.oids.get_sorted_aliases().contains(&lost_alias));
}

#[test]
fn walker_can_hide_commit_reachable_only_from_head_reflog() {
    let (path, repo) = temp_repo("hidden-lost-root");
    let base = commit(&repo, "file.txt", "base");
    let lost = commit(&repo, "file.txt", "lost");
    let base_commit = repo.find_commit(base).unwrap();
    repo.reset(base_commit.as_object(), ResetType::Hard, None).unwrap();

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), false).unwrap();
    walker.walk();
    let lost_alias = walker.oids.aliases.get(&lost).copied().unwrap();

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

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), false).unwrap();
    while walker.walk() {}

    let merge_alias = walker.oids.aliases.get(&merge).copied().unwrap();
    let head_alias = walker.oids.aliases.get(&left_tip).copied().unwrap();
    let aliases = walker.oids.get_sorted_aliases().clone();
    let merge_idx = aliases.iter().position(|alias| *alias == merge_alias).unwrap();
    assert!(merge_idx + 1 < aliases.len());

    let history = walker.buffer.borrow().window(0, aliases.len().saturating_add(1));
    let merge_history_idx = merge_idx;
    let merge_lane = history[merge_history_idx].iter().position(|chunk| chunk.alias == merge_alias).unwrap();

    assert_eq!(merge_lane + 1, history[merge_history_idx].len());
    assert!(history[merge_history_idx + 1].get(merge_lane).is_none());

    let rows: Vec<_> = aliases.iter().enumerate().map(|(index, &alias)| graph_row(index, alias, *walker.oids.get_oid_by_alias(alias))).collect();
    let lines = render_graph_projection(&Theme::classic(), &rows, &history, head_alias, 0, aliases.len(), true);
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

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), true).unwrap();
    while walker.walk() {}

    let base_alias = walker.oids.aliases.get(&base).copied().unwrap();
    let stash_alias = walker.oids.aliases.get(&stash).copied().unwrap();

    assert!(walker.branches_lanes.contains_key(&base_alias));
    assert!(walker.tags_lanes.contains_key(&base_alias));
    assert!(walker.reflogs_lanes.contains_key(&base_alias));
    assert!(walker.stashes_lanes.contains_key(&stash_alias));
}

#[test]
fn walker_keeps_stash_adjacent_to_its_base_parent() {
    let (path, mut repo) = temp_repo("stash-order");
    let base = commit(&repo, "file.txt", "base");
    let stash = stash_tracked_change(&mut repo, "file.txt", "stashed change");

    let mut walker = Walker::new(path.display().to_string(), 100, HashSet::new(), false).unwrap();
    while walker.walk() {}

    let aliases = walker.oids.get_sorted_aliases();
    let base_alias = walker.oids.aliases.get(&base).copied().unwrap();
    let stash_alias = walker.oids.aliases.get(&stash).copied().unwrap();
    let base_idx = aliases.iter().position(|alias| *alias == base_alias).unwrap();
    let stash_idx = aliases.iter().position(|alias| *alias == stash_alias).unwrap();

    assert_eq!(stash_idx + 1, base_idx);
}
