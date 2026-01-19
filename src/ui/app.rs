//! Main TUI application

use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Constraint,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame, Terminal,
};
use tokio::sync::{broadcast, mpsc};

use crate::app::events::{AppEvent, EventHandler, is_quit, tab_delta, tab_number};
use crate::app::state::{AppMessage, AppState, UiUpdateSignal};
use crate::ui::dialogs::prompt::PromptDialog;
use crate::ui::layout::AppLayout;
use crate::ui::tabs::{
    alerts::AlertsTab,
    connections::ConnectionsTab,
    firewall::FirewallTab,
    nodes::NodesTab,
    rules::RulesTab,
    statistics::StatisticsTab,
};
use crate::ui::theme::Theme;

/// Tab identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabId {
    Connections = 0,
    Rules = 1,
    Firewall = 2,
    Statistics = 3,
    Alerts = 4,
    Nodes = 5,
}

impl TabId {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Connections => "Connections",
            Self::Rules => "Rules",
            Self::Firewall => "Firewall",
            Self::Statistics => "Statistics",
            Self::Alerts => "Alerts",
            Self::Nodes => "Nodes",
        }
    }

    pub fn all() -> &'static [TabId] {
        &[
            Self::Connections,
            Self::Rules,
            Self::Firewall,
            Self::Statistics,
            Self::Alerts,
            Self::Nodes,
        ]
    }
}

/// Main TUI application
pub struct TuiApp {
    state: Arc<AppState>,
    state_tx: mpsc::Sender<AppMessage>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    event_handler: EventHandler,
    ui_update_rx: broadcast::Receiver<UiUpdateSignal>,

    // UI state
    current_tab: usize,
    theme: Theme,
    show_help: bool,
    show_prompt: bool,
    prompt_dialog: Option<PromptDialog>,

    // Tabs
    connections_tab: ConnectionsTab,
    rules_tab: RulesTab,
    firewall_tab: FirewallTab,
    statistics_tab: StatisticsTab,
    alerts_tab: AlertsTab,
    nodes_tab: NodesTab,
}

impl TuiApp {
    pub fn new(state: Arc<AppState>, state_tx: mpsc::Sender<AppMessage>) -> Result<Self> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let ui_update_rx = state.ui_update_tx.subscribe();

        Ok(Self {
            state,
            state_tx,
            terminal,
            event_handler: EventHandler::new(Duration::from_millis(100)),
            ui_update_rx,

            current_tab: 0,
            theme: Theme::default(),
            show_help: false,
            show_prompt: false,
            prompt_dialog: None,

            connections_tab: ConnectionsTab::new(),
            rules_tab: RulesTab::new(),
            firewall_tab: FirewallTab::new(),
            statistics_tab: StatisticsTab::new(),
            alerts_tab: AlertsTab::new(),
            nodes_tab: NodesTab::new(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            // Check for UI update signals
            while let Ok(signal) = self.ui_update_rx.try_recv() {
                match signal {
                    UiUpdateSignal::PromptReceived => {
                        let mut prompts = self.state.pending_prompts.write().await;
                        if let Some(pending) = prompts.pop_front() {
                            self.prompt_dialog = Some(PromptDialog::new(
                                pending.connection,
                                pending.node_addr,
                                pending.response_tx,
                            ));
                            self.show_prompt = true;
                        }
                    }
                    _ => {}
                }
            }

            // Update tab caches before drawing
            self.update_tab_caches().await;

            // Draw UI
            self.draw()?;

            // Handle input events
            if let Some(event) = self.event_handler.next() {
                match event {
                    AppEvent::Key(key) => {
                        if self.show_prompt {
                            if let Some(dialog) = &mut self.prompt_dialog {
                                if dialog.handle_key(key) {
                                    self.show_prompt = false;
                                    self.prompt_dialog = None;
                                }
                            }
                        } else if self.show_help {
                            self.show_help = false;
                        } else {
                            if is_quit(&key) {
                                break;
                            }

                            if key.code == crossterm::event::KeyCode::Char('?')
                                || key.code == crossterm::event::KeyCode::F(1)
                            {
                                self.show_help = true;
                                continue;
                            }

                            // Check if current tab has a dialog open - if so, pass keys to it first
                            let has_dialog = match TabId::all()[self.current_tab] {
                                TabId::Connections => self.connections_tab.showing_dialog(),
                                TabId::Rules => self.rules_tab.showing_dialog(),
                                _ => false,
                            };

                            // Only handle tab switching if no dialog is open
                            if !has_dialog {
                                if let Some(tab) = tab_number(&key) {
                                    if tab < TabId::all().len() {
                                        self.current_tab = tab;
                                    }
                                    continue;
                                }

                                if let Some(delta) = tab_delta(&key) {
                                    let len = TabId::all().len() as i32;
                                    self.current_tab = ((self.current_tab as i32 + delta).rem_euclid(len)) as usize;
                                    continue;
                                }
                            }

                            match TabId::all()[self.current_tab] {
                                TabId::Connections => self.connections_tab.handle_key(key, &self.state, &self.state_tx).await,
                                TabId::Rules => self.rules_tab.handle_key(key, &self.state, &self.state_tx).await,
                                TabId::Firewall => self.firewall_tab.handle_key(key, &self.state, &self.state_tx).await,
                                TabId::Statistics => self.statistics_tab.handle_key(key, &self.state).await,
                                TabId::Alerts => self.alerts_tab.handle_key(key, &self.state).await,
                                TabId::Nodes => self.nodes_tab.handle_key(key, &self.state).await,
                            }
                        }
                    }
                    AppEvent::Resize(_, _) => {}
                    AppEvent::Tick => {}
                }
            }
        }

        Ok(())
    }

    async fn update_tab_caches(&mut self) {
        match TabId::all()[self.current_tab] {
            TabId::Connections => self.connections_tab.update_cache(&self.state).await,
            TabId::Rules => self.rules_tab.update_cache(&self.state).await,
            TabId::Firewall => self.firewall_tab.update_cache(&self.state).await,
            TabId::Statistics => self.statistics_tab.update_cache(&self.state).await,
            TabId::Alerts => self.alerts_tab.update_cache(&self.state).await,
            TabId::Nodes => self.nodes_tab.update_cache(&self.state).await,
        }
    }

    fn draw(&mut self) -> Result<()> {
        let theme = &self.theme;
        let current_tab = self.current_tab;
        let show_help = self.show_help;
        let show_prompt = self.show_prompt;

        // Get status bar data synchronously using try_read
        let (connected_nodes, firewall_enabled, rule_count, connection_count, alert_count, uptime) = {
            // Try to get node info - use defaults if lock not available
            let nodes_guard = self.state.nodes.try_read();
            let (connected, fw, rules, up) = if let Ok(nodes) = nodes_guard {
                let active = nodes.active_node();
                (
                    nodes.connected_count(),
                    active.map(|n| n.firewall_running).unwrap_or(false),
                    active.map(|n| n.rules.len()).unwrap_or(0),
                    active
                        .and_then(|n| n.statistics.as_ref())
                        .map(|s| crate::utils::format_duration(s.uptime))
                        .unwrap_or_else(|| "N/A".to_string()),
                )
            } else {
                (0, false, 0, "N/A".to_string())
            };

            let conn_count = self.state.connections.try_read()
                .map(|c| c.len())
                .unwrap_or(0);

            let alert_cnt = self.state.alerts.try_read()
                .map(|a| a.len())
                .unwrap_or(0);

            (connected, fw, rules, conn_count, alert_cnt, up)
        };

        self.terminal.draw(|frame| {
            let layout = AppLayout::new(frame.area());

            // Tab bar
            let tab_titles: Vec<Line> = TabId::all()
                .iter()
                .enumerate()
                .map(|(i, tab)| {
                    let style = if i == current_tab {
                        theme.tab_active()
                    } else {
                        theme.tab_inactive()
                    };
                    Line::from(Span::styled(format!(" {} ", tab.title()), style))
                })
                .collect();

            let tabs = Tabs::new(tab_titles)
                .select(current_tab)
                .highlight_style(theme.tab_active())
                .divider("|");

            frame.render_widget(tabs, layout.tabs);

            // Content
            let content_block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border())
                .title(format!(" {} ", TabId::all()[current_tab].title()));

            let inner = content_block.inner(layout.content);
            frame.render_widget(content_block, layout.content);

            match TabId::all()[current_tab] {
                TabId::Connections => self.connections_tab.render(frame, inner, theme),
                TabId::Rules => self.rules_tab.render(frame, inner, theme),
                TabId::Firewall => self.firewall_tab.render(frame, inner, &self.state, theme),
                TabId::Statistics => self.statistics_tab.render(frame, inner, &self.state, theme),
                TabId::Alerts => self.alerts_tab.render(frame, inner, theme),
                TabId::Nodes => self.nodes_tab.render(frame, inner, theme),
            }

            // Status bar
            let daemon_status = if connected_nodes > 0 {
                Span::styled("● Connected", Style::default().fg(Color::Green))
            } else {
                Span::styled("○ Disconnected", Style::default().fg(Color::Red))
            };

            let firewall_status = if firewall_enabled {
                Span::styled("FW: ON", Style::default().fg(Color::Green))
            } else {
                Span::styled("FW: OFF", Style::default().fg(Color::Yellow))
            };

            let status_line = Line::from(vec![
                Span::raw(" "),
                daemon_status,
                Span::raw(" │ "),
                firewall_status,
                Span::raw(" │ "),
                Span::styled(format!("Rules: {}", rule_count), theme.normal()),
                Span::raw(" │ "),
                Span::styled(format!("Conns: {}", connection_count), theme.normal()),
                Span::raw(" │ "),
                Span::styled(format!("Alerts: {}", alert_count), theme.normal()),
                Span::raw(" │ "),
                Span::styled(format!("Up: {}", uptime), theme.normal()),
                Span::raw(" │ "),
                Span::styled("?=help q=quit", theme.dim()),
            ]);

            let status_bar = Paragraph::new(status_line);
            frame.render_widget(status_bar, layout.status);

            // Help overlay
            if show_help {
                render_help(frame, theme);
            }

            // Prompt dialog
            if show_prompt {
                if let Some(dialog) = &self.prompt_dialog {
                    dialog.render(frame, theme);
                }
            }
        })?;

        Ok(())
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

fn render_help(frame: &mut Frame, theme: &Theme) {
    let area = frame.area();
    let help_area = crate::ui::layout::DialogLayout::centered(area, 60, 20).dialog;

    let help_text = vec![
        "",
        "  OpenSnitch TUI - Keyboard Shortcuts",
        "  ────────────────────────────────────",
        "",
        "  Navigation:",
        "    1-6, Tab      Switch tabs",
        "    ↑/↓, j/k      Navigate list",
        "    PgUp/PgDn     Page up/down",
        "    Home/End      Go to top/bottom",
        "",
        "  Actions:",
        "    Enter         Select/confirm",
        "    e             Edit selected",
        "    d, Delete     Delete selected",
        "    n             New item",
        "    /             Filter",
        "    Esc           Clear filter/cancel",
        "",
        "  Press any key to close",
    ];

    let help_block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .style(theme.normal());

    let help_content = Paragraph::new(help_text.join("\n"))
        .block(help_block)
        .style(theme.normal());

    frame.render_widget(ratatui::widgets::Clear, help_area);
    frame.render_widget(help_content, help_area);
}
