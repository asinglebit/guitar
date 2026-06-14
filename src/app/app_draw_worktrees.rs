use crate::{
    app::app::{App, Focus},
    helpers::{
        symbols::{SYM_COMMIT_BRANCH, SYM_WORKTREE, SYM_WORKTREE_DIRTY, SYM_WORKTREE_EMPTY, SYM_WORKTREE_INVALID, SYM_WORKTREE_LOCKED, SYM_WORKTREE_OTHER},
        text::{center_line, empty_state_top_padding, truncate_with_ellipsis},
    },
};
use ratatui::widgets::Borders;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_worktrees(&mut self, frame: &mut Frame) {
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };
        let available_width = self.layout.worktrees.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let mut lines: Vec<Line<'_>> = Vec::new();
        for entry in &self.worktrees.entries {
            let target =
                entry.branch.as_ref().map(|branch| format!("{SYM_COMMIT_BRANCH} {branch}")).or_else(|| entry.head.map(|oid| format!("detached #{:.6}", oid))).unwrap_or_else(|| "no head".to_string());

            let dirty = if entry.is_dirty { format!(" {SYM_WORKTREE_DIRTY}") } else { String::new() };
            let locked = if entry.locked_reason.is_some() { format!(" {SYM_WORKTREE_LOCKED}") } else { String::new() };
            let invalid = if !entry.is_valid { format!(" {SYM_WORKTREE_INVALID}") } else { String::new() };
            let label = truncate_with_ellipsis(format!("{}, branch: {} {}{}{}", entry.name, target, dirty, locked, invalid).as_str(), max_text_width.saturating_sub(1));

            let icon = if entry.is_current { SYM_WORKTREE } else { SYM_WORKTREE_OTHER };
            let color = if !entry.is_valid {
                self.theme.COLOR_GREY_800
            } else if entry.is_current {
                self.theme.COLOR_GRASS
            } else if entry.locked_reason.is_some() {
                self.theme.COLOR_GREY_600
            } else {
                self.theme.COLOR_TEAL
            };

            lines.push(Line::from(Span::styled(format!("{icon} {label}"), Style::default().fg(color))));
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
            let empty_text = format!("{SYM_WORKTREE_EMPTY} no worktrees");
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

        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                if start + idx == self.worktrees_selected && self.focus == Focus::Worktrees && !worktrees_empty {
                    let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style)).collect();
                    ListItem::new(Line::from(spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)))
                } else if (idx + start).is_multiple_of(2) && !worktrees_empty {
                    ListItem::new(Line::from(line.clone().spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)))
                } else {
                    ListItem::new(line.clone())
                }
            })
            .collect();

        if self.layout_config.is_zen {
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_type(ratatui::widgets::BorderType::Rounded));

            frame.render_widget(list, self.layout.worktrees);

            let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.worktrees_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("╮"))
                .end_symbol(Some("╯"))
                .track_symbol(Some("│"))
                .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Worktrees { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.worktrees_scrollbar, &mut scrollbar_state);

            return;
        }

        if self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs {
            let top_border = Paragraph::new("─".repeat(self.layout.worktrees.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.worktrees.x + 1, y: self.layout.worktrees.y.saturating_sub(1), width: self.layout.worktrees.width, height: 1 });
        }

        let list = List::new(list_items).block(Block::default().padding(padding));
        frame.render_widget(list, self.layout.worktrees);

        let scroll_range = (total_lines.saturating_sub(visible_height)).max(1);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.worktrees_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs { "│" } else { "─" }))
            .end_symbol(Some("─"))
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Worktrees { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.worktrees_scrollbar, &mut scrollbar_state);
    }
}
