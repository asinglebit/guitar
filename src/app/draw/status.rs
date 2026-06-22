use crate::{
    app::app::{App, Focus},
    git::queries::helpers::FileStatus,
    helpers::{
        layout::scrollbar_content_length,
        localisation::{common, empty},
        text::*,
    },
};
use ratatui::Frame;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

#[derive(Clone)]
struct StatusRow<'a> {
    line: Line<'a>,
    path: Option<&'a str>,
}

impl<'a> StatusRow<'a> {
    fn plain(line: Line<'a>) -> Self {
        Self { line, path: None }
    }

    fn file(path: &'a str, symbol: &str, symbol_style: Style, text_style: Style, max_width: usize) -> Self {
        Self { line: Line::from(vec![Span::styled(symbol.to_string(), symbol_style), Span::styled(truncate_with_ellipsis(path, max_width), text_style)]), path: Some(path) }
    }
}

impl App {
    pub fn draw_status(&mut self, frame: &mut Frame) {
        // Status panes keep icons close to the border and filenames flush after them.
        let padding = ratatui::widgets::Padding { left: 1, right: 0, top: 0, bottom: 0 };

        // Top is staged or commit diff; bottom exists only for unstaged uncommitted changes.
        let mut is_staged_changes = false;
        let mut is_unstaged_changes = false;
        let is_showing_uncommitted = self.graph_selected == 0;

        let mut lines_status_top: Vec<StatusRow<'_>> = Vec::new();
        let mut lines_status_bottom: Vec<StatusRow<'_>> = Vec::new();

        let mut status_top_empty = false;
        let mut status_bottom_empty = false;

        // Width leaves room for the change symbol and a little border padding.
        let max_status_top_width = self.layout.status_top.width.saturating_sub(5) as usize;
        let max_status_bottom_width = self.layout.status_bottom.width.saturating_sub(5) as usize;
        let visible_height_status_top = self.layout.status_top.height.saturating_sub(2) as usize;
        let visible_height_status_bottom = self.layout.status_bottom.height.saturating_sub(2) as usize;
        let is_uncommitted_loading = is_showing_uncommitted && !self.is_uncommitted_loaded;
        let is_uncommitted_detail_loading = is_showing_uncommitted && self.is_uncommitted_loaded && self.is_uncommitted_detail_loading;
        let is_commit_diff_loading = !is_showing_uncommitted && !self.selected_commit_diff_is_loaded();

        // The pseudo-row splits uncommitted files into staged and unstaged panes.
        if is_uncommitted_loading {
            status_top_empty = true;
            status_bottom_empty = true;
            lines_status_top = centered_loading_lines(visible_height_status_top, max_status_top_width + 3, Style::default().fg(self.theme.COLOR_GREY_800));
            lines_status_bottom = centered_loading_lines(visible_height_status_bottom, max_status_bottom_width + 3, Style::default().fg(self.theme.COLOR_GREY_800));
        } else if is_showing_uncommitted {
            for file in self.uncommitted.conflicts.iter() {
                lines_status_top.push(StatusRow::file(
                    file,
                    &self.symbols.status.conflict_spaced,
                    Style::default().fg(self.theme.COLOR_ORANGE),
                    Style::default().fg(self.theme.COLOR_ORANGE),
                    max_status_top_width,
                ));
            }
            for file in self.uncommitted.staged.modified.iter() {
                lines_status_top.push(StatusRow::file(
                    file,
                    &self.symbols.status.modified_spaced,
                    Style::default().fg(self.theme.COLOR_BLUE),
                    Style::default().fg(self.theme.COLOR_TEXT),
                    max_status_top_width,
                ));
            }
            for file in self.uncommitted.staged.added.iter() {
                lines_status_top.push(StatusRow::file(
                    file,
                    &self.symbols.status.added_spaced,
                    Style::default().fg(self.theme.COLOR_GREEN),
                    Style::default().fg(self.theme.COLOR_TEXT),
                    max_status_top_width,
                ));
            }
            for file in self.uncommitted.staged.deleted.iter() {
                lines_status_top.push(StatusRow::file(
                    file,
                    &self.symbols.status.deleted_spaced,
                    Style::default().fg(self.theme.COLOR_RED),
                    Style::default().fg(self.theme.COLOR_TEXT),
                    max_status_top_width,
                ));
            }

            // Empty states are vertically padded to stay centered in short panes.
            if lines_status_top.is_empty() {
                status_top_empty = true;
                let blank_lines_before = empty_state_top_padding(visible_height_status_top);
                for _ in 0..blank_lines_before {
                    lines_status_top.push(StatusRow::plain(Line::from("")));
                }
                lines_status_top.push(StatusRow::plain(Line::from(Span::styled(
                    center_line(&truncate_with_ellipsis(&format!("{} {}", self.symbols.empty_state.mark, empty::NO_STAGED_CHANGES()), max_status_top_width), max_status_top_width + 3),
                    Style::default().fg(self.theme.COLOR_GREY_800),
                ))));
            } else {
                is_staged_changes = true;
            }

            for file in self.uncommitted.conflicts.iter() {
                lines_status_bottom.push(StatusRow::file(
                    file,
                    &self.symbols.status.conflict_spaced,
                    Style::default().fg(self.theme.COLOR_ORANGE),
                    Style::default().fg(self.theme.COLOR_ORANGE),
                    max_status_bottom_width,
                ));
            }
            if is_uncommitted_detail_loading {
                lines_status_bottom = centered_loading_lines(visible_height_status_bottom, max_status_bottom_width + 3, Style::default().fg(self.theme.COLOR_GREY_800));
            } else {
                for file in self.uncommitted.unstaged.modified.iter() {
                    lines_status_bottom.push(StatusRow::file(
                        file,
                        &self.symbols.status.modified_spaced,
                        Style::default().fg(self.theme.COLOR_BLUE),
                        Style::default().fg(self.theme.COLOR_TEXT),
                        max_status_bottom_width,
                    ));
                }
                for file in self.uncommitted.unstaged.added.iter() {
                    lines_status_bottom.push(StatusRow::file(
                        file,
                        &self.symbols.status.added_spaced,
                        Style::default().fg(self.theme.COLOR_GREEN),
                        Style::default().fg(self.theme.COLOR_TEXT),
                        max_status_bottom_width,
                    ));
                }
                for file in self.uncommitted.unstaged.deleted.iter() {
                    lines_status_bottom.push(StatusRow::file(
                        file,
                        &self.symbols.status.deleted_spaced,
                        Style::default().fg(self.theme.COLOR_RED),
                        Style::default().fg(self.theme.COLOR_TEXT),
                        max_status_bottom_width,
                    ));
                }
            }

            // Empty states are vertically padded to stay centered in short panes.
            if lines_status_bottom.is_empty() {
                status_bottom_empty = true;
                let blank_lines_before = empty_state_top_padding(visible_height_status_bottom);
                for _ in 0..blank_lines_before {
                    lines_status_bottom.push(StatusRow::plain(Line::from("")));
                }
                lines_status_bottom.push(StatusRow::plain(Line::from(Span::styled(
                    center_line(&truncate_with_ellipsis(&format!("{} {}", self.symbols.empty_state.mark, empty::NO_UNSTAGED_CHANGES()), max_status_bottom_width), max_status_bottom_width + 3),
                    Style::default().fg(self.theme.COLOR_GREY_800),
                ))));
            } else {
                is_unstaged_changes = true;
            }
        } else if is_commit_diff_loading {
            status_top_empty = true;
            lines_status_top = centered_loading_lines(visible_height_status_top, max_status_top_width + 3, Style::default().fg(self.theme.COLOR_GREY_800));
        } else {
            // Commit rows use the selected commit's file diff in the top pane only.
            for file_change in self.current_diff.iter() {
                let (symbol, color) = match file_change.status {
                    FileStatus::Added => (self.symbols.status.added_spaced.as_str(), self.theme.COLOR_GREEN),
                    FileStatus::Modified => (self.symbols.status.modified_spaced.as_str(), self.theme.COLOR_BLUE),
                    FileStatus::Deleted => (self.symbols.status.deleted_spaced.as_str(), self.theme.COLOR_RED),
                    FileStatus::Renamed => (self.symbols.status.renamed_arrow_spaced.as_str(), self.theme.COLOR_YELLOW),
                    FileStatus::Other => (self.symbols.status.other_spaced.as_str(), self.theme.COLOR_TEXT),
                };
                lines_status_top.push(StatusRow::file(&file_change.filename, symbol, Style::default().fg(color), Style::default().fg(self.theme.COLOR_TEXT), max_status_top_width));
            }

            // Empty commits and unresolved diff failures share the same quiet state.
            if lines_status_top.is_empty() {
                status_top_empty = true;
                let blank_lines_before = empty_state_top_padding(visible_height_status_top);
                for _ in 0..blank_lines_before {
                    lines_status_top.push(StatusRow::plain(Line::from("")));
                }
                lines_status_top.push(StatusRow::plain(Line::from(Span::styled(
                    center_line(&truncate_with_ellipsis(&format!("{} {}", self.symbols.empty_state.mark, empty::NO_STAGED_CHANGES()), max_status_top_width), max_status_top_width + 3),
                    Style::default().fg(self.theme.COLOR_GREY_800),
                ))));
            } else {
                is_staged_changes = true;
            }
        }

        let search_highlight_path = if self.layout_config.is_search { self.search_path.as_deref() } else { None };

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
            let list_items = status_list_items(
                &lines_status_top[start..end],
                visible_height,
                start,
                self.status_top_selected,
                self.focus == Focus::StatusTop,
                is_staged_changes && !status_top_empty,
                search_highlight_path,
                &self.theme,
            );

            if self.layout_config.is_zen {
                // Zen mode frames the pane as a full standalone list.
                let list = List::new(list_items)
                    .block(Block::default().padding(padding).borders(Borders::ALL).border_set(self.symbols.border.block_set()).border_style(Style::default().fg(self.theme.COLOR_BORDER)));

                frame.render_widget(list, self.layout.status_top);

                let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(self.status_top_scroll.get());
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
                    .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
                    .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                    .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
                    .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::StatusTop { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

                frame.render_stateful_widget(scrollbar, self.layout.status_top_scrollbar, &mut scrollbar_state);
            } else {
                // Normal mode lets inspector/status share border segments.
                let list = List::new(list_items).block(
                    Block::default()
                        .padding(padding)
                        .borders(if self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) { Borders::TOP } else { Borders::NONE })
                        .border_style(Style::default().fg(self.theme.COLOR_BORDER)),
                );

                frame.render_widget(list, self.layout.status_top);

                let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(self.status_top_scroll.get());
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(if self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) {
                        Some(self.symbols.border.vertical.as_str())
                    } else {
                        Some(self.symbols.scrollbar.begin.as_str())
                    })
                    .end_symbol(if self.graph_selected == 0 { Some(self.symbols.border.t_right.as_str()) } else { Some(self.symbols.scrollbar.end.as_str()) })
                    .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                    .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
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
                let list_items = status_list_items(
                    &lines_status_bottom[start..end],
                    visible_height,
                    start,
                    self.status_bottom_selected,
                    self.focus == Focus::StatusBottom,
                    is_unstaged_changes && !status_bottom_empty,
                    search_highlight_path,
                    &self.theme,
                );

                if self.layout_config.is_zen {
                    // Zen mode frames the pane as a full standalone list.
                    let list = List::new(list_items)
                        .block(Block::default().padding(padding).borders(Borders::ALL).border_set(self.symbols.border.block_set()).border_style(Style::default().fg(self.theme.COLOR_BORDER)));

                    frame.render_widget(list, self.layout.status_bottom);

                    let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(self.status_bottom_scroll.get());
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
                        .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
                        .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                        .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
                        .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::StatusBottom { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

                    frame.render_stateful_widget(scrollbar, self.layout.status_bottom_scrollbar, &mut scrollbar_state);

                    return;
                }

                // Normal mode top border separates staged and unstaged lists.
                let list = List::new(list_items).block(Block::default().padding(padding).borders(Borders::TOP).border_style(Style::default().fg(self.theme.COLOR_BORDER)));

                frame.render_widget(list, self.layout.status_bottom);

                let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(self.status_bottom_scroll.get());
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some(self.symbols.border.vertical.as_str()))
                    .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
                    .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                    .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
                    .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::StatusBottom { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

                frame.render_stateful_widget(scrollbar, self.layout.status_bottom_scrollbar, &mut scrollbar_state);
            }
        }
    }
}

fn centered_loading_lines(visible_height: usize, width: usize, style: Style) -> Vec<StatusRow<'static>> {
    let mut lines = Vec::new();
    for _ in 0..empty_state_top_padding(visible_height) {
        lines.push(StatusRow::plain(Line::from("")));
    }
    lines.push(StatusRow::plain(Line::from(Span::styled(center_line(&truncate_with_ellipsis(common::LOADING(), width), width), style))));
    lines
}

fn status_list_items<'a>(
    rows: &[StatusRow<'a>], visible_height: usize, start: usize, selected: usize, is_focused: bool, selection_enabled: bool, search_highlight_path: Option<&str>,
    theme: &crate::helpers::palette::Theme,
) -> Vec<ListItem<'a>> {
    (0..visible_height)
        .map(|idx| {
            let row = rows.get(idx).cloned().unwrap_or_else(|| StatusRow::plain(Line::default()));
            let global_idx = start + idx;
            let is_selected = selection_enabled && is_focused && global_idx == selected;
            let is_search_highlighted = row.path.zip(search_highlight_path).is_some_and(|(path, searched)| path == searched);
            let is_highlighted = is_selected || is_search_highlighted;

            let mut item = if is_highlighted {
                let spans: Vec<Span> = row.line.iter().map(|span| Span::styled(span.content.clone(), span.style.fg(theme.COLOR_HIGHLIGHTED))).collect();
                ListItem::new(Line::from(spans)).style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_800)).fg(theme.COLOR_HIGHLIGHTED))
            } else {
                ListItem::new(row.line)
            };

            if !is_highlighted && global_idx.is_multiple_of(2) {
                item = item.style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_900)));
            }

            item
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/app/draw/status.rs"]
mod tests;
