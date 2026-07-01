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

#[derive(Clone, Copy)]
struct GraphVisibleWindow {
    start: usize,
    end: usize,
    visible_height: usize,
    visible_len: usize,
    total_lines: usize,
}

struct CachedGraphProjection<'a> {
    cached_start: usize,
    cached_rows: &'a [GraphRow],
    graph_lines: Vec<Line<'a>>,
    message_lines: &'a [Line<'static>],
}

impl App {
    pub fn draw_graph(&mut self, frame: &mut Frame, repo: Option<&git2::Repository>) {
        if let Some(window) = self.prepare_graph_window(repo) {
            if self.graph.total == 0 && self.graph.is_complete {
                self.render_unborn_graph(frame, window.visible_height);
            } else {
                self.render_graph_window(frame, window);
            }
        }
    }

    fn prepare_graph_window(&mut self, repo: Option<&git2::Repository>) -> Option<GraphVisibleWindow> {
        (!self.layout.graph.is_empty()).then(|| {
            let total_lines = self.graph_commit_count();
            let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };

            let previous_selected = self.graph_selected;
            self.graph_selected = match total_lines {
                0 => 0,
                total if self.graph_selected >= total => total.saturating_sub(1),
                _ => self.graph_selected,
            };

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
            let window = GraphVisibleWindow { start, end, visible_height, visible_len: end.saturating_sub(start), total_lines };
            let (preload_start, preload_end) = graph_preload_window(start, end, total_lines, visible_height);
            self.request_graph_window(preload_start, preload_end);
            window
        })
    }

    fn render_unborn_graph(&self, frame: &mut Frame, visible_height: usize) {
        let table = Table::new(graph_backdrop_rows(visible_height, 0, None, &self.theme), [ratatui::layout::Constraint::Min(0)])
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT).border_style(Style::default().fg(self.theme.COLOR_BORDER)))
            .column_spacing(0);

        frame.render_widget(table, self.layout.graph);

        let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Length(3), Constraint::Percentage(50)]).split(self.layout.graph);

        let message = Paragraph::new(format!("{} {}", self.symbols.empty_state.mark, empty::NO_COMMITS())).alignment(Alignment::Center).style(Style::default().fg(self.theme.COLOR_BORDER));

        frame.render_widget(message, chunks[1]);
    }

    fn render_graph_window(&mut self, frame: &mut Frame, window: GraphVisibleWindow) {
        let projection_key = self.refresh_graph_projection_cache(window);
        let cached = self.cached_graph_projection(window, projection_key);
        let search_highlight_indices = self.search_highlight_indices();
        let graph_rows = self.graph_rows_widget(window, cached, &search_highlight_indices);

        let borders = if self.layout_config.is_zen { Borders::ALL } else { Borders::RIGHT | Borders::LEFT };
        let block = Block::default().borders(borders).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_set(self.symbols.border.block_set());
        let inner = block.inner(self.layout.graph);
        frame.render_widget(block, self.layout.graph);
        frame.render_widget(graph_rows, inner);

        if window.total_lines > window.visible_height {
            let (begin, end) = self.graph_scrollbar_symbols();
            let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(window.total_lines, window.visible_height)).position(self.graph_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some(begin))
                .end_symbol(Some(end))
                .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                .thumb_symbol(self.symbols.scrollbar.thumb.as_str())
                .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);
        }
    }

    fn refresh_graph_projection_cache(&mut self, window: GraphVisibleWindow) -> Option<GraphProjectionKey> {
        let key = self.graph.graph_window.as_ref().filter(|cached| cached.start < window.end && window.start < cached.end).map(|cached| {
            let render_uncommitted_row = graph_window_has_stable_visible_page(cached, window.start, window.end);
            GraphProjectionKey {
                version: cached.version,
                start: cached.start,
                end: cached.end,
                head_alias: cached.head_alias,
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

        if let Some(key) = key
            && self.graph.graph_projection.key != Some(key)
            && let Some(cached) = self.graph.graph_window.as_ref().filter(|cached| cached.version == key.version && cached.start == key.start && cached.end == key.end)
        {
            let message_lines =
                render_message_projection(&self.theme, &self.symbols, &cached.rows, key.show_reflog_labels, key.show_ref_labels, key.selected, &self.uncommitted, key.render_uncommitted_row);
            self.graph.graph_projection = GraphProjectionCache { key: Some(key), message_lines };
        }

        key
    }

    fn cached_graph_projection<'a>(&'a self, window: GraphVisibleWindow, projection_key: Option<GraphProjectionKey>) -> CachedGraphProjection<'a> {
        self.graph
            .graph_window
            .as_ref()
            .filter(|cached| cached.start < window.end && window.start < cached.end && self.graph.graph_projection.key.is_some())
            .map(|cached| {
                let graph_lines = render_graph_projection(
                    &self.theme,
                    &self.symbols,
                    &cached.rows,
                    &cached.history,
                    cached.head_alias,
                    cached.start,
                    cached.end,
                    projection_key.is_some_and(|key| key.render_uncommitted_row),
                );
                CachedGraphProjection { cached_start: cached.start, cached_rows: cached.rows.as_slice(), graph_lines, message_lines: self.graph.graph_projection.message_lines.as_slice() }
            })
            .unwrap_or_else(|| CachedGraphProjection { cached_start: window.start, cached_rows: &[], graph_lines: Vec::new(), message_lines: &[] })
    }

    fn search_highlight_indices(&self) -> HashSet<usize> {
        if self.layout_config.is_search && self.search_path.is_some() { self.search_rows.iter().map(|row| row.graph_index).filter(|&index| index != 0).collect() } else { HashSet::new() }
    }

    fn graph_rows_widget<'a>(&'a self, window: GraphVisibleWindow, cached: CachedGraphProjection<'a>, search_highlight_indices: &'a HashSet<usize>) -> GraphRowsWidget<'a, 'a> {
        let graph_width = (window.start..window.end).filter_map(|index| projected_line(&cached.graph_lines, cached.cached_start, index)).map(|line| line.width()).max().unwrap_or(0) as u16;

        GraphRowsWidget {
            start: window.start,
            visible_height: window.visible_height,
            visible_len: window.visible_len,
            cached_start: cached.cached_start,
            cached_rows: cached.cached_rows,
            graph_lines: cached.graph_lines,
            message_lines: cached.message_lines,
            graph_width: graph_width.saturating_add(5),
            selected: self.graph_selected,
            is_focused: self.focus == Focus::Viewport,
            search_highlight_indices,
            is_shas: self.layout_config.is_shas,
            is_dates: self.layout_config.is_graph_dates,
            is_committers: self.layout_config.is_graph_committers,
            theme: &self.theme,
        }
    }

    fn graph_scrollbar_symbols(&self) -> (&str, &str) {
        match (self.layout_config.is_zen, (self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts)) || self.layout_config.is_status) {
            (true, _) => (self.symbols.scrollbar.begin.as_str(), self.symbols.scrollbar.end.as_str()),
            (false, true) => (self.symbols.border.horizontal.as_str(), self.symbols.border.horizontal.as_str()),
            (false, false) => (self.symbols.scrollbar.begin.as_str(), self.symbols.scrollbar.end.as_str()),
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

struct GraphRenderRow<'a> {
    index: usize,
    area: Rect,
    source: Option<&'a GraphRow>,
    is_visible: bool,
}

struct RenderedGraphRow<'a, 'line> {
    area: Rect,
    style: Style,
    sha_area: Option<Rect>,
    graph_area: Rect,
    graph_line: Option<&'a Line<'line>>,
    date_area: Option<Rect>,
    committer_area: Option<Rect>,
    message_area: Rect,
    message_line: Option<&'a Line<'static>>,
    source: Option<&'a GraphRow>,
    selected: usize,
    theme: &'a crate::helpers::palette::Theme,
}

impl<'a, 'line> GraphRowsWidget<'a, 'line> {
    fn rows(&'a self, area: Rect) -> impl Iterator<Item = GraphRenderRow<'a>> + 'a {
        (0..self.visible_height.min(area.height as usize)).map(move |offset| {
            let index = self.start + offset;
            let y = area.y.saturating_add(offset as u16);

            GraphRenderRow {
                index,
                area: Rect::new(area.x, y, area.width, 1),
                source: index.checked_sub(self.cached_start).and_then(|source_index| self.cached_rows.get(source_index)),
                is_visible: offset < self.visible_len,
            }
        })
    }

    fn rendered_rows(&'a self, area: Rect) -> impl Iterator<Item = RenderedGraphRow<'a, 'line>> + 'a {
        let columns = graph_columns(area, self.is_shas, self.is_dates, self.is_committers, self.graph_width);
        self.rows(area).map(move |row| row.rendered(columns, self))
    }
}

impl<'a> GraphRenderRow<'a> {
    fn rendered<'line>(self, columns: GraphColumns, rows: &'a GraphRowsWidget<'a, 'line>) -> RenderedGraphRow<'a, 'line> {
        let y = self.area.y;
        RenderedGraphRow {
            area: self.area,
            style: row_style(self.index, self.is_visible, rows.selected, rows.is_focused, rows.search_highlight_indices, rows.theme),
            sha_area: columns.sha.map(|area| area.with_y(y)),
            graph_area: columns.graph.with_y(y),
            graph_line: projected_line(&rows.graph_lines, rows.cached_start, self.index),
            date_area: columns.date.map(|area| area.with_y(y)),
            committer_area: columns.committer.map(|area| area.with_y(y)),
            message_area: columns.message.with_y(y),
            message_line: projected_line(rows.message_lines, rows.cached_start, self.index),
            source: self.source,
            selected: rows.selected,
            theme: rows.theme,
        }
    }
}

impl RenderedGraphRow<'_, '_> {
    fn draw(self, buf: &mut Buffer) {
        buf.set_style(self.area, self.style);
        if let Some(area) = self.sha_area {
            render_sha(buf, area, self.source, self.selected, self.theme);
        }
        render_projected_line(buf, self.graph_area, self.graph_line);
        if let Some(area) = self.date_area {
            render_date(buf, area, self.source, self.selected, self.theme);
        }
        if let Some(area) = self.committer_area {
            render_committer(buf, area, self.source, self.selected, self.theme);
        }
        render_projected_line(buf, self.message_area, self.message_line);
    }
}

impl Widget for GraphRowsWidget<'_, '_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for row in self.rendered_rows(area) {
            row.draw(buf);
        }
    }
}

#[derive(Clone, Copy)]
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
    if area.width != 0
        && let Some(line) = line
    {
        buf.set_line(area.x, area.y, line, area.width);
    }
}

fn render_sha(buf: &mut Buffer, area: Rect, row: Option<&GraphRow>, selected: usize, theme: &crate::helpers::palette::Theme) {
    if let Some(row) = row.filter(|row| row.alias != NONE) {
        let short_oid = row.oid.to_string();
        buf.set_stringn(area.x, area.y, &short_oid[..9.min(short_oid.len())], area.width as usize, Style::default().fg(text_color(row, selected, theme)));
    }
}

fn render_date(buf: &mut Buffer, area: Rect, row: Option<&GraphRow>, selected: usize, theme: &crate::helpers::palette::Theme) {
    if let Some(row) = row.filter(|row| row.alias != NONE) {
        buf.set_stringn(area.x, area.y, row.committer_date.as_str(), area.width as usize, Style::default().fg(text_color(row, selected, theme)));
    }
}

fn render_committer(buf: &mut Buffer, area: Rect, row: Option<&GraphRow>, selected: usize, theme: &crate::helpers::palette::Theme) {
    if let Some(row) = row.filter(|row| row.alias != NONE) {
        let max_width = GRAPH_COMMITTER_WIDTH.min(area.width as usize);
        let style = Style::default().fg(text_color(row, selected, theme));
        if row.committer_name.chars().count() <= max_width {
            buf.set_stringn(area.x, area.y, row.committer_name.as_str(), max_width, style);
        } else {
            let truncated = truncate_with_ellipsis(&row.committer_name, max_width);
            buf.set_stringn(area.x, area.y, truncated, max_width, style);
        }
    }
}

fn text_color(row: &GraphRow, selected: usize, theme: &crate::helpers::palette::Theme) -> ratatui::style::Color {
    if row.index == selected { theme.COLOR_HIGHLIGHTED } else { theme.COLOR_TEXT }
}

#[cfg(test)]
#[path = "../../tests/app/draw/graph.rs"]
mod tests;
