use crate::{
    app::{
        app::{App, AuthInputField},
        draw::modals::shared::{action_row, modal_block, render_modal_text_input},
    },
    git::auth::AuthProtocol,
    helpers::{
        localisation::{common, modal},
        text::{truncate_start_with_ellipsis, wrap_words},
    },
};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

impl App {
    pub fn draw_modal_network_progress(&mut self, frame: &mut Frame) {
        let max_modal_width = (frame.area().width as f32 * 0.8) as usize;
        let text_width = max_modal_width.saturating_sub(10).clamp(1, 70);
        let wrapped_message = wrap_words(self.modal_network_message.clone(), text_width);
        let mut lines = Vec::new();
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(self.modal_network_title.clone(), Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::default());
        lines.extend(wrapped_message.into_iter().map(|line| Line::from(Span::styled(line, Style::default().fg(self.theme.COLOR_TEXT)))));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(common::WORKING, Style::default().fg(self.theme.COLOR_HIGHLIGHTED))));

        self.draw_auth_text_modal(frame, lines, self.theme.COLOR_BORDER);
    }

    pub fn draw_modal_auth(&mut self, frame: &mut Frame) {
        let Some(challenge) = self.pending_auth_prompt.clone() else {
            return;
        };

        let max_modal_width = (frame.area().width as f32 * 0.86) as usize;
        let modal_width = 72.min(max_modal_width).max(34) as u16;
        let inner_width = modal_width.saturating_sub(8) as usize;
        let mut lines = Vec::new();
        let mut field_offsets = Vec::new();
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(challenge.title(), Style::default().fg(self.theme.COLOR_TEXT))));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(challenge.operation.clone(), Style::default().fg(self.theme.COLOR_HIGHLIGHTED))));
        lines.push(Line::from(Span::styled(truncate_start_with_ellipsis(&challenge.url, inner_width), Style::default().fg(self.theme.COLOR_TEXT))));

        if challenge.protocol == AuthProtocol::Ssh {
            lines.push(Line::default());
            if let Some(username) = &challenge.username {
                lines.push(Line::from(Span::styled(format!("{} {username}", modal::AUTH_USER), Style::default().fg(self.theme.COLOR_GREY_600))));
            }
            if let Some(path) = &challenge.key_path {
                lines.push(Line::from(Span::styled(
                    format!("{} {}", modal::AUTH_KEY, truncate_start_with_ellipsis(&path.display().to_string(), inner_width.saturating_sub(5))),
                    Style::default().fg(self.theme.COLOR_GREY_600),
                )));
            }
        }

        lines.push(Line::default());
        field_offsets.push(lines.len());
        lines.extend(vec![Line::default(); 5]);
        if challenge.protocol.is_http() {
            lines.push(Line::default());
            field_offsets.push(lines.len());
            lines.extend(vec![Line::default(); 5]);
        }
        lines.push(Line::default());
        lines.push(action_row(&[(modal::ACTION_SUBMIT, modal::KEY_ENTER), (modal::ACTION_SWITCH_FIELD, modal::KEY_TAB)], Style::default().fg(self.theme.COLOR_GREY_600)));

        let desired_height = lines.len().saturating_add(4);
        let max_modal_height = (frame.area().height as usize).max(1);
        let modal_height = desired_height.min(max_modal_height).max(10) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);

        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());
        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = modal_block(self.theme.COLOR_ORANGE, self.theme.COLOR_HIGHLIGHTED);
        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());

        let field_width = modal_width.saturating_sub(14);
        let field_x = modal_area.x + 7;
        let first_field_y = modal_area.y + 2 + field_offsets[0] as u16;
        if challenge.protocol.is_http() {
            self.draw_auth_field(frame, Rect::new(field_x, first_field_y, field_width, 5), modal::AUTH_USERNAME, AuthInputField::Username, false);
            self.draw_auth_field(frame, Rect::new(field_x, modal_area.y + 2 + field_offsets[1] as u16, field_width, 5), modal::AUTH_PASSWORD_TOKEN, AuthInputField::Secret, true);
        } else {
            self.draw_auth_field(frame, Rect::new(field_x, first_field_y, field_width, 5), modal::AUTH_PASSPHRASE, AuthInputField::Secret, true);
        }
    }

    fn draw_auth_field(&mut self, frame: &mut Frame, area: Rect, label: &str, field: AuthInputField, masked: bool) {
        let active = self.auth_input_field == field;
        let border = if active { self.theme.COLOR_HIGHLIGHTED } else { self.theme.COLOR_GREY_800 };
        let label_style = if active { Style::default().fg(self.theme.COLOR_HIGHLIGHTED).add_modifier(Modifier::BOLD) } else { Style::default().fg(self.theme.COLOR_GREY_600) };
        let text_style = Style::default().fg(self.theme.COLOR_TEXT);
        let border_style = Style::default().fg(border);
        let input = if field == AuthInputField::Username { &mut self.auth_username_input } else { &mut self.auth_secret_input };
        render_modal_text_input(frame, area, input, masked, text_style, border_style, Some(Span::styled(format!(" {label} "), label_style)), active);
    }

    fn draw_auth_text_modal(&mut self, frame: &mut Frame, lines: Vec<Line>, border_color: ratatui::style::Color) {
        let max_modal_width = (frame.area().width as f32 * 0.8) as usize;
        let content_width = lines.iter().map(|line| line.width()).max().unwrap_or(0);
        let modal_width = (content_width + 10).max(34).min(max_modal_width) as u16;
        let max_modal_height = (frame.area().height as f32 * 0.6) as usize;
        let modal_height = (lines.len() + 4).max(8).min(max_modal_height.max(1)) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);

        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = modal_block(border_color, self.theme.COLOR_HIGHLIGHTED);

        Paragraph::new(Text::from(lines)).block(modal_block).alignment(Alignment::Center).render(modal_area, frame.buffer_mut());
    }
}

#[cfg(test)]
#[path = "../../../tests/app/draw/modals/auth.rs"]
mod tests;
