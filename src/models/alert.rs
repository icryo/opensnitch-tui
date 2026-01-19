use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Alert priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertPriority {
    Low = 0,
    Medium = 1,
    High = 2,
}

impl Default for AlertPriority {
    fn default() -> Self {
        Self::Low
    }
}

impl From<i32> for AlertPriority {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::Low,
            1 => Self::Medium,
            2 => Self::High,
            _ => Self::Low,
        }
    }
}

/// Alert types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    Error = 0,
    Warning = 1,
    Info = 2,
}

impl Default for AlertType {
    fn default() -> Self {
        Self::Info
    }
}

impl From<i32> for AlertType {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::Error,
            1 => Self::Warning,
            2 => Self::Info,
            _ => Self::Info,
        }
    }
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "ERROR"),
            Self::Warning => write!(f, "WARNING"),
            Self::Info => write!(f, "INFO"),
        }
    }
}

/// Alert action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertAction {
    None = 0,
    ShowAlert = 1,
    SaveToDb = 2,
}

impl Default for AlertAction {
    fn default() -> Self {
        Self::None
    }
}

impl From<i32> for AlertAction {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::None,
            1 => Self::ShowAlert,
            2 => Self::SaveToDb,
            _ => Self::None,
        }
    }
}

/// What caused the alert
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertWhat {
    Generic = 0,
    ProcMonitor = 1,
    Firewall = 2,
    Connection = 3,
    Rule = 4,
    Netlink = 5,
    KernelEvent = 6,
}

impl Default for AlertWhat {
    fn default() -> Self {
        Self::Generic
    }
}

impl From<i32> for AlertWhat {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::Generic,
            1 => Self::ProcMonitor,
            2 => Self::Firewall,
            3 => Self::Connection,
            4 => Self::Rule,
            5 => Self::Netlink,
            6 => Self::KernelEvent,
            _ => Self::Generic,
        }
    }
}

impl std::fmt::Display for AlertWhat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generic => write!(f, "Generic"),
            Self::ProcMonitor => write!(f, "Process Monitor"),
            Self::Firewall => write!(f, "Firewall"),
            Self::Connection => write!(f, "Connection"),
            Self::Rule => write!(f, "Rule"),
            Self::Netlink => write!(f, "Netlink"),
            Self::KernelEvent => write!(f, "Kernel Event"),
        }
    }
}

/// Alert data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertData {
    Text(String),
    Process(super::connection::Process),
    Connection(super::Connection),
    Rule(super::Rule),
    FirewallRule(super::FwRule),
}

/// An alert from the daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: u64,
    pub alert_type: AlertType,
    pub action: AlertAction,
    pub priority: AlertPriority,
    pub what: AlertWhat,
    pub data: Option<AlertData>,
    pub node: String,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub acknowledged: bool,
}

impl Alert {
    pub fn new(
        id: u64,
        alert_type: AlertType,
        priority: AlertPriority,
        what: AlertWhat,
        data: Option<AlertData>,
    ) -> Self {
        Self {
            id,
            alert_type,
            action: AlertAction::ShowAlert,
            priority,
            what,
            data,
            node: String::new(),
            timestamp: Utc::now(),
            acknowledged: false,
        }
    }

    pub fn text(&self) -> String {
        match &self.data {
            Some(AlertData::Text(s)) => s.clone(),
            Some(AlertData::Connection(c)) => {
                format!("{} -> {}", c.process_name(), c.destination())
            }
            Some(AlertData::Rule(r)) => format!("Rule: {}", r.name),
            Some(AlertData::Process(p)) => format!("Process: {} ({})", p.comm, p.pid),
            Some(AlertData::FirewallRule(r)) => format!("FW Rule: {}", r.description),
            None => String::new(),
        }
    }
}
