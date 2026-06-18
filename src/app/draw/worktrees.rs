use crate::{
    app::{
        app::{App, Focus},
        draw::pane_window::zebra_list_items,
    },
    helpers::{
        layout::scrollbar_content_length,
        localisation::{common, empty, status as status_text},
        symbols::{border, branch as branch_symbol, scrollbar, worktree},
        text::{center_line, empty_state_top_padding, truncate_with_ellipsis},
    },
};
use ratatui::Frame;
use ratatui::widgets::Borders;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_worktrees(&mut self, frame: &mut Frame) {
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };
        let available_width = self.layout.worktrees.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let mut lines: Vec<Line<'_>> = Vec::new();
        for entry in &self.worktrees.entries {
            let target = entry
                .branch
                .as_ref()
                .map(|branch| format!("{} {branch}", branch_symbol::LOCAL_VISIBLE))
                .or_else(|| entry.head.map(|oid| format!("{} #{:.6}", status_text::DETACHED, oid)))
                .unwrap_or_else(|| common::NO_HEAD.to_string());

            // Status icons stack on the right edge of the row; the worktree name and branch stay on the left.
            let mut status_parts: Vec<&str> = Vec::new();
            if entry.is_dirty {
                status_parts.push(worktree::DIRTY.trim_end());
            }
            if entry.locked_reason.is_some() {
                status_parts.push(worktree::LOCKED.trim_end());
            }
            if !entry.is_valid {
                status_parts.push(worktree::INVALID.trim_end());
            }
            let status = status_parts.join(" ");

            // Left side (name + branch) truncates first so the right-aligned status icons stay visible.
            let budget = max_text_width.saturating_sub(1);
            let status_len = status.chars().count();
            let reserve = if status_len == 0 { 0 } else { status_len + 1 };
            let left = truncate_with_ellipsis(format!("{} {}", entry.name, target).as_str(), budget.saturating_sub(reserve));
            let gap = budget.saturating_sub(left.chars().count() + status_len);
            let label = format!("{}{}{} ", left, " ".repeat(gap), status);

            let icon = if entry.is_current { worktree::CURRENT } else { worktree::OTHER };
            let color = if !entry.is_valid {
                self.theme.COLOR_GREY_800
            } else if entry.is_current {
                self.theme.COLOR_GRASS
            } else if entry.locked_reason.is_some() {
                self.theme.COLOR_GREY_600
            } else {
                self.theme.COLOR_TEAL
            };

            lines.push(Line::from(Span::styled(format!("{icon}{label}"), Style::default().fg(color))));
        }

        let visible_height = if self.layout_config.is_zen {
            self.layout.worktrees.height.saturating_sub(2) as usize
        } else {
            self.layout.worktrees.height.saturating_sub(if self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs {
                1
            } else {
                2
            }) as usize
        };

        let mut worktrees_empty = false;
        if lines.is_empty() {
            worktrees_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            let empty_text = format!("{}{}", worktree::EMPTY, empty::NO_WORKTREES);
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(&empty_text, max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        let total_lines = lines.len();

        if total_lines == 0 {
            self.worktrees_selected = 0;
        } else if self.worktrees_selected >= total_lines {
            self.worktrees_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.worktrees_selected, &self.worktrees_scroll, total_lines, visible_height);

        let start = self.worktrees_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        let list_items = zebra_list_items(&lines[start..end], visible_height, start, self.worktrees_selected, self.focus == Focus::Worktrees, !worktrees_empty, &self.theme);

        if self.layout_config.is_zen {
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_type(ratatui::widgets::BorderType::Rounded));

            frame.render_widget(list, self.layout.worktrees);

            let scroll_range = scrollbar_content_length(total_lines, visible_height);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.worktrees_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some(scrollbar::BEGIN))
                .end_symbol(Some(scrollbar::END))
                .track_symbol(Some(scrollbar::TRACK))
                .thumb_symbol(if total_lines > visible_height { scrollbar::THUMB } else { scrollbar::INACTIVE_THUMB })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Worktrees { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.worktrees_scrollbar, &mut scrollbar_state);

            return;
        }

        if self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs {
            let top_border = Paragraph::new(border::HORIZONTAL.repeat(self.layout.worktrees.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.worktrees.x + 1, y: self.layout.worktrees.y.saturating_sub(1), width: self.layout.worktrees.width, height: 1 });
        }

        let list = List::new(list_items).block(Block::default().padding(padding));
        frame.render_widget(list, self.layout.worktrees);

        let scroll_range = scrollbar_content_length(total_lines, visible_height);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.worktrees_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs {
                border::VERTICAL
            } else {
                border::HORIZONTAL
            }))
            .end_symbol(Some(if self.layout_config.is_submodules || self.layout_config.is_search { border::VERTICAL } else { border::HORIZONTAL }))
            .track_symbol(Some(scrollbar::TRACK))
            .thumb_symbol(if total_lines > visible_height { scrollbar::THUMB } else { scrollbar::INACTIVE_THUMB })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Worktrees { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.worktrees_scrollbar, &mut scrollbar_state);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/worktrees.rs"]
mod tests;
