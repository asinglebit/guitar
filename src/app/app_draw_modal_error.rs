use crate::{app::app::App, helpers::text::wrap_words};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_error(&mut self, frame: &mut Frame) {
        // Error text wraps to a readable width and then drives modal size.
        let max_modal_width = (frame.area().width as f32 * 0.8) as usize;
        let text_width = max_modal_width.saturating_sub(10).clamp(1, 70);
        let wrapped_message = wrap_words(self.modal_error_message.clone(), text_width);

        let mut lines = Vec::new();
        lines.push(Line::default());
        lines.push(Line::from(Span::styled("error", Style::default().fg(self.theme.COLOR_RED))));
        lines.push(Line::default());
        lines.extend(wrapped_message.into_iter().map(|line| Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_RED)))));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled("(enter) OK", Style::default().fg(self.theme.COLOR_RED))));

        let content_width = lines.iter().map(|line| line.width()).max().unwrap_or(30);
        let modal_width = (content_width + 10).max(30).min(max_modal_width) as u16;
        let max_modal_height = (frame.area().height as f32 * 0.6) as usize;
        let modal_height = (lines.len() + 4).max(8).min(max_modal_height.max(1)) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);

        // Paint a plain overlay before clearing the modal rectangle.
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.COLOR_RED))
            .title(Span::styled(" (esc) ", Style::default().fg(self.theme.COLOR_RED)))
            .title_alignment(Alignment::Right)
            .padding(ratatui::widgets::Padding { left: 3, right: 3, top: 1, bottom: 1 })
            .border_type(ratatui::widgets::BorderType::Rounded);

        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());
    }
}
