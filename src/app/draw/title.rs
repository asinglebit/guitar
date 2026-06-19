use crate::app::app::{App, Focus, Viewport};
use crate::helpers::localisation::{settings, status as status_text};
use crate::helpers::text::truncate_start_with_ellipsis;
use ratatui::Frame;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::Block,
};
impl App {
    pub fn draw_title(&mut self, frame: &mut Frame) {
        let available_width = self.layout.title_left.width.saturating_sub(15) as usize;

        // Logo and path
        let path = if self.viewport == Viewport::Viewer
            && let Some(file_name) = self.file_name.clone()
        {
            match &self.path {
                Some(base) => format!("{}/{}", base, file_name),
                None => file_name.clone(),
            }
        } else {
            self.path.clone().unwrap_or_else(|| ".".to_string())
        };

        let logo = self.logo.clone();
        let separator = Span::styled(" |", Style::default().fg(self.theme.COLOR_TEXT));
        let folder = Span::styled(format!(" {} {}", self.symbols.entity.folder, truncate_start_with_ellipsis(path.as_str(), available_width)), Style::default().fg(self.theme.COLOR_TEXT));

        let line = Line::from([logo, vec![separator, folder]].concat());
        let paragraph = ratatui::widgets::Paragraph::new(line).left_aligned().block(Block::default());

        frame.render_widget(paragraph, self.layout.title_left);

        let focus_name = match self.focus {
            Focus::Viewport if self.viewport == Viewport::Settings => settings::SETTINGS(),
            Focus::Viewport if self.viewport == Viewport::Viewer => status_text::VIEWER(),
            Focus::Viewport => status_text::GRAPH(),
            Focus::Branches => settings::BRANCHES(),
            Focus::Tags => settings::TAGS(),
            Focus::Stashes => settings::STASHES(),
            Focus::Reflogs => settings::REFLOG(),
            Focus::Worktrees => settings::WORKTREES(),
            Focus::Submodules => settings::SUBMODULES(),
            Focus::Search => status_text::SEARCH(),
            Focus::Inspector => status_text::INSPECTOR(),
            Focus::StatusTop => status_text::STAGED(),
            Focus::StatusBottom => status_text::UNSTAGED(),
            _ => status_text::MODAL(),
        };

        let hint_line = Line::from(Span::styled(format!("{} ", focus_name), Style::default().fg(self.theme.COLOR_HIGHLIGHTED)));

        let paragraph = ratatui::widgets::Paragraph::new(hint_line).right_aligned().block(Block::default());

        frame.render_widget(paragraph, self.layout.title_right);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/title.rs"]
mod tests;
