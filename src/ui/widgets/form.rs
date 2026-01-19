//! Form input widgets

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Text input field
pub struct TextInput {
    pub label: String,
    pub value: String,
    pub cursor_pos: usize,
    pub focused: bool,
}

impl TextInput {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            value: String::new(),
            cursor_pos: 0,
            focused: false,
        }
    }

    pub fn with_value(mut self, value: &str) -> Self {
        self.value = value.to_string();
        self.cursor_pos = value.len();
        self
    }

    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.value.remove(self.cursor_pos);
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, style: Style, focused_style: Style) {
        let border_style = if self.focused { focused_style } else { style };

        let block = Block::default()
            .title(format!(" {} ", self.label))
            .borders(Borders::ALL)
            .border_style(border_style);

        let paragraph = Paragraph::new(self.value.clone())
            .block(block)
            .style(style);

        frame.render_widget(paragraph, area);

        if self.focused {
            frame.set_cursor_position((
                area.x + 1 + self.cursor_pos as u16,
                area.y + 1,
            ));
        }
    }
}

/// Select/dropdown field
pub struct SelectInput {
    pub label: String,
    pub options: Vec<String>,
    pub selected: usize,
    pub focused: bool,
}

impl SelectInput {
    pub fn new(label: &str, options: Vec<String>) -> Self {
        Self {
            label: label.to_string(),
            options,
            selected: 0,
            focused: false,
        }
    }

    pub fn next(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + 1) % self.options.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.options.is_empty() {
            self.selected = if self.selected == 0 {
                self.options.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn value(&self) -> Option<&str> {
        self.options.get(self.selected).map(|s| s.as_str())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, style: Style, focused_style: Style) {
        let border_style = if self.focused { focused_style } else { style };

        let block = Block::default()
            .title(format!(" {} ", self.label))
            .borders(Borders::ALL)
            .border_style(border_style);

        let display = self.options.get(self.selected)
            .map(|s| format!("< {} >", s))
            .unwrap_or_else(|| "No options".to_string());

        let paragraph = Paragraph::new(display)
            .block(block)
            .style(style);

        frame.render_widget(paragraph, area);
    }
}

/// Checkbox field
pub struct Checkbox {
    pub label: String,
    pub checked: bool,
    pub focused: bool,
}

impl Checkbox {
    pub fn new(label: &str, checked: bool) -> Self {
        Self {
            label: label.to_string(),
            checked,
            focused: false,
        }
    }

    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, style: Style, focused_style: Style) {
        let display_style = if self.focused { focused_style } else { style };
        let checkbox = if self.checked { "[x]" } else { "[ ]" };
        let text = format!("{} {}", checkbox, self.label);

        let paragraph = Paragraph::new(text).style(display_style);
        frame.render_widget(paragraph, area);
    }
}
