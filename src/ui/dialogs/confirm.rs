//! Confirmation dialog

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::layout::DialogLayout;
use crate::ui::theme::Theme;

pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub cancel_label: String,
    pub selected: bool, // true = confirm selected
    pub result: Option<bool>,
}

impl ConfirmDialog {
    pub fn new(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            confirm_label: "Yes".to_string(),
            cancel_label: "No".to_string(),
            selected: false,
            result: None,
        }
    }

    pub fn with_labels(mut self, confirm: &str, cancel: &str) -> Self {
        self.confirm_label = confirm.to_string();
        self.cancel_label = cancel.to_string();
        self
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                self.selected = !self.selected;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.result = Some(true);
                return true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.result = Some(false);
                return true;
            }
            KeyCode::Enter => {
                self.result = Some(self.selected);
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        let area = frame.area();
        let dialog_area = DialogLayout::centered(area, 50, 10).dialog;

        // Clear background
        frame.render_widget(Clear, dialog_area);

        // Block
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(theme.border_focused());

        frame.render_widget(block.clone(), dialog_area);

        let inner = block.inner(dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(3),    // Message
                Constraint::Length(1), // Buttons
            ])
            .split(inner);

        // Message
        let message = Paragraph::new(self.message.clone())
            .style(theme.normal());
        frame.render_widget(message, chunks[0]);

        // Buttons
        let yes_style = if self.selected {
            theme.accent().add_modifier(Modifier::BOLD)
        } else {
            theme.dim()
        };
        let no_style = if !self.selected {
            theme.accent().add_modifier(Modifier::BOLD)
        } else {
            theme.dim()
        };

        let buttons = Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("[ {} ]", self.confirm_label), yes_style),
            Span::raw("    "),
            Span::styled(format!("[ {} ]", self.cancel_label), no_style),
        ]);

        let button_para = Paragraph::new(buttons);
        frame.render_widget(button_para, chunks[1]);
    }
}
