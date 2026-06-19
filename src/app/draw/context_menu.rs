use crate::app::app::{App, ContextMenuAction};
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

impl App {
    pub fn draw_context_menu(&mut self, frame: &mut Frame) {
        if self.is_modal_focus() {
            return;
        }

        let Some(menu) = self.context_menu.as_ref() else {
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
        let label_width = menu.label_width();
        let divider_width = label_width.saturating_add(3);
        let mut items = Vec::with_capacity(menu.items.len().saturating_add(2));
        items.push(ListItem::new(Line::from(Span::styled("", Style::default().bg(menu_bg)))).style(Style::default().bg(menu_bg)));
        items.extend(
            menu.items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    if item.action == ContextMenuAction::Divider {
                        let text = format!(" {} ", self.symbols.border.horizontal.repeat(divider_width));
                        return ListItem::new(Line::from(Span::styled(text, Style::default().fg(self.theme.COLOR_BORDER).bg(menu_bg)))).style(Style::default().bg(menu_bg));
                    }
                    if item.action == ContextMenuAction::Spacer {
                        return ListItem::new(Line::from(Span::styled("", Style::default().bg(menu_bg)))).style(Style::default().bg(menu_bg));
                    }

                    let enabled = item.enabled;
                    let selected = enabled && index == menu.selected;
                    let style = Style::default().fg(if enabled { self.theme.COLOR_TEXT } else { self.theme.COLOR_GREY_600 }).bg(if selected { selected_bg } else { menu_bg });
                    let marker = if selected { &self.symbols.modal.selected } else { &self.symbols.modal.unselected };
                    let text = format!(" {marker} {:<width$}  ", item.label, width = label_width);

                    ListItem::new(Line::from(Span::styled(text, style))).style(style)
                })
                .collect::<Vec<_>>(),
        );
        items.push(ListItem::new(Line::from(Span::styled("", Style::default().bg(menu_bg)))).style(Style::default().bg(menu_bg)));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.COLOR_BORDER).bg(menu_bg))
            .border_set(self.symbols.border.block_set())
            .style(Style::default().bg(menu_bg));

        frame.render_widget(List::new(items).block(block).style(Style::default().bg(menu_bg)), area);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/context_menu.rs"]
mod tests;
