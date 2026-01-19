use serde::{Deserialize, Serialize};
use std::fmt;

/// Operator types for rule matching
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatorType {
    Simple,
    Regexp,
    Network,
    List,
    Lists,
}

impl Default for OperatorType {
    fn default() -> Self {
        Self::Simple
    }
}

impl fmt::Display for OperatorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple => write!(f, "simple"),
            Self::Regexp => write!(f, "regexp"),
            Self::Network => write!(f, "network"),
            Self::List => write!(f, "list"),
            Self::Lists => write!(f, "lists"),
        }
    }
}

impl From<&str> for OperatorType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "simple" => Self::Simple,
            "regexp" => Self::Regexp,
            "network" => Self::Network,
            "list" => Self::List,
            "lists" => Self::Lists,
            _ => Self::Simple,
        }
    }
}

/// Operand - what the operator matches against
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operand {
    // Process-based
    #[serde(rename = "process.id")]
    ProcessId,
    #[serde(rename = "process.path")]
    ProcessPath,
    #[serde(rename = "process.command")]
    ProcessCommand,
    #[serde(rename = "process.env.")]
    ProcessEnv(String),
    #[serde(rename = "process.hash.md5")]
    ProcessHashMd5,
    #[serde(rename = "process.hash.sha1")]
    ProcessHashSha1,
    #[serde(rename = "process.parent.path")]
    ProcessParentPath,

    // User-based
    #[serde(rename = "user.id")]
    UserId,
    #[serde(rename = "user.name")]
    UserName,

    // Network-based
    #[serde(rename = "source.ip")]
    SourceIp,
    #[serde(rename = "source.port")]
    SourcePort,
    #[serde(rename = "source.network")]
    SourceNetwork,
    #[serde(rename = "dest.ip")]
    DestIp,
    #[serde(rename = "dest.host")]
    DestHost,
    #[serde(rename = "dest.port")]
    DestPort,
    #[serde(rename = "dest.network")]
    DestNetwork,
    #[serde(rename = "protocol")]
    Protocol,
    #[serde(rename = "iface.in")]
    IfaceIn,
    #[serde(rename = "iface.out")]
    IfaceOut,

    // List-based
    #[serde(rename = "list")]
    List,
    #[serde(rename = "lists.domains")]
    ListsDomains,
    #[serde(rename = "lists.domains_regexp")]
    ListsDomainsRegexp,
    #[serde(rename = "lists.ips")]
    ListsIps,
    #[serde(rename = "lists.nets")]
    ListsNets,
    #[serde(rename = "lists.hash.md5")]
    ListsHashMd5,

    // Unknown/custom
    Unknown(String),
}

impl Default for Operand {
    fn default() -> Self {
        Self::ProcessPath
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProcessId => write!(f, "process.id"),
            Self::ProcessPath => write!(f, "process.path"),
            Self::ProcessCommand => write!(f, "process.command"),
            Self::ProcessEnv(env) => write!(f, "process.env.{}", env),
            Self::ProcessHashMd5 => write!(f, "process.hash.md5"),
            Self::ProcessHashSha1 => write!(f, "process.hash.sha1"),
            Self::ProcessParentPath => write!(f, "process.parent.path"),
            Self::UserId => write!(f, "user.id"),
            Self::UserName => write!(f, "user.name"),
            Self::SourceIp => write!(f, "source.ip"),
            Self::SourcePort => write!(f, "source.port"),
            Self::SourceNetwork => write!(f, "source.network"),
            Self::DestIp => write!(f, "dest.ip"),
            Self::DestHost => write!(f, "dest.host"),
            Self::DestPort => write!(f, "dest.port"),
            Self::DestNetwork => write!(f, "dest.network"),
            Self::Protocol => write!(f, "protocol"),
            Self::IfaceIn => write!(f, "iface.in"),
            Self::IfaceOut => write!(f, "iface.out"),
            Self::List => write!(f, "list"),
            Self::ListsDomains => write!(f, "lists.domains"),
            Self::ListsDomainsRegexp => write!(f, "lists.domains_regexp"),
            Self::ListsIps => write!(f, "lists.ips"),
            Self::ListsNets => write!(f, "lists.nets"),
            Self::ListsHashMd5 => write!(f, "lists.hash.md5"),
            Self::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl From<&str> for Operand {
    fn from(s: &str) -> Self {
        match s {
            "process.id" => Self::ProcessId,
            "process.path" => Self::ProcessPath,
            "process.command" => Self::ProcessCommand,
            "process.hash.md5" => Self::ProcessHashMd5,
            "process.hash.sha1" => Self::ProcessHashSha1,
            "process.parent.path" => Self::ProcessParentPath,
            "user.id" => Self::UserId,
            "user.name" => Self::UserName,
            "source.ip" => Self::SourceIp,
            "source.port" => Self::SourcePort,
            "source.network" => Self::SourceNetwork,
            "dest.ip" => Self::DestIp,
            "dest.host" => Self::DestHost,
            "dest.port" => Self::DestPort,
            "dest.network" => Self::DestNetwork,
            "protocol" => Self::Protocol,
            "iface.in" => Self::IfaceIn,
            "iface.out" => Self::IfaceOut,
            "list" => Self::List,
            "lists.domains" => Self::ListsDomains,
            "lists.domains_regexp" => Self::ListsDomainsRegexp,
            "lists.ips" => Self::ListsIps,
            "lists.nets" => Self::ListsNets,
            "lists.hash.md5" => Self::ListsHashMd5,
            s if s.starts_with("process.env.") => {
                Self::ProcessEnv(s.strip_prefix("process.env.").unwrap_or("").to_string())
            }
            s => Self::Unknown(s.to_string()),
        }
    }
}

/// Operator for rule matching
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Operator {
    #[serde(rename = "type")]
    pub op_type: OperatorType,
    pub operand: String,
    pub data: String,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub list: Vec<Operator>,
}

impl Operator {
    pub fn new(op_type: OperatorType, operand: &str, data: &str) -> Self {
        Self {
            op_type,
            operand: operand.to_string(),
            data: data.to_string(),
            sensitive: false,
            list: Vec::new(),
        }
    }

    pub fn simple(operand: &str, data: &str) -> Self {
        Self::new(OperatorType::Simple, operand, data)
    }

    pub fn regexp(operand: &str, pattern: &str) -> Self {
        Self::new(OperatorType::Regexp, operand, pattern)
    }

    pub fn network(operand: &str, cidr: &str) -> Self {
        Self::new(OperatorType::Network, operand, cidr)
    }

    pub fn list(operators: Vec<Operator>) -> Self {
        Self {
            op_type: OperatorType::List,
            operand: "list".to_string(),
            data: String::new(),
            sensitive: false,
            list: operators,
        }
    }

    pub fn with_sensitive(mut self, sensitive: bool) -> Self {
        self.sensitive = sensitive;
        self
    }
}
