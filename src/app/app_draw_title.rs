use crate::app::app::{App, Focus};
use crate::helpers::symbols::SYM_FOLDER;
use crate::helpers::text::truncate_start_with_ellipsis;
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::Block,
};
impl App {
    pub fn draw_title(&mut self, frame: &mut Frame) {
        let available_width = self.layout.title_left.width.saturating_sub(15) as usize;

        // Logo and path
        let path = if let Some(file_name) = self.file_name.clone() {
            match &self.path {
                Some(base) => format!("{}/{}", base, file_name),
                None => file_name.clone(),
            }
        } else {
            self.path.clone().unwrap_or_else(|| ".".to_string())
        };

        let logo = self.logo.clone();
        let separator = Span::styled(" |", Style::default().fg(self.theme.COLOR_TEXT));
        let folder = Span::styled(format!(" {SYM_FOLDER} {}", truncate_start_with_ellipsis(path.as_str(), available_width)), Style::default().fg(self.theme.COLOR_TEXT));

        let line = Line::from([logo, vec![separator, folder]].concat());
        let paragraph = ratatui::widgets::Paragraph::new(line).left_aligned().block(Block::default());

        frame.render_widget(paragraph, self.layout.title_left);

        let focus_name = match self.focus {
            Focus::Viewport => "graph",
            Focus::Branches => "branches",
            Focus::Tags => "tags",
            Focus::Stashes => "stashes",
            Focus::Inspector => "inspector",
            Focus::StatusTop => "staged",
            Focus::StatusBottom => "unstaged",
            _ => "modal",
        };

        let hint_line = Line::from(Span::styled(format!("{} ", focus_name), Style::default().fg(self.theme.COLOR_GREY_700)));

        let paragraph = ratatui::widgets::Paragraph::new(hint_line).right_aligned().block(Block::default());

        frame.render_widget(paragraph, self.layout.title_right);
    }
}
