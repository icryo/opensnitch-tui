//! Statistics tab implementation

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{BarChart, Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::app::events::navigation_delta;
use crate::app::state::AppState;
use crate::models::Statistics;
use crate::ui::theme::Theme;
use crate::utils::format_duration;

/// Focus area for statistics tab
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsFocus {
    Summary,
    ByProtocol,
    ByHost,
    ByPort,
    ByUser,
    ByExecutable,
}

impl StatsFocus {
    fn next(self) -> Self {
        match self {
            Self::Summary => Self::ByProtocol,
            Self::ByProtocol => Self::ByHost,
            Self::ByHost => Self::ByPort,
            Self::ByPort => Self::ByUser,
            Self::ByUser => Self::ByExecutable,
            Self::ByExecutable => Self::Summary,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Summary => Self::ByExecutable,
            Self::ByProtocol => Self::Summary,
            Self::ByHost => Self::ByProtocol,
            Self::ByPort => Self::ByHost,
            Self::ByUser => Self::ByPort,
            Self::ByExecutable => Self::ByUser,
        }
    }
}

pub struct StatisticsTab {
    focus: StatsFocus,
    cached_stats: Option<Statistics>,
    connections_count: usize,
    rules_count: usize,
    alerts_count: usize,
}

impl StatisticsTab {
    pub fn new() -> Self {
        Self {
            focus: StatsFocus::Summary,
            cached_stats: None,
            connections_count: 0,
            rules_count: 0,
            alerts_count: 0,
        }
    }

    pub async fn update_cache(&mut self, state: &Arc<AppState>) {
        let nodes = state.nodes.read().await;
        if let Some(node) = nodes.active_node() {
            self.cached_stats = node.statistics.clone();
            self.rules_count = node.rules.len();
        } else {
            self.cached_stats = None;
            self.rules_count = 0;
        }
        drop(nodes);

        self.connections_count = state.connections.read().await.len();
        self.alerts_count = state.alerts.read().await.len();
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, _state: &Arc<AppState>, theme: &Theme) {
        // Main layout: top cards + bottom breakdown
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),  // Summary cards
                Constraint::Min(10),    // Breakdown panels
            ])
            .split(area);

        self.render_summary_cards(frame, chunks[0], theme);
        self.render_breakdowns(frame, chunks[1], theme);
    }

    fn render_summary_cards(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let cards = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // Uptime
                Constraint::Percentage(20), // Connections
                Constraint::Percentage(20), // Rules
                Constraint::Percentage(20), // Alerts
                Constraint::Percentage(20), // Bandwidth
            ])
            .split(area);

        let stats = self.cached_stats.as_ref();
        let uptime = stats.map(|s| format_duration(s.uptime)).unwrap_or_else(|| "N/A".to_string());
        let total_conns = stats.map(|s| s.connections).unwrap_or(0);
        let dropped = stats.map(|s| s.dropped).unwrap_or(0);
        let accepted = total_conns.saturating_sub(dropped);

        // Uptime card
        self.render_card(
            frame,
            cards[0],
            "Uptime",
            &uptime,
            Color::Cyan,
            theme,
        );

        // Connections card
        self.render_card(
            frame,
            cards[1],
            "Connections",
            &format!("{}", self.connections_count),
            Color::Blue,
            theme,
        );

        // Rules card
        self.render_card(
            frame,
            cards[2],
            "Rules",
            &format!("{}", self.rules_count),
            Color::Green,
            theme,
        );

        // Alerts card
        self.render_card(
            frame,
            cards[3],
            "Alerts",
            &format!("{}", self.alerts_count),
            if self.alerts_count > 0 { Color::Yellow } else { Color::Gray },
            theme,
        );

        // Accepted/Dropped ratio
        let ratio_text = format!("{}/{}", accepted, dropped);
        self.render_card(
            frame,
            cards[4],
            "Accepted/Dropped",
            &ratio_text,
            Color::Magenta,
            theme,
        );
    }

    fn render_card(&self, frame: &mut Frame, area: Rect, title: &str, value: &str, color: Color, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(format!(" {} ", title));

        frame.render_widget(block.clone(), area);

        let inner = block.inner(area);
        let value_para = Paragraph::new(value)
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
            .alignment(ratatui::layout::Alignment::Center);

        // Center vertically
        let centered_area = Rect::new(
            inner.x,
            inner.y + inner.height / 2,
            inner.width,
            1,
        );
        frame.render_widget(value_para, centered_area);
    }

    fn render_breakdowns(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // 2x3 grid layout
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(rows[0]);

        let bottom_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(rows[1]);

        let stats = self.cached_stats.as_ref();

        // By Protocol
        let by_proto = stats.map(|s| &s.by_proto).cloned().unwrap_or_default();
        self.render_breakdown_list(
            frame,
            top_cols[0],
            "By Protocol",
            &by_proto,
            self.focus == StatsFocus::ByProtocol,
            theme,
        );

        // By Host
        let by_host = stats.map(|s| &s.by_host).cloned().unwrap_or_default();
        self.render_breakdown_list(
            frame,
            top_cols[1],
            "By Host",
            &by_host,
            self.focus == StatsFocus::ByHost,
            theme,
        );

        // By Port
        let by_port = stats.map(|s| &s.by_port).cloned().unwrap_or_default();
        self.render_breakdown_list(
            frame,
            top_cols[2],
            "By Port",
            &by_port,
            self.focus == StatsFocus::ByPort,
            theme,
        );

        // By User
        let by_user = stats.map(|s| &s.by_uid).cloned().unwrap_or_default();
        self.render_breakdown_list(
            frame,
            bottom_cols[0],
            "By User",
            &by_user,
            self.focus == StatsFocus::ByUser,
            theme,
        );

        // By Executable
        let by_exe = stats.map(|s| &s.by_executable).cloned().unwrap_or_default();
        self.render_breakdown_list(
            frame,
            bottom_cols[1],
            "By Executable",
            &by_exe,
            self.focus == StatsFocus::ByExecutable,
            theme,
        );

        // Hints panel
        self.render_hints(frame, bottom_cols[2], theme);
    }

    fn render_breakdown_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        data: &std::collections::HashMap<String, u64>,
        focused: bool,
        theme: &Theme,
    ) {
        let border_style = if focused {
            theme.border_focused()
        } else {
            theme.border()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", title));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if data.is_empty() {
            let msg = Paragraph::new("No data").style(theme.dim());
            frame.render_widget(msg, inner);
            return;
        }

        // Sort by count descending, take top entries that fit
        let mut sorted: Vec<_> = data.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));

        let max_items = (inner.height as usize).saturating_sub(1);
        let items: Vec<ListItem> = sorted
            .iter()
            .take(max_items)
            .map(|(key, count)| {
                let truncated = if key.len() > 20 {
                    format!("{}...", &key[..17])
                } else {
                    key.to_string()
                };
                ListItem::new(format!("{:20} {:>6}", truncated, count))
            })
            .collect();

        let list = List::new(items).style(theme.normal());
        frame.render_widget(list, inner);
    }

    fn render_hints(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(" Navigation ");

        frame.render_widget(block.clone(), area);

        let inner = block.inner(area);
        let current_focus = match self.focus {
            StatsFocus::Summary => "Summary",
            StatsFocus::ByProtocol => "By Protocol",
            StatsFocus::ByHost => "By Host",
            StatsFocus::ByPort => "By Port",
            StatsFocus::ByUser => "By User",
            StatsFocus::ByExecutable => "By Executable",
        };

        let hint_text = format!(
            "\n  Tab    = Next panel\n  S-Tab  = Previous panel\n  ↑/↓    = Scroll list\n  r      = Refresh stats\n\n  Current:\n    {}",
            current_focus
        );
        let para = Paragraph::new(hint_text).style(theme.dim());
        frame.render_widget(para, inner);
    }

    pub async fn handle_key(&mut self, key: KeyEvent, _state: &Arc<AppState>) {
        match key.code {
            KeyCode::Tab => {
                self.focus = self.focus.next();
            }
            KeyCode::BackTab => {
                self.focus = self.focus.prev();
            }
            _ => {}
        }
    }
}
