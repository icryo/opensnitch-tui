//! gRPC server setup and lifecycle

use std::sync::Arc;
use anyhow::Result;
use tokio::sync::mpsc;
use tonic::transport::Server;

use crate::app::state::{AppMessage, AppState};
use crate::grpc::proto::ui_server::UiServer;
use crate::grpc::service::UiService;

#[cfg(unix)]
mod uds {
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::sync::Arc;
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tonic::transport::server::Connected;

    /// Wrapper for UnixStream that implements Connected
    #[derive(Debug)]
    pub struct UnixStreamWrapper {
        inner: tokio::net::UnixStream,
    }

    impl UnixStreamWrapper {
        pub fn new(stream: tokio::net::UnixStream) -> Self {
            Self { inner: stream }
        }
    }

    impl AsyncRead for UnixStreamWrapper {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.inner).poll_read(cx, buf)
        }
    }

    impl AsyncWrite for UnixStreamWrapper {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Pin::new(&mut self.inner).poll_write(cx, buf)
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.inner).poll_flush(cx)
        }

        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }

    /// Connection info for Unix sockets
    #[derive(Debug, Clone)]
    pub struct UdsConnectInfo {
        pub peer_addr: Option<Arc<tokio::net::unix::SocketAddr>>,
    }

    impl Connected for UnixStreamWrapper {
        type ConnectInfo = UdsConnectInfo;

        fn connect_info(&self) -> Self::ConnectInfo {
            UdsConnectInfo {
                peer_addr: self.inner.peer_addr().ok().map(Arc::new),
            }
        }
    }
}

/// gRPC server for daemon connections
pub struct GrpcServer {
    address: String,
    state: Arc<AppState>,
    state_tx: mpsc::Sender<AppMessage>,
}

impl GrpcServer {
    pub fn new(
        address: String,
        state: Arc<AppState>,
        state_tx: mpsc::Sender<AppMessage>,
    ) -> Self {
        Self {
            address,
            state,
            state_tx,
        }
    }

    pub async fn run(self) -> Result<()> {
        let address = self.address;
        let service = UiService::new(self.state, self.state_tx);

        if address.starts_with("unix://") {
            Self::run_unix_server(address, service).await
        } else {
            Self::run_tcp_server(address, service).await
        }
    }

    async fn run_unix_server(address: String, service: UiService) -> Result<()> {
        let path = address.strip_prefix("unix://").unwrap_or(&address);

        // Remove existing socket file if present
        let _ = std::fs::remove_file(path);

        tracing::info!("Starting gRPC server on unix://{}", path);

        #[cfg(unix)]
        {
            use tokio::net::UnixListener;
            use std::os::unix::fs::PermissionsExt;
            use uds::UnixStreamWrapper;

            let listener = UnixListener::bind(path)?;

            // Set permissions to allow daemon to connect
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o666))?;

            // Create a custom incoming stream that wraps UnixStream
            let incoming = async_stream::stream! {
                loop {
                    match listener.accept().await {
                        Ok((stream, _addr)) => {
                            yield Ok::<_, std::io::Error>(UnixStreamWrapper::new(stream));
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept Unix connection: {}", e);
                            yield Err(e);
                        }
                    }
                }
            };

            Server::builder()
                .add_service(UiServer::new(service))
                .serve_with_incoming(incoming)
                .await?;
        }

        #[cfg(not(unix))]
        {
            anyhow::bail!("Unix sockets not supported on this platform");
        }

        Ok(())
    }

    async fn run_tcp_server(address: String, service: UiService) -> Result<()> {
        let addr = address.parse()?;

        tracing::info!("Starting gRPC server on {}", addr);

        Server::builder()
            .add_service(UiServer::new(service))
            .serve(addr)
            .await?;

        Ok(())
    }
}
