use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Event;

/// Daemon statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Statistics {
    pub daemon_version: String,
    pub rules: u64,
    pub uptime: u64,
    pub dns_responses: u64,
    pub connections: u64,
    pub ignored: u64,
    pub accepted: u64,
    pub dropped: u64,
    pub rule_hits: u64,
    pub rule_misses: u64,
    pub by_proto: HashMap<String, u64>,
    pub by_address: HashMap<String, u64>,
    pub by_host: HashMap<String, u64>,
    pub by_port: HashMap<String, u64>,
    pub by_uid: HashMap<String, u64>,
    pub by_executable: HashMap<String, u64>,
    pub events: Vec<Event>,
}

impl Statistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn uptime_string(&self) -> String {
        let secs = self.uptime;
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;

        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, mins, secs)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, mins, secs)
        } else if mins > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}s", secs)
        }
    }

    /// Get top N entries from a map, sorted by value descending
    pub fn top_n<'a>(map: &'a HashMap<String, u64>, n: usize) -> Vec<(&'a String, &'a u64)> {
        let mut entries: Vec<_> = map.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        entries.truncate(n);
        entries
    }
}

/// Aggregated statistics for display
#[derive(Debug, Clone, Default)]
pub struct AggregatedStats {
    pub total_connections: u64,
    pub total_allowed: u64,
    pub total_denied: u64,
    pub total_rules: u64,
    pub by_protocol: HashMap<String, u64>,
    pub by_host: HashMap<String, u64>,
    pub by_port: HashMap<String, u64>,
    pub by_user: HashMap<String, u64>,
    pub by_executable: HashMap<String, u64>,
}

impl AggregatedStats {
    pub fn merge(&mut self, stats: &Statistics) {
        self.total_connections += stats.connections;
        self.total_allowed += stats.accepted;
        self.total_denied += stats.dropped;
        self.total_rules = stats.rules;

        for (k, v) in &stats.by_proto {
            *self.by_protocol.entry(k.clone()).or_insert(0) += v;
        }
        for (k, v) in &stats.by_host {
            *self.by_host.entry(k.clone()).or_insert(0) += v;
        }
        for (k, v) in &stats.by_port {
            *self.by_port.entry(k.clone()).or_insert(0) += v;
        }
        for (k, v) in &stats.by_uid {
            *self.by_user.entry(k.clone()).or_insert(0) += v;
        }
        for (k, v) in &stats.by_executable {
            *self.by_executable.entry(k.clone()).or_insert(0) += v;
        }
    }
}
