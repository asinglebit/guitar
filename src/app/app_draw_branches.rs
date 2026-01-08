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

impl App {
    pub fn draw_branches(&mut self, frame: &mut Frame) {
        // Padding
        let padding = ratatui::widgets::Padding {
            left: 2,
            right: 0,
            top: 0,
            bottom: 0,
        };

        // Calculate maximum available width for text
        let available_width = self.layout.branches.width as usize - 1;
        let max_text_width = available_width.saturating_sub(3);

        // Lines
        let mut lines: Vec<Line<'_>> = Vec::new();
        for (branch_alias, branch_name) in self.branches.get_sorted_aliases().iter() {
            // Get branch descriptors
            let is_visible = self.branches.is_visible(branch_alias, branch_name);
            let is_local = self.branches.is_local(branch_name);

            // Text
            let truncated = truncate_with_ellipsis(branch_name, max_text_width - 1);
            let icon = if is_visible {
                if is_local { "●" } else { "◆" }
            } else if is_local {
                "○"
            } else {
                "◇"
            };
            let color = if is_visible {
                self.branches.get_color(&self.theme, branch_alias)
            } else {
                self.theme.COLOR_TEXT
            };

            // Render a branch
            lines.push(Line::from(Span::styled(
                format!("{icon} {truncated}"),
                Style::default().fg(color),
            )));
        }

        // Get vertical dimensions
        let total_lines = lines.len();
        let visible_height = self.layout.branches.height as usize - 2;

        // Clamp selection
        if total_lines == 0 {
            self.branches_selected = 0;
        } else if self.branches_selected >= total_lines {
            self.branches_selected = total_lines - 1;
        }

        // Trap selection
        self.trap_selection(
            self.branches_selected,
            &self.branches_scroll,
            total_lines,
            visible_height,
        );

        // Calculate scroll
        let start = self
            .branches_scroll
            .get()
            .min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Setup list items
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                if start + idx == self.branches_selected && self.focus == Focus::Branches {
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
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.branches);

        // Setup the scrollbar
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height))
            .position(self.branches_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("─"))
            .end_symbol(Some(if self.is_tags || self.is_stashes {
                "│"
            } else {
                "─"
            }))
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height {
                "▌"
            } else {
                "│"
            })
            .thumb_style(Style::default().fg(
                if total_lines > visible_height && self.focus == Focus::Branches {
                    self.theme.COLOR_GREY_600
                } else {
                    self.theme.COLOR_BORDER
                },
            ));

        // Render the scrollbar
        frame.render_stateful_widget(
            scrollbar,
            self.layout.branches_scrollbar,
            &mut scrollbar_state,
        );
    }
}
