use crate::{
    app::{
        app::App,
        draw::modals::shared::{action_row, modal_block},
    },
    helpers::{
        keymap::{KeymapEditError, command_to_visual_string, input_mode_to_visual_string, keybinding_to_visual_string},
        localisation::modal,
        text::wrap_words,
    },
};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_key_capture(&mut self, frame: &mut Frame) {
        let Some(selection) = &self.modal_key_capture_selection else {
            return;
        };

        let mut lines = Vec::new();
        lines.push(Line::from(Span::styled(modal::SET_SHORTCUT(), Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(format!("{} / {}", input_mode_to_visual_string(selection.mode), command_to_visual_string(&selection.command)), Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::from(Span::styled(format!("{} {}", modal::CURRENT_SHORTCUT(), keybinding_to_visual_string(&selection.key)), Style::default().fg(self.theme.COLOR_GREY_600))));

        if let Some(candidate) = &self.modal_key_capture_candidate {
            let color = if self.modal_key_capture_error.is_some() { self.theme.COLOR_ORANGE } else { self.theme.COLOR_GRASS };
            lines.push(Line::from(Span::styled(format!("{} {}", modal::NEW_SHORTCUT(), keybinding_to_visual_string(candidate)), Style::default().fg(color))));
        } else {
            lines.push(Line::from(Span::styled(modal::NEW_SHORTCUT_WAITING(), Style::default().fg(self.theme.COLOR_GREY_600))));
        }

        if let Some(error) = &self.modal_key_capture_error {
            lines.push(Line::default());
            let message = match error {
                KeymapEditError::Conflict { mode, key, command } => modal::keymap_conflict(&input_mode_to_visual_string(*mode), &keybinding_to_visual_string(key), &command_to_visual_string(command)),
                KeymapEditError::MissingMode(mode) => modal::keymap_missing_mode(&input_mode_to_visual_string(*mode)),
                KeymapEditError::MissingBinding { mode, key } => modal::keymap_missing_binding(&input_mode_to_visual_string(*mode), &keybinding_to_visual_string(key)),
                KeymapEditError::CommandChanged { mode, key, expected, actual } => {
                    modal::keymap_binding_changed(&input_mode_to_visual_string(*mode), &keybinding_to_visual_string(key), &command_to_visual_string(expected), &command_to_visual_string(actual))
                },
            };
            for line in wrap_words(message, 64) {
                lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_ORANGE))));
            }
        }

        lines.push(Line::default());
        let line = if self.modal_key_capture_candidate.is_some() && self.modal_key_capture_error.is_none() {
            action_row(&[(modal::ACTION_SAVE(), modal::KEY_ENTER())], Style::default().fg(self.theme.COLOR_HIGHLIGHTED))
        } else {
            Line::from(Span::styled(modal::PRESS_KEY(), Style::default().fg(self.theme.COLOR_HIGHLIGHTED)))
        };
        lines.push(line);

        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        let content_width = lines.iter().map(|line| line.width()).max().unwrap_or(34);
        let modal_width = (content_width + 10).max(42).min((frame.area().width as f32 * 0.8) as usize) as u16;
        let modal_height = (lines.len() + 4).max(10).min(((frame.area().height as f32 * 0.6) as usize).max(1)) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let border_color = if self.modal_key_capture_error.is_some() { self.theme.COLOR_ORANGE } else { self.theme.COLOR_GREY_600 };
        let modal_block = modal_block(border_color, self.theme.COLOR_HIGHLIGHTED, &self.symbols);

        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());
    }
}

#[cfg(test)]
#[path = "../../../tests/app/draw/modals/key_capture.rs"]
mod tests;
