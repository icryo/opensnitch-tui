use serde::{Deserialize, Serialize};

/// Statement values for nftables expressions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatementValue {
    pub key: String,
    pub value: String,
}

/// nftables statement
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Statement {
    pub op: String,
    pub name: String,
    pub values: Vec<StatementValue>,
}

/// nftables expression
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Expression {
    pub statement: Statement,
}

/// Firewall rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FwRule {
    // DEPRECATED: for backward compatibility with iptables
    #[serde(default)]
    pub table: String,
    #[serde(default)]
    pub chain: String,

    pub uuid: String,
    pub enabled: bool,
    pub position: u64,
    pub description: String,
    #[serde(default)]
    pub parameters: String,
    pub expressions: Vec<Expression>,
    pub target: String,
    #[serde(default)]
    pub target_parameters: String,
}

impl Default for FwRule {
    fn default() -> Self {
        Self {
            table: String::new(),
            chain: String::new(),
            uuid: uuid::Uuid::new_v4().to_string(),
            enabled: true,
            position: 0,
            description: String::new(),
            parameters: String::new(),
            expressions: Vec::new(),
            target: "accept".to_string(),
            target_parameters: String::new(),
        }
    }
}

impl FwRule {
    pub fn new(description: &str, target: &str) -> Self {
        Self {
            description: description.to_string(),
            target: target.to_string(),
            ..Default::default()
        }
    }

    pub fn with_position(mut self, position: u64) -> Self {
        self.position = position;
        self
    }

    pub fn with_expressions(mut self, expressions: Vec<Expression>) -> Self {
        self.expressions = expressions;
        self
    }
}

/// Firewall chain (nftables)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FwChain {
    pub name: String,
    pub table: String,
    pub family: String,
    pub priority: String,
    #[serde(rename = "type")]
    pub chain_type: String,
    pub hook: String,
    pub policy: String,
    pub rules: Vec<FwRule>,
}

impl FwChain {
    pub fn new(name: &str, table: &str, hook: &str) -> Self {
        Self {
            name: name.to_string(),
            table: table.to_string(),
            family: "inet".to_string(),
            priority: "0".to_string(),
            chain_type: "filter".to_string(),
            hook: hook.to_string(),
            policy: "accept".to_string(),
            rules: Vec::new(),
        }
    }

    pub fn with_policy(mut self, policy: &str) -> Self {
        self.policy = policy.to_string();
        self
    }

    pub fn with_rules(mut self, rules: Vec<FwRule>) -> Self {
        self.rules = rules;
        self
    }

    /// Display name for the chain
    pub fn display_name(&self) -> String {
        format!("{} ({})", self.name, self.hook)
    }
}

/// Collection of firewall chains
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FwChains {
    // DEPRECATED: backward compatibility with iptables
    pub rule: Option<FwRule>,
    pub chains: Vec<FwChain>,
}

/// System firewall configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SysFirewall {
    pub enabled: bool,
    pub running: bool,
    pub version: u32,
    pub input_policy: String,
    pub output_policy: String,
    pub forward_policy: String,
    pub system_rules: Vec<FwChains>,
}

impl SysFirewall {
    pub fn new() -> Self {
        Self {
            enabled: false,
            running: false,
            version: 1,
            input_policy: "accept".to_string(),
            output_policy: "accept".to_string(),
            forward_policy: "accept".to_string(),
            system_rules: Vec::new(),
        }
    }

    pub fn all_chains(&self) -> impl Iterator<Item = &FwChain> {
        self.system_rules.iter().flat_map(|fc| fc.chains.iter())
    }

    pub fn all_chains_mut(&mut self) -> impl Iterator<Item = &mut FwChain> {
        self.system_rules.iter_mut().flat_map(|fc| fc.chains.iter_mut())
    }

    pub fn find_chain(&self, name: &str) -> Option<&FwChain> {
        self.all_chains().find(|c| c.name == name)
    }

    pub fn find_chain_mut(&mut self, name: &str) -> Option<&mut FwChain> {
        self.all_chains_mut().find(|c| c.name == name)
    }

    pub fn rule_count(&self) -> usize {
        self.all_chains().map(|c| c.rules.len()).sum()
    }

    pub fn chain_count(&self) -> usize {
        self.all_chains().count()
    }
}

/// Firewall policy presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallPolicy {
    Accept,
    Drop,
}

impl std::fmt::Display for FirewallPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accept => write!(f, "accept"),
            Self::Drop => write!(f, "drop"),
        }
    }
}

impl From<&str> for FirewallPolicy {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "drop" => Self::Drop,
            _ => Self::Accept,
        }
    }
}
