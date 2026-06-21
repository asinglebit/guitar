use crate::app::app::{App, Focus};
use crate::core::{
    chunk::NONE,
    graph_service::GraphRow,
    renderers::{GRAPH_COMMITTER_WIDTH, render_graph_projection, render_message_projection},
};
use crate::helpers::text::truncate_with_ellipsis;
use crate::helpers::{layout::scrollbar_content_length, localisation::empty};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{
    style::Style,
    widgets::{Block, Borders, Cell as WidgetCell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};
use std::collections::HashSet;

impl App {
    pub fn draw_graph(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        if self.layout.graph.width == 0 || self.layout.graph.height == 0 {
            return;
        }

        // Determine the visible graph window before requesting projected rows.
        let total_lines = self.graph_commit_count();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };

        let previous_selected = self.graph_selected;
        if total_lines == 0 {
            self.graph_selected = 0;
        } else if self.graph_selected >= total_lines {
            self.graph_selected = total_lines.saturating_sub(1);
        }
        if self.graph_selected != previous_selected {
            self.current_diff.clear();
            self.current_diff_identity = None;
            if self.graph_selected != 0
                && let Some(identity) = self.graph_identity_at(self.graph_selected)
            {
                self.current_diff = crate::git::queries::diffs::get_filenames_diff_at_oid(repo, identity.oid);
                self.current_diff_identity = Some(identity);
            }
        }

        self.trap_selection(self.graph_selected, &self.graph_scroll, total_lines, visible_height);

        let start = self.graph_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        let (preload_start, preload_end) = graph_preload_window(start, end, total_lines, visible_height);
        self.request_graph_window(preload_start, preload_end);

        // The graph service reports completion with zero rows for unborn repositories.
        if self.graph.total == 0 && self.graph.is_complete {
            let table = Table::new(graph_backdrop_rows(visible_height, 0, None, &self.theme), [ratatui::layout::Constraint::Min(0)])
                .block(Block::default().borders(Borders::LEFT | Borders::RIGHT).border_style(Style::default().fg(self.theme.COLOR_BORDER)))
                .column_spacing(0);

            frame.render_widget(table, self.layout.graph);

            let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Length(3), Constraint::Percentage(50)]).split(self.layout.graph);

            let message = Paragraph::new(format!("{} {}", self.symbols.empty_state.mark, empty::NO_COMMITS())).alignment(Alignment::Center).style(Style::default().fg(self.theme.COLOR_BORDER));

            frame.render_widget(message, chunks[1]);
            return;
        }

        let visible_len = end.saturating_sub(start);
        let (cached_start, cached_rows, graph_lines, message_lines) = if let Some(window) = self.graph.graph_window.as_ref().filter(|window| window.start < end && start < window.end) {
            // SHA, graph, and message columns are rendered from the cached window, then reindexed
            // into the requested viewport so scrolling still looks like movement while loading.
            let render_uncommitted_row = graph_window_has_stable_visible_page(window, start, end);
            let source_graph = render_graph_projection(&self.theme, &self.symbols, &window.rows, &window.history, window.head_alias, window.start, window.end, render_uncommitted_row);
            let source_message = render_message_projection(
                &self.theme,
                &self.symbols,
                &window.rows,
                self.layout_config.is_graph_reflogs,
                self.layout_config.is_graph_refs,
                self.graph_selected,
                &self.uncommitted,
                render_uncommitted_row,
            );

            (window.start, window.rows.as_slice(), source_graph, source_message)
        } else {
            (start, &[][..], blank_projection(0), blank_projection(0))
        };

        // Build table rows and measure the graph column from rendered span widths.
        let mut rows = Vec::with_capacity(visible_height);
        let width = (start..end)
            .filter_map(|index| projected_line(&graph_lines, cached_start, index))
            .map(|line| line.spans.iter().filter(|span| !span.content.is_empty()).map(|span| span.content.chars().count()).sum::<usize>())
            .max()
            .unwrap_or(0) as u16;
        let search_highlight_indices: HashSet<usize> =
            if self.layout_config.is_search && self.search_path.is_some() { self.search_rows.iter().map(|row| row.graph_index).filter(|&index| index != 0).collect() } else { HashSet::new() };
        for idx in 0..visible_height {
            let global_idx = idx + start;

            let mut row = Row::new(graph_row_cells(
                cached_rows,
                self.layout_config.is_shas,
                &graph_lines,
                self.layout_config.is_graph_dates,
                self.layout_config.is_graph_committers,
                &message_lines,
                cached_start,
                global_idx,
                self.graph_selected,
                &self.theme,
            ));

            // Selection highlighting is focus-sensitive so inactive panes stay quiet.
            let is_selected = idx < visible_len && global_idx == self.graph_selected && self.focus == Focus::Viewport;
            let is_search_highlighted = idx < visible_len && search_highlight_indices.contains(&global_idx);
            if is_selected || is_search_highlighted {
                row = row.style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)));
            } else if global_idx.is_multiple_of(2) {
                row = row.style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)));
            }

            rows.push(row);
        }

        // The graph column is fixed to its measured width; message text gets the rest.
        let mut constraints = Vec::new();
        if self.layout_config.is_shas {
            constraints.push(ratatui::layout::Constraint::Length(9));
        }
        constraints.push(ratatui::layout::Constraint::Length(width + 5));
        if self.layout_config.is_graph_dates {
            constraints.push(ratatui::layout::Constraint::Length(16));
        }
        if self.layout_config.is_graph_committers {
            constraints.push(ratatui::layout::Constraint::Length(GRAPH_COMMITTER_WIDTH as u16));
        }
        constraints.push(ratatui::layout::Constraint::Min(0));

        if self.layout_config.is_zen {
            // Zen mode owns the full rounded graph frame.
            let table = Table::new(rows, constraints)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_set(self.symbols.border.block_set()))
                .column_spacing(1);

            frame.render_widget(table, self.layout.graph);

            if total_lines > visible_height {
                let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(self.graph_scroll.get());
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
                    .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
                    .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                    .thumb_symbol(self.symbols.scrollbar.thumb.as_str())
                    .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

                frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);
            }

            return;
        }

        // Normal mode draws only side borders because title and status bars provide the rest.
        let table = Table::new(rows, constraints)
            .block(Block::default().borders(Borders::RIGHT | Borders::LEFT).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_set(self.symbols.border.block_set()))
            .column_spacing(1);

        frame.render_widget(table, self.layout.graph);

        if total_lines > visible_height {
            let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(self.graph_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(if (self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts)) || self.layout_config.is_status {
                    Some(self.symbols.border.horizontal.as_str())
                } else {
                    Some(self.symbols.scrollbar.begin.as_str())
                })
                .end_symbol(if (self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts)) || self.layout_config.is_status {
                    Some(self.symbols.border.horizontal.as_str())
                } else {
                    Some(self.symbols.scrollbar.end.as_str())
                })
                .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                .thumb_symbol(self.symbols.scrollbar.thumb.as_str())
                .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);
        }
    }
}

fn blank_projection(len: usize) -> Vec<Line<'static>> {
    vec![Line::default(); len]
}

fn graph_backdrop_rows<'a>(visible_height: usize, start: usize, selected: Option<usize>, theme: &crate::helpers::palette::Theme) -> Vec<Row<'a>> {
    (0..visible_height)
        .map(|idx| {
            let global_idx = start + idx;
            let mut row = Row::new([WidgetCell::from(Line::default())]);
            if selected == Some(global_idx) {
                row = row.style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_800)));
            } else if global_idx.is_multiple_of(2) {
                row = row.style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_900)));
            }
            row
        })
        .collect()
}

fn graph_window_has_stable_visible_page(window: &crate::app::app::GraphWindowCache, target_start: usize, target_end: usize) -> bool {
    let cached_len = window.end.saturating_sub(window.start);
    window.start <= target_start && target_end <= window.end && window.rows.len() >= cached_len && window.history.len() >= cached_len
}

fn graph_preload_window(start: usize, end: usize, total_lines: usize, visible_height: usize) -> (usize, usize) {
    if visible_height == 0 {
        return (start, end);
    }

    (start.saturating_sub(visible_height), end.saturating_add(visible_height).min(total_lines))
}

fn projected_line<'a, 'line>(lines: &'a [Line<'line>], cached_start: usize, target_index: usize) -> Option<&'a Line<'line>> {
    target_index.checked_sub(cached_start).and_then(|source_index| lines.get(source_index))
}

fn graph_row_cells<'a, 'line>(
    rows: &'a [GraphRow], is_shas: bool, graph_lines: &'a [Line<'line>], is_dates: bool, is_committers: bool, message_lines: &'a [Line<'static>], cached_start: usize, global_idx: usize,
    selected: usize, theme: &crate::helpers::palette::Theme,
) -> impl Iterator<Item = WidgetCell<'a>>
where
    'line: 'a,
{
    let row = projected_row(rows, cached_start, global_idx);
    [
        is_shas.then(|| sha_cell(row, selected, theme)),
        Some(WidgetCell::from(projected_line(graph_lines, cached_start, global_idx).cloned().unwrap_or_default())),
        is_dates.then(|| date_cell(row, selected, theme)),
        is_committers.then(|| committer_cell(row, selected, theme)),
        Some(WidgetCell::from(projected_line(message_lines, cached_start, global_idx).cloned().unwrap_or_default())),
    ]
    .into_iter()
    .flatten()
}

fn projected_row(rows: &[GraphRow], cached_start: usize, target_index: usize) -> Option<&GraphRow> {
    target_index.checked_sub(cached_start).and_then(|source_index| rows.get(source_index))
}

fn text_color(row: &GraphRow, selected: usize, theme: &crate::helpers::palette::Theme) -> ratatui::style::Color {
    if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT }
}

fn sha_cell<'a>(row: Option<&'a GraphRow>, selected: usize, theme: &crate::helpers::palette::Theme) -> WidgetCell<'a> {
    let Some(row) = row.filter(|row| row.alias != NONE) else {
        return WidgetCell::from(Line::default());
    };
    WidgetCell::from(Line::from(Span::styled(row.short_oid.as_str(), Style::default().fg(text_color(row, selected, theme)))))
}

fn date_cell<'a>(row: Option<&'a GraphRow>, selected: usize, theme: &crate::helpers::palette::Theme) -> WidgetCell<'a> {
    let Some(row) = row.filter(|row| row.alias != NONE) else {
        return WidgetCell::from(Line::default());
    };
    WidgetCell::from(Line::from(Span::styled(row.committer_date.as_str(), Style::default().fg(text_color(row, selected, theme)))))
}

fn committer_cell(row: Option<&GraphRow>, selected: usize, theme: &crate::helpers::palette::Theme) -> WidgetCell<'static> {
    let Some(row) = row.filter(|row| row.alias != NONE) else {
        return WidgetCell::from(Line::default());
    };
    let truncated = truncate_with_ellipsis(&row.committer_name, GRAPH_COMMITTER_WIDTH);
    WidgetCell::from(Line::from(Span::styled(format!("{:<width$}", truncated, width = GRAPH_COMMITTER_WIDTH), Style::default().fg(text_color(row, selected, theme)))))
}

#[cfg(test)]
#[path = "../../tests/app/draw/graph.rs"]
mod tests;
