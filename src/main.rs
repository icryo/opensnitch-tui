use anyhow::{bail, Result};
use clap::Parser;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

mod app;
mod config;
mod db;
mod grpc;
mod models;
mod ui;
mod utils;

use app::state::AppState;
use config::settings::Settings;
use grpc::server::GrpcServer;
use ui::app::TuiApp;

const DAEMON_CONFIG_PATH: &str = "/etc/opensnitchd/default-config.json";
const SERVER_ADDR: &str = "127.0.0.1:50051";

#[derive(Parser, Debug)]
#[command(name = "opensnitch-tui")]
#[command(about = "Terminal UI for OpenSnitch application firewall")]
#[command(version)]
struct Args {
    /// Database path (use :memory: for in-memory)
    #[arg(short, long)]
    database: Option<String>,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
}

fn check_root() -> Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        bail!("This program must be run as root. Use: sudo opensnitch-tui");
    }
    Ok(())
}

fn configure_daemon() -> Result<()> {
    // Read current config
    let config_content = std::fs::read_to_string(DAEMON_CONFIG_PATH)
        .unwrap_or_else(|_| default_daemon_config());

    // Parse and update the Server.Address
    let mut config: serde_json::Value = serde_json::from_str(&config_content)
        .unwrap_or_else(|_| serde_json::from_str(&default_daemon_config()).unwrap());

    if let Some(server) = config.get_mut("Server") {
        if let Some(obj) = server.as_object_mut() {
            obj.insert("Address".to_string(), serde_json::Value::String(SERVER_ADDR.to_string()));
        }
    }

    // Write back
    let updated = serde_json::to_string_pretty(&config)?;
    std::fs::write(DAEMON_CONFIG_PATH, updated)?;

    Ok(())
}

fn default_daemon_config() -> String {
    format!(r#"{{
    "Server": {{
        "Address": "{}",
        "LogFile": "/var/log/opensnitchd.log"
    }},
    "DefaultAction": "allow",
    "DefaultDuration": "once",
    "InterceptUnknown": false,
    "ProcMonitorMethod": "proc",
    "LogLevel": 1,
    "Firewall": "iptables",
    "Stats": {{
        "MaxEvents": 150,
        "MaxStats": 25
    }}
}}"#, SERVER_ADDR)
}

fn restart_daemon() -> Result<()> {
    // Try systemctl first
    let status = Command::new("systemctl")
        .args(["restart", "opensnitch"])
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => {
            // Try opensnitch.service explicitly
            let status2 = Command::new("systemctl")
                .args(["restart", "opensnitch.service"])
                .status();

            match status2 {
                Ok(s) if s.success() => Ok(()),
                _ => bail!("Failed to restart opensnitch daemon. Is it installed?"),
            }
        }
    }
}

fn stop_daemon() -> Result<()> {
    let _ = Command::new("systemctl")
        .args(["stop", "opensnitch"])
        .status();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Check root
    check_root()?;

    // Suppress all panic output in TUI mode
    std::panic::set_hook(Box::new(|_| {}));

    // Configure daemon to use our socket
    configure_daemon()?;

    // Load settings
    let settings = Settings::load(args.config.as_deref())?;

    // Initialize database
    let db = db::Database::open(args.database.as_deref().unwrap_or(&settings.database_path))?;

    // Create channels for communication
    let (state_tx, state_rx) = mpsc::channel(1000);
    let (ui_update_tx, _) = broadcast::channel(100);

    // Create shared application state
    let state = Arc::new(AppState::new(db, ui_update_tx.clone()));

    // Start gRPC server FIRST (so it's ready when daemon starts)
    let grpc_server = GrpcServer::new(SERVER_ADDR.to_string(), state.clone(), state_tx.clone());
    let grpc_handle = tokio::spawn(async move {
        let _ = grpc_server.run().await;
    });

    // Give server a moment to bind
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Restart daemon to connect to our socket
    if let Err(e) = restart_daemon() {
        eprintln!("Warning: {}", e);
    }

    // Start state manager
    let state_clone = state.clone();
    let state_manager_handle = tokio::spawn(async move {
        app::state::run_state_manager(state_clone, state_rx, ui_update_tx).await;
    });

    // Run TUI (blocks until user quits)
    let mut tui = TuiApp::new(state.clone(), state_tx)?;
    let result = tui.run().await;

    // Cleanup
    grpc_handle.abort();
    state_manager_handle.abort();

    // Stop daemon on exit (optional - comment out to keep daemon running)
    // stop_daemon()?;

    result
}
