use crate::helpers::layout::scrollbar_content_length;
use crate::helpers::localisation::empty;
use crate::helpers::text::{center_line, empty_state_top_padding};
use crate::{
    app::{
        app::{App, Focus},
        draw::pane_window::{aligned_pane_rows, blank_lines, preloaded_pane_window, zebra_list_items},
    },
    core::graph_service::{GraphPane, GraphPaneRow},
    helpers::colors::ColorPicker,
    helpers::text::truncate_with_ellipsis,
};
use ratatui::Frame;
use ratatui::widgets::Borders;
use ratatui::{layout::Rect, widgets::Paragraph};
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_stashes(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        // Left pane padding changes in zen mode because the pane has its own border.
        let padding = ratatui::widgets::Padding { left: if self.layout_config.is_zen { 1 } else { 2 }, right: 0, top: 0, bottom: 0 };

        // Reserve space for the stash icon and a separating space.
        let available_width = self.layout.stashes.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(3);

        let visible_height = if self.layout_config.is_zen {
            self.layout.stashes.height.saturating_sub(2) as usize
        } else {
            self.layout.stashes.height.saturating_sub(if self.layout_config.is_branches || self.layout_config.is_tags { 1 } else { 2 }) as usize
        };

        // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
        let total_lines = self.graph.stashes_window.as_ref().map(|window| window.total).unwrap_or(self.oids.stashes.len());

        if total_lines == 0 {
            self.stashes_selected = 0;
        } else if self.stashes_selected >= total_lines {
            self.stashes_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.stashes_selected, &self.stashes_scroll, total_lines, visible_height);

        let start = self.stashes_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);
        let (preload_start, preload_end) = preloaded_pane_window(start, end, total_lines, visible_height);
        self.request_pane_window(GraphPane::Stashes, preload_start, preload_end);

        // Stashes are stored as commit aliases, so each row reads its summary from git.
        let mut lines: Vec<Line<'_>> = Vec::new();
        let lines_are_windowed = self.graph_tx.is_some();
        let known_empty = self.graph.stashes_window.as_ref().is_some_and(|window| window.total == 0);
        if let Some(rows) = self.graph.stashes_window.as_ref().and_then(|window| aligned_pane_rows(window, start, end)) {
            let color_picker = ColorPicker::from_theme(&self.theme);
            for row in rows {
                if let Some(GraphPaneRow::Stash { summary, lane, .. }) = row {
                    let truncated = truncate_with_ellipsis(summary.as_str(), max_text_width.saturating_sub(1));
                    let color = lane.map(|lane| color_picker.get_lane_ref(lane)).unwrap_or(self.theme.COLOR_TEXT);
                    lines.push(Line::from(Span::styled(format!("{} {truncated}", self.symbols.graph.commit_stash), Style::default().fg(color))));
                } else {
                    lines.push(Line::default());
                }
            }
        } else if self.graph_tx.is_none() {
            for stash_alias in &self.oids.stashes {
                let oid = self.oids.get_git2_oid_by_alias(*stash_alias);
                let commit = repo.find_commit(oid).unwrap();
                let message = commit.summary().unwrap_or(empty::NO_MESSAGE()).to_string();

                let truncated = truncate_with_ellipsis(message.as_str(), max_text_width.saturating_sub(1));
                let color = if let Some(color) = self.stashes.colors.get(stash_alias) { *color } else { self.theme.COLOR_TEXT };

                lines.push(Line::from(Span::styled(format!("{} {truncated}", self.symbols.graph.commit_stash), Style::default().fg(color))));
            }
        } else if !known_empty {
            lines = blank_lines(if total_lines == 0 { visible_height } else { end.saturating_sub(start) });
        }

        // Empty state is part of the list so scrolling and borders still behave normally.
        let mut stashes_empty = false;
        if lines.is_empty() && (!lines_are_windowed || known_empty) {
            stashes_empty = true;
            let blank_lines_before = empty_state_top_padding(visible_height);
            for _ in 0..blank_lines_before {
                lines.push(Line::default());
            }
            let empty_text = format!("{} {}", self.symbols.empty_state.mark, empty::NO_STASHES());
            lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(&empty_text, max_text_width), max_text_width + 3), Style::default().fg(self.theme.COLOR_GREY_800))));
        }

        // Selection is skipped for the synthetic empty row; striping still fills the pane.
        let display_start = if stashes_empty || lines_are_windowed { 0 } else { start };
        let display_end = if stashes_empty || lines_are_windowed { lines.len() } else { end };
        let list_items = zebra_list_items(&lines[display_start..display_end], visible_height, start, self.stashes_selected, self.focus == Focus::Stashes, !stashes_empty, &self.theme);

        if self.layout_config.is_zen {
            // Zen mode frames the pane as a full standalone list.
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).padding(padding).border_set(self.symbols.border.block_set()));

            frame.render_widget(list, self.layout.stashes);

            let scroll_range = scrollbar_content_length(total_lines, visible_height);
            let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.stashes_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
                .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
                .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
                .track_style(Style::default().fg(self.theme.COLOR_BORDER))
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Stashes { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.stashes_scrollbar, &mut scrollbar_state);

            return;
        }

        // Normal mode draws a top separator when this pane is stacked under another pane.
        if self.layout_config.is_branches || self.layout_config.is_tags {
            let top_border = Paragraph::new(self.symbols.border.horizontal.repeat(self.layout.stashes.width.saturating_sub(1) as usize)).style(Style::default().fg(self.theme.COLOR_BORDER));
            frame.render_widget(top_border, Rect { x: self.layout.stashes.x + 1, y: self.layout.stashes.y.saturating_sub(1), width: self.layout.stashes.width, height: 1 });
        }
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.stashes);

        let scroll_range = scrollbar_content_length(total_lines, visible_height);
        let mut scrollbar_state = ScrollbarState::new(scroll_range).position(self.stashes_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(if self.layout_config.is_branches || self.layout_config.is_tags { self.symbols.border.vertical.as_str() } else { self.symbols.border.horizontal.as_str() }))
            .end_symbol(Some(if self.layout_config.is_reflogs || self.layout_config.is_worktrees || self.layout_config.is_search {
                self.symbols.border.vertical.as_str()
            } else {
                self.symbols.border.horizontal.as_str()
            }))
            .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
            .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
            .track_style(Style::default().fg(self.theme.COLOR_BORDER))
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Stashes { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.stashes_scrollbar, &mut scrollbar_state);
    }
}
