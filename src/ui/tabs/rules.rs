//! Rules tab implementation

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use tokio::sync::mpsc;

use crate::app::events::navigation_delta;
use crate::app::state::{AppMessage, AppState};
use crate::grpc::notifications::NotificationAction;
use crate::models::Rule;
use crate::ui::dialogs::rule_editor::{RuleEditorDialog, RuleEditorResult};
use crate::ui::theme::Theme;
use crate::ui::widgets::searchbar::SearchBar;

pub struct RulesTab {
    table_state: TableState,
    search_bar: SearchBar,
    filter_active: bool,
    cached_rules: Vec<Rule>,

    // Editor dialog state
    show_editor: bool,
    editor: Option<RuleEditorDialog>,

    // Confirmation dialog state
    show_delete_confirm: bool,
    rule_to_delete: Option<String>,
}

impl RulesTab {
    pub fn new() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            table_state: state,
            search_bar: SearchBar::new(),
            filter_active: false,
            cached_rules: Vec::new(),
            show_editor: false,
            editor: None,
            show_delete_confirm: false,
            rule_to_delete: None,
        }
    }

    pub fn showing_dialog(&self) -> bool {
        self.show_editor || self.show_delete_confirm
    }

    pub async fn update_cache(&mut self, state: &Arc<AppState>) {
        let nodes = state.nodes.read().await;
        if let Some(node) = nodes.active_node() {
            self.cached_rules = node.rules.clone();
        } else {
            self.cached_rules.clear();
        }
    }

    /// Get currently selected rule
    fn selected_rule(&self) -> Option<&Rule> {
        let idx = self.table_state.selected()?;
        // Get filtered rules to match what's displayed
        let filtered: Vec<&Rule> = if self.search_bar.query.is_empty() {
            self.cached_rules.iter().collect()
        } else {
            let query = self.search_bar.query.to_lowercase();
            self.cached_rules
                .iter()
                .filter(|r| {
                    r.name.to_lowercase().contains(&query)
                        || r.description.to_lowercase().contains(&query)
                        || r.operator.operand.to_lowercase().contains(&query)
                        || r.operator.data.to_lowercase().contains(&query)
                })
                .collect()
        };
        filtered.get(idx).copied()
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // If editor is showing, render it on top
        if self.show_editor {
            if let Some(editor) = &self.editor {
                editor.render(frame, theme);
            }
            return;
        }

        // If delete confirmation is showing, render it
        if self.show_delete_confirm {
            self.render_delete_confirm(frame, area, theme);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(if self.filter_active {
                vec![Constraint::Length(3), Constraint::Min(5)]
            } else {
                vec![Constraint::Length(0), Constraint::Min(5)]
            })
            .split(area);

        if self.filter_active {
            self.search_bar.render(frame, chunks[0], theme.normal(), theme.border_focused());
        }

        // Filter rules
        let filtered_rules: Vec<&Rule> = if self.search_bar.query.is_empty() {
            self.cached_rules.iter().collect()
        } else {
            let query = self.search_bar.query.to_lowercase();
            self.cached_rules
                .iter()
                .filter(|r| {
                    r.name.to_lowercase().contains(&query)
                        || r.description.to_lowercase().contains(&query)
                        || r.operator.operand.to_lowercase().contains(&query)
                        || r.operator.data.to_lowercase().contains(&query)
                })
                .collect()
        };

        let header_cells = ["Name", "Enabled", "Action", "Duration", "Operand", "Data"]
            .iter()
            .map(|h| Cell::from(*h).style(theme.accent().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = if filtered_rules.is_empty() {
            vec![Row::new(vec![
                Cell::from("No rules loaded"),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ])
            .style(theme.dim())]
        } else {
            filtered_rules
                .iter()
                .map(|rule| {
                    let enabled_style = if rule.enabled {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    let action_style = match rule.action.to_string().as_str() {
                        "allow" => Style::default().fg(Color::Green),
                        "deny" => Style::default().fg(Color::Red),
                        "reject" => Style::default().fg(Color::Magenta),
                        _ => theme.normal(),
                    };

                    Row::new(vec![
                        Cell::from(truncate(&rule.name, 25).to_string()),
                        Cell::from(if rule.enabled { "✓" } else { "✗" }).style(enabled_style),
                        Cell::from(rule.action.to_string()).style(action_style),
                        Cell::from(rule.duration.to_string()),
                        Cell::from(truncate(&rule.operator.operand, 18).to_string()),
                        Cell::from(truncate(&rule.operator.data, 25).to_string()),
                    ])
                })
                .collect()
        };

        let widths = [
            Constraint::Percentage(20), // Name
            Constraint::Length(8),      // Enabled
            Constraint::Length(8),      // Action
            Constraint::Length(14),     // Duration
            Constraint::Percentage(18), // Operand
            Constraint::Percentage(25), // Data
        ];

        let title = if self.search_bar.query.is_empty() {
            format!(" Rules ({}) ", filtered_rules.len())
        } else {
            format!(
                " Rules ({}/{}) [filter: {}] ",
                filtered_rules.len(),
                self.cached_rules.len(),
                self.search_bar.query
            )
        };

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::NONE)
                    .title(Span::styled(title, theme.accent())),
            )
            .row_highlight_style(theme.selected())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(table, chunks[1], &mut self.table_state);

        if chunks[1].height > 10 && !self.filter_active {
            let hint_area = Rect::new(
                chunks[1].x,
                chunks[1].y + chunks[1].height - 1,
                chunks[1].width,
                1,
            );
            let hint = Paragraph::new(" / = filter  e = edit  n = new  d = delete  space = toggle")
                .style(theme.dim());
            frame.render_widget(hint, hint_area);
        }
    }

    fn render_delete_confirm(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::widgets::Clear;
        use crate::ui::layout::DialogLayout;

        let dialog_area = DialogLayout::centered(area, 50, 8).dialog;
        frame.render_widget(Clear, dialog_area);

        let rule_name = self.rule_to_delete.as_deref().unwrap_or("unknown");
        let block = Block::default()
            .title(" Confirm Delete ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

        frame.render_widget(block.clone(), dialog_area);

        let inner = block.inner(dialog_area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(1),
            ])
            .split(inner);

        let msg = Paragraph::new(format!("Delete rule '{}'?", rule_name))
            .style(theme.normal());
        frame.render_widget(msg, chunks[0]);

        let hint = Paragraph::new("  y = yes, delete  |  n/Esc = cancel")
            .style(theme.dim());
        frame.render_widget(hint, chunks[1]);
    }

    pub async fn handle_key(&mut self, key: KeyEvent, state: &Arc<AppState>, state_tx: &mpsc::Sender<AppMessage>) {
        // Handle editor dialog
        if self.show_editor {
            if let Some(editor) = &mut self.editor {
                if let Some(result) = editor.handle_key(key) {
                    match result {
                        RuleEditorResult::Save(rule) => {
                            // Determine if this is add or modify
                            let is_new = editor.original_name.is_none();

                            // Get active node address
                            let node_addr = {
                                let nodes = state.nodes.read().await;
                                nodes.active_addr().map(|s| s.to_string())
                            };

                            if let Some(addr) = node_addr {
                                if is_new {
                                    // Send add rule notification
                                    let _ = state_tx.send(AppMessage::RuleAdded {
                                        node_addr: addr.clone(),
                                        rule: rule.clone(),
                                    }).await;
                                    let _ = state_tx.send(AppMessage::SendNotification {
                                        node_addr: addr,
                                        action: NotificationAction::ChangeRule(rule),
                                    }).await;
                                } else {
                                    // Send modify rule notification
                                    let _ = state_tx.send(AppMessage::RuleModified {
                                        node_addr: addr.clone(),
                                        rule: rule.clone(),
                                    }).await;
                                    let _ = state_tx.send(AppMessage::SendNotification {
                                        node_addr: addr,
                                        action: NotificationAction::ChangeRule(rule),
                                    }).await;
                                }
                            }
                        }
                        RuleEditorResult::Cancel => {}
                    }
                    self.show_editor = false;
                    self.editor = None;
                }
            }
            return;
        }

        // Handle delete confirmation
        if self.show_delete_confirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(name) = self.rule_to_delete.take() {
                        let node_addr = {
                            let nodes = state.nodes.read().await;
                            nodes.active_addr().map(|s| s.to_string())
                        };

                        if let Some(addr) = node_addr {
                            let _ = state_tx.send(AppMessage::RuleDeleted {
                                node_addr: addr.clone(),
                                name: name.clone(),
                            }).await;
                            let _ = state_tx.send(AppMessage::SendNotification {
                                node_addr: addr,
                                action: NotificationAction::DeleteRule(name),
                            }).await;
                        }
                    }
                    self.show_delete_confirm = false;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.show_delete_confirm = false;
                    self.rule_to_delete = None;
                }
                _ => {}
            }
            return;
        }

        if self.filter_active {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.filter_active = false;
                    self.search_bar.deactivate();
                }
                KeyCode::Backspace => self.search_bar.backspace(),
                KeyCode::Delete => self.search_bar.delete(),
                KeyCode::Left => self.search_bar.move_left(),
                KeyCode::Right => self.search_bar.move_right(),
                KeyCode::Char(c) => self.search_bar.insert(c),
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('/') => {
                self.filter_active = true;
                self.search_bar.activate();
            }
            KeyCode::Esc => self.search_bar.clear(),
            KeyCode::Char('n') => {
                // New rule
                self.editor = Some(RuleEditorDialog::new());
                self.show_editor = true;
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                // Edit selected rule
                if let Some(rule) = self.selected_rule() {
                    self.editor = Some(RuleEditorDialog::edit(rule));
                    self.show_editor = true;
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                // Delete selected rule
                if let Some(rule) = self.selected_rule() {
                    self.rule_to_delete = Some(rule.name.clone());
                    self.show_delete_confirm = true;
                }
            }
            KeyCode::Char(' ') => {
                // Toggle enable/disable
                if let Some(rule) = self.selected_rule() {
                    let node_addr = {
                        let nodes = state.nodes.read().await;
                        nodes.active_addr().map(|s| s.to_string())
                    };

                    if let Some(addr) = node_addr {
                        let new_enabled = !rule.enabled;
                        let _ = state_tx.send(AppMessage::RuleToggled {
                            node_addr: addr.clone(),
                            name: rule.name.clone(),
                            enabled: new_enabled,
                        }).await;

                        // Send notification to daemon
                        let action = if new_enabled {
                            NotificationAction::EnableRule(rule.name.clone())
                        } else {
                            NotificationAction::DisableRule(rule.name.clone())
                        };
                        let _ = state_tx.send(AppMessage::SendNotification {
                            node_addr: addr,
                            action,
                        }).await;
                    }
                }
            }
            _ => {
                if let Some(delta) = navigation_delta(&key) {
                    // Get filtered rules length
                    let filtered_len = if self.search_bar.query.is_empty() {
                        self.cached_rules.len()
                    } else {
                        let query = self.search_bar.query.to_lowercase();
                        self.cached_rules
                            .iter()
                            .filter(|r| {
                                r.name.to_lowercase().contains(&query)
                                    || r.description.to_lowercase().contains(&query)
                                    || r.operator.operand.to_lowercase().contains(&query)
                                    || r.operator.data.to_lowercase().contains(&query)
                            })
                            .count()
                    };

                    if filtered_len == 0 {
                        return;
                    }
                    let current = self.table_state.selected().unwrap_or(0);
                    let new_index = if delta == i32::MIN {
                        0
                    } else if delta == i32::MAX {
                        filtered_len.saturating_sub(1)
                    } else {
                        (current as i32 + delta).clamp(0, filtered_len as i32 - 1) as usize
                    };
                    self.table_state.select(Some(new_index));
                }
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}
