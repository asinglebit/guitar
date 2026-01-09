use crate::helpers::keymap::{Command, KeyBinding, keycode_to_visual_string};
use crate::{
    core::{
        chunk::{Chunk, NONE},
        oids::Oids,
    },
    git::queries::helpers::UncommittedChanges,
    helpers::{
        colors::ColorPicker,
        palette::*,
        symbols::*,
        text::{modifiers_to_string, pascal_to_spaced},
    },
    layers,
};
use git2::Repository;
use im::{HashSet, Vector};
use indexmap::IndexMap;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub fn render_graph_range(theme: &Theme, oids: &Oids, all: &HashMap<u32, Vec<String>>, history: &Vector<Vector<Chunk>>, head_alias: u32, start: usize, end: usize) -> Vec<Line<'static>> {
    let mut layers = layers!(Rc::new(RefCell::new(ColorPicker::from_theme(theme))));
    let mut lines: Vec<Line> = Vec::new();

    // Go through the sorted commits, inferring the graph
    let sorted_aliases = oids.get_sorted_aliases();
    for (global_idx, alias) in sorted_aliases.iter().enumerate().take(end).skip(start) {
        // Get commit oid
        let oid = oids.get_oid_by_alias(*alias);

        // Clear the render line
        layers.clear();
        let mut spans = vec![Span::raw(" ")];

        // Iterate over the buffer chunks, rendering the graph line
        let mut is_commit_found = false;
        let mut is_merged_before = false;
        let mut lane_idx = 0;

        if history.is_empty() {
            return vec![Line::default()];
        }
        let delta = (history.len() + global_idx).saturating_sub(end);
        let prev = if delta == 0 { None } else { history.get(delta - 1) };
        let last = history.get(delta).unwrap();

        if oids.is_zero(oid) {
            lines.push(Line::from(Span::styled(" ◌", Style::default().fg(theme.COLOR_GREY_400))));
            continue;
        }

        // Find branching lanes
        let mut branching_lanes: Vec<usize> = Vec::new();
        for (lane_idx, chunk) in last.iter().enumerate() {
            // Dummy in the end, chunk exists ont the same lane in prev
            if chunk.is_dummy()
                && let Some(prev_snapshot) = prev
                && let Some(prev) = prev_snapshot.get(lane_idx)
                && ((prev.parent_a != NONE && prev.parent_b == NONE) || (prev.parent_a == NONE && prev.parent_b != NONE))
            {
                branching_lanes.push(lane_idx);
                continue;
            }

            // Dummy in the end, while nothing existed on the same lane in the prev
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
                    layers.merge(SYM_EMPTY, closest_lane);
                    layers.merge(SYM_EMPTY, closest_lane);
                    layers.commit(SYM_EMPTY, closest_lane);
                    layers.commit(SYM_EMPTY, closest_lane);
                    layers.pipe(SYM_HORIZONTAL, closest_lane);
                    layers.pipe(SYM_HORIZONTAL, closest_lane);
                    lane_idx += 1;
                    continue;
                }
            }

            if chunk.is_dummy() {
                if let Some(prev_snapshot) = prev {
                    match prev_snapshot.get(lane_idx) {
                        Some(prev) => {
                            // Dummy in the end, chunk exists ont the same lane in prev
                            if (prev.parent_a != NONE && prev.parent_b == NONE) || (prev.parent_a == NONE && prev.parent_b != NONE) {
                                layers.commit(SYM_EMPTY, lane_idx);
                                layers.commit(SYM_EMPTY, lane_idx);
                                layers.pipe(SYM_BRANCH_UP, lane_idx);
                                layers.pipe(SYM_EMPTY, lane_idx);
                            } else {
                                layers.commit(SYM_EMPTY, lane_idx);
                                layers.commit(SYM_EMPTY, lane_idx);
                                layers.pipe(SYM_EMPTY, lane_idx);
                                layers.pipe(SYM_EMPTY, lane_idx);
                            }
                        },
                        None => {
                            // Dummy in the end, while nothing existed on the same lane in the prev
                            layers.commit(SYM_EMPTY, lane_idx);
                            layers.commit(SYM_EMPTY, lane_idx);
                            layers.pipe(SYM_BRANCH_UP, lane_idx);
                            layers.pipe(SYM_EMPTY, lane_idx);
                        },
                    }
                }
            } else if *alias == chunk.alias {
                is_commit_found = true;
                let is_two_parents = chunk.parent_a != NONE && chunk.parent_b != NONE;
                if is_two_parents && !(all.contains_key(alias)) {
                    layers.commit(SYM_MERGE, lane_idx);
                } else if all.contains_key(alias) {
                    layers.commit(SYM_COMMIT_BRANCH, lane_idx);
                } else if oids.stashes.contains(alias) {
                    layers.commit(SYM_COMMIT_STASH, lane_idx);
                } else {
                    layers.commit(SYM_COMMIT, lane_idx);
                }
                layers.commit(SYM_EMPTY, lane_idx);
                layers.pipe(SYM_EMPTY, lane_idx);
                layers.pipe(SYM_EMPTY, lane_idx);

                // Check if commit is being merged into
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
                        if *alias == chunk_nested.alias {
                            break;
                        }
                        mergee_idx += 1;
                    }

                    for (chunk_nested_idx, chunk_nested) in last.iter().enumerate() {
                        if !is_mergee_found {
                            if *alias == chunk_nested.alias {
                                is_mergee_found = true;
                                if is_merger_found {
                                    is_drawing = !is_drawing;
                                }
                                if !is_drawing {
                                    is_merged_before = true;
                                }
                                layers.merge(SYM_EMPTY, merger_idx);
                                layers.merge(SYM_EMPTY, merger_idx);
                            } else {
                                // Before the commit
                                if !is_merger_found {
                                    layers.merge(SYM_EMPTY, merger_idx);
                                    layers.merge(SYM_EMPTY, merger_idx);
                                } else if ((chunk_nested.parent_a != NONE && chunk_nested.parent_b == NONE) || (chunk_nested.parent_a == NONE && chunk_nested.parent_b != NONE))
                                    && (chunk.parent_a == chunk_nested.parent_a || chunk.parent_b == chunk_nested.parent_a)
                                {
                                    // We need to find if the merger is further to the left than on the next lane
                                    if chunk_nested_idx == merger_idx {
                                        layers.merge(SYM_MERGE_RIGHT_FROM, merger_idx);
                                    } else {
                                        layers.merge(SYM_HORIZONTAL, merger_idx);
                                    }

                                    if chunk_nested_idx + 1 == mergee_idx {
                                        layers.merge(SYM_EMPTY, merger_idx);
                                    } else {
                                        layers.merge(SYM_HORIZONTAL, merger_idx);
                                    }
                                    is_drawing = true;
                                } else if is_drawing {
                                    if chunk_nested_idx + 1 == mergee_idx {
                                        layers.merge(SYM_HORIZONTAL, merger_idx);
                                        layers.merge(SYM_EMPTY, merger_idx);
                                    } else {
                                        layers.merge(SYM_HORIZONTAL, merger_idx);
                                        layers.merge(SYM_HORIZONTAL, merger_idx);
                                    }
                                } else {
                                    layers.merge(SYM_EMPTY, merger_idx);
                                    layers.merge(SYM_EMPTY, merger_idx);
                                }
                            }
                        } else {
                            // After the commit
                            if is_merger_found && !is_merged_before {
                                if ((chunk_nested.parent_a != NONE && chunk_nested.parent_b == NONE) || (chunk_nested.parent_a == NONE && chunk_nested.parent_b != NONE))
                                    && (chunk.parent_a == chunk_nested.parent_a || chunk.parent_b == chunk_nested.parent_a)
                                {
                                    layers.merge(SYM_MERGE_LEFT_FROM, merger_idx);
                                    layers.merge(SYM_EMPTY, merger_idx);
                                    is_merged_before = true;
                                    is_drawing = false;
                                } else if is_drawing {
                                    layers.merge(SYM_HORIZONTAL, merger_idx);
                                    layers.merge(SYM_HORIZONTAL, merger_idx);
                                } else {
                                    layers.merge(SYM_EMPTY, merger_idx);
                                    layers.merge(SYM_EMPTY, merger_idx);
                                }
                            }
                        }
                    }

                    if !is_merger_found {
                        // Count how many dummies in the end to get the real last element, append there
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

                        // Meet some corner cases against the previous buffer line - if there are further branches
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
                            layers.merge(SYM_BRANCH_DOWN, idx + 1);
                            layers.merge(SYM_EMPTY, idx + 1);
                        } else if trailing_dummies > 0 {
                            // Calculate how many lanes before we reach the branch character
                            for _ in lane_idx..idx {
                                layers.merge(SYM_HORIZONTAL, idx + 1);
                                layers.merge(SYM_HORIZONTAL, idx + 1);
                            }

                            layers.merge(SYM_MERGE_LEFT_FROM, idx + 1);
                            layers.merge(SYM_EMPTY, idx + 1);
                        } else {
                            // Calculate how many lanes before we reach the branch character
                            for _ in lane_idx..idx {
                                layers.merge(SYM_HORIZONTAL, idx + 1);
                                layers.merge(SYM_HORIZONTAL, idx + 1);
                            }

                            layers.merge(SYM_BRANCH_DOWN, idx + 1);
                            layers.merge(SYM_EMPTY, idx + 1);
                        }
                    }
                }
            } else {
                layers.commit(SYM_EMPTY, lane_idx);
                layers.commit(SYM_EMPTY, lane_idx);
                if (chunk.parent_a == head_alias || chunk.parent_b == head_alias) && lane_idx == 0 {
                    layers.pipe_custom(SYM_VERTICAL_DOTTED, lane_idx, theme.COLOR_GREY_500);
                } else if chunk.parent_a == NONE && chunk.parent_b == NONE {
                    layers.pipe(" ", lane_idx);
                } else {
                    layers.pipe(SYM_VERTICAL, lane_idx);
                }
                layers.pipe(SYM_EMPTY, lane_idx);
            }

            lane_idx += 1;
        }

        if !is_commit_found {
            if all.contains_key(alias) {
                layers.commit(SYM_COMMIT_BRANCH, lane_idx);
            } else {
                layers.commit(SYM_COMMIT, lane_idx);
            };
            layers.commit(SYM_EMPTY, lane_idx);
            layers.pipe(SYM_EMPTY, lane_idx);
            layers.pipe(SYM_EMPTY, lane_idx);
        }

        // Blend layers into the graph
        layers.bake(&mut spans);

        // Render
        lines.push(Line::from(spans));
    }

    remove_empty_columns(&mut lines);
    lines
}

pub fn remove_empty_columns(lines: &mut Vec<Line<'_>>) {
    let mut non_empty_counts: HashMap<usize, usize> = HashMap::new();

    // Count non-empty "pairs" of spans per column
    for line in lines.iter() {
        let spans = &line.spans;
        let mut idx = 0;
        while idx + 1 < spans.len() {
            let a = &spans[idx];
            let b = &spans[idx + 1];
            let x = non_empty_counts.entry(idx).or_insert(0);
            if a.content != " " && a.content != "─" || b.content != " " && b.content != "─" {
                *x += 1;
            }
            idx += 2; // move to next pair
        }
    }

    // Find indices (first span of pair) that are empty in all rows
    let empty_indices: HashSet<usize> = non_empty_counts.iter().filter_map(|(&idx, &count)| if count == 0 { Some(idx) } else { None }).collect();

    // Rebuild each line without empty span pairs
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
        // Handle odd span at the end if exists
        if idx < line.spans.len() {
            new_spans.push(line.spans[idx].clone());
        }
        *line = Line::from(new_spans);
    }
}

#[allow(dead_code)]
pub fn render_buffer_range(theme: &Theme, oids: &Oids, history: &Vector<Vector<Chunk>>, start: usize, end: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for global_idx in start..end {
        if history.is_empty() {
            lines.push(Line::default());
            continue;
        }

        let delta = (history.len() + global_idx).saturating_sub(end);
        let snapshot = match history.get(delta) {
            Some(s) => s,
            None => {
                lines.push(Line::default());
                continue;
            },
        };

        let oid = oids.get_oid_by_idx(global_idx);

        let mut spans = vec![Span::styled(format!("{:.2} ", oid), Style::default().fg(theme.COLOR_TEXT))];

        let formatted_snapshot = snapshot
            .iter()
            .map(|chunk| {
                let oid_str = if chunk.alias == NONE { "".to_string() } else { format!("{:.2}", oids.get_oid_by_alias(chunk.alias).to_string()) };

                let parents_formatted = match (chunk.parent_a, chunk.parent_b) {
                    (NONE, NONE) => "".to_string(),
                    (a, NONE) => format!("{:.2},--", oids.get_oid_by_alias(a)),
                    (NONE, b) => format!("--,{:.2}", oids.get_oid_by_alias(b)),
                    (a, b) => format!("{:.2},{:.2}", oids.get_oid_by_alias(a), oids.get_oid_by_alias(b)),
                };

                format!("{}({})", oid_str, parents_formatted)
            })
            .collect::<Vec<_>>()
            .join(" ");

        spans.push(Span::styled(formatted_snapshot, Style::default().fg(theme.COLOR_TEXT)));

        lines.push(Line::from(spans));
    }

    lines
}

pub fn render_sha_range(theme: &Theme, oids: &Oids, start: usize, end: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for global_idx in start..end {
        let alias = oids.get_alias_by_idx(global_idx);

        if alias != NONE {
            let oid = oids.get_oid_by_alias(alias);

            lines.push(Line::from(Span::styled(format!("{:.9} ", oid), Style::default().fg(theme.COLOR_GREY_700))));
        } else {
            lines.push(Line::from(""));
        }
    }

    lines
}

#[allow(clippy::too_many_arguments)]
pub fn render_message_range(
    theme: &Theme, repo: &Repository, oids: &Oids, local: &HashMap<u32, Vec<String>>, visible: &HashMap<u32, Vec<String>>, tags: &HashMap<u32, Vec<String>>, branch_colors: &mut HashMap<u32, Color>,
    tag_colors: &mut HashMap<u32, Color>, stashes_colors: &mut HashMap<u32, Color>, start: usize, end: usize, selected: usize, uncommitted: &UncommittedChanges,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    // Go through the commits, inferring the graph
    for global_idx in start..end {
        let alias = oids.get_alias_by_idx(global_idx);
        let mut spans = Vec::new();

        if alias != NONE {
            let oid = oids.get_oid_by_alias(alias);
            let commit = repo.find_commit(*oid).unwrap();

            if let Some(visible) = visible.get(&alias) {
                for branch in visible {
                    // Only render branches that are visible
                    if visible.iter().any(|b| b == branch) {
                        // Check if the branch is local
                        let is_local = local.values().any(|branches| branches.iter().any(|b| b.as_str() == branch));

                        spans.push(Span::styled(
                            format!("{} {} ", if is_local { SYM_COMMIT_BRANCH } else { "◆" }, branch),
                            Style::default().fg(if let Some(color) = branch_colors.get(&alias) { *color } else { theme.COLOR_TEXT }),
                        ));
                    }
                }
            }

            if let Some(tags) = tags.get(&alias) {
                for tag in tags {
                    // Render tags
                    if tags.iter().any(|b| b == tag) {
                        spans.push(Span::styled(format!("{} {} ", SYM_TAG, tag), Style::default().fg(if let Some(color) = tag_colors.get(&alias) { *color } else { theme.COLOR_TEXT })));
                    }
                }
            }

            if oids.stashes.contains(&alias) {
                spans.push(Span::styled(format!("{SYM_COMMIT_STASH} stash "), Style::default().fg(if let Some(color) = stashes_colors.get(&alias) { *color } else { theme.COLOR_TEXT })));
            }

            spans.push(Span::styled(commit.summary().unwrap_or("⊘ no message").to_string(), Style::default().fg(if global_idx == selected { theme.COLOR_GREY_500 } else { theme.COLOR_TEXT })));

            lines.push(Line::from(spans));
        } else {
            let color = if global_idx == selected { theme.COLOR_GREY_500 } else { theme.COLOR_GREY_600 };
            if uncommitted.modified_count > 0 {
                spans.push(Span::styled("~ ", Style::default().fg(theme.COLOR_BLUE)));
                spans.push(Span::styled(format!("{} ", uncommitted.modified_count), Style::default().fg(color)));
            }
            if uncommitted.added_count > 0 {
                spans.push(Span::styled("+ ", Style::default().fg(theme.COLOR_GREEN)));
                spans.push(Span::styled(format!("{} ", uncommitted.added_count), Style::default().fg(color)));
            }
            if uncommitted.deleted_count > 0 {
                spans.push(Span::styled("- ", Style::default().fg(theme.COLOR_RED)));
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
            // Build key string
            let mut key_string = modifiers_to_string(kb.modifiers);
            if !key_string.is_empty() {
                key_string = format!("{} + ", key_string);
            }
            key_string.push_str(&keycode_to_visual_string(kb.code));

            // Command string
            let mut cmd_string = format!("{:?}", cmd);
            cmd_string = pascal_to_spaced(&cmd_string);

            // Calculate available space for filler
            let key_len = key_string.len();
            let cmd_len = cmd_string.len();
            let filler = " ";
            let mut filler_fill = 0;
            if width > key_len + cmd_len {
                filler_fill = (width - key_len - cmd_len).saturating_sub(4); // -2 for spaces
            }

            let fillers = filler.repeat(filler_fill.max(1)); // at least one

            Line::from(vec![
                Span::styled(format!(" {}", cmd_string), Style::default().fg(theme.COLOR_TEXT)),
                Span::styled(format!(" {} ", fillers), Style::default().fg(theme.COLOR_GREY_800)),
                Span::styled(format!("{} ", key_string), Style::default().fg(theme.COLOR_TEXT)),
            ])
            .alignment(ratatui::layout::Alignment::Center)
        })
        .collect()
}
