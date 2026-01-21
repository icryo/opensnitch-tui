//! Firewall rule editor dialog

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::models::{FwRule, Expression, Statement, StatementValue};
use crate::ui::layout::DialogLayout;
use crate::ui::theme::Theme;

/// Editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FwEditorMode {
    Create,
    Edit,
}

/// Which field is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FwEditorFocus {
    Description,
    Target,
    Enabled,
    Protocol,
    SourceIp,
    SourcePort,
    DestIp,
    DestPort,
}

impl FwEditorFocus {
    fn next(self) -> Self {
        match self {
            Self::Description => Self::Target,
            Self::Target => Self::Enabled,
            Self::Enabled => Self::Protocol,
            Self::Protocol => Self::SourceIp,
            Self::SourceIp => Self::SourcePort,
            Self::SourcePort => Self::DestIp,
            Self::DestIp => Self::DestPort,
            Self::DestPort => Self::Description,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Description => Self::DestPort,
            Self::Target => Self::Description,
            Self::Enabled => Self::Target,
            Self::Protocol => Self::Enabled,
            Self::SourceIp => Self::Protocol,
            Self::SourcePort => Self::SourceIp,
            Self::DestIp => Self::SourcePort,
            Self::DestPort => Self::DestIp,
        }
    }
}

/// Firewall rule editor result
pub enum FwRuleEditorResult {
    Save(FwRule),
    Cancel,
}

/// Firewall rule editor dialog
pub struct FwRuleEditorDialog {
    pub mode: FwEditorMode,
    pub focus: FwEditorFocus,
    pub editing_text: bool,

    // Rule fields
    pub description: String,
    pub target: String,
    pub enabled: bool,
    pub protocol: String,
    pub source_ip: String,
    pub source_port: String,
    pub dest_ip: String,
    pub dest_port: String,

    // Original UUID for edits
    pub original_uuid: Option<String>,
    pub position: u64,

    cursor_pos: usize,
}

impl FwRuleEditorDialog {
    pub fn new() -> Self {
        Self {
            mode: FwEditorMode::Create,
            focus: FwEditorFocus::Description,
            editing_text: false,
            description: String::new(),
            target: "ACCEPT".to_string(),
            enabled: true,
            protocol: String::new(),
            source_ip: String::new(),
            source_port: String::new(),
            dest_ip: String::new(),
            dest_port: String::new(),
            original_uuid: None,
            position: 0,
            cursor_pos: 0,
        }
    }

    pub fn edit(rule: &FwRule) -> Self {
        // Extract values from expressions
        let mut protocol = String::new();
        let mut source_ip = String::new();
        let mut source_port = String::new();
        let mut dest_ip = String::new();
        let mut dest_port = String::new();

        for expr in &rule.expressions {
            let stmt = &expr.statement;
            match stmt.name.as_str() {
                "protocol" => {
                    if let Some(v) = stmt.values.first() {
                        protocol = v.value.clone();
                    }
                }
                "saddr" => {
                    if let Some(v) = stmt.values.first() {
                        source_ip = v.value.clone();
                    }
                }
                "sport" => {
                    if let Some(v) = stmt.values.first() {
                        source_port = v.value.clone();
                    }
                }
                "daddr" => {
                    if let Some(v) = stmt.values.first() {
                        dest_ip = v.value.clone();
                    }
                }
                "dport" => {
                    if let Some(v) = stmt.values.first() {
                        dest_port = v.value.clone();
                    }
                }
                _ => {}
            }
        }

        Self {
            mode: FwEditorMode::Edit,
            focus: FwEditorFocus::Description,
            editing_text: false,
            description: rule.description.clone(),
            target: rule.target.clone(),
            enabled: rule.enabled,
            protocol,
            source_ip,
            source_port,
            dest_ip,
            dest_port,
            original_uuid: Some(rule.uuid.clone()),
            position: rule.position,
            cursor_pos: 0,
        }
    }

    pub fn build_rule(&self) -> FwRule {
        let mut expressions = Vec::new();

        // Add protocol expression if set
        if !self.protocol.is_empty() {
            expressions.push(Expression {
                statement: Statement {
                    op: "==".to_string(),
                    name: "protocol".to_string(),
                    values: vec![StatementValue {
                        key: "value".to_string(),
                        value: self.protocol.clone(),
                    }],
                },
            });
        }

        // Add source IP if set
        if !self.source_ip.is_empty() {
            expressions.push(Expression {
                statement: Statement {
                    op: "==".to_string(),
                    name: "saddr".to_string(),
                    values: vec![StatementValue {
                        key: "value".to_string(),
                        value: self.source_ip.clone(),
                    }],
                },
            });
        }

        // Add source port if set
        if !self.source_port.is_empty() {
            expressions.push(Expression {
                statement: Statement {
                    op: "==".to_string(),
                    name: "sport".to_string(),
                    values: vec![StatementValue {
                        key: "value".to_string(),
                        value: self.source_port.clone(),
                    }],
                },
            });
        }

        // Add dest IP if set
        if !self.dest_ip.is_empty() {
            expressions.push(Expression {
                statement: Statement {
                    op: "==".to_string(),
                    name: "daddr".to_string(),
                    values: vec![StatementValue {
                        key: "value".to_string(),
                        value: self.dest_ip.clone(),
                    }],
                },
            });
        }

        // Add dest port if set
        if !self.dest_port.is_empty() {
            expressions.push(Expression {
                statement: Statement {
                    op: "==".to_string(),
                    name: "dport".to_string(),
                    values: vec![StatementValue {
                        key: "value".to_string(),
                        value: self.dest_port.clone(),
                    }],
                },
            });
        }

        FwRule {
            uuid: self.original_uuid.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            enabled: self.enabled,
            position: self.position,
            description: self.description.clone(),
            target: self.target.clone(),
            expressions,
            ..Default::default()
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<FwRuleEditorResult> {
        if self.editing_text {
            return self.handle_text_input(key);
        }

        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                self.focus = self.focus.next();
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.focus = self.focus.prev();
            }
            KeyCode::Enter => {
                match self.focus {
                    FwEditorFocus::Enabled => self.enabled = !self.enabled,
                    FwEditorFocus::Target => self.cycle_target(true),
                    _ => {
                        self.editing_text = true;
                        self.cursor_pos = self.current_text().len();
                    }
                }
            }
            KeyCode::Left | KeyCode::Right => {
                if self.focus == FwEditorFocus::Target {
                    self.cycle_target(key.code == KeyCode::Right);
                }
            }
            KeyCode::Char(' ') => {
                match self.focus {
                    FwEditorFocus::Enabled => self.enabled = !self.enabled,
                    FwEditorFocus::Target => self.cycle_target(true),
                    _ => {}
                }
            }
            KeyCode::Esc => {
                return Some(FwRuleEditorResult::Cancel);
            }
            KeyCode::F(2) => {
                // Save
                return Some(FwRuleEditorResult::Save(self.build_rule()));
            }
            KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                return Some(FwRuleEditorResult::Save(self.build_rule()));
            }
            _ => {}
        }
        None
    }

    fn handle_text_input(&mut self, key: KeyEvent) -> Option<FwRuleEditorResult> {
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
            FwEditorFocus::Description => &self.description,
            FwEditorFocus::Protocol => &self.protocol,
            FwEditorFocus::SourceIp => &self.source_ip,
            FwEditorFocus::SourcePort => &self.source_port,
            FwEditorFocus::DestIp => &self.dest_ip,
            FwEditorFocus::DestPort => &self.dest_port,
            _ => "",
        }
    }

    fn current_text_mut(&mut self) -> &mut String {
        match self.focus {
            FwEditorFocus::Description => &mut self.description,
            FwEditorFocus::Protocol => &mut self.protocol,
            FwEditorFocus::SourceIp => &mut self.source_ip,
            FwEditorFocus::SourcePort => &mut self.source_port,
            FwEditorFocus::DestIp => &mut self.dest_ip,
            FwEditorFocus::DestPort => &mut self.dest_port,
            _ => &mut self.description,
        }
    }

    fn cycle_target(&mut self, forward: bool) {
        let targets = ["ACCEPT", "DROP", "REJECT"];
        let current = targets.iter().position(|t| t.eq_ignore_ascii_case(&self.target)).unwrap_or(0);
        let new_idx = if forward {
            (current + 1) % targets.len()
        } else {
            if current == 0 { targets.len() - 1 } else { current - 1 }
        };
        self.target = targets[new_idx].to_string();
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        let area = frame.area();
        let dialog_area = DialogLayout::centered(area, 65, 18).dialog;

        frame.render_widget(Clear, dialog_area);

        let title = match self.mode {
            FwEditorMode::Create => " Create Firewall Rule ",
            FwEditorMode::Edit => " Edit Firewall Rule ",
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(theme.border_focused())
            .style(theme.normal());

        frame.render_widget(block.clone(), dialog_area);

        let inner = block.inner(dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Description
                Constraint::Length(1), // Target
                Constraint::Length(1), // Enabled
                Constraint::Length(1), // Separator
                Constraint::Length(1), // Protocol
                Constraint::Length(1), // Source IP
                Constraint::Length(1), // Source Port
                Constraint::Length(1), // Dest IP
                Constraint::Length(1), // Dest Port
                Constraint::Length(1), // Separator
                Constraint::Min(1),    // Hints
            ])
            .split(inner);

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

            let text = format!("{:14} {}", format!("{}:", label), value);
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
            let text = format!("{:14} {}", format!("{}:", label), checkbox);
            let para = Paragraph::new(text).style(style);
            frame.render_widget(para, area);
        };

        render_field(frame, chunks[0], "Description", &self.description,
            self.focus == FwEditorFocus::Description, self.editing_text && self.focus == FwEditorFocus::Description);

        let target_style = match self.target.to_uppercase().as_str() {
            "ACCEPT" => Style::default().fg(Color::Green),
            "DROP" => Style::default().fg(Color::Red),
            "REJECT" => Style::default().fg(Color::Magenta),
            _ => theme.normal(),
        };
        let target_focused = self.focus == FwEditorFocus::Target;
        let target_display = format!("◄ {} ►", self.target);
        let target_text = format!("{:14} {}", "Target:", target_display);
        let target_final_style = if target_focused {
            target_style.add_modifier(Modifier::REVERSED)
        } else {
            target_style
        };
        frame.render_widget(Paragraph::new(target_text).style(target_final_style), chunks[1]);

        render_toggle(frame, chunks[2], "Enabled", self.enabled, self.focus == FwEditorFocus::Enabled);

        frame.render_widget(Paragraph::new("─".repeat(55)).style(theme.dim()), chunks[3]);

        render_field(frame, chunks[4], "Protocol", &self.protocol,
            self.focus == FwEditorFocus::Protocol, self.editing_text && self.focus == FwEditorFocus::Protocol);
        render_field(frame, chunks[5], "Source IP", &self.source_ip,
            self.focus == FwEditorFocus::SourceIp, self.editing_text && self.focus == FwEditorFocus::SourceIp);
        render_field(frame, chunks[6], "Source Port", &self.source_port,
            self.focus == FwEditorFocus::SourcePort, self.editing_text && self.focus == FwEditorFocus::SourcePort);
        render_field(frame, chunks[7], "Dest IP", &self.dest_ip,
            self.focus == FwEditorFocus::DestIp, self.editing_text && self.focus == FwEditorFocus::DestIp);
        render_field(frame, chunks[8], "Dest Port", &self.dest_port,
            self.focus == FwEditorFocus::DestPort, self.editing_text && self.focus == FwEditorFocus::DestPort);

        frame.render_widget(Paragraph::new("─".repeat(55)).style(theme.dim()), chunks[9]);

        let hints = if self.editing_text {
            "Enter/Esc=done  ←→=cursor  Backspace=delete"
        } else {
            "Tab/↑↓=navigate  Enter=edit  ←→/Space=change  F2/Ctrl+S=save  Esc=cancel"
        };
        let hint_para = Paragraph::new(hints)
            .style(theme.dim())
            .wrap(Wrap { trim: true });
        frame.render_widget(hint_para, chunks[10]);
    }
}
