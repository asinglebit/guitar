use crate::{
    app::{
        app::App,
        draw::modals::shared::{action_row, modal_block, render_modal_text_input},
    },
    git::queries::files::FileSearchResult,
    helpers::localisation::modal,
};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph, Widget},
};
use std::collections::HashSet;

fn path_spans(result: &FileSearchResult, max_width: usize, text_color: Color, match_color: Color) -> Vec<Span<'static>> {
    let matched: HashSet<usize> = result.matched_indices.iter().copied().collect();
    let chars: Vec<char> = result.path.chars().collect();
    let is_truncated = chars.len() > max_width;
    let visible_chars = if is_truncated && max_width > 3 { max_width - 3 } else { max_width };
    let base_style = |color| Style::default().fg(color);

    let mut spans: Vec<Span<'static>> = chars
        .iter()
        .take(visible_chars)
        .enumerate()
        .map(|(idx, ch)| {
            let color = if matched.contains(&idx) { match_color } else { text_color };
            Span::styled(ch.to_string(), base_style(color))
        })
        .collect();

    if is_truncated && max_width > 3 {
        spans.push(Span::styled("...", base_style(text_color)));
    }

    spans
}

impl App {
    pub fn draw_modal_file_search(&mut self, frame: &mut Frame, title: &str) {
        let length = 76;
        let height = 20;

        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        let modal_width = length.min((frame.area().width as f32 * 0.85) as usize) as u16;
        let modal_height = height.min((frame.area().height as f32 * 0.8) as usize) as u16;
        let x = frame.area().x + (frame.area().width.saturating_sub(modal_width)) / 2;
        let y = frame.area().y + (frame.area().height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);

        self.theme.clear_area(modal_area, frame.buffer_mut());

        let modal_block = modal_block(self.theme.COLOR_GREY_600, self.theme.COLOR_HIGHLIGHTED, &self.symbols);
        modal_block.render(modal_area, frame.buffer_mut());

        let inner_width = modal_area.width.saturating_sub(8);
        let inner_x = modal_area.x + 4;
        let title_area = Rect { x: inner_x, y: modal_area.y + 2, width: inner_width, height: 1 };
        let input_area = Rect { x: modal_area.x + 1, y: modal_area.y + 4, width: modal_area.width.saturating_sub(2), height: 5 };
        let action_area = Rect { x: inner_x, y: modal_area.y + modal_area.height.saturating_sub(3), width: inner_width, height: 1 };
        let list_y = modal_area.y + 10;
        let list_bottom = action_area.y.saturating_sub(1);
        let list_area = Rect { x: modal_area.x + 1, y: list_y, width: inner_width, height: list_bottom.saturating_sub(list_y) };

        frame.render_widget(Paragraph::new(Line::from(Span::styled(title.to_string(), Style::default().fg(self.theme.COLOR_TEXT)))).alignment(Alignment::Center), title_area);

        render_modal_text_input(frame, input_area, &mut self.modal_input, false, Style::default().fg(self.theme.COLOR_TEXT), Style::default().fg(self.theme.COLOR_GREY_800), None, true, &self.symbols);

        let total = self.modal_file_search_results.len();
        let visible_height = list_area.height as usize;
        let mut selected = usize::try_from(self.modal_file_search_selected).unwrap_or(0);
        if total == 0 {
            selected = 0;
            self.modal_file_search_selected = 0;
            self.modal_file_search_scroll.set(0);
        } else {
            selected = selected.min(total.saturating_sub(1));
            self.modal_file_search_selected = selected as i32;
            self.trap_selection(selected, &self.modal_file_search_scroll, total, visible_height);
        }

        let start = self.modal_file_search_scroll.get().min(total.saturating_sub(visible_height));
        let end = (start + visible_height).min(total);
        let max_path_width = list_area.width.saturating_sub(2) as usize;

        let list_items: Vec<ListItem<'static>> = if total == 0 {
            let message = if self.modal_input.value().trim().is_empty() { modal::TYPE_TO_SEARCH() } else { modal::NO_MATCHES() };
            vec![ListItem::new(Line::from(Span::styled(message, Style::default().fg(self.theme.COLOR_GREY_800))))]
        } else {
            self.modal_file_search_results[start..end]
                .iter()
                .enumerate()
                .map(|(idx, result)| {
                    let absolute = start + idx;
                    let is_selected = absolute == selected;
                    let text_color = if is_selected { self.theme.COLOR_HIGHLIGHTED } else { self.theme.COLOR_TEXT };
                    let mut spans = vec![Span::raw("  ")];
                    spans.extend(path_spans(result, max_path_width, text_color, self.theme.COLOR_GRASS));
                    ListItem::new(Line::from(spans))
                })
                .collect()
        };

        frame.render_widget(List::new(list_items), list_area);
        frame.render_widget(
            Paragraph::new(action_row(&[(modal::ACTION_CHOOSE(), modal::KEY_ENTER()), (modal::ACTION_MOVE(), modal::KEY_CTRL_J_K())], Style::default().fg(self.theme.COLOR_HIGHLIGHTED)))
                .alignment(Alignment::Center),
            action_area,
        );
    }
}

#[cfg(test)]
#[path = "../../../tests/app/draw/modals/file_search.rs"]
mod tests;
