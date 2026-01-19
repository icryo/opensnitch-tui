//! Search/filter bar widget

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Search bar state
pub struct SearchBar {
    pub query: String,
    pub active: bool,
    pub cursor_pos: usize,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            active: false,
            cursor_pos: 0,
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.cursor_pos = self.query.len();
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.cursor_pos = 0;
    }

    pub fn insert(&mut self, c: char) {
        self.query.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.query.remove(self.cursor_pos);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor_pos < self.query.len() {
            self.query.remove(self.cursor_pos);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_pos < self.query.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_pos = self.query.len();
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, style: Style, focused_style: Style) {
        let border_style = if self.active { focused_style } else { style };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Filter (/ to edit, Esc to clear) ");

        let display_text = if self.query.is_empty() && !self.active {
            "Type to filter...".to_string()
        } else {
            self.query.clone()
        };

        let paragraph = Paragraph::new(display_text)
            .block(block)
            .style(style);

        frame.render_widget(paragraph, area);

        // Show cursor if active
        if self.active {
            frame.set_cursor_position((
                area.x + 1 + self.cursor_pos as u16,
                area.y + 1,
            ));
        }
    }
}

impl Default for SearchBar {
    fn default() -> Self {
        Self::new()
    }
}
