use crate::app::app::{App, CONTEXT_MENU_LABEL_WIDTH, ContextMenuAction};
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
};

impl App {
    pub fn draw_context_menu(&mut self, frame: &mut Frame) {
        if self.is_modal_focus() {
            return;
        }

        let Some(menu) = self.context_menu else {
            return;
        };
        let Some(area) = self.context_menu_area_for_bounds(frame.area()) else {
            return;
        };
        if area.width < 4 || area.height < 3 {
            return;
        }

        self.theme.clear_area(area, frame.buffer_mut());

        let menu_bg = self.theme.background_or_default(self.theme.COLOR_GREY_900);
        let selected_bg = self.theme.background_or_default(self.theme.COLOR_GREY_800);
        let items = ContextMenuAction::ALL
            .iter()
            .enumerate()
            .map(|(index, &action)| {
                let enabled = self.context_menu_action_enabled(action);
                let selected = enabled && index == menu.selected;
                let style = Style::default().fg(if enabled { self.theme.COLOR_TEXT } else { self.theme.COLOR_GREY_600 }).bg(if selected { selected_bg } else { menu_bg });
                let marker = if selected { ">" } else { " " };
                let text = format!("{marker} {:<width$} ", action.label(), width = CONTEXT_MENU_LABEL_WIDTH);

                ListItem::new(Line::from(Span::styled(text, style))).style(style)
            })
            .collect::<Vec<_>>();

        let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(self.theme.COLOR_BORDER).bg(menu_bg)).border_type(BorderType::Rounded).style(Style::default().bg(menu_bg));

        frame.render_widget(List::new(items).block(block).style(Style::default().bg(menu_bg)), area);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/context_menu.rs"]
mod tests;
