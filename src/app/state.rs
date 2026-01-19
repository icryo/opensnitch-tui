//! Application state management

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot, RwLock};

use crate::db::Database;
use crate::grpc::notifications::{NotificationAction, NotificationIdGenerator};
use crate::grpc::proto;
use crate::models::{
    Alert, Connection, Event, Node, NodeManager, Rule, Statistics, SysFirewall,
    node::ClientConfig,
};

/// Messages for state updates
#[derive(Debug)]
pub enum AppMessage {
    // Node events
    NodeConnected {
        addr: String,
        config: ClientConfig,
    },
    NodeDisconnected {
        addr: String,
    },
    StatsUpdate {
        node_addr: String,
        stats: Statistics,
    },
    NotificationChannelOpened {
        node_addr: String,
        tx: mpsc::Sender<proto::Notification>,
    },
    NotificationReply {
        node_addr: String,
        id: u64,
        code: i32,
        data: String,
    },

    // Connection events
    ConnectionEvent {
        node_addr: String,
        event: Event,
    },
    NewConnection {
        node_addr: String,
        connection: Connection,
    },
    ConnectionPrompt {
        node_addr: String,
        connection: Connection,
        response_tx: oneshot::Sender<Rule>,
    },

    // Rule events
    RuleAdded {
        node_addr: String,
        rule: Rule,
    },
    RuleModified {
        node_addr: String,
        rule: Rule,
    },
    RuleDeleted {
        node_addr: String,
        name: String,
    },
    RuleToggled {
        node_addr: String,
        name: String,
        enabled: bool,
    },

    // Firewall events
    FirewallConfigUpdate {
        node_addr: String,
        config: SysFirewall,
    },

    // Alert events
    AlertReceived {
        alert: Alert,
    },

    // User actions
    SendNotification {
        node_addr: String,
        action: NotificationAction,
    },
    PromptResponse {
        rule: Rule,
    },
}

/// UI update signals
#[derive(Debug, Clone)]
pub enum UiUpdateSignal {
    NodeChanged,
    StatsUpdated,
    ConnectionsUpdated,
    RulesUpdated,
    FirewallUpdated,
    AlertsUpdated,
    PromptReceived,
    Redraw,
}

/// Pending prompt for user interaction
pub struct PendingPrompt {
    pub connection: Connection,
    pub node_addr: String,
    pub response_tx: oneshot::Sender<Rule>,
}

/// Central application state
pub struct AppState {
    pub nodes: RwLock<NodeManager>,
    pub connections: RwLock<VecDeque<Event>>,
    pub alerts: RwLock<VecDeque<Alert>>,
    pub pending_prompts: RwLock<VecDeque<PendingPrompt>>,
    pub notification_channels: RwLock<HashMap<String, mpsc::Sender<proto::Notification>>>,
    pub notification_id_gen: NotificationIdGenerator,
    pub db: Database,
    pub ui_update_tx: broadcast::Sender<UiUpdateSignal>,

    // Configuration
    pub max_connections: usize,
    pub max_alerts: usize,
}

impl AppState {
    pub fn new(db: Database, ui_update_tx: broadcast::Sender<UiUpdateSignal>) -> Self {
        Self {
            nodes: RwLock::new(NodeManager::new()),
            connections: RwLock::new(VecDeque::with_capacity(1000)),
            alerts: RwLock::new(VecDeque::with_capacity(500)),
            pending_prompts: RwLock::new(VecDeque::new()),
            notification_channels: RwLock::new(HashMap::new()),
            notification_id_gen: NotificationIdGenerator::new(),
            db,
            ui_update_tx,
            max_connections: 1000,
            max_alerts: 500,
        }
    }

    pub fn notify_ui(&self, signal: UiUpdateSignal) {
        let _ = self.ui_update_tx.send(signal);
    }

    pub async fn add_connection(&self, event: Event) {
        let mut connections = self.connections.write().await;
        connections.push_front(event.clone());
        while connections.len() > self.max_connections {
            connections.pop_back();
        }

        // Persist to database
        if let Err(e) = self.db.insert_connection(&event) {
            tracing::error!("Failed to persist connection: {}", e);
        }
    }

    pub async fn add_alert(&self, alert: Alert) {
        let mut alerts = self.alerts.write().await;
        alerts.push_front(alert.clone());
        while alerts.len() > self.max_alerts {
            alerts.pop_back();
        }

        // Persist to database
        if let Err(e) = self.db.insert_alert(&alert) {
            tracing::error!("Failed to persist alert: {}", e);
        }
    }

    pub async fn get_active_node(&self) -> Option<Node> {
        let nodes = self.nodes.read().await;
        nodes.active_node().cloned()
    }

    pub async fn send_notification(&self, node_addr: &str, action: NotificationAction) {
        let channels = self.notification_channels.read().await;
        if let Some(tx) = channels.get(node_addr) {
            let notification = crate::grpc::notifications::create_notification(
                self.notification_id_gen.next(),
                node_addr,
                "opensnitch-tui",
                action,
                None,
            );
            if let Err(e) = tx.send(notification).await {
                tracing::error!("Failed to send notification to {}: {}", node_addr, e);
            }
        } else {
            tracing::warn!("No notification channel for node {}", node_addr);
        }
    }
}

/// Run the state manager task
pub async fn run_state_manager(
    state: Arc<AppState>,
    mut rx: mpsc::Receiver<AppMessage>,
    ui_update_tx: broadcast::Sender<UiUpdateSignal>,
) {
    tracing::info!("State manager started");

    while let Some(msg) = rx.recv().await {
        match msg {
            AppMessage::NodeConnected { addr, config } => {
                tracing::info!("Node connected: {} ({})", config.name, addr);
                let mut nodes = state.nodes.write().await;
                nodes.add_node(&addr, config);
                drop(nodes);
                let _ = ui_update_tx.send(UiUpdateSignal::NodeChanged);
            }

            AppMessage::NodeDisconnected { addr } => {
                tracing::info!("Node disconnected: {}", addr);
                let mut nodes = state.nodes.write().await;
                nodes.remove_node(&addr);
                drop(nodes);

                // Remove notification channel
                let mut channels = state.notification_channels.write().await;
                channels.remove(&addr);
                drop(channels);

                let _ = ui_update_tx.send(UiUpdateSignal::NodeChanged);
            }

            AppMessage::StatsUpdate { node_addr, stats } => {
                // Add events to connections list
                let has_events = !stats.events.is_empty();
                for event in &stats.events {
                    state.add_connection(event.clone()).await;
                }

                let mut nodes = state.nodes.write().await;
                if let Some(node) = nodes.get_node_mut(&node_addr) {
                    node.update_stats(stats);
                }
                drop(nodes);

                let _ = ui_update_tx.send(UiUpdateSignal::StatsUpdated);
                if has_events {
                    let _ = ui_update_tx.send(UiUpdateSignal::ConnectionsUpdated);
                }
            }

            AppMessage::NotificationChannelOpened { node_addr, tx } => {
                let mut channels = state.notification_channels.write().await;
                channels.insert(node_addr, tx);
            }

            AppMessage::NotificationReply { node_addr, id, code, data } => {
                tracing::debug!(
                    "Notification reply from {}: id={} code={} data={}",
                    node_addr, id, code, data
                );
            }

            AppMessage::ConnectionPrompt { node_addr, connection, response_tx } => {
                tracing::info!(
                    "Connection prompt: {} -> {}",
                    connection.process_name(),
                    connection.destination()
                );
                let mut prompts = state.pending_prompts.write().await;
                prompts.push_back(PendingPrompt {
                    connection,
                    node_addr,
                    response_tx,
                });
                drop(prompts);
                let _ = ui_update_tx.send(UiUpdateSignal::PromptReceived);
            }

            AppMessage::ConnectionEvent { node_addr: _, event } => {
                state.add_connection(event).await;
                let _ = ui_update_tx.send(UiUpdateSignal::ConnectionsUpdated);
            }

            AppMessage::NewConnection { node_addr: _, connection } => {
                // Convert connection to event for monitoring
                let event = Event::new(connection, None);
                state.add_connection(event).await;
                let _ = ui_update_tx.send(UiUpdateSignal::ConnectionsUpdated);
            }

            AppMessage::RuleAdded { node_addr, rule } => {
                let mut nodes = state.nodes.write().await;
                if let Some(node) = nodes.get_node_mut(&node_addr) {
                    node.rules.push(rule.clone());
                }
                drop(nodes);

                if let Err(e) = state.db.insert_rule(&node_addr, &rule) {
                    tracing::error!("Failed to persist rule: {}", e);
                }

                let _ = ui_update_tx.send(UiUpdateSignal::RulesUpdated);
            }

            AppMessage::RuleModified { node_addr, rule } => {
                let mut nodes = state.nodes.write().await;
                if let Some(node) = nodes.get_node_mut(&node_addr) {
                    if let Some(existing) = node.rules.iter_mut().find(|r| r.name == rule.name) {
                        *existing = rule.clone();
                    }
                }
                drop(nodes);

                if let Err(e) = state.db.update_rule(&node_addr, &rule) {
                    tracing::error!("Failed to update rule: {}", e);
                }

                let _ = ui_update_tx.send(UiUpdateSignal::RulesUpdated);
            }

            AppMessage::RuleDeleted { node_addr, name } => {
                let mut nodes = state.nodes.write().await;
                if let Some(node) = nodes.get_node_mut(&node_addr) {
                    node.rules.retain(|r| r.name != name);
                }
                drop(nodes);

                if let Err(e) = state.db.delete_rule(&node_addr, &name) {
                    tracing::error!("Failed to delete rule: {}", e);
                }

                let _ = ui_update_tx.send(UiUpdateSignal::RulesUpdated);
            }

            AppMessage::RuleToggled { node_addr, name, enabled } => {
                let mut nodes = state.nodes.write().await;
                if let Some(node) = nodes.get_node_mut(&node_addr) {
                    if let Some(rule) = node.rules.iter_mut().find(|r| r.name == name) {
                        rule.enabled = enabled;
                    }
                }
                drop(nodes);
                let _ = ui_update_tx.send(UiUpdateSignal::RulesUpdated);
            }

            AppMessage::FirewallConfigUpdate { node_addr, config } => {
                let mut nodes = state.nodes.write().await;
                if let Some(node) = nodes.get_node_mut(&node_addr) {
                    node.firewall = Some(config);
                }
                drop(nodes);
                let _ = ui_update_tx.send(UiUpdateSignal::FirewallUpdated);
            }

            AppMessage::AlertReceived { alert } => {
                state.add_alert(alert).await;
                let _ = ui_update_tx.send(UiUpdateSignal::AlertsUpdated);
            }

            AppMessage::SendNotification { node_addr, action } => {
                state.send_notification(&node_addr, action).await;
            }

            AppMessage::PromptResponse { rule } => {
                // This is handled by the prompt dialog
                tracing::debug!("Prompt response: {} - {}", rule.action, rule.name);
            }
        }
    }

    tracing::info!("State manager stopped");
}
