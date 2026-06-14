use crate::{
    app::app::{App, Focus},
    helpers::{
        symbols::SYM_REFLOG,
        text::{center_line, empty_state_top_padding, truncate_with_ellipsis},
    },
};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_reflogs(&mut self, frame: &mut Frame) {
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };
        let available_width = self.layout.reflogs.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let mut lines: Vec<Line<'_>> = Vec::new();
        for entry in &self.reflogs.entries {
            let label = truncate_with_ellipsis(&format!("{} {}", entry.selector, entry.message), max_text_width.saturating_sub(1));
            let color = self.reflogs.get_color(entry.new_alias).unwrap_or(self.theme.COLOR_TEXT);
            lines.push(Line::from(Span::styled(format!("{SYM_REFLOG} {label}"), Style::default().fg(color))));
        }

        let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes;
        let has_next = self.layout_config.is_worktrees;
        let visible_height =
            if self.layout_config.is_zen { self.layout.reflogs.height.saturating_sub(2) as usize } else { self.layout.reflogs.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize };

        let mut reflogs_empty = false;
        if lines.is_empty() {
            reflogs_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            let empty_text = format!("{SYM_REFLOG} no HEAD reflog");
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(&empty_text, max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        let total_lines = lines.len();

        if total_lines == 0 {
            self.reflogs_selected = 0;
        } else if self.reflogs_selected >= total_lines {
            self.reflogs_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.reflogs_selected, &self.reflogs_scroll, total_lines, visible_height);

        let start = self.reflogs_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                if start + idx == self.reflogs_selected && self.focus == Focus::Reflogs && !reflogs_empty {
                    let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style)).collect();
                    ListItem::new(Line::from(spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)))
                } else if (idx + start).is_multiple_of(2) && !reflogs_empty {
                    ListItem::new(Line::from(line.clone().spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)))
                } else {
                    ListItem::new(line.clone())
                }
            })
            .collect();

        if self.layout_config.is_zen {
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_type(ratatui::widgets::BorderType::Rounded));
            frame.render_widget(list, self.layout.reflogs);

            let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.reflogs_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("╮"))
                .end_symbol(Some("╯"))
                .track_symbol(Some("│"))
                .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Reflogs { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.reflogs_scrollbar, &mut scrollbar_state);
            return;
        }

        if has_previous {
            let top_border = Paragraph::new("─".repeat(self.layout.reflogs.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.reflogs.x + 1, y: self.layout.reflogs.y.saturating_sub(1), width: self.layout.reflogs.width, height: 1 });
        }

        let list = List::new(list_items).block(Block::default().padding(padding));
        frame.render_widget(list, self.layout.reflogs);

        let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.reflogs_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if has_previous { "│" } else { "─" }))
            .end_symbol(Some(if has_next { "│" } else { "─" }))
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Reflogs { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.reflogs_scrollbar, &mut scrollbar_state);
    }
}
