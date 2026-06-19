use super::*;
use crate::core::{
    chunk::NONE,
    graph_service::{GraphBranchLabel, GraphReflogLabel, GraphTagLabel},
    worktrees::{WorktreeEntry, WorktreeKind},
};
use crate::helpers::symbols::{SymbolTheme, graph};
use git2::Oid;
use im::Vector;
use ratatui::style::Color;
use std::path::PathBuf;

fn graph_row(index: usize, oid: Oid, summary: &str) -> GraphRow {
    GraphRow {
        index,
        alias: index as u32 + 1,
        oid,
        summary: summary.to_string(),
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

fn graph_row_with_alias(index: usize, alias: u32) -> GraphRow {
    let mut row = graph_row(index, Oid::from_str("1111111111111111111111111111111111111111").unwrap(), "merge");
    row.alias = alias;
    row
}

fn merge_right_from_history(prev_lane_parent: u32) -> GraphHistory {
    GraphHistory::from(Vector::from(vec![
        Vector::from(vec![Chunk::commit(20, prev_lane_parent, NONE), Chunk::commit(21, 200, NONE), Chunk::dummy()]),
        Vector::from(vec![Chunk::commit(10, 1, NONE), Chunk::commit(11, 2, NONE), Chunk::commit(4, 1, 2)]),
    ]))
}

#[test]
fn sha_projection_uses_text_and_highlighted_text_colors() {
    let theme = Theme::classic();
    let rows =
        vec![graph_row(0, Oid::from_str("1111111111111111111111111111111111111111").unwrap(), "first"), graph_row(1, Oid::from_str("2222222222222222222222222222222222222222").unwrap(), "second")];

    let lines = render_sha_projection(&theme, &rows, 1);

    assert_eq!(lines[0].spans[0].style.fg, Some(theme.COLOR_TEXT));
    assert_eq!(lines[1].spans[0].style.fg, Some(theme.COLOR_HIGHLIGHTED));
}

#[test]
fn message_projection_uses_text_and_highlighted_text_colors() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let rows =
        vec![graph_row(0, Oid::from_str("1111111111111111111111111111111111111111").unwrap(), "first"), graph_row(1, Oid::from_str("2222222222222222222222222222222222222222").unwrap(), "second")];

    let lines = render_message_projection(&theme, &symbols, &rows, false, true, 1, &UncommittedChanges::default(), true);

    assert_eq!(lines[0].spans[0].style.fg, Some(theme.COLOR_TEXT));
    assert_eq!(lines[1].spans[0].style.fg, Some(theme.COLOR_HIGHLIGHTED));
}

#[test]
fn date_projection_renders_commit_dates_and_blanks_uncommitted_rows() {
    let theme = Theme::classic();
    let mut commit = graph_row(1, Oid::from_str("2222222222222222222222222222222222222222").unwrap(), "commit");
    commit.committer_date = "2026-06-17 14:23".to_string();
    let mut uncommitted = graph_row(0, Oid::zero(), "");
    uncommitted.alias = NONE;
    uncommitted.committer_date = "ignored".to_string();

    let lines = render_date_projection(&theme, &[uncommitted, commit], 1);

    assert_eq!(line_text(&lines[0]), "");
    assert_eq!(line_text(&lines[1]), "2026-06-17 14:23");
}

#[test]
fn committer_projection_renders_fixed_width_names_and_blanks_uncommitted_rows() {
    let theme = Theme::classic();
    let mut commit = graph_row(1, Oid::from_str("2222222222222222222222222222222222222222").unwrap(), "commit");
    commit.committer_name = "Very Long Committer Name".to_string();
    let mut uncommitted = graph_row(0, Oid::zero(), "");
    uncommitted.alias = NONE;
    uncommitted.committer_name = "ignored".to_string();

    let lines = render_committer_projection(&theme, &[uncommitted, commit], 1);
    let rendered = line_text(&lines[1]);

    assert_eq!(line_text(&lines[0]), "");
    assert_eq!(rendered.chars().count(), GRAPH_COMMITTER_WIDTH);
    assert!(rendered.contains("..."));
}

#[test]
fn graph_projection_uses_merge_right_from_and_up_when_previous_lane_carries_same_parent() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let row = graph_row_with_alias(1, 4);
    let with_up_rows = vec![graph_row_with_alias(0, 20), row.clone()];

    let with_up = render_graph_projection(&theme, &symbols, &with_up_rows, &merge_right_from_history(1), NONE, 0, 2, true);
    let without_up = render_graph_projection(&theme, &symbols, &[row], &merge_right_from_history(99), NONE, 1, 2, true);
    let with_up_text = line_text(&with_up[1]);
    let without_up_text = line_text(&without_up[0]);
    let (_, after_merge_right_from) = with_up_text.split_once(graph::MERGE_RIGHT_FROM).unwrap();
    let (connector_after_merge_right_from, _) = after_merge_right_from.split_once(graph::MERGE).unwrap();

    assert_eq!(with_up_text.matches(graph::MERGE_RIGHT_FROM).count(), 1, "{with_up_text:?}");
    assert!(connector_after_merge_right_from.contains(graph::HORIZONTAL), "{with_up_text:?}");
    assert!(connector_after_merge_right_from.replace(graph::HORIZONTAL, "").trim().is_empty(), "{with_up_text:?}");
    assert_eq!(without_up_text.matches(graph::MERGE_RIGHT_FROM).count(), 1, "{without_up_text:?}");
}

#[test]
fn empty_column_pruning_preserves_visible_spans_and_styles() {
    let visible_style = Style::default().fg(Color::Red);
    let symbols = SymbolTheme::main();
    let mut lines = vec![
        Line::from(vec![Span::raw(graph::EMPTY), Span::raw(graph::HORIZONTAL), Span::styled(graph::VERTICAL, visible_style), Span::raw(graph::EMPTY)]),
        Line::from(vec![Span::raw(graph::HORIZONTAL), Span::raw(graph::EMPTY), Span::raw(graph::EMPTY), Span::raw(graph::EMPTY)]),
    ];

    remove_empty_columns(&mut lines, &symbols);

    assert_eq!(line_text(&lines[0]), format!("{}{}", graph::VERTICAL, graph::EMPTY));
    assert_eq!(lines[0].spans[0].style.fg, Some(Color::Red));
    assert_eq!(line_text(&lines[1]), format!("{}{}", graph::EMPTY, graph::EMPTY));
}

#[test]
fn message_projection_toggles_refs_without_hiding_reflog_labels() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let mut row = graph_row(0, Oid::from_str("1111111111111111111111111111111111111111").unwrap(), "summary");
    row.branches = vec![GraphBranchLabel { name: "main".to_string(), is_local: true, lane: Some(0) }];
    row.tags = vec![GraphTagLabel { name: "v1".to_string(), lane: Some(0) }];
    row.is_stash = true;
    row.worktrees = vec![WorktreeEntry {
        name: "wt".to_string(),
        path: PathBuf::from("/tmp/wt"),
        branch: Some("main".to_string()),
        head: Some(row.oid),
        alias: Some(row.alias),
        kind: WorktreeKind::Linked,
        is_current: false,
        is_valid: true,
        is_prunable: false,
        locked_reason: None,
        is_dirty: false,
    }];
    row.reflog = Some(GraphReflogLabel { selector: "HEAD@{0}".to_string(), message: "commit: summary".to_string(), lane: Some(0) });

    let shown = render_message_projection(&theme, &symbols, &[row.clone()], true, true, 0, &UncommittedChanges::default(), true);
    let hidden = render_message_projection(&theme, &symbols, &[row], true, false, 0, &UncommittedChanges::default(), true);
    let shown = line_text(&shown[0]);
    let hidden = line_text(&hidden[0]);

    assert!(shown.contains("main"));
    assert!(shown.contains("v1"));
    assert!(shown.contains("stash"));
    assert!(shown.contains("wt"));
    assert!(!shown.contains("HEAD@{0}"));
    assert!(hidden.contains("HEAD@{0}"));
    assert!(hidden.contains("summary"));
    assert!(!hidden.contains("main"));
    assert!(!hidden.contains("v1"));
    assert!(!hidden.contains("stash"));
    assert!(!hidden.contains("wt"));
}

#[test]
fn graph_projection_uses_ascii_symbols_when_requested() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::ascii();
    let row = graph_row_with_alias(0, 1);
    let history = GraphHistory::from(Vector::from(vec![Vector::from(vec![Chunk::commit(1, NONE, NONE)])]));

    let lines = render_graph_projection(&theme, &symbols, &[row], &history, NONE, 0, 1, true);
    let rendered = line_text(&lines[0]);

    assert!(rendered.contains(&symbols.graph.commit));
    assert!(rendered.is_ascii());
}

#[test]
fn message_projection_uses_ascii_symbols_when_requested() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::ascii();
    let mut row = graph_row(0, Oid::from_str("1111111111111111111111111111111111111111").unwrap(), "summary");
    row.branches = vec![GraphBranchLabel { name: "main".to_string(), is_local: true, lane: Some(0) }];
    row.tags = vec![GraphTagLabel { name: "v1".to_string(), lane: Some(0) }];

    let lines = render_message_projection(&theme, &symbols, &[row], true, true, 0, &UncommittedChanges::default(), true);
    let rendered = line_text(&lines[0]);

    assert!(rendered.contains(&symbols.branch.local_visible));
    assert!(rendered.contains(&symbols.entity.tag));
    assert!(rendered.is_ascii());
}
