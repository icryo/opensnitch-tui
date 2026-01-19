//! Status bar widget

use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Status bar item
pub struct StatusItem {
    pub label: String,
    pub value: String,
    pub style: Style,
}

impl StatusItem {
    pub fn new(label: &str, value: &str) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            style: Style::default(),
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

/// Build a status bar line from items
pub fn build_status_line(items: Vec<StatusItem>, separator: &str) -> Line<'static> {
    let mut spans = Vec::new();

    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(format!(" {} ", separator)));
        }

        if !item.label.is_empty() {
            spans.push(Span::raw(format!("{}: ", item.label)));
        }
        spans.push(Span::styled(item.value, item.style));
    }

    Line::from(spans)
}
