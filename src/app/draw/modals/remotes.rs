use crate::{
    app::{
        app::App,
        draw::modals::shared::{action_row, modal_block},
        input::remotes::REMOTE_ACTIONS,
    },
    helpers::{localisation::modal, symbols, text::truncate_with_ellipsis},
};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_remote_action(&mut self, frame: &mut Frame) {
        let remote_name = self.modal_remote_target.as_deref().unwrap_or(modal::REMOTE_FALLBACK);
        let mut lines = Vec::new();
        let mut length = 36usize;

        lines.push(Line::default());
        lines.push(Line::from(Span::styled(format!("{} {remote_name}", modal::REMOTE_LABEL), Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::default());

        for (idx, action) in REMOTE_ACTIONS.iter().enumerate() {
            let is_selected = idx == self.modal_remote_selected.rem_euclid(REMOTE_ACTIONS.len() as i32) as usize;
            let marker = if is_selected { symbols::modal::SELECTED } else { symbols::modal::UNSELECTED };
            let text = format!("{} {}", marker, action.label());
            length = length.max(text.len());
            lines.push(Line::from(Span::styled(text, Style::default().fg(if is_selected { self.theme.COLOR_GRASS } else { self.theme.COLOR_TEXT }))));
        }

        lines.push(Line::default());
        lines.push(action_row(&[(modal::ACTION_CONFIRM, modal::KEY_ENTER)], Style::default().fg(self.theme.COLOR_HIGHLIGHTED)));
        self.draw_remote_lines_modal(frame, lines, length);
    }

    pub fn draw_modal_delete_remote(&mut self, frame: &mut Frame) {
        let remote_name = self.modal_remote_target.as_deref().unwrap_or(modal::REMOTE_FALLBACK);
        let mut lines = Vec::new();
        let mut length = 34usize;

        lines.push(Line::default());
        lines.push(Line::from(Span::styled(modal::DELETE_SELECTED_REMOTE, Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::default());

        let name_line = format!("{} {}", modal::NAME_LABEL, truncate_with_ellipsis(remote_name, 60));
        length = length.max(name_line.len());
        lines.push(Line::from(Span::styled(name_line, Style::default().fg(self.theme.COLOR_GRAPEFRUIT))));

        lines.push(Line::default());
        lines.push(action_row(&[(modal::ACTION_CONFIRM, modal::KEY_ENTER)], Style::default().fg(self.theme.COLOR_RED)));
        self.draw_remote_lines_modal(frame, lines, length);
    }

    fn draw_remote_lines_modal(&mut self, frame: &mut Frame, lines: Vec<Line<'_>>, length: usize) {
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        let modal_width = (length + 10).min((frame.area().width as f32 * 0.8) as usize) as u16;
        let modal_height = (lines.len() + 4).min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = modal_block(self.theme.COLOR_GREY_600, self.theme.COLOR_HIGHLIGHTED);
        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());
    }
}
