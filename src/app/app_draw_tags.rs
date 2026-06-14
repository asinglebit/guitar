use crate::helpers::symbols::SYM_TAG;
use crate::helpers::text::{center_line, empty_state_top_padding};
use crate::{
    app::app::{App, Focus},
    helpers::text::truncate_with_ellipsis,
};
use ratatui::widgets::Borders;
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use ratatui::{layout::Rect, widgets::Paragraph};

impl App {
    pub fn draw_tags(&mut self, frame: &mut Frame) {
        // Left pane padding changes in zen mode because the pane has its own border.
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };

        // Reserve space for the tag icon and a separating space.
        let available_width = self.layout.tags.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        // Tag rows are already sorted by name in Tags::feed.
        let mut lines: Vec<Line<'_>> = Vec::new();
        for (tag_alias, tag_name) in self.tags.get_sorted_aliases() {
            let truncated = truncate_with_ellipsis(tag_name, max_text_width.saturating_sub(1));
            let color = self.tags.get_color(&self.theme, tag_alias);

            lines.push(Line::from(Span::styled(format!("{SYM_TAG} {truncated}"), Style::default().fg(color))));
        }

        let visible_height = if self.layout_config.is_zen {
            self.layout.tags.height.saturating_sub(2) as usize
        } else {
            self.layout.tags.height.saturating_sub(if self.layout_config.is_branches { 1 } else { 2 }) as usize
        };

        // Empty state is part of the list so scrolling and borders still behave normally.
        let mut tags_empty = false;
        if lines.is_empty() {
            tags_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis("⊘ no tags", max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
        let total_lines = lines.len();

        if total_lines == 0 {
            self.tags_selected = 0;
        } else if self.tags_selected >= total_lines {
            self.tags_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.tags_selected, &self.tags_scroll, total_lines, visible_height);

        let start = self.tags_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Selection and zebra striping are skipped for the synthetic empty row.
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                if start + idx == self.tags_selected && self.focus == Focus::Tags && !tags_empty {
                    let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style)).collect();
                    ListItem::new(Line::from(spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)))
                } else if (idx + start).is_multiple_of(2) && !tags_empty {
                    ListItem::new(Line::from(line.clone().spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)))
                } else {
                    ListItem::new(line.clone())
                }
            })
            .collect();

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

            return;
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
    }
}
