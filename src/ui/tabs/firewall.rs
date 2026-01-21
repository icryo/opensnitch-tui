//! Firewall tab implementation

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState},
    Frame,
};
use tokio::sync::mpsc;

use crate::app::events::navigation_delta;
use crate::app::state::{AppMessage, AppState};
use crate::grpc::notifications::NotificationAction;
use crate::models::{FwChain, FwRule, SysFirewall};
use crate::ui::dialogs::fw_rule::{FwRuleEditorDialog, FwRuleEditorResult};
use crate::ui::layout::DialogLayout;
use crate::ui::theme::Theme;

const FIREWALL_CONFIG_PATH: &str = "/etc/opensnitchd/system-fw.json";

/// Focus area within firewall tab
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallFocus {
    Chains,
    Rules,
}

pub struct FirewallTab {
    focus: FirewallFocus,
    chain_state: ListState,
    rule_state: TableState,
    cached_firewall: Option<SysFirewall>,
    cached_chains: Vec<FwChain>,
    selected_chain_idx: usize,

    // Dialogs
    show_toggle_confirm: bool,
    toggle_to_enable: bool,

    // Rule editor
    show_editor: bool,
    editor: Option<FwRuleEditorDialog>,

    // Delete confirmation
    show_delete_confirm: bool,
    rule_to_delete: Option<String>,
}

impl FirewallTab {
    pub fn new() -> Self {
        let mut chain_state = ListState::default();
        chain_state.select(Some(0));
        let mut rule_state = TableState::default();
        rule_state.select(Some(0));

        Self {
            focus: FirewallFocus::Chains,
            chain_state,
            rule_state,
            cached_firewall: None,
            cached_chains: Vec::new(),
            selected_chain_idx: 0,
            show_toggle_confirm: false,
            toggle_to_enable: false,
            show_editor: false,
            editor: None,
            show_delete_confirm: false,
            rule_to_delete: None,
        }
    }

    pub fn showing_dialog(&self) -> bool {
        self.show_editor || self.show_toggle_confirm || self.show_delete_confirm
    }

    /// Get currently selected rule
    fn selected_rule(&self) -> Option<&FwRule> {
        let chain = self.selected_chain()?;
        let idx = self.rule_state.selected()?;
        chain.rules.get(idx)
    }

    /// Save firewall config to disk
    fn save_firewall_config(&self) -> Result<(), std::io::Error> {
        if let Some(fw) = &self.cached_firewall {
            let json = serde_json::to_string_pretty(fw)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            std::fs::write(FIREWALL_CONFIG_PATH, json)?;
        }
        Ok(())
    }

    pub async fn update_cache(&mut self, state: &Arc<AppState>) {
        let nodes = state.nodes.read().await;
        if let Some(node) = nodes.active_node() {
            if let Some(fw) = &node.firewall {
                self.cached_firewall = Some(fw.clone());
                self.cached_chains = fw.all_chains().cloned().collect();
            } else {
                self.cached_firewall = None;
                self.cached_chains.clear();
            }
        } else {
            self.cached_firewall = None;
            self.cached_chains.clear();
        }
    }

    fn selected_chain(&self) -> Option<&FwChain> {
        self.cached_chains.get(self.selected_chain_idx)
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, _state: &Arc<AppState>, theme: &Theme) {
        // Rule editor dialog
        if self.show_editor {
            if let Some(editor) = &self.editor {
                editor.render(frame, theme);
            }
            return;
        }

        // Toggle confirmation dialog
        if self.show_toggle_confirm {
            self.render_toggle_confirm(frame, area, theme);
            return;
        }

        // Delete confirmation dialog
        if self.show_delete_confirm {
            self.render_delete_confirm(frame, area, theme);
            return;
        }

        // Main layout: Status bar + split view (chains | rules)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Status
                Constraint::Min(10),   // Main content
            ])
            .split(area);

        // Render status bar
        self.render_status(frame, chunks[0], theme);

        // Split view: chains list | rules table
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Chains
                Constraint::Percentage(70), // Rules
            ])
            .split(chunks[1]);

        self.render_chains(frame, split[0], theme);
        self.render_rules(frame, split[1], theme);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let fw = self.cached_firewall.as_ref();

        let enabled = fw.map(|f| f.enabled).unwrap_or(false);
        let running = fw.map(|f| f.running).unwrap_or(false);
        let input_policy = fw.map(|f| f.input_policy.as_str()).unwrap_or("N/A");
        let output_policy = fw.map(|f| f.output_policy.as_str()).unwrap_or("N/A");

        let status_style = if running && enabled {
            Style::default().fg(Color::Green)
        } else if running {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Red)
        };

        let status_text = if running && enabled {
            "ENABLED"
        } else if running {
            "RUNNING (rules disabled)"
        } else {
            "DISABLED"
        };

        let status_line = Line::from(vec![
            Span::raw(" Status: "),
            Span::styled(status_text, status_style.add_modifier(Modifier::BOLD)),
            Span::raw(" │ Input: "),
            Span::styled(input_policy, policy_style(input_policy)),
            Span::raw(" │ Output: "),
            Span::styled(output_policy, policy_style(output_policy)),
            Span::raw(" │ Chains: "),
            Span::raw(format!("{}", self.cached_chains.len())),
            Span::raw(" │ "),
            Span::styled("F2=Toggle  F5=Reload", theme.dim()),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(" Firewall Status ");

        let para = Paragraph::new(status_line).block(block);
        frame.render_widget(para, area);
    }

    fn render_chains(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focused = self.focus == FirewallFocus::Chains;
        let border_style = if focused {
            theme.border_focused()
        } else {
            theme.border()
        };

        let items: Vec<ListItem> = if self.cached_chains.is_empty() {
            vec![ListItem::new("No chains configured").style(theme.dim())]
        } else {
            self.cached_chains
                .iter()
                .map(|chain| {
                    let icon = match chain.hook.as_str() {
                        "input" => "▼",
                        "output" => "▲",
                        "forward" => "↔",
                        _ => "•",
                    };
                    let name = format!("{} {} ({})", icon, chain.name, chain.rules.len());
                    ListItem::new(name)
                })
                .collect()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(" Chains "),
            )
            .highlight_style(theme.selected())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.chain_state);
    }

    fn render_rules(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focused = self.focus == FirewallFocus::Rules;
        let border_style = if focused {
            theme.border_focused()
        } else {
            theme.border()
        };

        let chain = self.selected_chain();
        let chain_name = chain.map(|c| c.name.as_str()).unwrap_or("None");
        let rules = chain.map(|c| &c.rules).cloned().unwrap_or_default();

        let header_cells = ["#", "Enabled", "Action", "Description"]
            .iter()
            .map(|h| Cell::from(*h).style(theme.accent().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = if rules.is_empty() {
            vec![Row::new(vec![
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from("No rules in this chain"),
            ])
            .style(theme.dim())]
        } else {
            rules
                .iter()
                .enumerate()
                .map(|(i, rule)| {
                    let enabled_style = if rule.enabled {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    let action_style = match rule.target.to_lowercase().as_str() {
                        "accept" => Style::default().fg(Color::Green),
                        "drop" => Style::default().fg(Color::Red),
                        "reject" => Style::default().fg(Color::Magenta),
                        _ => theme.normal(),
                    };

                    Row::new(vec![
                        Cell::from(format!("{}", i + 1)),
                        Cell::from(if rule.enabled { "✓" } else { "✗" }).style(enabled_style),
                        Cell::from(rule.target.clone()).style(action_style),
                        Cell::from(truncate(&rule.description, 40).to_string()),
                    ])
                })
                .collect()
        };

        let widths = [
            Constraint::Length(4),       // #
            Constraint::Length(8),       // Enabled
            Constraint::Length(10),      // Action
            Constraint::Percentage(70),  // Description
        ];

        let title = format!(" Rules: {} ", chain_name);
        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(title),
            )
            .row_highlight_style(theme.selected())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(table, area, &mut self.rule_state);

        // Hints
        if area.height > 10 && focused {
            let hint_area = Rect::new(
                area.x + 1,
                area.y + area.height - 2,
                area.width - 2,
                1,
            );
            let hint = Paragraph::new(" n=new  e/Enter=edit  d=delete  space=toggle")
                .style(theme.dim());
            frame.render_widget(hint, hint_area);
        }
    }

    fn render_toggle_confirm(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_area = DialogLayout::centered(area, 45, 8).dialog;
        frame.render_widget(Clear, dialog_area);

        let action = if self.toggle_to_enable {
            "enable"
        } else {
            "disable"
        };

        let block = Block::default()
            .title(" Confirm ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

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

        let msg = Paragraph::new(format!("Are you sure you want to {} the firewall?", action))
            .style(theme.normal());
        frame.render_widget(msg, chunks[0]);

        let hint = Paragraph::new("  y = yes  |  n/Esc = cancel")
            .style(theme.dim());
        frame.render_widget(hint, chunks[1]);
    }

    fn render_delete_confirm(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_area = DialogLayout::centered(area, 50, 8).dialog;
        frame.render_widget(Clear, dialog_area);

        let rule_desc = self.rule_to_delete.as_deref().unwrap_or("unknown");

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

        let msg = Paragraph::new(format!("Delete rule '{}'?", truncate(rule_desc, 30)))
            .style(theme.normal());
        frame.render_widget(msg, chunks[0]);

        let hint = Paragraph::new("  y = yes, delete  |  n/Esc = cancel")
            .style(theme.dim());
        frame.render_widget(hint, chunks[1]);
    }

    pub async fn handle_key(&mut self, key: KeyEvent, state: &Arc<AppState>, state_tx: &mpsc::Sender<AppMessage>) {
        // Handle rule editor dialog
        if self.show_editor {
            if let Some(editor) = &mut self.editor {
                if let Some(result) = editor.handle_key(key) {
                    match result {
                        FwRuleEditorResult::Save(rule) => {
                            // Add/update rule in cached firewall
                            if let Some(fw) = &mut self.cached_firewall {
                                if let Some(chain) = self.cached_chains.get_mut(self.selected_chain_idx) {
                                    if editor.original_uuid.is_some() {
                                        // Edit existing
                                        if let Some(existing) = chain.rules.iter_mut().find(|r| r.uuid == rule.uuid) {
                                            *existing = rule;
                                        }
                                    } else {
                                        // Add new
                                        chain.rules.push(rule);
                                    }
                                    // Update the main firewall struct
                                    for fc in &mut fw.system_rules {
                                        if let Some(c) = fc.chains.iter_mut().find(|c| c.name == chain.name) {
                                            c.rules = chain.rules.clone();
                                        }
                                    }
                                }
                            }

                            // Save to disk and reload
                            if let Err(e) = self.save_firewall_config() {
                                tracing::error!("Failed to save firewall config: {}", e);
                            } else {
                                // Send reload notification
                                let node_addr = {
                                    let nodes = state.nodes.read().await;
                                    nodes.active_addr().map(|s| s.to_string())
                                };
                                if let Some(addr) = node_addr {
                                    let _ = state_tx.send(AppMessage::SendNotification {
                                        node_addr: addr,
                                        action: NotificationAction::ReloadFwRules,
                                    }).await;
                                }
                            }
                        }
                        FwRuleEditorResult::Cancel => {}
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
                    if let Some(uuid) = self.rule_to_delete.take() {
                        // Remove rule from cached firewall
                        if let Some(fw) = &mut self.cached_firewall {
                            if let Some(chain) = self.cached_chains.get_mut(self.selected_chain_idx) {
                                chain.rules.retain(|r| r.uuid != uuid);
                                // Update the main firewall struct
                                for fc in &mut fw.system_rules {
                                    if let Some(c) = fc.chains.iter_mut().find(|c| c.name == chain.name) {
                                        c.rules = chain.rules.clone();
                                    }
                                }
                            }
                        }

                        // Save to disk and reload
                        if let Err(e) = self.save_firewall_config() {
                            tracing::error!("Failed to save firewall config: {}", e);
                        } else {
                            let node_addr = {
                                let nodes = state.nodes.read().await;
                                nodes.active_addr().map(|s| s.to_string())
                            };
                            if let Some(addr) = node_addr {
                                let _ = state_tx.send(AppMessage::SendNotification {
                                    node_addr: addr,
                                    action: NotificationAction::ReloadFwRules,
                                }).await;
                            }
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

        // Handle toggle confirmation
        if self.show_toggle_confirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let node_addr = {
                        let nodes = state.nodes.read().await;
                        nodes.active_addr().map(|s| s.to_string())
                    };

                    if let Some(addr) = node_addr {
                        let action = if self.toggle_to_enable {
                            NotificationAction::EnableFirewall
                        } else {
                            NotificationAction::DisableFirewall
                        };
                        let _ = state_tx.send(AppMessage::SendNotification {
                            node_addr: addr,
                            action,
                        }).await;
                    }
                    self.show_toggle_confirm = false;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.show_toggle_confirm = false;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab => {
                self.focus = match self.focus {
                    FirewallFocus::Chains => FirewallFocus::Rules,
                    FirewallFocus::Rules => FirewallFocus::Chains,
                };
            }
            KeyCode::F(2) => {
                // Toggle firewall
                let currently_enabled = self.cached_firewall
                    .as_ref()
                    .map(|f| f.enabled)
                    .unwrap_or(false);
                self.toggle_to_enable = !currently_enabled;
                self.show_toggle_confirm = true;
            }
            KeyCode::F(5) => {
                // Reload firewall rules
                let node_addr = {
                    let nodes = state.nodes.read().await;
                    nodes.active_addr().map(|s| s.to_string())
                };

                if let Some(addr) = node_addr {
                    let _ = state_tx.send(AppMessage::SendNotification {
                        node_addr: addr,
                        action: NotificationAction::ReloadFwRules,
                    }).await;
                }
            }
            KeyCode::Char('n') => {
                // New rule (only in Rules focus)
                if self.focus == FirewallFocus::Rules && !self.cached_chains.is_empty() {
                    let mut editor = FwRuleEditorDialog::new();
                    // Set position to end of list
                    if let Some(chain) = self.selected_chain() {
                        editor.position = chain.rules.len() as u64;
                    }
                    self.editor = Some(editor);
                    self.show_editor = true;
                }
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                // Edit selected rule
                if self.focus == FirewallFocus::Rules {
                    if let Some(rule) = self.selected_rule() {
                        self.editor = Some(FwRuleEditorDialog::edit(rule));
                        self.show_editor = true;
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                // Delete selected rule
                if self.focus == FirewallFocus::Rules {
                    if let Some(rule) = self.selected_rule() {
                        self.rule_to_delete = Some(rule.uuid.clone());
                        self.show_delete_confirm = true;
                    }
                }
            }
            KeyCode::Char(' ') => {
                // Toggle rule enabled
                if self.focus == FirewallFocus::Rules {
                    if let Some(rule) = self.selected_rule() {
                        let uuid = rule.uuid.clone();
                        let new_enabled = !rule.enabled;

                        // Update in cached data
                        if let Some(chain) = self.cached_chains.get_mut(self.selected_chain_idx) {
                            if let Some(r) = chain.rules.iter_mut().find(|r| r.uuid == uuid) {
                                r.enabled = new_enabled;
                            }
                            // Update main firewall struct
                            if let Some(fw) = &mut self.cached_firewall {
                                for fc in &mut fw.system_rules {
                                    if let Some(c) = fc.chains.iter_mut().find(|c| c.name == chain.name) {
                                        if let Some(r) = c.rules.iter_mut().find(|r| r.uuid == uuid) {
                                            r.enabled = new_enabled;
                                        }
                                    }
                                }
                            }
                        }

                        // Save and reload
                        if let Err(e) = self.save_firewall_config() {
                            tracing::error!("Failed to save firewall config: {}", e);
                        } else {
                            let node_addr = {
                                let nodes = state.nodes.read().await;
                                nodes.active_addr().map(|s| s.to_string())
                            };
                            if let Some(addr) = node_addr {
                                let _ = state_tx.send(AppMessage::SendNotification {
                                    node_addr: addr,
                                    action: NotificationAction::ReloadFwRules,
                                }).await;
                            }
                        }
                    }
                }
            }
            _ => {
                if let Some(delta) = navigation_delta(&key) {
                    match self.focus {
                        FirewallFocus::Chains => {
                            let len = self.cached_chains.len();
                            if len == 0 {
                                return;
                            }
                            let current = self.chain_state.selected().unwrap_or(0);
                            let new_index = if delta == i32::MIN {
                                0
                            } else if delta == i32::MAX {
                                len.saturating_sub(1)
                            } else {
                                (current as i32 + delta).clamp(0, len as i32 - 1) as usize
                            };
                            self.chain_state.select(Some(new_index));
                            self.selected_chain_idx = new_index;
                            self.rule_state.select(Some(0)); // Reset rule selection
                        }
                        FirewallFocus::Rules => {
                            let len = self.selected_chain()
                                .map(|c| c.rules.len())
                                .unwrap_or(0);
                            if len == 0 {
                                return;
                            }
                            let current = self.rule_state.selected().unwrap_or(0);
                            let new_index = if delta == i32::MIN {
                                0
                            } else if delta == i32::MAX {
                                len.saturating_sub(1)
                            } else {
                                (current as i32 + delta).clamp(0, len as i32 - 1) as usize
                            };
                            self.rule_state.select(Some(new_index));
                        }
                    }
                }
            }
        }
    }
}

fn policy_style(policy: &str) -> Style {
    match policy.to_lowercase().as_str() {
        "accept" => Style::default().fg(Color::Green),
        "drop" => Style::default().fg(Color::Red),
        "reject" => Style::default().fg(Color::Magenta),
        _ => Style::default(),
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}
