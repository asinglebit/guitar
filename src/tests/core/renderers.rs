use super::*;
use crate::core::{
    chunk::{LaneRef, NONE},
    graph_service::{GraphBranchLabel, GraphReflogLabel, GraphTagLabel},
    worktrees::{WorktreeEntry, WorktreeKind},
};
use crate::helpers::colors::ColorPicker;
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

fn span_color(line: &Line<'_>, symbol: &str) -> Option<Color> {
    line.spans.iter().find(|span| span.content.as_ref() == symbol).and_then(|span| span.style.fg)
}

fn span_containing_color(line: &Line<'_>, text: &str) -> Option<Color> {
    line.spans.iter().find(|span| span.content.contains(text)).and_then(|span| span.style.fg)
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

fn branch_up_history(current_lane: usize) -> GraphHistory {
    let mut last = Vector::from(vec![Chunk::dummy(), Chunk::dummy()]);
    last[current_lane] = Chunk::commit(4, NONE, NONE);
    let mut prev = Vector::from(vec![Chunk::commit(10, 1, NONE), Chunk::commit(11, 2, NONE)]);
    for lane_idx in 0..prev.len() {
        if lane_idx != current_lane {
            prev[lane_idx].parent_a = 4;
        }
    }

    GraphHistory::from(Vector::from(vec![prev, last]))
}

fn branch_up_bridge_history() -> GraphHistory {
    GraphHistory::from(Vector::from(vec![
        Vector::from(vec![Chunk::commit(30, 4, NONE), Chunk::commit(31, 101, NONE), Chunk::commit(32, 102, NONE), Chunk::commit(33, 103, NONE)]),
        Vector::from(vec![Chunk::dummy(), Chunk::commit(31, 101, NONE), Chunk::commit(32, 102, NONE), Chunk::commit(4, NONE, NONE)]),
    ]))
}

fn transient_merge_closeout_history() -> GraphHistory {
    GraphHistory::from(Vector::from(vec![
        Vector::from(vec![Chunk::commit(10, 100, NONE), Chunk::commit(11, 101, NONE), Chunk::commit(20, 4, 2), Chunk::commit(12, 102, NONE)]),
        Vector::from(vec![Chunk::commit(4, NONE, NONE), Chunk::commit(11, 101, NONE), Chunk::dummy(), Chunk::commit(12, 102, NONE)]),
    ]))
}

fn capped_flattened_history(chunk: Chunk) -> GraphHistory {
    GraphHistory::from(Vector::from(vec![Vector::from(vec![Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), chunk.with_flattened(true)])]))
}

fn capped_merge_closeout_to_flattened_lane_history() -> GraphHistory {
    GraphHistory::from(Vector::from(vec![Vector::from(vec![Chunk::commit(9, 1, 2), Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::commit(50, 99, NONE).with_flattened(true)])]))
}

fn capped_merge_closeout_with_dummy_after_flattened_lane_history() -> GraphHistory {
    GraphHistory::from(Vector::from(vec![
        Vector::from(vec![Chunk::commit(40, NONE, NONE), Chunk::dummy(), Chunk::dummy(), Chunk::commit(50, 99, NONE).with_flattened(true), Chunk::commit(60, 100, NONE)]),
        Vector::from(vec![Chunk::commit(9, 1, 2), Chunk::dummy(), Chunk::dummy(), Chunk::commit(50, 99, NONE).with_flattened(true), Chunk::dummy()]),
    ]))
}

fn capped_merge_closeout_before_flattened_lane_history() -> GraphHistory {
    GraphHistory::from(Vector::from(vec![
        Vector::from(vec![Chunk::commit(40, NONE, NONE), Chunk::dummy(), Chunk::dummy(), Chunk::commit(50, 99, NONE), Chunk::commit(60, 100, NONE)]),
        Vector::from(vec![Chunk::commit(9, 1, 2), Chunk::dummy(), Chunk::dummy(), Chunk::commit(50, 99, NONE), Chunk::commit(60, 100, NONE)]),
        Vector::from(vec![Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::commit(50, 99, NONE).with_flattened(true), Chunk::commit(60, 100, NONE)]),
    ]))
}

fn capped_merge_on_last_lane_before_flattening_history() -> GraphHistory {
    GraphHistory::from(Vector::from(vec![
        Vector::from(vec![Chunk::commit(40, NONE, NONE), Chunk::dummy(), Chunk::dummy(), Chunk::commit(50, 99, NONE), Chunk::commit(60, 100, NONE)]),
        Vector::from(vec![Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::commit(9, 1, 2)]),
        Vector::from(vec![Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::commit(60, 100, NONE).with_flattened(true)]),
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
fn graph_projection_uses_branch_up_right_when_dummy_lane_points_to_commit_on_right() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let row = graph_row_with_alias(1, 4);
    let lines = render_graph_projection(&theme, &symbols, &[row], &branch_up_history(1), NONE, 1, 2, true);
    let text = line_text(&lines[0]);

    assert!(text.contains(graph::BRANCH_UP_RIGHT), "{text:?}");
    assert!(!text.contains(graph::BRANCH_UP), "{text:?}");
    assert!(text.find(graph::BRANCH_UP_RIGHT) < text.find(graph::COMMIT), "{text:?}");
}

#[test]
fn graph_projection_bridges_left_dummy_lane_to_current_row() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let rows = vec![graph_row_with_alias(0, 30), graph_row_with_alias(1, 4)];
    let lines = render_graph_projection(&theme, &symbols, &rows, &branch_up_bridge_history(), NONE, 0, 2, true);
    let text = line_text(&lines[1]);
    let (_, after_branch_up) = text.split_once(graph::BRANCH_UP_RIGHT).unwrap();
    let (connector, _) = after_branch_up.split_once(graph::COMMIT).unwrap();

    assert!(text.contains("╰──── ○"), "{text:?}");
    assert_eq!(connector, format!("{}{}{}{}{}", graph::HORIZONTAL, graph::HORIZONTAL, graph::HORIZONTAL, graph::HORIZONTAL, graph::EMPTY), "{text:?}");
    assert!(!text.contains(graph::HORIZONTAL_DOTTED), "{text:?}");
    assert!(!connector.contains(graph::VERTICAL), "{text:?}");
    assert!(!text.contains(graph::BRANCH_UP), "{text:?}");
}

#[test]
fn graph_projection_keeps_branch_up_when_dummy_lane_points_back_left() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let row = graph_row_with_alias(1, 4);
    let lines = render_graph_projection(&theme, &symbols, &[row], &branch_up_history(0), NONE, 1, 2, true);
    let text = line_text(&lines[0]);

    assert!(text.contains(graph::BRANCH_UP), "{text:?}");
    assert!(!text.contains(graph::BRANCH_UP_RIGHT), "{text:?}");
    assert!(text.find(graph::COMMIT) < text.find(graph::BRANCH_UP), "{text:?}");
}

#[test]
fn graph_projection_closes_transient_merge_lane_to_first_parent() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let rows = vec![graph_row_with_alias(0, 20), graph_row_with_alias(1, 4)];
    let lines = render_graph_projection(&theme, &symbols, &rows, &transient_merge_closeout_history(), NONE, 0, 2, true);
    let text = line_text(&lines[1]);

    let commit_idx = text.find(graph::COMMIT).unwrap();
    let horizontal_idx = text.find(graph::HORIZONTAL).unwrap();
    let branch_up_idx = text.find(graph::BRANCH_UP).unwrap();

    assert!(commit_idx < horizontal_idx, "{text:?}");
    assert!(horizontal_idx < branch_up_idx, "{text:?}");
    assert!(!text.contains(graph::BRANCH_UP_RIGHT), "{text:?}");
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
    row.branches = vec![GraphBranchLabel { name: "main".to_string(), is_local: true, lane: Some(LaneRef::new(0, false)) }];
    row.tags = vec![GraphTagLabel { name: "v1".to_string(), lane: Some(LaneRef::new(0, false)) }];
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
    row.reflog = Some(GraphReflogLabel { selector: "HEAD@{0}".to_string(), message: "commit: summary".to_string(), lane: Some(LaneRef::new(0, false)) });

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
fn graph_projection_renders_flattened_commit_on_capped_lane_in_grey() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let row = graph_row_with_alias(0, 9);
    let history = capped_flattened_history(Chunk::commit(9, NONE, NONE));

    let lines = render_graph_projection(&theme, &symbols, &[row], &history, NONE, 0, 1, true);

    assert_eq!(span_color(&lines[0], graph::COMMIT), Some(theme.COLOR_GREY_500));
}

#[test]
fn graph_projection_keeps_normal_last_lane_palette_colored_without_overflow() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let row = graph_row_with_alias(0, 5);
    let history = GraphHistory::from(Vector::from(vec![Vector::from(vec![Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::commit(5, NONE, NONE)])]));

    let lines = render_graph_projection(&theme, &symbols, &[row], &history, NONE, 0, 1, true);

    assert_eq!(span_color(&lines[0], graph::COMMIT), Some(ColorPicker::from_theme(&theme).get_lane(4)));
}

#[test]
fn graph_projection_uses_flattened_color_for_pipe_merge_and_connector_spans() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();

    let pipe_history =
        GraphHistory::from(Vector::from(vec![Vector::from(vec![Chunk::commit(1, NONE, NONE), Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::commit(9, 99, NONE).with_flattened(true)])]));
    let pipe_lines = render_graph_projection(&theme, &symbols, &[graph_row_with_alias(0, 1)], &pipe_history, NONE, 0, 1, true);
    assert_eq!(span_color(&pipe_lines[0], graph::VERTICAL_DOTTED), Some(theme.COLOR_GREY_500));
    assert_eq!(span_color(&pipe_lines[0], graph::VERTICAL), None);

    let merge_history = capped_flattened_history(Chunk::commit(9, 1, 2));
    let merge_lines = render_graph_projection(&theme, &symbols, &[graph_row_with_alias(0, 9)], &merge_history, NONE, 0, 1, true);
    assert_eq!(span_color(&merge_lines[0], graph::MERGE), Some(theme.COLOR_GREY_500));

    let connector_history = GraphHistory::from(Vector::from(vec![
        Vector::from(vec![Chunk::commit(30, 4, NONE).with_flattened(true), Chunk::commit(31, 101, NONE), Chunk::commit(32, 102, NONE), Chunk::commit(33, 103, NONE)]),
        Vector::from(vec![Chunk::dummy(), Chunk::commit(31, 101, NONE), Chunk::commit(32, 102, NONE), Chunk::commit(4, NONE, NONE)]),
    ]));
    let connector_rows = vec![graph_row_with_alias(0, 30), graph_row_with_alias(1, 4)];
    let connector_lines = render_graph_projection(&theme, &symbols, &connector_rows, &connector_history, NONE, 0, 2, true);
    assert_eq!(span_color(&connector_lines[1], graph::HORIZONTAL_DOTTED), Some(theme.COLOR_GREY_500));
    assert_eq!(span_color(&connector_lines[1], graph::HORIZONTAL), None);
}

#[test]
fn graph_projection_draws_branch_down_on_flattened_closeout_lane() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let row = graph_row_with_alias(0, 9);
    let history = capped_merge_closeout_to_flattened_lane_history();

    let lines = render_graph_projection(&theme, &symbols, &[row], &history, NONE, 0, 1, true);
    let text = line_text(&lines[0]);

    assert!(text.contains(graph::BRANCH_DOWN), "{text:?}");
    assert_eq!(span_color(&lines[0], graph::BRANCH_DOWN), Some(theme.COLOR_GREY_500));
    assert_eq!(span_color(&lines[0], graph::HORIZONTAL_DOTTED), Some(theme.COLOR_GREY_500));
    assert!(lines[0].spans.len() <= 5 * 2, "{text:?}");
}

#[test]
fn graph_projection_redirects_inside_snapshot_closeout_to_flattened_lane() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let rows = vec![graph_row_with_alias(0, 40), graph_row_with_alias(1, 9)];
    let history = capped_merge_closeout_with_dummy_after_flattened_lane_history();

    let lines = render_graph_projection(&theme, &symbols, &rows, &history, NONE, 0, 2, true);
    let text = line_text(&lines[1]);
    let expected_closeout = format!("{}{}{}{}{}", graph::HORIZONTAL_DOTTED, graph::HORIZONTAL_DOTTED, graph::HORIZONTAL_DOTTED, graph::HORIZONTAL_DOTTED, graph::BRANCH_DOWN);

    assert!(text.contains(&expected_closeout), "{text:?}");
    assert_eq!(text.matches(graph::HORIZONTAL_DOTTED).count(), 4, "{text:?}");
    assert!(!text.contains(graph::HORIZONTAL), "{text:?}");
    assert_eq!(span_color(&lines[1], graph::BRANCH_DOWN), Some(theme.COLOR_GREY_500));
}

#[test]
fn graph_projection_looks_ahead_for_flattened_closeout_lane() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let rows = vec![graph_row_with_alias(0, 40), graph_row_with_alias(1, 9), graph_row_with_alias(2, 50)];
    let history = capped_merge_closeout_before_flattened_lane_history();

    let lines = render_graph_projection(&theme, &symbols, &rows, &history, NONE, 0, 3, true);
    let text = line_text(&lines[1]);
    let expected_closeout = format!("{}{}{}{}{}", graph::HORIZONTAL_DOTTED, graph::HORIZONTAL_DOTTED, graph::HORIZONTAL_DOTTED, graph::HORIZONTAL_DOTTED, graph::BRANCH_DOWN);

    assert!(text.contains(&expected_closeout), "{text:?}");
    assert!(!text.contains(graph::HORIZONTAL), "{text:?}");
    assert_eq!(span_color(&lines[1], graph::BRANCH_DOWN), Some(theme.COLOR_GREY_500));
}

#[test]
fn graph_projection_skips_branch_down_when_merge_is_on_lookahead_flattened_lane() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let rows = vec![graph_row_with_alias(0, 40), graph_row_with_alias(1, 9), graph_row_with_alias(2, 60)];
    let history = capped_merge_on_last_lane_before_flattening_history();

    let lines = render_graph_projection(&theme, &symbols, &rows, &history, NONE, 0, 3, true);
    let text = line_text(&lines[1]);

    assert!(!text.contains(graph::BRANCH_DOWN), "{text:?}");
}

#[test]
fn graph_projection_keeps_nonflattened_pipe_symbols_solid() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let pipe_history = GraphHistory::from(Vector::from(vec![Vector::from(vec![Chunk::commit(1, NONE, NONE), Chunk::dummy(), Chunk::dummy(), Chunk::dummy(), Chunk::commit(9, 99, NONE)])]));

    let pipe_lines = render_graph_projection(&theme, &symbols, &[graph_row_with_alias(0, 1)], &pipe_history, NONE, 0, 1, true);

    assert_eq!(span_color(&pipe_lines[0], graph::VERTICAL), Some(ColorPicker::from_theme(&theme).get_lane(4)));
    assert_eq!(span_color(&pipe_lines[0], graph::VERTICAL_DOTTED), None);
}

#[test]
fn graph_projection_does_not_draw_flattened_merge_past_snapshot_width() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let row = graph_row_with_alias(0, 9);
    let history = capped_flattened_history(Chunk::commit(9, 1, 2));

    let lines = render_graph_projection(&theme, &symbols, &[row], &history, NONE, 0, 1, true);

    assert!(lines[0].spans.len() <= 1 + 5 * 2, "{:?}", line_text(&lines[0]));
    assert!(!line_text(&lines[0]).contains(graph::BRANCH_DOWN), "{:?}", line_text(&lines[0]));
}

#[test]
fn message_projection_uses_flattened_lane_color_for_ref_labels() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::main();
    let flattened = LaneRef::new(4, true);
    let mut row = graph_row(0, Oid::from_str("1111111111111111111111111111111111111111").unwrap(), "summary");
    row.branches = vec![GraphBranchLabel { name: "main".to_string(), is_local: true, lane: Some(flattened) }];
    row.tags = vec![GraphTagLabel { name: "v1".to_string(), lane: Some(flattened) }];
    row.is_stash = true;
    row.stash_lane = Some(flattened);

    let lines = render_message_projection(&theme, &symbols, &[row], true, true, 0, &UncommittedChanges::default(), true);

    assert_eq!(span_containing_color(&lines[0], "main"), Some(theme.COLOR_GREY_500));
    assert_eq!(span_containing_color(&lines[0], "v1"), Some(theme.COLOR_GREY_500));
    assert_eq!(span_containing_color(&lines[0], "stash"), Some(theme.COLOR_GREY_500));

    let mut reflog_row = graph_row(0, Oid::from_str("2222222222222222222222222222222222222222").unwrap(), "summary");
    reflog_row.reflog = Some(GraphReflogLabel { selector: "HEAD@{0}".to_string(), message: "commit: summary".to_string(), lane: Some(flattened) });
    let reflog_lines = render_message_projection(&theme, &symbols, &[reflog_row], true, false, 0, &UncommittedChanges::default(), true);

    assert_eq!(span_containing_color(&reflog_lines[0], "HEAD@{0}"), Some(theme.COLOR_GREY_500));
}

#[test]
fn message_projection_uses_ascii_symbols_when_requested() {
    let theme = Theme::classic();
    let symbols = SymbolTheme::ascii();
    let mut row = graph_row(0, Oid::from_str("1111111111111111111111111111111111111111").unwrap(), "summary");
    row.branches = vec![GraphBranchLabel { name: "main".to_string(), is_local: true, lane: Some(LaneRef::new(0, false)) }];
    row.tags = vec![GraphTagLabel { name: "v1".to_string(), lane: Some(LaneRef::new(0, false)) }];

    let lines = render_message_projection(&theme, &symbols, &[row], true, true, 0, &UncommittedChanges::default(), true);
    let rendered = line_text(&lines[0]);

    assert!(rendered.contains(&symbols.branch.local_visible));
    assert!(rendered.contains(&symbols.entity.tag));
    assert!(rendered.is_ascii());
}
