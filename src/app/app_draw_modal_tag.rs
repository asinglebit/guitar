#[rustfmt::skip]
use ratatui::{
    Frame,
    style::{
        Style,
    },
    layout::{
        Alignment,
        Rect
    },
    text::{
        Line,
        Span,
        Text
    },
    widgets::{
        Block,
        Borders,
        Clear,
        Padding,
        Paragraph,
        Widget
    },
};
#[rustfmt::skip]
use edtui::{
    EditorStatusLine,
    EditorTheme,
    EditorView,
    EditorMode
};
#[rustfmt::skip]
use crate::app::app::{
    App
};

impl App {

    pub fn draw_modal_tag(&mut self, frame: &mut Frame) {
        
        let length = 60;
        let height = 12;

        let lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled("enter new tag name:", Style::default().fg(self.theme.COLOR_TEXT)),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(""),
            Line::from(""),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled(if self.modal_editor.mode == EditorMode::Normal {"(enter)".to_string()} else { "enter".to_string() }, Style::default().fg(if self.modal_editor.mode == EditorMode::Normal { self.theme.COLOR_GREY_500 } else { self.theme.COLOR_GREY_600 })),
            ]),
        ];
            
        let bg_block = Block::default().style(Style::default().fg(self.theme.COLOR_BORDER));
        bg_block.render(frame.area(), frame.buffer_mut());

        // Modal size (smaller than area)
        let modal_width = length.min((frame.area().width as f32 * 0.8) as usize) as u16;
        let modal_height = height.min((frame.area().height as f32 * 0.6) as usize) as u16;
        let x = frame.area().x + (frame.area().width - modal_width) / 2;
        let y = frame.area().y + (frame.area().height - modal_height) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);

        frame.render_widget(Clear, modal_area);
        
        // Modal block
        let modal_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.COLOR_GREY_600))
            .title(Span::styled(if self.modal_editor.mode == EditorMode::Normal {" (esc) "} else { "─ esc ─" }, Style::default().fg(if self.modal_editor.mode == EditorMode::Normal { self.theme.COLOR_GREY_500 } else { self.theme.COLOR_GREY_600 })))
            .title_alignment(Alignment::Right)
            .padding(Padding { left: 3, right: 3, top: 1, bottom: 1})
            .border_type(ratatui::widgets::BorderType::Rounded);

        // Modal content
        let paragraph = Paragraph::new(Text::from(lines))
            .block(modal_block)
            .alignment(Alignment::Center);
        
        // Render the paragraph
        paragraph.render(modal_area, frame.buffer_mut());

        let custom_theme = EditorTheme {
            base: Style::default().fg(self.theme.COLOR_GREY_500),
            cursor_style: Style::default().bg(self.theme.COLOR_TEXT),
            selection_style: Style::default(),
            block: Some(
                Block::default()
                    .padding(Padding { left: 1, right: 1, top: 0, bottom: 0})
                    .borders(Borders::TOP)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(self.theme.COLOR_GREY_800))),
            status_line: Some(EditorStatusLine::default()
                .style_text(Style::default().fg(self.theme.COLOR_TEXT))
                .style_line(Style::default().fg(self.theme.COLOR_GREY_800))
                .align_left(true))
        };
        let editor_view = EditorView::new(&mut self.modal_editor).theme(custom_theme);
        
        let input_area = Rect {
            x: modal_area.x + modal_area.width / 2 - 29,
            y: modal_area.y + 4,
            width: 58,
            height: 4,
        };

        // Render the editor in the modal area
        editor_view.render(input_area, frame.buffer_mut());
        
        // Modal block
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(self.theme.COLOR_GREY_800))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .render(Rect {
            x: modal_area.x + 1,
            y: modal_area.y + 7,
            width: 2,
            height: 1,
        }, frame.buffer_mut());

        // Modal block
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(self.theme.COLOR_GREY_800))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .render(Rect {
            x: modal_area.x + 11,
            y: modal_area.y + 7,
            width: modal_width - 12,
            height: 1,
        }, frame.buffer_mut());

    }
}
