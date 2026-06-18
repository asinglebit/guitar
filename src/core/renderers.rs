use crate::core::graph_service::{GraphHistory, GraphRow};
use crate::helpers::text::truncate_with_ellipsis;
use crate::helpers::{
    keymap::{Command, KeyBinding, keycode_to_visual_string},
    localisation::status as status_text,
};
use crate::{
    core::chunk::NONE,
    git::queries::helpers::UncommittedChanges,
    helpers::{
        colors::ColorPicker,
        palette::*,
        symbols::{branch as branch_symbol, entity, graph, status, worktree},
        text::{modifiers_to_string, pascal_to_spaced},
    },
    layers,
};
use im::HashSet;
use indexmap::IndexMap;
use ratatui::{
    style::Style,
    text::{Line, Span},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub const GRAPH_COMMITTER_WIDTH: usize = 18;

// Render graph symbols from worker-projected rows. The lane history is still
// precomputed by Buffer, but only for the requested visible range.
pub fn render_graph_projection(theme: &Theme, rows: &[GraphRow], history: &GraphHistory, head_alias: u32, start: usize, end: usize, render_uncommitted_row: bool) -> Vec<Line<'static>> {
    let mut layers = layers!(Rc::new(RefCell::new(ColorPicker::from_theme(theme))));
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

        if row.alias == NONE {
            lines.push(Line::from(Span::styled(format!(" {}", graph::UNCOMMITTED), Style::default().fg(theme.COLOR_GREY_400))));
            continue;
        }

        let mut branching_lanes: Vec<usize> = Vec::new();
        for (lane_idx, chunk) in last.iter().enumerate() {
            if chunk.is_dummy()
                && let Some(prev_snapshot) = prev
                && let Some(prev) = prev_snapshot.get(lane_idx)
                && ((prev.parent_a != NONE && prev.parent_b == NONE) || (prev.parent_a == NONE && prev.parent_b != NONE))
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

        for chunk in last.iter() {
            if is_commit_found
                && !branching_lanes.is_empty()
                && let Some(&closest_lane) = branching_lanes.first()
            {
                if closest_lane == lane_idx {
                    branching_lanes.remove(0);
                } else if lane_idx < closest_lane {
                    layers.merge(graph::EMPTY, closest_lane);
                    layers.merge(graph::EMPTY, closest_lane);
                    layers.commit(graph::EMPTY, closest_lane);
                    layers.commit(graph::EMPTY, closest_lane);
                    layers.pipe(graph::HORIZONTAL, closest_lane);
                    layers.pipe(graph::HORIZONTAL, closest_lane);
                    lane_idx += 1;
                    continue;
                }
            }

            if chunk.is_dummy() {
                if let Some(prev_snapshot) = prev {
                    match prev_snapshot.get(lane_idx) {
                        Some(prev) => {
                            if (prev.parent_a != NONE && prev.parent_b == NONE) || (prev.parent_a == NONE && prev.parent_b != NONE) {
                                layers.commit(graph::EMPTY, lane_idx);
                                layers.commit(graph::EMPTY, lane_idx);
                                layers.pipe(graph::BRANCH_UP, lane_idx);
                                layers.pipe(graph::EMPTY, lane_idx);
                            } else {
                                layers.commit(graph::EMPTY, lane_idx);
                                layers.commit(graph::EMPTY, lane_idx);
                                layers.pipe(graph::EMPTY, lane_idx);
                                layers.pipe(graph::EMPTY, lane_idx);
                            }
                        },
                        None => {
                            layers.commit(graph::EMPTY, lane_idx);
                            layers.commit(graph::EMPTY, lane_idx);
                            layers.pipe(graph::BRANCH_UP, lane_idx);
                            layers.pipe(graph::EMPTY, lane_idx);
                        },
                    }
                }
            } else if row.alias == chunk.alias {
                is_commit_found = true;
                let is_two_parents = chunk.parent_a != NONE && chunk.parent_b != NONE;
                if is_two_parents && !row.has_any_branch {
                    layers.commit(graph::MERGE, lane_idx);
                } else if row.has_any_branch {
                    layers.commit(graph::COMMIT_BRANCH, lane_idx);
                } else if row.worktrees.iter().any(|entry| entry.branch.is_none() || !row.has_any_branch) {
                    layers.commit(worktree::CURRENT, lane_idx);
                } else if row.is_stash {
                    layers.commit(graph::COMMIT_STASH, lane_idx);
                } else {
                    layers.commit(graph::COMMIT, lane_idx);
                }
                layers.commit(graph::EMPTY, lane_idx);
                layers.pipe(graph::EMPTY, lane_idx);
                layers.pipe(graph::EMPTY, lane_idx);

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
                                layers.merge(graph::EMPTY, merger_idx);
                                layers.merge(graph::EMPTY, merger_idx);
                            } else if !is_merger_found {
                                layers.merge(graph::EMPTY, merger_idx);
                                layers.merge(graph::EMPTY, merger_idx);
                            } else if ((chunk_nested.parent_a != NONE && chunk_nested.parent_b == NONE) || (chunk_nested.parent_a == NONE && chunk_nested.parent_b != NONE))
                                && (chunk.parent_a == chunk_nested.parent_a || chunk.parent_b == chunk_nested.parent_a)
                            {
                                if chunk_nested_idx == merger_idx {
                                    layers.merge(graph::MERGE_RIGHT_FROM, merger_idx);
                                } else {
                                    layers.merge(graph::HORIZONTAL, merger_idx);
                                }

                                if chunk_nested_idx + 1 == mergee_idx {
                                    layers.merge(graph::EMPTY, merger_idx);
                                } else {
                                    layers.merge(graph::HORIZONTAL, merger_idx);
                                }
                                is_drawing = true;
                            } else if is_drawing {
                                if chunk_nested_idx + 1 == mergee_idx {
                                    layers.merge(graph::HORIZONTAL, merger_idx);
                                    layers.merge(graph::EMPTY, merger_idx);
                                } else {
                                    layers.merge(graph::HORIZONTAL, merger_idx);
                                    layers.merge(graph::HORIZONTAL, merger_idx);
                                }
                            } else {
                                layers.merge(graph::EMPTY, merger_idx);
                                layers.merge(graph::EMPTY, merger_idx);
                            }
                        } else if is_merger_found && !is_merged_before {
                            if ((chunk_nested.parent_a != NONE && chunk_nested.parent_b == NONE) || (chunk_nested.parent_a == NONE && chunk_nested.parent_b != NONE))
                                && (chunk.parent_a == chunk_nested.parent_a || chunk.parent_b == chunk_nested.parent_a)
                            {
                                layers.merge(graph::MERGE_LEFT_FROM, merger_idx);
                                layers.merge(graph::EMPTY, merger_idx);
                                is_merged_before = true;
                                is_drawing = false;
                            } else if is_drawing {
                                layers.merge(graph::HORIZONTAL, merger_idx);
                                layers.merge(graph::HORIZONTAL, merger_idx);
                            } else {
                                layers.merge(graph::EMPTY, merger_idx);
                                layers.merge(graph::EMPTY, merger_idx);
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

                        if trailing_dummies > 0 && prev.is_some() && prev.unwrap().len() > idx + 1 && prev.unwrap()[idx + 1].is_dummy() {
                            layers.merge(graph::BRANCH_DOWN, idx + 1);
                            layers.merge(graph::EMPTY, idx + 1);
                        } else if trailing_dummies > 0 {
                            for _ in lane_idx..idx {
                                layers.merge(graph::HORIZONTAL, idx + 1);
                                layers.merge(graph::HORIZONTAL, idx + 1);
                            }

                            layers.merge(graph::MERGE_LEFT_FROM, idx + 1);
                            layers.merge(graph::EMPTY, idx + 1);
                        } else {
                            for _ in lane_idx..idx {
                                layers.merge(graph::HORIZONTAL, idx + 1);
                                layers.merge(graph::HORIZONTAL, idx + 1);
                            }

                            layers.merge(graph::BRANCH_DOWN, idx + 1);
                            layers.merge(graph::EMPTY, idx + 1);
                        }
                    }
                }
            } else {
                layers.commit(graph::EMPTY, lane_idx);
                layers.commit(graph::EMPTY, lane_idx);
                if (chunk.parent_a == head_alias || chunk.parent_b == head_alias) && lane_idx == 0 {
                    layers.pipe_custom(graph::VERTICAL_DOTTED, lane_idx, theme.COLOR_GREY_500);
                } else if chunk.parent_a == NONE && chunk.parent_b == NONE {
                    layers.pipe(" ", lane_idx);
                } else {
                    layers.pipe(graph::VERTICAL, lane_idx);
                }
                layers.pipe(graph::EMPTY, lane_idx);
            }

            lane_idx += 1;
        }

        if !is_commit_found {
            if row.has_any_branch {
                layers.commit(graph::COMMIT_BRANCH, lane_idx);
            } else if row.worktrees.iter().any(|entry| entry.branch.is_none() || !row.has_any_branch) {
                layers.commit(worktree::CURRENT, lane_idx);
            } else {
                layers.commit(graph::COMMIT, lane_idx);
            };
            layers.commit(graph::EMPTY, lane_idx);
            layers.pipe(graph::EMPTY, lane_idx);
            layers.pipe(graph::EMPTY, lane_idx);
        }

        layers.bake(&mut spans);
        lines.push(Line::from(spans));
    }

    remove_empty_columns(&mut lines);
    let _ = start;
    lines
}

// Remove graph lane pairs that are visually empty across every rendered row.
pub fn remove_empty_columns(lines: &mut Vec<Line<'_>>) {
    let mut non_empty_counts: HashMap<usize, usize> = HashMap::new();

    // Graph lanes occupy two spans, so pruning must happen in span pairs.
    for line in lines.iter() {
        let spans = &line.spans;
        let mut idx = 0;
        while idx + 1 < spans.len() {
            let a = &spans[idx];
            let b = &spans[idx + 1];
            let x = non_empty_counts.entry(idx).or_insert(0);
            if a.content != graph::EMPTY && a.content != graph::HORIZONTAL || b.content != graph::EMPTY && b.content != graph::HORIZONTAL {
                *x += 1;
            }
            idx += 2;
        }
    }

    // Missing entries are not empty; only recorded zero-count pairs are removed.
    let empty_indices: HashSet<usize> = non_empty_counts.iter().filter_map(|(&idx, &count)| if count == 0 { Some(idx) } else { None }).collect();

    for line in lines.iter_mut() {
        let mut new_spans: Vec<Span> = Vec::with_capacity(line.spans.len());
        let mut idx = 0;
        while idx + 1 < line.spans.len() {
            if !empty_indices.contains(&idx) {
                new_spans.push(line.spans[idx].clone());
                new_spans.push(line.spans[idx + 1].clone());
            }
            idx += 2;
        }
        // Preserve any leading or trailing single span outside lane pairs.
        if idx < line.spans.len() {
            new_spans.push(line.spans[idx].clone());
        }
        *line = Line::from(new_spans);
    }
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
    theme: &Theme, rows: &[GraphRow], show_reflog_labels: bool, show_ref_labels: bool, selected: usize, uncommitted: &UncommittedChanges, render_uncommitted_row: bool,
) -> Vec<Line<'static>> {
    let color_picker = ColorPicker::from_theme(theme);
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
                    spans.push(Span::styled(format!("{}{} ", worktree::CURRENT, worktree.name), Style::default().fg(color)));
                }
            }
            let has_worktree_label = show_ref_labels && !row.worktrees.is_empty();

            if show_ref_labels {
                for branch in &row.branches {
                    let color = branch.lane.map(|lane| color_picker.get_lane(lane)).unwrap_or(theme.COLOR_TEXT);
                    spans.push(Span::styled(format!("{} {} ", if branch.is_local { branch_symbol::LOCAL_VISIBLE } else { branch_symbol::REMOTE_VISIBLE }, branch.name), Style::default().fg(color)));
                }
            }
            let has_visible_branch_label = show_ref_labels && !row.branches.is_empty();

            if show_ref_labels {
                for tag in &row.tags {
                    let color = tag.lane.map(|lane| color_picker.get_lane(lane)).unwrap_or(theme.COLOR_TEXT);
                    spans.push(Span::styled(format!("{} {} ", entity::TAG, tag.name), Style::default().fg(color)));
                }
            }
            let has_tag_label = show_ref_labels && !row.tags.is_empty();

            if show_ref_labels && row.is_stash {
                let color = row.stash_lane.map(|lane| color_picker.get_lane(lane)).unwrap_or(theme.COLOR_TEXT);
                spans.push(Span::styled(format!("{} {} ", graph::COMMIT_STASH, status_text::STASH), Style::default().fg(color)));
            }
            let has_stash_label = show_ref_labels && row.is_stash;

            if show_reflog_labels
                && !has_visible_branch_label
                && !has_tag_label
                && !has_stash_label
                && !has_worktree_label
                && let Some(reflog) = &row.reflog
            {
                let color = reflog.lane.map(|lane| color_picker.get_lane(lane)).unwrap_or(theme.COLOR_TEXT);
                spans.push(Span::styled(format!("{} {} ", entity::REFLOG, reflog.selector), Style::default().fg(color)));
            }

            spans.push(Span::styled(row.summary.clone(), Style::default().fg(if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT })));
            lines.push(Line::from(spans));
        } else {
            let color = if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT };
            if uncommitted.conflict_count > 0 {
                spans.push(Span::styled(status::CONFLICT_SPACED, Style::default().fg(theme.COLOR_ORANGE)));
                spans.push(Span::styled(format!("{} ", uncommitted.conflict_count), Style::default().fg(theme.COLOR_ORANGE)));
            }
            if uncommitted.modified_count > 0 {
                spans.push(Span::styled(status::MODIFIED_SPACED, Style::default().fg(theme.COLOR_BLUE)));
                spans.push(Span::styled(format!("{} ", uncommitted.modified_count), Style::default().fg(color)));
            }
            if uncommitted.added_count > 0 {
                spans.push(Span::styled(status::ADDED_SPACED, Style::default().fg(theme.COLOR_GREEN)));
                spans.push(Span::styled(format!("{} ", uncommitted.added_count), Style::default().fg(color)));
            }
            if uncommitted.deleted_count > 0 {
                spans.push(Span::styled(status::DELETED_SPACED, Style::default().fg(theme.COLOR_RED)));
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
mod tests {
    use super::*;
    use crate::core::{
        chunk::NONE,
        graph_service::{GraphBranchLabel, GraphReflogLabel, GraphTagLabel},
        worktrees::{WorktreeEntry, WorktreeKind},
    };
    use git2::Oid;
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
        let rows =
            vec![graph_row(0, Oid::from_str("1111111111111111111111111111111111111111").unwrap(), "first"), graph_row(1, Oid::from_str("2222222222222222222222222222222222222222").unwrap(), "second")];

        let lines = render_message_projection(&theme, &rows, false, true, 1, &UncommittedChanges::default(), true);

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
    fn message_projection_toggles_refs_without_hiding_reflog_labels() {
        let theme = Theme::classic();
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

        let shown = render_message_projection(&theme, &[row.clone()], true, true, 0, &UncommittedChanges::default(), true);
        let hidden = render_message_projection(&theme, &[row], true, false, 0, &UncommittedChanges::default(), true);
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
}
