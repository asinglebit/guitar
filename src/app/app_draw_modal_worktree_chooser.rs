use crate::{
    app::app::{App, WorktreeModalAction},
    helpers::{
        symbols::{SYM_COMMIT_BRANCH, SYM_WORKTREE, SYM_WORKTREE_DIRTY, SYM_WORKTREE_INVALID, SYM_WORKTREE_LOCKED, SYM_WORKTREE_OTHER},
        text::truncate_with_ellipsis,
    },
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_worktree_chooser(&mut self, frame: &mut Frame) {
        let title = match self.modal_worktree_action {
            WorktreeModalAction::Open => "select a worktree to open",
            WorktreeModalAction::Remove => "select a worktree to remove",
        };

        let mut length = title.len().max(34);
        let mut height = 6;
        let mut lines = vec![Line::default(), Line::from(Span::styled(title, Style::default().fg(self.theme.COLOR_TEXT))), Line::default()];
        let max_line_width = (frame.area().width as usize).saturating_sub(16).max(24);

        for (idx, entry_idx) in self.modal_worktree_candidates.iter().enumerate() {
            let Some(entry) = self.worktrees.entries.get(*entry_idx) else {
                continue;
            };

            let target =
                entry.branch.as_ref().map(|branch| format!("{SYM_COMMIT_BRANCH} {branch}")).or_else(|| entry.head.map(|oid| format!("detached #{:.6}", oid))).unwrap_or_else(|| "no head".to_string());
            let dirty = if entry.is_dirty { format!(" {SYM_WORKTREE_DIRTY}") } else { String::new() };
            let locked = if entry.locked_reason.is_some() { format!(" {SYM_WORKTREE_LOCKED}") } else { String::new() };
            let invalid = if !entry.is_valid { format!(" {SYM_WORKTREE_INVALID}") } else { String::new() };
            let icon = if entry.is_current { SYM_WORKTREE } else { SYM_WORKTREE_OTHER };
            let label = format!("{icon} {}  {}{}{}{}  {}", entry.name, target, dirty, locked, invalid, entry.path.display());
            let label = truncate_with_ellipsis(&label, max_line_width);
            length = length.max(label.len());
            height += 1;

            let color = if !entry.is_valid {
                self.theme.COLOR_GREY_800
            } else if idx == self.modal_worktree_selected as usize {
                self.theme.COLOR_TEAL
            } else if entry.is_current {
                self.theme.COLOR_GRASS
            } else if entry.locked_reason.is_some() {
                self.theme.COLOR_GREY_600
            } else {
                self.theme.COLOR_TEXT
            };

            lines.push(Line::from(Span::styled(label, Style::default().fg(color))));
        }

        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        let modal_width = (length + 8).min((frame.area().width as f32 * 0.85) as usize) as u16;
        let modal_height = height.min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let padding = ratatui::widgets::Padding { left: 3, right: 3, top: 1, bottom: 1 };
        let modal_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.COLOR_GREY_600))
            .title(Span::styled(" (esc) ", Style::default().fg(self.theme.COLOR_GREY_500)))
            .title_alignment(Alignment::Right)
            .padding(padding)
            .border_type(ratatui::widgets::BorderType::Rounded);

        let paragraph = Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center);
        paragraph.render(modal_area, frame.buffer_mut());
    }
}
