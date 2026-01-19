//! Connection prompt dialog

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};
use tokio::sync::oneshot;

use crate::models::{Connection, Operator, OperatorType, Rule, RuleAction, RuleDuration};
use crate::ui::layout::DialogLayout;
use crate::ui::theme::Theme;

/// Connection prompt dialog state
pub struct PromptDialog {
    pub connection: Connection,
    pub node_addr: String,
    pub response_tx: Option<oneshot::Sender<Rule>>,

    // Selection state
    pub action: RuleAction,
    pub duration: RuleDuration,
    pub focus: PromptFocus,

    // Advanced options
    pub show_advanced: bool,
    pub advanced_focus: usize,
    pub match_dest_host: bool,
    pub match_dest_ip: bool,
    pub match_dest_port: bool,
    pub match_user: bool,
    pub match_checksum: bool,

    // Timeout tracking
    pub created_at: Instant,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptFocus {
    Action,
    Duration,
    Advanced,
}

impl PromptDialog {
    pub fn new(
        connection: Connection,
        node_addr: String,
        response_tx: oneshot::Sender<Rule>,
    ) -> Self {
        Self {
            connection,
            node_addr,
            response_tx: Some(response_tx),
            action: RuleAction::Allow,
            duration: RuleDuration::Once,
            focus: PromptFocus::Action,
            show_advanced: false,
            advanced_focus: 0,
            match_dest_host: true, // Default to matching by executable
            match_dest_ip: false,
            match_dest_port: false,
            match_user: false,
            match_checksum: false,
            created_at: Instant::now(),
            timeout_secs: 15,
        }
    }

    /// Returns remaining seconds until timeout
    pub fn remaining_secs(&self) -> u64 {
        let elapsed = self.created_at.elapsed().as_secs();
        self.timeout_secs.saturating_sub(elapsed)
    }

    /// Returns timeout progress as a ratio (0.0 to 1.0)
    pub fn timeout_ratio(&self) -> f64 {
        let elapsed = self.created_at.elapsed().as_secs_f64();
        1.0 - (elapsed / self.timeout_secs as f64).min(1.0)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            // Quick action keys
            KeyCode::Char('a') => {
                self.action = RuleAction::Allow;
                return self.confirm();
            }
            KeyCode::Char('d') => {
                self.action = RuleAction::Deny;
                return self.confirm();
            }
            KeyCode::Char('r') => {
                self.action = RuleAction::Reject;
                return self.confirm();
            }

            // Navigation
            KeyCode::Tab => {
                self.focus = match self.focus {
                    PromptFocus::Action => PromptFocus::Duration,
                    PromptFocus::Duration => {
                        if self.show_advanced {
                            PromptFocus::Advanced
                        } else {
                            PromptFocus::Action
                        }
                    }
                    PromptFocus::Advanced => PromptFocus::Action,
                };
            }
            KeyCode::BackTab => {
                self.focus = match self.focus {
                    PromptFocus::Action => {
                        if self.show_advanced {
                            PromptFocus::Advanced
                        } else {
                            PromptFocus::Duration
                        }
                    }
                    PromptFocus::Duration => PromptFocus::Action,
                    PromptFocus::Advanced => PromptFocus::Duration,
                };
            }

            // Left/Right to change selection
            KeyCode::Left | KeyCode::Right => {
                match self.focus {
                    PromptFocus::Action => {
                        self.action = match (key.code, self.action) {
                            (KeyCode::Left, RuleAction::Allow) => RuleAction::Reject,
                            (KeyCode::Left, RuleAction::Deny) => RuleAction::Allow,
                            (KeyCode::Left, RuleAction::Reject) => RuleAction::Deny,
                            (KeyCode::Right, RuleAction::Allow) => RuleAction::Deny,
                            (KeyCode::Right, RuleAction::Deny) => RuleAction::Reject,
                            (KeyCode::Right, RuleAction::Reject) => RuleAction::Allow,
                            _ => self.action,
                        };
                    }
                    PromptFocus::Duration => {
                        let durations = [
                            RuleDuration::Once,
                            RuleDuration::UntilRestart,
                            RuleDuration::Always,
                            RuleDuration::FiveMinutes,
                            RuleDuration::FifteenMinutes,
                            RuleDuration::ThirtyMinutes,
                            RuleDuration::OneHour,
                        ];
                        let current = durations.iter().position(|d| d == &self.duration).unwrap_or(0);
                        let new_idx = if key.code == KeyCode::Left {
                            if current == 0 { durations.len() - 1 } else { current - 1 }
                        } else {
                            (current + 1) % durations.len()
                        };
                        self.duration = durations[new_idx].clone();
                    }
                    PromptFocus::Advanced => {}
                }
            }

            // Up/Down for advanced options
            KeyCode::Up if self.focus == PromptFocus::Advanced => {
                if self.advanced_focus > 0 {
                    self.advanced_focus -= 1;
                } else {
                    self.advanced_focus = 4; // 5 options (0-4)
                }
            }
            KeyCode::Down if self.focus == PromptFocus::Advanced => {
                self.advanced_focus = (self.advanced_focus + 1) % 5;
            }

            // Space to toggle advanced option or show advanced
            KeyCode::Char(' ') => {
                if self.focus == PromptFocus::Advanced {
                    // Toggle current advanced option
                    match self.advanced_focus {
                        0 => self.match_dest_host = !self.match_dest_host,
                        1 => self.match_dest_ip = !self.match_dest_ip,
                        2 => self.match_dest_port = !self.match_dest_port,
                        3 => self.match_user = !self.match_user,
                        4 => self.match_checksum = !self.match_checksum,
                        _ => {}
                    }
                } else {
                    self.show_advanced = !self.show_advanced;
                    if self.show_advanced {
                        self.focus = PromptFocus::Advanced;
                    }
                }
            }

            // Enter to confirm
            KeyCode::Enter => {
                return self.confirm();
            }

            // Escape to cancel (use default action)
            KeyCode::Esc => {
                return self.cancel();
            }

            _ => {}
        }

        false
    }

    fn confirm(&mut self) -> bool {
        if let Some(tx) = self.response_tx.take() {
            let rule = self.create_rule();
            let _ = tx.send(rule);
        }
        true
    }

    fn cancel(&mut self) -> bool {
        // Send default allow rule
        if let Some(tx) = self.response_tx.take() {
            let mut rule = self.create_rule();
            rule.action = RuleAction::Allow;
            rule.duration = RuleDuration::Once;
            let _ = tx.send(rule);
        }
        true
    }

    fn create_rule(&self) -> Rule {
        // Generate rule name based on process and destination
        let name = format!(
            "{}-{}",
            self.connection.process_name(),
            if !self.connection.dst_host.is_empty() {
                self.connection.dst_host.split('.').next().unwrap_or("unknown")
            } else {
                &self.connection.dst_ip
            }
        );

        // Build operators based on selected options
        let mut operators = Vec::new();

        // Always include process path as base
        operators.push(Operator::simple("process.path", &self.connection.process_path));

        // Add optional matchers
        if self.match_dest_host && !self.connection.dst_host.is_empty() {
            operators.push(Operator::simple("dest.host", &self.connection.dst_host));
        }

        if self.match_dest_ip && !self.connection.dst_ip.is_empty() {
            operators.push(Operator::simple("dest.ip", &self.connection.dst_ip));
        }

        if self.match_dest_port {
            operators.push(Operator::simple("dest.port", &self.connection.dst_port.to_string()));
        }

        if self.match_user {
            operators.push(Operator::simple("user.id", &self.connection.user_id.to_string()));
        }

        if self.match_checksum {
            if let Some(md5) = self.connection.process_checksums.get("md5") {
                operators.push(Operator::simple("process.hash.md5", md5));
            }
        }

        // If only one operator, use it directly; otherwise combine with list
        let operator = if operators.len() == 1 {
            operators.remove(0)
        } else {
            Operator {
                op_type: OperatorType::List,
                operand: "list".to_string(),
                data: String::new(),
                sensitive: false,
                list: operators,
            }
        };

        Rule::new(&name, self.action, self.duration.clone(), operator)
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        let area = frame.area();
        let height = if self.show_advanced { 28 } else { 22 };
        let dialog_area = DialogLayout::centered(area, 62, height).dialog;

        // Clear background
        frame.render_widget(Clear, dialog_area);

        // Main block
        let remaining = self.remaining_secs();
        let title = format!(" New Connection ({remaining}s) ");
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(theme.border_focused())
            .style(theme.normal());

        frame.render_widget(block.clone(), dialog_area);

        let inner = block.inner(dialog_area);

        // Layout - dynamic based on advanced options
        let constraints = if self.show_advanced {
            vec![
                Constraint::Length(5), // Connection info
                Constraint::Length(3), // Action
                Constraint::Length(3), // Duration
                Constraint::Length(7), // Advanced options
                Constraint::Length(2), // Timeout bar
                Constraint::Min(1),    // Hints
            ]
        } else {
            vec![
                Constraint::Length(5), // Connection info
                Constraint::Length(3), // Action
                Constraint::Length(3), // Duration
                Constraint::Length(2), // Timeout bar
                Constraint::Min(1),    // Hints
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(constraints)
            .split(inner);

        // Connection info
        let info_lines = vec![
            Line::from(vec![
                Span::styled(
                    self.connection.process_name(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" wants to connect to:"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Destination: "),
                Span::styled(
                    self.connection.destination(),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(format!(" ({})", self.connection.protocol)),
            ]),
            Line::from(vec![
                Span::raw("  Process: "),
                Span::styled(&self.connection.process_path, theme.dim()),
            ]),
            Line::from(vec![
                Span::raw("  User: "),
                Span::raw(format!("UID {} | PID {}", self.connection.user_id, self.connection.process_id)),
            ]),
        ];

        let info = Paragraph::new(info_lines);
        frame.render_widget(info, chunks[0]);

        // Action selection
        let action_focused = self.focus == PromptFocus::Action;
        let action_block = Block::default()
            .title(" Action ")
            .borders(Borders::ALL)
            .border_style(if action_focused {
                theme.border_focused()
            } else {
                theme.border()
            });

        let action_spans = vec![
            Span::raw("  "),
            if self.action == RuleAction::Allow {
                Span::styled("[a] ALLOW", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(" a  allow", theme.dim())
            },
            Span::raw("  "),
            if self.action == RuleAction::Deny {
                Span::styled("[d] DENY", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(" d  deny", theme.dim())
            },
            Span::raw("  "),
            if self.action == RuleAction::Reject {
                Span::styled("[r] REJECT", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(" r  reject", theme.dim())
            },
        ];

        let action_para = Paragraph::new(Line::from(action_spans))
            .block(action_block);
        frame.render_widget(action_para, chunks[1]);

        // Duration selection
        let duration_focused = self.focus == PromptFocus::Duration;
        let duration_block = Block::default()
            .title(" Duration ")
            .borders(Borders::ALL)
            .border_style(if duration_focused {
                theme.border_focused()
            } else {
                theme.border()
            });

        let duration_text = format!("  ◄ {} ►  (←/→ to change)", self.duration);
        let duration_para = Paragraph::new(duration_text)
            .block(duration_block)
            .style(theme.normal());
        frame.render_widget(duration_para, chunks[2]);

        let (advanced_chunk_idx, timeout_chunk_idx, hints_chunk_idx) = if self.show_advanced {
            (3, 4, 5)
        } else {
            (0, 3, 4) // advanced_chunk_idx unused when not showing
        };

        // Advanced options (if shown)
        if self.show_advanced {
            let advanced_focused = self.focus == PromptFocus::Advanced;
            let advanced_block = Block::default()
                .title(" Apply to (Space=toggle) ")
                .borders(Borders::ALL)
                .border_style(if advanced_focused {
                    theme.border_focused()
                } else {
                    theme.border()
                });

            let options = [
                ("Destination host", self.match_dest_host, !self.connection.dst_host.is_empty()),
                ("Destination IP", self.match_dest_ip, !self.connection.dst_ip.is_empty()),
                ("Destination port", self.match_dest_port, true),
                ("This user", self.match_user, true),
                ("Executable checksum", self.match_checksum, self.connection.process_checksums.contains_key("md5")),
            ];

            let option_lines: Vec<Line> = options
                .iter()
                .enumerate()
                .map(|(i, (label, checked, available))| {
                    let checkbox = if *checked { "[x]" } else { "[ ]" };
                    let style = if !available {
                        theme.dim()
                    } else if advanced_focused && i == self.advanced_focus {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        theme.normal()
                    };
                    Line::from(Span::styled(format!("  {} {}", checkbox, label), style))
                })
                .collect();

            let advanced_para = Paragraph::new(option_lines)
                .block(advanced_block);
            frame.render_widget(advanced_para, chunks[advanced_chunk_idx]);
        }

        // Timeout progress bar
        let ratio = self.timeout_ratio();
        let color = if ratio > 0.5 {
            Color::Green
        } else if ratio > 0.25 {
            Color::Yellow
        } else {
            Color::Red
        };

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color))
            .ratio(ratio)
            .label(format!("Timeout: {}s", remaining));
        frame.render_widget(gauge, chunks[timeout_chunk_idx]);

        // Hints
        let hint_text = if self.show_advanced {
            "Enter=confirm  Esc=cancel  Tab=navigate  Space=toggle"
        } else {
            "Enter=confirm  Esc=cancel  Tab=navigate  Space=advanced"
        };
        let hints = Paragraph::new(format!("  {}", hint_text))
            .style(theme.dim())
            .wrap(Wrap { trim: true });
        frame.render_widget(hints, chunks[hints_chunk_idx]);
    }
}
