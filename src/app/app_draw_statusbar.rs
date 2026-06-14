use crate::{
    app::app::{App, Focus, Viewport},
    git::queries::commits::get_current_branch,
    helpers::keymap::InputMode,
};
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span, Text},
    widgets::Block,
};

impl App {
    pub fn draw_statusbar(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        let lines = match get_current_branch(repo) {
            Some(branch) => Line::from(vec![Span::styled(format!("  ● {}", branch), Style::default().fg(self.theme.COLOR_GRASS))]),
            None => match repo.head().ok().and_then(|h| h.target()) {
                Some(oid) => Line::from(vec![Span::styled(format!("  detached head: #{:.6}", oid), Style::default().fg(self.theme.COLOR_TEXT))]),
                None => Line::from(vec![Span::styled("  no head (no commits yet)", Style::default().fg(self.theme.COLOR_TEXT))]),
            },
        };

        let status_paragraph = ratatui::widgets::Paragraph::new(Text::from(lines)).left_aligned().block(Block::default());

        frame.render_widget(status_paragraph, self.layout.statusbar_left);

        let total = match self.focus {
            Focus::Viewport => match self.viewport {
                Viewport::Graph => self.oids.get_commit_count(),
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
            Focus::Branches => self.branches.sorted.len(),
            Focus::Worktrees => self.worktrees.entries.len(),
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
                    if self.branches.visible_branch_names.is_empty() {
                        self.branches.all.len()
                    } else {
                        self.branches.visible_branch_names.len()
                    }
                },
                Focus::Worktrees => self.worktrees_selected + 1,
                _ => 0,
            }
        };

        let icon_spinner = if self.spinner.is_running() { format!("{} ", self.spinner.get_char()) } else { "".to_string() };

        // Action mode indicator (moved here)
        let mut action_hint = if self.mode == InputMode::Action { vec![Span::styled("● ", Style::default().fg(self.theme.COLOR_GRAPEFRUIT))] } else { Vec::new() };

        // Zen mode indicator
        if self.layout_config.is_zen {
            action_hint.push(Span::styled("● ", Style::default().fg(self.theme.COLOR_GRASS)));
        }

        let mut right_spans = vec![Span::styled(if total == 0 { "".to_string() } else { format!("{}/{}{} ", cursor, total, icon_spinner) }, Style::default().fg(self.theme.COLOR_TEXT))];

        right_spans.extend(action_hint);

        let title_paragraph = ratatui::widgets::Paragraph::new(Text::from(Line::from(right_spans))).right_aligned().block(Block::default());

        frame.render_widget(title_paragraph, self.layout.statusbar_right);
    }
}
