use crate::{
    app::app::{App, Focus, Viewport},
    helpers::{keymap::InputMode, localisation::status as status_text},
};
use ratatui::Frame;
use ratatui::{
    style::Style,
    text::{Line, Span, Text},
    widgets::Block,
};

impl App {
    fn submodule_stack_status_label(&self) -> Option<String> {
        let first = self.submodule_stack.first()?;
        let root = first.parent_path.file_name().and_then(|value| value.to_str()).map(str::to_string).unwrap_or_else(|| first.parent_path.display().to_string());
        let mut parts = vec![root];
        parts.extend(self.submodule_stack.iter().map(|entry| entry.submodule_path.display().to_string()));
        Some(format!("{} {} ", self.symbols.submodule.default, parts.join(&self.symbols.submodule.stack_separator)))
    }

    fn head_status_label(&self) -> Span<'static> {
        let text_style = Style::default().fg(self.theme.COLOR_TEXT);
        self.worktrees
            .entries
            .iter()
            .find(|entry| entry.is_current)
            .map(|current| match (current.branch.as_deref(), current.head) {
                (Some(branch), _) => Span::styled(format!("{} {}", self.symbols.branch.local_visible, branch), Style::default().fg(self.theme.COLOR_GRASS)),
                (None, Some(oid)) => Span::styled(format!("{} #{:.6}", status_text::DETACHED_HEAD(), oid), text_style),
                (None, None) => Span::styled(status_text::NO_HEAD_NO_COMMITS(), text_style),
            })
            .unwrap_or_else(|| Span::styled(status_text::NO_HEAD_NO_COMMITS(), text_style))
    }

    fn statusbar_branch_total(&self) -> usize {
        self.graph.branches_window.as_ref().map_or_else(|| self.branches.sorted.iter().filter(|(_, branch)| !self.branches.hidden_branch_names.contains(branch)).count(), |window| window.total)
    }

    pub fn draw_statusbar(&mut self, frame: &mut Frame) {
        let mut left_spans: Vec<Span> = match self.worktrees.current_name() {
            Some(name) => vec![Span::styled(format!("  {} {name} ", self.symbols.worktree.current), Style::default().fg(self.theme.COLOR_GRASS))],
            None => vec![Span::raw("  ")],
        };
        if let Some(label) = self.submodule_stack_status_label() {
            left_spans.push(Span::styled(label, Style::default().fg(self.theme.COLOR_TEAL)));
        }
        left_spans.push(self.head_status_label());
        let lines = Line::from(left_spans);

        let status_paragraph = ratatui::widgets::Paragraph::new(Text::from(lines)).left_aligned().block(Block::default());

        frame.render_widget(status_paragraph, self.layout.statusbar_left);

        let total = match self.focus {
            Focus::Viewport => match self.viewport {
                Viewport::Graph => self.graph_commit_count(),
                Viewport::Viewer => self.viewer_row_count(),
                _ => 0,
            },
            Focus::StatusTop => {
                if self.graph_selected == 0 {
                    self.uncommitted.conflicts.len() + self.uncommitted.staged.modified.len() + self.uncommitted.staged.added.len() + self.uncommitted.staged.deleted.len()
                } else {
                    self.current_diff.len()
                }
            },
            Focus::StatusBottom => self.uncommitted.conflicts.len() + self.uncommitted.unstaged.modified.len() + self.uncommitted.unstaged.added.len() + self.uncommitted.unstaged.deleted.len(),
            Focus::Branches => self.statusbar_branch_total(),
            Focus::Tags => self.graph.tags_window.as_ref().map(|window| window.total).unwrap_or(self.tags.sorted.len()),
            Focus::Stashes => self.graph.stashes_window.as_ref().map(|window| window.total).unwrap_or(self.oids.stashes.len()),
            Focus::Reflogs => self.graph.reflogs_window.as_ref().map(|window| window.total).unwrap_or(self.reflogs.entries.len()),
            Focus::Worktrees => self.worktrees.entries.len(),
            Focus::Submodules => self.submodules.entries.len(),
            Focus::Search => self.search_rows.len(),
            _ => 0,
        };

        let cursor = if total == 0 {
            0
        } else {
            match self.focus {
                Focus::Viewport => match self.viewport {
                    Viewport::Graph => self.graph_selected + 1,
                    Viewport::Viewer => self.viewer_selected + 1,
                    _ => 0,
                },
                Focus::StatusTop => self.status_top_selected + 1,
                Focus::StatusBottom => self.status_bottom_selected + 1,
                Focus::Branches => (self.branches_selected + 1).min(total),
                Focus::Tags => self.tags_selected + 1,
                Focus::Stashes => self.stashes_selected + 1,
                Focus::Reflogs => self.reflogs_selected + 1,
                Focus::Worktrees => self.worktrees_selected + 1,
                Focus::Submodules => self.submodules_selected + 1,
                Focus::Search => self.search_selected + 1,
                _ => 0,
            }
        };

        let count_hint = (total != 0).then(|| {
            let text = if self.spinner.is_running() { format!("{}/{}{} ", cursor, total, self.spinner.get_char()) } else { format!("{}/{} ", cursor, total) };
            Span::styled(text, Style::default().fg(self.theme.COLOR_TEXT))
        });

        let action_hint = [
            (self.mode == InputMode::Action).then(|| Span::styled(format!("{} ", self.symbols.graph.commit_branch), Style::default().fg(self.theme.COLOR_GRAPEFRUIT))),
            self.layout_config.is_zen.then(|| Span::styled(format!("{} ", self.symbols.graph.commit_branch), Style::default().fg(self.theme.COLOR_GRASS))),
        ];
        let right_spans: Vec<_> = count_hint.into_iter().chain(action_hint.into_iter().flatten()).collect();

        let title_paragraph = ratatui::widgets::Paragraph::new(Text::from(Line::from(right_spans))).right_aligned().block(Block::default());

        frame.render_widget(title_paragraph, self.layout.statusbar_right);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/statusbar.rs"]
mod tests;
