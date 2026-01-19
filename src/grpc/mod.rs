pub mod notifications;
pub mod server;
pub mod service;
pub mod types;

pub use server::GrpcServer;
pub use service::UiService;

// Re-export generated protobuf types
pub mod proto {
    tonic::include_proto!("protocol");
}
