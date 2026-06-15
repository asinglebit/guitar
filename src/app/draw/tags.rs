use crate::helpers::symbols::SYM_TAG;
use crate::helpers::text::{center_line, empty_state_top_padding};
use crate::{
    app::{
        app::{App, Focus},
        draw::{
            buffered::{DrawTarget, SurfaceRender},
            pane_window::{aligned_pane_rows, blank_lines, preloaded_pane_window, zebra_list_items},
        },
    },
    core::graph_service::{GraphPane, GraphPaneRow},
    helpers::colors::ColorPicker,
    helpers::text::truncate_with_ellipsis,
};
use ratatui::widgets::Borders;
use ratatui::{layout::Rect, widgets::Paragraph};
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_tags(&mut self, frame: &mut impl DrawTarget) -> SurfaceRender {
        // Left pane padding changes in zen mode because the pane has its own border.
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };

        // Reserve space for the tag icon and a separating space.
        let available_width = self.layout.tags.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let visible_height = if self.layout_config.is_zen {
            self.layout.tags.height.saturating_sub(2) as usize
        } else {
            self.layout.tags.height.saturating_sub(if self.layout_config.is_branches { 1 } else { 2 }) as usize
        };

        // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
        let total_lines = self.graph.tags_window.as_ref().map(|window| window.total).unwrap_or_else(|| self.tags.get_sorted_aliases().len());

        if total_lines == 0 {
            self.tags_selected = 0;
        } else if self.tags_selected >= total_lines {
            self.tags_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.tags_selected, &self.tags_scroll, total_lines, visible_height);

        let start = self.tags_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);
        let (preload_start, preload_end) = preloaded_pane_window(start, end, total_lines, visible_height);
        self.request_pane_window(GraphPane::Tags, preload_start, preload_end);

        // Tag rows are already sorted by name in the worker projection.
        let mut lines: Vec<Line<'_>> = Vec::new();
        let lines_are_windowed = self.graph_tx.is_some();
        let known_empty = self.graph.tags_window.as_ref().is_some_and(|window| window.total == 0);
        if let Some(rows) = self.graph.tags_window.as_ref().and_then(|window| aligned_pane_rows(window, start, end)) {
            let color_picker = ColorPicker::from_theme(&self.theme);
            for row in rows {
                if let Some(GraphPaneRow::Tag { name, lane, .. }) = row {
                    let truncated = truncate_with_ellipsis(name, max_text_width.saturating_sub(1));
                    let color = lane.map(|lane| color_picker.get_lane(lane)).unwrap_or(self.theme.COLOR_TEXT);
                    lines.push(Line::from(Span::styled(format!("{SYM_TAG} {truncated}"), Style::default().fg(color))));
                } else {
                    lines.push(Line::default());
                }
            }
        } else if self.graph_tx.is_none() {
            for (tag_alias, tag_name) in self.tags.get_sorted_aliases() {
                let truncated = truncate_with_ellipsis(tag_name, max_text_width.saturating_sub(1));
                let color = self.tags.get_color(&self.theme, tag_alias);

                lines.push(Line::from(Span::styled(format!("{SYM_TAG} {truncated}"), Style::default().fg(color))));
            }
        } else if !known_empty {
            lines = blank_lines(if total_lines == 0 { visible_height } else { end.saturating_sub(start) });
        }

        // Empty state is part of the list so scrolling and borders still behave normally.
        let mut tags_empty = false;
        if lines.is_empty() && (!lines_are_windowed || known_empty) {
            tags_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis("⊘ no tags", max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        // Selection is skipped for the synthetic empty row; striping still fills the pane.
        let display_start = if tags_empty || lines_are_windowed { 0 } else { start };
        let display_end = if tags_empty || lines_are_windowed { lines.len() } else { end };
        let list_items = zebra_list_items(&lines[display_start..display_end], visible_height, start, self.tags_selected, self.focus == Focus::Tags, !tags_empty, &self.theme);

        if self.layout_config.is_zen {
            // Zen mode frames the pane as a full standalone list.
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_type(ratatui::widgets::BorderType::Rounded));

            frame.render_widget(list, self.layout.tags);

            let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.tags_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("╮"))
                .end_symbol(Some("╯"))
                .track_symbol(Some("│"))
                .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Tags { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.tags_scrollbar, &mut scrollbar_state);

            return SurfaceRender::Ready;
        }

        // Normal mode draws a top separator when this pane is stacked under another pane.
        if self.layout_config.is_branches || self.layout_config.is_tags {
            let top_border = Paragraph::new("─".repeat(self.layout.tags.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.tags.x + 1, y: self.layout.tags.y.saturating_sub(1), width: self.layout.tags.width, height: 1 });
        }
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.tags);

        let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.tags_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if self.layout_config.is_branches { "│" } else { "─" }))
            .end_symbol(Some(if self.layout_config.is_stashes || self.layout_config.is_reflogs || self.layout_config.is_worktrees { "│" } else { "─" }))
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Tags { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.tags_scrollbar, &mut scrollbar_state);
        SurfaceRender::Ready
    }
}
