use super::*;
use crate::{
    core::{
        graph_service::GraphRow,
        oids::{Oids, gix_to_git2_oid},
        renderers::render_graph_projection,
    },
    git::actions::worktrees::create_worktree,
    git::queries::commits::get_tag_oids,
    git::test_support::{TestDir, commit_file, commit_file_with_parents, stash_tracked_change, temp_repo},
    helpers::{
        palette::Theme,
        symbols::{SymbolTheme, graph},
    },
};
use git2::{Oid, Repository, ResetType, Signature};
use ratatui::text::Line;
use std::{
    collections::HashSet as StdHashSet,
    path::{Path, PathBuf},
};

fn graph_row(index: usize, alias: u32, oid: Oid) -> GraphRow {
    GraphRow {
        index,
        alias,
        oid,
        summary: String::new(),
        committer_date: String::new(),
        committer_name: String::new(),
        is_merge: false,
        has_any_branch: false,
        branches: Default::default(),
        tags: Default::default(),
        is_stash: false,
        stash_lane: None,
        worktrees: Default::default(),
        has_current_worktree: false,
        reflog: None,
    }
}

fn line_text(line: &Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect()
}

fn walked_walker(path: &Path, buffer_size: usize, include_head_reflog_roots: bool) -> Walker {
    let mut walker = Walker::new(path.display().to_string(), buffer_size, HashSet::new(), include_head_reflog_roots, 20).unwrap();
    while walker.walk() {}
    walker
}

fn representative_graph_fixture(name: &str, tail_commits: usize) -> (TestDir, PathBuf, Repository) {
    let (dir, repo) = temp_repo(name);
    let path = dir.join("repo");
    let root = commit_file(&repo, "root.txt", "root", "root");
    let main_1 = commit_file_with_parents(&repo, "main.txt", "main-1", "main-1", &[root], Some(1));
    let side_1 = commit_file_with_parents(&repo, "side.txt", "side-1", "side-1", &[root], Some(2));
    let main_2 = commit_file_with_parents(&repo, "main.txt", "main-2", "main-2", &[main_1], Some(3));
    let side_2 = commit_file_with_parents(&repo, "side.txt", "side-2", "side-2", &[side_1], Some(4));
    let merge = commit_file_with_parents(&repo, "merge.txt", "merge", "merge", &[main_2, side_2], Some(5));

    let mut tip = commit_file_with_parents(&repo, "tail.txt", "tail-0", "tail-0", &[merge], Some(6));
    for idx in 1..tail_commits {
        let value = format!("tail-{idx}");
        tip = commit_file_with_parents(&repo, "tail.txt", &value, &value, &[tip], Some(6 + idx as i64));
    }

    repo.reference("refs/heads/main", tip, true, "test").unwrap();
    repo.reference("refs/heads/feature", side_2, true, "test").unwrap();
    repo.reference("refs/tags/v-main", main_1, true, "test").unwrap();
    repo.set_head("refs/heads/main").unwrap();

    (dir, path, repo)
}

fn linked_worktree_startup_fixture(name: &str) -> (TestDir, PathBuf, PathBuf, Repository) {
    let (dir, path, repo) = representative_graph_fixture(name, 32);
    let head = repo.head().unwrap().target().unwrap();
    let linked_path = path.parent().unwrap_or_else(|| Path::new(".")).join(format!("{}-linked", path.file_name().and_then(|name| name.to_str()).unwrap_or("repo")));
    create_worktree(&repo, "linked", &linked_path, head).unwrap();
    (dir, path, linked_path, repo)
}

#[test]
fn walker_loads_commit_reachable_only_from_head_reflog() {
    let (dir, repo) = temp_repo("lost-root");
    let path = dir.join("repo");
    let base = commit_file(&repo, "file.txt", "base", "base");
    let lost = commit_file(&repo, "file.txt", "lost", "lost");
    let base_commit = repo.find_commit(base).unwrap();
    repo.reset(base_commit.as_object(), ResetType::Hard, None).unwrap();

    let walker = walked_walker(&path, 100, true);
    let lost_alias = walker.oids.get_existing_alias(lost).unwrap();

    assert!(walker.oids.get_sorted_aliases().contains(&lost_alias));
}

#[test]
fn walker_can_hide_commit_reachable_only_from_head_reflog() {
    let (dir, repo) = temp_repo("hidden-lost-root");
    let path = dir.join("repo");
    let base = commit_file(&repo, "file.txt", "base", "base");
    let lost = commit_file(&repo, "file.txt", "lost", "lost");
    let base_commit = repo.find_commit(base).unwrap();
    repo.reset(base_commit.as_object(), ResetType::Hard, None).unwrap();

    let walker = walked_walker(&path, 100, false);
    let lost_alias = walker.oids.get_existing_alias(lost).unwrap();

    assert!(!walker.oids.get_sorted_aliases().contains(&lost_alias));
    assert!(walker.head_reflog_entries.iter().any(|entry| gix_to_git2_oid(entry.new_oid) == lost));
}

#[test]
fn walker_expires_new_right_merge_lane_before_next_rendered_row() {
    let (dir, repo) = temp_repo("transient-merge-lane");
    let path = dir.join("repo");
    let root = commit_file_with_parents(&repo, "root.txt", "root", "root", &[], Some(1));
    let left_parent = commit_file_with_parents(&repo, "left-parent.txt", "left parent", "left parent", &[root], Some(2));
    let right_parent = commit_file_with_parents(&repo, "right-parent.txt", "right parent", "right parent", &[root], Some(3));
    let merge = commit_file_with_parents(&repo, "merge.txt", "merge", "merge", &[left_parent, right_parent], Some(4));
    let right_tip = commit_file_with_parents(&repo, "right-tip.txt", "right tip", "right tip", &[right_parent], Some(5));
    let left_tip = commit_file_with_parents(&repo, "left-tip.txt", "left tip", "left tip", &[left_parent], Some(6));

    repo.reference("refs/heads/main", left_tip, true, "test").unwrap();
    repo.reference("refs/heads/right", right_tip, true, "test").unwrap();
    repo.reference("refs/heads/merge", merge, true, "test").unwrap();
    repo.set_head("refs/heads/main").unwrap();

    let walker = walked_walker(&path, 100, false);

    let merge_alias = walker.oids.get_existing_alias(merge).unwrap();
    let head_alias = walker.oids.get_existing_alias(left_tip).unwrap();
    let aliases = walker.oids.get_sorted_aliases();
    let merge_idx = aliases.iter().position(|alias| *alias == merge_alias).unwrap();
    assert!(merge_idx + 1 < aliases.len());

    let history = walker.buffer.borrow().window(0, aliases.len().saturating_add(1));
    let merge_history_idx = merge_idx;
    let merge_lane = history.get(merge_history_idx).unwrap().iter().position(|chunk| chunk.alias == merge_alias).unwrap();

    assert_eq!(merge_lane + 1, history.get(merge_history_idx).unwrap().len());
    assert!(history.get(merge_history_idx + 1).unwrap().get(merge_lane).is_none());

    let rows: Vec<_> = aliases
        .iter()
        .enumerate()
        .map(|(index, &alias)| {
            let mut row = graph_row(index, alias, walker.oids.get_oid_by_alias(alias));
            row.is_merge = alias == merge_alias;
            row
        })
        .collect();
    let symbols = SymbolTheme::main();
    let theme = Theme::classic();
    let lines = render_graph_projection(&theme, &symbols, &rows, &history, head_alias, 0, aliases.len(), true);
    let merge_text = line_text(&lines[merge_idx]);
    let next_text = line_text(&lines[merge_idx + 1]);
    let merge_col = merge_text.chars().position(|ch| ch == graph::MERGE.chars().next().unwrap()).unwrap();

    assert_ne!(next_text.chars().nth(merge_col), graph::VERTICAL.chars().next());
}

#[test]
fn walker_records_ref_stash_and_reflog_lanes_from_update_lane() {
    let (dir, mut repo) = temp_repo("cached-lanes");
    let path = dir.join("repo");
    let base = commit_file(&repo, "file.txt", "base", "base");
    {
        let base_commit = repo.find_commit(base).unwrap();
        repo.tag_lightweight("v-base", base_commit.as_object(), false).unwrap();
    }
    let stash = stash_tracked_change(&mut repo, "file.txt", "stashed change", "stashed change");

    let walker = walked_walker(&path, 100, true);

    let base_alias = walker.oids.get_existing_alias(base).unwrap();
    let stash_alias = walker.oids.get_existing_alias(stash).unwrap();

    assert!(walker.branches_lanes.contains_key(&base_alias));
    assert!(walker.tags_lanes.contains_key(&base_alias));
    assert!(walker.reflogs_lanes.contains_key(&base_alias));
    assert!(walker.stashes_lanes.contains_key(&stash_alias));
}

#[test]
fn walker_new_collects_startup_metadata_before_walking() {
    let (_dir, path, mut repo) = representative_graph_fixture("startup-metadata", 32);
    let stash = stash_tracked_change(&mut repo, "tail.txt", "stashed change", "stashed change");

    let walker = Walker::new(path.display().to_string(), 100, HashSet::new(), true, 20).unwrap();

    assert!(walker.branches_local.values().flatten().any(|name| name == "main"));
    assert!(walker.branches_local.values().flatten().any(|name| name == "feature"));
    assert!(walker.tags_local.values().flatten().any(|name| name == "v-main"));
    assert!(!walker.oids.stashes.is_empty());
    assert!(walker.oids.get_existing_alias(stash).is_some());
    assert!(!walker.head_reflog_entries.is_empty());
}

#[test]
fn walker_matches_rev_list_all_for_lightweight_and_annotated_tags() {
    for annotated in [false, true] {
        let (dir, repo) = temp_repo(if annotated { "annotated-tag-root" } else { "tag-only-root" });
        let path = dir.join("repo");
        let root = commit_file(&repo, "root.txt", "root", "root");
        let branch_tip = commit_file(&repo, "branch.txt", "branch", "branch");
        let tagged = commit_file_with_parents(&repo, "tagged.txt", "tagged", "tagged", &[], Some(if annotated { 100 } else { 99 }));
        let tagged_commit = repo.find_commit(tagged).unwrap();

        if annotated {
            let sig = Signature::now("Test User", "test@example.com").unwrap();
            repo.tag("annotated", tagged_commit.as_object(), &sig, "annotated", false).unwrap();
        } else {
            repo.tag_lightweight("tag-only", tagged_commit.as_object(), false).unwrap();
        }
        repo.reference("refs/heads/main", branch_tip, true, "test").unwrap();
        repo.set_head("refs/heads/main").unwrap();

        let walker = walked_walker(&path, 1, false);

        let sorted_oids: StdHashSet<Oid> =
            walker.oids.get_sorted_aliases().iter().filter_map(|alias| (!walker.oids.is_zero(walker.oids.get_gix_oid_by_alias(*alias))).then(|| walker.oids.get_oid_by_alias(*alias))).collect();

        assert_eq!(sorted_oids, StdHashSet::from([root, branch_tip, tagged]));
    }
}

#[test]
fn gix_tag_oids_resolves_tags_without_collecting_other_refs() {
    let (dir, repo) = temp_repo("gix-tag-oids");
    let path = dir.join("repo");
    let base = commit_file(&repo, "base.txt", "base", "base");
    let tagged = commit_file_with_parents(&repo, "tagged.txt", "tagged", "tagged", &[], Some(100));
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
    let (_dir, _repo_path, linked_path, repo) = linked_worktree_startup_fixture("startup-linked-worktree");

    let walker = Walker::new(linked_path.display().to_string(), 100, HashSet::new(), true, 20).unwrap();

    assert!(walker.gix_repo.worktree().is_some());
    assert_eq!(walker.gix_repo.common_dir(), repo.commondir());
    assert!(walker.branches_local.values().flatten().any(|name| name == "main"));
    assert!(walker.tags_local.values().flatten().any(|name| name == "v-main"));
}

#[test]
fn walker_keeps_stash_adjacent_to_its_base_parent() {
    let (dir, mut repo) = temp_repo("stash-order");
    let path = dir.join("repo");
    let base = commit_file(&repo, "file.txt", "base", "base");
    let stash = stash_tracked_change(&mut repo, "file.txt", "stashed change", "stashed change");

    let walker = walked_walker(&path, 100, false);

    let aliases = walker.oids.get_sorted_aliases();
    let base_alias = walker.oids.get_existing_alias(base).unwrap();
    let stash_alias = walker.oids.get_existing_alias(stash).unwrap();
    let base_idx = aliases.iter().position(|alias| *alias == base_alias).unwrap();
    let stash_idx = aliases.iter().position(|alias| *alias == stash_alias).unwrap();

    assert_eq!(stash_idx + 1, base_idx);
    assert_eq!(aliases.iter().filter(|&&alias| alias == stash_alias).count(), 1);
}
