use crate::{
    app::{
        app::{App, Focus},
        draw::{buffered::DrawTarget, pane_window::zebra_list_items},
    },
    helpers::text::{center_line, empty_state_top_padding, truncate_with_ellipsis},
};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_search(&mut self, frame: &mut impl DrawTarget) {
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };
        let available_width = self.layout.search.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs || self.layout_config.is_worktrees;
        let visible_height =
            if self.layout_config.is_zen { self.layout.search.height.saturating_sub(2) as usize } else { self.layout.search.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize };

        self.search_selected = 0;
        self.trap_selection(self.search_selected, &self.search_scroll, 0, visible_height);

        let mut lines: Vec<Line<'_>> = Vec::new();
        let blank_lines_before = empty_state_top_padding(visible_height);
        for _ in 0..blank_lines_before {
            lines.push(Line::default());
        }
        lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis("search", max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));

        let list_items = zebra_list_items(&lines, visible_height, 0, self.search_selected, self.focus == Focus::Search, false, &self.theme);

        if self.layout_config.is_zen {
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_type(ratatui::widgets::BorderType::Rounded));
            frame.render_widget(list, self.layout.search);

            let mut scrollbar_state = ScrollbarState::new(1).position(0);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("╮"))
                .end_symbol(Some("╯"))
                .track_symbol(Some("│"))
                .thumb_symbol("│")
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(self.theme.COLOR_BORDER));

            frame.render_stateful_widget(scrollbar, self.layout.search_scrollbar, &mut scrollbar_state);
            return;
        }

        if has_previous {
            let top_border = Paragraph::new("─".repeat(self.layout.search.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.search.x + 1, y: self.layout.search.y.saturating_sub(1), width: self.layout.search.width, height: 1 });
        }

        let list = List::new(list_items).block(Block::default().padding(padding));
        frame.render_widget(list, self.layout.search);

        let mut scrollbar_state = ScrollbarState::new(1).position(0);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if has_previous { "│" } else { "─" }))
            .end_symbol(Some("─"))
            .track_symbol(Some("│"))
            .thumb_symbol("│")
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(self.theme.COLOR_BORDER));

        frame.render_stateful_widget(scrollbar, self.layout.search_scrollbar, &mut scrollbar_state);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/search.rs"]
mod tests;
