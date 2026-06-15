use crate::{
    app::{
        app::App,
        draw::{
            buffered::DrawTarget,
            modals::shared::{action_row, modal_block},
        },
    },
    helpers::{
        keymap::{KeymapEditError, command_to_visual_string, input_mode_to_visual_string, keybinding_to_visual_string},
        text::wrap_words,
    },
};
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_key_capture(&mut self, frame: &mut impl DrawTarget) {
        let Some(selection) = &self.modal_key_capture_selection else {
            return;
        };

        let mut lines = Vec::new();
        lines.push(Line::from(Span::styled("set shortcut", Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(format!("{} / {}", input_mode_to_visual_string(selection.mode), command_to_visual_string(&selection.command)), Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::from(Span::styled(format!("current: {}", keybinding_to_visual_string(&selection.key)), Style::default().fg(self.theme.COLOR_GREY_600))));

        if let Some(candidate) = &self.modal_key_capture_candidate {
            let color = if self.modal_key_capture_error.is_some() { self.theme.COLOR_ORANGE } else { self.theme.COLOR_GRASS };
            lines.push(Line::from(Span::styled(format!("new: {}", keybinding_to_visual_string(candidate)), Style::default().fg(color))));
        } else {
            lines.push(Line::from(Span::styled("new: waiting for key", Style::default().fg(self.theme.COLOR_GREY_600))));
        }

        if let Some(error) = &self.modal_key_capture_error {
            lines.push(Line::default());
            let message = match error {
                KeymapEditError::Conflict { mode, key, command } => {
                    format!("conflict: {} {} already runs {}", input_mode_to_visual_string(*mode), keybinding_to_visual_string(key), command_to_visual_string(command))
                },
                KeymapEditError::MissingMode(mode) => format!("missing keymap mode: {}", input_mode_to_visual_string(*mode)),
                KeymapEditError::MissingBinding { mode, key } => format!("missing binding: {} {}", input_mode_to_visual_string(*mode), keybinding_to_visual_string(key)),
                KeymapEditError::CommandChanged { mode, key, expected, actual } => format!(
                    "binding changed: {} {} was {}, now {}",
                    input_mode_to_visual_string(*mode),
                    keybinding_to_visual_string(key),
                    command_to_visual_string(expected),
                    command_to_visual_string(actual)
                ),
            };
            for line in wrap_words(message, 64) {
                lines.push(Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_ORANGE))));
            }
        }

        lines.push(Line::default());
        let line = if self.modal_key_capture_candidate.is_some() && self.modal_key_capture_error.is_none() {
            action_row(&[("save", "enter")], Style::default().fg(self.theme.COLOR_HIGHLIGHTED))
        } else {
            Line::from(Span::styled("press key", Style::default().fg(self.theme.COLOR_HIGHLIGHTED)))
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
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let border_color = if self.modal_key_capture_error.is_some() { self.theme.COLOR_ORANGE } else { self.theme.COLOR_GREY_600 };
        let modal_block = modal_block(border_color, self.theme.COLOR_HIGHLIGHTED);

        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());
    }
}

#[cfg(test)]
#[path = "../../../tests/app/draw/modals/key_capture.rs"]
mod tests;
