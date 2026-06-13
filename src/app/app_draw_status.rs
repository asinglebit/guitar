use crate::{
    app::app::{App, Focus},
    git::queries::helpers::FileStatus,
    helpers::text::*,
};
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_status(&mut self, frame: &mut Frame) {
        // Status panes keep icons close to the border and filenames flush after them.
        let padding = ratatui::widgets::Padding { left: 1, right: 0, top: 0, bottom: 0 };

        // Top is staged or commit diff; bottom exists only for unstaged uncommitted changes.
        let mut is_staged_changes = false;
        let mut is_unstaged_changes = false;
        let is_showing_uncommitted = self.graph_selected == 0;

        let mut lines_status_top: Vec<Line<'_>> = Vec::new();
        let mut lines_status_bottom: Vec<Line<'_>> = Vec::new();

        let mut status_top_empty = false;
        let mut status_bottom_empty = false;

        // Width leaves room for the change symbol and a little border padding.
        let max_status_top_width = self.layout.status_top.width.saturating_sub(5) as usize;
        let max_status_bottom_width = self.layout.status_bottom.width.saturating_sub(5) as usize;
        let visible_height_status_top = self.layout.status_top.height.saturating_sub(2) as usize;
        let visible_height_status_bottom = self.layout.status_bottom.height.saturating_sub(2) as usize;

        // The pseudo-row splits uncommitted files into staged and unstaged panes.
        if is_showing_uncommitted {
            for file in self.uncommitted.staged.modified.iter() {
                lines_status_top.push(Line::from(vec![
                    Span::styled("~ ", Style::default().fg(self.theme.COLOR_BLUE)),
                    Span::styled(truncate_with_ellipsis(file, max_status_top_width), Style::default().fg(self.theme.COLOR_TEXT)),
                ]));
            }
            for file in self.uncommitted.staged.added.iter() {
                lines_status_top.push(Line::from(vec![
                    Span::styled("+ ", Style::default().fg(self.theme.COLOR_GREEN)),
                    Span::styled(truncate_with_ellipsis(file, max_status_top_width), Style::default().fg(self.theme.COLOR_TEXT)),
                ]));
            }
            for file in self.uncommitted.staged.deleted.iter() {
                lines_status_top.push(Line::from(vec![
                    Span::styled("- ", Style::default().fg(self.theme.COLOR_RED)),
                    Span::styled(truncate_with_ellipsis(file, max_status_top_width), Style::default().fg(self.theme.COLOR_TEXT)),
                ]));
            }

            // Empty states are vertically padded to stay centered in short panes.
            if lines_status_top.is_empty() {
                status_top_empty = true;
                let blank_lines_before = empty_state_top_padding(visible_height_status_top);
                for _ in 0..blank_lines_before {
                    lines_status_top.push(Line::from(""));
                }
                lines_status_top.push(Line::from(Span::styled(
                    center_line(&truncate_with_ellipsis("⊘ no staged changes", max_status_top_width), max_status_top_width + 3),
                    Style::default().fg(self.theme.COLOR_GREY_800),
                )));
            } else {
                is_staged_changes = true;
            }

            for file in self.uncommitted.unstaged.modified.iter() {
                lines_status_bottom.push(Line::from(vec![
                    Span::styled("~ ", Style::default().fg(self.theme.COLOR_BLUE)),
                    Span::styled(truncate_with_ellipsis(file, max_status_bottom_width), Style::default().fg(self.theme.COLOR_TEXT)),
                ]));
            }
            for file in self.uncommitted.unstaged.added.iter() {
                lines_status_bottom.push(Line::from(vec![
                    Span::styled("+ ", Style::default().fg(self.theme.COLOR_GREEN)),
                    Span::styled(truncate_with_ellipsis(file, max_status_bottom_width), Style::default().fg(self.theme.COLOR_TEXT)),
                ]));
            }
            for file in self.uncommitted.unstaged.deleted.iter() {
                lines_status_bottom.push(Line::from(vec![
                    Span::styled("- ", Style::default().fg(self.theme.COLOR_RED)),
                    Span::styled(truncate_with_ellipsis(file, max_status_bottom_width), Style::default().fg(self.theme.COLOR_TEXT)),
                ]));
            }

            // Empty states are vertically padded to stay centered in short panes.
            if lines_status_bottom.is_empty() {
                status_bottom_empty = true;
                let blank_lines_before = empty_state_top_padding(visible_height_status_bottom);
                for _ in 0..blank_lines_before {
                    lines_status_bottom.push(Line::from(""));
                }
                lines_status_bottom.push(Line::from(Span::styled(
                    center_line(&truncate_with_ellipsis("⊘ no unstaged changes", max_status_bottom_width), max_status_bottom_width + 3),
                    Style::default().fg(self.theme.COLOR_GREY_800),
                )));
            } else {
                is_unstaged_changes = true;
            }
        } else {
            // Commit rows use the selected commit's file diff in the top pane only.
            for file_change in self.current_diff.iter() {
                let (symbol, color) = match file_change.status {
                    FileStatus::Added => ("+ ", self.theme.COLOR_GREEN),
                    FileStatus::Modified => ("~ ", self.theme.COLOR_BLUE),
                    FileStatus::Deleted => ("- ", self.theme.COLOR_RED),
                    FileStatus::Renamed => ("→ ", self.theme.COLOR_YELLOW),
                    FileStatus::Other => ("  ", self.theme.COLOR_TEXT),
                };
                let display_filename = truncate_with_ellipsis(&file_change.filename, max_status_top_width);
                lines_status_top.push(Line::from(vec![Span::styled(symbol, Style::default().fg(color)), Span::styled(display_filename, Style::default().fg(self.theme.COLOR_TEXT))]));
            }

            // Empty commits and unresolved diff failures share the same quiet state.
            if lines_status_top.is_empty() {
                status_top_empty = true;
                let blank_lines_before = empty_state_top_padding(visible_height_status_top);
                for _ in 0..blank_lines_before {
                    lines_status_top.push(Line::from(""));
                }
                lines_status_top.push(Line::from(Span::styled(
                    center_line(&truncate_with_ellipsis("⊘ no staged changes", max_status_top_width), max_status_top_width + 3),
                    Style::default().fg(self.theme.COLOR_GREY_800),
                )));
            } else {
                is_staged_changes = true;
            }
        }

        // Top status pane shows staged files on the pseudo-row or commit file changes otherwise.
        {
            // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
            let total_lines = lines_status_top.len();
            let visible_height = visible_height_status_top;

            if total_lines == 0 {
                self.status_top_selected = 0;
            } else if self.status_top_selected >= total_lines {
                self.status_top_selected = total_lines.saturating_sub(1);
            }

            self.trap_selection(self.status_top_selected, &self.status_top_scroll, total_lines, visible_height);

            let start = self.status_top_scroll.get().min(total_lines.saturating_sub(visible_height));
            let end = (start + visible_height).min(total_lines);

            // Selection is disabled for synthetic empty-state rows.
            let list_items: Vec<ListItem> = lines_status_top[start..end]
                .iter()
                .enumerate()
                .map(|(idx, line)| {
                    if is_staged_changes && start + idx == self.status_top_selected && self.focus == Focus::StatusTop {
                        let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style.fg(self.theme.COLOR_GREY_500))).collect();
                        ListItem::new(Line::from(spans)).style(Style::default().bg(self.theme.COLOR_GREY_800).fg(self.theme.COLOR_GREY_500))
                    } else if !status_top_empty {
                        if (idx + start).is_multiple_of(2) { ListItem::new(Line::from(line.clone().spans)).style(Style::default().bg(self.theme.COLOR_GREY_900)) } else { ListItem::new(line.clone()) }
                    } else {
                        ListItem::new(line.clone())
                    }
                })
                .collect();

            if self.layout_config.is_zen {
                // Zen mode frames the pane as a full standalone list.
                let list = List::new(list_items)
                    .block(Block::default().padding(padding).borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Rounded).border_style(Style::default().fg(self.theme.COLOR_BORDER)));

                frame.render_widget(list, self.layout.status_top);

                let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.status_top_scroll.get());
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("╮"))
                    .end_symbol(Some("╯"))
                    .track_symbol(Some("│"))
                    .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                    .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::StatusTop { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

                frame.render_stateful_widget(scrollbar, self.layout.status_top_scrollbar, &mut scrollbar_state);
            } else {
                // Normal mode lets inspector/status share border segments.
                let list = List::new(list_items).block(
                    Block::default()
                        .padding(padding)
                        .borders(if self.layout_config.is_inspector && self.graph_selected != 0 { Borders::TOP } else { Borders::NONE })
                        .border_style(Style::default().fg(self.theme.COLOR_BORDER)),
                );

                frame.render_widget(list, self.layout.status_top);

                let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.status_top_scroll.get());
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(if self.layout_config.is_inspector && self.graph_selected != 0 { Some("│") } else { Some("╮") })
                    .end_symbol(if self.graph_selected == 0 { Some("┤") } else { Some("╯") })
                    .track_symbol(Some("│"))
                    .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                    .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::StatusTop { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

                frame.render_stateful_widget(scrollbar, self.layout.status_top_scrollbar, &mut scrollbar_state);
            }
        }

        // Bottom status pane is reserved for unstaged files on the pseudo-row.
        {
            if is_showing_uncommitted {
                // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
                let total_lines = lines_status_bottom.len();
                let visible_height = visible_height_status_bottom;

                if total_lines == 0 {
                    self.status_bottom_selected = 0;
                } else if self.status_bottom_selected >= total_lines {
                    self.status_bottom_selected = total_lines.saturating_sub(1);
                }

                self.trap_selection(self.status_bottom_selected, &self.status_bottom_scroll, total_lines, visible_height);

                let start = self.status_bottom_scroll.get().min(total_lines.saturating_sub(visible_height));
                let end = (start + visible_height).min(total_lines);

                // Selection is disabled for synthetic empty-state rows.
                let list_items: Vec<ListItem> = lines_status_bottom[start..end]
                    .iter()
                    .enumerate()
                    .map(|(idx, line)| {
                        if is_unstaged_changes && start + idx == self.status_bottom_selected && self.focus == Focus::StatusBottom {
                            let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style.fg(self.theme.COLOR_GREY_500))).collect();
                            ListItem::new(Line::from(spans)).style(Style::default().bg(self.theme.COLOR_GREY_800).fg(self.theme.COLOR_GREY_500))
                        } else if !status_bottom_empty {
                            if (idx + start).is_multiple_of(2) {
                                ListItem::new(Line::from(line.clone().spans)).style(Style::default().bg(self.theme.COLOR_GREY_900))
                            } else {
                                ListItem::new(line.clone())
                            }
                        } else {
                            ListItem::new(line.clone())
                        }
                    })
                    .collect();

                if self.layout_config.is_zen {
                    // Zen mode frames the pane as a full standalone list.
                    let list = List::new(list_items)
                        .block(Block::default().padding(padding).borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Rounded).border_style(Style::default().fg(self.theme.COLOR_BORDER)));

                    frame.render_widget(list, self.layout.status_bottom);

                    let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.status_bottom_scroll.get());
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("╮"))
                        .end_symbol(Some("╯"))
                        .track_symbol(Some("│"))
                        .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                        .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::StatusBottom { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

                    frame.render_stateful_widget(scrollbar, self.layout.status_bottom_scrollbar, &mut scrollbar_state);

                    return;
                }

                // Normal mode top border separates staged and unstaged lists.
                let list = List::new(list_items).block(Block::default().padding(padding).borders(Borders::TOP).border_style(Style::default().fg(self.theme.COLOR_BORDER)));

                frame.render_widget(list, self.layout.status_bottom);

                let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.status_bottom_scroll.get());
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("│"))
                    .end_symbol(Some("╯"))
                    .track_symbol(Some("│"))
                    .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                    .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::StatusBottom { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

                frame.render_stateful_widget(scrollbar, self.layout.status_bottom_scrollbar, &mut scrollbar_state);
            }
        }
    }
}
