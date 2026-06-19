use crate::app::input::TextInput;
use crate::helpers::{localisation::keymap, symbols::SymbolTheme};
use ratatui::Frame;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Widget},
};

pub(crate) fn modal_padding() -> Padding {
    Padding { left: 3, right: 3, top: 1, bottom: 1 }
}

pub(crate) fn esc_title(color: Color) -> Span<'static> {
    Span::styled(format!(" ({}) ", keymap::ESC().to_lowercase()), Style::default().fg(color))
}

pub(crate) fn modal_block<'a>(border_color: Color, esc_color: Color, symbols: &'a SymbolTheme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(esc_title(esc_color))
        .title_alignment(Alignment::Right)
        .padding(modal_padding())
        .border_set(symbols.border.block_set())
}

pub(crate) fn action_row(actions: &[(&str, &str)], style: Style) -> Line<'static> {
    let text = actions.iter().map(|(operation, key)| format!("{operation} ({key})")).collect::<Vec<_>>().join(" ");
    Line::from(Span::styled(text, style))
}

pub(crate) fn render_modal_text_input(
    frame: &mut Frame, area: Rect, input: &mut TextInput, masked: bool, text_style: Style, border_style: Style, title: Option<Span<'static>>, show_cursor: bool, symbols: &SymbolTheme,
) {
    let visible_width = area.width.saturating_sub(1) as usize;
    input.set_max_width(visible_width);
    let start = *input.scroll();
    let end = (start + visible_width).min(input.value().len());
    let visible = if masked {
        let value = symbols.modal.mask.repeat(input.value().chars().count());
        let start = start.min(value.len());
        let end = end.min(value.len());
        value[start..end].to_string()
    } else {
        input.value()[start..end].to_string()
    };

    let mut block = Block::default().padding(Padding { left: 1, right: 1, top: 1, bottom: 0 }).borders(Borders::TOP).border_style(border_style);
    if let Some(title) = title {
        block = block.title(title);
    }

    frame.render_widget(Paragraph::new(Line::from(Span::styled(visible, text_style))).block(block), area);

    if show_cursor {
        let cursor_x = input.cursor().saturating_sub(*input.scroll()) as u16 + 1;
        frame.set_cursor_position((area.x + cursor_x, area.y + 2));
    }

    Block::default()
        .borders(Borders::TOP)
        .border_style(border_style)
        .border_set(symbols.border.block_set())
        .render(Rect { x: area.x, y: area.y + 4, width: area.width, height: 1 }, frame.buffer_mut());
}
