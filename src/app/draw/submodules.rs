use crate::{
    app::{
        app::{App, Focus},
        draw::pane_window::zebra_list_items,
    },
    helpers::{
        layout::scrollbar_content_length,
        localisation::{common, empty, status as status_text},
        symbols::{border, branch as branch_symbol, scrollbar, submodule},
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
    fn submodule_status_suffix(&self, entry: &crate::core::submodules::SubmoduleEntry) -> String {
        let mut parts = Vec::new();
        if entry.is_uninitialized || !entry.is_open {
            parts.push(submodule::UNINITIALIZED.to_string());
        }
        if entry.is_index_modified {
            parts.push(status_text::STAGED.to_string());
        }
        if entry.has_new_commits {
            parts.push(status_text::NEW_COMMITS.to_string());
        }
        if entry.has_modified_content {
            parts.push(status_text::MODIFIED.to_string());
        }
        if entry.has_untracked_content {
            parts.push(status_text::UNTRACKED.to_string());
        }
        if entry.is_dirty() && parts.is_empty() {
            parts.push(submodule::DIRTY.to_string());
        }

        if parts.is_empty() { String::new() } else { format!(" [{}]", parts.join(", ")) }
    }

    pub fn draw_submodules(&mut self, frame: &mut Frame) {
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };
        let available_width = self.layout.submodules.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let mut lines: Vec<Line<'_>> = Vec::new();
        for entry in &self.submodules.entries {
            let target = entry
                .branch
                .as_ref()
                .map(|branch| format!("{} {branch}", branch_symbol::LOCAL_VISIBLE))
                .or_else(|| entry.workdir.map(|oid| format!("{} #{:.6}", status_text::DETACHED, oid)))
                .unwrap_or_else(|| common::NOT_INITIALIZED.to_string());
            let suffix = self.submodule_status_suffix(entry);
            let label = truncate_with_ellipsis(format!("{}, {}{}", entry.path.display(), target, suffix).as_str(), max_text_width.saturating_sub(1));

            let color = if !entry.is_open {
                self.theme.COLOR_GREY_800
            } else if entry.is_dirty() {
                self.theme.COLOR_ORANGE
            } else {
                self.theme.COLOR_TEAL
            };

            lines.push(Line::from(Span::styled(format!("{} {label}", submodule::DEFAULT), Style::default().fg(color))));
        }

        let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs || self.layout_config.is_worktrees;
        let visible_height =
            if self.layout_config.is_zen { self.layout.submodules.height.saturating_sub(2) as usize } else { self.layout.submodules.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize };

        let mut submodules_empty = false;
        if lines.is_empty() {
            submodules_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            let empty_text = format!("{} {}", submodule::EMPTY, empty::NO_SUBMODULES);
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(&empty_text, max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        let total_lines = lines.len();

        if total_lines == 0 {
            self.submodules_selected = 0;
        } else if self.submodules_selected >= total_lines {
            self.submodules_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.submodules_selected, &self.submodules_scroll, total_lines, visible_height);

        let start = self.submodules_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        let list_items = zebra_list_items(&lines[start..end], visible_height, start, self.submodules_selected, self.focus == Focus::Submodules, !submodules_empty, &self.theme);

        if self.layout_config.is_zen {
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_type(ratatui::widgets::BorderType::Rounded));

            frame.render_widget(list, self.layout.submodules);

            let scroll_range = scrollbar_content_length(total_lines, visible_height);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.submodules_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some(scrollbar::BEGIN))
                .end_symbol(Some(scrollbar::END))
                .track_symbol(Some(scrollbar::TRACK))
                .thumb_symbol(if total_lines > visible_height { scrollbar::THUMB } else { scrollbar::INACTIVE_THUMB })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Submodules { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.submodules_scrollbar, &mut scrollbar_state);

            return;
        }

        if has_previous {
            let top_border = Paragraph::new(border::HORIZONTAL.repeat(self.layout.submodules.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.submodules.x + 1, y: self.layout.submodules.y.saturating_sub(1), width: self.layout.submodules.width, height: 1 });
        }

        let list = List::new(list_items).block(Block::default().padding(padding));
        frame.render_widget(list, self.layout.submodules);

        let scroll_range = scrollbar_content_length(total_lines, visible_height);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(start);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if has_previous { border::VERTICAL } else { border::HORIZONTAL }))
            .end_symbol(Some(if self.layout_config.is_search { border::VERTICAL } else { border::HORIZONTAL }))
            .track_symbol(Some(scrollbar::TRACK))
            .thumb_symbol(if total_lines > visible_height { scrollbar::THUMB } else { scrollbar::INACTIVE_THUMB })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Submodules { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.submodules_scrollbar, &mut scrollbar_state);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/submodules.rs"]
mod tests;
