use crate::helpers::symbols::SYM_COMMIT_STASH;
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
    pub fn draw_stashes(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        // Left pane padding changes in zen mode because the pane has its own border.
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };

        // Reserve space for the stash icon and a separating space.
        let available_width = self.layout.stashes.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        // Stashes are stored as commit aliases, so each row reads its summary from git.
        let mut lines: Vec<Line<'_>> = Vec::new();
        for stash_alias in &self.oids.stashes {
            let oid = self.oids.get_oid_by_alias(*stash_alias);
            let commit = repo.find_commit(*oid).unwrap();
            let message = commit.summary().unwrap_or("⊘ no message").to_string();

            let truncated = truncate_with_ellipsis(message.as_str(), max_text_width.saturating_sub(1));
            let color = if let Some(color) = self.stashes.colors.get(stash_alias) { *color } else { self.theme.COLOR_TEXT };

            lines.push(Line::from(Span::styled(format!("{SYM_COMMIT_STASH} {truncated}"), Style::default().fg(color))));
        }

        let visible_height = if self.layout_config.is_zen {
            self.layout.stashes.height.saturating_sub(2) as usize
        } else {
            self.layout.stashes.height.saturating_sub(if self.layout_config.is_branches || self.layout_config.is_tags { 1 } else { 2 }) as usize
        };

        // Empty state is part of the list so scrolling and borders still behave normally.
        let mut stashes_empty = false;
        if lines.is_empty() {
            stashes_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis("⊘ no stashes", max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
        let total_lines = lines.len();

        if total_lines == 0 {
            self.stashes_selected = 0;
        } else if self.stashes_selected >= total_lines {
            self.stashes_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.stashes_selected, &self.stashes_scroll, total_lines, visible_height);

        let start = self.stashes_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Selection and zebra striping are skipped for the synthetic empty row.
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                if start + idx == self.stashes_selected && self.focus == Focus::Stashes && !stashes_empty {
                    let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style)).collect();
                    ListItem::new(Line::from(spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)))
                } else if (idx + start).is_multiple_of(2) && !stashes_empty {
                    ListItem::new(Line::from(line.clone().spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)))
                } else {
                    ListItem::new(line.clone())
                }
            })
            .collect();

        if self.layout_config.is_zen {
            // Zen mode frames the pane as a full standalone list.
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_type(ratatui::widgets::BorderType::Rounded));

            frame.render_widget(list, self.layout.stashes);

            let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.stashes_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("╮"))
                .end_symbol(Some("╯"))
                .track_symbol(Some("│"))
                .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Stashes { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.stashes_scrollbar, &mut scrollbar_state);

            return;
        }

        // Normal mode draws a top separator when this pane is stacked under another pane.
        if self.layout_config.is_branches || self.layout_config.is_tags {
            let top_border = Paragraph::new("─".repeat(self.layout.stashes.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.stashes.x + 1, y: self.layout.stashes.y.saturating_sub(1), width: self.layout.stashes.width, height: 1 });
        }
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.stashes);

        let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.stashes_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if self.layout_config.is_branches || self.layout_config.is_tags { "│" } else { "─" }))
            .end_symbol(Some(if self.layout_config.is_reflogs || self.layout_config.is_worktrees { "│" } else { "─" }))
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Stashes { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.stashes_scrollbar, &mut scrollbar_state);
    }
}
