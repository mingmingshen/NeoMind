//! Embedded MQTT Broker
//!
//! This module provides an embedded MQTT broker using rumqttd.
//! The broker runs in the same process as NeoMind, eliminating the need
//! for an external MQTT broker installation.
//!
//! ## Configuration
//!
//! ```toml
//! [mqtt]
//! listen = "0.0.0.0"  # Listen address
//! port = 1883        # Broker listening port
//! ```
//!
//! External broker connections are managed via the data sources page.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Embedded MQTT broker error type
#[derive(Debug, Error)]
pub enum EmbeddedBrokerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Broker error: {0}")]
    Broker(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

/// Broker mode configuration (deprecated, kept for compatibility)
///
/// Note: NeoMind now always uses the embedded broker. External broker
/// connections are managed via the data sources page (ExternalBroker).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum BrokerMode {
    /// Use external MQTT broker (deprecated)
    External,
    /// Use embedded broker (default)
    #[default]
    Embedded,
}

/// Configuration for the embedded broker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedBrokerConfig {
    /// Listening address for embedded broker
    #[serde(default = "default_listen_addr")]
    pub listen: String,

    /// Listening port for embedded broker
    #[serde(default = "default_port")]
    pub port: u16,

    /// Maximum number of concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Maximum payload size in bytes
    #[serde(default = "default_max_payload")]
    pub max_payload_size: usize,

    /// Connection timeout in milliseconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_ms: u16,

    /// Enable dynamic topic filters
    #[serde(default = "default_dynamic_filters")]
    pub dynamic_filters: bool,

    /// Enable authentication
    #[serde(default)]
    pub auth_enabled: bool,

    /// Enable TLS
    #[serde(default)]
    pub tls_enabled: bool,

    /// Path to TLS certificate file
    #[serde(default)]
    pub tls_cert_path: Option<String>,

    /// Path to TLS private key file
    #[serde(default)]
    pub tls_key_path: Option<String>,

    /// Path to TLS CA certificate file
    #[serde(default)]
    pub tls_ca_path: Option<String>,
}

fn default_listen_addr() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    1883
}

fn default_max_connections() -> usize {
    1000
}

fn default_max_payload() -> usize {
    268435456 // 256 MB
}

fn default_connection_timeout() -> u16 {
    60000 // 60 seconds
}

fn default_dynamic_filters() -> bool {
    true
}

impl Default for EmbeddedBrokerConfig {
    fn default() -> Self {
        Self {
            listen: default_listen_addr(),
            port: default_port(),
            max_connections: default_max_connections(),
            max_payload_size: default_max_payload(),
            connection_timeout_ms: default_connection_timeout(),
            dynamic_filters: default_dynamic_filters(),
            auth_enabled: false,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
        }
    }
}

impl EmbeddedBrokerConfig {
    /// Create a new embedded broker config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the listening address
    pub fn with_listen(mut self, listen: impl Into<String>) -> Self {
        self.listen = listen.into();
        self
    }

    /// Set the listening port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set max connections
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Get the full socket address
    pub fn socket_addr(&self) -> Result<SocketAddr, EmbeddedBrokerError> {
        format!("{}:{}", self.listen, self.port)
            .parse()
            .map_err(|e| EmbeddedBrokerError::Config(format!("Invalid address: {}", e)))
    }
}

/// Embedded MQTT broker handle
///
/// This handle manages the lifecycle of the embedded broker.
pub struct EmbeddedBroker {
    config: Mutex<EmbeddedBrokerConfig>,
    running: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    auth_handler: Option<rumqttd::AuthHandler>,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl EmbeddedBroker {
    /// Create a new embedded broker with the given configuration
    pub fn new(config: EmbeddedBrokerConfig) -> Self {
        Self {
            config: Mutex::new(config),
            running: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
            auth_handler: None,
            thread_handle: Mutex::new(None),
        }
    }

    /// Create with default configuration
    pub fn with_default() -> Self {
        Self::new(EmbeddedBrokerConfig::default())
    }

    /// Check if the broker is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Set the authentication handler for external auth
    pub fn set_auth_handler<F, O>(&mut self, handler: F)
    where
        F: Fn(String, String, String) -> O + Send + Sync + 'static,
        O: std::future::Future<Output = bool> + Send + 'static,
    {
        self.auth_handler = Some(Arc::new(move |client_id, username, password| {
            Box::pin(handler(client_id, username, password))
                as std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>
        }));
    }

    /// Get the broker configuration
    pub fn config(&self) -> EmbeddedBrokerConfig {
        self.config.lock().unwrap().clone()
    }

    /// Stop the broker
    pub fn stop(&self) -> Result<(), EmbeddedBrokerError> {
        if !self.is_running() {
            return Ok(());
        }
        tracing::info!("Stopping embedded MQTT broker...");
        self.stop.store(true, Ordering::Relaxed);

        // Connect to own port to unblock accept loop
        let port = self.config.lock().unwrap().port;
        let _ = std::net::TcpStream::connect(format!("127.0.0.1:{}", port));

        // Wait for thread to finish (with timeout)
        if let Some(handle) = self.thread_handle.lock().unwrap().take() {
            let _ = handle.join();
        }

        self.running.store(false, Ordering::Relaxed);
        tracing::info!("Embedded MQTT broker stopped");
        Ok(())
    }

    /// Restart the broker with new configuration
    pub fn restart(&self, config: EmbeddedBrokerConfig) -> Result<(), EmbeddedBrokerError> {
        self.stop()?;
        *self.config.lock().unwrap() = config;
        self.start()
    }

    /// Start the embedded broker in a background thread
    ///
    /// This method spawns a new thread that runs the broker.
    /// The broker will listen on the configured address and port.
    pub fn start(&self) -> Result<(), EmbeddedBrokerError> {
        if self.is_running() {
            tracing::warn!("Embedded broker is already running");
            return Ok(());
        }

        // Check if port is already in use (possibly by a previous instance)
        let config = self.config.lock().unwrap().clone();
        if is_broker_running(config.port) {
            tracing::info!(
                "Embedded broker port {} already in use, assuming already running",
                config.port
            );
            self.running
                .store(true, Ordering::Relaxed);
            return Ok(());
        }

        let addr = config.socket_addr()?;
        let running = self.running.clone();
        let _stop = self.stop.clone();
        let max_connections = config.max_connections;
        let max_payload = config.max_payload_size;
        let connection_timeout = config.connection_timeout_ms;
        let dynamic_filters = config.dynamic_filters;
        let auth_enabled = config.auth_enabled;
        let tls_enabled = config.tls_enabled;
        let tls_cert_path = config.tls_cert_path.clone();
        let tls_key_path = config.tls_key_path.clone();
        let tls_ca_path = config.tls_ca_path.clone();
        let auth_handler = self.auth_handler.clone();

        running.store(true, Ordering::Relaxed);
        self.stop.store(false, Ordering::Relaxed);

        let handle = thread::Builder::new()
            .name("neomind-broker".to_string())
            .spawn(move || {
                tracing::info!("Starting embedded MQTT broker on {}", addr);

                // Build minimal broker config
                let mut broker_config = rumqttd::Config {
                    id: 0,
                    router: rumqttd::RouterConfig {
                        max_connections,
                        max_outgoing_packet_count: 200,
                        max_segment_size: 1048576, // 1 MB
                        max_segment_count: 10,
                        custom_segment: None,
                        initialized_filters: None,
                        // Note: shared_subscriptions_strategy is private in rumqttd 0.19
                        // Using default value through serde default
                        ..Default::default()
                    },
                    v4: None,
                    v5: None,
                    ws: None,
                    cluster: None,
                    console: None,
                    bridge: None,
                    prometheus: None,
                    metrics: None,
                };

                // Configure TLS if enabled
                let tls = if tls_enabled {
                    let cert_path = tls_cert_path.expect("TLS certificate path required when TLS is enabled");
                    let key_path = tls_key_path.expect("TLS key path required when TLS is enabled");
                    tracing::info!("TLS enabled with cert: {}, key: {}", cert_path, key_path);
                    Some(rumqttd::TlsConfig::Rustls {
                        capath: tls_ca_path,
                        certpath: cert_path,
                        keypath: key_path,
                    })
                } else {
                    None
                };

                // Configure v4 (MQTT 3.1.1) server
                let mut v4_config = HashMap::new();
                v4_config.insert(
                    "main".to_string(),
                    rumqttd::ServerSettings {
                        name: "neomind-broker".to_string(),
                        listen: addr,
                        tls,
                        next_connection_delay_ms: 1,
                        connections: rumqttd::ConnectionSettings {
                            connection_timeout_ms: connection_timeout,
                            max_payload_size: max_payload,
                            max_inflight_count: 200,
                            auth: None,
                            external_auth: if auth_enabled {
                                auth_handler
                            } else {
                                None
                            },
                            dynamic_filters,
                        },
                    },
                );
                broker_config.v4 = Some(v4_config);

                // Create and start broker
                let mut broker = rumqttd::Broker::new(broker_config);

                // The broker.start() method blocks until broker stops
                match broker.start() {
                    Ok(_) => {
                        tracing::info!("Embedded MQTT broker stopped");
                    }
                    Err(e) => {
                        tracing::error!("Embedded MQTT broker error: {}", e);
                    }
                }

                running.store(false, Ordering::Relaxed);
            })?;

        // Store the thread handle
        *self.thread_handle.lock().unwrap() = Some(handle);

        tracing::info!("Embedded MQTT broker thread started");

        // Wait for the broker to become ready with retries
        // rumqttd binds the port asynchronously, so we poll until it's listening
        let max_wait = std::time::Duration::from_secs(5);
        let check_interval = std::time::Duration::from_millis(100);
        let start = std::time::Instant::now();

        loop {
            if is_broker_running(config.port) {
                break;
            }
            if start.elapsed() >= max_wait {
                return Err(EmbeddedBrokerError::Broker(
                    "Broker failed to start or port not available".to_string(),
                ));
            }
            std::thread::sleep(check_interval);
        }

        tracing::info!(
            "Embedded broker started successfully on port {}",
            config.port
        );

        Ok(())
    }
}

/// Check if a broker is listening on the given port by attempting a TCP connect.
///
/// Uses connect (not bind) because rumqttd sets SO_REUSEADDR, which makes
/// bind-based checks unreliable — they succeed even when the port is in use.
pub fn is_broker_running(port: u16) -> bool {
    use std::net::{IpAddr, Ipv4Addr, TcpStream};
    let addr = (IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    TcpStream::connect_timeout(&addr.into(), std::time::Duration::from_millis(200)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EmbeddedBrokerConfig::default();
        assert_eq!(config.listen, "0.0.0.0");
        assert_eq!(config.port, 1883);
        assert_eq!(config.max_connections, 1000);
    }

    #[test]
    fn test_config_builder() {
        let config = EmbeddedBrokerConfig::new()
            .with_port(8883)
            .with_listen("127.0.0.1")
            .with_max_connections(500);

        assert_eq!(config.port, 8883);
        assert_eq!(config.listen, "127.0.0.1");
        assert_eq!(config.max_connections, 500);
    }

    #[test]
    fn test_socket_addr() {
        let config = EmbeddedBrokerConfig::new()
            .with_port(1883)
            .with_listen("0.0.0.0");

        let addr = config
            .socket_addr()
            .expect("Failed to get socket address from config");
        assert_eq!(addr.port(), 1883);
        assert_eq!(addr.ip(), std::net::Ipv4Addr::new(0, 0, 0, 0));
    }

    #[test]
    fn test_broker_mode_default() {
        assert_eq!(BrokerMode::default(), BrokerMode::Embedded);
    }

    #[test]
    fn test_broker_mode_deserialize() {
        let external: BrokerMode =
            serde_json::from_str("\"external\"").expect("Failed to deserialize external mode");
        let embedded: BrokerMode =
            serde_json::from_str("\"embedded\"").expect("Failed to deserialize embedded mode");

        assert_eq!(external, BrokerMode::External);
        assert_eq!(embedded, BrokerMode::Embedded);
    }
}
