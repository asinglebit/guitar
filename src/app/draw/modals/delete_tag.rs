use crate::app::{app::App, draw::modals::shared::modal_block};
use crate::helpers::{localisation::modal, symbols::entity};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_delete_tag(&mut self, frame: &mut Frame) {
        let mut length = 30;
        let mut height = 8;
        let Some(alias) = self.graph_alias_at(self.graph_selected) else {
            return;
        };
        let mut lines = Vec::new();
        let line_text = modal::SELECT_TAG_DELETE;
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(line_text, Style::default().fg(self.theme.COLOR_TEXT))]));
        lines.push(Line::default());

        // Tag choices come from the selected commit alias.
        let tags: Vec<String> =
            self.graph_row_at(self.graph_selected).map(|row| row.tags.iter().map(|tag| tag.name.clone()).collect()).unwrap_or_else(|| self.tags.local.get(&alias).cloned().unwrap_or_default());
        tags.iter().enumerate().for_each(|(idx, tag)| {
            height += 1;
            let is_selected = idx == self.modal_delete_tag_selected as usize;
            let line_text = format!("{} {} ", entity::TAG, tag);
            length = length.max(line_text.len());

            let style = Style::default().fg(if is_selected { self.theme.COLOR_GRASS } else { self.theme.COLOR_TEXT });

            lines.push(Line::from(Span::styled(line_text, style)));
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
        self.modal_area = Some(modal_area);
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = modal_block(self.theme.COLOR_GREY_600, self.theme.COLOR_HIGHLIGHTED);

        let paragraph = Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center);

        paragraph.render(modal_area, frame.buffer_mut());
    }
}
