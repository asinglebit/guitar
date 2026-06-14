use crate::{
    app::app::{App, Focus},
    helpers::text::{center_line, empty_state_top_padding, truncate_with_ellipsis},
};
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_branches(&mut self, frame: &mut Frame) {
        // Left pane padding changes in zen mode because the pane has its own border.
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };

        // Reserve space for the branch visibility icon and a separating space.
        let available_width = self.layout.branches.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        // Render local/remote and visible/hidden state through the branch icon.
        let mut lines: Vec<Line<'_>> = Vec::new();
        for (branch_alias, branch_name) in self.branches.get_sorted_aliases().iter() {
            let is_visible = self.branches.visible_branch_names.contains(branch_name) || self.branches.visible_branch_names.is_empty();
            let is_local = self.branches.is_local(branch_name);

            let truncated = truncate_with_ellipsis(branch_name, max_text_width.saturating_sub(1));
            let icon = if is_visible {
                if is_local { "●" } else { "◆" }
            } else if is_local {
                "○"
            } else {
                "◇"
            };
            let color = if is_visible { self.branches.get_color(&self.theme, branch_alias) } else { self.theme.COLOR_TEXT };

            lines.push(Line::from(Span::styled(format!("{icon} {truncated}"), Style::default().fg(color))));
        }

        let visible_height = self.layout.branches.height.saturating_sub(2) as usize;

        // Empty state is part of the list so scrolling and borders still behave normally.
        let mut branches_empty = false;
        if lines.is_empty() {
            branches_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis("⊘ no branches", max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
        let total_lines = lines.len();

        if total_lines == 0 {
            self.branches_selected = 0;
        } else if self.branches_selected >= total_lines {
            self.branches_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.branches_selected, &self.branches_scroll, total_lines, visible_height);

        let start = self.branches_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Selection and zebra striping are skipped for the synthetic empty row.
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                if start + idx == self.branches_selected && self.focus == Focus::Branches && !branches_empty {
                    let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style)).collect();
                    ListItem::new(Line::from(spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)))
                } else if (idx + start).is_multiple_of(2) && !branches_empty {
                    ListItem::new(Line::from(line.clone().spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)))
                } else {
                    ListItem::new(line.clone())
                }
            })
            .collect();

        if self.layout_config.is_zen {
            // Zen mode frames the pane as a full standalone list.
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_type(ratatui::widgets::BorderType::Rounded));

            frame.render_widget(list, self.layout.branches);

            let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.branches_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("╮"))
                .end_symbol(Some("╯"))
                .track_symbol(Some("│"))
                .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Branches { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.branches_scrollbar, &mut scrollbar_state);

            return;
        }

        // Normal mode relies on the parent layout to draw pane separators.
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.branches);

        let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.branches_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("─"))
            .end_symbol(Some(if self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs || self.layout_config.is_worktrees { "│" } else { "─" }))
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Branches { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.branches_scrollbar, &mut scrollbar_state);
    }
}
