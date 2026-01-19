//! Connections tab implementation

use std::collections::HashMap;
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
use crate::models::Event;
use crate::ui::dialogs::connection_details::ConnectionDetailsDialog;
use crate::ui::theme::Theme;
use crate::ui::widgets::searchbar::SearchBar;

/// Aggregated connection entry
#[derive(Clone)]
struct AggregatedConnection {
    /// Most recent event for this connection
    latest_event: Event,
    /// Number of times this connection was seen
    count: u64,
    /// Unique key for this connection
    key: String,
}

impl AggregatedConnection {
    fn new(event: Event) -> Self {
        let key = Self::make_key(&event);
        Self {
            latest_event: event,
            count: 1,
            key,
        }
    }

    fn make_key(event: &Event) -> String {
        let conn = &event.connection;
        // Use process name (not full path) for more consistent grouping
        let process = conn.process_name();
        // Always prefer IP for consistency (DNS resolution can vary)
        // But if IP is empty, fall back to host
        let dest = if !conn.dst_ip.is_empty() {
            &conn.dst_ip
        } else {
            &conn.dst_host
        };
        format!("{}|{}|{}|{}", process, conn.protocol.to_lowercase(), dest, conn.dst_port)
    }

    fn increment(&mut self, event: Event) {
        self.latest_event = event;
        self.count += 1;
    }
}

pub struct ConnectionsTab {
    table_state: TableState,
    search_bar: SearchBar,
    filter_active: bool,
    /// Aggregated unique connections
    aggregated: Vec<AggregatedConnection>,
    details_dialog: Option<ConnectionDetailsDialog>,
    cached_node_addr: Option<String>,
}

impl ConnectionsTab {
    pub fn new() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            table_state: state,
            search_bar: SearchBar::new(),
            filter_active: false,
            aggregated: Vec::new(),
            details_dialog: None,
            cached_node_addr: None,
        }
    }

    pub fn showing_dialog(&self) -> bool {
        self.details_dialog.is_some()
    }

    /// Update cached data from state (call before render)
    pub async fn update_cache(&mut self, state: &Arc<AppState>) {
        let connections = state.connections.read().await;

        // Aggregate connections by process+destination
        let mut map: HashMap<String, AggregatedConnection> = HashMap::new();

        for event in connections.iter() {
            let key = AggregatedConnection::make_key(event);
            if let Some(agg) = map.get_mut(&key) {
                agg.increment(event.clone());
            } else {
                map.insert(key.clone(), AggregatedConnection::new(event.clone()));
            }
        }

        // Sort by most recent (latest timestamp first)
        let mut aggregated: Vec<AggregatedConnection> = map.into_values().collect();
        aggregated.sort_by(|a, b| b.latest_event.time.cmp(&a.latest_event.time));
        self.aggregated = aggregated;

        // Cache node address for rule creation
        let nodes = state.nodes.read().await;
        self.cached_node_addr = nodes.active_addr().map(|s| s.to_string());
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Layout with optional filter bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(if self.filter_active {
                vec![Constraint::Length(3), Constraint::Min(5)]
            } else {
                vec![Constraint::Length(0), Constraint::Min(5)]
            })
            .split(area);

        // Render filter bar if active
        if self.filter_active {
            self.search_bar.render(
                frame,
                chunks[0],
                theme.normal(),
                theme.border_focused(),
            );
        }

        // Filter aggregated connections
        let filtered: Vec<&AggregatedConnection> = if self.search_bar.query.is_empty() {
            self.aggregated.iter().collect()
        } else {
            let query = self.search_bar.query.to_lowercase();
            self.aggregated
                .iter()
                .filter(|agg| {
                    let conn = &agg.latest_event.connection;
                    conn.process_path.to_lowercase().contains(&query)
                        || conn.dst_host.to_lowercase().contains(&query)
                        || conn.dst_ip.to_lowercase().contains(&query)
                        || conn.protocol.to_lowercase().contains(&query)
                })
                .collect()
        };

        // Header
        let header_cells = ["Time", "Count", "Proto", "Destination", "Process"]
            .iter()
            .map(|h| Cell::from(*h).style(theme.accent().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1);

        // Build rows
        let rows: Vec<Row> = if filtered.is_empty() {
            vec![Row::new(vec![
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from("Waiting for connections..."),
                Cell::from(""),
            ])
            .style(theme.dim())]
        } else {
            filtered
                .iter()
                .map(|agg| {
                    let event = &agg.latest_event;
                    let conn = &event.connection;

                    let time = if event.time.len() > 8 {
                        // Extract HH:MM:SS from ISO timestamp
                        event.time.split('T').nth(1)
                            .and_then(|t| t.split('.').next())
                            .unwrap_or(&event.time[..8.min(event.time.len())])
                    } else {
                        &event.time
                    };

                    let dest = if conn.dst_host.is_empty() {
                        format!("{}:{}", conn.dst_ip, conn.dst_port)
                    } else {
                        format!("{}:{}", truncate(&conn.dst_host, 30), conn.dst_port)
                    };

                    let process = truncate(conn.process_name(), 25);

                    let count_style = if agg.count > 100 {
                        Style::default().fg(Color::Red)
                    } else if agg.count > 10 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        theme.normal()
                    };

                    Row::new(vec![
                        Cell::from(time.to_string()),
                        Cell::from(format!("{}", agg.count)).style(count_style),
                        Cell::from(conn.protocol.clone()),
                        Cell::from(dest),
                        Cell::from(process.to_string()),
                    ])
                })
                .collect()
        };

        let widths = [
            Constraint::Length(10),     // Time
            Constraint::Length(7),      // Count
            Constraint::Length(6),      // Protocol
            Constraint::Percentage(40), // Destination
            Constraint::Percentage(30), // Process
        ];

        // Show count in title
        let title = if self.search_bar.query.is_empty() {
            format!(" Unique Connections ({}) ", filtered.len())
        } else {
            format!(
                " Unique Connections ({}/{}) [filter: {}] ",
                filtered.len(),
                self.aggregated.len(),
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

        // Show help hint at bottom if space
        if chunks[1].height > 10 && !self.filter_active {
            let hint_area = Rect::new(
                chunks[1].x,
                chunks[1].y + chunks[1].height - 1,
                chunks[1].width,
                1,
            );
            let hint = Paragraph::new(" / = filter  ↑↓ = navigate  Enter = details")
                .style(theme.dim());
            frame.render_widget(hint, hint_area);
        }

        // Render details dialog if active
        if let Some(dialog) = &self.details_dialog {
            dialog.render(frame, theme);
        }
    }

    pub async fn handle_key(&mut self, key: KeyEvent, _state: &Arc<AppState>, state_tx: &mpsc::Sender<AppMessage>) {
        // Handle details dialog input
        if let Some(dialog) = &mut self.details_dialog {
            if dialog.handle_key(key, state_tx, self.cached_node_addr.as_deref()) {
                self.details_dialog = None;
            }
            return;
        }

        // Handle filter input mode
        if self.filter_active {
            match key.code {
                KeyCode::Esc => {
                    self.filter_active = false;
                    self.search_bar.deactivate();
                }
                KeyCode::Enter => {
                    self.filter_active = false;
                    self.search_bar.deactivate();
                }
                KeyCode::Backspace => {
                    self.search_bar.backspace();
                }
                KeyCode::Delete => {
                    self.search_bar.delete();
                }
                KeyCode::Left => {
                    self.search_bar.move_left();
                }
                KeyCode::Right => {
                    self.search_bar.move_right();
                }
                KeyCode::Home => {
                    self.search_bar.move_home();
                }
                KeyCode::End => {
                    self.search_bar.move_end();
                }
                KeyCode::Char(c) => {
                    self.search_bar.insert(c);
                }
                _ => {}
            }
            return;
        }

        // Normal mode
        match key.code {
            KeyCode::Char('/') => {
                self.filter_active = true;
                self.search_bar.activate();
            }
            KeyCode::Esc => {
                self.search_bar.clear();
            }
            KeyCode::Enter => {
                // Open details dialog for selected connection
                if let Some(idx) = self.table_state.selected() {
                    if idx < self.aggregated.len() {
                        let event = self.aggregated[idx].latest_event.clone();
                        self.details_dialog = Some(ConnectionDetailsDialog::new(event));
                    }
                }
            }
            _ => {
                if let Some(delta) = navigation_delta(&key) {
                    let len = self.aggregated.len();
                    if len == 0 {
                        return;
                    }

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
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
