use crate::app::app::{App, Focus, GraphProjectionCache, GraphProjectionKey};
use crate::core::{
    chunk::NONE,
    graph_service::GraphRow,
    renderers::{GRAPH_COMMITTER_WIDTH, render_graph_projection, render_message_projection},
};
use crate::helpers::text::truncate_with_ellipsis;
use crate::helpers::{layout::scrollbar_content_length, localisation::empty};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{
    style::Style,
    widgets::{Block, Borders, Cell as WidgetCell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};
use std::collections::HashSet;

impl App {
    pub fn draw_graph(&mut self, frame: &mut Frame, repo: Option<&git2::Repository>) {
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
                && let Some(repo) = repo
            {
                self.refresh_current_diff_for_identity(repo, identity);
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
        let projection_key = self.graph.graph_window.as_ref().filter(|window| window.start < end && start < window.end).map(|window| {
            let render_uncommitted_row = graph_window_has_stable_visible_page(window, start, end);
            GraphProjectionKey {
                version: window.version,
                start: window.start,
                end: window.end,
                head_alias: window.head_alias,
                selected: self.graph_selected,
                show_reflog_labels: self.layout_config.is_graph_reflogs,
                show_ref_labels: self.layout_config.is_graph_refs,
                render_uncommitted_row,
                conflict_count: self.uncommitted.conflict_count,
                modified_count: self.uncommitted.modified_count,
                added_count: self.uncommitted.added_count,
                deleted_count: self.uncommitted.deleted_count,
            }
        });

        if let Some(key) = projection_key
            && self.graph.graph_projection.key != Some(key)
        {
            let window = self.graph.graph_window.as_ref().expect("projection key requires graph window");
            let message_lines =
                render_message_projection(&self.theme, &self.symbols, &window.rows, key.show_reflog_labels, key.show_ref_labels, key.selected, &self.uncommitted, key.render_uncommitted_row);
            self.graph.graph_projection = GraphProjectionCache { key: Some(key), message_lines };
        }

        // Render from the cached window, then reindex into the requested viewport so scrolling
        // still looks like movement while loading.
        let (cached_start, cached_rows, graph_lines, message_lines) =
            if let Some(window) = self.graph.graph_window.as_ref().filter(|window| window.start < end && start < window.end && self.graph.graph_projection.key.is_some()) {
                let graph_lines = render_graph_projection(
                    &self.theme,
                    &self.symbols,
                    &window.rows,
                    &window.history,
                    window.head_alias,
                    window.start,
                    window.end,
                    projection_key.is_some_and(|key| key.render_uncommitted_row),
                );
                (window.start, window.rows.as_slice(), graph_lines, self.graph.graph_projection.message_lines.as_slice())
            } else {
                (start, &[][..], Vec::new(), &[][..])
            };

        // Measure the graph column from visible rendered span widths only.
        let width = (start..end).filter_map(|index| projected_line(&graph_lines, cached_start, index)).map(|line| line.width()).max().unwrap_or(0) as u16;
        let search_highlight_indices: HashSet<usize> =
            if self.layout_config.is_search && self.search_path.is_some() { self.search_rows.iter().map(|row| row.graph_index).filter(|&index| index != 0).collect() } else { HashSet::new() };

        let graph_rows = GraphRowsWidget {
            start,
            visible_height,
            visible_len,
            cached_start,
            cached_rows,
            graph_lines,
            message_lines,
            graph_width: width.saturating_add(5),
            selected: self.graph_selected,
            is_focused: self.focus == Focus::Viewport,
            search_highlight_indices: &search_highlight_indices,
            is_shas: self.layout_config.is_shas,
            is_dates: self.layout_config.is_graph_dates,
            is_committers: self.layout_config.is_graph_committers,
            theme: &self.theme,
        };

        if self.layout_config.is_zen {
            // Zen mode owns the full rounded graph frame.
            let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_set(self.symbols.border.block_set());
            let inner = block.inner(self.layout.graph);
            frame.render_widget(block, self.layout.graph);
            frame.render_widget(graph_rows, inner);

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
        let block = Block::default().borders(Borders::RIGHT | Borders::LEFT).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_set(self.symbols.border.block_set());
        let inner = block.inner(self.layout.graph);
        frame.render_widget(block, self.layout.graph);
        frame.render_widget(graph_rows, inner);

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

fn graph_backdrop_rows<'a>(visible_height: usize, start: usize, selected: Option<usize>, theme: &crate::helpers::palette::Theme) -> Vec<Row<'a>> {
    (0..visible_height)
        .map(|idx| {
            let global_idx = start + idx;
            let row = Row::new([WidgetCell::from(Line::default())]);
            match (selected == Some(global_idx), global_idx.is_multiple_of(2)) {
                (true, _) => row.style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_800))),
                (false, true) => row.style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_900))),
                (false, false) => row,
            }
        })
        .collect()
}

fn graph_window_has_stable_visible_page(window: &crate::app::app::GraphWindowCache, target_start: usize, target_end: usize) -> bool {
    let cached_len = window.end.saturating_sub(window.start);
    window.start <= target_start && target_end <= window.end && window.rows.len() >= cached_len && window.history.len() >= cached_len
}

fn graph_preload_window(start: usize, end: usize, total_lines: usize, visible_height: usize) -> (usize, usize) {
    match visible_height {
        0 => (start, end),
        height => (start.saturating_sub(height), end.saturating_add(height).min(total_lines)),
    }
}

fn projected_line<'a, 'line>(lines: &'a [Line<'line>], cached_start: usize, target_index: usize) -> Option<&'a Line<'line>> {
    target_index.checked_sub(cached_start).and_then(|source_index| lines.get(source_index))
}

struct GraphRowsWidget<'a, 'line> {
    start: usize,
    visible_height: usize,
    visible_len: usize,
    cached_start: usize,
    cached_rows: &'a [GraphRow],
    graph_lines: Vec<Line<'line>>,
    message_lines: &'line [Line<'static>],
    graph_width: u16,
    selected: usize,
    is_focused: bool,
    search_highlight_indices: &'a HashSet<usize>,
    is_shas: bool,
    is_dates: bool,
    is_committers: bool,
    theme: &'a crate::helpers::palette::Theme,
}

impl Widget for GraphRowsWidget<'_, '_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !area.is_empty() && self.visible_height != 0 {
            let columns = graph_columns(area, self.is_shas, self.is_dates, self.is_committers, self.graph_width);
            (0..self.visible_height.min(area.height as usize)).for_each(|idx| {
                let global_idx = self.start + idx;
                let y = area.y.saturating_add(idx as u16);
                let row_area = Rect::new(area.x, y, area.width, 1);
                buf.set_style(row_area, row_style(global_idx, idx < self.visible_len, self.selected, self.is_focused, self.search_highlight_indices, self.theme));

                let row = global_idx.checked_sub(self.cached_start).and_then(|source_index| self.cached_rows.get(source_index));
                columns.sha.into_iter().for_each(|sha| render_sha(buf, sha.with_y(y), row, self.selected, self.theme));
                render_projected_line(buf, columns.graph.with_y(y), projected_line(&self.graph_lines, self.cached_start, global_idx));
                columns.date.into_iter().for_each(|date| render_date(buf, date.with_y(y), row, self.selected, self.theme));
                columns.committer.into_iter().for_each(|committer| render_committer(buf, committer.with_y(y), row, self.selected, self.theme));
                render_projected_line(buf, columns.message.with_y(y), projected_line(self.message_lines, self.cached_start, global_idx));
            });
        }
    }
}

struct GraphColumns {
    sha: Option<Rect>,
    graph: Rect,
    date: Option<Rect>,
    committer: Option<Rect>,
    message: Rect,
}

struct GraphColumnCursor {
    cursor: u16,
    right: u16,
    y: u16,
}

impl GraphColumnCursor {
    fn new(area: Rect) -> Self {
        Self { cursor: area.x, right: area.x.saturating_add(area.width), y: area.y }
    }

    fn take(&mut self, width: u16) -> Rect {
        let actual = width.min(self.right.saturating_sub(self.cursor));
        let rect = Rect::new(self.cursor, self.y, actual, 1);
        self.cursor = self.cursor.saturating_add(actual).saturating_add(u16::from(self.cursor.saturating_add(actual) < self.right));
        rect
    }

    fn rest(self) -> Rect {
        Rect::new(self.cursor, self.y, self.right.saturating_sub(self.cursor), 1)
    }
}

fn graph_columns(area: Rect, is_shas: bool, is_dates: bool, is_committers: bool, graph_width: u16) -> GraphColumns {
    let mut cursor = GraphColumnCursor::new(area);
    let sha = is_shas.then(|| cursor.take(9));
    let graph = cursor.take(graph_width);
    let date = is_dates.then(|| cursor.take(16));
    let committer = is_committers.then(|| cursor.take(GRAPH_COMMITTER_WIDTH as u16));
    let message = cursor.rest();

    GraphColumns { sha, graph, date, committer, message }
}

trait RectRowExt {
    fn with_y(self, y: u16) -> Self;
}

impl RectRowExt for Rect {
    fn with_y(mut self, y: u16) -> Self {
        self.y = y;
        self
    }
}

fn row_style(global_idx: usize, is_visible: bool, selected: usize, is_focused: bool, search_highlight_indices: &HashSet<usize>, theme: &crate::helpers::palette::Theme) -> Style {
    let is_selected = is_visible && global_idx == selected && is_focused;
    let is_search_highlighted = is_visible && search_highlight_indices.contains(&global_idx);
    if is_selected || is_search_highlighted {
        Style::default().bg(theme.background_or_default(theme.COLOR_GREY_800))
    } else if global_idx.is_multiple_of(2) {
        Style::default().bg(theme.background_or_default(theme.COLOR_GREY_900))
    } else {
        Style::default()
    }
}

fn render_projected_line(buf: &mut Buffer, area: Rect, line: Option<&Line<'_>>) {
    if area.width != 0 {
        line.into_iter().for_each(|line| {
            buf.set_line(area.x, area.y, line, area.width);
        });
    }
}

fn render_sha(buf: &mut Buffer, area: Rect, row: Option<&GraphRow>, selected: usize, theme: &crate::helpers::palette::Theme) {
    row.filter(|row| row.alias != NONE).into_iter().for_each(|row| {
        let short_oid = row.oid.to_string();
        buf.set_stringn(area.x, area.y, &short_oid[..9.min(short_oid.len())], area.width as usize, Style::default().fg(text_color(row, selected, theme)));
    });
}

fn render_date(buf: &mut Buffer, area: Rect, row: Option<&GraphRow>, selected: usize, theme: &crate::helpers::palette::Theme) {
    row.filter(|row| row.alias != NONE).into_iter().for_each(|row| {
        buf.set_stringn(area.x, area.y, row.committer_date.as_str(), area.width as usize, Style::default().fg(text_color(row, selected, theme)));
    });
}

fn render_committer(buf: &mut Buffer, area: Rect, row: Option<&GraphRow>, selected: usize, theme: &crate::helpers::palette::Theme) {
    row.filter(|row| row.alias != NONE).into_iter().for_each(|row| {
        let max_width = GRAPH_COMMITTER_WIDTH.min(area.width as usize);
        let style = Style::default().fg(text_color(row, selected, theme));
        if row.committer_name.chars().count() <= max_width {
            buf.set_stringn(area.x, area.y, row.committer_name.as_str(), max_width, style);
        } else {
            let truncated = truncate_with_ellipsis(&row.committer_name, max_width);
            buf.set_stringn(area.x, area.y, truncated, max_width, style);
        }
    });
}

fn text_color(row: &GraphRow, selected: usize, theme: &crate::helpers::palette::Theme) -> ratatui::style::Color {
    if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT }
}

#[cfg(test)]
#[path = "../../tests/app/draw/graph.rs"]
mod tests;
