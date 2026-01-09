use crate::app::app::{App, Focus};
use crate::core::renderers::{render_buffer_range, render_graph_range, render_message_range, render_sha_range};
use ratatui::{
    Frame,
    style::Style,
    widgets::{Block, Borders, Cell as WidgetCell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};

impl App {
    pub fn draw_graph(&mut self, frame: &mut Frame) {
        // Get vertical dimensions
        let total_lines = self.oids.get_commit_count();
        let visible_height = self.layout.graph.height as usize;

        // Clamp selection
        if total_lines == 0 {
            self.graph_selected = 0;
        } else if self.graph_selected >= total_lines {
            self.graph_selected = total_lines - 1;
        }

        // Trap selection
        self.trap_selection(self.graph_selected, &self.graph_scroll, total_lines, visible_height);

        // Calculate scroll
        let start = self.graph_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // History
        let mut buffer = self.buffer.borrow_mut();
        buffer.decompress(start, end + 1);

        // Get head
        let head_oid = self.repo.head().unwrap().target().unwrap();
        let head_oid_alias = self.oids.get_alias_by_oid(head_oid);

        // Rendered lines
        let _buffer_range = render_buffer_range(&self.theme, &self.oids, &buffer.history, start + 1, end + 1);

        // Shas
        let sha_range = if self.layout_config.is_shas { Some(render_sha_range(&self.theme, &self.oids, start, end)) } else { None };

        // Graph rendering
        let graph_range = render_graph_range(&self.theme, &self.oids, &self.branches.all, &buffer.history, head_oid_alias, start, end);

        // Messages and metadata
        let message_range = render_message_range(
            &self.theme,
            &self.repo,
            &self.oids,
            &self.branches.local,
            &self.branches.visible,
            &self.tags.local,
            &mut self.branches.colors,
            &mut self.tags.colors,
            &mut self.stashes.colors,
            start,
            end,
            self.graph_selected,
            &self.uncommitted,
        );

        // Add rows
        let mut rows = Vec::with_capacity(end - start + 1);
        let mut width = 0;
        if !graph_range.is_empty() {
            for idx in 0..graph_range.len() {
                // Find the maximum width of the graph range
                width = graph_range
                    .iter()
                    .map(|line| {
                        line.spans
                            .iter()
                            .filter(|span| !span.content.is_empty()) // Only non-empty spans
                            .map(|span| span.content.chars().count()) // Use chars() for wide characters
                            .sum::<usize>()
                    })
                    .max()
                    .unwrap_or(0) as u16;

                // Create cells vector
                let mut cells = Vec::with_capacity(if self.layout_config.is_shas { 3 } else { 2 });

                // Fill the vector with cells
                if let Some(sha) = &sha_range {
                    cells.push(WidgetCell::from(sha.get(idx).cloned().unwrap_or_default()));
                }
                cells.push(WidgetCell::from(graph_range.get(idx).cloned().unwrap_or_default()));
                cells.push(WidgetCell::from(message_range.get(idx).cloned().unwrap_or_default()));

                // Assemble the row
                let mut row = Row::new(cells);

                // Change the row background if selected
                if idx + start == self.graph_selected && self.focus == Focus::Viewport {
                    row = row.style(Style::default().bg(self.theme.COLOR_GREY_800));
                } else if (idx + start).is_multiple_of(2) {
                    row = row.style(Style::default().bg(self.theme.COLOR_GREY_900));
                }

                // Save out
                rows.push(row);
            }
        }

        // Conditional constraints
        let constraints = if self.layout_config.is_shas {
            vec![ratatui::layout::Constraint::Length(9), ratatui::layout::Constraint::Length(width + 5), ratatui::layout::Constraint::Min(0)]
        } else {
            vec![ratatui::layout::Constraint::Length(width + 5), ratatui::layout::Constraint::Min(0)]
        };

        // Setup the table
        let table = Table::new(rows, constraints)
            .block(Block::default().borders(Borders::RIGHT | Borders::LEFT).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_type(ratatui::widgets::BorderType::Rounded))
            .column_spacing(1);

        // Render the table
        frame.render_widget(table, self.layout.graph);

        // Setup the scrollbar
        if total_lines > visible_height {
            let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.graph_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(if (self.layout_config.is_inspector && self.graph_selected != 0) || self.layout_config.is_status { Some("─") } else { Some("╮") })
                .end_symbol(if (self.layout_config.is_inspector && self.graph_selected != 0) || self.layout_config.is_status { Some("─") } else { Some("╯") })
                .track_symbol(Some("│"))
                .thumb_symbol("▌")
                .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            // Render the scrollbar
            frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);
        }
    }
}
