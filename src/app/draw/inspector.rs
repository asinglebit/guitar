use crate::{
    app::app::{App, Focus, PendingGraphLookup},
    helpers::{
        colors::ColorPicker,
        layout::scrollbar_content_length,
        localisation::{common, empty, inspector},
        text::{center_line, empty_state_top_padding, sanitize, truncate_with_ellipsis, wrap_words},
        time::timestamp_to_utc,
    },
};
use ratatui::Frame;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_inspector(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        // Inspector text is padded on both sides for readability in the narrow right pane.
        let padding = ratatui::widgets::Padding { left: 1, right: 1, top: 0, bottom: 0 };

        // Width leaves room for padding and the scrollbar edge.
        let available_width = self.layout.inspector.width.saturating_sub(1) as usize;
        let max_text_width = available_width.saturating_sub(2);
        let visible_height = if self.layout_config.is_zen { self.layout.inspector.height.saturating_sub(2) as usize } else { self.layout.inspector.height.saturating_sub(1) as usize };

        // The inspector is intentionally empty for the uncommitted pseudo-row.
        let is_showing_uncommitted = self.graph_selected == 0;

        let mut lines: Vec<Line<'_>> = Vec::new();

        if is_showing_uncommitted && self.uncommitted.has_conflicts {
            lines = vec![
                Line::from(Span::styled(inspector::REPOSITORY_STATE, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))),
                Line::from(Span::styled(inspector::OPERATION_CONFLICTS, Style::default().fg(self.theme.COLOR_ORANGE))),
                Line::default(),
                Line::from(Span::styled(inspector::CONFLICTED_FILES, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))),
                Line::from(Span::styled(self.uncommitted.conflict_count.to_string(), Style::default().fg(self.theme.COLOR_ORANGE))),
                Line::default(),
                Line::from(Span::styled(inspector::NEXT_ACTION, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))),
                Line::from(Span::styled(inspector::RESOLVE_CONFLICTS_ACTION, Style::default().fg(self.theme.COLOR_TEXT))),
            ];
        } else if !is_showing_uncommitted {
            // Commit metadata is read lazily for the selected graph row.
            if let Some(identity) = self.graph_identity_at(self.graph_selected) {
                let alias = identity.alias;
                let oid = identity.oid;
                let commit = repo.find_commit(oid).unwrap();
                let author = commit.author();
                let committer = commit.committer();
                let summary = commit.summary().map(str::to_string).unwrap_or_else(|| format!("{} {}", self.symbols.empty_state.mark, empty::NO_SUMMARY));
                let body = commit.body().map(str::to_string).unwrap_or_else(|| format!("{} {}", self.symbols.empty_state.mark, empty::NO_BODY));

                // Sections are plain list rows so they scroll with the same pane machinery.
                lines = vec![
                    Line::from(Span::styled(inspector::COMMIT_SHA, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))),
                    Line::from(Span::styled(truncate_with_ellipsis(&format!("#{}", oid), max_text_width), Style::default().fg(self.theme.COLOR_TEXT))),
                    Line::default(),
                    Line::from(Span::styled(inspector::PARENT_SHAS, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))),
                ];
                for parent_id in commit.parent_ids() {
                    let text = truncate_with_ellipsis(&format!("#{}", parent_id), max_text_width);
                    lines.push(Line::from(Span::styled(text, Style::default().fg(self.theme.COLOR_TEXT))));
                }
                if let Some(row) = self.graph_row_at(self.graph_selected)
                    && !row.branches.is_empty()
                {
                    let color_picker = ColorPicker::from_theme(&self.theme);
                    lines.push(Line::default());
                    lines.push(Line::from(Span::styled(inspector::FEATURED_BRANCHES, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))));
                    for branch in &row.branches {
                        let text = truncate_with_ellipsis(&format!("{} {}", self.symbols.branch.local_visible, branch.name), max_text_width);
                        let color = branch.lane.map(|lane| color_picker.get_lane(lane)).unwrap_or(self.theme.COLOR_TEXT);
                        lines.push(Line::from(Span::styled(text, Style::default().fg(color))));
                    }
                } else if let Some(branches) = self.branches.all.get(&alias)
                    && let Some(color) = self.branches.colors.get(&alias)
                {
                    lines.push(Line::default());
                    lines.push(Line::from(Span::styled(inspector::FEATURED_BRANCHES, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))));
                    for branch in branches.iter().filter(|branch| !self.branches.hidden_branch_names.contains(*branch)) {
                        let text = truncate_with_ellipsis(&format!("{} {}", self.symbols.branch.local_visible, branch), max_text_width);
                        lines.push(Line::from(Span::styled(text, Style::default().fg(*color))));
                    }
                }
                if let Some(row) = self.graph_row_at(self.graph_selected)
                    && let Some(entry) = &row.reflog
                {
                    lines.push(Line::default());
                    lines.push(Line::from(Span::styled(inspector::HEAD_REFLOG, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))));
                    lines.push(Line::from(Span::styled(truncate_with_ellipsis(&entry.selector, max_text_width), Style::default().fg(self.theme.COLOR_TEXT))));
                    let wrapped = wrap_words(sanitize(entry.message.clone()), max_text_width);
                    for line in wrapped {
                        lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT))));
                    }
                } else if let Some(entry) = self.reflogs.latest_for_alias(alias) {
                    lines.push(Line::default());
                    lines.push(Line::from(Span::styled(inspector::HEAD_REFLOG, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))));
                    lines.push(Line::from(Span::styled(truncate_with_ellipsis(&entry.selector, max_text_width), Style::default().fg(self.reflogs.get_color(alias).unwrap_or(self.theme.COLOR_TEXT)))));
                    lines.push(Line::from(Span::styled(timestamp_to_utc(entry.time), Style::default().fg(self.theme.COLOR_TEXT))));
                    let wrapped = wrap_words(sanitize(entry.message.clone()), max_text_width);
                    for line in wrapped {
                        lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT))));
                    }
                }
                lines.push(Line::default());
                lines.extend(vec![
                    Line::from(Span::styled(format!("{} {}", inspector::AUTHORED_BY, author.name().unwrap_or(common::UNKNOWN)), Style::default().fg(self.theme.COLOR_HIGHLIGHTED))),
                    Line::from(Span::styled(author.email().unwrap_or("").to_string(), Style::default().fg(self.theme.COLOR_TEXT))),
                    Line::from(Span::styled(timestamp_to_utc(author.when()), Style::default().fg(self.theme.COLOR_TEXT))),
                    Line::default(),
                    Line::from(Span::styled(format!("{} {}", inspector::COMMITTED_BY, committer.name().unwrap_or(common::UNKNOWN)), Style::default().fg(self.theme.COLOR_HIGHLIGHTED))),
                    Line::from(Span::styled(committer.email().unwrap_or("").to_string(), Style::default().fg(self.theme.COLOR_TEXT))),
                    Line::from(Span::styled(timestamp_to_utc(committer.when()).to_string(), Style::default().fg(self.theme.COLOR_TEXT))),
                    Line::default(),
                    Line::from(Span::styled(inspector::MESSAGE_SUMMARY, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))),
                ]);
                let wrapped = wrap_words(sanitize(summary), max_text_width);
                for line in wrapped {
                    lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT))));
                }
                lines.extend(vec![Line::default(), Line::from(Span::styled(inspector::MESSAGE_BODY, Style::default().fg(self.theme.COLOR_HIGHLIGHTED)))]);
                let wrapped = wrap_words(sanitize(body), max_text_width);
                for line in wrapped {
                    lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT))));
                }
            } else {
                if self.graph_tx.is_some() && self.graph.pending_lookup.is_none() {
                    self.request_graph_row_lookup(self.graph_selected, PendingGraphLookup::CacheGraphRow);
                }

                lines = centered_loading_lines(visible_height, max_text_width, Style::default().fg(self.theme.COLOR_GREY_800));
            }
        }

        // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
        let total_lines = lines.len();

        if total_lines == 0 {
            self.inspector_selected = 0;
        } else if self.inspector_selected >= total_lines {
            self.inspector_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.inspector_selected, &self.inspector_scroll, total_lines, visible_height);

        let start = self.inspector_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Selection highlight dims text to keep metadata subordinate to graph selection.
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if start + i == self.inspector_selected && self.focus == Focus::Inspector {
                    let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style.fg(self.theme.COLOR_HIGHLIGHTED))).collect();
                    ListItem::new(Line::from(spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)).fg(self.theme.COLOR_HIGHLIGHTED))
                } else {
                    ListItem::new(line.clone())
                }
            })
            .collect();

        if self.layout_config.is_zen {
            // Zen mode frames the pane as a full standalone list.
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).border_set(self.symbols.border.block_set()).padding(padding));

            frame.render_widget(list, self.layout.inspector);

            let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(self.inspector_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
                .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
                .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Inspector { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.inspector_scrollbar, &mut scrollbar_state);

            return;
        }

        // Normal mode relies on the right pane border and scrollbar for framing.
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.inspector);

        let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(self.inspector_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
            .end_symbol(if self.layout_config.is_status { Some(self.symbols.border.vertical.as_str()) } else { Some(self.symbols.scrollbar.end.as_str()) })
            .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
            .thumb_symbol(if total_lines > visible_height { self.symbols.scrollbar.thumb.as_str() } else { self.symbols.scrollbar.inactive_thumb.as_str() })
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Inspector { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.inspector_scrollbar, &mut scrollbar_state);
    }
}

fn centered_loading_lines(visible_height: usize, max_width: usize, style: Style) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for _ in 0..empty_state_top_padding(visible_height) {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(center_line(&truncate_with_ellipsis(common::LOADING, max_width), max_width), style)));
    lines
}

#[cfg(test)]
#[path = "../../tests/app/draw/inspector.rs"]
mod tests;
