use crate::{
    app::{
        app::{App, Focus},
        draw::{
            buffered::DrawTarget,
            modals::shared::{action_row, modal_block},
        },
    },
    helpers::text::wrap_words,
};
use ratatui::{
    layout::Alignment,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget},
};

impl App {
    pub fn draw_modal_rebase(&mut self, frame: &mut impl DrawTarget) {
        let title = match self.focus {
            Focus::ModalOperationProgress => self.modal_operation_kind.label().to_string(),
            Focus::ModalOperationConflict => format!("{} conflict", self.modal_operation_kind.label()),
            Focus::ModalOperationSuccess => format!("{} complete", self.modal_operation_kind.label()),
            _ => self.modal_operation_kind.label().to_string(),
        };

        let max_modal_width = (frame.area().width as f32 * 0.8) as usize;
        let text_width = max_modal_width.saturating_sub(10).clamp(1, 70);
        let wrapped_message = wrap_words(self.modal_operation_message.clone(), text_width);
        let mut lines = Vec::new();
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(title, Style::default().fg(if self.focus == Focus::ModalOperationConflict { self.theme.COLOR_ORANGE } else { self.theme.COLOR_TEXT }))));
        lines.push(Line::default());
        for line in wrapped_message {
            lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT))));
        }
        lines.push(Line::default());
        if self.focus == Focus::ModalOperationConflict {
            lines.push(Line::from(Span::styled("resolve conflicts in your editor, then action+Shift+C", Style::default().fg(self.theme.COLOR_TEXT))));
            lines.push(Line::default());
        }
        let action_line = if self.focus == Focus::ModalOperationProgress {
            Line::from(Span::styled("working...", Style::default().fg(self.theme.COLOR_HIGHLIGHTED)))
        } else {
            action_row(&[("ok", "enter")], Style::default().fg(self.theme.COLOR_HIGHLIGHTED))
        };
        lines.push(action_line);

        let content_width = lines.iter().map(|line| line.width()).max().unwrap_or(0);
        let modal_width = (content_width + 10).max(34).min(max_modal_width) as u16;
        let max_modal_height = (frame.area().height as f32 * 0.6) as usize;
        let modal_height = (lines.len() + 4).max(8).min(max_modal_height.max(1)) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = ratatui::layout::Rect::new(x, y, modal_width, modal_height);

        self.theme.clear_area(modal_area, frame.buffer_mut());

        let border_color = if self.focus == Focus::ModalOperationConflict { self.theme.COLOR_ORANGE } else { self.theme.COLOR_BORDER };
        let modal_block = modal_block(border_color, self.theme.COLOR_HIGHLIGHTED);

        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());
    }
}
