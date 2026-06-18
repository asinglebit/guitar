use crate::{
    app::{
        app::App,
        draw::modals::shared::{action_row, modal_block},
    },
    helpers::{localisation::modal, text::wrap_words},
};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_error(&mut self, frame: &mut Frame) {
        // Error text wraps to a readable width and then drives modal size.
        let max_modal_width = (frame.area().width as f32 * 0.8) as usize;
        let text_width = max_modal_width.saturating_sub(10).clamp(1, 70);
        let wrapped_message = wrap_words(self.modal_error_message.clone(), text_width);

        let mut lines = Vec::new();
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(modal::ERROR_TITLE, Style::default().fg(self.theme.COLOR_RED))));
        lines.push(Line::default());
        lines.extend(wrapped_message.into_iter().map(|line| Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_RED)))));
        lines.push(Line::default());
        lines.push(action_row(&[(modal::ACTION_OK, modal::KEY_ENTER)], Style::default().fg(self.theme.COLOR_RED)));

        let content_width = lines.iter().map(|line| line.width()).max().unwrap_or(30);
        let modal_width = (content_width + 10).max(30).min(max_modal_width) as u16;
        let max_modal_height = (frame.area().height as f32 * 0.6) as usize;
        let modal_height = (lines.len() + 4).max(8).min(max_modal_height.max(1)) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);

        // Paint a plain overlay before clearing the modal rectangle.
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = modal_block(self.theme.COLOR_RED, self.theme.COLOR_RED);

        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());
    }
}
