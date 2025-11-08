//! Pulsar MultiEdit - Production-grade multiplayer editing server
//!
//! This crate provides a complete rendezvous, relay, and session coordination
//! service for Pulsar's collaborative editing features.

pub mod auth;
pub mod config;
pub mod crdt;
pub mod health;
pub mod http_server;
pub mod logging;
pub mod metrics;
pub mod nat;
pub mod persistence;
pub mod relay;
pub mod rendezvous;
pub mod session;
pub mod shutdown;
pub mod telemetry;
pub mod transport;

// Re-export commonly used types
pub use auth::{AuthService, Role};
pub use config::Config;
pub use crdt::{ORSet, RGASeq};
pub use health::HealthChecker;
pub use metrics::METRICS;
pub use persistence::PersistenceLayer;
pub use session::SessionStore;
pub use transport::{QuicServer, TcpSimultaneousOpen, UdpHolePuncher};

#[cfg(test)]
pub(crate) fn init_test_crypto() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}
