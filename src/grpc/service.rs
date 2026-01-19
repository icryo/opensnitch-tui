//! gRPC UI service implementation

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::app::state::{AppMessage, AppState};
use crate::grpc::proto;
use crate::grpc::proto::ui_server::Ui;
use crate::models;

/// Pending connection prompt waiting for user response
pub struct PendingPrompt {
    pub connection: models::Connection,
    pub node_addr: String,
    pub response_tx: oneshot::Sender<models::Rule>,
}

/// UI service implementation
pub struct UiService {
    state: Arc<AppState>,
    state_tx: mpsc::Sender<AppMessage>,
    default_action: models::RuleAction,
    default_duration: models::RuleDuration,
    prompt_timeout: Duration,
}

impl UiService {
    pub fn new(
        state: Arc<AppState>,
        state_tx: mpsc::Sender<AppMessage>,
    ) -> Self {
        Self {
            state,
            state_tx,
            default_action: models::RuleAction::Allow, // User preference: permissive
            default_duration: models::RuleDuration::Once,
            prompt_timeout: Duration::from_secs(15),
        }
    }

    fn create_default_rule(&self, conn: &models::Connection) -> models::Rule {
        models::Rule::new(
            &format!("{}-{}", conn.process_name(), conn.dst_port),
            self.default_action,
            self.default_duration.clone(),
            models::Operator::simple("process.path", &conn.process_path),
        )
    }

    fn peer_addr(req: &Request<impl std::any::Any>) -> String {
        req.remote_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

#[tonic::async_trait]
impl Ui for UiService {
    /// Health check with statistics
    async fn ping(
        &self,
        request: Request<proto::PingRequest>,
    ) -> Result<Response<proto::PingReply>, Status> {
        let peer = Self::peer_addr(&request);
        let ping = request.into_inner();

        tracing::debug!("Ping from {} (id: {})", peer, ping.id);

        // Forward stats to state manager
        if let Some(stats) = ping.stats {
            let _ = self.state_tx.send(AppMessage::StatsUpdate {
                node_addr: peer,
                stats: stats.into(),
            }).await;
        }

        Ok(Response::new(proto::PingReply { id: ping.id }))
    }

    /// Connection notification - auto-allow and log for monitoring
    async fn ask_rule(
        &self,
        request: Request<proto::Connection>,
    ) -> Result<Response<proto::Rule>, Status> {
        let peer = Self::peer_addr(&request);
        let proto_conn = request.into_inner();
        let connection: models::Connection = proto_conn.into();

        tracing::info!(
            "Connection from {}: {} -> {}",
            peer,
            connection.process_name(),
            connection.destination()
        );

        // Log connection for monitoring (no popup)
        let _ = self.state_tx.send(AppMessage::NewConnection {
            node_addr: peer.clone(),
            connection: connection.clone(),
        }).await;

        // Auto-allow with default rule (monitoring mode)
        let rule = self.create_default_rule(&connection);
        tracing::debug!("Auto-allowing: {} ({})", connection.process_name(), rule.action);
        Ok(Response::new(rule.into()))
    }

    /// Initial daemon subscription
    async fn subscribe(
        &self,
        request: Request<proto::ClientConfig>,
    ) -> Result<Response<proto::ClientConfig>, Status> {
        let peer = Self::peer_addr(&request);
        let config = request.into_inner();

        tracing::info!(
            "Subscribe from {}: {} v{} ({} rules)",
            peer,
            config.name,
            config.version,
            config.rules.len()
        );

        let client_config: models::node::ClientConfig = config.clone().into();

        // Notify state manager of new node
        let _ = self.state_tx.send(AppMessage::NodeConnected {
            addr: peer,
            config: client_config,
        }).await;

        // Return config (potentially modified)
        Ok(Response::new(config))
    }

    /// Bidirectional notification streaming
    type NotificationsStream = Pin<Box<dyn Stream<Item = Result<proto::Notification, Status>> + Send>>;

    async fn notifications(
        &self,
        request: Request<Streaming<proto::NotificationReply>>,
    ) -> Result<Response<Self::NotificationsStream>, Status> {
        let peer = Self::peer_addr(&request);
        let mut inbound = request.into_inner();

        tracing::info!("Notifications stream opened from {}", peer);

        // Create outbound channel for this node
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<proto::Notification>(100);

        // Register notification channel with state
        let _ = self.state_tx.send(AppMessage::NotificationChannelOpened {
            node_addr: peer.clone(),
            tx: outbound_tx,
        }).await;

        // Spawn task to handle inbound replies
        let state_tx = self.state_tx.clone();
        let peer_clone = peer.clone();
        tokio::spawn(async move {
            while let Some(result) = inbound.next().await {
                match result {
                    Ok(reply) => {
                        tracing::debug!(
                            "Notification reply from {}: code={:?}",
                            peer_clone,
                            reply.code
                        );
                        let _ = state_tx.send(AppMessage::NotificationReply {
                            node_addr: peer_clone.clone(),
                            id: reply.id,
                            code: reply.code,
                            data: reply.data,
                        }).await;
                    }
                    Err(e) => {
                        tracing::warn!("Notification stream error from {}: {}", peer_clone, e);
                        break;
                    }
                }
            }
            tracing::info!("Notification stream closed from {}", peer_clone);
            let _ = state_tx.send(AppMessage::NodeDisconnected {
                addr: peer_clone,
            }).await;
        });

        // Return outbound stream
        let stream = async_stream::stream! {
            while let Some(notification) = outbound_rx.recv().await {
                yield Ok(notification);
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }

    /// Receive alerts from daemon
    async fn post_alert(
        &self,
        request: Request<proto::Alert>,
    ) -> Result<Response<proto::MsgResponse>, Status> {
        let peer = Self::peer_addr(&request);
        let alert = request.into_inner();

        tracing::info!(
            "Alert from {}: type={:?} priority={:?} what={:?}",
            peer,
            alert.r#type,
            alert.priority,
            alert.what
        );

        let mut model_alert: models::Alert = alert.into();
        model_alert.node = peer;

        let _ = self.state_tx.send(AppMessage::AlertReceived {
            alert: model_alert,
        }).await;

        Ok(Response::new(proto::MsgResponse { id: 0 }))
    }
}
