use crate::{
    app::{app::App, draw::modals::shared::modal_block},
    git::queries::commits::get_current_branch,
    helpers::{localisation::modal, symbols::branch as branch_symbol},
};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_delete_branch(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        let mut length = 30;
        let mut height = 8;
        let Some(alias) = self.graph_alias_at(self.graph_selected) else {
            return;
        };
        let mut lines = Vec::new();
        let line_text = modal::SELECT_BRANCH_DELETE;
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(line_text, Style::default().fg(self.theme.COLOR_TEXT))]));
        lines.push(Line::default());

        // The current branch is omitted because deleting it would invalidate HEAD.
        let current = get_current_branch(repo);
        let branches = self.graph_deletable_branch_choices(alias, current.as_deref());

        branches.iter().enumerate().for_each(|(idx, branch)| {
            height += 1;
            let is_selected = idx == self.modal_delete_branch_selected as usize;
            let is_local = self.branches.local.values().any(|branches| branches.iter().any(|b| b.as_str() == branch));

            let marker = if is_local { branch_symbol::LOCAL_VISIBLE } else { branch_symbol::REMOTE_VISIBLE };
            let line_text = format!("{marker} {branch} ");
            length = length.max(line_text.len());

            let style = Style::default().fg(if is_selected { self.theme.COLOR_GRASS } else { self.theme.COLOR_TEXT });

            lines.push(Line::from(Span::styled(line_text, style)));
        });

        // Paint a plain overlay before clearing the modal rectangle.
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        // The modal grows with branch names but is capped to the terminal size.
        length += 10;
        let modal_width = length.min((frame.area().width as f32 * 0.8) as usize) as u16;
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
