//! Rule editor dialog for creating and editing rules

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::models::{Operator, OperatorType, Rule, RuleAction, RuleDuration};
use crate::ui::layout::DialogLayout;
use crate::ui::theme::Theme;

/// Editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Create,
    Edit,
}

/// Which field is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorFocus {
    Name,
    Description,
    Action,
    Duration,
    OperatorType,
    Operand,
    Data,
    Enabled,
    Precedence,
    NoLog,
}

impl EditorFocus {
    fn next(self) -> Self {
        match self {
            Self::Name => Self::Description,
            Self::Description => Self::Action,
            Self::Action => Self::Duration,
            Self::Duration => Self::OperatorType,
            Self::OperatorType => Self::Operand,
            Self::Operand => Self::Data,
            Self::Data => Self::Enabled,
            Self::Enabled => Self::Precedence,
            Self::Precedence => Self::NoLog,
            Self::NoLog => Self::Name,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Name => Self::NoLog,
            Self::Description => Self::Name,
            Self::Action => Self::Description,
            Self::Duration => Self::Action,
            Self::OperatorType => Self::Duration,
            Self::Operand => Self::OperatorType,
            Self::Data => Self::Operand,
            Self::Enabled => Self::Data,
            Self::Precedence => Self::Enabled,
            Self::NoLog => Self::Precedence,
        }
    }
}

/// Rule editor dialog
pub struct RuleEditorDialog {
    pub mode: EditorMode,
    pub focus: EditorFocus,
    pub editing_text: bool,

    // Rule fields
    pub name: String,
    pub description: String,
    pub action: RuleAction,
    pub duration: RuleDuration,
    pub operator_type: OperatorType,
    pub operand: String,
    pub data: String,
    pub enabled: bool,
    pub precedence: bool,
    pub nolog: bool,

    // Original name for edits (public for checking if new rule)
    pub original_name: Option<String>,

    // Cursor position for text editing
    cursor_pos: usize,
}

impl RuleEditorDialog {
    /// Create new rule editor for creating a rule
    pub fn new() -> Self {
        Self {
            mode: EditorMode::Create,
            focus: EditorFocus::Name,
            editing_text: false,
            name: String::new(),
            description: String::new(),
            action: RuleAction::Allow,
            duration: RuleDuration::Always,
            operator_type: OperatorType::Simple,
            operand: "process.path".to_string(),
            data: String::new(),
            enabled: true,
            precedence: false,
            nolog: false,
            original_name: None,
            cursor_pos: 0,
        }
    }

    /// Create editor for editing an existing rule
    pub fn edit(rule: &Rule) -> Self {
        Self {
            mode: EditorMode::Edit,
            focus: EditorFocus::Name,
            editing_text: false,
            name: rule.name.clone(),
            description: rule.description.clone(),
            action: rule.action,
            duration: rule.duration.clone(),
            operator_type: rule.operator.op_type.clone(),
            operand: rule.operator.operand.clone(),
            data: rule.operator.data.clone(),
            enabled: rule.enabled,
            precedence: rule.precedence,
            nolog: rule.nolog,
            original_name: Some(rule.name.clone()),
            cursor_pos: rule.name.len(),
        }
    }

    /// Build rule from current state
    pub fn build_rule(&self) -> Rule {
        let operator = Operator {
            op_type: self.operator_type.clone(),
            operand: self.operand.clone(),
            data: self.data.clone(),
            sensitive: false,
            list: Vec::new(),
        };

        let mut rule = Rule::new(&self.name, self.action, self.duration.clone(), operator);
        rule.description = self.description.clone();
        rule.enabled = self.enabled;
        rule.precedence = self.precedence;
        rule.nolog = self.nolog;
        rule
    }

    /// Handle key event, returns true if dialog should close
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<RuleEditorResult> {
        if self.editing_text {
            return self.handle_text_input(key);
        }

        match key.code {
            KeyCode::Tab => {
                self.focus = self.focus.next();
            }
            KeyCode::BackTab => {
                self.focus = self.focus.prev();
            }
            KeyCode::Up => {
                self.focus = self.focus.prev();
            }
            KeyCode::Down => {
                self.focus = self.focus.next();
            }
            KeyCode::Enter => {
                match self.focus {
                    EditorFocus::Name | EditorFocus::Description |
                    EditorFocus::Operand | EditorFocus::Data => {
                        self.editing_text = true;
                        self.cursor_pos = self.current_text().len();
                    }
                    EditorFocus::Enabled => self.enabled = !self.enabled,
                    EditorFocus::Precedence => self.precedence = !self.precedence,
                    EditorFocus::NoLog => self.nolog = !self.nolog,
                    _ => {}
                }
            }
            KeyCode::Left | KeyCode::Right => {
                self.cycle_option(key.code == KeyCode::Right);
            }
            KeyCode::Char(' ') => {
                match self.focus {
                    EditorFocus::Enabled => self.enabled = !self.enabled,
                    EditorFocus::Precedence => self.precedence = !self.precedence,
                    EditorFocus::NoLog => self.nolog = !self.nolog,
                    _ => self.cycle_option(true),
                }
            }
            KeyCode::Esc => {
                return Some(RuleEditorResult::Cancel);
            }
            KeyCode::F(2) | KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                // Save
                if !self.name.is_empty() && !self.data.is_empty() {
                    return Some(RuleEditorResult::Save(self.build_rule()));
                }
            }
            _ => {}
        }
        None
    }

    fn handle_text_input(&mut self, key: KeyEvent) -> Option<RuleEditorResult> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.editing_text = false;
            }
            KeyCode::Char(c) => {
                let cursor = self.cursor_pos;
                let text = self.current_text_mut();
                if cursor <= text.len() {
                    text.insert(cursor, c);
                    self.cursor_pos = cursor + 1;
                }
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    let cursor = self.cursor_pos;
                    let text = self.current_text_mut();
                    text.remove(cursor);
                }
            }
            KeyCode::Delete => {
                let cursor = self.cursor_pos;
                let text = self.current_text_mut();
                if cursor < text.len() {
                    text.remove(cursor);
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                let len = self.current_text().len();
                if self.cursor_pos < len {
                    self.cursor_pos += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
            }
            KeyCode::End => {
                self.cursor_pos = self.current_text().len();
            }
            _ => {}
        }
        None
    }

    fn current_text(&self) -> &str {
        match self.focus {
            EditorFocus::Name => &self.name,
            EditorFocus::Description => &self.description,
            EditorFocus::Operand => &self.operand,
            EditorFocus::Data => &self.data,
            _ => "",
        }
    }

    fn current_text_mut(&mut self) -> &mut String {
        match self.focus {
            EditorFocus::Name => &mut self.name,
            EditorFocus::Description => &mut self.description,
            EditorFocus::Operand => &mut self.operand,
            EditorFocus::Data => &mut self.data,
            _ => &mut self.name, // Fallback
        }
    }

    fn cycle_option(&mut self, forward: bool) {
        match self.focus {
            EditorFocus::Action => {
                self.action = if forward {
                    match self.action {
                        RuleAction::Allow => RuleAction::Deny,
                        RuleAction::Deny => RuleAction::Reject,
                        RuleAction::Reject => RuleAction::Allow,
                    }
                } else {
                    match self.action {
                        RuleAction::Allow => RuleAction::Reject,
                        RuleAction::Deny => RuleAction::Allow,
                        RuleAction::Reject => RuleAction::Deny,
                    }
                };
            }
            EditorFocus::Duration => {
                let durations = [
                    RuleDuration::Once,
                    RuleDuration::UntilRestart,
                    RuleDuration::Always,
                    RuleDuration::FiveMinutes,
                    RuleDuration::FifteenMinutes,
                    RuleDuration::ThirtyMinutes,
                    RuleDuration::OneHour,
                    RuleDuration::TwelveHours,
                    RuleDuration::TwentyFourHours,
                ];
                let current = durations.iter().position(|d| d == &self.duration).unwrap_or(0);
                let new_idx = if forward {
                    (current + 1) % durations.len()
                } else {
                    if current == 0 { durations.len() - 1 } else { current - 1 }
                };
                self.duration = durations[new_idx].clone();
            }
            EditorFocus::OperatorType => {
                let types = [
                    OperatorType::Simple,
                    OperatorType::Regexp,
                    OperatorType::Network,
                    OperatorType::Lists,
                ];
                let current = types.iter().position(|t| t == &self.operator_type).unwrap_or(0);
                let new_idx = if forward {
                    (current + 1) % types.len()
                } else {
                    if current == 0 { types.len() - 1 } else { current - 1 }
                };
                self.operator_type = types[new_idx].clone();
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        let area = frame.area();
        let dialog_area = DialogLayout::centered(area, 70, 24).dialog;

        // Clear background
        frame.render_widget(Clear, dialog_area);

        // Title
        let title = match self.mode {
            EditorMode::Create => " Create Rule ",
            EditorMode::Edit => " Edit Rule ",
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(theme.border_focused())
            .style(theme.normal());

        frame.render_widget(block.clone(), dialog_area);

        let inner = block.inner(dialog_area);

        // Layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Name
                Constraint::Length(1), // Description
                Constraint::Length(1), // Action
                Constraint::Length(1), // Duration
                Constraint::Length(1), // Separator
                Constraint::Length(1), // Operator type
                Constraint::Length(1), // Operand
                Constraint::Length(1), // Data
                Constraint::Length(1), // Separator
                Constraint::Length(1), // Enabled
                Constraint::Length(1), // Precedence
                Constraint::Length(1), // NoLog
                Constraint::Length(1), // Separator
                Constraint::Min(1),    // Hints
            ])
            .split(inner);

        // Helper to render a field
        let render_field = |frame: &mut Frame, area: ratatui::layout::Rect, label: &str, value: &str, focused: bool, editing: bool| {
            let style = if focused {
                if editing {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED)
                } else {
                    Style::default().add_modifier(Modifier::REVERSED)
                }
            } else {
                theme.normal()
            };

            let text = format!("{:15} {}", format!("{}:", label), value);
            let para = Paragraph::new(text).style(style);
            frame.render_widget(para, area);
        };

        let render_toggle = |frame: &mut Frame, area: ratatui::layout::Rect, label: &str, value: bool, focused: bool| {
            let checkbox = if value { "[x]" } else { "[ ]" };
            let style = if focused {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                theme.normal()
            };
            let text = format!("{:15} {}", format!("{}:", label), checkbox);
            let para = Paragraph::new(text).style(style);
            frame.render_widget(para, area);
        };

        // Render fields
        render_field(frame, chunks[0], "Name", &self.name,
            self.focus == EditorFocus::Name, self.editing_text && self.focus == EditorFocus::Name);
        render_field(frame, chunks[1], "Description", &self.description,
            self.focus == EditorFocus::Description, self.editing_text && self.focus == EditorFocus::Description);
        render_field(frame, chunks[2], "Action", &format!("◄ {} ►", self.action),
            self.focus == EditorFocus::Action, false);
        render_field(frame, chunks[3], "Duration", &format!("◄ {} ►", self.duration),
            self.focus == EditorFocus::Duration, false);

        // Separator
        frame.render_widget(Paragraph::new("─".repeat(60)).style(theme.dim()), chunks[4]);

        render_field(frame, chunks[5], "Operator", &format!("◄ {} ►", self.operator_type),
            self.focus == EditorFocus::OperatorType, false);
        render_field(frame, chunks[6], "Operand", &self.operand,
            self.focus == EditorFocus::Operand, self.editing_text && self.focus == EditorFocus::Operand);
        render_field(frame, chunks[7], "Data", &self.data,
            self.focus == EditorFocus::Data, self.editing_text && self.focus == EditorFocus::Data);

        // Separator
        frame.render_widget(Paragraph::new("─".repeat(60)).style(theme.dim()), chunks[8]);

        render_toggle(frame, chunks[9], "Enabled", self.enabled, self.focus == EditorFocus::Enabled);
        render_toggle(frame, chunks[10], "Precedence", self.precedence, self.focus == EditorFocus::Precedence);
        render_toggle(frame, chunks[11], "No Log", self.nolog, self.focus == EditorFocus::NoLog);

        // Separator
        frame.render_widget(Paragraph::new("─".repeat(60)).style(theme.dim()), chunks[12]);

        // Hints
        let hints = if self.editing_text {
            "Enter/Esc=done editing  ←→=move cursor  Backspace=delete"
        } else {
            "Tab/↑↓=navigate  Enter=edit  ←→/Space=change  Ctrl+S=save  Esc=cancel"
        };
        let hint_para = Paragraph::new(hints)
            .style(theme.dim())
            .wrap(Wrap { trim: true });
        frame.render_widget(hint_para, chunks[13]);
    }
}

/// Result of rule editor interaction
pub enum RuleEditorResult {
    Save(Rule),
    Cancel,
}
