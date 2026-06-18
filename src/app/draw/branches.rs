use crate::{
    app::app::{App, Focus},
    app::draw::pane_window::{aligned_pane_rows, blank_lines, preloaded_pane_window, zebra_list_items},
    core::graph_service::{GraphPane, GraphPaneRow},
    helpers::colors::ColorPicker,
    helpers::layout::scrollbar_content_length,
    helpers::localisation::empty,
    helpers::symbols::{border, branch as branch_symbol, empty_state, scrollbar},
    helpers::text::{center_line, empty_state_top_padding, truncate_with_ellipsis},
};
use ratatui::Frame;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_branches(&mut self, frame: &mut Frame) {
        // Left pane padding changes in zen mode because the pane has its own border.
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };

        // Reserve space for the branch visibility icon and a separating space.
        let available_width = self.layout.branches.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let visible_height = self.layout.branches.height.saturating_sub(2) as usize;

        // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
        let total_lines = self.graph.branches_window.as_ref().map(|window| window.total).unwrap_or_else(|| self.branches.get_sorted_aliases().len());

        if total_lines == 0 {
            self.branches_selected = 0;
        } else if self.branches_selected >= total_lines {
            self.branches_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.branches_selected, &self.branches_scroll, total_lines, visible_height);

        let start = self.branches_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);
        let (preload_start, preload_end) = preloaded_pane_window(start, end, total_lines, visible_height);
        self.request_pane_window(GraphPane::Branches, preload_start, preload_end);

        // Render local/remote and visible/hidden state through the branch icon.
        let mut lines: Vec<Line<'_>> = Vec::new();
        let lines_are_windowed = self.graph_tx.is_some();
        let known_empty = self.graph.branches_window.as_ref().is_some_and(|window| window.total == 0);
        if let Some(rows) = self.graph.branches_window.as_ref().and_then(|window| aligned_pane_rows(window, start, end)) {
            let color_picker = ColorPicker::from_theme(&self.theme);
            for row in rows {
                if let Some(GraphPaneRow::Branch { name, is_local, lane, .. }) = row {
                    let is_visible = !self.branches.hidden_branch_names.contains(name);
                    let truncated = truncate_with_ellipsis(name, max_text_width.saturating_sub(1));
                    let icon = if is_visible {
                        if *is_local { branch_symbol::LOCAL_VISIBLE } else { branch_symbol::REMOTE_VISIBLE }
                    } else if *is_local {
                        branch_symbol::LOCAL_HIDDEN
                    } else {
                        branch_symbol::REMOTE_HIDDEN
                    };
                    let color = if is_visible { lane.map(|lane| color_picker.get_lane(lane)).unwrap_or(self.theme.COLOR_TEXT) } else { self.theme.COLOR_TEXT };
                    lines.push(Line::from(Span::styled(format!("{icon} {truncated}"), Style::default().fg(color))));
                } else {
                    lines.push(Line::default());
                }
            }
        } else if self.graph_tx.is_none() {
            for (branch_alias, branch_name) in self.branches.get_sorted_aliases().iter() {
                let is_visible = !self.branches.hidden_branch_names.contains(branch_name);
                let is_local = self.branches.is_local(branch_name);

                let truncated = truncate_with_ellipsis(branch_name, max_text_width.saturating_sub(1));
                let icon = if is_visible {
                    if is_local { branch_symbol::LOCAL_VISIBLE } else { branch_symbol::REMOTE_VISIBLE }
                } else if is_local {
                    branch_symbol::LOCAL_HIDDEN
                } else {
                    branch_symbol::REMOTE_HIDDEN
                };
                let color = if is_visible { self.branches.get_color(&self.theme, branch_alias) } else { self.theme.COLOR_TEXT };

                lines.push(Line::from(Span::styled(format!("{icon} {truncated}"), Style::default().fg(color))));
            }
        } else if !known_empty {
            lines = blank_lines(if total_lines == 0 { visible_height } else { end.saturating_sub(start) });
        }

        // Empty state is part of the list so scrolling and borders still behave normally.
        let mut branches_empty = false;
        if lines.is_empty() && (!lines_are_windowed || known_empty) {
            branches_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            let empty_text = format!("{} {}", empty_state::MARK, empty::NO_BRANCHES);
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(&empty_text, max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        // Selection is skipped for the synthetic empty row; striping still fills the pane.
        let display_start = if branches_empty || lines_are_windowed { 0 } else { start };
        let display_end = if branches_empty || lines_are_windowed { lines.len() } else { end };
        let list_items = zebra_list_items(&lines[display_start..display_end], visible_height, start, self.branches_selected, self.focus == Focus::Branches, !branches_empty, &self.theme);

        if self.layout_config.is_zen {
            // Zen mode frames the pane as a full standalone list.
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_type(ratatui::widgets::BorderType::Rounded));

            frame.render_widget(list, self.layout.branches);

            let scroll_range = scrollbar_content_length(total_lines, visible_height);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.branches_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some(scrollbar::BEGIN))
                .end_symbol(Some(scrollbar::END))
                .track_symbol(Some(scrollbar::TRACK))
                .thumb_symbol(if total_lines > visible_height { scrollbar::THUMB } else { scrollbar::INACTIVE_THUMB })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Branches { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.branches_scrollbar, &mut scrollbar_state);

            return;
        }

        // Normal mode relies on the parent layout to draw pane separators.
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.branches);

        let scroll_range = scrollbar_content_length(total_lines, visible_height);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.branches_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(border::HORIZONTAL))
            .end_symbol(Some(if self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs || self.layout_config.is_worktrees || self.layout_config.is_search {
                border::VERTICAL
            } else {
                border::HORIZONTAL
            }))
            .track_symbol(Some(scrollbar::TRACK))
            .thumb_symbol(if total_lines > visible_height { scrollbar::THUMB } else { scrollbar::INACTIVE_THUMB })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Branches { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.branches_scrollbar, &mut scrollbar_state);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/branches.rs"]
mod tests;
