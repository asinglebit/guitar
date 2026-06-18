use crate::{
    app::{
        app::{App, WorktreeModalAction},
        draw::modals::shared::modal_block,
    },
    helpers::{
        localisation::{common, modal, status as status_text},
        symbols::{branch as branch_symbol, worktree},
        text::truncate_with_ellipsis,
    },
};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_worktree_chooser(&mut self, frame: &mut Frame) {
        let title = match self.modal_worktree_action {
            WorktreeModalAction::Open => modal::SELECT_WORKTREE_OPEN,
            WorktreeModalAction::Remove => modal::SELECT_WORKTREE_REMOVE,
        };

        let mut length = title.len().max(34);
        let mut height = 6;
        let mut lines = vec![Line::default(), Line::from(Span::styled(title, Style::default().fg(self.theme.COLOR_TEXT))), Line::default()];
        let max_line_width = (frame.area().width as usize).saturating_sub(16).max(24);

        for (idx, entry_idx) in self.modal_worktree_candidates.iter().enumerate() {
            let Some(entry) = self.worktrees.entries.get(*entry_idx) else {
                continue;
            };

            let target = entry
                .branch
                .as_ref()
                .map(|branch| format!("{} {branch}", branch_symbol::LOCAL_VISIBLE))
                .or_else(|| entry.head.map(|oid| format!("{} #{:.6}", status_text::DETACHED, oid)))
                .unwrap_or_else(|| common::NO_HEAD.to_string());
            let dirty = if entry.is_dirty { format!(" {}", worktree::DIRTY) } else { String::new() };
            let locked = if entry.locked_reason.is_some() { format!(" {}", worktree::LOCKED) } else { String::new() };
            let invalid = if !entry.is_valid { format!(" {}", worktree::INVALID) } else { String::new() };
            let icon = if entry.is_current { worktree::CURRENT } else { worktree::OTHER };
            let label = format!("{icon}{}  {}{}{}{}  {}", entry.name, target, dirty, locked, invalid, entry.path.display());
            let label = truncate_with_ellipsis(&label, max_line_width);
            length = length.max(label.len());
            height += 1;
            let is_selected = idx == self.modal_worktree_selected as usize;

            let color = if is_selected {
                self.theme.COLOR_GRASS
            } else if !entry.is_valid {
                self.theme.COLOR_GREY_800
            } else if entry.is_current {
                self.theme.COLOR_GRASS
            } else if entry.locked_reason.is_some() {
                self.theme.COLOR_GREY_600
            } else {
                self.theme.COLOR_TEXT
            };

            let style = Style::default().fg(color);

            lines.push(Line::from(Span::styled(label, style)));
        }

        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        let modal_width = (length + 8).min((frame.area().width as f32 * 0.85) as usize) as u16;
        let modal_height = height.min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = modal_block(self.theme.COLOR_GREY_600, self.theme.COLOR_HIGHLIGHTED);

        let paragraph = Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center);
        paragraph.render(modal_area, frame.buffer_mut());
    }
}
