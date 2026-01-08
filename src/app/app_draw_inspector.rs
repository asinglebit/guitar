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
    widgets::{Block, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_inspector(&mut self, frame: &mut Frame) {
        // Padding
        let padding = ratatui::widgets::Padding {
            left: 1,
            right: 1,
            top: 0,
            bottom: 0,
        };

        // Calculate maximum available width for text
        let available_width = self.layout.inspector.width as usize - 1;
        let max_text_width = available_width.saturating_sub(2);

        // Flags
        let is_showing_uncommitted = self.graph_selected == 0;

        // Lines
        let mut lines: Vec<Line<'_>> = Vec::new();

        if !is_showing_uncommitted {
            // Query commit info
            let alias = self.oids.get_alias_by_idx(self.graph_selected);
            let oid = self.oids.get_oid_by_alias(alias);
            let commit = self.repo.find_commit(*oid).unwrap();
            let author = commit.author();
            let committer = commit.committer();
            let summary = commit.summary().unwrap_or("⊘ no summary").to_string();
            let body = commit.body().unwrap_or("⊘ no body").to_string();

            // Assemble lines
            lines = vec![
                Line::from(Span::styled(
                    "commit sha:",
                    Style::default().fg(self.theme.COLOR_GREY_500),
                )),
                Line::from(Span::styled(
                    truncate_with_ellipsis(&format!("#{}", oid), max_text_width),
                    Style::default().fg(self.theme.COLOR_TEXT),
                )),
                Line::default(),
                Line::from(Span::styled(
                    "parent shas:",
                    Style::default().fg(self.theme.COLOR_GREY_500),
                )),
            ];
            for parent_id in commit.parent_ids() {
                let text = truncate_with_ellipsis(&format!("#{}", parent_id), max_text_width);
                lines.push(Line::from(Span::styled(
                    text,
                    Style::default().fg(self.theme.COLOR_TEXT),
                )));
            }
            if let Some(branches) = self.branches.all.get(&alias)
                && let Some(color) = self.branches.colors.get(&alias)
            {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    "featured branches:",
                    Style::default().fg(self.theme.COLOR_GREY_500),
                )));
                for branch in branches {
                    let text = truncate_with_ellipsis(&format!("● {}", branch), max_text_width);
                    lines.push(Line::from(Span::styled(text, Style::default().fg(*color))));
                }
            }
            lines.push(Line::default());
            lines.extend(vec![
                Line::from(Span::styled(
                    format!("authored by: {}", author.name().unwrap_or("-")),
                    Style::default().fg(self.theme.COLOR_GREY_500),
                )),
                Line::from(Span::styled(
                    author.email().unwrap_or("").to_string(),
                    Style::default().fg(self.theme.COLOR_TEXT),
                )),
                Line::from(Span::styled(
                    timestamp_to_utc(author.when()),
                    Style::default().fg(self.theme.COLOR_TEXT),
                )),
                Line::default(),
                Line::from(Span::styled(
                    format!("committed by: {}", committer.name().unwrap_or("-")),
                    Style::default().fg(self.theme.COLOR_GREY_500),
                )),
                Line::from(Span::styled(
                    committer.email().unwrap_or("").to_string(),
                    Style::default().fg(self.theme.COLOR_TEXT),
                )),
                Line::from(Span::styled(
                    timestamp_to_utc(committer.when()).to_string(),
                    Style::default().fg(self.theme.COLOR_TEXT),
                )),
                Line::default(),
                Line::from(Span::styled(
                    "message summary:",
                    Style::default().fg(self.theme.COLOR_GREY_500),
                )),
            ]);
            let wrapped = wrap_words(sanitize(summary), max_text_width);
            for line in wrapped {
                lines.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(self.theme.COLOR_TEXT),
                )));
            }
            lines.extend(vec![
                Line::default(),
                Line::from(Span::styled(
                    "message body:",
                    Style::default().fg(self.theme.COLOR_GREY_500),
                )),
            ]);
            let wrapped = wrap_words(sanitize(body), max_text_width);
            for line in wrapped {
                lines.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(self.theme.COLOR_TEXT),
                )));
            }
        }

        // Get vertical dimensions
        let total_lines = lines.len();
        let visible_height = self.layout.inspector.height as usize - 2;

        // Clamp selection
        if total_lines == 0 {
            self.inspector_selected = 0;
        } else if self.inspector_selected >= total_lines {
            self.inspector_selected = total_lines - 1;
        }

        // Trap selection
        self.trap_selection(
            self.inspector_selected,
            &self.inspector_scroll,
            total_lines,
            visible_height,
        );

        // Calculate scroll
        let start = self
            .inspector_scroll
            .get()
            .min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Setup list items
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if start + i == self.inspector_selected && self.focus == Focus::Inspector {
                    let spans: Vec<Span> = line
                        .iter()
                        .map(|span| {
                            Span::styled(
                                span.content.clone(),
                                span.style.fg(self.theme.COLOR_GREY_500),
                            )
                        })
                        .collect();
                    ListItem::new(Line::from(spans)).style(
                        Style::default()
                            .bg(self.theme.COLOR_GREY_800)
                            .fg(self.theme.COLOR_GREY_500),
                    )
                } else {
                    ListItem::new(line.clone())
                }
            })
            .collect();

        // Setup the list
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.inspector);

        // Setup the scrollbar
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height))
            .position(self.inspector_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("╮"))
            .end_symbol(if self.is_status {
                Some("│")
            } else {
                Some("╯")
            })
            .track_symbol(Some("│"))
            .thumb_symbol(if total_lines > visible_height {
                "▌"
            } else {
                "│"
            })
            .thumb_style(Style::default().fg(
                if total_lines > visible_height && self.focus == Focus::Inspector {
                    self.theme.COLOR_GREY_600
                } else {
                    self.theme.COLOR_BORDER
                },
            ));

        // Render the scrollbar
        frame.render_stateful_widget(
            scrollbar,
            self.layout.inspector_scrollbar,
            &mut scrollbar_state,
        );
    }
}
