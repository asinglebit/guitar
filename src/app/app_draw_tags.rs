use crate::helpers::symbols::SYM_TAG;
use crate::{
    app::app::{App, Focus},
    helpers::text::truncate_with_ellipsis,
};
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use ratatui::{layout::Rect, widgets::Paragraph};

impl App {
    pub fn draw_tags(&mut self, frame: &mut Frame) {
        // Padding
        let padding = ratatui::widgets::Padding {
            left: 2,
            right: 0,
            top: 0,
            bottom: 0,
        };

        // Calculate maximum available width for text
        let available_width = self.layout.tags.width as usize - 1;
        let max_text_width = available_width.saturating_sub(3);

        // Lines
        let mut lines: Vec<Line<'_>> = Vec::new();
        for (tag_alias, tag_name) in self.tags.get_sorted_aliases() {
            // Text
            let truncated = truncate_with_ellipsis(tag_name, max_text_width - 1);
            let color = self.tags.get_color(&self.theme, tag_alias);

            // Render a tag
            lines.push(Line::from(Span::styled(
                format!("{SYM_TAG} {truncated}"),
                Style::default().fg(color),
            )));
        }

        // Get vertical dimensions
        let total_lines = lines.len();
        let visible_height =
            self.layout.tags.height as usize - if self.is_branches { 1 } else { 2 };

        // Clamp selection
        if total_lines == 0 {
            self.tags_selected = 0;
        } else if self.tags_selected >= total_lines {
            self.tags_selected = total_lines - 1;
        }

        // Trap selection
        self.trap_selection(
            self.tags_selected,
            &self.tags_scroll,
            total_lines,
            visible_height,
        );

        // Calculate scroll
        let start = self
            .tags_scroll
            .get()
            .min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Setup list items
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                if start + idx == self.tags_selected && self.focus == Focus::Tags {
                    let spans: Vec<Span> = line
                        .iter()
                        .map(|span| Span::styled(span.content.clone(), span.style))
                        .collect();
                    ListItem::new(Line::from(spans))
                        .style(Style::default().bg(self.theme.COLOR_GREY_800))
                } else if (idx + start).is_multiple_of(2) {
                    ListItem::new(Line::from(line.clone().spans))
                        .style(Style::default().bg(self.theme.COLOR_GREY_900))
                } else {
                    ListItem::new(line.clone())
                }
            })
            .collect();

        // Setup the list
        if self.is_branches || self.is_tags {
            let top_border = Paragraph::new("─".repeat(self.layout.tags.width as usize - 1_usize))
                .style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(
                top_border,
                Rect {
                    x: self.layout.tags.x + 1,
                    y: self.layout.tags.y - 1,
                    width: self.layout.tags.width,
                    height: 1,
                },
            );
        }
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.tags);

        // Setup the scrollbar
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height))
            .position(self.tags_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if self.is_branches { "│" } else { "─" }))
            .end_symbol(Some(if self.is_stashes { "│" } else { "─" }))
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height {
                "▌"
            } else {
                "│"
            })
            .thumb_style(Style::default().fg(
                if total_lines > visible_height && self.focus == Focus::Tags {
                    self.theme.COLOR_GREY_600
                } else {
                    self.theme.COLOR_BORDER
                },
            ));

        // Render the scrollbar
        frame.render_stateful_widget(scrollbar, self.layout.tags_scrollbar, &mut scrollbar_state);
    }
}
