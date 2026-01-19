//! Database schema definitions

pub const SCHEMA_VERSION: i32 = 3;

pub const CREATE_TABLES: &str = r#"
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER PRIMARY KEY
    );

    CREATE TABLE IF NOT EXISTS connections (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        time TEXT NOT NULL,
        node TEXT NOT NULL,
        action TEXT,
        protocol TEXT,
        src_ip TEXT,
        src_port TEXT,
        dst_ip TEXT,
        dst_host TEXT,
        dst_port TEXT,
        uid TEXT,
        pid TEXT,
        process TEXT,
        process_args TEXT,
        process_cwd TEXT,
        rule TEXT,
        UNIQUE(node, action, protocol, src_ip, src_port, dst_ip, dst_port, uid, pid, process, process_args)
    );

    CREATE TABLE IF NOT EXISTS nodes (
        addr TEXT PRIMARY KEY,
        hostname TEXT,
        daemon_version TEXT,
        daemon_uptime TEXT,
        daemon_rules TEXT,
        cons TEXT,
        cons_dropped TEXT,
        version TEXT,
        status TEXT,
        last_connection TEXT
    );

    CREATE TABLE IF NOT EXISTS rules (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        time TEXT NOT NULL,
        node TEXT NOT NULL,
        name TEXT NOT NULL,
        enabled TEXT,
        precedence TEXT,
        action TEXT,
        duration TEXT,
        operator_type TEXT,
        operator_sensitive TEXT,
        operator_operand TEXT,
        operator_data TEXT,
        description TEXT,
        nolog TEXT,
        created TEXT,
        UNIQUE(node, name)
    );

    CREATE TABLE IF NOT EXISTS alerts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        time TEXT NOT NULL,
        node TEXT,
        type TEXT,
        action TEXT,
        priority TEXT,
        what TEXT,
        body TEXT,
        status INTEGER DEFAULT 0
    );

    -- Statistics tables
    CREATE TABLE IF NOT EXISTS hosts (
        what TEXT PRIMARY KEY,
        hits INTEGER DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS procs (
        what TEXT PRIMARY KEY,
        hits INTEGER DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS addrs (
        what TEXT PRIMARY KEY,
        hits INTEGER DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS ports (
        what TEXT PRIMARY KEY,
        hits INTEGER DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS users (
        what TEXT PRIMARY KEY,
        hits INTEGER DEFAULT 0
    );

    -- Indexes for faster queries
    CREATE INDEX IF NOT EXISTS idx_conn_time ON connections(time);
    CREATE INDEX IF NOT EXISTS idx_conn_action ON connections(action);
    CREATE INDEX IF NOT EXISTS idx_conn_process ON connections(process);
    CREATE INDEX IF NOT EXISTS idx_conn_rule ON connections(rule);
    CREATE INDEX IF NOT EXISTS idx_conn_node ON connections(node);
    CREATE INDEX IF NOT EXISTS idx_rules_time ON rules(time);
    CREATE INDEX IF NOT EXISTS idx_rules_node ON rules(node);
    CREATE INDEX IF NOT EXISTS idx_alerts_time ON alerts(time);
    CREATE INDEX IF NOT EXISTS idx_alerts_node ON alerts(node);
"#;
