use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Rule, Statistics, SysFirewall};

/// Node connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected => write!(f, "Connected"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// A connected daemon node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub addr: String,
    pub name: String,
    pub version: String,
    pub status: NodeStatus,
    pub firewall_running: bool,
    pub log_level: u32,
    pub config: String,
    pub rules: Vec<Rule>,
    pub firewall: Option<SysFirewall>,
    pub statistics: Option<Statistics>,
    pub last_seen: DateTime<Utc>,
    pub connected_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub notifications_enabled: bool,
}

impl Node {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
            name: String::new(),
            version: String::new(),
            status: NodeStatus::Connecting,
            firewall_running: false,
            log_level: 0,
            config: String::new(),
            rules: Vec::new(),
            firewall: None,
            statistics: None,
            last_seen: Utc::now(),
            connected_at: None,
            notifications_enabled: false,
        }
    }

    pub fn update_from_config(&mut self, config: &ClientConfig) {
        self.name = config.name.clone();
        self.version = config.version.clone();
        self.firewall_running = config.is_firewall_running;
        self.log_level = config.log_level;
        self.config = config.config.clone();
        self.rules = config.rules.clone();
        self.firewall = config.system_firewall.clone();
        self.status = NodeStatus::Connected;
        self.connected_at = Some(Utc::now());
        self.last_seen = Utc::now();
    }

    pub fn disconnect(&mut self) {
        self.status = NodeStatus::Disconnected;
    }

    pub fn update_stats(&mut self, stats: Statistics) {
        self.statistics = Some(stats);
        self.last_seen = Utc::now();
    }

    pub fn uptime(&self) -> Option<u64> {
        self.statistics.as_ref().map(|s| s.uptime)
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            &self.addr
        } else {
            &self.name
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new("unknown")
    }
}

/// Client configuration received during Subscribe
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientConfig {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub is_firewall_running: bool,
    pub config: String,
    pub log_level: u32,
    pub rules: Vec<Rule>,
    pub system_firewall: Option<SysFirewall>,
}

/// Node manager for handling multiple daemon connections
#[derive(Debug, Default)]
pub struct NodeManager {
    pub nodes: HashMap<String, Node>,
    pub active_node: Option<String>,
}

impl NodeManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, addr: &str, config: ClientConfig) -> &mut Node {
        let node = self.nodes.entry(addr.to_string()).or_insert_with(|| {
            let mut n = Node::new(addr);
            n.notifications_enabled = true;
            n
        });
        node.update_from_config(&config);

        // Set as active if this is the first node
        if self.active_node.is_none() {
            self.active_node = Some(addr.to_string());
        }

        node
    }

    pub fn remove_node(&mut self, addr: &str) {
        if let Some(node) = self.nodes.get_mut(addr) {
            node.disconnect();
        }

        // If this was the active node, switch to another
        if self.active_node.as_deref() == Some(addr) {
            self.active_node = self.nodes
                .iter()
                .find(|(_, n)| n.status == NodeStatus::Connected)
                .map(|(a, _)| a.clone());
        }
    }

    pub fn get_node(&self, addr: &str) -> Option<&Node> {
        self.nodes.get(addr)
    }

    pub fn get_node_mut(&mut self, addr: &str) -> Option<&mut Node> {
        self.nodes.get_mut(addr)
    }

    pub fn active_node(&self) -> Option<&Node> {
        self.active_node.as_ref().and_then(|a| self.nodes.get(a))
    }

    pub fn active_node_mut(&mut self) -> Option<&mut Node> {
        let addr = self.active_node.clone()?;
        self.nodes.get_mut(&addr)
    }

    pub fn active_addr(&self) -> Option<&str> {
        self.active_node.as_deref()
    }

    pub fn connected_nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values().filter(|n| n.status == NodeStatus::Connected)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn connected_count(&self) -> usize {
        self.nodes.values().filter(|n| n.status == NodeStatus::Connected).count()
    }
}
