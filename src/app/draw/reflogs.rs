use crate::{
    app::{
        app::{App, Focus},
        draw::pane_window::{aligned_pane_rows, blank_lines, preloaded_pane_window, zebra_list_items},
    },
    core::graph_service::{GraphPane, GraphPaneRow},
    helpers::{
        colors::ColorPicker,
        layout::scrollbar_content_length,
        localisation::empty,
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

impl App {
    pub fn draw_reflogs(&mut self, frame: &mut Frame) {
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };
        let available_width = self.layout.reflogs.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes;
        let has_next = self.layout_config.is_worktrees || self.layout_config.is_search;
        let visible_height =
            if self.layout_config.is_zen { self.layout.reflogs.height.saturating_sub(2) as usize } else { self.layout.reflogs.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize };
        let total_lines = self.graph.reflogs_window.as_ref().map(|window| window.total).unwrap_or(self.reflogs.entries.len());

        if total_lines == 0 {
            self.reflogs_selected = 0;
        } else if self.reflogs_selected >= total_lines {
            self.reflogs_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.reflogs_selected, &self.reflogs_scroll, total_lines, visible_height);

        let start = self.reflogs_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);
        let (preload_start, preload_end) = preloaded_pane_window(start, end, total_lines, visible_height);
        self.request_pane_window(GraphPane::Reflogs, preload_start, preload_end);

        let mut lines: Vec<Line<'_>> = Vec::new();
        let lines_are_windowed = self.graph_tx.is_some();
        let known_empty = self.graph.reflogs_window.as_ref().is_some_and(|window| window.total == 0);
        if let Some(rows) = self.graph.reflogs_window.as_ref().and_then(|window| aligned_pane_rows(window, start, end)) {
            let color_picker = ColorPicker::from_theme(&self.theme);
            for row in rows {
                if let Some(GraphPaneRow::Reflog { selector, message, lane, .. }) = row {
                    let label = truncate_with_ellipsis(&format!("{selector} {message}"), max_text_width.saturating_sub(1));
                    let color = lane.map(|lane| color_picker.get_lane(lane)).unwrap_or(self.theme.COLOR_TEXT);
                    lines.push(Line::from(Span::styled(format!("{} {label}", self.symbols.entity.reflog), Style::default().fg(color))));
                } else {
                    lines.push(Line::default());
                }
            }
        } else if self.graph_tx.is_none() {
            for entry in &self.reflogs.entries {
                let label = truncate_with_ellipsis(&format!("{} {}", entry.selector, entry.message), max_text_width.saturating_sub(1));
                let color = self.reflogs.get_color(entry.new_alias).unwrap_or(self.theme.COLOR_TEXT);
                lines.push(Line::from(Span::styled(format!("{} {label}", self.symbols.entity.reflog), Style::default().fg(color))));
            }
        } else if !known_empty {
            lines = blank_lines(if total_lines == 0 { visible_height } else { end.saturating_sub(start) });
        }

        let mut reflogs_empty = false;
        if lines.is_empty() && (!lines_are_windowed || known_empty) {
            reflogs_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            let empty_text = format!("{} {}", self.symbols.entity.reflog, empty::NO_HEAD_REFLOG());
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(&empty_text, max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        let display_start = if reflogs_empty || lines_are_windowed { 0 } else { start };
        let display_end = if reflogs_empty || lines_are_windowed { lines.len() } else { end };
        let list_items = zebra_list_items(&lines[display_start..display_end], visible_height, start, self.reflogs_selected, self.focus == Focus::Reflogs, !reflogs_empty, &self.theme);

        if self.layout_config.is_zen {
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_set(self.symbols.border.block_set()));
            frame.render_widget(list, self.layout.reflogs);

            let scroll_range = scrollbar_content_length(total_lines, visible_height);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.reflogs_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
                .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
                .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Reflogs { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.reflogs_scrollbar, &mut scrollbar_state);
            return;
        }

        if has_previous {
            let top_border = Paragraph::new(self.symbols.border.horizontal.repeat(self.layout.reflogs.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.reflogs.x + 1, y: self.layout.reflogs.y.saturating_sub(1), width: self.layout.reflogs.width, height: 1 });
        }

        let list = List::new(list_items).block(Block::default().padding(padding));
        frame.render_widget(list, self.layout.reflogs);

        let scroll_range = scrollbar_content_length(total_lines, visible_height);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.reflogs_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if has_previous { self.symbols.border.vertical.as_str() } else { self.symbols.border.horizontal.as_str() }))
            .end_symbol(Some(if has_next { self.symbols.border.vertical.as_str() } else { self.symbols.border.horizontal.as_str() }))
            .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
            .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Reflogs { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.reflogs_scrollbar, &mut scrollbar_state);
    }
}
