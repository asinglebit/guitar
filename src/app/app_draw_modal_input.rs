use crate::app::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_input(&mut self, frame: &mut Frame, title: &str) {
        // Fixed content dimensions are later clamped against the terminal size.
        let length = 60;
        let height = 13;
        let fill = 7;

        // Text content reserves vertical space for the input field drawn below it.
        let mut lines: Vec<Line> = Vec::with_capacity(fill + 2);

        lines.push(Line::from(Span::styled(title, Style::default().fg(self.theme.COLOR_TEXT))));

        lines.extend(vec![Line::default(); fill]);

        lines.push(Line::from(Span::styled("(enter)".to_string(), Style::default().fg(self.theme.COLOR_GREY_500))));

        // Paint a plain overlay before clearing the modal rectangle.
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        // Keep the modal centered and no larger than most of the current frame.
        let modal_width = length.min((frame.area().width as f32 * 0.8) as usize) as u16;
        let modal_height = height.min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);

        self.theme.clear_area(modal_area, frame.buffer_mut());

        // The title on the right doubles as the close affordance.
        let modal_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.COLOR_GREY_600))
            .title(Span::styled(" (esc) ", Style::default().fg(self.theme.COLOR_GREY_500)))
            .title_alignment(Alignment::Right)
            .padding(Padding { left: 3, right: 3, top: 1, bottom: 1 })
            .border_type(ratatui::widgets::BorderType::Rounded);

        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());

        // The input area is fixed-width inside the modal to keep cursor math stable.
        let input_area = Rect { x: modal_area.x + (modal_area.width / 2).saturating_sub(29), y: modal_area.y + 4, width: 58, height: 5 };

        // TextInput owns horizontal scroll; the modal slices only the visible text.
        let visible_width = input_area.width.saturating_sub(1) as usize;
        self.modal_input.set_max_width(visible_width);
        let start: usize = *self.modal_input.scroll();
        let end: usize = (start + visible_width).min(self.modal_input.value().len());
        let visible_text = &self.modal_input.value()[start..end];

        // Cursor position is relative to the scrolled input viewport.
        let cursor_x = (self.modal_input.cursor() - self.modal_input.scroll()) as u16 + 1;

        // The top divider visually anchors the input value.
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(visible_text, Style::default().fg(self.theme.COLOR_TEXT))))
                .block(Block::default().padding(ratatui::widgets::Padding { left: 1, right: 1, top: 1, bottom: 0 }).borders(Borders::TOP).border_style(Style::default().fg(self.theme.COLOR_GREY_800))),
            input_area,
        );

        frame.set_cursor_position((input_area.x + cursor_x, input_area.y + 2));

        // The bottom divider balances the input control inside the modal.
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(self.theme.COLOR_GREY_800))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .render(Rect { x: modal_area.x + 1, y: modal_area.y + 8, width: modal_width.saturating_sub(2), height: 1 }, frame.buffer_mut());
    }
}
