# Embedded MQTT Broker Auth & TLS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add authentication, TLS, and management UI to NeoMind's embedded MQTT broker.

**Architecture:** Use rumqttd's `external_auth` async callback to check credentials against redb on every connection. Passwords stored as bcrypt hashes. System credential (`__neomind_internal__`) auto-generated for internal adapter bypass. TLS uses rumqttd's built-in Rustls support. Only TLS/port changes need broker restart; credential changes are hot-reloaded.

**Tech Stack:** Rust (rumqttd 0.20, redb, bcrypt, rustls), React/TypeScript (lucide-react, Tailwind, shadcn/ui)

**Spec:** `docs/superpowers/specs/2026-05-27-mqtt-broker-auth-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `crates/neomind-devices/src/embedded_broker.rs` | Broker config, lifecycle (start/stop/restart), external_auth callback, TLS config |
| `crates/neomind-devices/Cargo.toml` | Add `bcrypt` dependency |
| `crates/neomind-storage/src/settings.rs` | Broker config + credentials redb tables |
| `crates/neomind-api/src/handlers/mqtt/broker_config.rs` | New API handlers for embedded broker config |
| `crates/neomind-api/src/handlers/mqtt/mod.rs` | Register new module |
| `crates/neomind-api/src/server/router.rs` | Register new routes (line ~762) |
| `crates/neomind-api/src/server/types.rs` | Load config from redb, pass DB to broker, system credential for internal adapter |
| `crates/neomind-api/src/config.rs` | Update `get_embedded_broker_config()` to check redb first |
| `web/src/components/connections/EmbeddedBrokerConfigDialog.tsx` | New dialog component |
| `web/src/components/connections/UnifiedDeviceConnectionsTab.tsx` | Add settings button to builtin card |
| `web/src/lib/api.ts` | New API methods |

---

### Task 1: Add bcrypt dependency and credential storage

**Files:**
- Modify: `crates/neomind-devices/Cargo.toml:25` (add bcrypt)
- Modify: `crates/neomind-storage/src/settings.rs` (add tables and CRUD methods)

- [ ] **Step 1: Add bcrypt to Cargo.toml**

In `crates/neomind-devices/Cargo.toml`, add after the rumqttd line (line 25):

```toml
bcrypt = { workspace = true, optional = true }
```

The workspace already defines `bcrypt = "0.15"` with `hash()`, `verify()`, and `DEFAULT_COST`.

Update the `embedded-broker` feature to include it:

```toml
embedded-broker = ["rumqttd", "bcrypt"]
```

- [ ] **Step 2: Add table definitions and credential types to settings.rs**

In `crates/neomind-storage/src/settings.rs`, after line 31 (`CONFIG_HISTORY_TABLE`), add:

```rust
// MQTT credentials table: key = username, value = password_hash (bcrypt)
pub const MQTT_CREDENTIALS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("mqtt_credentials");

// System credential key in SETTINGS_TABLE
pub const KEY_MQTT_BROKER_CONFIG: &str = "embedded_broker_config";
pub const KEY_SYSTEM_MQTT_CREDENTIAL: &str = "system_mqtt_internal_credential";
```

Add the credential struct:

```rust
/// MQTT broker credential (username + bcrypt hash).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttCredential {
    pub username: String,
    pub password_hash: String,
}
```

- [ ] **Step 3: Add storage methods for credentials**

Also add `MQTT_CREDENTIALS_TABLE` to `ensure_tables()` in settings.rs (around line 674-686 where other tables are explicitly opened during initialization) for consistency with the existing pattern.

Add these methods to `SettingsStore` impl in settings.rs:

```rust
/// Save embedded broker config.
pub fn save_embedded_broker_config(&self, config: &serde_json::Value) -> Result<(), Error> {
    let write_txn = self.db.begin_write()?;
    {
        let mut table = write_txn.open_table(SETTINGS_TABLE)?;
        let value = serde_json::to_vec(config).map_err(|e| Error::Serialization(e.to_string()))?;
        table.insert(KEY_MQTT_BROKER_CONFIG, value.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Load embedded broker config.
pub fn load_embedded_broker_config(&self) -> Result<Option<serde_json::Value>, Error> {
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(SETTINGS_TABLE)?;
    if let Some(data) = table.get(KEY_MQTT_BROKER_CONFIG)? {
        let config: serde_json::Value = serde_json::from_slice(data.value())
            .map_err(|e| Error::Serialization(e.to_string()))?;
        Ok(Some(config))
    } else {
        Ok(None)
    }
}

/// Add an MQTT credential.
pub fn add_mqtt_credential(&self, username: &str, password_hash: &str) -> Result<(), Error> {
    let write_txn = self.db.begin_write()?;
    {
        let mut table = write_txn.open_table(MQTT_CREDENTIALS_TABLE)?;
        let cred = MqttCredential {
            username: username.to_string(),
            password_hash: password_hash.to_string(),
        };
        let value = serde_json::to_vec(&cred).map_err(|e| Error::Serialization(e.to_string()))?;
        table.insert(username, value.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Delete an MQTT credential.
pub fn delete_mqtt_credential(&self, username: &str) -> Result<bool, Error> {
    let write_txn = self.db.begin_write()?;
    let existed = {
        let mut table = write_txn.open_table(MQTT_CREDENTIALS_TABLE)?;
        table.remove(username)?.is_some()
    };
    write_txn.commit()?;
    Ok(existed)
}

/// List all MQTT credentials.
pub fn list_mqtt_credentials(&self) -> Result<Vec<MqttCredential>, Error> {
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(MQTT_CREDENTIALS_TABLE)?;
    let mut creds = Vec::new();
    for entry in table.range(..)? {
        let (_, value) = entry?;
        let cred: MqttCredential = serde_json::from_slice(value.value())
            .map_err(|e| Error::Serialization(e.to_string()))?;
        creds.push(cred);
    }
    Ok(creds)
}

/// Get system internal MQTT credential (auto-generated).
pub fn get_system_mqtt_credential(&self) -> Result<Option<String>, Error> {
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(SETTINGS_TABLE)?;
    if let Some(data) = table.get(KEY_SYSTEM_MQTT_CREDENTIAL)? {
        Ok(Some(std::str::from_utf8(data.value())
            .map_err(|e| Error::Serialization(e.to_string()))?
            .to_string()))
    } else {
        Ok(None)
    }
}

/// Set system internal MQTT credential.
pub fn set_system_mqtt_credential(&self, password: &str) -> Result<(), Error> {
    let write_txn = self.db.begin_write()?;
    {
        let mut table = write_txn.open_table(SETTINGS_TABLE)?;
        table.insert(KEY_SYSTEM_MQTT_CREDENTIAL, password.as_bytes())?;
    }
    write_txn.commit()?;
    Ok(())
}
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check -p neomind-storage -p neomind-devices --features embedded-broker`
Expected: Compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-devices/Cargo.toml crates/neomind-storage/src/settings.rs
git commit -m "feat(storage): add MQTT credential storage tables and CRUD methods"
```

---

### Task 2: Redesign EmbeddedBroker with external_auth, stop/restart, TLS

**Files:**
- Modify: `crates/neomind-devices/src/embedded_broker.rs` (full rewrite of struct + start/stop)
- Modify: `crates/neomind-devices/src/lib.rs` (export new types if needed)

- [ ] **Step 1: Extend EmbeddedBrokerConfig with auth/TLS fields**

In `crates/neomind-devices/src/embedded_broker.rs`, add to `EmbeddedBrokerConfig` struct (after line 78):

```rust
    // Authentication
    #[serde(default)]
    pub auth_enabled: bool,

    // TLS
    #[serde(default)]
    pub tls_enabled: bool,
    #[serde(default)]
    pub tls_cert_path: Option<String>,
    #[serde(default)]
    pub tls_key_path: Option<String>,
    #[serde(default)]
    pub tls_ca_path: Option<String>,
```

Add serde defaults:

```rust
fn default_false() -> bool { false }
```

Update `Default` impl to include the new fields.

- [ ] **Step 2: Redesign EmbeddedBroker struct**

Replace the `EmbeddedBroker` struct (line 153-156) with:

```rust
pub struct EmbeddedBroker {
    config: Mutex<EmbeddedBrokerConfig>,  // Mutex for restart() via Arc
    running: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    auth_handler: Option<rumqttd::AuthHandler>,  // Set before start(), cloned into thread
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}
```

- [ ] **Step 3: Implement start() with external_auth callback**

Rewrite the `start()` method. The key change is setting `external_auth` on the `ConnectionSettings` and configuring TLS:

```rust
pub fn start(&self) -> Result<(), EmbeddedBrokerError> {
    if self.is_running() {
        tracing::warn!("Embedded broker is already running");
        return Ok(());
    }

    if is_broker_running(self.config.port) {
        tracing::info!("Port {} already in use, assuming broker running", self.config.port);
        self.running.store(true, Ordering::Relaxed);
        return Ok(());
    }

    let addr = self.config.socket_addr()?;
    let running = self.running.clone();
    let stop = self.stop.clone();
    let max_connections = self.config.max_connections;
    let max_payload = self.config.max_payload_size;
    let connection_timeout = self.config.connection_timeout_ms;
    let dynamic_filters = self.config.dynamic_filters;
    let auth_enabled = self.config.auth_enabled;
    let tls_enabled = self.config.tls_enabled;
    let tls_cert_path = self.config.tls_cert_path.clone();
    let tls_key_path = self.config.tls_key_path.clone();
    let tls_ca_path = self.config.tls_ca_path.clone();

    running.store(true, Ordering::Relaxed);
    stop.store(false, Ordering::Relaxed);

    let handle = thread::Builder::new()
        .name("neomind-broker".to_string())
        .spawn(move || {
            tracing::info!("Starting embedded MQTT broker on {}", addr);

            let mut broker_config = rumqttd::Config {
                id: 0,
                router: rumqttd::RouterConfig {
                    max_connections,
                    max_outgoing_packet_count: 200,
                    max_segment_size: 1048576,
                    max_segment_count: 10,
                    custom_segment: None,
                    initialized_filters: None,
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

            let mut server_settings = rumqttd::ServerSettings {
                name: "neomind-broker".to_string(),
                listen: addr,
                tls: None,
                next_connection_delay_ms: 1,
                connections: rumqttd::ConnectionSettings {
                    connection_timeout_ms: connection_timeout,
                    max_payload_size: max_payload,
                    max_inflight_count: 200,
                    auth: None,
                    external_auth: None,
                    dynamic_filters,
                },
            };

            // Configure TLS if enabled
            if tls_enabled {
                if let (Some(cert), Some(key)) = (&tls_cert_path, &tls_key_path) {
                    // rumqttd 0.20 TlsConfig::Rustls fields: capath, certpath, keypath (all String)
                    let tls_config = rumqttd::TlsConfig::Rustls {
                        capath: tls_ca_path.clone(),
                        certpath: cert.clone(),
                        keypath: key.clone(),
                    };
                    server_settings.tls = Some(tls_config);
                    tracing::info!("TLS enabled for embedded broker");
                } else {
                    tracing::error!("TLS enabled but cert/key paths missing");
                    running.store(false, Ordering::Relaxed);
                    return;
                }
            }

            // Configure external_auth — use injected handler if present, otherwise deny all
            if auth_enabled {
                // The auth_handler is cloned from self.auth_handler (set via set_auth_handler)
                // and moved into this closure. See Step 5 for how it's injected.
                // The handler is an Arc<AuthHandler> that reads credentials from redb.
            }

            let mut v4_config = HashMap::new();
            v4_config.insert("main".to_string(), server_settings);
            broker_config.v4 = Some(v4_config);

            let mut broker = rumqttd::Broker::new(broker_config);
            match broker.start() {
                Ok(_) => tracing::info!("Embedded MQTT broker stopped"),
                Err(e) => tracing::error!("Embedded MQTT broker error: {}", e),
            }

            running.store(false, Ordering::Relaxed);
        })?;

    *self.thread_handle.lock().unwrap() = Some(handle);

    // Wait for broker to become ready
    let max_wait = std::time::Duration::from_secs(5);
    let check_interval = std::time::Duration::from_millis(100);
    let start = std::time::Instant::now();

    loop {
        if is_broker_running(self.config.port) {
            break;
        }
        if start.elapsed() >= max_wait {
            return Err(EmbeddedBrokerError::Broker(
                "Broker failed to start within 5 seconds".to_string(),
            ));
        }
        std::thread::sleep(check_interval);
    }

    tracing::info!("Embedded broker started on port {}", self.config.port);
    Ok(())
}
```

**IMPORTANT: Auth handler injection.** The `EmbeddedBroker` struct must have a field `auth_handler: Option<rumqttd::AuthHandler>`. In `start()`, clone it into the thread closure:

```rust
let auth_handler = self.auth_handler.clone();
// Inside the thread, after building server_settings:
if let Some(handler) = auth_handler {
    server_settings.connections.external_auth = Some(handler);
}
```

`set_auth_handler(&mut self)` stores the Arc, and `start(&self)` clones it into the thread. No borrow conflicts since `AuthHandler = Arc<...>`.

- [ ] **Step 4: Implement stop() and restart()**

Add to `EmbeddedBroker`:

```rust
/// Stop the embedded broker.
pub fn stop(&self) -> Result<(), EmbeddedBrokerError> {
    if !self.is_running() {
        return Ok(());
    }

    tracing::info!("Stopping embedded MQTT broker...");
    self.stop.store(true, Ordering::Relaxed);

    // Connect to own port to unblock accept loop
    let port = self.config.port;
    let _ = std::net::TcpStream::connect(format!("127.0.0.1:{}", port));

    // Wait for thread to finish
    if let Some(handle) = self.thread_handle.lock().unwrap().take() {
        let _ = handle.join();
    }

    tracing::info!("Embedded MQTT broker stopped");
    Ok(())
}

/// Restart the broker with a new config.
/// Uses interior mutability (config behind Mutex) since broker is behind Arc<EmbeddedBroker>.
pub fn restart(&self, config: EmbeddedBrokerConfig) -> Result<(), EmbeddedBrokerError> {
    self.stop()?;
    *self.config.lock().unwrap() = config;
    self.start()
}
```

- [ ] **Step 5: Add set_auth_handler method**

```rust
/// Set the external auth handler for the broker.
/// Must be called before start().
pub fn set_auth_handler<F, O>(&mut self, handler: F)
where
    F: Fn(String, String, String) -> O + Send + Sync + 'static,
    O: std::future::Future<Output = bool> + 'static,
    O::Output: Send,
{
    self.auth_handler = Some(Arc::new(move |client_id, username, password| {
        let fut = handler(client_id, username, password);
        Box::pin(fut) as Pin<Box<dyn std::future::Future<Output = bool> + Send>>
    }));
}
```

- [ ] **Step 6: Run cargo check**

Run: `cargo check -p neomind-devices --features embedded-broker`
Expected: Compiles. There may be warnings about unused fields - that's OK.

- [ ] **Step 7: Commit**

```bash
git add crates/neomind-devices/src/embedded_broker.rs
git commit -m "feat(broker): redesign EmbeddedBroker with external_auth, stop/restart, TLS support"
```

---

### Task 3: Wire up broker init with redb config and auth handler

**Files:**
- Modify: `crates/neomind-api/src/server/types.rs:1083-1125` (embedded broker init + internal adapter)
- Modify: `crates/neomind-api/src/config.rs:448-461` (load from redb first)

- [ ] **Step 1: Update config.rs to load from redb first**

In `crates/neomind-api/src/config.rs`, update `get_embedded_broker_config()` (line 448):

```rust
pub fn get_embedded_broker_config() -> EmbeddedBrokerConfig {
    // Priority 1: redb database (set via API)
    if let Ok(store) = open_settings_store() {
        if let Ok(Some(config_value)) = store.load_embedded_broker_config() {
            if let Ok(config) = serde_json::from_value(config_value) {
                return config;
            }
        }
    }

    // Priority 2: config.toml
    if let Some(config) = load_embedded_broker_config() {
        return config;
    }

    // Default configuration
    info!(category = "mqtt", "Using default embedded broker configuration: 0.0.0.0:1883");
    EmbeddedBrokerConfig::default()
}
```

- [ ] **Step 2: Update init_device_adapters to inject auth handler**

In `crates/neomind-api/src/server/types.rs`, replace the embedded broker init block (lines 1083-1099) with:

```rust
#[cfg(feature = "embedded-broker")]
{
    use crate::config::get_embedded_broker_config;

    let mut config = get_embedded_broker_config();
    let port = config.port;
    let auth_enabled = config.auth_enabled;

    let mut broker = EmbeddedBroker::new(config);

    // Inject external_auth handler with redb access
    if auth_enabled {
        let store = match crate::config::open_settings_store() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open settings store for MQTT auth: {}", e);
                return;
            }
        };

        broker.set_auth_handler(move |_client_id: String, username: String, password: String| {
            let store = match crate::config::open_settings_store() {
                Ok(s) => s,
                Err(_) => return std::future::ready(false),
            };

            // Check system credential first
            if username == "__neomind_internal__" {
                if let Ok(Some(system_pass)) = store.get_system_mqtt_credential() {
                    return std::future::ready(password == system_pass);
                }
                return std::future::ready(false);
            }

            // Check user credentials
            let creds = match store.list_mqtt_credentials() {
                Ok(c) => c,
                Err(_) => return std::future::ready(false),
            };

            for cred in creds {
                if cred.username == username {
                    return std::future::ready(
                        bcrypt::verify(&password, &cred.password_hash).unwrap_or(false)
                    );
                }
            }
            std::future::ready(false)
        });

        // Ensure system credential exists
        if store.get_system_mqtt_credential().ok().flatten().is_none() {
            let system_password = generate_system_password();
            if let Err(e) = store.set_system_mqtt_credential(&system_password) {
                tracing::error!("Failed to save system MQTT credential: {}", e);
            }
        }
    }

    match broker.start() {
        Ok(_) => {
            tracing::info!("Embedded MQTT broker started on :{}", port);
            self.devices.embedded_broker = Some(Arc::new(broker));
        }
        Err(e) => {
            tracing::error!("Failed to start embedded broker: {}", e);
            tracing::warn!("Device management may not work properly");
        }
    }
}
```

Also add the helper function:

```rust
fn generate_system_password() -> String {
    // Use uuid which is already a workspace dependency
    uuid::Uuid::new_v4().to_string().replace("-", "")
}
```

- [ ] **Step 3: Update internal MQTT adapter to use system credential**

In `crates/neomind-api/src/server/types.rs`, update the `MqttAdapterConfig` section (lines 1102-1125). After getting config, resolve auth:

```rust
// Resolve internal adapter credentials if auth is enabled
let (adapter_username, adapter_password) = {
    #[cfg(feature = "embedded-broker")]
    {
        let broker_config = get_embedded_broker_config();
        if broker_config.auth_enabled {
            if let Ok(store) = crate::config::open_settings_store() {
                if let Ok(Some(pass)) = store.get_system_mqtt_credential() {
                    (Some("__neomind_internal__".to_string()), Some(pass))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    }
    #[cfg(not(feature = "embedded-broker"))]
    {
        (None, None)
    }
};

let broker_config = get_embedded_broker_config();
let mqtt_config = MqttAdapterConfig {
    name: "internal-mqtt".to_string(),
    mqtt: neomind_devices::mqtt::MqttConfig {
        broker: "127.0.0.1".to_string(),
        port: broker_config.port,  // Use dynamic port from config
        client_id: Some("neomind-internal".to_string()),
        username: adapter_username,
        password: adapter_password,
        tls: false,
        ca_cert: None,
        client_cert: None,
        client_key: None,
        keep_alive: 60,
        clean_session: true,
        qos: 1,
        topic_prefix: "device".to_string(),
        command_topic: "downlink".to_string(),
    },
    subscribe_topics: vec!["#".to_string()],
    discovery_topic: Some("device/+/+/uplink".to_string()),
    discovery_prefix: "device".to_string(),
    auto_discovery: true,
    storage_dir: Some("data".to_string()),
};
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check -p neomind-api --features embedded-broker`
Expected: Compiles.

- [ ] **Step 6: Commit**

```bash
git add crates/neomind-api/src/server/types.rs crates/neomind-api/src/config.rs crates/neomind-api/Cargo.toml
git commit -m "feat(api): wire broker init with redb config and external_auth handler"
```

---

### Task 4: Add API endpoints for broker config management

**Files:**
- Create: `crates/neomind-api/src/handlers/mqtt/broker_config.rs`
- Modify: `crates/neomind-api/src/handlers/mqtt/mod.rs` (add module)
- Modify: `crates/neomind-api/src/server/router.rs:762` (add routes)

- [ ] **Step 1: Create broker_config.rs handler**

Create `crates/neomind-api/src/handlers/mqtt/broker_config.rs`:

```rust
//! Embedded broker configuration management handlers.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::handlers::common::{ok, HandlerResult};
use crate::handlers::ServerState;

// ── Types ──

#[derive(Debug, Serialize)]
pub struct BrokerConfigResponse {
    pub listen: String,
    pub port: u16,
    pub max_connections: usize,
    pub auth_enabled: bool,
    pub credentials: Vec<CredentialEntry>,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub tls_ca_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CredentialEntry {
    pub username: String,
    pub password: String, // always masked
}

#[derive(Debug, Deserialize)]
pub struct UpdateBrokerConfigRequest {
    pub listen: Option<String>,
    pub port: Option<u16>,
    pub auth_enabled: Option<bool>,
    pub tls_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AddCredentialRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteCredentialRequest {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadTlsRequest {
    pub cert_pem: String,
    pub key_pem: String,
    pub ca_pem: Option<String>,
}

// ── Handlers ──

/// GET /api/mqtt/broker-config
pub async fn get_broker_config_handler(
    State(_state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let store = crate::config::open_settings_store()
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    let config = crate::config::get_embedded_broker_config();
    let credentials = store.list_mqtt_credentials()
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.username != "__neomind_internal__")
        .map(|c| CredentialEntry {
            username: c.username,
            password: "••••••••".to_string(),
        })
        .collect();

    ok(BrokerConfigResponse {
        listen: config.listen,
        port: config.port,
        max_connections: config.max_connections,
        auth_enabled: config.auth_enabled,
        credentials,
        tls_enabled: config.tls_enabled,
        tls_cert_path: config.tls_cert_path,
        tls_key_path: config.tls_key_path,
        tls_ca_path: config.tls_ca_path,
    })
}

/// PUT /api/mqtt/broker-config
pub async fn update_broker_config_handler(
    State(_state): State<ServerState>,
    Json(req): Json<UpdateBrokerConfigRequest>,
) -> HandlerResult<serde_json::Value> {
    let store = crate::config::open_settings_store()
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    let mut config = crate::config::get_embedded_broker_config();

    if let Some(listen) = req.listen { config.listen = listen; }
    if let Some(port) = req.port {
        if port == 0 || port > 65535 {
            return Err(crate::handlers::common::bad_request("Port must be 1-65535"));
        }
        config.port = port;
    }
    if let Some(auth_enabled) = req.auth_enabled { config.auth_enabled = auth_enabled; }
    if let Some(tls_enabled) = req.tls_enabled {
        if tls_enabled && (config.tls_cert_path.is_none() || config.tls_key_path.is_none()) {
            return Err(crate::handlers::common::bad_request(
                "TLS certificate and key must be uploaded before enabling TLS"
            ));
        }
        config.tls_enabled = tls_enabled;
    }

    // Save to redb
    let config_value = serde_json::to_value(&config)
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;
    store.save_embedded_broker_config(&config_value)
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    // Ensure system credential when auth enabled
    if config.auth_enabled {
        if store.get_system_mqtt_credential().ok().flatten().is_none() {
            let password = generate_system_password();
            store.set_system_mqtt_credential(&password)
                .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;
        }
    }

    ok(serde_json::json!({
        "restart_required": true,
        "message": "Configuration saved. Broker restart required for port/TLS changes."
    }))
}

/// POST /api/mqtt/broker-config/credentials
pub async fn add_credential_handler(
    State(_state): State<ServerState>,
    Json(req): Json<AddCredentialRequest>,
) -> HandlerResult<serde_json::Value> {
    // Validate username
    let username = req.username.trim();
    if username.is_empty() || username.len() > 64 {
        return Err(crate::handlers::common::bad_request("Username must be 1-64 characters"));
    }
    if username.starts_with("__neomind") {
        return Err(crate::handlers::common::bad_request("Reserved username prefix"));
    }
    if req.password.len() < 4 {
        return Err(crate::handlers::common::bad_request("Password must be at least 4 characters"));
    }

    let store = crate::config::open_settings_store()
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    // Check credential count
    let creds = store.list_mqtt_credentials()
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;
    if creds.len() >= 100 {
        return Err(crate::handlers::common::bad_request("Maximum 100 credentials allowed"));
    }

    let hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    store.add_mqtt_credential(username, &hash)
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    ok(serde_json::json!({
        "message": "Credential added. No broker restart needed."
    }))
}

/// POST /api/mqtt/broker-config/credentials/delete
pub async fn delete_credential_handler(
    State(_state): State<ServerState>,
    Json(req): Json<DeleteCredentialRequest>,
) -> HandlerResult<serde_json::Value> {
    let store = crate::config::open_settings_store()
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    let existed = store.delete_mqtt_credential(&req.username)
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    if !existed {
        return Err(crate::handlers::common::not_found("Credential not found"));
    }

    ok(serde_json::json!({
        "message": "Credential deleted. No broker restart needed."
    }))
}

/// PUT /api/mqtt/broker-config/tls
pub async fn upload_tls_handler(
    State(_state): State<ServerState>,
    Json(req): Json<UploadTlsRequest>,
) -> HandlerResult<serde_json::Value> {
    // Validate PEM format (basic check)
    if !req.cert_pem.contains("BEGIN CERTIFICATE") {
        return Err(crate::handlers::common::bad_request("Invalid server certificate PEM"));
    }
    if !req.key_pem.contains("PRIVATE KEY") {
        return Err(crate::handlers::common::bad_request("Invalid private key PEM"));
    }

    // Write cert files to data/tls/
    let tls_dir = std::path::Path::new("data/tls");
    std::fs::create_dir_all(tls_dir)
        .map_err(|e| crate::handlers::common::internal_error(&format!("Failed to create TLS dir: {}", e)))?;

    let cert_path = tls_dir.join("server.cert.pem");
    let key_path = tls_dir.join("server.key.pem");

    std::fs::write(&cert_path, &req.cert_pem)
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;
    std::fs::write(&key_path, &req.key_pem)
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    let ca_path = if let Some(ca_pem) = &req.ca_pem {
        let path = tls_dir.join("ca.cert.pem");
        std::fs::write(&path, ca_pem)
            .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;
        Some(path.to_string_lossy().to_string())
    } else {
        None
    };

    // Update config
    let store = crate::config::open_settings_store()
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    let mut config = crate::config::get_embedded_broker_config();
    config.tls_cert_path = Some(cert_path.to_string_lossy().to_string());
    config.tls_key_path = Some(key_path.to_string_lossy().to_string());
    config.tls_ca_path = ca_path;

    let config_value = serde_json::to_value(&config)
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;
    store.save_embedded_broker_config(&config_value)
        .map_err(|e| crate::handlers::common::internal_error(&e.to_string()))?;

    ok(serde_json::json!({
        "message": "TLS certificates saved. Enable TLS and restart broker to apply."
    }))
}
```

**NOTE for implementer:** Error responses in this codebase use `ErrorResponse` from `crate::models::ErrorResponse`, NOT helper functions from `common`. The correct pattern is:

```rust
use crate::models::ErrorResponse;

// Bad request:
return Err(ErrorResponse::bad_request("message"));

// Not found:
return Err(ErrorResponse::not_found("message"));

// Internal error:
return Err(ErrorResponse::internal(e.to_string()));
```

Replace ALL `crate::handlers::common::internal_error(...)`, `crate::handlers::common::bad_request(...)`, and `crate::handlers::common::not_found(...)` in the code above with the `ErrorResponse` pattern. Check existing handlers like `brokers.rs` for reference.

Also, `bcrypt` is already in `neomind-api/Cargo.toml` as a workspace dependency — no additional import needed.

- [ ] **Step 2: Register module in mod.rs**

In `crates/neomind-api/src/handlers/mqtt/mod.rs`, add:

```rust
pub mod broker_config;
pub use broker_config::*;
```

- [ ] **Step 3: Register routes in router.rs**

In `crates/neomind-api/src/server/router.rs`, after line 754 (after the unsubscribe routes), add:

```rust
        // Embedded Broker Config API
        .route("/api/mqtt/broker-config", get(mqtt::get_broker_config_handler))
        .route("/api/mqtt/broker-config", put(mqtt::update_broker_config_handler))
        .route("/api/mqtt/broker-config/credentials", post(mqtt::add_credential_handler))
        .route("/api/mqtt/broker-config/credentials/delete", post(mqtt::delete_credential_handler))
        .route("/api/mqtt/broker-config/tls", put(mqtt::upload_tls_handler))
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check -p neomind-api --features embedded-broker`
Expected: Compiles. Fix any import/type errors.

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-api/src/handlers/mqtt/broker_config.rs crates/neomind-api/src/handlers/mqtt/mod.rs crates/neomind-api/src/server/router.rs
git commit -m "feat(api): add embedded broker config API endpoints"
```

---

### Task 5: Frontend — API client and EmbeddedBrokerConfigDialog

**Files:**
- Modify: `web/src/lib/api.ts` (add new methods)
- Create: `web/src/components/connections/EmbeddedBrokerConfigDialog.tsx`
- Modify: `web/src/components/connections/UnifiedDeviceConnectionsTab.tsx:809` (add settings button)

- [ ] **Step 1: Add API methods to api.ts**

In `web/src/lib/api.ts`, add near the existing broker methods (around line 964):

```typescript
  // Embedded Broker Config
  getEmbeddedBrokerConfig: () =>
    fetchAPI<{
      listen: string
      port: number
      max_connections: number
      auth_enabled: boolean
      credentials: { username: string; password: string }[]
      tls_enabled: boolean
      tls_cert_path: string | null
      tls_key_path: string | null
      tls_ca_path: string | null
    }>('/mqtt/broker-config'),

  updateEmbeddedBrokerConfig: (config: {
    listen?: string
    port?: number
    auth_enabled?: boolean
    tls_enabled?: boolean
  }) => fetchAPI<{ restart_required: boolean; message: string }>('/mqtt/broker-config', {
    method: 'PUT',
    body: JSON.stringify(config),
  }),

  addMqttCredential: (username: string, password: string) =>
    fetchAPI<{ message: string }>('/mqtt/broker-config/credentials', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),

  deleteMqttCredential: (username: string) =>
    fetchAPI<{ message: string }>('/mqtt/broker-config/credentials/delete', {
      method: 'POST',
      body: JSON.stringify({ username }),
    }),

  updateMqttTlsCert: (certPem: string, keyPem: string, caPem?: string) =>
    fetchAPI<{ message: string }>('/mqtt/broker-config/tls', {
      method: 'PUT',
      body: JSON.stringify({ cert_pem: certPem, key_pem: keyPem, ca_pem: caPem }),
    }),
```

- [ ] **Step 2: Create EmbeddedBrokerConfigDialog component**

Create `web/src/components/connections/EmbeddedBrokerConfigDialog.tsx`. This is a `UnifiedFormDialog` or standard `Dialog` with three sections: General, Authentication, TLS. Follow existing dialog patterns from `UnifiedDeviceConnectionsTab.tsx`.

Key implementation details:
- Uses `api.getEmbeddedBrokerConfig()` to load config
- Auth toggle and credential changes call API immediately (no restart needed)
- Port/TLS changes show "Save" button with restart warning
- Add User uses a nested small dialog with username + password fields
- TLS section has textarea inputs for PEM content (not file upload — simpler)
- Follow the design spec wireframe from the spec doc
- Use design tokens from CLAUDE.md (never hardcoded colors)
- All text via `t()` i18n

The implementer should reference existing dialog patterns in the codebase (e.g., how `UniversalPluginConfigDialog` works) for consistent styling.

- [ ] **Step 3: Add settings button to builtin card**

In `web/src/components/connections/UnifiedDeviceConnectionsTab.tsx`, add `Settings` to the lucide-react imports (line 19) and add a settings button to the builtin broker card. Around line 809 where external broker buttons are rendered, add for the builtin card:

```typescript
{isBuiltin && (
  <Button
    variant="ghost"
    size="sm"
    className="h-8 w-8 p-0"
    onClick={(e) => {
      e.stopPropagation()
      setBrokerConfigDialogOpen(true)
    }}
    title={t('mqtt:settings')}
  >
    <Settings className="h-4 w-4" />
  </Button>
)}
```

Add state and dialog:

```typescript
const [brokerConfigDialogOpen, setBrokerConfigDialogOpen] = useState(false)
```

Render the dialog at the bottom of the component:

```typescript
<EmbeddedBrokerConfigDialog
  open={brokerConfigDialogOpen}
  onOpenChange={setBrokerConfigDialogOpen}
/>
```

- [ ] **Step 4: Add i18n keys**

Add translation keys in the existing locale files. Look for where `settings` translations live and add a `mqtt` section with keys like:
- `mqtt:settings` — "Broker Settings" / "Broker 设置"
- `mqtt:authEnabled` — "Enable Authentication" / "启用认证"
- `mqtt:tlsEnabled` — "Enable TLS" / "启用 TLS"
- `mqtt:addUser` — "Add User" / "添加用户"
- `mqtt:restartWarning` — warning text about restart
- etc.

- [ ] **Step 5: Run type check and build**

Run: `cd web && npm run build`
Expected: Build succeeds with no type errors.

- [ ] **Step 6: Commit**

```bash
git add web/src/lib/api.ts web/src/components/connections/ web/src/locales/
git commit -m "feat(ui): add embedded broker config dialog with auth and TLS management"
```

---

### Task 6: Integration test and cleanup

**Files:**
- Test: Manual testing + any automated tests for the storage layer

- [ ] **Step 1: Test storage layer**

Run: `cargo test -p neomind-storage`
Expected: All existing tests pass.

- [ ] **Step 2: Test broker start/stop with auth**

Manual test:
1. Start NeoMind: `cargo run -p neomind-cli -- serve`
2. Verify broker starts on :1883 with no auth (default)
3. Call `GET /api/mqtt/broker-config` — should return default config with auth_enabled: false
4. Call `POST /api/mqtt/broker-config/credentials` with `{"username": "test", "password": "test1234"}`
5. Call `PUT /api/mqtt/broker-config` with `{"auth_enabled": true}`
6. Restart NeoMind
7. Verify MQTT connection without credentials fails
8. Verify MQTT connection with test/test1234 succeeds
9. Verify internal adapter still works (devices still discovered)

- [ ] **Step 3: Test TLS flow**

Manual test:
1. Generate self-signed cert: `openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes -subj '/CN=localhost'`
2. Upload via `PUT /api/mqtt/broker-config/tls`
3. Enable TLS via `PUT /api/mqtt/broker-config` with `{"tls_enabled": true}`
4. Restart NeoMind
5. Verify MQTT over TLS works on port 1883

- [ ] **Step 4: Test frontend UI**

Manual test:
1. Open Settings → Device Connections → MQTT
2. Verify internal broker card shows settings gear icon
3. Click gear → dialog opens with current config
4. Toggle auth → warning shown
5. Add user → appears in list immediately
6. Delete user → removed from list immediately
7. Verify all text is i18n'd

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: integration testing and cleanup for MQTT broker auth"
```
