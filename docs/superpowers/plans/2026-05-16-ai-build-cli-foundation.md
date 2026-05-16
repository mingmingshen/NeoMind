# AI Build CLI Foundation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the CLI command library that powers AI Build Mode — all product capabilities accessible via `neomind` CLI commands with `--json` structured output, callable both as CLI binary and as in-process library functions for multi-environment support (CLI / Tauri / Web).

**Architecture:** Extract CLI command logic into a new library crate `neomind-cli-ops`. The existing `neomind-cli` binary becomes a thin wrapper. The Agent's ShellTool gains an "internal execution" mode that calls the library directly, avoiding process spawning in Tauri/Web environments.

**Tech Stack:** Rust, clap 4 (derive), reqwest 0.12, serde_json, tokio

**Design Spec:** `docs/superpowers/specs/2026-05-16-ai-build-mode-design.md`

---

## File Structure

### New crate: `crates/neomind-cli-ops/`

```
crates/neomind-cli-ops/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API re-exports
    ├── api_client.rs       # HTTP client for API server calls
    ├── output.rs           # Output formatting (Human/Json)
    ├── types.rs            # Shared types (ApiResponse, BuildMeta, OutputFormat)
    ├── device.rs           # neomind device * commands
    ├── dashboard.rs        # neomind dashboard * commands
    ├── rule.rs             # neomind rule * commands
    ├── transform.rs        # neomind transform * commands
    ├── extension.rs        # neomind extension * commands (enhanced)
    ├── agent.rs            # neomind agent * commands
    ├── message.rs          # neomind message * commands
    └── widget.rs           # neomind widget * commands
```

### Modified files

```
crates/neomind-cli/
├── Cargo.toml              # Add neomind-cli-ops dependency
└── src/main.rs             # Refactor to use neomind-cli-ops, add new subcommands

crates/neomind-agent/
└── src/
    ├── toolkit/shell.rs    # Add internal execution mode
    └── prompts/builder.rs  # Inject CLI reference into system prompt

Cargo.toml                  # Add neomind-cli-ops to workspace
```

---

## Task 1: Create `neomind-cli-ops` Crate Skeleton

**Files:**
- Create: `crates/neomind-cli-ops/Cargo.toml`
- Create: `crates/neomind-cli-ops/src/lib.rs`
- Create: `crates/neomind-cli-ops/src/types.rs`
- Create: `crates/neomind-cli-ops/src/api_client.rs`
- Create: `crates/neomind-cli-ops/src/output.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Add workspace member**

In `Cargo.toml`, add `crates/neomind-cli-ops` to `workspace.members`.

- [ ] **Step 2: Create `crates/neomind-cli-ops/Cargo.toml`**

```toml
[package]
name = "neomind-cli-ops"
version = "0.1.0"
edition = "2021"

[dependencies]
neomind-core = { path = "../neomind-core" }
reqwest = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }
```

- [ ] **Step 3: Create `types.rs`**

```rust
use serde::{Deserialize, Serialize};

/// Structured output for all CLI commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_meta: Option<BuildMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMeta {
    pub r#type: String,       // "device" | "dashboard" | "rule" | ...
    pub action: String,       // "create" | "update" | "delete"
    pub entity_id: String,
    pub entity_name: Option<String>,
    pub undo_command: String,
}

impl CliResponse {
    pub fn success(data: serde_json::Value, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.into()),
            error: None,
            code: None,
            build_meta: None,
        }
    }

    pub fn success_with_meta(
        data: serde_json::Value,
        message: impl Into<String>,
        meta: BuildMeta,
    ) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.into()),
            error: None,
            code: None,
            build_meta: Some(meta),
        }
    }

    pub fn error(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(error.into()),
            code: Some(code.into()),
            build_meta: None,
        }
    }
}

/// Output format control
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
}
```

- [ ] **Step 4: Create `api_client.rs`**

```rust
use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "http://localhost:9375/api";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub struct ApiClient {
    base_url: String,
    client: Client,
}

impl ApiClient {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    pub fn with_base_url(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.to_string(),
            client,
        }
    }

    pub async fn get(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(body)
    }

    pub async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.post(&url).json(body).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = resp_body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn post_raw(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.post(&url).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = resp_body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn put(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.put(&url).json(body).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = resp_body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.delete(&url).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = resp_body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }
}
```

- [ ] **Step 5: Create `output.rs`**

```rust
use crate::types::{CliResponse, OutputFormat};

pub fn format_output(response: &CliResponse, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(response).unwrap_or_default(),
        OutputFormat::Human => format_human(response),
    }
}

fn format_human(resp: &CliResponse) -> String {
    if resp.success {
        let mut out = String::new();
        if let Some(msg) = &resp.message {
            out.push_str(&format!("✅ {}\n", msg));
        }
        if let Some(data) = &resp.data {
            // Pretty-print key fields from data
            if let Some(obj) = data.as_object() {
                for (key, value) in obj {
                    if let Some(s) = value.as_str() {
                        out.push_str(&format!("  {}: {}\n", key, s));
                    } else if value.is_number() || value.is_boolean() {
                        out.push_str(&format!("  {}: {}\n", key, value));
                    }
                }
            }
        }
        out
    } else {
        format!("❌ {}\n", resp.error.as_deref().unwrap_or("Unknown error"))
    }
}
```

- [ ] **Step 6: Create `lib.rs`**

```rust
pub mod api_client;
pub mod output;
pub mod types;

// Command modules will be added in subsequent tasks
// pub mod device;
// pub mod dashboard;
// pub mod rule;
// pub mod transform;
// pub mod extension;
// pub mod agent;
// pub mod message;
// pub mod widget;

pub use api_client::ApiClient;
pub use types::{BuildMeta, CliResponse, OutputFormat};
```

- [ ] **Step 7: Verify compilation**

Run: `cargo build -p neomind-cli-ops`
Expected: Compiles successfully with no errors.

- [ ] **Step 8: Commit**

```bash
git add crates/neomind-cli-ops/ Cargo.toml
git commit -m "feat: create neomind-cli-ops library crate with shared types and API client"
```

---

## Task 2: Device Commands

**Files:**
- Create: `crates/neomind-cli-ops/src/device.rs`
- Modify: `crates/neomind-cli-ops/src/lib.rs` (uncomment device module)
- Modify: `crates/neomind-cli/src/main.rs` (add device subcommand)
- Modify: `crates/neomind-cli/Cargo.toml` (add neomind-cli-ops dependency)

**API Endpoints used:**
- `GET /api/devices` — list
- `GET /api/devices/:id` — get
- `POST /api/devices` — create
- `PUT /api/devices/:id` — update
- `DELETE /api/devices/:id` — delete
- `GET /api/devices/:id/current` — latest metrics
- `GET /api/devices/:id/telemetry` — history
- `POST /api/devices/:id/command/:command` — control
- `GET /api/device-types` — list types
- `GET /api/device-types/:id` — get type
- `POST /api/device-types` — create type
- `DELETE /api/device-types/:id` — delete type

- [ ] **Step 1: Create `device.rs` with all command functions**

Each function takes `&ApiClient` and typed parameters, returns `Result<CliResponse>`.

```rust
use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

pub async fn list_devices(
    client: &ApiClient,
    device_type: Option<&str>,
    status: Option<&str>,
) -> Result<CliResponse> {
    let mut path = "/devices?limit=100".to_string();
    if let Some(dt) = device_type {
        path.push_str(&format!("&device_type={}", dt));
    }
    if let Some(s) = status {
        path.push_str(&format!("&status={}", s));
    }
    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Devices listed"))
}

pub async fn get_device(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/devices/{}", id)).await?;
    Ok(CliResponse::success(data, "Device details"))
}

pub async fn create_device(
    client: &ApiClient,
    name: &str,
    device_type: &str,
    adapter: &str,
    config: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({
        "name": name,
        "device_type": device_type,
        "adapter_type": adapter,
    });
    if let Some(cfg) = config {
        let cfg_value: serde_json::Value = serde_json::from_str(cfg)
            .map_err(|e| anyhow::anyhow!("Invalid config JSON: {}", e))?;
        body["connection_config"] = cfg_value;
    }
    let data = client.post("/devices", &body).await?;
    let entity_id = data["id"].as_str().unwrap_or("unknown").to_string();
    Ok(CliResponse::success_with_meta(
        data,
        "Device created successfully",
        BuildMeta {
            r#type: "device".into(),
            action: "create".into(),
            entity_id: entity_id.clone(),
            entity_name: Some(name.to_string()),
            undo_command: format!("neomind device delete {} --force", entity_id),
        },
    ))
}

pub async fn update_device(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    config: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(cfg) = config {
        let cfg_value: serde_json::Value = serde_json::from_str(cfg)
            .map_err(|e| anyhow::anyhow!("Invalid config JSON: {}", e))?;
        body["connection_config"] = cfg_value;
    }
    let data = client.put(&format!("/devices/{}", id), &body).await?;
    Ok(CliResponse::success(data, "Device updated"))
}

pub async fn delete_device(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.delete(&format!("/devices/{}", id)).await?;
    Ok(CliResponse::success(data, format!("Device {} deleted", id)))
}

pub async fn get_latest(
    client: &ApiClient,
    id: &str,
    metric: Option<&str>,
) -> Result<CliResponse> {
    let path = format!("/devices/{}/current", id);
    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Current values"))
}

pub async fn get_history(
    client: &ApiClient,
    id: &str,
    metric: &str,
    time_range: &str,
) -> Result<CliResponse> {
    let path = format!(
        "/devices/{}/telemetry?metric={}&time_range={}",
        id, metric, time_range
    );
    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Telemetry history"))
}

pub async fn control_device(
    client: &ApiClient,
    id: &str,
    command: &str,
    params: Option<&str>,
) -> Result<CliResponse> {
    let body = if let Some(p) = params {
        let p_value: serde_json::Value = serde_json::from_str(p)
            .map_err(|e| anyhow::anyhow!("Invalid params JSON: {}", e))?;
        json!({ "params": p_value })
    } else {
        json!({ "params": {} })
    };
    let path = format!("/devices/{}/command/{}", id, command);
    let data = client.post(&path, &body).await?;
    Ok(CliResponse::success(data, format!("Command '{}' sent", command)))
}

// Device Types
pub async fn list_device_types(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/device-types").await?;
    Ok(CliResponse::success(data, "Device types listed"))
}

pub async fn get_device_type(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/device-types/{}", id)).await?;
    Ok(CliResponse::success(data, "Device type details"))
}

pub async fn create_device_type(
    client: &ApiClient,
    name: &str,
    metrics: &str,
    commands: Option<&str>,
) -> Result<CliResponse> {
    let metrics_value: serde_json::Value = serde_json::from_str(metrics)
        .map_err(|e| anyhow::anyhow!("Invalid metrics JSON: {}", e))?;
    let mut body = json!({
        "name": name,
        "metrics": metrics_value,
    });
    if let Some(cmds) = commands {
        let cmds_value: serde_json::Value = serde_json::from_str(cmds)
            .map_err(|e| anyhow::anyhow!("Invalid commands JSON: {}", e))?;
        body["commands"] = cmds_value;
    }
    let data = client.post("/device-types", &body).await?;
    let type_id = data["id"].as_str().unwrap_or("unknown").to_string();
    Ok(CliResponse::success_with_meta(
        data,
        "Device type created",
        BuildMeta {
            r#type: "device_type".into(),
            action: "create".into(),
            entity_id: type_id.clone(),
            entity_name: Some(name.to_string()),
            undo_command: format!("neomind device types delete {} --force", type_id),
        },
    ))
}

pub async fn delete_device_type(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.delete(&format!("/device-types/{}", id)).await?;
    Ok(CliResponse::success(data, format!("Device type {} deleted", id)))
}
```

- [ ] **Step 2: Uncomment `device` module in `lib.rs`**

- [ ] **Step 3: Add `neomind-cli-ops` dependency to `crates/neomind-cli/Cargo.toml`**

```toml
neomind-cli-ops = { path = "../neomind-cli-ops" }
```

- [ ] **Step 4: Add device subcommand to CLI `main.rs`**

Add clap subcommand enum variants and match arms for `neomind device <action>`. Use the existing pattern from `ExtensionCommand` as reference. The handler functions create an `ApiClient` and call the corresponding `neomind_cli_ops::device::*` functions, then format output.

**Clap structure to add:**

```rust
#[derive(Subcommand, Debug)]
enum DeviceCommand {
    List {
        #[arg(long)]
        r#type: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Get {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Create {
        #[arg(long)]
        name: String,
        #[arg(long = "type")]
        device_type: String,
        #[arg(long)]
        adapter: String,
        #[arg(long)]
        config: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Update {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        config: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Delete {
        id: String,
        #[arg(long)]
        force: bool,
    },
    Latest {
        id: String,
        #[arg(long)]
        metric: Option<String>,
        #[arg(long)]
        json: bool,
    },
    History {
        id: String,
        #[arg(long)]
        metric: String,
        #[arg(long = "time-range")]
        time_range: String,
        #[arg(long)]
        json: bool,
    },
    Control {
        id: String,
        #[arg(long)]
        command: String,
        #[arg(long)]
        params: Option<String>,
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        json: bool,
    },
    Types {
        #[command(subcommand)]
        types_cmd: DeviceTypeCommand,
    },
}

#[derive(Subcommand, Debug)]
enum DeviceTypeCommand {
    List {
        #[arg(long)]
        json: bool,
    },
    Get {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        metrics: String,
        #[arg(long)]
        commands: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Delete {
        id: String,
        #[arg(long)]
        force: bool,
    },
}
```

Add `Device { #[command(subcommand)] device_cmd: DeviceCommand }` to the top-level `Command` enum.

- [ ] **Step 5: Verify compilation**

Run: `cargo build -p neomind-cli`
Expected: Compiles successfully.

- [ ] **Step 6: Test device commands manually**

```bash
# Start server in one terminal
cargo run -p neomind-cli -- serve

# Test in another terminal
cargo run -p neomind-cli -- device list --json
cargo run -p neomind-cli -- device types list --json
cargo run -p neomind-cli -- device --help
```

Expected: JSON output or human-readable output depending on `--json` flag.

- [ ] **Step 7: Commit**

```bash
git add crates/neomind-cli-ops/src/device.rs crates/neomind-cli-ops/src/lib.rs crates/neomind-cli/
git commit -m "feat: add device CLI commands (list/get/create/update/delete/latest/history/control/types)"
```

---

## Task 3: Dashboard Commands

**Files:**
- Create: `crates/neomind-cli-ops/src/dashboard.rs`
- Modify: `crates/neomind-cli-ops/src/lib.rs`
- Modify: `crates/neomind-cli/src/main.rs`

**API Endpoints:**
- `GET /api/dashboards` — list
- `GET /api/dashboards/:id` — get
- `POST /api/dashboards` — create
- `PUT /api/dashboards/:id` — update
- `DELETE /api/dashboards/:id` — delete

- [ ] **Step 1: Create `dashboard.rs` with command functions**

Functions: `list_dashboards`, `get_dashboard`, `create_dashboard`, `update_dashboard`, `delete_dashboard`, `add_widget`, `remove_widget`, `share_dashboard`.

Follow same pattern as `device.rs` — take `&ApiClient` + typed params, return `Result<CliResponse>`. Include `BuildMeta` with `undo_command` for create operations.

- [ ] **Step 2: Add `DashboardCommand` clap subcommand to `main.rs`**

```rust
#[derive(Subcommand, Debug)]
enum DashboardCommand {
    List { #[arg(long)] json: bool },
    Get { id: String, #[arg(long)] json: bool },
    Create { #[arg(long)] name: String, #[arg(long)] description: Option<String>, #[arg(long)] layout: Option<String>, #[arg(long)] json: bool },
    Update { id: String, #[arg(long)] name: Option<String>, #[arg(long)] description: Option<String>, #[arg(long)] layout: Option<String>, #[arg(long)] json: bool },
    Delete { id: String },
    AddWidget { id: String, #[arg(long = "widget-type")] widget_type: String, #[arg(long)] datasource: Option<String>, #[arg(long)] config: Option<String>, #[arg(long)] json: bool },
    RemoveWidget { #[arg(long = "dashboard-id")] dashboard_id: String, #[arg(long = "widget-id")] widget_id: String },
    Share { id: String, #[arg(long)] public: bool, #[arg(long)] expires: Option<String>, #[arg(long)] json: bool },
}
```

- [ ] **Step 3: Verify and test**

```bash
cargo build -p neomind-cli
cargo run -p neomind-cli -- dashboard list --json
cargo run -p neomind-cli -- dashboard create --name "Test" --json
```

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add dashboard CLI commands (list/get/create/update/delete/widget/share)"
```

---

## Task 4: Rule Commands

**Files:**
- Create: `crates/neomind-cli-ops/src/rule.rs`
- Modify: `crates/neomind-cli-ops/src/lib.rs`
- Modify: `crates/neomind-cli/src/main.rs`

**API Endpoints:**
- `GET /api/rules` — list
- `GET /api/rules/:id` — get
- `POST /api/rules` — create
- `PUT /api/rules/:id` — update
- `DELETE /api/rules/:id` — delete
- `POST /api/rules/:id/enable` — enable/disable (body: `{enabled: bool}`)
- `POST /api/rules/:id/test` — test
- `GET /api/rules/:id/history` — history

- [ ] **Step 1: Create `rule.rs` with command functions**

Functions: `list_rules`, `get_rule`, `create_rule`, `update_rule`, `delete_rule`, `enable_rule`, `disable_rule`, `test_rule`, `get_rule_history`.

- [ ] **Step 2: Add `RuleCommand` clap subcommand**

- [ ] **Step 3: Verify and test**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add rule CLI commands (list/get/create/update/delete/enable/disable/test/history)"
```

---

## Task 5: Transform Commands

**Files:**
- Create: `crates/neomind-cli-ops/src/transform.rs`
- Modify: `crates/neomind-cli-ops/src/lib.rs`
- Modify: `crates/neomind-cli/src/main.rs`

**API Endpoints:**
- `GET /api/automations/transforms` — list
- `POST /api/automations/transforms/process` — process (test)

Note: Transform CRUD may need additional API endpoints. Check `crates/neomind-api/src/server/router.rs` for existing transform routes and adapt accordingly. If transform create/update/delete don't exist as API endpoints, implement them as direct operations through the transform service or skip those commands.

- [ ] **Step 1: Create `transform.rs` with available command functions**

- [ ] **Step 2: Add `TransformCommand` clap subcommand**

- [ ] **Step 3: Verify and test**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add transform CLI commands"
```

---

## Task 6: Extension Commands Enhancement

**Files:**
- Create: `crates/neomind-cli-ops/src/extension.rs`
- Modify: `crates/neomind-cli-ops/src/lib.rs`
- Modify: `crates/neomind-cli/src/main.rs`

**API Endpoints:**
- `GET /api/extensions` — list
- `GET /api/extensions/:id` — get
- `GET /api/extensions/:id/health` — status
- `GET /api/extensions/:id/logs` — logs
- `POST /api/extensions/upload/file` — install (file upload)
- `DELETE /api/extensions/:id/uninstall` — uninstall

**New CLI-only operations (not API calls):**
- `extension create` — Scaffold new extension (already exists, migrate to ops library)
- `extension build` — Compile extension via `build.sh` or cargo
- `extension dev` — Development mode with auto-install

- [ ] **Step 1: Create `extension.rs` combining existing and new functions**

Migrate existing extension command logic from `main.rs` into `extension.rs`. Add new functions: `build_extension`, `dev_extension`, `get_extension_logs`.

- [ ] **Step 2: Update `ExtensionCommand` clap enum with new variants**

Add: `Build { #[arg(long)] path: Option<String>, #[arg(long)] release: bool }`, `Dev { #[arg(long)] path: Option<String> }`, `Logs { id: String, #[arg(long)] follow: bool }`, `Status { id: String, #[arg(long)] json: bool }`.

- [ ] **Step 3: Verify and test**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: enhance extension CLI commands (build/dev/logs/status) with library extraction"
```

---

## Task 7: Agent Commands

**Files:**
- Create: `crates/neomind-cli-ops/src/agent.rs`
- Modify: `crates/neomind-cli-ops/src/lib.rs`
- Modify: `crates/neomind-cli/src/main.rs`

**API Endpoints:**
- `GET /api/agents` — list
- `GET /api/agents/:id` — get
- `POST /api/agents` — create
- `PUT /api/agents/:id` — update
- `DELETE /api/agents/:id` — delete
- `POST /api/agents/:id/status` — control (body: `{"status": "paused"|"active"}`)
- `POST /api/agents/:id/execute` — invoke
- `GET /api/agents/:id/memory` — memory
- `GET /api/agents/:id/executions` — executions

- [ ] **Step 1: Create `agent.rs` with command functions**

- [ ] **Step 2: Add `AgentCommand` clap subcommand**

- [ ] **Step 3: Verify and test**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add agent CLI commands (list/get/create/update/delete/control/invoke/memory)"
```

---

## Task 8: Message Commands

**Files:**
- Create: `crates/neomind-cli-ops/src/message.rs`
- Modify: `crates/neomind-cli-ops/src/lib.rs`
- Modify: `crates/neomind-cli/src/main.rs`

**API Endpoints:**
- `GET /api/messages` — list
- `GET /api/messages/:id` — get
- `POST /api/messages` — create/send
- `POST /api/messages/:id/acknowledge` — acknowledge (mark as read)

- [ ] **Step 1: Create `message.rs` with command functions**

- [ ] **Step 2: Add `MessageCommand` clap subcommand**

- [ ] **Step 3: Verify and test**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add message CLI commands (list/get/send/read)"
```

---

## Task 9: Widget (Dashboard Component) Commands

**Files:**
- Create: `crates/neomind-cli-ops/src/widget.rs`
- Modify: `crates/neomind-cli-ops/src/lib.rs`
- Modify: `crates/neomind-cli/src/main.rs`

**API Endpoints:**
- `GET /api/frontend-components` — list
- `GET /api/frontend-components/:id/bundle` — get bundle
- `DELETE /api/frontend-components/:id` — uninstall
- `POST /api/frontend-components` — install (file upload)
- `GET /api/frontend-components/market/list` — marketplace list
- `POST /api/frontend-components/market/install` — marketplace install

**CLI-only operations:**
- `widget create` — Scaffold widget component (generate manifest.json + bundle.js template)
- `widget build` — Build widget (bundle React component into IIFE)
- `widget dev` — Development mode

- [ ] **Step 1: Create `widget.rs` with command functions**

- [ ] **Step 2: Add `WidgetCommand` clap subcommand**

- [ ] **Step 3: Verify and test**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add widget CLI commands (list/get/create/build/install/uninstall/publish)"
```

---

## Task 10: ShellTool Internal Execution Mode

**Files:**
- Modify: `crates/neomind-agent/src/toolkit/shell.rs`
- Modify: `crates/neomind-api/src/server/types.rs` (tool registration)

**Purpose:** Enable ShellTool to execute CLI commands directly as library function calls instead of spawning processes. This is essential for Tauri/Web environments where the `neomind` binary may not be in PATH.

- [ ] **Step 1: Add internal execution mode to ShellTool**

In `crates/neomind-agent/src/toolkit/shell.rs`, add:

```rust
pub enum ExecutionMode {
    Shell,       // Spawn shell process (CLI environment)
    Internal,    // Call neomind-cli-ops directly (Tauri/Web)
}

pub struct ShellTool {
    config: ShellConfig,
    mode: ExecutionMode,
}
```

When mode is `Internal`, parse the command string:
- If it starts with `neomind `, extract the subcommand and arguments
- Call the corresponding `neomind_cli_ops::*` function directly
- Return the `CliResponse` as tool output

Add `neomind-cli-ops` as a dependency to `crates/neomind-agent/Cargo.toml`.

- [ ] **Step 2: Implement command parsing for internal mode**

```rust
fn execute_internal(&self, command: &str) -> Result<ToolOutput> {
    // Parse "neomind device list --json" → domain="device", action="list", args={json:true}
    // Call neomind_cli_ops::device::list_devices(...)
    // Convert CliResponse to ToolOutput
}
```

Use a simple command parser (split by spaces, match domain/action, extract flags). No need for full clap parsing — just enough for the AI-generated command patterns.

- [ ] **Step 3: Update tool registration**

In `crates/neomind-api/src/server/types.rs`, configure the execution mode based on environment:

```rust
let mode = if cfg!(feature = "tauri") {
    ShellToolMode::Internal
} else {
    ShellToolMode::Shell
};

.with_shell_tool(Some(ShellConfig {
    enabled: true,
    timeout_secs: 30,
    max_output_chars: 50000,
    mode,
}))
```

- [ ] **Step 4: Verify both modes work**

Test in CLI mode (ShellTool spawns process) and verify internal mode compiles.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: add internal execution mode to ShellTool for Tauri/Web environments"
```

---

## Task 11: System Prompt CLI Reference Injection

**Files:**
- Modify: `crates/neomind-agent/src/prompts/builder.rs`

- [ ] **Step 1: Add CLI Build Tools section to system prompt**

In the system prompt builder (around line 487 where shell tool is documented), add a new section:

```rust
// After the existing shell tool documentation, add:
prompt.push_str("\n## CLI Build Commands\n\n");
prompt.push_str("You can execute `neomind` CLI commands to create and manage platform resources.\n");
prompt.push_str("Always use `--json` flag for structured output that you can parse reliably.\n\n");
prompt.push_str("### Command Reference\n\n");
prompt.push_str("| Domain | Commands |\n");
prompt.push_str("|--------|----------|\n");
prompt.push_str("| device | `neomind device list/get/create/update/delete/latest/history/control/types`\n");
prompt.push_str("| dashboard | `neomind dashboard list/get/create/update/delete/add-widget/remove-widget/share`\n");
prompt.push_str("| rule | `neomind rule list/get/create/update/delete/enable/disable/test/history`\n");
prompt.push_str("| transform | `neomind transform list/get/create/update/delete`\n");
prompt.push_str("| extension | `neomind extension list/get/status/logs/create/build/dev/install/uninstall`\n");
prompt.push_str("| widget | `neomind widget list/get/create/build/install/uninstall/publish`\n");
prompt.push_str("| agent | `neomind agent list/get/create/update/delete/control/invoke/memory/executions`\n");
prompt.push_str("| message | `neomind message list/get/send/read`\n\n");
prompt.push_str("Use `neomind <domain> <action> --help` to see parameters and examples.\n\n");
prompt.push_str("### Build Workflow\n");
prompt.push_str("1. Understand user intent\n");
prompt.push_str("2. Query existing resources if needed (`list`/`get`)\n");
prompt.push_str("3. Create/configure resources (`create`/`update`)\n");
prompt.push_str("4. Verify results (`get`/`list`)\n");
prompt.push_str("5. Present summary to user\n\n");
prompt.push_str("### Conventions\n");
prompt.push_str("- Always use `--json` for reliable result parsing\n");
prompt.push_str("- For complex JSON arguments, pass as JSON string\n");
prompt.push_str("- Chain multiple commands for multi-step builds\n");
prompt.push_str("- If a command fails, read the error, adjust, and retry\n");
```

- [ ] **Step 2: Verify prompt appears in agent session**

Start server, open chat, check system prompt includes CLI reference.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat: inject CLI command reference into agent system prompt for AI Build"
```

---

## Task 12: Integration Test

**Files:**
- Create: `crates/neomind-cli/tests/commands/device_test.rs`
- Create: `crates/neomind-cli/tests/commands/dashboard_test.rs`
- Create: `crates/neomind-cli/tests/commands/rule_test.rs`

- [ ] **Step 1: Write CLI integration tests**

Test each command with `--help` flag first (smoke test, no server needed):

```rust
#[test]
fn test_device_list_help() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("device").arg("list").arg("--help");
    cmd.assert().success();
}

#[test]
fn test_device_create_help() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("device").arg("create").arg("--help");
    cmd.assert().success();
}
```

- [ ] **Step 2: Write library unit tests**

In `crates/neomind-cli-ops/src/device.rs`, add `#[cfg(test)] mod tests` with tests for JSON parsing, response formatting, etc.

- [ ] **Step 3: Run all tests**

```bash
cargo test -p neomind-cli-ops
cargo test -p neomind-cli
```

- [ ] **Step 4: Commit**

```bash
git commit -m "test: add CLI command integration and unit tests"
```

---

## Task 13: End-to-End Verification

- [ ] **Step 1: Start server**

```bash
cargo run -p neomind-cli -- serve
```

- [ ] **Step 2: Test full AI Build workflow via CLI chat**

```bash
cargo run -p neomind-cli -- chat
> 帮我创建一个温度监控系统
```

Verify the AI uses CLI commands to create device type → device → dashboard → rule.

- [ ] **Step 3: Verify --json output is parseable**

```bash
cargo run -p neomind-cli -- device list --json | jq .
cargo run -p neomind-cli -- dashboard list --json | jq .
cargo run -p neomind-cli -- rule list --json | jq .
```

- [ ] **Step 4: Final commit**

```bash
git commit -m "chore: v0.8 AI Build CLI foundation complete"
```

---

## Summary

| Task | Description | New Files | Modified Files |
|------|------------|-----------|----------------|
| 1 | Crate skeleton + shared types | 5 | 1 |
| 2 | Device commands | 1 | 3 |
| 3 | Dashboard commands | 1 | 2 |
| 4 | Rule commands | 1 | 2 |
| 5 | Transform commands | 1 | 2 |
| 6 | Extension commands enhancement | 1 | 2 |
| 7 | Agent commands | 1 | 2 |
| 8 | Message commands | 1 | 2 |
| 9 | Widget commands | 1 | 2 |
| 10 | ShellTool internal mode | 0 | 2 |
| 11 | System prompt injection | 0 | 1 |
| 12 | Tests | 3 | 0 |
| 13 | E2E verification | 0 | 0 |

**Total: 16 new files, ~21 modified files, 13 tasks**
