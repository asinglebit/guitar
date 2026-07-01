use crate::{
    app::{
        app::{App, Focus},
        draw::pane_window::zebra_list_items,
    },
    git::queries::helpers::FileStatus,
    helpers::{
        layout::scrollbar_content_length,
        localisation::{common, empty},
        symbols::SymbolTheme,
        text::{center_line, empty_state_top_padding, truncate_with_ellipsis},
    },
};
use ratatui::Frame;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

fn status_marker(status: FileStatus, symbols: &SymbolTheme) -> &str {
    match status {
        FileStatus::Added => symbols.status.added.as_str(),
        FileStatus::Modified => symbols.status.modified.as_str(),
        FileStatus::Deleted => symbols.status.deleted.as_str(),
        FileStatus::Renamed => symbols.status.renamed.as_str(),
        FileStatus::Other => symbols.status.other.as_str(),
    }
}

impl App {
    pub fn draw_search(&mut self, frame: &mut Frame) {
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };
        let available_width = self.layout.search.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs || self.layout_config.is_worktrees;
        let visible_height =
            if self.layout_config.is_zen { self.layout.search.height.saturating_sub(2) as usize } else { self.layout.search.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize };

        let total_lines = self.search_rows.len();
        if total_lines == 0 {
            self.search_selected = 0;
        } else if self.search_selected >= total_lines {
            self.search_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.search_selected, &self.search_scroll, total_lines, visible_height);
        let start = self.search_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        let mut lines: Vec<Line<'_>> = Vec::new();
        let selection_enabled = !self.search_is_loading && self.search_error.is_none() && total_lines > 0;

        if self.search_is_loading {
            let message = self
                .search_path
                .as_ref()
                .map(|path| format!("{} {}", common::LOADING(), truncate_with_ellipsis(path, max_text_width.saturating_sub(8))))
                .unwrap_or_else(|| common::LOADING().to_string());
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(&message, max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        } else if let Some(error) = &self.search_error {
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(error, max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_ORANGE))));
        } else if total_lines == 0 {
            let message = if self.search_path.is_some() { format!("{} {}", self.symbols.empty_state.mark, empty::NO_COMMITS()) } else { empty::SEARCH().to_string() };
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(&message, max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        } else {
            let summary_width = max_text_width.saturating_sub(12);
            for row in &self.search_rows[start..end] {
                let marker_color = match row.status {
                    FileStatus::Added => self.theme.COLOR_GRASS,
                    FileStatus::Modified => self.theme.COLOR_BLUE,
                    FileStatus::Deleted => self.theme.COLOR_RED,
                    FileStatus::Renamed => self.theme.COLOR_BLUE,
                    FileStatus::Other => self.theme.COLOR_TEXT,
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", status_marker(row.status, &self.symbols)), Style::default().fg(marker_color)),
                    Span::styled(format!("{:.8} ", row.oid), Style::default().fg(self.theme.COLOR_GREY_600)),
                    Span::styled(truncate_with_ellipsis(&row.summary, summary_width), Style::default().fg(self.theme.COLOR_TEXT)),
                ]));
            }
        }

        let display_start = if selection_enabled { start } else { 0 };
        let list_items = zebra_list_items(&lines, visible_height, display_start, self.search_selected, self.focus == Focus::Search, selection_enabled, &self.theme);

        if self.layout_config.is_zen {
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_set(self.symbols.border.block_set()));
            frame.render_widget(list, self.layout.search);

            let scroll_range = scrollbar_content_length(total_lines, visible_height);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.search_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
                .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
                .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Search { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.search_scrollbar, &mut scrollbar_state);
            return;
        }

        if has_previous {
            let top_border = Paragraph::new(self.symbols.border.horizontal.repeat(self.layout.search.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.search.x + 1, y: self.layout.search.y.saturating_sub(1), width: self.layout.search.width, height: 1 });
        }

        let list = List::new(list_items).block(Block::default().padding(padding));
        frame.render_widget(list, self.layout.search);

        let scroll_range = scrollbar_content_length(total_lines, visible_height);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.search_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if has_previous { self.symbols.border.vertical.as_str() } else { self.symbols.border.horizontal.as_str() }))
            .end_symbol(Some(self.symbols.border.horizontal.as_str()))
            .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
            .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Search { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.search_scrollbar, &mut scrollbar_state);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/search.rs"]
mod tests;
