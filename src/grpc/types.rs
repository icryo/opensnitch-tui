//! Type conversions between protobuf and domain models

use std::collections::HashMap;

use crate::grpc::proto;
use crate::models;

// Connection conversions
impl From<proto::Connection> for models::Connection {
    fn from(c: proto::Connection) -> Self {
        Self {
            protocol: c.protocol,
            src_ip: c.src_ip,
            src_port: c.src_port,
            dst_ip: c.dst_ip,
            dst_host: c.dst_host,
            dst_port: c.dst_port,
            user_id: c.user_id,
            process_id: c.process_id,
            process_path: c.process_path,
            process_cwd: c.process_cwd,
            process_args: c.process_args,
            process_env: c.process_env,
            process_checksums: c.process_checksums,
            process_tree: c.process_tree.into_iter().map(|si| (si.key, si.value)).collect(),
            timestamp: None,
            action: None,
            rule_name: None,
        }
    }
}

impl From<models::Connection> for proto::Connection {
    fn from(c: models::Connection) -> Self {
        Self {
            protocol: c.protocol,
            src_ip: c.src_ip,
            src_port: c.src_port,
            dst_ip: c.dst_ip,
            dst_host: c.dst_host,
            dst_port: c.dst_port,
            user_id: c.user_id,
            process_id: c.process_id,
            process_path: c.process_path,
            process_cwd: c.process_cwd,
            process_args: c.process_args,
            process_env: c.process_env,
            process_checksums: c.process_checksums,
            process_tree: c.process_tree.into_iter().map(|(k, v)| proto::StringInt { key: k, value: v }).collect(),
        }
    }
}

// Operator conversions
impl From<proto::Operator> for models::Operator {
    fn from(o: proto::Operator) -> Self {
        Self {
            op_type: models::OperatorType::from(o.r#type.as_str()),
            operand: o.operand,
            data: o.data,
            sensitive: o.sensitive,
            list: o.list.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<models::Operator> for proto::Operator {
    fn from(o: models::Operator) -> Self {
        Self {
            r#type: o.op_type.to_string(),
            operand: o.operand,
            data: o.data,
            sensitive: o.sensitive,
            list: o.list.into_iter().map(Into::into).collect(),
        }
    }
}

// Rule conversions
impl From<proto::Rule> for models::Rule {
    fn from(r: proto::Rule) -> Self {
        Self {
            name: r.name,
            description: r.description,
            enabled: r.enabled,
            precedence: r.precedence,
            nolog: r.nolog,
            action: models::RuleAction::from(r.action.as_str()),
            duration: models::RuleDuration::from(r.duration.as_str()),
            operator: r.operator.map(Into::into).unwrap_or_default(),
            created: chrono::DateTime::from_timestamp(r.created, 0)
                .unwrap_or_else(chrono::Utc::now),
            updated: None,
        }
    }
}

impl From<models::Rule> for proto::Rule {
    fn from(r: models::Rule) -> Self {
        Self {
            created: r.created.timestamp(),
            name: r.name,
            description: r.description,
            enabled: r.enabled,
            precedence: r.precedence,
            nolog: r.nolog,
            action: r.action.to_string(),
            duration: r.duration.to_string(),
            operator: Some(r.operator.into()),
        }
    }
}

// Statistics conversions
impl From<proto::Statistics> for models::Statistics {
    fn from(s: proto::Statistics) -> Self {
        Self {
            daemon_version: s.daemon_version,
            rules: s.rules,
            uptime: s.uptime,
            dns_responses: s.dns_responses,
            connections: s.connections,
            ignored: s.ignored,
            accepted: s.accepted,
            dropped: s.dropped,
            rule_hits: s.rule_hits,
            rule_misses: s.rule_misses,
            by_proto: s.by_proto,
            by_address: s.by_address,
            by_host: s.by_host,
            by_port: s.by_port,
            by_uid: s.by_uid,
            by_executable: s.by_executable,
            events: s.events.into_iter().map(Into::into).collect(),
        }
    }
}

// Event conversions
impl From<proto::Event> for models::Event {
    fn from(e: proto::Event) -> Self {
        Self {
            time: e.time,
            connection: e.connection.map(Into::into).unwrap_or_default(),
            rule: e.rule.map(Into::into),
            unix_nano: e.unixnano,
        }
    }
}

// Alert conversions
impl From<proto::Alert> for models::Alert {
    fn from(a: proto::Alert) -> Self {
        let data = match a.data {
            Some(proto::alert::Data::Text(text)) => Some(models::AlertData::Text(text)),
            Some(proto::alert::Data::Proc(p)) => Some(models::AlertData::Process(p.into())),
            Some(proto::alert::Data::Conn(c)) => Some(models::AlertData::Connection(c.into())),
            Some(proto::alert::Data::Rule(r)) => Some(models::AlertData::Rule(r.into())),
            Some(proto::alert::Data::Fwrule(f)) => Some(models::AlertData::FirewallRule(f.into())),
            None => None,
        };

        Self {
            id: a.id,
            alert_type: models::AlertType::from(a.r#type),
            action: models::AlertAction::from(a.action),
            priority: models::AlertPriority::from(a.priority),
            what: models::AlertWhat::from(a.what),
            data,
            node: String::new(),
            timestamp: chrono::Utc::now(),
            acknowledged: false,
        }
    }
}

// Process conversions
impl From<proto::Process> for models::connection::Process {
    fn from(p: proto::Process) -> Self {
        Self {
            pid: p.pid,
            ppid: p.ppid,
            uid: p.uid,
            comm: p.comm,
            path: p.path,
            args: p.args,
            env: p.env,
            cwd: p.cwd,
            checksums: p.checksums,
            io_reads: p.io_reads,
            io_writes: p.io_writes,
            net_reads: p.net_reads,
            net_writes: p.net_writes,
            process_tree: p.process_tree.into_iter().map(|si| (si.key, si.value)).collect(),
        }
    }
}

// Firewall conversions
impl From<proto::FwRule> for models::FwRule {
    fn from(r: proto::FwRule) -> Self {
        Self {
            table: r.table,
            chain: r.chain,
            uuid: r.uuid,
            enabled: r.enabled,
            position: r.position,
            description: r.description,
            parameters: r.parameters,
            expressions: r.expressions.into_iter().map(Into::into).collect(),
            target: r.target,
            target_parameters: r.target_parameters,
        }
    }
}

impl From<models::FwRule> for proto::FwRule {
    fn from(r: models::FwRule) -> Self {
        Self {
            table: r.table,
            chain: r.chain,
            uuid: r.uuid,
            enabled: r.enabled,
            position: r.position,
            description: r.description,
            parameters: r.parameters,
            expressions: r.expressions.into_iter().map(Into::into).collect(),
            target: r.target,
            target_parameters: r.target_parameters,
        }
    }
}

impl From<proto::Expressions> for models::Expression {
    fn from(e: proto::Expressions) -> Self {
        Self {
            statement: e.statement.map(Into::into).unwrap_or_default(),
        }
    }
}

impl From<models::Expression> for proto::Expressions {
    fn from(e: models::Expression) -> Self {
        Self {
            statement: Some(e.statement.into()),
        }
    }
}

impl From<proto::Statement> for models::Statement {
    fn from(s: proto::Statement) -> Self {
        Self {
            op: s.op,
            name: s.name,
            values: s.values.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<models::Statement> for proto::Statement {
    fn from(s: models::Statement) -> Self {
        Self {
            op: s.op,
            name: s.name,
            values: s.values.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<proto::StatementValues> for models::StatementValue {
    fn from(sv: proto::StatementValues) -> Self {
        Self {
            key: sv.key,
            value: sv.value,
        }
    }
}

impl From<models::StatementValue> for proto::StatementValues {
    fn from(sv: models::StatementValue) -> Self {
        Self {
            key: sv.key,
            value: sv.value,
        }
    }
}

impl From<proto::FwChain> for models::FwChain {
    fn from(c: proto::FwChain) -> Self {
        Self {
            name: c.name,
            table: c.table,
            family: c.family,
            priority: c.priority,
            chain_type: c.r#type,
            hook: c.hook,
            policy: c.policy,
            rules: c.rules.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<models::FwChain> for proto::FwChain {
    fn from(c: models::FwChain) -> Self {
        Self {
            name: c.name,
            table: c.table,
            family: c.family,
            priority: c.priority,
            r#type: c.chain_type,
            hook: c.hook,
            policy: c.policy,
            rules: c.rules.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<proto::FwChains> for models::FwChains {
    fn from(fc: proto::FwChains) -> Self {
        Self {
            rule: fc.rule.map(Into::into),
            chains: fc.chains.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<models::FwChains> for proto::FwChains {
    fn from(fc: models::FwChains) -> Self {
        Self {
            rule: fc.rule.map(Into::into),
            chains: fc.chains.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<proto::SysFirewall> for models::SysFirewall {
    fn from(sf: proto::SysFirewall) -> Self {
        Self {
            enabled: sf.enabled,
            running: true, // Proto doesn't have this, assume running if received
            version: sf.version,
            input_policy: "accept".to_string(), // These would come from chain policies
            output_policy: "accept".to_string(),
            forward_policy: "accept".to_string(),
            system_rules: sf.system_rules.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<models::SysFirewall> for proto::SysFirewall {
    fn from(sf: models::SysFirewall) -> Self {
        Self {
            enabled: sf.enabled,
            version: sf.version,
            system_rules: sf.system_rules.into_iter().map(Into::into).collect(),
        }
    }
}

// ClientConfig conversions
impl From<proto::ClientConfig> for models::node::ClientConfig {
    fn from(c: proto::ClientConfig) -> Self {
        Self {
            id: c.id,
            name: c.name,
            version: c.version,
            is_firewall_running: c.is_firewall_running,
            config: c.config,
            log_level: c.log_level,
            rules: c.rules.into_iter().map(Into::into).collect(),
            system_firewall: c.system_firewall.map(Into::into),
        }
    }
}

impl From<models::node::ClientConfig> for proto::ClientConfig {
    fn from(c: models::node::ClientConfig) -> Self {
        Self {
            id: c.id,
            name: c.name,
            version: c.version,
            is_firewall_running: c.is_firewall_running,
            config: c.config,
            log_level: c.log_level,
            rules: c.rules.into_iter().map(Into::into).collect(),
            system_firewall: c.system_firewall.map(Into::into),
        }
    }
}
