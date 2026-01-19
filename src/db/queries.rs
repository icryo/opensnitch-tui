//! Database query definitions

pub const INSERT_CONNECTION: &str = r#"
    INSERT OR REPLACE INTO connections (
        time, node, action, protocol, src_ip, src_port, dst_ip, dst_host,
        dst_port, uid, pid, process, process_args, process_cwd, rule
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
"#;

pub const INSERT_RULE: &str = r#"
    INSERT OR REPLACE INTO rules (
        time, node, name, enabled, precedence, action, duration,
        operator_type, operator_sensitive, operator_operand, operator_data,
        description, nolog, created
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
"#;

pub const UPDATE_RULE: &str = r#"
    UPDATE rules SET
        time = ?1,
        enabled = ?4,
        precedence = ?5,
        action = ?6,
        duration = ?7,
        operator_type = ?8,
        operator_sensitive = ?9,
        operator_operand = ?10,
        operator_data = ?11,
        description = ?12,
        nolog = ?13
    WHERE node = ?2 AND name = ?3
"#;

pub const DELETE_RULE: &str = r#"
    DELETE FROM rules WHERE node = ?1 AND name = ?2
"#;

pub const INSERT_ALERT: &str = r#"
    INSERT INTO alerts (time, node, type, action, priority, what, body, status)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
"#;

pub const SELECT_CONNECTIONS: &str = r#"
    SELECT time, node, action, protocol, src_ip, src_port, dst_ip, dst_host,
           dst_port, uid, pid, process, process_args, process_cwd, rule
    FROM connections
    ORDER BY time DESC
    LIMIT ?1
"#;

pub const SELECT_RULES: &str = r#"
    SELECT time, node, name, enabled, precedence, action, duration,
           operator_type, operator_sensitive, operator_operand, operator_data,
           description, nolog, created
    FROM rules
    WHERE node = ?1
    ORDER BY name
"#;

pub const SELECT_ALERTS: &str = r#"
    SELECT id, time, node, type, action, priority, what, body, status
    FROM alerts
    ORDER BY time DESC
    LIMIT ?1
"#;

pub const UPDATE_STATS_HOST: &str = r#"
    INSERT INTO hosts (what, hits) VALUES (?1, 1)
    ON CONFLICT(what) DO UPDATE SET hits = hits + 1
"#;

pub const UPDATE_STATS_PROC: &str = r#"
    INSERT INTO procs (what, hits) VALUES (?1, 1)
    ON CONFLICT(what) DO UPDATE SET hits = hits + 1
"#;

pub const UPDATE_STATS_ADDR: &str = r#"
    INSERT INTO addrs (what, hits) VALUES (?1, 1)
    ON CONFLICT(what) DO UPDATE SET hits = hits + 1
"#;

pub const UPDATE_STATS_PORT: &str = r#"
    INSERT INTO ports (what, hits) VALUES (?1, 1)
    ON CONFLICT(what) DO UPDATE SET hits = hits + 1
"#;

pub const UPDATE_STATS_USER: &str = r#"
    INSERT INTO users (what, hits) VALUES (?1, 1)
    ON CONFLICT(what) DO UPDATE SET hits = hits + 1
"#;

pub const PURGE_OLD_CONNECTIONS: &str = r#"
    DELETE FROM connections WHERE time < ?1
"#;

pub const PURGE_OLD_ALERTS: &str = r#"
    DELETE FROM alerts WHERE time < ?1
"#;
