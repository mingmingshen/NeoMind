# Community Dashboard Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a community dashboard component system with two installation paths: (1) browse & install from a GitHub-based marketplace, (2) manual import from local files. Components are pure frontend, no Rust extension needed.

**Architecture:** New GitHub repo `camthink-ai/NeoMind-Dashboard-Components` hosts component index + bundles. Backend proxies GitHub API (same pattern as extension marketplace). Components stored on filesystem at `data/frontend-components/{id}/`. Frontend adds marketplace UI + import dialog in the component library sidebar, with a `CommunityRegistry` that reuses `DynamicRegistry`'s IIFE loading mechanism. WebSocket events for real-time install/uninstall notification.

**Tech Stack:** Rust (axum, reqwest, serde), TypeScript (React, Zustand), GitHub raw URLs for distribution

---

## Overview

```
Installation Paths:

1. Marketplace (一键安装)
   用户点击 → 前端调 API → 后端从 GitHub 下载 bundle.js + manifest.json
   → 存入 data/frontend-components/{id}/ → WebSocket 通知前端 → 出现在侧边栏

2. Import (手动导入)
   用户上传 manifest.json + bundle.js → 直接存入文件系统 → 出现在侧边栏

Component Lifecycle:
   安装 → CommunityRegistry 注册 → 出现在组件库 "Community" 分类
   卸载 → CommunityRegistry 移除 → Dashboard 中使用该组件的显示 fallback
```

---

## File Structure

### New Repository: `camthink-ai/NeoMind-Dashboard-Components`

```
NeoMind-Dashboard-Components/
├── index.json                    # 组件索引（后端拉取此文件）
├── components/
│   └── clock/                    # 示例组件
│       ├── manifest.json
│       ├── bundle.js
│       └── screenshot.png
├── README.md
└── README.zh.md
```

### Backend (Rust)

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/neomind-storage/src/frontend_components.rs` | Create | Types + filesystem store |
| `crates/neomind-storage/src/lib.rs` | Modify | Add module + re-exports |
| `crates/neomind-api/src/handlers/frontend_components.rs` | Create | All handlers (market list, market install, manual install, list, bundle serve, uninstall) |
| `crates/neomind-api/src/handlers/mod.rs` | Modify | Add module |
| `crates/neomind-api/src/server/types.rs` | Modify | Add store to ServerState |
| `crates/neomind-api/src/server/router.rs` | Modify | Register routes |

### Frontend (TypeScript/React)

| File | Action | Responsibility |
|------|--------|---------------|
| `web/src/types/frontend-component.ts` | Create | DTO types |
| `web/src/components/dashboard/registry/CommunityRegistry.ts` | Create | Community component registry |
| `web/src/components/dashboard/registry/registry.ts` | Modify | Merge community components |
| `web/src/components/dashboard/registry/ComponentRenderer.tsx` | Modify | Add community component branch |
| `web/src/store/slices/frontendComponentSlice.ts` | Create | Zustand store slice |
| `web/src/pages/dashboard-components/ComponentMarketplace.tsx` | Create | Marketplace browser UI |
| `web/src/pages/dashboard-components/InstallComponentDialog.tsx` | Create | Manual import dialog |
| `web/src/pages/dashboard-components/ComponentLibrary.tsx` | Modify | Add marketplace + import buttons |
| `web/src/hooks/useCommunityComponentLifecycle.ts` | Create | WebSocket lifecycle events |
| `web/src/lib/events.ts` | Modify | Add event type |
| `web/src/i18n/locales/en/dashboard-components.json` | Modify | i18n |
| `web/src/i18n/locales/zh/dashboard-components.json` | Modify | i18n |

---

## Task 1: Storage Layer — Filesystem-based `FrontendComponentStore`

**Files:**
- Create: `crates/neomind-storage/src/frontend_components.rs`
- Modify: `crates/neomind-storage/src/lib.rs`

**Storage layout on disk:**
```
data/frontend-components/
├── clock/
│   ├── manifest.json
│   └── bundle.js
├── rss-feed/
│   ├── manifest.json
│   └── bundle.js
```

- [ ] **Step 1: Create `crates/neomind-storage/src/frontend_components.rs`**

```rust
//! Frontend component storage using filesystem.
//!
//! data/frontend-components/{id}/manifest.json
//! data/frontend-components/{id}/bundle.js

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::Error;

// ============================================================================
// Types
// ============================================================================

/// Component manifest (stored as manifest.json on disk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentManifest {
    pub id: String,
    pub name: serde_json::Value,
    pub description: serde_json::Value,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    pub size_constraints: SizeConstraints,
    #[serde(default)]
    pub has_data_source: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_data_sources: Option<u32>,
    #[serde(default)]
    pub has_display_config: bool,
    #[serde(default)]
    pub has_actions: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<String>>,
    pub global_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_name: Option<String>,
    pub installed_at: i64,
}

fn default_icon() -> String { "Box".to_string() }
fn default_category() -> String { "custom".to_string() }
fn default_version() -> String { "1.0.0".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeConstraints {
    pub min_w: u32, pub min_h: u32,
    pub default_w: u32, pub default_h: u32,
    pub max_w: u32, pub max_h: u32,
}

/// Market index entry (from GitHub repo index.json).
/// Lighter than ComponentManifest — used for marketplace listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketComponentEntry {
    pub id: String,
    pub name: serde_json::Value,
    pub description: serde_json::Value,
    pub icon: String,
    pub category: String,
    pub version: String,
    pub author: Option<String>,
    pub size_constraints: SizeConstraints,
    pub has_data_source: bool,
    pub max_data_sources: Option<u32>,
    pub has_display_config: bool,
    pub has_actions: bool,
    pub screenshot_url: Option<String>,
    /// URL to manifest.json on GitHub
    pub manifest_url: String,
    /// URL to bundle.js on GitHub
    pub bundle_url: String,
}

/// Market index.json structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketIndex {
    pub version: String,
    pub components: Vec<MarketComponentEntry>,
}

// ============================================================================
// Store (filesystem-based)
// ============================================================================

#[derive(Clone)]
pub struct FrontendComponentStore {
    base_dir: PathBuf,
}

impl FrontendComponentStore {
    pub fn open<P: AsRef<Path>>(base_dir: P) -> Result<Self, Error> {
        let base_dir = base_dir.as_ref().to_path_buf();
        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)?;
        }
        Ok(Self { base_dir })
    }

    fn component_dir(&self, id: &str) -> PathBuf { self.base_dir.join(id) }
    fn manifest_path(&self, id: &str) -> PathBuf { self.component_dir(id).join("manifest.json") }
    fn bundle_path(&self, id: &str) -> PathBuf { self.component_dir(id).join("bundle.js") }

    pub fn install(&self, manifest: &ComponentManifest, bundle_bytes: &[u8]) -> Result<(), Error> {
        let dir = self.component_dir(&manifest.id);
        fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(manifest)?;
        fs::write(self.manifest_path(&manifest.id), json)?;
        fs::write(self.bundle_path(&manifest.id), bundle_bytes)?;
        Ok(())
    }

    pub fn list_all(&self) -> Result<Vec<ComponentManifest>, Error> {
        if !self.base_dir.exists() { return Ok(Vec::new()); }
        let mut components = Vec::new();
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() { continue; }
            let id = entry.file_name().to_string_lossy().to_string();
            if let Some(m) = self.load_manifest(&id)? { components.push(m); }
        }
        Ok(components)
    }

    pub fn load_manifest(&self, id: &str) -> Result<Option<ComponentManifest>, Error> {
        let path = self.manifest_path(id);
        if !path.exists() { return Ok(None); }
        let content = fs::read_to_string(&path)?;
        let m: ComponentManifest = serde_json::from_str(&content)?;
        Ok(Some(m))
    }

    pub fn exists(&self, id: &str) -> bool { self.manifest_path(id).exists() }

    pub fn get_bundle_path(&self, id: &str) -> Option<PathBuf> {
        let p = self.bundle_path(id);
        if p.exists() { Some(p) } else { None }
    }

    pub fn delete(&self, id: &str) -> Result<(), Error> {
        let dir = self.component_dir(id);
        if dir.exists() { fs::remove_dir_all(&dir)?; }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> FrontendComponentStore {
        let temp = std::env::temp_dir().join(format!("fc_test_{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&temp);
        FrontendComponentStore::open(&temp).unwrap()
    }

    fn sample_manifest(id: &str) -> ComponentManifest {
        ComponentManifest {
            id: id.to_string(),
            name: serde_json::json!("Clock"),
            description: serde_json::json!("A clock widget"),
            icon: "Clock".to_string(),
            category: "display".to_string(),
            version: "1.0.0".to_string(),
            author: Some("test".to_string()),
            screenshot: None,
            size_constraints: SizeConstraints {
                min_w: 2, min_h: 1, default_w: 3, default_h: 2, max_w: 6, max_h: 4,
            },
            has_data_source: false, max_data_sources: None,
            has_display_config: true, has_actions: false,
            config_schema: None, default_config: None, variants: None,
            global_name: "ClockWidget".to_string(),
            export_name: Some("default".to_string()),
            installed_at: 1234567890,
        }
    }

    #[test]
    fn test_install_and_load() {
        let s = create_test_store();
        s.install(&sample_manifest("clock"), b"// bundle").unwrap();
        let m = s.load_manifest("clock").unwrap().unwrap();
        assert_eq!(m.id, "clock");
    }

    #[test]
    fn test_list_all() {
        let s = create_test_store();
        s.install(&sample_manifest("a"), b"b1").unwrap();
        s.install(&sample_manifest("b"), b"b2").unwrap();
        assert_eq!(s.list_all().unwrap().len(), 2);
    }

    #[test]
    fn test_delete() {
        let s = create_test_store();
        s.install(&sample_manifest("x"), b"b").unwrap();
        assert!(s.exists("x"));
        s.delete("x").unwrap();
        assert!(!s.exists("x"));
    }

    #[test]
    fn test_nonexistent() {
        let s = create_test_store();
        assert!(s.load_manifest("nope").unwrap().is_none());
    }
}
```

- [ ] **Step 2: Register in `crates/neomind-storage/src/lib.rs`**

Add module + re-exports:
```rust
pub mod frontend_components;
pub use frontend_components::{ComponentManifest, FrontendComponentStore, MarketComponentEntry, MarketIndex, SizeConstraints};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p neomind-storage -- frontend_components`
Expected: 4 tests PASS

- [ ] **Step 4: Commit**

```
feat(storage): add filesystem-based FrontendComponentStore
```

---

## Task 2: API Handlers — Market + Manual Install + CRUD

**Files:**
- Create: `crates/neomind-api/src/handlers/frontend_components.rs`
- Modify: `crates/neomind-api/src/handlers/mod.rs`
- Modify: `crates/neomind-api/src/server/types.rs`
- Modify: `crates/neomind-api/src/server/router.rs`

**API Endpoints:**

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/frontend-components/market/list` | public | Fetch component index from GitHub |
| POST | `/api/frontend-components/market/install` | protected | Download & install from GitHub |
| POST | `/api/frontend-components` | protected | Manual install (multipart upload) |
| GET | `/api/frontend-components` | protected | List installed components |
| GET | `/api/frontend-components/:id/bundle` | public | Serve bundle.js |
| DELETE | `/api/frontend-components/:id` | protected | Uninstall |

- [ ] **Step 1: Add `frontend_component_store` to `ServerState`**

In `types.rs`:
```rust
pub frontend_component_store: FrontendComponentStore,
// Initialize in ServerState::new():
frontend_component_store: FrontendComponentStore::open(data_dir.join("frontend-components"))
    .expect("Failed to init frontend component store"),
```

- [ ] **Step 2: Create `crates/neomind-api/src/handlers/frontend_components.rs`**

```rust
//! Frontend component API handlers — marketplace + manual install + CRUD.

use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use tokio::task;

use super::common::{ok, HandlerResult};
use crate::models::ErrorResponse;
use crate::ServerState;
use neomind_storage::frontend_components::{
    ComponentManifest, FrontendComponentStore, MarketComponentEntry, MarketIndex, SizeConstraints,
};

const MARKET_BASE_URL: &str =
    "https://raw.githubusercontent.com/camthink-ai/NeoMind-Dashboard-Components";
const MARKET_BRANCH: &str = "main";

// Reserved built-in component IDs
const RESERVED_IDS: &[&str] = &[
    "value-card", "led-indicator", "sparkline", "progress-bar",
    "line-chart", "area-chart", "bar-chart", "pie-chart",
    "toggle-switch", "image-display", "image-history", "web-display",
    "markdown-display", "map-display", "video-display", "custom-layer",
    "agent-monitor-widget", "ai-analyst",
];

fn validate_component_id(id: &str) -> Result<(), ErrorResponse> {
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(ErrorResponse::bad_request("Invalid component ID"));
    }
    if RESERVED_IDS.contains(&id) {
        return Err(ErrorResponse::bad_request(&format!(
            "Component ID '{}' is reserved", id
        )));
    }
    Ok(())
}

// ============================================================================
// Marketplace APIs
// ============================================================================

/// GET /api/frontend-components/market/list
pub async fn market_list_handler(
    State(_state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let cache_buster = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let url = format!(
        "{}/{}/index.json?t={}",
        MARKET_BASE_URL, MARKET_BRANCH, cache_buster
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ErrorResponse::internal(&format!("HTTP client: {}", e)))?;

    let response = match client.get(&url)
        .header("User-Agent", "NeoMind-Component-Marketplace")
        .send().await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Marketplace fetch failed: {}", e);
            return ok(serde_json::json!({
                "components": [],
                "error": "network_error",
                "message": "Unable to connect to component marketplace."
            }));
        }
    };

    if !response.status().is_success() {
        return ok(serde_json::json!({
            "components": [],
            "error": format!("http_{}", response.status().as_u16()),
        }));
    }

    let index: MarketIndex = match response.json().await {
        Ok(i) => i,
        Err(e) => {
            tracing::error!("Marketplace parse failed: {}", e);
            return ok(serde_json::json!({ "components": [], "error": "parse_error" }));
        }
    };

    ok(serde_json::json!({
        "components": index.components,
        "total": index.components.len(),
        "market_version": index.version,
    }))
}

/// POST /api/frontend-components/market/install
/// Body: { "component_id": "clock" }
#[derive(Deserialize)]
pub struct MarketInstallRequest {
    pub component_id: String,
}

pub async fn market_install_handler(
    State(state): State<ServerState>,
    Json(req): Json<MarketInstallRequest>,
) -> HandlerResult<ComponentManifest> {
    validate_component_id(&req.component_id)?;

    // 1. Fetch index to find the component entry
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ErrorResponse::internal(&format!("HTTP client: {}", e)))?;

    let cache_buster = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let index_url = format!("{}/{}/index.json?t={}", MARKET_BASE_URL, MARKET_BRANCH, cache_buster);

    let index: MarketIndex = client.get(&index_url)
        .header("User-Agent", "NeoMind-Component-Marketplace")
        .send().await.map_err(|e| ErrorResponse::internal(&format!("Fetch index: {}", e)))?
        .json().await.map_err(|e| ErrorResponse::internal(&format!("Parse index: {}", e)))?;

    let entry = index.components.iter().find(|c| c.id == req.component_id)
        .ok_or_else(|| ErrorResponse::not_found(&format!("Component '{}' in marketplace", req.component_id)))?;

    // 2. Download manifest.json and bundle.js in parallel
    let (manifest_resp, bundle_resp) = tokio::join!(
        client.get(&entry.manifest_url).send(),
        client.get(&entry.bundle_url).send(),
    );

    let manifest_text = manifest_resp.map_err(|e| ErrorResponse::internal(&format!("Download manifest: {}", e)))?
        .text().await.map_err(|e| ErrorResponse::internal(&format!("Read manifest: {}", e)))?;
    let bundle_bytes = bundle_resp.map_err(|e| ErrorResponse::internal(&format!("Download bundle: {}", e)))?
        .bytes().await.map_err(|e| ErrorResponse::internal(&format!("Read bundle: {}", e)))?;

    // 3. Parse manifest and add installed_at
    let mut manifest: ComponentManifest = serde_json::from_str(&manifest_text)
        .map_err(|e| ErrorResponse::internal(&format!("Parse manifest: {}", e)))?;
    manifest.installed_at = chrono::Utc::now().timestamp();

    // 4. Write to filesystem
    let store = state.frontend_component_store.clone();
    let m = manifest.clone();
    let b = bundle_bytes.to_vec();
    task::spawn_blocking(move || store.install(&m, &b))
        .await.map_err(|e| ErrorResponse::internal(&e.to_string()))?
        .map_err(|e| ErrorResponse::internal(&e.to_string()))?;

    // 5. Publish event
    if let Some(bus) = &state.core.event_bus {
        let _ = bus.publish(neomind_core::event::NeoMindEvent::Custom {
            event_type: "FrontendComponentLifecycle".to_string(),
            data: serde_json::json!({ "component_id": manifest.id, "state": "installed" }),
        }).await;
    }

    ok(manifest)
}

// ============================================================================
// Manual Install + CRUD
// ============================================================================

/// POST /api/frontend-components (multipart: manifest + bundle)
pub async fn install_component_handler(
    State(state): State<ServerState>,
    mut multipart: Multipart,
) -> HandlerResult<ComponentManifest> {
    let mut manifest_json: Option<serde_json::Value> = None;
    let mut bundle: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ErrorResponse::bad_request(&format!("Multipart: {}", e))
    })? {
        match field.name() {
            Some("manifest") => {
                let text = field.text().await.map_err(|e| ErrorResponse::bad_request(&format!("Read manifest: {}", e)))?;
                manifest_json = Some(serde_json::from_str(&text).map_err(|e| ErrorResponse::bad_request(&format!("JSON: {}", e)))?);
            }
            Some("bundle") => {
                bundle = Some(field.bytes().await.map_err(|e| ErrorResponse::bad_request(&format!("Read bundle: {}", e)))?.to_vec());
            }
            _ => {}
        }
    }

    let mj = manifest_json.ok_or_else(|| ErrorResponse::bad_request("Missing manifest"))?;
    let bundle_bytes = bundle.ok_or_else(|| ErrorResponse::bad_request("Missing bundle"))?;

    let mut manifest: ComponentManifest = serde_json::from_value(mj)
        .map_err(|e| ErrorResponse::bad_request(&format!("Invalid manifest: {}", e)))?;
    validate_component_id(&manifest.id)?;
    if manifest.global_name.is_empty() {
        return Err(ErrorResponse::bad_request("global_name is required"));
    }
    manifest.installed_at = chrono::Utc::now().timestamp();

    let store = state.frontend_component_store.clone();
    let m = manifest.clone();
    let b = bundle_bytes.clone();
    task::spawn_blocking(move || store.install(&m, &b))
        .await.map_err(|e| ErrorResponse::internal(&e.to_string()))?
        .map_err(|e| ErrorResponse::internal(&e.to_string()))?;

    if let Some(bus) = &state.core.event_bus {
        let _ = bus.publish(neomind_core::event::NeoMindEvent::Custom {
            event_type: "FrontendComponentLifecycle".to_string(),
            data: serde_json::json!({ "component_id": manifest.id, "state": "installed" }),
        }).await;
    }

    ok(manifest)
}

/// GET /api/frontend-components
pub async fn list_components_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let store = state.frontend_component_store.clone();
    let components = task::spawn_blocking(move || store.list_all())
        .await.map_err(|e| ErrorResponse::internal(&e.to_string()))?
        .map_err(|e| ErrorResponse::internal(&e.to_string()))?;
    ok(serde_json::json!({ "components": components }))
}

/// GET /api/frontend-components/:id/bundle
pub async fn get_bundle_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Response, ErrorResponse> {
    validate_component_id(&id)?;
    let store = state.frontend_component_store.clone();
    let id_c = id.clone();
    let path = task::spawn_blocking(move || store.get_bundle_path(&id_c))
        .await.map_err(|e| ErrorResponse::internal(&e.to_string()))?
        .map_err(|e| ErrorResponse::internal(&e.to_string()))?
        .ok_or_else(|| ErrorResponse::not_found(&format!("Component '{}'", id)))?;

    let bytes = tokio::fs::read(&path).await
        .map_err(|e| ErrorResponse::internal(&format!("Read bundle: {}", e)))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/javascript")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(bytes))
        .unwrap())
}

/// DELETE /api/frontend-components/:id
pub async fn uninstall_component_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    validate_component_id(&id)?;
    if !state.frontend_component_store.exists(&id) {
        return Err(ErrorResponse::not_found(&format!("Component '{}'", id)));
    }

    let store = state.frontend_component_store.clone();
    let id_c = id.clone();
    task::spawn_blocking(move || store.delete(&id_c))
        .await.map_err(|e| ErrorResponse::internal(&e.to_string()))?
        .map_err(|e| ErrorResponse::internal(&e.to_string()))?;

    if let Some(bus) = &state.core.event_bus {
        let _ = bus.publish(neomind_core::event::NeoMindEvent::Custom {
            event_type: "FrontendComponentLifecycle".to_string(),
            data: serde_json::json!({ "component_id": id, "state": "uninstalled" }),
        }).await;
    }

    ok(serde_json::json!({ "deleted": id }))
}
```

- [ ] **Step 3: Register module + routes**

`handlers/mod.rs`: `pub mod frontend_components;`

`router.rs` — add to imports and routes:

Public routes (market list, bundle serve):
```rust
.route("/api/frontend-components/market/list",
    get(frontend_components::market_list_handler))
.route("/api/frontend-components/:id/bundle",
    get(frontend_components::get_bundle_handler))
```

Protected routes:
```rust
.route("/api/frontend-components/market/install",
    post(frontend_components::market_install_handler))
.route("/api/frontend-components",
    get(frontend_components::list_components_handler))
.route("/api/frontend-components",
    post(frontend_components::install_component_handler))
.route("/api/frontend-components/:id",
    delete(frontend_components::uninstall_component_handler))
```

Upload route with body limit (5MB):
```rust
let component_upload_routes = Router::new()
    .route("/api/frontend-components",
        post(frontend_components::install_component_handler)
            .layer(DefaultBodyLimit::max(5 * 1024 * 1024)))
    .route_layer(hybrid_auth_middleware)
    .route_layer(rate_limit_middleware);
```

- [ ] **Step 4: Build and verify**

Run: `cargo build -p neomind-api`

- [ ] **Step 5: Commit**

```
feat(api): add community component marketplace + manual install endpoints
```

---

## Task 3: New GitHub Repo — `NeoMind-Dashboard-Components`

- [ ] **Step 1: Create repo with example component**

Create `index.json`:
```json
{
  "version": "1.0.0",
  "components": [
    {
      "id": "clock",
      "name": { "en": "Clock", "zh": "时钟" },
      "description": { "en": "A real-time clock widget", "zh": "实时时钟组件" },
      "icon": "Clock",
      "category": "display",
      "version": "1.0.0",
      "author": "NeoMind Team",
      "size_constraints": { "min_w": 2, "min_h": 1, "default_w": 3, "default_h": 2, "max_w": 6, "max_h": 4 },
      "has_data_source": false,
      "has_display_config": true,
      "has_actions": false,
      "screenshot_url": "https://raw.githubusercontent.com/camthink-ai/NeoMind-Dashboard-Components/main/components/clock/screenshot.png",
      "manifest_url": "https://raw.githubusercontent.com/camthink-ai/NeoMind-Dashboard-Components/main/components/clock/manifest.json",
      "bundle_url": "https://raw.githubusercontent.com/camthink-ai/NeoMind-Dashboard-Components/main/components/clock/bundle.js"
    }
  ]
}
```

Create `components/clock/manifest.json` (same format as `ComponentManifest`).

Create `components/clock/bundle.js` — a simple IIFE React clock component:
```javascript
var ClockWidget = (function() {
  // Uses window.React, window.jsxRuntime from globals
  var React = window.React;
  var jsx = window.jsxRuntime.jsx;

  function Clock(props) {
    var state = React.useState(new Date());
    var time = state[0];
    var setTime = state[1];

    React.useEffect(function() {
      var timer = setInterval(function() { setTime(new Date()); }, 1000);
      return function() { clearInterval(timer); };
    }, []);

    var config = props.config || {};
    var format = config.format || '24h';
    var timeStr = format === '12h'
      ? time.toLocaleTimeString('en-US', { hour12: true })
      : time.toLocaleTimeString('en-US', { hour12: false });

    return jsx('div', {
      className: 'flex flex-col items-center justify-center h-full gap-1',
      children: [
        jsx('span', { key: 'time', className: 'text-3xl font-mono font-bold text-foreground', children: timeStr }),
        jsx('span', { key: 'date', className: 'text-sm text-muted-foreground', children: time.toLocaleDateString() })
      ]
    });
  }

  return { default: Clock, Clock: Clock };
})();
```

- [ ] **Step 2: Push to GitHub**

```bash
cd /Users/shenmingming/CamThink\ Project/
mkdir NeoMind-Dashboard-Components
cd NeoMind-Dashboard-Components
git init && git add . && git commit -m "init: dashboard component marketplace"
# Create repo on GitHub, push
```

- [ ] **Step 3: Commit**

```
chore: create NeoMind-Dashboard-Components repo with clock example
```

---

## Task 4: Frontend Types + CommunityRegistry

**Files:**
- Create: `web/src/types/frontend-component.ts`
- Create: `web/src/components/dashboard/registry/CommunityRegistry.ts`

- [ ] **Step 1: Create types**

`web/src/types/frontend-component.ts`:
```typescript
export interface SizeConstraints {
  min_w: number; min_h: number
  default_w: number; default_h: number
  max_w: number; max_h: number
}

export interface FrontendComponentMeta {
  id: string
  name: string | Record<string, string>
  description: string | Record<string, string>
  icon: string; category: string; version: string
  author?: string; screenshot?: string
  size_constraints: SizeConstraints
  has_data_source: boolean; max_data_sources?: number
  has_display_config: boolean; has_actions: boolean
  config_schema?: Record<string, unknown>
  default_config?: Record<string, unknown>
  variants?: string[]
  global_name: string; export_name?: string
  installed_at: number
}

export interface MarketComponentEntry {
  id: string
  name: string | Record<string, string>
  description: string | Record<string, string>
  icon: string; category: string; version: string
  author?: string; screenshot_url?: string
  size_constraints: SizeConstraints
  has_data_source: boolean; max_data_sources?: number
  has_display_config: boolean; has_actions: boolean
  manifest_url: string; bundle_url: string
}

export interface ComponentManifest {
  id: string
  name: string | Record<string, string>
  description: string | Record<string, string>
  icon?: string; category?: string; version?: string
  author?: string; screenshot?: string
  size_constraints: SizeConstraints
  has_data_source?: boolean; max_data_sources?: number
  has_display_config?: boolean; has_actions?: boolean
  config_schema?: Record<string, unknown>
  default_config?: Record<string, unknown>
  variants?: string[]
  global_name: string; export_name?: string
}
```

- [ ] **Step 2: Create CommunityRegistry** (same as previous version — IIFE script tag loading, syncFromApi, unregister)

`web/src/components/dashboard/registry/CommunityRegistry.ts`:
- `isCommunity(type)` / `getMeta(type)` / `getAllMetas()`
- `syncFromApi(metas: FrontendComponentMeta[])` — incremental sync
- `loadComponent(type)` — IIFE via script tag (reuse DynamicRegistry patterns)
- `unregister(type)` — cleanup global + cache
- `communityMetaToComponentMeta(meta)` — convert to `ComponentMeta`

(Carried forward from previous plan — same implementation)

- [ ] **Step 3: Verify tsc**

- [ ] **Step 4: Commit**

```
feat(web): add CommunityRegistry and frontend component types
```

---

## Task 5: Integrate into Registry + ComponentRenderer

**Files:**
- Modify: `web/src/components/dashboard/registry/registry.ts`
- Modify: `web/src/components/dashboard/registry/ComponentRenderer.tsx`

- [ ] **Step 1: Update `registry.ts`** — add community components to `getAllComponents()`, `filterComponents()`, `getComponentMeta()`, add `'community'` category

- [ ] **Step 2: Update `ComponentRenderer.tsx`** — add `CommunityComponentLoader` branch after dynamic registry check

- [ ] **Step 3: Verify tsc**

- [ ] **Step 4: Commit**

```
feat(web): integrate CommunityRegistry into dashboard component system
```

---

## Task 6: Zustand Store Slice

**Files:**
- Create: `web/src/store/slices/frontendComponentSlice.ts`

- [ ] **Step 1: Create store slice**

```typescript
interface FrontendComponentState {
  installed: FrontendComponentMeta[]
  marketComponents: MarketComponentEntry[]
  marketLoading: boolean
  loading: boolean
  error: string | null
  fetchCache: Record<string, { timestamp: number }>

  fetchInstalled: () => Promise<void>
  fetchMarket: () => Promise<void>
  installFromMarket: (componentId: string) => Promise<void>
  installManual: (manifest: ComponentManifest, bundleFile: File) => Promise<FrontendComponentMeta>
  uninstall: (id: string) => Promise<void>
}
```

- `fetchMarket()` → `GET /api/frontend-components/market/list`
- `installFromMarket(id)` → `POST /api/frontend-components/market/install`
- `installManual(manifest, bundle)` → `POST /api/frontend-components` (multipart)
- `fetchInstalled()` → `GET /api/frontend-components`
- `uninstall(id)` → `DELETE /api/frontend-components/:id`

- [ ] **Step 2: Commit**

```
feat(web): add frontend component Zustand store slice with marketplace support
```

---

## Task 7: WebSocket Event + Lifecycle Hook

**Files:**
- Modify: `web/src/lib/events.ts`
- Create: `web/src/hooks/useCommunityComponentLifecycle.ts`

- [ ] **Step 1: Add `FrontendComponentLifecycle` event type to `events.ts`**

- [ ] **Step 2: Create `useCommunityComponentLifecycle` hook**
  - Listens for `FrontendComponentLifecycle` custom events
  - `installed` → refetch installed components
  - `uninstalled` → immediately remove from registry + store

- [ ] **Step 3: Commit**

```
feat(web): add WebSocket lifecycle events for community components
```

---

## Task 8: UI — Marketplace + Import Dialog in ComponentLibrary Sidebar

**Files:**
- Create: `web/src/pages/dashboard-components/ComponentMarketplace.tsx`
- Create: `web/src/pages/dashboard-components/InstallComponentDialog.tsx`
- Modify: `web/src/pages/dashboard-components/ComponentLibrary.tsx`
- Modify: i18n files

### UI Layout

```
ComponentLibrary Sidebar
├── Search bar
├── Category: Indicators / Charts / ...
├── Category: Community (已安装的社区组件)
│   ├── Clock Widget    [v1.0.0]
│   └── RSS Feed        [v1.2.0]
├── ─────────────────────────
│   [🏪 组件市场]          ← 打开 marketplace 弹窗
│   [📦 导入组件]          ← 打开 import 弹窗
└──────────────────────────

ComponentMarketplace (FullScreenDialog):
├── Header: "组件市场"
├── Component cards (from GitHub index)
│   ├── [screenshot] Clock Widget v1.0.0 by NeoMind Team
│   │   "A real-time clock widget"
│   │   [安装] or [已安装 ✓]
│   └── ...
├── Empty state: "暂无可用组件"
└── Error state: "无法连接到组件市场"

InstallComponentDialog (UnifiedFormDialog):
├── Drop zone: manifest.json
├── Drop zone: bundle.js
├── Preview (parsed manifest info)
└── [确认安装]
```

- [ ] **Step 1: Create `ComponentMarketplace.tsx`**
  - `FullScreenDialog` wrapper
  - Fetch market components via store
  - Grid of component cards with screenshot, name, description, version
  - Install button → calls `installFromMarket(id)`
  - Shows "已安装 ✓" for already installed components
  - Uninstall button for installed ones

- [ ] **Step 2: Create `InstallComponentDialog.tsx`**
  - `UnifiedFormDialog`
  - Two drop zones (manifest + bundle)
  - Preview step after parsing manifest
  - Confirm button

- [ ] **Step 3: Modify `ComponentLibrary.tsx`**
  - Add bottom section with two buttons: "组件市场" + "导入组件"
  - `useCommunityComponentLifecycle()` hook for real-time updates
  - `fetchInstalled()` on mount

- [ ] **Step 4: Add i18n keys (en + zh)**

- [ ] **Step 5: Verify build**

- [ ] **Step 6: Commit**

```
feat(web): add component marketplace browser and import dialog to sidebar
```

---

## Task 9: Build Verification + Integration Test

- [ ] **Step 1: Backend build + tests**

Run: `cargo build -p neomind-api && cargo test -p neomind-storage -- frontend_components`

- [ ] **Step 2: Frontend build**

Run: `cd web && npm run build`

- [ ] **Step 3: Manual E2E test**

1. Open dashboard → open component library sidebar
2. Click "组件市场" → see clock component
3. Click "安装" → component downloads from GitHub
4. Component appears in "Community" category
5. Drag to dashboard → renders clock
6. Uninstall → component removed, dashboard shows fallback

- [ ] **Step 4: Final commit (if fixes needed)**

---

## Summary

| Task | Scope | Effort |
|------|-------|--------|
| 1. Storage Layer | Filesystem store | 30min |
| 2. API Handlers | 6 endpoints + EventBus | 60min |
| 3. GitHub Repo | index.json + example component | 30min |
| 4. CommunityRegistry | IIFE loader + type conversion | 40min |
| 5. Registry Integration | registry.ts + ComponentRenderer | 30min |
| 6. Zustand Slice | Store with market + CRUD | 30min |
| 7. WebSocket Events | Lifecycle hook | 20min |
| 8. UI | Marketplace + Import + Sidebar | 90min |
| 9. Build + Test | Verification | 20min |
| **Total** | **~20 files** | **~6h** |
