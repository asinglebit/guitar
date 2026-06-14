use crate::app::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_remove_worktree(&mut self, frame: &mut Frame) {
        let mut length = 42;
        let mut lines = Vec::new();

        lines.push(Line::default());
        lines.push(Line::from(Span::styled("remove selected worktree?", Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::default());

        if let Some(entry) = self.modal_worktree_target.and_then(|idx| self.worktrees.entries.get(idx)) {
            length = length.max(entry.name.len() + 12);
            length = length.max(entry.path.display().to_string().len() + 8);
            lines.push(Line::from(Span::styled(format!("name: {}", entry.name), Style::default().fg(self.theme.COLOR_GRAPEFRUIT))));
            lines.push(Line::from(Span::styled(format!("path: {}", entry.path.display()), Style::default().fg(self.theme.COLOR_TEXT))));
        }

        lines.push(Line::default());
        lines.push(Line::from(Span::styled("Enter confirms", Style::default().fg(self.theme.COLOR_GREY_500))));

        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        let modal_width = (length + 8).min((frame.area().width as f32 * 0.8) as usize) as u16;
        let modal_height = (lines.len() + 4).min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let padding = ratatui::widgets::Padding { left: 3, right: 3, top: 1, bottom: 1 };
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
