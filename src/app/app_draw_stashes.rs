use crate::helpers::symbols::SYM_COMMIT_STASH;
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
    pub fn draw_stashes(&mut self, frame: &mut Frame) {
        // Padding
        let padding = ratatui::widgets::Padding {
            left: 2,
            right: 0,
            top: 0,
            bottom: 0,
        };

        // Calculate maximum available width for text
        let available_width = self.layout.stashes.width as usize - 1;
        let max_text_width = available_width.saturating_sub(3);

        // Lines
        let mut lines: Vec<Line<'_>> = Vec::new();
        for stash_alias in &self.oids.stashes {
            let oid = self.oids.get_oid_by_alias(*stash_alias);
            let commit = self.repo.find_commit(*oid).unwrap();
            let message = commit.summary().unwrap_or("⊘ no message").to_string();

            // Text
            let truncated = truncate_with_ellipsis(message.as_str(), max_text_width - 1);
            let color = if let Some(color) = self.stashes.colors.get(stash_alias) {
                *color
            } else {
                self.theme.COLOR_TEXT
            };

            // Render a stash
            lines.push(Line::from(Span::styled(
                format!("{SYM_COMMIT_STASH} {truncated}"),
                Style::default().fg(color),
            )));
        }

        // Get vertical dimensions
        let total_lines = lines.len();
        let visible_height = self.layout.stashes.height as usize
            - if self.is_branches || self.is_tags {
                1
            } else {
                2
            };

        // Clamp selection
        if total_lines == 0 {
            self.stashes_selected = 0;
        } else if self.stashes_selected >= total_lines {
            self.stashes_selected = total_lines - 1;
        }

        // Trap selection
        self.trap_selection(
            self.stashes_selected,
            &self.stashes_scroll,
            total_lines,
            visible_height,
        );

        // Calculate scroll
        let start = self
            .stashes_scroll
            .get()
            .min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Setup list items
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                if start + idx == self.stashes_selected && self.focus == Focus::Stashes {
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
            let top_border =
                Paragraph::new("─".repeat(self.layout.stashes.width as usize - 1_usize))
                    .style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(
                top_border,
                Rect {
                    x: self.layout.stashes.x + 1,
                    y: self.layout.stashes.y - 1,
                    width: self.layout.stashes.width,
                    height: 1,
                },
            );
        }
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.stashes);

        // Setup the scrollbar
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height))
            .position(self.stashes_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if self.is_branches || self.is_tags {
                "│"
            } else {
                "─"
            }))
            .end_symbol(Some("─"))
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height {
                "▌"
            } else {
                "│"
            })
            .thumb_style(Style::default().fg(
                if total_lines > visible_height && self.focus == Focus::Stashes {
                    self.theme.COLOR_GREY_600
                } else {
                    self.theme.COLOR_BORDER
                },
            ));

        // Render the scrollbar
        frame.render_stateful_widget(
            scrollbar,
            self.layout.stashes_scrollbar,
            &mut scrollbar_state,
        );
    }
}
