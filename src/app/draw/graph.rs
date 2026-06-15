use crate::app::{
    app::{App, Focus},
    draw::buffered::{DrawTarget, SurfaceRender},
};
use crate::core::renderers::{render_graph_projection, render_message_projection, render_sha_projection};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::{
    style::Style,
    widgets::{Block, Borders, Cell as WidgetCell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};

impl App {
    pub fn draw_graph(&mut self, frame: &mut impl DrawTarget, repo: &git2::Repository) -> SurfaceRender {
        if self.layout.graph.width == 0 || self.layout.graph.height == 0 {
            return SurfaceRender::Ready;
        }

        // Determine the visible graph window before requesting projected rows.
        let total_lines = self.graph_commit_count();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };

        let previous_selected = self.graph_selected;
        if total_lines == 0 {
            self.graph_selected = 0;
        } else if self.graph_selected >= total_lines {
            self.graph_selected = total_lines.saturating_sub(1);
        }
        if self.graph_selected != previous_selected {
            self.current_diff.clear();
            self.current_diff_identity = None;
            if self.graph_selected != 0
                && let Some(identity) = self.graph_identity_at(self.graph_selected)
            {
                self.current_diff = crate::git::queries::diffs::get_filenames_diff_at_oid(repo, identity.oid);
                self.current_diff_identity = Some(identity);
            }
        }

        self.trap_selection(self.graph_selected, &self.graph_scroll, total_lines, visible_height);

        let start = self.graph_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        let (preload_start, preload_end) = graph_preload_window(start, end, total_lines, visible_height);
        self.request_graph_window(preload_start, preload_end);

        // An unborn repository has no graph data, so render a centered empty state.
        match repo.head().ok().and_then(|h| h.target()) {
            Some(_) => {},
            None => {
                let table = Table::new(graph_backdrop_rows(visible_height, 0, None, &self.theme), [ratatui::layout::Constraint::Min(0)])
                    .block(Block::default().borders(Borders::LEFT | Borders::RIGHT).border_style(Style::default().fg(self.theme.COLOR_BORDER)))
                    .column_spacing(0);

                frame.render_widget(table, self.layout.graph);

                let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Length(3), Constraint::Percentage(50)]).split(self.layout.graph);

                let message = Paragraph::new("⊘ no commits").alignment(Alignment::Center).style(Style::default().fg(self.theme.COLOR_BORDER));

                frame.render_widget(message, chunks[1]);
                return SurfaceRender::Ready;
            },
        }

        let visible_len = end.saturating_sub(start);
        let (sha_range, graph_range, message_range) = if let Some(window) = self.graph.graph_window.as_ref().filter(|window| window.start < end && start < window.end) {
            // SHA, graph, and message columns are rendered from the cached window, then reindexed
            // into the requested viewport so scrolling still looks like movement while loading.
            let render_uncommitted_row = graph_window_has_stable_visible_page(window, start, end);
            let source_sha = if self.layout_config.is_shas { Some(render_sha_projection(&self.theme, &window.rows, self.graph_selected)) } else { None };
            let source_graph = render_graph_projection(&self.theme, &window.rows, &window.history, window.head_alias, window.start, window.end, render_uncommitted_row);
            let source_message = render_message_projection(&self.theme, &window.rows, self.layout_config.is_graph_reflogs, self.graph_selected, &self.uncommitted, render_uncommitted_row);

            (
                source_sha.as_ref().map(|lines| align_projection(lines, window.start, start, end)),
                align_projection(&source_graph, window.start, start, end),
                align_projection(&source_message, window.start, start, end),
            )
        } else {
            (self.layout_config.is_shas.then(|| blank_projection(visible_len)), blank_projection(visible_len), blank_projection(visible_len))
        };

        // Build table rows and measure the graph column from rendered span widths.
        let mut rows = Vec::with_capacity(visible_height);
        let width = graph_range.iter().map(|line| line.spans.iter().filter(|span| !span.content.is_empty()).map(|span| span.content.chars().count()).sum::<usize>()).max().unwrap_or(0) as u16;
        for idx in 0..visible_height {
            let mut cells = Vec::with_capacity(if self.layout_config.is_shas { 3 } else { 2 });

            if let Some(sha) = &sha_range {
                cells.push(WidgetCell::from(sha.get(idx).cloned().unwrap_or_default()));
            }
            cells.push(WidgetCell::from(graph_range.get(idx).cloned().unwrap_or_default()));
            cells.push(WidgetCell::from(message_range.get(idx).cloned().unwrap_or_default()));

            let mut row = Row::new(cells);

            // Selection highlighting is focus-sensitive so inactive panes stay quiet.
            if idx < visible_len && idx + start == self.graph_selected && self.focus == Focus::Viewport {
                row = row.style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_800)));
            } else if (idx + start).is_multiple_of(2) {
                row = row.style(Style::default().bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)));
            }

            rows.push(row);
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

            return SurfaceRender::Ready;
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
        SurfaceRender::Ready
    }
}

fn blank_projection(len: usize) -> Vec<Line<'static>> {
    vec![Line::default(); len]
}

fn graph_backdrop_rows<'a>(visible_height: usize, start: usize, selected: Option<usize>, theme: &crate::helpers::palette::Theme) -> Vec<Row<'a>> {
    (0..visible_height)
        .map(|idx| {
            let global_idx = start + idx;
            let mut row = Row::new([WidgetCell::from(Line::default())]);
            if selected == Some(global_idx) {
                row = row.style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_800)));
            } else if global_idx.is_multiple_of(2) {
                row = row.style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_900)));
            }
            row
        })
        .collect()
}

fn graph_window_has_stable_visible_page(window: &crate::app::app::GraphWindowCache, target_start: usize, target_end: usize) -> bool {
    let cached_len = window.end.saturating_sub(window.start);
    window.start <= target_start && target_end <= window.end && window.rows.len() >= cached_len && window.history.len() >= cached_len
}

fn graph_preload_window(start: usize, end: usize, total_lines: usize, visible_height: usize) -> (usize, usize) {
    if visible_height == 0 {
        return (start, end);
    }

    (start.saturating_sub(visible_height), end.saturating_add(visible_height).min(total_lines))
}

fn align_projection(lines: &[Line<'static>], cached_start: usize, target_start: usize, target_end: usize) -> Vec<Line<'static>> {
    (target_start..target_end)
        .map(|index| {
            if index < cached_start {
                return Line::default();
            }
            lines.get(index - cached_start).cloned().unwrap_or_default()
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/app/draw/graph.rs"]
mod tests;
