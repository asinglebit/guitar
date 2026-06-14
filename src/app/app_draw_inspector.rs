use crate::{
    app::app::{App, Focus},
    helpers::{
        text::{sanitize, truncate_with_ellipsis, wrap_words},
        time::timestamp_to_utc,
    },
};
use ratatui::{
    Frame,
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

        // The inspector is intentionally empty for the uncommitted pseudo-row.
        let is_showing_uncommitted = self.graph_selected == 0;

        let mut lines: Vec<Line<'_>> = Vec::new();

        if is_showing_uncommitted && self.uncommitted.has_conflicts {
            lines = vec![
                Line::from(Span::styled("repository state:", Style::default().fg(self.theme.COLOR_GREY_500))),
                Line::from(Span::styled("operation conflicts", Style::default().fg(self.theme.COLOR_ORANGE))),
                Line::default(),
                Line::from(Span::styled("conflicted files:", Style::default().fg(self.theme.COLOR_GREY_500))),
                Line::from(Span::styled(self.uncommitted.conflict_count.to_string(), Style::default().fg(self.theme.COLOR_ORANGE))),
                Line::default(),
                Line::from(Span::styled("next action:", Style::default().fg(self.theme.COLOR_GREY_500))),
                Line::from(Span::styled("resolve files externally, then action+Shift+C", Style::default().fg(self.theme.COLOR_TEXT))),
            ];
        } else if !is_showing_uncommitted {
            // Commit metadata is read lazily for the selected graph row.
            let alias = self.oids.get_alias_by_idx(self.graph_selected);
            let oid = self.oids.get_oid_by_alias(alias);
            let commit = repo.find_commit(*oid).unwrap();
            let author = commit.author();
            let committer = commit.committer();
            let summary = commit.summary().unwrap_or("⊘ no summary").to_string();
            let body = commit.body().unwrap_or("⊘ no body").to_string();

            // Sections are plain list rows so they scroll with the same pane machinery.
            lines = vec![
                Line::from(Span::styled("commit sha:", Style::default().fg(self.theme.COLOR_GREY_500))),
                Line::from(Span::styled(truncate_with_ellipsis(&format!("#{}", oid), max_text_width), Style::default().fg(self.theme.COLOR_TEXT))),
                Line::default(),
                Line::from(Span::styled("parent shas:", Style::default().fg(self.theme.COLOR_GREY_500))),
            ];
            for parent_id in commit.parent_ids() {
                let text = truncate_with_ellipsis(&format!("#{}", parent_id), max_text_width);
                lines.push(Line::from(Span::styled(text, Style::default().fg(self.theme.COLOR_TEXT))));
            }
            if let Some(branches) = self.branches.all.get(&alias)
                && let Some(color) = self.branches.colors.get(&alias)
            {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled("featured branches:", Style::default().fg(self.theme.COLOR_GREY_500))));
                for branch in branches {
                    let text = truncate_with_ellipsis(&format!("● {}", branch), max_text_width);
                    lines.push(Line::from(Span::styled(text, Style::default().fg(*color))));
                }
            }
            if let Some(entry) = self.reflogs.latest_for_alias(alias) {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled("head reflog:", Style::default().fg(self.theme.COLOR_GREY_500))));
                lines.push(Line::from(Span::styled(truncate_with_ellipsis(&entry.selector, max_text_width), Style::default().fg(self.reflogs.get_color(alias).unwrap_or(self.theme.COLOR_TEXT)))));
                lines.push(Line::from(Span::styled(timestamp_to_utc(entry.time), Style::default().fg(self.theme.COLOR_TEXT))));
                let wrapped = wrap_words(sanitize(entry.message.clone()), max_text_width);
                for line in wrapped {
                    lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT))));
                }
            }
            lines.push(Line::default());
            lines.extend(vec![
                Line::from(Span::styled(format!("authored by: {}", author.name().unwrap_or("-")), Style::default().fg(self.theme.COLOR_GREY_500))),
                Line::from(Span::styled(author.email().unwrap_or("").to_string(), Style::default().fg(self.theme.COLOR_TEXT))),
                Line::from(Span::styled(timestamp_to_utc(author.when()), Style::default().fg(self.theme.COLOR_TEXT))),
                Line::default(),
                Line::from(Span::styled(format!("committed by: {}", committer.name().unwrap_or("-")), Style::default().fg(self.theme.COLOR_GREY_500))),
                Line::from(Span::styled(committer.email().unwrap_or("").to_string(), Style::default().fg(self.theme.COLOR_TEXT))),
                Line::from(Span::styled(timestamp_to_utc(committer.when()).to_string(), Style::default().fg(self.theme.COLOR_TEXT))),
                Line::default(),
                Line::from(Span::styled("message summary:", Style::default().fg(self.theme.COLOR_GREY_500))),
            ]);
            let wrapped = wrap_words(sanitize(summary), max_text_width);
            for line in wrapped {
                lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT))));
            }
            lines.extend(vec![Line::default(), Line::from(Span::styled("message body:", Style::default().fg(self.theme.COLOR_GREY_500)))]);
            let wrapped = wrap_words(sanitize(body), max_text_width);
            for line in wrapped {
                lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT))));
            }
        }

        // Shared pane list pattern: clamp selection, trap scroll, then slice visible rows.
        let total_lines = lines.len();
        let visible_height = if self.layout_config.is_zen { self.layout.inspector.height.saturating_sub(2) as usize } else { self.layout.inspector.height.saturating_sub(1) as usize };

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
                    let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style.fg(self.theme.COLOR_GREY_500))).collect();
                    ListItem::new(Line::from(spans)).style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)).fg(self.theme.COLOR_GREY_500))
                } else {
                    ListItem::new(line.clone())
                }
            })
            .collect();

        if self.layout_config.is_zen {
            // Zen mode frames the pane as a full standalone list.
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Rounded).padding(padding));

            frame.render_widget(list, self.layout.inspector);

            let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.inspector_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("╮"))
                .end_symbol(Some("╯"))
                .track_symbol(Some("│"))
                .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
                .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Inspector { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.inspector_scrollbar, &mut scrollbar_state);

            return;
        }

        // Normal mode relies on the right pane border and scrollbar for framing.
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.inspector);

        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.inspector_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("╮"))
            .end_symbol(if self.layout_config.is_status { Some("│") } else { Some("╯") })
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height { "▌" } else { "│" })
            .thumb_style(Style::default().fg(if total_lines > visible_height && self.focus == Focus::Inspector { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.inspector_scrollbar, &mut scrollbar_state);
    }
}
