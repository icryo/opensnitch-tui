use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Process information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Process {
    pub pid: u64,
    pub ppid: u64,
    pub uid: u64,
    pub comm: String,
    pub path: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: String,
    pub checksums: HashMap<String, String>,
    pub io_reads: u64,
    pub io_writes: u64,
    pub net_reads: u64,
    pub net_writes: u64,
    pub process_tree: Vec<(String, u32)>,
}

impl Process {
    pub fn command_line(&self) -> String {
        if self.args.is_empty() {
            self.path.clone()
        } else {
            format!("{} {}", self.path, self.args.join(" "))
        }
    }

    pub fn basename(&self) -> &str {
        self.path
            .rsplit('/')
            .next()
            .unwrap_or(&self.path)
    }
}

/// A network connection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Connection {
    pub protocol: String,
    pub src_ip: String,
    pub src_port: u32,
    pub dst_ip: String,
    pub dst_host: String,
    pub dst_port: u32,
    pub user_id: u32,
    pub process_id: u32,
    pub process_path: String,
    pub process_cwd: String,
    pub process_args: Vec<String>,
    pub process_env: HashMap<String, String>,
    pub process_checksums: HashMap<String, String>,
    pub process_tree: Vec<(String, u32)>,
    // Additional fields for display
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub rule_name: Option<String>,
}

impl Connection {
    pub fn destination(&self) -> String {
        if self.dst_host.is_empty() {
            format!("{}:{}", self.dst_ip, self.dst_port)
        } else {
            format!("{}:{}", self.dst_host, self.dst_port)
        }
    }

    pub fn source(&self) -> String {
        format!("{}:{}", self.src_ip, self.src_port)
    }

    pub fn process_name(&self) -> &str {
        self.process_path
            .rsplit('/')
            .next()
            .unwrap_or(&self.process_path)
    }

    pub fn command_line(&self) -> String {
        if self.process_args.is_empty() {
            self.process_path.clone()
        } else {
            format!("{} {}", self.process_path, self.process_args.join(" "))
        }
    }
}

/// An event containing a connection and its matched rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub time: String,
    pub connection: Connection,
    pub rule: Option<super::Rule>,
    pub unix_nano: i64,
}

impl Event {
    pub fn new(connection: Connection, rule: Option<super::Rule>) -> Self {
        Self {
            time: Utc::now().to_rfc3339(),
            connection,
            rule,
            unix_nano: Utc::now().timestamp_nanos_opt().unwrap_or(0),
        }
    }
}
