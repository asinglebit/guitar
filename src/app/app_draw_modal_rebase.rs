use crate::{
    app::app::{App, Focus},
    helpers::text::wrap_words,
};
use ratatui::{
    Frame,
    layout::Alignment,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_rebase(&mut self, frame: &mut Frame) {
        let (title, hint) = match self.focus {
            Focus::ModalRebaseProgress => ("rebase", "working..."),
            Focus::ModalRebaseConflict => ("rebase conflict", "resolve conflicts in your editor, then press (Enter)"),
            Focus::ModalRebaseSuccess => ("rebase complete", "press (Enter)"),
            _ => ("rebase", "press Enter"),
        };

        let max_modal_width = (frame.area().width as f32 * 0.8) as usize;
        let text_width = max_modal_width.saturating_sub(10).clamp(1, 70);
        let wrapped_message = wrap_words(self.modal_rebase_message.clone(), text_width);
        let mut lines = Vec::new();
        lines.push(Line::default());
        lines.push(Line::default());
        for line in wrapped_message {
            lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT))));
        }
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(hint, Style::default().fg(self.theme.COLOR_GREY_500))));

        let content_width = lines.iter().map(|line| line.width()).max().unwrap_or(0);
        let modal_width = (content_width + 10).max(34).min(max_modal_width) as u16;
        let max_modal_height = (frame.area().height as f32 * 0.6) as usize;
        let modal_height = (lines.len() + 4).max(8).min(max_modal_height.max(1)) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = ratatui::layout::Rect::new(x, y, modal_width, modal_height);

        frame.render_widget(Clear, modal_area);

        let border_color = if self.focus == Focus::ModalRebaseConflict { self.theme.COLOR_ORANGE } else { self.theme.COLOR_BORDER };
        let modal_block = Block::default()
            .title(Span::styled(format!(" {title} "), Style::default().fg(if self.focus == Focus::ModalRebaseConflict { self.theme.COLOR_ORANGE } else { self.theme.COLOR_TEXT })))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());
    }
}
