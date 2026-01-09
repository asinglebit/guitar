use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Default)]
pub struct TextInput {
    value: String,
    cursor: usize,
    scroll: usize,
    max_width: usize,
}

impl TextInput {
    pub fn scroll(&self) -> &usize {
        &self.scroll
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn set_max_width(&mut self, max_width: usize) {
        self.max_width = max_width;
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.value.insert(self.cursor, c);
                self.cursor += 1;
            },
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.value.remove(self.cursor);
                }
            },
            KeyCode::Delete => {
                if self.cursor < self.value.len() {
                    self.value.remove(self.cursor);
                }
            },
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
            },
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.value.len());
            },
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.value.len(),
            _ => {},
        }

        // Always update scroll after any key event
        self.update_scroll(self.max_width);
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn update_scroll(&mut self, max_width: usize) {
        // Ensure cursor is visible
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + max_width {
            self.scroll = self.cursor - max_width + 1;
        }
    }
}
