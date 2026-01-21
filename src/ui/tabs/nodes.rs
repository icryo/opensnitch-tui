//! Nodes tab implementation

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
use crate::app::state::{AppMessage, AppState, UiUpdateSignal};
use crate::models::{Node, node::NodeStatus};
use crate::ui::theme::Theme;
use crate::utils::format_duration;

pub struct NodesTab {
    table_state: TableState,
    cached_nodes: Vec<Node>,
    active_addr: Option<String>,
}

impl NodesTab {
    pub fn new() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            table_state: state,
            cached_nodes: Vec::new(),
            active_addr: None,
        }
    }

    pub async fn update_cache(&mut self, state: &Arc<AppState>) {
        let nodes = state.nodes.read().await;
        self.cached_nodes = nodes.nodes.values().cloned().collect();
        self.active_addr = nodes.active_addr().map(|s| s.to_string());
    }

    /// Get currently selected node
    fn selected_node(&self) -> Option<&Node> {
        let idx = self.table_state.selected()?;
        self.cached_nodes.get(idx)
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Layout with hint bar at bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(1)])
            .split(area);

        let header_cells = ["", "Address", "Name", "Version", "Status", "Rules", "Uptime"]
            .iter()
            .map(|h| Cell::from(*h).style(theme.accent().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = if self.cached_nodes.is_empty() {
            vec![Row::new(vec![
                Cell::from(""),
                Cell::from("unix:///tmp/osui.sock"),
                Cell::from(""),
                Cell::from(""),
                Cell::from("Waiting for daemon..."),
                Cell::from(""),
                Cell::from(""),
            ])
            .style(theme.dim())]
        } else {
            self.cached_nodes
                .iter()
                .map(|node| {
                    let is_active = self.active_addr.as_deref() == Some(&node.addr);
                    let active_marker = if is_active { "★" } else { "" };
                    let active_style = if is_active {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        theme.normal()
                    };

                    let status_style = match node.status {
                        NodeStatus::Connected => Style::default().fg(Color::Green),
                        NodeStatus::Disconnected => Style::default().fg(Color::Red),
                        NodeStatus::Connecting => Style::default().fg(Color::Yellow),
                        NodeStatus::Error => Style::default().fg(Color::Red),
                    };

                    let uptime = node
                        .statistics
                        .as_ref()
                        .map(|s| format_duration(s.uptime))
                        .unwrap_or_else(|| "N/A".to_string());

                    Row::new(vec![
                        Cell::from(active_marker).style(active_style),
                        Cell::from(truncate(&node.addr, 28).to_string()),
                        Cell::from(node.display_name().to_string()),
                        Cell::from(node.version.clone()),
                        Cell::from(format!("{}", node.status)).style(status_style),
                        Cell::from(format!("{}", node.rules.len())),
                        Cell::from(uptime),
                    ])
                })
                .collect()
        };

        let widths = [
            Constraint::Length(2),      // Active marker
            Constraint::Percentage(28), // Address
            Constraint::Percentage(15), // Name
            Constraint::Length(12),     // Version
            Constraint::Length(12),     // Status
            Constraint::Length(8),      // Rules
            Constraint::Length(12),     // Uptime
        ];

        let title = format!(" Nodes ({}) ", self.cached_nodes.len());

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::NONE)
                    .title(Span::styled(title, theme.accent())),
            )
            .row_highlight_style(theme.selected())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(table, chunks[0], &mut self.table_state);

        // Hint bar
        let hint = Paragraph::new(" ↑↓ = navigate  Enter = set active node  ★ = active")
            .style(theme.dim());
        frame.render_widget(hint, chunks[1]);
    }

    pub async fn handle_key(&mut self, key: KeyEvent, state: &Arc<AppState>, _state_tx: &mpsc::Sender<AppMessage>) {
        match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Switch to selected node
                if let Some(node) = self.selected_node() {
                    let addr = node.addr.clone();
                    let mut nodes = state.nodes.write().await;
                    if nodes.set_active(&addr) {
                        self.active_addr = Some(addr);
                        state.notify_ui(UiUpdateSignal::NodeChanged);
                    }
                }
            }
            _ => {
                if let Some(delta) = navigation_delta(&key) {
                    let len = self.cached_nodes.len();
                    if len == 0 { return; }
                    let current = self.table_state.selected().unwrap_or(0);
                    let new_index = if delta == i32::MIN {
                        0
                    } else if delta == i32::MAX {
                        len.saturating_sub(1)
                    } else {
                        (current as i32 + delta).clamp(0, len as i32 - 1) as usize
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
