use crate::app::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_checkout(&mut self, frame: &mut Frame) {
        let mut length = 30;
        let mut height = 8;
        let alias = self.oids.get_alias_by_idx(self.graph_selected);
        let mut lines = Vec::new();
        let line_text = "select a branch to checkout";
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(
            line_text,
            Style::default().fg(self.theme.COLOR_TEXT),
        )]));
        lines.push(Line::default());

        // Render list
        let color = self.branches.colors.get(&alias).unwrap();
        let branches = self.branches.visible.get(&alias).unwrap();
        branches.iter().enumerate().for_each(|(idx, branch)| {
            height += 1;
            let is_local = self
                .branches
                .local
                .values()
                .any(|branches| branches.iter().any(|b| b.as_str() == branch));
            length = (10 + branch.len()).max(length);
            lines.push(Line::from(Span::styled(
                format!("{} {} ", if is_local { "●" } else { "◆" }, branch),
                Style::default().fg(if idx == self.modal_checkout_selected as usize {
                    *color
                } else {
                    self.theme.COLOR_TEXT
                }),
            )));
        });

        // Background
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        // Modal size (smaller than area)
        let modal_width = length.min((frame.area().width as f32 * 0.8) as usize) as u16;
        let modal_height = height.min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width - modal_width) / 2;
        let y = frame.area().y + (frame.area().height - modal_height) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        frame.render_widget(Clear, modal_area);

        // Padding
        let padding = ratatui::widgets::Padding {
            left: 3,
            right: 3,
            top: 1,
            bottom: 1,
        };

        // Modal block
        let modal_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.COLOR_GREY_600))
            .title(Span::styled(
                " (esc) ",
                Style::default().fg(self.theme.COLOR_GREY_500),
            ))
            .title_alignment(Alignment::Right)
            .padding(padding)
            .border_type(ratatui::widgets::BorderType::Rounded);

        // Modal content
        let paragraph = Paragraph::new(Text::from(lines))
            .block(modal_block)
            .alignment(Alignment::Center);

        paragraph.render(modal_area, frame.buffer_mut());
    }
}
