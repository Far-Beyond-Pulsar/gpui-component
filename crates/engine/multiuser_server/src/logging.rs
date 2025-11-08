use anyhow::Result;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

use crate::config::Config;

/// Initialize logging and tracing subsystem
pub fn init(config: &Config) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // JSON formatting for structured logs
    let json_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(true)
        .with_file(true)
        .with_line_number(true);

    // Build the subscriber
    Registry::default()
        .with(env_filter)
        .with(json_layer)
        .try_init()?;

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Pulsar MultiEdit service starting"
    );

    Ok(())
}

/// Log configuration (without sensitive data)
pub fn log_config(config: &Config) {
    tracing::info!(
        http_bind = %config.http_bind,
        quic_bind = %config.quic_bind,
        udp_bind = %config.udp_bind,
        max_sessions = config.max_sessions,
        relay_bandwidth_limit = config.relay_bandwidth_limit,
        "Service configuration loaded"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging() {
        let config = Config::default();
        // We can't easily test logging initialization in a unit test
        // since it can only be done once per process
        assert_eq!(config.log_level, "info");
    }
}
