use crate::app::app::{App, Focus};
use crate::helpers::keymap::{Command, InputMode, keybinding_to_visual_string};
use crate::helpers::localisation::splash as splash_text;
use ratatui::Frame;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem},
};

impl App {
    pub(crate) fn recent_repository_command_key(&self, command: &Command, fallback: &str) -> String {
        self.keymaps
            .get(&InputMode::Normal)
            .and_then(|mode_keymap| mode_keymap.iter().find(|(_, current)| *current == command).map(|(key, _)| keybinding_to_visual_string(key)))
            .unwrap_or_else(|| fallback.to_string())
    }

    pub(crate) fn recent_repository_actions_detail_text(&self) -> String {
        let remove = self.recent_repository_command_key(&Command::RemoveRecentRepository, splash_text::KEY_REMOVE_FALLBACK());
        let move_up = self.recent_repository_command_key(&Command::MoveRecentRepositoryUp, splash_text::KEY_MOVE_UP_FALLBACK());
        let move_down = self.recent_repository_command_key(&Command::MoveRecentRepositoryDown, splash_text::KEY_MOVE_DOWN_FALLBACK());

        splash_text::recent_actions(&remove, &move_up, &move_down)
    }

    pub(crate) fn recent_repository_actions_text(&self) -> String {
        splash_text::actions(&self.recent_repository_actions_detail_text())
    }

    #[rustfmt::skip]
    pub fn draw_splash(&mut self, frame: &mut Frame) {
        // Splash owns the full app rectangle and centers its content.
        let padding = ratatui::widgets::Padding { left: 1, right: 1, top: 0, bottom: 0 };

        // Keep the width calculation visible for future splash text truncation.
        let available_width = self.layout.graph.width.saturating_sub(1) as usize;
        let _max_text_width = available_width.saturating_sub(2);

        // Reuse viewer scroll fields because splash behaves like a simple list.
        let total_lines = self.viewer_lines.len();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(4) as usize } else { self.layout.graph.height.saturating_sub(2) as usize };

        if total_lines == 0 {
            self.viewer_selected = 0;
        } else if self.viewer_selected >= total_lines {
            self.viewer_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.viewer_selected, &self.viewer_scroll, total_lines, visible_height);

        let start = self.viewer_scroll.get().min(total_lines.saturating_sub(visible_height));
        let _end = (start + visible_height).min(total_lines);

        // Lines are assembled manually so the logo and recent list share centering.
        let mut lines: Vec<Line> = Vec::new();

        // Content height varies between loading, empty recent list, and recent repositories.
        let content_rows =
            if self.spinner.is_running() {
                1
            } else if self.recent.is_empty() && self.repo.is_none() {
                5
            } else if self.recent.is_empty() {
                3
            } else {
                5 + self.recent.len()
            };

        // Logo detail scales down for narrow terminals.
        let logo_rows = if self.layout.app.width < 80 {
            1
        } else if self.layout.app.width < 120 {
            9
        } else {
            11
        };

        let visible = visible_height;

        let splash_rows = logo_rows + content_rows;

        // Add blank rows above the splash body to center it vertically.
        let dummies = visible
            .saturating_sub(splash_rows)
            .saturating_div(2);

        for _ in 0..dummies {
            lines.push(Line::default());
        }

        if self.layout.app.width < 80 {
            lines.push(Line::from(Span::styled(self.symbols.splash.logo_compact.clone(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
        } else if self.layout.app.width < 120 {
            for (idx, row) in self.symbols.splash.logo_narrow.iter().enumerate() {
                let color = if idx < 4 { self.theme.COLOR_GRASS } else { self.theme.COLOR_GREEN };
                lines.push(Line::from(Span::styled(row.clone(), Style::default().fg(color))).centered());
            }
        } else {
            for (idx, row) in self.symbols.splash.logo_wide.iter().enumerate() {
                let color = if idx < 5 { self.theme.COLOR_GRASS } else { self.theme.COLOR_GREEN };
                lines.push(Line::from(Span::styled(row.clone(), Style::default().fg(color))).centered());
            }
        }

        lines.push(Line::default());
        if self.spinner.is_running() {
            let icon_spinner = format!("{} ", self.spinner.get_char());
            lines.push(Line::from(vec![Span::styled(format!("{} {}", icon_spinner, splash_text::LOADING()), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
        } else if self.recent.is_empty() {
            lines.push(Line::from(vec![Span::styled(splash_text::MADE_WITH().to_string(), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
            lines.push(Line::default());
            lines.push(Line::from(vec![Span::styled(splash_text::REPOSITORY_URL().to_string(), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
            if self.repo.is_none() {
                lines.push(Line::default());
                lines.push(Line::from(vec![Span::styled(splash_text::NOT_A_VALID_GIT_REPOSITORY().to_string(), Style::default().fg(self.theme.COLOR_ORANGE))]).centered());
            }
        } else {
            lines.push(Line::from(vec![Span::styled(splash_text::RECENT_REPOSITORIES().to_string(), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
            lines.push(Line::default());
            lines.push(Line::from(vec![Span::styled(self.recent_repository_actions_text(), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
            lines.push(Line::default());
            // Recent repositories are selectable only when loading has finished.
            self.recent.iter().enumerate().for_each(|(i, path)| {
                let style = if Some(path) == self.path.as_ref() {
                    self.theme.COLOR_GRASS
                } else {
                    self.theme.COLOR_TEXT
                };

                let mut line = Line::from(Span::styled(path.clone(), Style::default().fg(style))).centered();

                // Brackets make the current splash selection visible without changing row width too much.
                if i == self.splash_selected && self.focus == Focus::Viewport && !self.spinner.is_running() {
                    let mut spans = Vec::new();
                    spans.push(Span::styled(self.symbols.splash.selected_left.clone(), Style::default().fg(self.theme.COLOR_GRASS)));
                    spans.extend(line.spans.clone());
                    spans.push(Span::styled(self.symbols.splash.selected_right.clone(), Style::default().fg(self.theme.COLOR_GRASS)));
                    line = Line::from(spans).centered();
                }

                lines.push(line);
            });
        }

        // Convert the assembled splash lines into the list widget expected by ratatui.
        let list_items: Vec<ListItem> = lines.into_iter().map(ListItem::from).collect();
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.app);
    }
}
