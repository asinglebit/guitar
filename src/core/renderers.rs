use crate::core::{
    graph_service::{GraphHistory, GraphRow},
    layers::LayersContext,
};
use crate::helpers::text::truncate_with_ellipsis;
use crate::helpers::{
    keymap::{Command, KeyBinding, keycode_to_visual_string},
    localisation::status as status_text,
};
use crate::{
    core::chunk::{Chunk, LaneRef, NONE},
    git::queries::helpers::UncommittedChanges,
    helpers::{
        colors::ColorPicker,
        palette::*,
        symbols::{GraphSymbols, SymbolTheme},
        text::{modifiers_to_string, pascal_to_spaced},
    },
};
use im::Vector;
use indexmap::IndexMap;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub const GRAPH_COMMITTER_WIDTH: usize = 18;

// Render graph symbols from worker-projected rows. The lane history is still
// precomputed by Buffer, but only for the requested visible range.
pub fn render_graph_projection(
    theme: &Theme, symbols: &SymbolTheme, rows: &[GraphRow], history: &GraphHistory, head_alias: u32, start: usize, end: usize, render_uncommitted_row: bool,
) -> Vec<Line<'static>> {
    let graph = &symbols.graph;
    let worktree = &symbols.worktree;
    let mut layers = LayersContext::new(ColorPicker::from_theme(theme));
    let mut lines: Vec<Line> = Vec::new();

    for row in rows {
        let global_idx = row.index;

        layers.clear();
        let mut spans = vec![Span::raw(" ")];

        let mut is_commit_found = false;
        let mut is_merged_before = false;
        let mut lane_idx = 0;

        if row.alias == NONE && !render_uncommitted_row {
            lines.push(Line::default());
            continue;
        }
        if history.is_empty() {
            return vec![Line::default()];
        }
        let delta = (history.len() + global_idx).saturating_sub(end);
        let prev = if delta == 0 { None } else { history.get(delta - 1) };
        let last = match history.get(delta) {
            Some(snapshot) => snapshot,
            None => {
                lines.push(Line::default());
                continue;
            },
        };
        let next = history.get(delta + 1);
        layers.reserve(last.len().saturating_mul(2));
        let flattened_lanes = flattened_lanes(last, prev);
        let closeout_flattened_lanes = flattened_lanes_around_closeout(last, prev, next);
        layers.set_flattened_lanes(flattened_lanes.clone());

        if row.alias == NONE {
            lines.push(Line::from(Span::styled(format!(" {}", graph.uncommitted), Style::default().fg(theme.COLOR_GREY_400))));
            continue;
        }

        let current_row_lane_idx = last.iter().position(|chunk| !chunk.is_dummy() && chunk.alias == row.alias);
        let mut branch_up_bridges: Vec<(usize, usize)> = Vec::new();
        let mut branching_lanes: Vec<usize> = Vec::new();
        for (lane_idx, chunk) in last.iter().enumerate() {
            if chunk.is_dummy()
                && let Some(prev_snapshot) = prev
                && let Some(prev) = prev_snapshot.get(lane_idx)
                && dummy_lane_closes_to_row(prev, row.alias)
            {
                branching_lanes.push(lane_idx);
                continue;
            }

            if chunk.is_dummy()
                && let Some(prev_snapshot) = prev
                && prev_snapshot.get(lane_idx).is_none()
            {
                branching_lanes.push(lane_idx);
            }
        }
        let mut branching_lane_idx = 0;

        for chunk in last.iter() {
            if is_commit_found
                && !branching_lanes.is_empty()
                && let Some(&closest_lane) = branching_lanes.get(branching_lane_idx)
            {
                if closest_lane == lane_idx {
                    branching_lane_idx += 1;
                } else if lane_idx < closest_lane {
                    layers.merge(&graph.empty, closest_lane);
                    layers.merge(&graph.empty, closest_lane);
                    layers.commit(&graph.empty, closest_lane);
                    layers.commit(&graph.empty, closest_lane);
                    layers.pipe(pipe_symbol(graph, &flattened_lanes, closest_lane, &graph.horizontal), closest_lane);
                    layers.pipe(pipe_symbol(graph, &flattened_lanes, closest_lane, &graph.horizontal), closest_lane);
                    lane_idx += 1;
                    continue;
                }
            }

            if chunk.is_dummy() {
                if let Some(prev_snapshot) = prev {
                    match prev_snapshot.get(lane_idx) {
                        Some(prev) => {
                            if dummy_lane_closes_to_row(prev, row.alias) {
                                layers.commit(&graph.empty, lane_idx);
                                layers.commit(&graph.empty, lane_idx);
                                layers.pipe(branch_up_symbol(graph, lane_idx, current_row_lane_idx), lane_idx);
                                layers.pipe(&graph.empty, lane_idx);
                                if let Some(row_lane_idx) = current_row_lane_idx
                                    && lane_idx < row_lane_idx
                                {
                                    branch_up_bridges.push((lane_idx, row_lane_idx));
                                }
                            } else {
                                layers.commit(&graph.empty, lane_idx);
                                layers.commit(&graph.empty, lane_idx);
                                layers.pipe(&graph.empty, lane_idx);
                                layers.pipe(&graph.empty, lane_idx);
                            }
                        },
                        None => {
                            layers.commit(&graph.empty, lane_idx);
                            layers.commit(&graph.empty, lane_idx);
                            layers.pipe(branch_up_symbol(graph, lane_idx, current_row_lane_idx), lane_idx);
                            layers.pipe(&graph.empty, lane_idx);
                            if let Some(row_lane_idx) = current_row_lane_idx
                                && lane_idx < row_lane_idx
                            {
                                branch_up_bridges.push((lane_idx, row_lane_idx));
                            }
                        },
                    }
                }
            } else if row.alias == chunk.alias {
                is_commit_found = true;
                let is_two_parents = chunk.parent_a != NONE && chunk.parent_b != NONE;
                if is_two_parents && !row.has_any_branch {
                    layers.commit(&graph.merge, lane_idx);
                } else if row.has_any_branch {
                    layers.commit(&graph.commit_branch, lane_idx);
                } else if row.worktrees.iter().any(|entry| entry.branch.is_none() || !row.has_any_branch) {
                    layers.commit(&worktree.current, lane_idx);
                } else if row.is_stash {
                    layers.commit(&graph.commit_stash, lane_idx);
                } else {
                    layers.commit(&graph.commit, lane_idx);
                }
                layers.commit(&graph.empty, lane_idx);
                layers.pipe(&graph.empty, lane_idx);
                layers.pipe(&graph.empty, lane_idx);

                let mut is_mergee_found = false;
                let mut is_drawing = false;
                if is_two_parents {
                    let mut is_merger_found = false;
                    let mut merger_idx: usize = 0;
                    for chunk_nested in last {
                        if ((chunk_nested.parent_a != NONE && chunk_nested.parent_b == NONE) || (chunk_nested.parent_a == NONE && chunk_nested.parent_b != NONE))
                            && chunk.parent_b == chunk_nested.parent_a
                        {
                            is_merger_found = true;
                            break;
                        }
                        merger_idx += 1;
                    }

                    let mut mergee_idx: usize = 0;
                    for chunk_nested in last {
                        if row.alias == chunk_nested.alias {
                            break;
                        }
                        mergee_idx += 1;
                    }

                    let mut is_merge_right_from_drawn = false;
                    for (chunk_nested_idx, chunk_nested) in last.iter().enumerate() {
                        if !is_mergee_found {
                            if row.alias == chunk_nested.alias {
                                is_mergee_found = true;
                                if is_merger_found {
                                    is_drawing = !is_drawing;
                                }
                                if !is_drawing {
                                    is_merged_before = true;
                                }
                                layers.merge(&graph.empty, merger_idx);
                                layers.merge(&graph.empty, merger_idx);
                            } else if !is_merger_found {
                                layers.merge(&graph.empty, merger_idx);
                                layers.merge(&graph.empty, merger_idx);
                            } else if ((chunk_nested.parent_a != NONE && chunk_nested.parent_b == NONE) || (chunk_nested.parent_a == NONE && chunk_nested.parent_b != NONE))
                                && (chunk.parent_a == chunk_nested.parent_a || chunk.parent_b == chunk_nested.parent_a)
                            {
                                let is_merge_start = chunk_nested_idx == merger_idx || previous_scanline_carries_parent(prev, chunk_nested_idx, chunk_nested);
                                let symbol = if is_merge_start && !is_merge_right_from_drawn {
                                    is_merge_right_from_drawn = true;
                                    &graph.merge_right_from
                                } else {
                                    &graph.horizontal
                                };
                                layers.merge(pipe_symbol(graph, &flattened_lanes, merger_idx, symbol), merger_idx);

                                if chunk_nested_idx + 1 == mergee_idx {
                                    layers.merge(&graph.empty, merger_idx);
                                } else {
                                    layers.merge(pipe_symbol(graph, &flattened_lanes, merger_idx, &graph.horizontal), merger_idx);
                                }
                                is_drawing = true;
                            } else if is_drawing {
                                if chunk_nested_idx + 1 == mergee_idx {
                                    layers.merge(pipe_symbol(graph, &flattened_lanes, merger_idx, &graph.horizontal), merger_idx);
                                    layers.merge(&graph.empty, merger_idx);
                                } else {
                                    layers.merge(pipe_symbol(graph, &flattened_lanes, merger_idx, &graph.horizontal), merger_idx);
                                    layers.merge(pipe_symbol(graph, &flattened_lanes, merger_idx, &graph.horizontal), merger_idx);
                                }
                            } else {
                                layers.merge(&graph.empty, merger_idx);
                                layers.merge(&graph.empty, merger_idx);
                            }
                        } else if is_merger_found && !is_merged_before {
                            if ((chunk_nested.parent_a != NONE && chunk_nested.parent_b == NONE) || (chunk_nested.parent_a == NONE && chunk_nested.parent_b != NONE))
                                && (chunk.parent_a == chunk_nested.parent_a || chunk.parent_b == chunk_nested.parent_a)
                            {
                                layers.merge(&graph.merge_left_from, merger_idx);
                                layers.merge(&graph.empty, merger_idx);
                                is_merged_before = true;
                                is_drawing = false;
                            } else if is_drawing {
                                layers.merge(pipe_symbol(graph, &flattened_lanes, merger_idx, &graph.horizontal), merger_idx);
                                layers.merge(pipe_symbol(graph, &flattened_lanes, merger_idx, &graph.horizontal), merger_idx);
                            } else {
                                layers.merge(&graph.empty, merger_idx);
                                layers.merge(&graph.empty, merger_idx);
                            }
                        }
                    }

                    if !is_merger_found {
                        let mut idx = last.len() - 1;
                        let mut trailing_dummies = 0;
                        for (i, c) in last.iter().enumerate().rev() {
                            if !c.is_dummy() {
                                idx = i;
                                break;
                            } else {
                                trailing_dummies += 1;
                            }
                        }

                        if let Some(prev) = prev {
                            let mut prev_trailing_dummies = 0;
                            for (_, c) in prev.iter().enumerate().rev() {
                                if c.is_dummy() {
                                    prev_trailing_dummies += 1;
                                } else {
                                    break;
                                }
                            }
                            if prev_trailing_dummies < trailing_dummies {
                                trailing_dummies = prev_trailing_dummies;
                            }
                        }

                        if let Some(closeout_idx) = merge_closeout_lane(last, &closeout_flattened_lanes, lane_idx, idx + 1) {
                            if trailing_dummies > 0 && prev.is_some_and(|prev| prev.len() > closeout_idx && prev[closeout_idx].is_dummy()) {
                                draw_merge_closeout_symbol(&mut layers, &closeout_flattened_lanes, &graph.branch_down, closeout_idx);
                            } else if trailing_dummies > 0 {
                                draw_merge_closeout_horizontals(&mut layers, graph, &closeout_flattened_lanes, lane_idx, closeout_idx);
                                draw_merge_closeout_symbol(&mut layers, &closeout_flattened_lanes, &graph.merge_left_from, closeout_idx);
                            } else {
                                draw_merge_closeout_horizontals(&mut layers, graph, &closeout_flattened_lanes, lane_idx, closeout_idx);
                                draw_merge_closeout_symbol(&mut layers, &closeout_flattened_lanes, &graph.branch_down, closeout_idx);
                            }
                        }
                    }
                }
            } else {
                layers.commit(&graph.empty, lane_idx);
                layers.commit(&graph.empty, lane_idx);
                if (chunk.parent_a == head_alias || chunk.parent_b == head_alias) && lane_idx == 0 {
                    layers.pipe_custom(&graph.vertical_dotted, lane_idx, theme.COLOR_GREY_500);
                } else if chunk.parent_a == NONE && chunk.parent_b == NONE {
                    layers.pipe(" ", lane_idx);
                } else {
                    layers.pipe(pipe_symbol(graph, &flattened_lanes, lane_idx, &graph.vertical), lane_idx);
                }
                layers.pipe(&graph.empty, lane_idx);
            }

            lane_idx += 1;
        }

        if !is_commit_found {
            let symbol = if row.has_any_branch {
                &graph.commit_branch
            } else if row.worktrees.iter().any(|entry| entry.branch.is_none() || !row.has_any_branch) {
                &worktree.current
            } else {
                &graph.commit
            };

            if let Some(flattened_lane_idx) = flattened_lane_idx(last) {
                layers.commit_at(flattened_lane_idx.saturating_mul(2), symbol, flattened_lane_idx);
            } else {
                layers.commit(symbol, lane_idx);
                layers.commit(&graph.empty, lane_idx);
                layers.pipe(&graph.empty, lane_idx);
                layers.pipe(&graph.empty, lane_idx);
            }
        }

        for (from_lane_idx, to_lane_idx) in branch_up_bridges {
            draw_branch_up_bridge(&mut layers, graph, &flattened_lanes, from_lane_idx, to_lane_idx);
        }

        layers.bake(&mut spans);
        lines.push(Line::from(spans));
    }

    remove_empty_columns(&mut lines, symbols);
    let _ = start;
    lines
}

fn branch_up_symbol(graph: &GraphSymbols, lane_idx: usize, current_row_lane_idx: Option<usize>) -> &str {
    if current_row_lane_idx.is_some_and(|row_lane_idx| lane_idx < row_lane_idx) { &graph.branch_up_right } else { &graph.branch_up }
}

fn flattened_lanes(last: &Vector<Chunk>, prev: Option<&Vector<Chunk>>) -> Vec<bool> {
    let len = last.len().max(prev.map_or(0, |snapshot| snapshot.len()));
    (0..len).map(|lane_idx| last.get(lane_idx).is_some_and(|chunk| chunk.is_flattened) || prev.and_then(|snapshot| snapshot.get(lane_idx)).is_some_and(|chunk| chunk.is_flattened)).collect()
}

fn flattened_lanes_around_closeout(last: &Vector<Chunk>, prev: Option<&Vector<Chunk>>, next: Option<&Vector<Chunk>>) -> Vec<bool> {
    let len = last.len().max(prev.map_or(0, |snapshot| snapshot.len())).max(next.map_or(0, |snapshot| snapshot.len()));
    (0..len)
        .map(|lane_idx| {
            last.get(lane_idx).is_some_and(|chunk| chunk.is_flattened)
                || prev.and_then(|snapshot| snapshot.get(lane_idx)).is_some_and(|chunk| chunk.is_flattened)
                || next.and_then(|snapshot| snapshot.get(lane_idx)).is_some_and(|chunk| chunk.is_flattened)
        })
        .collect()
}

fn flattened_lane_idx(snapshot: &Vector<Chunk>) -> Option<usize> {
    snapshot.iter().position(|chunk| chunk.is_flattened)
}

fn merge_closeout_lane(snapshot: &Vector<Chunk>, flattened_lanes: &[bool], current_lane_idx: usize, candidate_lane_idx: usize) -> Option<usize> {
    let lane_idx = if let Some(flattened_idx) = flattened_lanes.iter().position(|is_flattened| *is_flattened).filter(|flattened_idx| candidate_lane_idx >= *flattened_idx) {
        flattened_idx
    } else if draws_past_flattened_cap(snapshot, candidate_lane_idx) {
        flattened_lane_idx(snapshot)?
    } else {
        candidate_lane_idx
    };

    (lane_idx > current_lane_idx).then_some(lane_idx)
}

fn draws_past_flattened_cap(snapshot: &Vector<Chunk>, lane_idx: usize) -> bool {
    lane_idx >= snapshot.len() && snapshot.back().is_some_and(|chunk| chunk.is_flattened)
}

fn pipe_symbol<'a>(graph: &'a GraphSymbols, flattened_lanes: &[bool], lane_idx: usize, symbol: &'a str) -> &'a str {
    if !flattened_lanes.get(lane_idx).copied().unwrap_or(false) {
        return symbol;
    }

    if symbol == graph.vertical.as_str() {
        graph.vertical_dotted.as_str()
    } else if symbol == graph.horizontal.as_str() {
        graph.horizontal_dotted.as_str()
    } else {
        symbol
    }
}

fn draw_merge_closeout_horizontals(layers: &mut LayersContext, graph: &GraphSymbols, flattened_lanes: &[bool], from_lane_idx: usize, to_lane_idx: usize) {
    let symbol = pipe_symbol(graph, flattened_lanes, to_lane_idx, &graph.horizontal);
    let start_token_idx = from_lane_idx.saturating_add(1).saturating_mul(2);
    let end_token_idx = to_lane_idx.saturating_mul(2);
    let lane = closeout_lane_ref(flattened_lanes, to_lane_idx);

    for token_idx in start_token_idx..end_token_idx {
        layers.merge_at_ref(token_idx, symbol, lane);
    }
}

fn draw_merge_closeout_symbol(layers: &mut LayersContext, flattened_lanes: &[bool], symbol: &str, lane_idx: usize) {
    layers.merge_at_ref(lane_idx.saturating_mul(2), symbol, closeout_lane_ref(flattened_lanes, lane_idx));
}

fn closeout_lane_ref(flattened_lanes: &[bool], lane_idx: usize) -> LaneRef {
    LaneRef::new(lane_idx, flattened_lanes.get(lane_idx).copied().unwrap_or(false))
}

fn draw_branch_up_bridge(layers: &mut LayersContext, graph: &GraphSymbols, flattened_lanes: &[bool], from_lane_idx: usize, to_lane_idx: usize) {
    if from_lane_idx >= to_lane_idx {
        return;
    }

    let start_token_idx = from_lane_idx.saturating_mul(2).saturating_add(1);
    let end_token_idx = to_lane_idx.saturating_mul(2).saturating_sub(1);
    for token_idx in start_token_idx..end_token_idx {
        layers.merge_at(token_idx, pipe_symbol(graph, flattened_lanes, from_lane_idx, &graph.horizontal), from_lane_idx);
    }
}

fn dummy_lane_closes_to_row(prev: &Chunk, row_alias: u32) -> bool {
    single_active_parent(prev).is_some_and(|parent| parent == row_alias) || (prev.parent_a != NONE && prev.parent_b != NONE && prev.parent_a == row_alias)
}

fn previous_scanline_carries_parent(prev: Option<&Vector<Chunk>>, lane_idx: usize, chunk: &Chunk) -> bool {
    let Some(parent) = single_active_parent(chunk) else {
        return false;
    };
    let Some(prev_chunk) = prev.and_then(|snapshot| snapshot.get(lane_idx)) else {
        return false;
    };

    !prev_chunk.is_dummy() && (prev_chunk.parent_a == parent || prev_chunk.parent_b == parent)
}

fn single_active_parent(chunk: &Chunk) -> Option<u32> {
    match (chunk.parent_a != NONE, chunk.parent_b != NONE) {
        (true, false) => Some(chunk.parent_a),
        (false, true) => Some(chunk.parent_b),
        _ => None,
    }
}

// Remove graph lane pairs that are visually empty across every rendered row.
pub fn remove_empty_columns(lines: &mut Vec<Line<'_>>, symbols: &SymbolTheme) {
    let pair_count = lines.iter().map(|line| line.spans.len() / 2).max().unwrap_or(0);
    if pair_count == 0 {
        return;
    }

    let mut seen_pair = vec![false; pair_count];
    let mut keep_pair = vec![false; pair_count];

    // Graph lanes occupy two spans, so pruning must happen in span pairs.
    for line in lines.iter() {
        for (pair_idx, pair) in line.spans.chunks_exact(2).enumerate() {
            seen_pair[pair_idx] = true;
            if is_visible_lane_symbol(&pair[0], symbols) || is_visible_lane_symbol(&pair[1], symbols) {
                keep_pair[pair_idx] = true;
            }
        }
    }

    // Missing entries are not empty; only observed all-empty/horizontal pairs are removed.
    for (idx, keep) in keep_pair.iter_mut().enumerate() {
        *keep = *keep || !seen_pair[idx];
    }

    for line in lines.iter_mut() {
        let old_spans = std::mem::take(&mut line.spans);
        let mut new_spans = Vec::with_capacity(old_spans.len());
        let mut pending_pair_start = None;

        for (span_idx, span) in old_spans.into_iter().enumerate() {
            let pair_idx = span_idx / 2;
            let keep = keep_pair.get(pair_idx).copied().unwrap_or(true);
            if span_idx.is_multiple_of(2) {
                if keep {
                    pending_pair_start = Some(span);
                } else {
                    pending_pair_start = None;
                }
            } else if keep && let Some(first) = pending_pair_start.take() {
                new_spans.push(first);
                new_spans.push(span);
            }
        }

        // Preserve any leading or trailing single span outside lane pairs.
        if let Some(span) = pending_pair_start {
            new_spans.push(span);
        }
        line.spans = new_spans;
    }
}

fn is_visible_lane_symbol(span: &Span<'_>, symbols: &SymbolTheme) -> bool {
    span.content.as_ref() != symbols.graph.empty.as_str() && span.content.as_ref() != symbols.graph.horizontal.as_str()
}

pub fn render_sha_projection(theme: &Theme, rows: &[GraphRow], selected: usize) -> Vec<Line<'static>> {
    rows.iter()
        .map(|row| {
            if row.alias != NONE {
                let color = if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT };
                Line::from(Span::styled(format!("{:.9} ", row.oid), Style::default().fg(color)))
            } else {
                Line::from("")
            }
        })
        .collect()
}

pub fn render_date_projection(theme: &Theme, rows: &[GraphRow], selected: usize) -> Vec<Line<'static>> {
    rows.iter()
        .map(|row| {
            if row.alias != NONE {
                let color = if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT };
                Line::from(Span::styled(row.committer_date.clone(), Style::default().fg(color)))
            } else {
                Line::from("")
            }
        })
        .collect()
}

pub fn render_committer_projection(theme: &Theme, rows: &[GraphRow], selected: usize) -> Vec<Line<'static>> {
    rows.iter()
        .map(|row| {
            if row.alias != NONE {
                let color = if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT };
                let truncated = truncate_with_ellipsis(&row.committer_name, GRAPH_COMMITTER_WIDTH);
                Line::from(Span::styled(format!("{:<width$}", truncated, width = GRAPH_COMMITTER_WIDTH), Style::default().fg(color)))
            } else {
                Line::from("")
            }
        })
        .collect()
}

pub fn render_message_projection(
    theme: &Theme, symbols: &SymbolTheme, rows: &[GraphRow], show_reflog_labels: bool, show_ref_labels: bool, selected: usize, uncommitted: &UncommittedChanges, render_uncommitted_row: bool,
) -> Vec<Line<'static>> {
    let color_picker = ColorPicker::from_theme(theme);
    let branch_symbols = &symbols.branch;
    let entity = &symbols.entity;
    let graph = &symbols.graph;
    let status = &symbols.status;
    let worktree_symbols = &symbols.worktree;
    let mut lines = Vec::new();

    for row in rows {
        let mut spans = Vec::new();

        if row.alias == NONE && !render_uncommitted_row {
            lines.push(Line::default());
        } else if row.alias != NONE {
            if show_ref_labels {
                for worktree in &row.worktrees {
                    let color = if !worktree.is_valid || worktree.locked_reason.is_some() {
                        theme.COLOR_GREY_600
                    } else if worktree.is_current {
                        theme.COLOR_GRASS
                    } else {
                        theme.COLOR_TEAL
                    };
                    spans.push(Span::styled(format!("{} {} ", worktree_symbols.current, worktree.name), Style::default().fg(color)));
                }
            }
            let has_worktree_label = show_ref_labels && !row.worktrees.is_empty();

            if show_ref_labels {
                for branch in &row.branches {
                    let color = branch.lane.map(|lane| color_picker.get_lane_ref(lane)).unwrap_or(theme.COLOR_TEXT);
                    spans.push(Span::styled(
                        format!("{} {} ", if branch.is_local { branch_symbols.local_visible.as_str() } else { branch_symbols.remote_visible.as_str() }, branch.name),
                        Style::default().fg(color),
                    ));
                }
            }
            let has_visible_branch_label = show_ref_labels && !row.branches.is_empty();

            if show_ref_labels {
                for tag in &row.tags {
                    let color = tag.lane.map(|lane| color_picker.get_lane_ref(lane)).unwrap_or(theme.COLOR_TEXT);
                    spans.push(Span::styled(format!("{} {} ", entity.tag, tag.name), Style::default().fg(color)));
                }
            }
            let has_tag_label = show_ref_labels && !row.tags.is_empty();

            if show_ref_labels && row.is_stash {
                let color = row.stash_lane.map(|lane| color_picker.get_lane_ref(lane)).unwrap_or(theme.COLOR_TEXT);
                spans.push(Span::styled(format!("{} {} ", graph.commit_stash, status_text::STASH()), Style::default().fg(color)));
            }
            let has_stash_label = show_ref_labels && row.is_stash;

            if show_reflog_labels
                && !has_visible_branch_label
                && !has_tag_label
                && !has_stash_label
                && !has_worktree_label
                && let Some(reflog) = &row.reflog
            {
                let color = reflog.lane.map(|lane| color_picker.get_lane_ref(lane)).unwrap_or(theme.COLOR_TEXT);
                spans.push(Span::styled(format!("{} {} ", entity.reflog, reflog.selector), Style::default().fg(color)));
            }

            spans.push(Span::styled(row.summary.clone(), Style::default().fg(if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT })));
            lines.push(Line::from(spans));
        } else {
            let color = if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT };
            if uncommitted.conflict_count > 0 {
                spans.push(Span::styled(status.conflict_spaced.clone(), Style::default().fg(theme.COLOR_ORANGE)));
                spans.push(Span::styled(format!("{} ", uncommitted.conflict_count), Style::default().fg(theme.COLOR_ORANGE)));
            }
            if uncommitted.modified_count > 0 {
                spans.push(Span::styled(status.modified_spaced.clone(), Style::default().fg(theme.COLOR_BLUE)));
                spans.push(Span::styled(format!("{} ", uncommitted.modified_count), Style::default().fg(color)));
            }
            if uncommitted.added_count > 0 {
                spans.push(Span::styled(status.added_spaced.clone(), Style::default().fg(theme.COLOR_GREEN)));
                spans.push(Span::styled(format!("{} ", uncommitted.added_count), Style::default().fg(color)));
            }
            if uncommitted.deleted_count > 0 {
                spans.push(Span::styled(status.deleted_spaced.clone(), Style::default().fg(theme.COLOR_RED)));
                spans.push(Span::styled(format!("{} ", uncommitted.deleted_count), Style::default().fg(color)));
            }
            lines.push(Line::from(spans));
        }
    }

    lines
}

pub fn render_keybindings(theme: &Theme, keymap: &IndexMap<KeyBinding, Command>, width: usize) -> Vec<Line<'static>> {
    keymap
        .iter()
        .map(|(kb, cmd)| {
            // Build a human-readable key label from crossterm key parts.
            let mut key_string = modifiers_to_string(kb.modifiers);
            if !key_string.is_empty() {
                key_string = format!("{} + ", key_string);
            }
            key_string.push_str(&keycode_to_visual_string(kb.code));

            // Command enum names double as display labels after spacing.
            let mut cmd_string = format!("{:?}", cmd);
            cmd_string = pascal_to_spaced(&cmd_string);

            // Fill the middle so shortcuts line up on the right edge.
            let key_len = key_string.len();
            let cmd_len = cmd_string.len();
            let filler = " ";
            let mut filler_fill = 0;
            if width > key_len + cmd_len {
                filler_fill = (width - key_len - cmd_len).saturating_sub(4);
            }

            let fillers = filler.repeat(filler_fill.max(1));
            Line::from(Span::styled(truncate_with_ellipsis(format!(" {} {} {} ", cmd_string, fillers, key_string).as_str(), width), Style::default().fg(theme.COLOR_TEXT)))
                .alignment(ratatui::layout::Alignment::Center)
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/core/renderers.rs"]
mod tests;
