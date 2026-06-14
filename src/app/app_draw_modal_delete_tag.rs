use crate::app::app::App;
use crate::helpers::symbols::SYM_TAG;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_delete_tag(&mut self, frame: &mut Frame) {
        let mut length = 30;
        let mut height = 8;
        let alias = self.oids.get_alias_by_idx(self.graph_selected);
        let mut lines = Vec::new();
        let line_text = "select a tag to delete";
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(line_text, Style::default().fg(self.theme.COLOR_TEXT))]));
        lines.push(Line::default());

        // Tag choices come from the selected commit alias.
        let color = self.tags.colors.get(&alias).copied().unwrap_or(self.theme.COLOR_TEXT);
        let tags = self.tags.local.get(&alias).cloned().unwrap_or_default();
        tags.iter().enumerate().for_each(|(idx, tag)| {
            height += 1;
            let line_text = format!("{} {} ", SYM_TAG, tag);
            length = length.max(line_text.len());

            lines.push(Line::from(Span::styled(line_text, Style::default().fg(if idx == self.modal_delete_tag_selected as usize { color } else { self.theme.COLOR_TEXT }))));
        });

        // Paint a plain overlay before clearing the modal rectangle.
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        // The modal grows with tag names but is capped to the terminal size.
        length += 10;
        let modal_width = length.min((frame.area().width as f32 * 0.8) as usize) as u16;
        let modal_height = height.min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.theme.clear_area(modal_area, frame.buffer_mut());

        // Padding keeps tag names away from rounded borders.
        let padding = ratatui::widgets::Padding { left: 3, right: 3, top: 1, bottom: 1 };

        // The title on the right doubles as the close affordance.
        let modal_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.COLOR_GREY_600))
            .title(Span::styled(" (esc) ", Style::default().fg(self.theme.COLOR_GREY_500)))
            .title_alignment(Alignment::Right)
            .padding(padding)
            .border_type(ratatui::widgets::BorderType::Rounded);

        let paragraph = Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center);

        paragraph.render(modal_area, frame.buffer_mut());
    }
}
