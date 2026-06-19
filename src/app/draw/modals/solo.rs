use crate::app::{
    app::{App, BranchModalAction},
    draw::modals::shared::modal_block,
};
use crate::helpers::localisation::modal;
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_solo(&mut self, frame: &mut Frame) {
        let mut length = 30;
        let mut height = 8;
        let Some(alias) = self.graph_alias_at(self.graph_selected) else {
            return;
        };
        let mut lines = Vec::new();
        let line_text = match self.modal_branch_action {
            BranchModalAction::Solo => modal::SELECT_BRANCH_SOLO,
            BranchModalAction::Toggle => modal::SELECT_BRANCH_TOGGLE,
            BranchModalAction::Rename => modal::SELECT_BRANCH_RENAME,
        };
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(line_text, Style::default().fg(self.theme.COLOR_TEXT))]));
        lines.push(Line::default());

        // Modal choices mirror the branches currently selectable from the graph row.
        let branches = self.modal_branch_action_choices(alias);

        branches.iter().enumerate().for_each(|(idx, branch)| {
            height += 1;
            let is_selected = idx == self.modal_solo_selected as usize;
            let is_local = self.branches.local.values().any(|branches| branches.iter().any(|b| b.as_str() == branch));
            length = (10 + branch.len()).max(length);
            let style = Style::default().fg(if is_selected { self.theme.COLOR_GRASS } else { self.theme.COLOR_TEXT });
            let marker = if is_local { &self.symbols.branch.local_visible } else { &self.symbols.branch.remote_visible };
            lines.push(Line::from(Span::styled(format!("{marker} {branch} "), style)));
        });

        // Paint a plain overlay before clearing the modal rectangle.
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        // The modal grows with branch names but is capped to the terminal size.
        let modal_width = length.min((frame.area().width as f32 * 0.8) as usize) as u16;
        let modal_height = height.min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = modal_block(self.theme.COLOR_GREY_600, self.theme.COLOR_HIGHLIGHTED, &self.symbols);

        let paragraph = Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center);

        paragraph.render(modal_area, frame.buffer_mut());
    }
}

#[cfg(test)]
#[path = "../../../tests/app/draw/modals/solo.rs"]
mod tests;
