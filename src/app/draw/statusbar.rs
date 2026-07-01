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
    fn statusbar_left_span_capacity(&self) -> usize {
        2 + self.submodule_stack.first().map_or(0, |_| 4 + self.submodule_stack.len() * 2)
    }

    fn submodule_stack_status_spans<'a>(&'a self) -> impl Iterator<Item = Span<'a>> + 'a {
        self.submodule_stack.first().into_iter().flat_map(move |first| {
            let style = Style::default().fg(self.theme.COLOR_TEAL);
            let root = first.parent_path.file_name().unwrap_or_else(|| first.parent_path.as_os_str()).to_string_lossy();
            [Span::styled(self.symbols.submodule.default.as_str(), style), Span::styled(" ", style), Span::styled(root, style)]
                .into_iter()
                .chain(
                    self.submodule_stack
                        .iter()
                        .flat_map(move |entry| [Span::styled(self.symbols.submodule.stack_separator.as_str(), style), Span::styled(entry.submodule_path.as_os_str().to_string_lossy(), style)]),
                )
                .chain(std::iter::once(Span::styled(" ", style)))
        })
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

    fn statusbar_left_spans<'a>(&'a self) -> impl Iterator<Item = Span<'a>> + 'a {
        let worktree = match self.worktrees.current_name() {
            Some(name) => Span::styled(format!("  {} {name} ", self.symbols.worktree.current), Style::default().fg(self.theme.COLOR_GRASS)),
            None => Span::raw("  "),
        };

        std::iter::once(worktree).chain(self.submodule_stack_status_spans()).chain(std::iter::once(self.head_status_label()))
    }

    fn statusbar_position(&self) -> Option<(usize, usize)> {
        let (cursor, total) = match self.focus {
            Focus::Viewport => match &self.viewport {
                Viewport::Graph => (self.graph_selected + 1, self.graph_commit_count()),
                Viewport::Viewer => (self.viewer_selected + 1, self.viewer_row_count()),
                _ => (0, 0),
            },
            Focus::StatusTop => {
                if self.graph_selected == 0 {
                    (
                        self.status_top_selected + 1,
                        self.uncommitted.conflicts.len() + self.uncommitted.staged.modified.len() + self.uncommitted.staged.added.len() + self.uncommitted.staged.deleted.len(),
                    )
                } else {
                    (self.status_top_selected + 1, self.current_diff.len())
                }
            },
            Focus::StatusBottom => (
                self.status_bottom_selected + 1,
                self.uncommitted.conflicts.len() + self.uncommitted.unstaged.modified.len() + self.uncommitted.unstaged.added.len() + self.uncommitted.unstaged.deleted.len(),
            ),
            Focus::Branches => {
                let total = self.statusbar_branch_total();
                ((self.branches_selected + 1).min(total), total)
            },
            Focus::Tags => (self.tags_selected + 1, self.graph.tags_window.as_ref().map_or(self.tags.sorted.len(), |window| window.total)),
            Focus::Stashes => (self.stashes_selected + 1, self.graph.stashes_window.as_ref().map_or(self.oids.stashes.len(), |window| window.total)),
            Focus::Reflogs => (self.reflogs_selected + 1, self.graph.reflogs_window.as_ref().map_or(self.reflogs.entries.len(), |window| window.total)),
            Focus::Worktrees => (self.worktrees_selected + 1, self.worktrees.entries.len()),
            Focus::Submodules => (self.submodules_selected + 1, self.submodules.entries.len()),
            Focus::Search => (self.search_selected + 1, self.search_rows.len()),
            _ => (0, 0),
        };

        (total != 0).then_some((cursor, total))
    }

    fn statusbar_count_hint<'a>(&self) -> Option<Span<'a>> {
        self.statusbar_position().map(|(cursor, total)| {
            let text = if self.spinner.is_running() { format!("{cursor}/{total}{} ", self.spinner.get_char()) } else { format!("{cursor}/{total} ") };
            Span::styled(text, Style::default().fg(self.theme.COLOR_TEXT))
        })
    }

    fn action_hint_spans<'a>(&'a self) -> impl Iterator<Item = Span<'a>> + 'a {
        [(self.mode == InputMode::Action, self.theme.COLOR_GRAPEFRUIT), (self.layout_config.is_zen, self.theme.COLOR_GRASS)]
            .into_iter()
            .filter_map(|(enabled, color)| enabled.then_some(color))
            .flat_map(|color| [Span::styled(self.symbols.graph.commit_branch.as_str(), Style::default().fg(color)), Span::raw(" ")])
    }

    pub fn draw_statusbar(&mut self, frame: &mut Frame) {
        let mut left_spans = Vec::with_capacity(self.statusbar_left_span_capacity());
        left_spans.extend(self.statusbar_left_spans());

        let status_paragraph = ratatui::widgets::Paragraph::new(Text::from(Line::from(left_spans))).left_aligned().block(Block::default());

        frame.render_widget(status_paragraph, self.layout.statusbar_left);

        let mut right_spans = Vec::with_capacity(5);
        right_spans.extend(self.statusbar_count_hint());
        right_spans.extend(self.action_hint_spans());

        let title_paragraph = ratatui::widgets::Paragraph::new(Text::from(Line::from(right_spans))).right_aligned().block(Block::default());

        frame.render_widget(title_paragraph, self.layout.statusbar_right);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/statusbar.rs"]
mod tests;
