//! Alerts tab implementation

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::events::navigation_delta;
use crate::app::state::AppState;
use crate::models::{Alert, AlertPriority, AlertType};
use crate::ui::theme::Theme;
use crate::ui::widgets::searchbar::SearchBar;

pub struct AlertsTab {
    table_state: TableState,
    search_bar: SearchBar,
    filter_active: bool,
    cached_alerts: Vec<Alert>,
}

impl AlertsTab {
    pub fn new() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            table_state: state,
            search_bar: SearchBar::new(),
            filter_active: false,
            cached_alerts: Vec::new(),
        }
    }

    pub async fn update_cache(&mut self, state: &Arc<AppState>) {
        let alerts = state.alerts.read().await;
        self.cached_alerts = alerts.iter().cloned().collect();
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
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

        let filtered_alerts: Vec<&Alert> = if self.search_bar.query.is_empty() {
            self.cached_alerts.iter().collect()
        } else {
            let query = self.search_bar.query.to_lowercase();
            self.cached_alerts
                .iter()
                .filter(|a| {
                    a.text().to_lowercase().contains(&query)
                        || a.node.to_lowercase().contains(&query)
                })
                .collect()
        };

        let header_cells = ["Time", "Type", "Priority", "Source", "Message"]
            .iter()
            .map(|h| Cell::from(*h).style(theme.accent().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = if filtered_alerts.is_empty() {
            vec![Row::new(vec![
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from("No alerts"),
                Cell::from(""),
            ])
            .style(theme.dim())]
        } else {
            filtered_alerts
                .iter()
                .map(|alert| {
                    let type_style = match alert.alert_type {
                        AlertType::Error => Style::default().fg(Color::Red),
                        AlertType::Warning => Style::default().fg(Color::Yellow),
                        AlertType::Info => Style::default().fg(Color::Blue),
                    };

                    let priority_style = match alert.priority {
                        AlertPriority::High => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        AlertPriority::Medium => Style::default().fg(Color::Yellow),
                        AlertPriority::Low => Style::default().fg(Color::DarkGray),
                    };

                    let time = alert.timestamp.format("%H:%M:%S").to_string();

                    Row::new(vec![
                        Cell::from(time),
                        Cell::from(format!("{}", alert.alert_type)).style(type_style),
                        Cell::from(format!("{:?}", alert.priority)).style(priority_style),
                        Cell::from(format!("{}", alert.what)),
                        Cell::from(truncate(&alert.text(), 40).to_string()),
                    ])
                })
                .collect()
        };

        let widths = [
            Constraint::Length(10),     // Time
            Constraint::Length(10),     // Type
            Constraint::Length(10),     // Priority
            Constraint::Length(15),     // Source
            Constraint::Percentage(50), // Message
        ];

        let title = format!(" Alerts ({}) ", filtered_alerts.len());

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
    }

    pub async fn handle_key(&mut self, key: KeyEvent, _state: &Arc<AppState>) {
        if self.filter_active {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.filter_active = false;
                    self.search_bar.deactivate();
                }
                KeyCode::Backspace => self.search_bar.backspace(),
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
            _ => {
                if let Some(delta) = navigation_delta(&key) {
                    let len = self.cached_alerts.len();
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
