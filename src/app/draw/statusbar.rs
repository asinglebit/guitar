use crate::{
    app::app::{App, Focus, Viewport},
    git::queries::commits::get_current_branch,
    helpers::{
        branch_visibility::current_branch_names,
        keymap::InputMode,
        localisation::status as status_text,
        symbols::{branch as branch_symbol, graph, submodule, worktree},
    },
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
        Some(format!("{} {} ", submodule::DEFAULT, parts.join(submodule::STACK_SEPARATOR)))
    }

    pub fn draw_statusbar(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        let mut left_spans: Vec<Span> = match self.worktrees.current_name() {
            Some(name) => vec![Span::styled(format!("  {} {name} ", worktree::CURRENT), Style::default().fg(self.theme.COLOR_GRASS))],
            None => vec![Span::raw("  ")],
        };
        if let Some(label) = self.submodule_stack_status_label() {
            left_spans.push(Span::styled(label, Style::default().fg(self.theme.COLOR_TEAL)));
        }
        match get_current_branch(repo) {
            Some(branch) => left_spans.push(Span::styled(format!("{} {}", branch_symbol::LOCAL_VISIBLE, branch), Style::default().fg(self.theme.COLOR_GRASS))),
            None => match repo.head().ok().and_then(|h| h.target()) {
                Some(oid) => left_spans.push(Span::styled(format!("{} #{:.6}", status_text::DETACHED_HEAD, oid), Style::default().fg(self.theme.COLOR_TEXT))),
                None => left_spans.push(Span::styled(status_text::NO_HEAD_NO_COMMITS, Style::default().fg(self.theme.COLOR_TEXT))),
            },
        }
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
            Focus::Branches => self.graph.branches_window.as_ref().map(|window| window.total).unwrap_or_else(|| current_branch_names(repo).len()),
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
                Focus::Branches => {
                    let branch_names = current_branch_names(repo);
                    let hidden = branch_names.iter().filter(|branch| self.branches.hidden_branch_names.contains(*branch)).count();
                    branch_names.len().saturating_sub(hidden)
                },
                Focus::Tags => self.tags_selected + 1,
                Focus::Stashes => self.stashes_selected + 1,
                Focus::Reflogs => self.reflogs_selected + 1,
                Focus::Worktrees => self.worktrees_selected + 1,
                Focus::Submodules => self.submodules_selected + 1,
                Focus::Search => self.search_selected + 1,
                _ => 0,
            }
        };

        let icon_spinner = if self.spinner.is_running() { format!("{} ", self.spinner.get_char()) } else { "".to_string() };

        // Action mode indicator (moved here)
        let mut action_hint = if self.mode == InputMode::Action { vec![Span::styled(format!("{} ", graph::COMMIT_BRANCH), Style::default().fg(self.theme.COLOR_GRAPEFRUIT))] } else { Vec::new() };

        // Zen mode indicator
        if self.layout_config.is_zen {
            action_hint.push(Span::styled(format!("{} ", graph::COMMIT_BRANCH), Style::default().fg(self.theme.COLOR_GRASS)));
        }

        let mut right_spans = vec![Span::styled(if total == 0 { "".to_string() } else { format!("{}/{}{} ", cursor, total, icon_spinner) }, Style::default().fg(self.theme.COLOR_TEXT))];

        right_spans.extend(action_hint);

        let title_paragraph = ratatui::widgets::Paragraph::new(Text::from(Line::from(right_spans))).right_aligned().block(Block::default());

        frame.render_widget(title_paragraph, self.layout.statusbar_right);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/statusbar.rs"]
mod tests;
