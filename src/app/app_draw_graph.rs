use crate::app::app::{App, Focus};
use crate::core::renderers::{render_buffer_range, render_graph_range, render_message_range, render_sha_range};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::widgets::Paragraph;
use ratatui::{
    Frame,
    style::Style,
    widgets::{Block, Borders, Cell as WidgetCell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};

impl App {
    pub fn draw_graph(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        // Determine the visible graph window before asking the buffer to decompress it.
        let total_lines = self.oids.get_commit_count();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };

        if total_lines == 0 {
            self.graph_selected = 0;
        } else if self.graph_selected >= total_lines {
            self.graph_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.graph_selected, &self.graph_scroll, total_lines, visible_height);

        let start = self.graph_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Decompress one extra row so merge/branch connectors can see the previous snapshot.
        let mut buffer = self.buffer.borrow_mut();
        buffer.decompress(start, end + 1);

        // An unborn repository has no graph data, so render a centered empty state.
        let head_oid = match repo.head().ok().and_then(|h| h.target()) {
            Some(oid) => oid,
            None => {
                let outer_block = Block::default().borders(Borders::LEFT | Borders::RIGHT).border_style(Style::default().fg(self.theme.COLOR_BORDER));

                frame.render_widget(outer_block, self.layout.graph);

                let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Length(3), Constraint::Percentage(50)]).split(self.layout.graph);

                let message = Paragraph::new("⊘ no commits").alignment(Alignment::Center).style(Style::default().fg(self.theme.COLOR_BORDER));

                frame.render_widget(message, chunks[1]);
                return;
            },
        };

        let head_oid_alias = self.oids.get_alias_by_oid(head_oid);

        // Keep the raw buffer renderer nearby for debugging graph topology.
        let _buffer_range = render_buffer_range(&self.theme, &self.oids, &buffer.history, start + 1, end + 1);

        // SHA, graph, and message columns are rendered independently, then joined as rows.
        let sha_range = if self.layout_config.is_shas { Some(render_sha_range(&self.theme, &self.oids, start, end)) } else { None };

        let graph_range = render_graph_range(&self.theme, &self.oids, &self.branches.all, &self.worktrees, &buffer.history, head_oid_alias, start, end);

        let message_range = render_message_range(
            &self.theme,
            repo,
            &self.oids,
            &self.branches.local,
            &self.branches.all,
            &self.branches.visible_branch_names,
            &self.tags.local,
            &self.worktrees,
            &self.reflogs,
            &mut self.branches.colors,
            &mut self.tags.colors,
            &mut self.stashes.colors,
            self.layout_config.is_graph_reflogs,
            start,
            end,
            self.graph_selected,
            &self.uncommitted,
        );

        // Build table rows and measure the graph column from rendered span widths.
        let mut rows = Vec::with_capacity(end - start + 1);
        let mut width = 0;
        if !graph_range.is_empty() {
            for idx in 0..graph_range.len() {
                width = graph_range.iter().map(|line| line.spans.iter().filter(|span| !span.content.is_empty()).map(|span| span.content.chars().count()).sum::<usize>()).max().unwrap_or(0) as u16;

                let mut cells = Vec::with_capacity(if self.layout_config.is_shas { 3 } else { 2 });

                if let Some(sha) = &sha_range {
                    cells.push(WidgetCell::from(sha.get(idx).cloned().unwrap_or_default()));
                }
                cells.push(WidgetCell::from(graph_range.get(idx).cloned().unwrap_or_default()));
                cells.push(WidgetCell::from(message_range.get(idx).cloned().unwrap_or_default()));

                let mut row = Row::new(cells);

                // Selection highlighting is focus-sensitive so inactive panes stay quiet.
                if idx + start == self.graph_selected && self.focus == Focus::Viewport {
                    row = row.style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)));
                } else if (idx + start).is_multiple_of(2) {
                    row = row.style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)));
                }

                rows.push(row);
            }
        }

        // The graph column is fixed to its measured width; message text gets the rest.
        let constraints = if self.layout_config.is_shas {
            vec![ratatui::layout::Constraint::Length(9), ratatui::layout::Constraint::Length(width + 5), ratatui::layout::Constraint::Min(0)]
        } else {
            vec![ratatui::layout::Constraint::Length(width + 5), ratatui::layout::Constraint::Min(0)]
        };

        if self.layout_config.is_zen {
            // Zen mode owns the full rounded graph frame.
            let table = Table::new(rows, constraints)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_type(ratatui::widgets::BorderType::Rounded))
                .column_spacing(1);

            frame.render_widget(table, self.layout.graph);

            if total_lines > visible_height {
                let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.graph_scroll.get());
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("╮"))
                    .end_symbol(Some("╯"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("▌")
                    .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

                frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);
            }

            return;
        }

        // Normal mode draws only side borders because title and status bars provide the rest.
        let table = Table::new(rows, constraints)
            .block(Block::default().borders(Borders::RIGHT | Borders::LEFT).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_type(ratatui::widgets::BorderType::Rounded))
            .column_spacing(1);

        frame.render_widget(table, self.layout.graph);

        if total_lines > visible_height {
            let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.graph_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(if (self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts)) || self.layout_config.is_status { Some("─") } else { Some("╮") })
                .end_symbol(if (self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts)) || self.layout_config.is_status { Some("─") } else { Some("╯") })
                .track_symbol(Some("│"))
                .thumb_symbol("▌")
                .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);
        }
    }
}
