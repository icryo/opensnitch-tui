//! Connection details dialog with blocking capability

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::app::state::AppMessage;
use crate::models::{Event, Operator, Rule, RuleAction, RuleDuration};
use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
enum DetailsFocus {
    Info,
    Actions,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ActionItem {
    BlockProcess,
    BlockDestination,
    BlockPort,
    AllowProcess,
    Close,
}

impl ActionItem {
    fn all() -> &'static [ActionItem] {
        &[
            ActionItem::BlockProcess,
            ActionItem::BlockDestination,
            ActionItem::BlockPort,
            ActionItem::AllowProcess,
            ActionItem::Close,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            ActionItem::BlockProcess => "Block this process",
            ActionItem::BlockDestination => "Block this destination",
            ActionItem::BlockPort => "Block this port",
            ActionItem::AllowProcess => "Always allow this process",
            ActionItem::Close => "Close",
        }
    }
}

pub struct ConnectionDetailsDialog {
    event: Event,
    focus: DetailsFocus,
    action_index: usize,
    scroll_offset: u16,
}

impl ConnectionDetailsDialog {
    pub fn new(event: Event) -> Self {
        Self {
            event,
            focus: DetailsFocus::Info,
            action_index: 0,
            scroll_offset: 0,
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        state_tx: &mpsc::Sender<AppMessage>,
        node_addr: Option<&str>,
    ) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Tab => {
                self.focus = match self.focus {
                    DetailsFocus::Info => DetailsFocus::Actions,
                    DetailsFocus::Actions => DetailsFocus::Info,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focus == DetailsFocus::Actions {
                    if self.action_index > 0 {
                        self.action_index -= 1;
                    }
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.focus == DetailsFocus::Actions {
                    if self.action_index < ActionItem::all().len() - 1 {
                        self.action_index += 1;
                    }
                } else {
                    self.scroll_offset += 1;
                }
            }
            KeyCode::Enter => {
                if self.focus == DetailsFocus::Actions {
                    let action = ActionItem::all()[self.action_index];
                    if action == ActionItem::Close {
                        return true;
                    }
                    if let Some(addr) = node_addr {
                        if let Some(rule) = self.create_rule(action) {
                            let _ = state_tx.try_send(AppMessage::RuleAdded {
                                node_addr: addr.to_string(),
                                rule,
                            });
                        }
                    }
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn create_rule(&self, action: ActionItem) -> Option<Rule> {
        let conn = &self.event.connection;

        match action {
            ActionItem::BlockProcess => {
                let name = format!("block-{}", conn.process_name());
                Some(Rule::new(
                    &name,
                    RuleAction::Deny,
                    RuleDuration::Always,
                    Operator::simple("process.path", &conn.process_path),
                ))
            }
            ActionItem::BlockDestination => {
                let dest = if !conn.dst_host.is_empty() {
                    &conn.dst_host
                } else {
                    &conn.dst_ip
                };
                let name = format!("block-{}", dest);
                Some(Rule::new(
                    &name,
                    RuleAction::Deny,
                    RuleDuration::Always,
                    Operator::simple("dest.host", dest),
                ))
            }
            ActionItem::BlockPort => {
                let name = format!("block-port-{}", conn.dst_port);
                Some(Rule::new(
                    &name,
                    RuleAction::Deny,
                    RuleDuration::Always,
                    Operator::simple("dest.port", &conn.dst_port.to_string()),
                ))
            }
            ActionItem::AllowProcess => {
                let name = format!("allow-{}", conn.process_name());
                Some(Rule::new(
                    &name,
                    RuleAction::Allow,
                    RuleDuration::Always,
                    Operator::simple("process.path", &conn.process_path),
                ))
            }
            ActionItem::Close => None,
        }
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        let area = frame.area();

        // Center dialog - 80% width, 80% height
        let dialog_width = (area.width as f32 * 0.8) as u16;
        let dialog_height = (area.height as f32 * 0.8) as u16;
        let x = (area.width - dialog_width) / 2;
        let y = (area.height - dialog_height) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(" Connection Details ")
            .borders(Borders::ALL)
            .border_style(theme.border_focused());

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Split into info panel and actions panel
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(inner);

        self.render_info_panel(frame, chunks[0], theme);
        self.render_actions_panel(frame, chunks[1], theme);
    }

    fn render_info_panel(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let conn = &self.event.connection;

        let mut lines: Vec<Line> = vec![];

        // Process section
        lines.push(Line::from(Span::styled(
            "PROCESS",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format!("  Path: {}", conn.process_path)));
        lines.push(Line::from(format!("  Name: {}", conn.process_name())));
        lines.push(Line::from(format!("  PID:  {}", conn.process_id)));
        lines.push(Line::from(format!("  UID:  {}", conn.user_id)));
        lines.push(Line::from(format!("  CWD:  {}", conn.process_cwd)));

        if !conn.process_args.is_empty() {
            lines.push(Line::from(format!("  Args: {}", conn.process_args.join(" "))));
        }

        lines.push(Line::from(""));

        // Connection section
        lines.push(Line::from(Span::styled(
            "CONNECTION",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format!("  Protocol: {}", conn.protocol)));
        lines.push(Line::from(format!("  Source:   {}:{}", conn.src_ip, conn.src_port)));

        let dest = if !conn.dst_host.is_empty() {
            format!("{} ({})", conn.dst_host, conn.dst_ip)
        } else {
            conn.dst_ip.clone()
        };
        lines.push(Line::from(format!("  Dest:     {}:{}", dest, conn.dst_port)));

        lines.push(Line::from(""));

        // Checksums section
        if !conn.process_checksums.is_empty() {
            lines.push(Line::from(Span::styled(
                "CHECKSUMS",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            for (algo, hash) in &conn.process_checksums {
                lines.push(Line::from(format!("  {}: {}", algo, hash)));
            }
            lines.push(Line::from(""));
        }

        // Environment section (truncated)
        if !conn.process_env.is_empty() {
            lines.push(Line::from(Span::styled(
                "ENVIRONMENT (selected)",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            let important_vars = ["PATH", "HOME", "USER", "SHELL", "DISPLAY", "TERM"];
            for var in important_vars {
                if let Some(val) = conn.process_env.get(var) {
                    let truncated = if val.len() > 50 {
                        format!("{}...", &val[..47])
                    } else {
                        val.clone()
                    };
                    lines.push(Line::from(format!("  {}={}", var, truncated)));
                }
            }
            lines.push(Line::from(""));
        }

        // Time
        lines.push(Line::from(Span::styled(
            "TIMESTAMP",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format!("  {}", self.event.time)));

        // Apply scroll offset
        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.scroll_offset as usize)
            .collect();

        let border_style = if self.focus == DetailsFocus::Info {
            theme.border_focused()
        } else {
            theme.border()
        };

        let info_block = Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let paragraph = Paragraph::new(visible_lines)
            .block(info_block)
            .style(theme.normal());

        frame.render_widget(paragraph, area);
    }

    fn render_actions_panel(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = ActionItem::all()
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let style = if i == self.action_index && self.focus == DetailsFocus::Actions {
                    theme.selected()
                } else {
                    match action {
                        ActionItem::BlockProcess | ActionItem::BlockDestination | ActionItem::BlockPort => {
                            Style::default().fg(Color::Red)
                        }
                        ActionItem::AllowProcess => Style::default().fg(Color::Green),
                        ActionItem::Close => theme.normal(),
                    }
                };
                ListItem::new(action.label()).style(style)
            })
            .collect();

        let border_style = if self.focus == DetailsFocus::Actions {
            theme.border_focused()
        } else {
            theme.border()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Actions ")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_symbol("▶ ");

        frame.render_widget(list, area);

        // Help hint at bottom
        if area.height > 8 {
            let hint_area = Rect::new(area.x + 1, area.y + area.height - 2, area.width - 2, 1);
            let hint = Paragraph::new("Tab=switch  Enter=select")
                .style(theme.dim());
            frame.render_widget(hint, hint_area);
        }
    }
}
