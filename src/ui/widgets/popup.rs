//! Popup/modal dialog widget

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Popup dialog
pub struct Popup {
    pub title: String,
    pub content: String,
    pub style: Style,
    pub border_style: Style,
}

impl Popup {
    pub fn new(title: &str, content: &str) -> Self {
        Self {
            title: title.to_string(),
            content: content.to_string(),
            style: Style::default(),
            border_style: Style::default(),
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Clear background
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(self.border_style)
            .style(self.style);

        let paragraph = Paragraph::new(self.content.clone())
            .block(block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}

/// Confirmation dialog
pub struct ConfirmDialog {
    pub message: String,
    pub confirm_label: String,
    pub cancel_label: String,
    pub selected: bool, // true = confirm, false = cancel
}

impl ConfirmDialog {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            confirm_label: "Yes".to_string(),
            cancel_label: "No".to_string(),
            selected: false,
        }
    }

    pub fn with_labels(mut self, confirm: &str, cancel: &str) -> Self {
        self.confirm_label = confirm.to_string();
        self.cancel_label = cancel.to_string();
        self
    }

    pub fn toggle(&mut self) {
        self.selected = !self.selected;
    }

    pub fn confirm(&self) -> bool {
        self.selected
    }
}
