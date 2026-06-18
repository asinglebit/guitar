use crate::{
    app::{
        app::App,
        draw::modals::shared::{action_row, modal_block, render_modal_text_input},
    },
    helpers::localisation::modal,
};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_input(&mut self, frame: &mut Frame, title: &str) {
        // Fixed content dimensions are later clamped against the terminal size.
        let length = 60;
        let height = 13;
        let fill = 5;

        // Text content reserves vertical space for the input field drawn below it.
        let mut lines: Vec<Line> = Vec::with_capacity(fill + 4);

        lines.push(Line::default());
        lines.push(Line::from(Span::styled(title, Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::default());
        lines.extend(vec![Line::default(); fill]);
        lines.push(action_row(&[(modal::ACTION_CONFIRM, modal::KEY_ENTER)], Style::default().fg(self.theme.COLOR_HIGHLIGHTED)));

        // Paint a plain overlay before clearing the modal rectangle.
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        // Keep the modal centered and no larger than most of the current frame.
        let modal_width = length.min((frame.area().width as f32 * 0.8) as usize) as u16;
        let modal_height = height.min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);

        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = modal_block(self.theme.COLOR_GREY_600, self.theme.COLOR_HIGHLIGHTED);

        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());

        // The input area is fixed-width inside the modal to keep cursor math stable.
        let input_area = Rect { x: modal_area.x + (modal_area.width / 2).saturating_sub(29), y: modal_area.y + 4, width: 58, height: 5 };

        render_modal_text_input(frame, input_area, &mut self.modal_input, false, Style::default().fg(self.theme.COLOR_TEXT), Style::default().fg(self.theme.COLOR_GREY_800), None, true);
    }
}

#[cfg(test)]
#[path = "../../../tests/app/draw/modals/input.rs"]
mod tests;
