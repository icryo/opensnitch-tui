//! Application settings

use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::models::{RuleAction, RuleDuration};

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// gRPC socket address
    pub socket_address: String,

    /// Database file path
    pub database_path: String,

    /// Default action when prompt times out
    pub default_action: RuleAction,

    /// Default rule duration
    pub default_duration: RuleDuration,

    /// Prompt timeout in seconds
    pub prompt_timeout: u64,

    /// Maximum connections to keep in memory
    pub max_connections: usize,

    /// Maximum alerts to keep in memory
    pub max_alerts: usize,

    /// Log level
    pub log_level: String,

    /// Theme name
    pub theme: String,

    /// Show notifications
    pub show_notifications: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            socket_address: "unix:///tmp/osui.sock".to_string(),
            database_path: Self::default_db_path()
                .to_string_lossy()
                .to_string(),
            default_action: RuleAction::Allow, // User preference: permissive
            default_duration: RuleDuration::Once,
            prompt_timeout: 15,
            max_connections: 1000,
            max_alerts: 500,
            log_level: "info".to_string(),
            theme: "default".to_string(),
            show_notifications: true,
        }
    }
}

impl Settings {
    /// Load settings from file or create default
    pub fn load(path: Option<&str>) -> Result<Self> {
        let config_path = path
            .map(PathBuf::from)
            .unwrap_or_else(Self::default_config_path);

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let settings: Self = serde_json::from_str(&content)?;
            Ok(settings)
        } else {
            Ok(Self::default())
        }
    }

    /// Save settings to file
    pub fn save(&self, path: Option<&str>) -> Result<()> {
        let config_path = path
            .map(PathBuf::from)
            .unwrap_or_else(Self::default_config_path);

        // Create parent directory if needed
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get default config directory
    pub fn config_dir() -> PathBuf {
        ProjectDirs::from("com", "opensnitch", "opensnitch-tui")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config")
                    .join("opensnitch-tui")
            })
    }

    /// Get default config file path
    pub fn default_config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    /// Get default database path
    pub fn default_db_path() -> PathBuf {
        Self::config_dir().join("opensnitch.db")
    }
}
