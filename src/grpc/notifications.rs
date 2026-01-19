//! Notification handling for daemon communication

use crate::grpc::proto;
use crate::models;

/// Actions that can be sent to daemons via notifications
#[derive(Debug, Clone)]
pub enum NotificationAction {
    EnableInterception,
    DisableInterception,
    EnableFirewall,
    DisableFirewall,
    ReloadFwRules,
    ChangeConfig(String),
    EnableRule(String),
    DisableRule(String),
    DeleteRule(String),
    ChangeRule(models::Rule),
    SetLogLevel(u32),
    Stop,
    TaskStart { name: String, data: String },
    TaskStop { name: String },
}

impl NotificationAction {
    /// Convert to protobuf Action enum
    pub fn to_proto_action(&self) -> i32 {
        match self {
            Self::EnableInterception => proto::Action::EnableInterception as i32,
            Self::DisableInterception => proto::Action::DisableInterception as i32,
            Self::EnableFirewall => proto::Action::EnableFirewall as i32,
            Self::DisableFirewall => proto::Action::DisableFirewall as i32,
            Self::ReloadFwRules => proto::Action::ReloadFwRules as i32,
            Self::ChangeConfig(_) => proto::Action::ChangeConfig as i32,
            Self::EnableRule(_) => proto::Action::EnableRule as i32,
            Self::DisableRule(_) => proto::Action::DisableRule as i32,
            Self::DeleteRule(_) => proto::Action::DeleteRule as i32,
            Self::ChangeRule(_) => proto::Action::ChangeRule as i32,
            Self::SetLogLevel(_) => proto::Action::LogLevel as i32,
            Self::Stop => proto::Action::Stop as i32,
            Self::TaskStart { .. } => proto::Action::TaskStart as i32,
            Self::TaskStop { .. } => proto::Action::TaskStop as i32,
        }
    }

    /// Get data payload for the notification
    pub fn data(&self) -> String {
        match self {
            Self::ChangeConfig(config) => config.clone(),
            Self::EnableRule(name) | Self::DisableRule(name) | Self::DeleteRule(name) => {
                name.clone()
            }
            Self::ChangeRule(rule) => serde_json::to_string(rule).unwrap_or_default(),
            Self::SetLogLevel(level) => level.to_string(),
            Self::TaskStart { name, data } => {
                serde_json::json!({ "Name": name, "Data": data }).to_string()
            }
            Self::TaskStop { name } => {
                serde_json::json!({ "Name": name }).to_string()
            }
            _ => String::new(),
        }
    }

    /// Get rules to include in notification (for rule changes)
    pub fn rules(&self) -> Vec<models::Rule> {
        match self {
            Self::ChangeRule(rule) => vec![rule.clone()],
            _ => Vec::new(),
        }
    }
}

/// Create a notification message for sending to daemon
pub fn create_notification(
    id: u64,
    client_name: &str,
    server_name: &str,
    action: NotificationAction,
    firewall: Option<models::SysFirewall>,
) -> proto::Notification {
    proto::Notification {
        id,
        client_name: client_name.to_string(),
        server_name: server_name.to_string(),
        r#type: action.to_proto_action(),
        data: action.data(),
        rules: action.rules().into_iter().map(Into::into).collect(),
        sys_firewall: firewall.map(Into::into),
    }
}

/// Notification ID generator
pub struct NotificationIdGenerator {
    next_id: std::sync::atomic::AtomicU64,
}

impl NotificationIdGenerator {
    pub fn new() -> Self {
        Self {
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    pub fn next(&self) -> u64 {
        self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for NotificationIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}
