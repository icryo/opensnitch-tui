//! SQLite database implementation

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::models::{
    Alert, AlertAction, AlertData, AlertPriority, AlertType, AlertWhat,
    Event, Operator, OperatorType, Rule, RuleAction, RuleDuration,
};

use super::{queries, schema};

/// SQLite database wrapper
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create database at the specified path
    pub fn open(path: &str) -> Result<Self> {
        let conn = if path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            // Create parent directory if needed
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            Connection::open(path)?
        };

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        // Create tables
        conn.execute_batch(schema::CREATE_TABLES)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert a connection event
    pub fn insert_connection(&self, event: &Event) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let c = &event.connection;

        conn.execute(
            queries::INSERT_CONNECTION,
            params![
                event.time,
                "", // node - set by caller
                event.rule.as_ref().map(|r| r.action.to_string()).unwrap_or_default(),
                c.protocol,
                c.src_ip,
                c.src_port.to_string(),
                c.dst_ip,
                c.dst_host,
                c.dst_port.to_string(),
                c.user_id.to_string(),
                c.process_id.to_string(),
                c.process_path,
                c.process_args.join(" "),
                c.process_cwd,
                event.rule.as_ref().map(|r| &r.name).unwrap_or(&String::new()),
            ],
        )?;

        // Update statistics
        if !c.dst_host.is_empty() {
            conn.execute(queries::UPDATE_STATS_HOST, params![c.dst_host])?;
        }
        conn.execute(queries::UPDATE_STATS_PROC, params![c.process_path])?;
        if !c.dst_ip.is_empty() {
            conn.execute(queries::UPDATE_STATS_ADDR, params![c.dst_ip])?;
        }
        conn.execute(queries::UPDATE_STATS_PORT, params![c.dst_port.to_string()])?;
        conn.execute(queries::UPDATE_STATS_USER, params![c.user_id.to_string()])?;

        Ok(())
    }

    /// Insert a rule
    pub fn insert_rule(&self, node: &str, rule: &Rule) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            queries::INSERT_RULE,
            params![
                Utc::now().to_rfc3339(),
                node,
                rule.name,
                rule.enabled.to_string(),
                rule.precedence.to_string(),
                rule.action.to_string(),
                rule.duration.to_string(),
                rule.operator.op_type.to_string(),
                rule.operator.sensitive.to_string(),
                rule.operator.operand,
                rule.operator.data,
                rule.description,
                rule.nolog.to_string(),
                rule.created.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Update an existing rule
    pub fn update_rule(&self, node: &str, rule: &Rule) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            queries::UPDATE_RULE,
            params![
                Utc::now().to_rfc3339(),
                node,
                rule.name,
                rule.enabled.to_string(),
                rule.precedence.to_string(),
                rule.action.to_string(),
                rule.duration.to_string(),
                rule.operator.op_type.to_string(),
                rule.operator.sensitive.to_string(),
                rule.operator.operand,
                rule.operator.data,
                rule.description,
                rule.nolog.to_string(),
            ],
        )?;

        Ok(())
    }

    /// Delete a rule
    pub fn delete_rule(&self, node: &str, name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(queries::DELETE_RULE, params![node, name])?;
        Ok(())
    }

    /// Insert an alert
    pub fn insert_alert(&self, alert: &Alert) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            queries::INSERT_ALERT,
            params![
                alert.timestamp.to_rfc3339(),
                alert.node,
                format!("{:?}", alert.alert_type),
                format!("{:?}", alert.action),
                format!("{:?}", alert.priority),
                format!("{:?}", alert.what),
                alert.text(),
                if alert.acknowledged { 1 } else { 0 },
            ],
        )?;

        Ok(())
    }

    /// Purge old connections
    pub fn purge_connections_before(&self, before: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(queries::PURGE_OLD_CONNECTIONS, params![before])?;
        Ok(count)
    }

    /// Purge old alerts
    pub fn purge_alerts_before(&self, before: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(queries::PURGE_OLD_ALERTS, params![before])?;
        Ok(count)
    }

    /// Get connection count
    pub fn connection_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM connections",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get rule count
    pub fn rule_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM rules",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get alert count
    pub fn alert_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM alerts",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Load recent connections from database
    pub fn select_connections(&self, limit: i64) -> Result<Vec<Event>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(queries::SELECT_CONNECTIONS)?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(Self::row_to_event(row))
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Load rules for a specific node from database
    pub fn select_rules(&self, node: &str) -> Result<Vec<Rule>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(queries::SELECT_RULES)?;
        let rows = stmt.query_map(params![node], |row| {
            Ok(Self::row_to_rule(row))
        })?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(row?);
        }
        Ok(rules)
    }

    /// Load recent alerts from database
    pub fn select_alerts(&self, limit: i64) -> Result<Vec<Alert>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(queries::SELECT_ALERTS)?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(Self::row_to_alert(row))
        })?;

        let mut alerts = Vec::new();
        for row in rows {
            alerts.push(row?);
        }
        Ok(alerts)
    }

    /// Load statistics by host
    pub fn select_stats_by_host(&self, limit: i64) -> Result<HashMap<String, u64>> {
        self.select_stats_table("hosts", limit)
    }

    /// Load statistics by process
    pub fn select_stats_by_proc(&self, limit: i64) -> Result<HashMap<String, u64>> {
        self.select_stats_table("procs", limit)
    }

    /// Load statistics by address
    pub fn select_stats_by_addr(&self, limit: i64) -> Result<HashMap<String, u64>> {
        self.select_stats_table("addrs", limit)
    }

    /// Load statistics by port
    pub fn select_stats_by_port(&self, limit: i64) -> Result<HashMap<String, u64>> {
        self.select_stats_table("ports", limit)
    }

    /// Load statistics by user
    pub fn select_stats_by_user(&self, limit: i64) -> Result<HashMap<String, u64>> {
        self.select_stats_table("users", limit)
    }

    fn select_stats_table(&self, table: &str, limit: i64) -> Result<HashMap<String, u64>> {
        let conn = self.conn.lock().unwrap();
        let query = format!(
            "SELECT what, hits FROM {} ORDER BY hits DESC LIMIT ?1",
            table
        );
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![limit], |row| {
            let what: String = row.get(0)?;
            let hits: i64 = row.get(1)?;
            Ok((what, hits as u64))
        })?;

        let mut map = HashMap::new();
        for row in rows {
            let (what, hits) = row?;
            map.insert(what, hits);
        }
        Ok(map)
    }

    fn row_to_event(row: &Row) -> Event {
        let time: String = row.get(0).unwrap_or_default();
        let _node: String = row.get(1).unwrap_or_default();
        let action: String = row.get(2).unwrap_or_default();
        let protocol: String = row.get(3).unwrap_or_default();
        let src_ip: String = row.get(4).unwrap_or_default();
        let src_port: String = row.get(5).unwrap_or_default();
        let dst_ip: String = row.get(6).unwrap_or_default();
        let dst_host: String = row.get(7).unwrap_or_default();
        let dst_port: String = row.get(8).unwrap_or_default();
        let uid: String = row.get(9).unwrap_or_default();
        let pid: String = row.get(10).unwrap_or_default();
        let process: String = row.get(11).unwrap_or_default();
        let process_args: String = row.get(12).unwrap_or_default();
        let process_cwd: String = row.get(13).unwrap_or_default();
        let rule_name: String = row.get(14).unwrap_or_default();

        let connection = crate::models::Connection {
            protocol,
            src_ip,
            src_port: src_port.parse().unwrap_or(0),
            dst_ip,
            dst_host,
            dst_port: dst_port.parse().unwrap_or(0),
            user_id: uid.parse().unwrap_or(0),
            process_id: pid.parse().unwrap_or(0),
            process_path: process,
            process_cwd,
            process_args: if process_args.is_empty() {
                Vec::new()
            } else {
                process_args.split(' ').map(String::from).collect()
            },
            process_env: HashMap::new(),
            process_checksums: HashMap::new(),
            process_tree: Vec::new(),
            timestamp: DateTime::parse_from_rfc3339(&time)
                .map(|dt| dt.with_timezone(&Utc))
                .ok(),
            action: Some(action),
            rule_name: if rule_name.is_empty() { None } else { Some(rule_name) },
        };

        Event {
            time,
            connection,
            rule: None,
            unix_nano: 0,
        }
    }

    fn row_to_rule(row: &Row) -> Rule {
        let _time: String = row.get(0).unwrap_or_default();
        let _node: String = row.get(1).unwrap_or_default();
        let name: String = row.get(2).unwrap_or_default();
        let enabled: String = row.get(3).unwrap_or_default();
        let precedence: String = row.get(4).unwrap_or_default();
        let action: String = row.get(5).unwrap_or_default();
        let duration: String = row.get(6).unwrap_or_default();
        let operator_type: String = row.get(7).unwrap_or_default();
        let operator_sensitive: String = row.get(8).unwrap_or_default();
        let operator_operand: String = row.get(9).unwrap_or_default();
        let operator_data: String = row.get(10).unwrap_or_default();
        let description: String = row.get(11).unwrap_or_default();
        let nolog: String = row.get(12).unwrap_or_default();
        let created: String = row.get(13).unwrap_or_default();

        Rule {
            name,
            description,
            enabled: enabled == "true",
            precedence: precedence == "true",
            nolog: nolog == "true",
            action: RuleAction::from(action.as_str()),
            duration: RuleDuration::from(duration.as_str()),
            operator: Operator {
                op_type: OperatorType::from(operator_type.as_str()),
                operand: operator_operand,
                data: operator_data,
                sensitive: operator_sensitive == "true",
                list: Vec::new(),
            },
            created: DateTime::parse_from_rfc3339(&created)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated: None,
        }
    }

    fn row_to_alert(row: &Row) -> Alert {
        let id: i64 = row.get(0).unwrap_or(0);
        let time: String = row.get(1).unwrap_or_default();
        let node: String = row.get(2).unwrap_or_default();
        let alert_type: String = row.get(3).unwrap_or_default();
        let action: String = row.get(4).unwrap_or_default();
        let priority: String = row.get(5).unwrap_or_default();
        let what: String = row.get(6).unwrap_or_default();
        let body: String = row.get(7).unwrap_or_default();
        let status: i32 = row.get(8).unwrap_or(0);

        let alert_type_enum = match alert_type.as_str() {
            "Error" => AlertType::Error,
            "Warning" => AlertType::Warning,
            "Info" => AlertType::Info,
            _ => AlertType::Info,
        };

        let action_enum = match action.as_str() {
            "None" => AlertAction::None,
            "ShowAlert" => AlertAction::ShowAlert,
            "SaveToDb" => AlertAction::SaveToDb,
            _ => AlertAction::None,
        };

        let priority_enum = match priority.as_str() {
            "Low" => AlertPriority::Low,
            "Medium" => AlertPriority::Medium,
            "High" => AlertPriority::High,
            _ => AlertPriority::Low,
        };

        let what_enum = match what.as_str() {
            "Generic" => AlertWhat::Generic,
            "ProcMonitor" => AlertWhat::ProcMonitor,
            "Firewall" => AlertWhat::Firewall,
            "Connection" => AlertWhat::Connection,
            "Rule" => AlertWhat::Rule,
            "Netlink" => AlertWhat::Netlink,
            "KernelEvent" => AlertWhat::KernelEvent,
            _ => AlertWhat::Generic,
        };

        Alert {
            id: id as u64,
            alert_type: alert_type_enum,
            action: action_enum,
            priority: priority_enum,
            what: what_enum,
            data: if body.is_empty() { None } else { Some(AlertData::Text(body)) },
            node,
            timestamp: DateTime::parse_from_rfc3339(&time)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            acknowledged: status != 0,
        }
    }
}
