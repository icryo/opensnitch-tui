use super::operator::Operator;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Rule action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Deny,
    Reject,
}

impl Default for RuleAction {
    fn default() -> Self {
        Self::Allow
    }
}

impl fmt::Display for RuleAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::Deny => write!(f, "deny"),
            Self::Reject => write!(f, "reject"),
        }
    }
}

impl From<&str> for RuleAction {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "allow" => Self::Allow,
            "deny" => Self::Deny,
            "reject" => Self::Reject,
            _ => Self::Allow,
        }
    }
}

/// Rule duration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleDuration {
    Once,
    #[serde(rename = "until restart")]
    UntilRestart,
    Always,
    // Time-based durations
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "30m")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "12h")]
    TwelveHours,
    #[serde(rename = "24h")]
    TwentyFourHours,
}

impl Default for RuleDuration {
    fn default() -> Self {
        Self::Once
    }
}

impl fmt::Display for RuleDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Once => write!(f, "once"),
            Self::UntilRestart => write!(f, "until restart"),
            Self::Always => write!(f, "always"),
            Self::FiveMinutes => write!(f, "5m"),
            Self::FifteenMinutes => write!(f, "15m"),
            Self::ThirtyMinutes => write!(f, "30m"),
            Self::OneHour => write!(f, "1h"),
            Self::TwelveHours => write!(f, "12h"),
            Self::TwentyFourHours => write!(f, "24h"),
        }
    }
}

impl From<&str> for RuleDuration {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "once" => Self::Once,
            "until restart" => Self::UntilRestart,
            "always" => Self::Always,
            "5m" => Self::FiveMinutes,
            "15m" => Self::FifteenMinutes,
            "30m" => Self::ThirtyMinutes,
            "1h" => Self::OneHour,
            "12h" => Self::TwelveHours,
            "24h" => Self::TwentyFourHours,
            _ => Self::Once,
        }
    }
}

impl RuleDuration {
    /// Returns duration in seconds, None for permanent durations
    pub fn as_seconds(&self) -> Option<u64> {
        match self {
            Self::Once => None,
            Self::UntilRestart => None,
            Self::Always => None,
            Self::FiveMinutes => Some(5 * 60),
            Self::FifteenMinutes => Some(15 * 60),
            Self::ThirtyMinutes => Some(30 * 60),
            Self::OneHour => Some(60 * 60),
            Self::TwelveHours => Some(12 * 60 * 60),
            Self::TwentyFourHours => Some(24 * 60 * 60),
        }
    }

    pub fn is_temporary(&self) -> bool {
        matches!(
            self,
            Self::Once
                | Self::FiveMinutes
                | Self::FifteenMinutes
                | Self::ThirtyMinutes
                | Self::OneHour
                | Self::TwelveHours
                | Self::TwentyFourHours
        )
    }
}

/// A firewall rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub precedence: bool,
    #[serde(default)]
    pub nolog: bool,
    pub action: RuleAction,
    pub duration: RuleDuration,
    pub operator: Operator,
    #[serde(default = "Utc::now")]
    pub created: DateTime<Utc>,
    #[serde(default)]
    pub updated: Option<DateTime<Utc>>,
}

fn default_true() -> bool {
    true
}

impl Rule {
    pub fn new(name: &str, action: RuleAction, duration: RuleDuration, operator: Operator) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            enabled: true,
            precedence: false,
            nolog: false,
            action,
            duration,
            operator,
            created: Utc::now(),
            updated: None,
        }
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn with_precedence(mut self, precedence: bool) -> Self {
        self.precedence = precedence;
        self
    }

    pub fn with_nolog(mut self, nolog: bool) -> Self {
        self.nolog = nolog;
        self
    }

    /// Generate a slug-based filename for this rule
    pub fn filename(&self) -> String {
        let slug: String = self
            .name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        format!("{}.json", slug)
    }
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            enabled: true,
            precedence: false,
            nolog: false,
            action: RuleAction::Allow,
            duration: RuleDuration::Once,
            operator: Operator::default(),
            created: Utc::now(),
            updated: None,
        }
    }
}
