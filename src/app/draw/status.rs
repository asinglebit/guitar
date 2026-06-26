use crate::{
    app::app::{App, Focus},
    git::queries::helpers::{FileChanges, FileStatus},
    helpers::{
        layout::scrollbar_content_length,
        localisation::{common, empty},
        palette::Theme,
        symbols::SymbolTheme,
        text::*,
    },
};
use ratatui::Frame;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::cell::Cell;

#[derive(Clone)]
struct StatusRow<'a> {
    line: Line<'a>,
    path: Option<&'a str>,
}

#[derive(Clone, Copy)]
enum StatusPaneKind {
    Top,
    Bottom,
}

struct BuiltStatusRows<'a> {
    rows: Vec<StatusRow<'a>>,
    has_selectable_changes: bool,
}

impl<'a> StatusRow<'a> {
    fn plain(line: Line<'a>) -> Self {
        Self { line, path: None }
    }

    fn file(path: &'a str, symbol: &str, symbol_style: Style, text_style: Style, max_width: usize) -> Self {
        Self { line: Line::from(vec![Span::styled(symbol.to_string(), symbol_style), Span::styled(truncate_with_ellipsis(path, max_width), text_style)]), path: Some(path) }
    }
}

impl App {
    fn status_empty_rows(&self, visible_height: usize, max_width: usize, message: &str) -> BuiltStatusRows<'static> {
        let mut rows = Vec::new();
        self.push_empty_status_row(&mut rows, visible_height, max_width, message);
        BuiltStatusRows { rows, has_selectable_changes: false }
    }

    fn status_loading_rows(&self, visible_height: usize, max_width: usize) -> BuiltStatusRows<'static> {
        BuiltStatusRows { rows: centered_loading_lines(visible_height, max_width + 3, Style::default().fg(self.theme.COLOR_GREY_800)), has_selectable_changes: false }
    }

    fn push_conflict_status_rows<'a>(&self, rows: &mut Vec<StatusRow<'a>>, files: &'a [String], max_width: usize) {
        rows.extend(
            files.iter().map(|file| StatusRow::file(file, &self.symbols.status.conflict_spaced, Style::default().fg(self.theme.COLOR_ORANGE), Style::default().fg(self.theme.COLOR_ORANGE), max_width)),
        );
    }

    fn push_file_status_rows<'a>(&self, rows: &mut Vec<StatusRow<'a>>, files: &'a [String], status: FileStatus, max_width: usize) {
        let (symbol, color) = match status {
            FileStatus::Added => (self.symbols.status.added_spaced.as_str(), self.theme.COLOR_GREEN),
            FileStatus::Modified => (self.symbols.status.modified_spaced.as_str(), self.theme.COLOR_BLUE),
            FileStatus::Deleted => (self.symbols.status.deleted_spaced.as_str(), self.theme.COLOR_RED),
            FileStatus::Renamed => (self.symbols.status.renamed_arrow_spaced.as_str(), self.theme.COLOR_YELLOW),
            FileStatus::Other => (self.symbols.status.other_spaced.as_str(), self.theme.COLOR_TEXT),
        };

        rows.extend(files.iter().map(|file| StatusRow::file(file, symbol, Style::default().fg(color), Style::default().fg(self.theme.COLOR_TEXT), max_width)));
    }

    fn push_file_change_rows<'a>(&self, rows: &mut Vec<StatusRow<'a>>, files: &'a FileChanges, max_width: usize) {
        self.push_file_status_rows(rows, &files.modified, FileStatus::Modified, max_width);
        self.push_file_status_rows(rows, &files.added, FileStatus::Added, max_width);
        self.push_file_status_rows(rows, &files.deleted, FileStatus::Deleted, max_width);
    }

    fn uncommitted_status_rows<'a>(&self, files: &'a FileChanges, conflicts: &'a [String], visible_height: usize, max_width: usize, empty_message: &str) -> BuiltStatusRows<'a> {
        let mut rows = Vec::new();
        self.push_conflict_status_rows(&mut rows, conflicts, max_width);
        self.push_file_change_rows(&mut rows, files, max_width);

        if rows.is_empty() { self.status_empty_rows(visible_height, max_width, empty_message) } else { BuiltStatusRows { rows, has_selectable_changes: true } }
    }

    fn commit_status_rows(&self, visible_height: usize, max_width: usize) -> BuiltStatusRows<'_> {
        let mut rows = Vec::with_capacity(self.current_diff.len());
        self.current_diff.iter().for_each(|file_change| {
            self.push_file_status_rows(&mut rows, std::slice::from_ref(&file_change.filename), file_change.status, max_width);
        });

        if rows.is_empty() { self.status_empty_rows(visible_height, max_width, empty::NO_STAGED_CHANGES()) } else { BuiltStatusRows { rows, has_selectable_changes: true } }
    }

    fn push_empty_status_row(&self, rows: &mut Vec<StatusRow<'_>>, visible_height: usize, max_width: usize, message: &str) {
        rows.extend((0..empty_state_top_padding(visible_height)).map(|_| StatusRow::plain(Line::from(""))));
        rows.push(StatusRow::plain(Line::from(Span::styled(
            center_line(&truncate_with_ellipsis(&format!("{} {message}", self.symbols.empty_state.mark), max_width), max_width + 3),
            Style::default().fg(self.theme.COLOR_GREY_800),
        ))));
    }

    pub fn draw_status(&mut self, frame: &mut Frame) {
        // Status panes keep icons close to the border and filenames flush after them.
        let padding = ratatui::widgets::Padding { left: 1, right: 0, top: 0, bottom: 0 };

        let is_showing_uncommitted = self.graph_selected == 0;

        // Width leaves room for the change symbol and a little border padding.
        let max_status_top_width = self.layout.status_top.width.saturating_sub(5) as usize;
        let max_status_bottom_width = self.layout.status_bottom.width.saturating_sub(5) as usize;
        let visible_height_status_top = self.layout.status_top.height.saturating_sub(2) as usize;
        let visible_height_status_bottom = self.layout.status_bottom.height.saturating_sub(2) as usize;
        let is_uncommitted_loading = is_showing_uncommitted && !self.is_uncommitted_loaded;
        let is_uncommitted_detail_loading = is_showing_uncommitted && self.is_uncommitted_loaded && self.is_uncommitted_detail_loading;
        let is_commit_diff_loading = !is_showing_uncommitted && !self.selected_commit_diff_is_loaded();

        // The pseudo-row splits uncommitted files into staged and unstaged panes.
        let (status_top, status_bottom) = if is_uncommitted_loading {
            (self.status_loading_rows(visible_height_status_top, max_status_top_width), self.status_loading_rows(visible_height_status_bottom, max_status_bottom_width))
        } else if is_showing_uncommitted {
            let top = self.uncommitted_status_rows(&self.uncommitted.staged, &self.uncommitted.conflicts, visible_height_status_top, max_status_top_width, empty::NO_STAGED_CHANGES());
            let bottom = if is_uncommitted_detail_loading {
                self.status_loading_rows(visible_height_status_bottom, max_status_bottom_width)
            } else {
                self.uncommitted_status_rows(&self.uncommitted.unstaged, &self.uncommitted.conflicts, visible_height_status_bottom, max_status_bottom_width, empty::NO_UNSTAGED_CHANGES())
            };
            (top, bottom)
        } else if is_commit_diff_loading {
            (self.status_loading_rows(visible_height_status_top, max_status_top_width), BuiltStatusRows { rows: Vec::new(), has_selectable_changes: false })
        } else {
            // Commit rows use the selected commit's file diff in the top pane only.
            (self.commit_status_rows(visible_height_status_top, max_status_top_width), BuiltStatusRows { rows: Vec::new(), has_selectable_changes: false })
        };

        let search_highlight_path = if self.layout_config.is_search { self.search_path.clone() } else { None };
        let top_has_inspector_border = self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts);

        // Top status pane shows staged files on the pseudo-row or commit file changes otherwise.
        self.status_top_selected = render_status_pane(
            frame,
            StatusPaneConfig {
                kind: StatusPaneKind::Top,
                rows: &status_top.rows,
                visible_height: visible_height_status_top,
                selected: self.status_top_selected,
                scroll: &self.status_top_scroll,
                is_focused: self.focus == Focus::StatusTop,
                selection_enabled: status_top.has_selectable_changes,
                search_highlight_path: search_highlight_path.as_deref(),
                area: self.layout.status_top,
                scrollbar_area: self.layout.status_top_scrollbar,
                is_zen: self.layout_config.is_zen,
                top_has_inspector_border,
                is_uncommitted_row: self.graph_selected == 0,
                padding,
                symbols: &self.symbols,
                theme: &self.theme,
            },
        );

        // Bottom status pane is reserved for unstaged files on the pseudo-row.
        is_showing_uncommitted.then(|| {
            self.status_bottom_selected = render_status_pane(
                frame,
                StatusPaneConfig {
                    kind: StatusPaneKind::Bottom,
                    rows: &status_bottom.rows,
                    visible_height: visible_height_status_bottom,
                    selected: self.status_bottom_selected,
                    scroll: &self.status_bottom_scroll,
                    is_focused: self.focus == Focus::StatusBottom,
                    selection_enabled: status_bottom.has_selectable_changes,
                    search_highlight_path: search_highlight_path.as_deref(),
                    area: self.layout.status_bottom,
                    scrollbar_area: self.layout.status_bottom_scrollbar,
                    is_zen: self.layout_config.is_zen,
                    top_has_inspector_border,
                    is_uncommitted_row: self.graph_selected == 0,
                    padding,
                    symbols: &self.symbols,
                    theme: &self.theme,
                },
            );
        });
    }
}

struct StatusPaneConfig<'a, 'b> {
    kind: StatusPaneKind,
    rows: &'a [StatusRow<'a>],
    visible_height: usize,
    selected: usize,
    scroll: &'b Cell<usize>,
    is_focused: bool,
    selection_enabled: bool,
    search_highlight_path: Option<&'b str>,
    area: Rect,
    scrollbar_area: Rect,
    is_zen: bool,
    top_has_inspector_border: bool,
    is_uncommitted_row: bool,
    padding: ratatui::widgets::Padding,
    symbols: &'b SymbolTheme,
    theme: &'b Theme,
}

struct StatusListConfig<'a, 'b> {
    rows: &'a [StatusRow<'a>],
    visible_height: usize,
    start: usize,
    selected: usize,
    is_focused: bool,
    selection_enabled: bool,
    search_highlight_path: Option<&'b str>,
    theme: &'b Theme,
}

fn render_status_pane(frame: &mut Frame, config: StatusPaneConfig<'_, '_>) -> usize {
    let total_lines = config.rows.len();
    let selected = if total_lines == 0 { 0 } else { config.selected.min(total_lines.saturating_sub(1)) };
    trap_status_selection(selected, config.scroll, total_lines, config.visible_height);

    let start = config.scroll.get().min(total_lines.saturating_sub(config.visible_height));
    let end = (start + config.visible_height).min(total_lines);
    let list_items = status_list_items(StatusListConfig {
        rows: &config.rows[start..end],
        visible_height: config.visible_height,
        start,
        selected,
        is_focused: config.is_focused,
        selection_enabled: config.selection_enabled,
        search_highlight_path: config.search_highlight_path,
        theme: config.theme,
    });
    let (borders, use_border_set) = status_pane_borders(config.kind, config.is_zen, config.top_has_inspector_border);
    let mut block = Block::default().padding(config.padding).borders(borders).border_style(Style::default().fg(config.theme.COLOR_BORDER));
    if use_border_set {
        block = block.border_set(config.symbols.border.block_set());
    }

    frame.render_widget(List::new(list_items).block(block), config.area);
    render_status_scrollbar(frame, &config, total_lines);
    selected
}

fn trap_status_selection(selected: usize, scroll: &Cell<usize>, total_lines: usize, visible_height: usize) {
    if visible_height == 0 || total_lines == 0 {
        scroll.set(0);
        return;
    }

    let max_scroll = total_lines.saturating_sub(visible_height);
    let mut scroll_val = scroll.get().min(max_scroll);
    let selected = selected.min(total_lines.saturating_sub(1));

    if selected < scroll_val {
        scroll.set(selected);
    } else if selected >= scroll_val + visible_height {
        scroll_val = selected.saturating_sub(visible_height).saturating_add(1).min(max_scroll);
        scroll.set(scroll_val);
    } else {
        scroll.set(scroll_val);
    }
}

fn status_pane_borders(kind: StatusPaneKind, is_zen: bool, top_has_inspector_border: bool) -> (Borders, bool) {
    match (is_zen, kind) {
        (true, _) => (Borders::ALL, true),
        (false, StatusPaneKind::Top) if top_has_inspector_border => (Borders::TOP, false),
        (false, StatusPaneKind::Bottom) => (Borders::TOP, false),
        (false, StatusPaneKind::Top) => (Borders::NONE, false),
    }
}

fn render_status_scrollbar(frame: &mut Frame, config: &StatusPaneConfig<'_, '_>, total_lines: usize) {
    let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, config.visible_height)).position(config.scroll.get());
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(status_scrollbar_begin(config))
        .end_symbol(status_scrollbar_end(config))
        .track_symbol(Some(config.symbols.scrollbar.track.as_str()))
        .thumb_symbol(if total_lines > config.visible_height { config.symbols.scrollbar.thumb.as_str() } else { config.symbols.scrollbar.inactive_thumb.as_str() })
        .thumb_style(Style::default().fg(if total_lines > config.visible_height && config.is_focused { config.theme.COLOR_GREY_600 } else { config.theme.COLOR_BORDER }));

    frame.render_stateful_widget(scrollbar, config.scrollbar_area, &mut scrollbar_state);
}

fn status_scrollbar_begin<'a>(config: &'a StatusPaneConfig<'_, '_>) -> Option<&'a str> {
    match (config.is_zen, config.kind) {
        (true, _) => Some(config.symbols.scrollbar.begin.as_str()),
        (false, StatusPaneKind::Top) if config.top_has_inspector_border => Some(config.symbols.border.vertical.as_str()),
        (false, StatusPaneKind::Top) => Some(config.symbols.scrollbar.begin.as_str()),
        (false, StatusPaneKind::Bottom) => Some(config.symbols.border.vertical.as_str()),
    }
}

fn status_scrollbar_end<'a>(config: &'a StatusPaneConfig<'_, '_>) -> Option<&'a str> {
    match (config.is_zen, config.kind) {
        (true, _) => Some(config.symbols.scrollbar.end.as_str()),
        (false, StatusPaneKind::Top) if config.is_uncommitted_row => Some(config.symbols.border.t_right.as_str()),
        (false, StatusPaneKind::Top | StatusPaneKind::Bottom) => Some(config.symbols.scrollbar.end.as_str()),
    }
}

fn centered_loading_lines(visible_height: usize, width: usize, style: Style) -> Vec<StatusRow<'static>> {
    (0..empty_state_top_padding(visible_height))
        .map(|_| StatusRow::plain(Line::from("")))
        .chain(std::iter::once(StatusRow::plain(Line::from(Span::styled(center_line(&truncate_with_ellipsis(common::LOADING(), width), width), style)))))
        .collect()
}

fn status_list_items<'a>(config: StatusListConfig<'a, '_>) -> Vec<ListItem<'a>> {
    (0..config.visible_height)
        .map(|idx| {
            let row = config.rows.get(idx).cloned().unwrap_or_else(|| StatusRow::plain(Line::default()));
            let global_idx = config.start + idx;
            let is_selected = config.selection_enabled && config.is_focused && global_idx == config.selected;
            let is_search_highlighted = row.path.zip(config.search_highlight_path).is_some_and(|(path, searched)| path == searched);
            let is_highlighted = is_selected || is_search_highlighted;

            let mut item = if is_highlighted {
                let spans: Vec<Span> = row.line.iter().map(|span| Span::styled(span.content.clone(), span.style.fg(config.theme.COLOR_HIGHLIGHTED))).collect();
                ListItem::new(Line::from(spans)).style(Style::default().bg(config.theme.background_or_default(config.theme.COLOR_GREY_800)).fg(config.theme.COLOR_HIGHLIGHTED))
            } else {
                ListItem::new(row.line)
            };

            if !is_highlighted && global_idx.is_multiple_of(2) {
                item = item.style(Style::default().bg(config.theme.background_or_default(config.theme.COLOR_GREY_900)));
            }

            item
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/app/draw/status.rs"]
mod tests;
