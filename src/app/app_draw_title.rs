use crate::app::app::App;
use crate::helpers::{keymap::InputMode, symbols::SYM_FOLDER};
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::Block,
};

impl App {
    pub fn draw_title(&mut self, frame: &mut Frame) {
        // Logo and path
        let path = if let Some(file_name) = self.file_name.clone() {
            format!("{}/{}", self.path.clone(), file_name)
        } else {
            self.path.clone()
        };
        let logo = self.logo.clone();
        let separator = Span::styled(" |", Style::default().fg(self.theme.COLOR_TEXT));
        let folder = Span::styled(
            format!(" {SYM_FOLDER}  {}", path),
            Style::default().fg(self.theme.COLOR_TEXT),
        );
        let line = Line::from([logo, vec![separator, folder]].concat());
        let paragraph = ratatui::widgets::Paragraph::new(line)
            .left_aligned()
            .block(Block::default());
        frame.render_widget(paragraph, self.layout.title_left);

        let hint = if self.mode == InputMode::Action {
            Span::styled("action ", Style::default().fg(self.theme.COLOR_GRASS))
        } else {
            Span::styled("normal ", Style::default().fg(self.theme.COLOR_GREY_700))
        };
        let paragraph = ratatui::widgets::Paragraph::new(Line::from(hint))
            .right_aligned()
            .block(Block::default());
        frame.render_widget(paragraph, self.layout.title_right);
    }
}
