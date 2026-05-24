---
id: extension-development
name: Extension Development Guide
category: extension
origin: builtin
priority: 85
token_budget: 12000
triggers:
  keywords: [extension development, create extension, build extension, extension sdk, neomind extension, 扩展开发, 开发扩展, extension create, extension build, custom extension, neomind_export, Extension trait, capability, FFI, native extension, .nep]
  tool_target:
    - tool: extension
      actions: [create, build, validate, install, uninstall]
anti_triggers:
  keywords: [widget, component, IIFE, bundle.js, dashboard component]
---

# Extension Development Guide

NeoMind extensions are Rust programs running in isolated processes. They communicate with the platform via FFI and a capability system.

## Architecture

```
NeoMind Server
  └── Extension Runner (isolated process)
        └── Your Extension (.dylib/.so)
              ├── implements Extension trait
              ├── exposes FFI via neomind_export!()
              ├── produces metrics
              └── executes commands
```

- **Process isolation**: Each extension runs in its own process — crashes don't affect the server
- **FFI boundary**: ABI version 3, JSON-serialized IPC
- **Capabilities**: Extensions request platform access (device read/write, storage, events)

## Development Flow

### Step 1: Scaffold

```bash
neomind extension create my-extension --extension-type tool -o ./extensions
```

Generates:
```
extensions/my-extension/
├── Cargo.toml          # With neomind-extension-sdk dependency
├── src/lib.rs          # Basic extension skeleton
├── manifest.json       # Extension metadata
└── .gitignore
```

### Step 2: Implement the Extension Trait

```rust
use neomind_extension_sdk::prelude::*;

pub struct MyExtension;

#[async_trait]
impl Extension for MyExtension {
    fn metadata(&self) -> &ExtensionMetadata {
        // Return static metadata
    }

    fn commands(&self) -> Vec<ExtensionCommand> {
        // Define commands the extension supports
    }

    fn metrics(&self) -> Vec<MetricDescriptor> {
        // Define metrics the extension produces
    }

    async fn execute_command(&self, command: &str, args: &serde_json::Value) -> Result<serde_json::Value> {
        // Handle command execution
    }

    fn produce_metrics(&self) -> Result<Vec<ExtensionMetricValue>> {
        // Produce current metric values
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}

// Required: export FFI entry points
neomind_export!(MyExtension);
```

### Step 3: Define Metadata

```rust
use neomind_extension_sdk::{ExtensionMetadata, ExtensionType};

fn metadata() -> ExtensionMetadata {
    ExtensionMetadata::new("my-extension", "My Extension")
        .version("1.0.0")
        .description("Does something useful")
        .extension_type(ExtensionType::Tool)
        .min_neomind_version("0.5.0")
}
```

### Step 4: Define Commands

```rust
use neomind_extension_sdk::{CommandBuilder, ParamBuilder};

fn commands() -> Vec<ExtensionCommand> {
    vec![
        CommandBuilder::new("analyze", "Analyze input data")
            .param(ParamBuilder::new("input", "Data to analyze").required().string())
            .param(ParamBuilder::new("threshold", "Min confidence").float().default(0.5))
            .build(),
    ]
}
```

### Step 5: Define Metrics

```rust
use neomind_extension_sdk::{MetricBuilder, MetricDataType};

fn metrics() -> Vec<MetricDescriptor> {
    vec![
        MetricBuilder::new("result_count", "Number of results")
            .data_type(MetricDataType::Integer)
            .unit("count")
            .build(),
        MetricBuilder::new("confidence", "Average confidence")
            .data_type(MetricDataType::Float)
            .unit("percent")
            .build(),
    ]
}
```

### Step 6: Handle Commands

```rust
async fn execute_command(&self, command: &str, args: &serde_json::Value) -> Result<serde_json::Value> {
    match command {
        "analyze" => {
            let input = args["input"].as_str().unwrap_or("");
            let threshold = args["threshold"].as_f64().unwrap_or(0.5);
            // ... your logic ...
            Ok(serde_json::json!({"results": [], "count": 0}))
        }
        _ => Err(ExtensionError::command_not_found(command)),
    }
}
```

### Step 7: Produce Metrics

```rust
use neomind_extension_sdk::ExtensionMetricValue;

fn produce_metrics(&self) -> Result<Vec<ExtensionMetricValue>> {
    Ok(vec![
        ExtensionMetricValue::float("result_count", self.result_count as f64),
        ExtensionMetricValue::float("confidence", self.avg_confidence),
    ])
}
```

## Capability System

Extensions access platform features through capabilities:

| Capability | Description |
|-----------|-------------|
| `device_metrics_read` | Read device telemetry |
| `device_metrics_write` | Write device metrics |
| `device_control` | Send control commands to devices |
| `storage_query` | Query persistent storage |
| `event_publish` / `event_subscribe` | Publish/subscribe to events |
| `telemetry_history` | Query historical telemetry |
| `metrics_aggregate` | Aggregate metrics |
| `extension_call` | Call other extensions |
| `agent_trigger` | Trigger AI agents |
| `rule_trigger` | Trigger automation rules |

```rust
use neomind_extension_sdk::capabilities::CapabilityContext;

let ctx = CapabilityContext::default();
let response = ctx.invoke_capability(
    "device_metrics_read",
    &json!({"device_id": "sensor-001"}),
)?;
```

## Build, Package & Install

### Build

```bash
neomind extension build ./extensions/my-extension
```

Compiles in release mode, outputs `.nep` package.

### Cross-Platform Build

Build for multiple targets:

```bash
# macOS ARM64
cargo build --release --target aarch64-apple-darwin

# Linux x86_64
cross build --release --target x86_64-unknown-linux-gnu

# Linux ARM64 (Raspberry Pi)
cross build --release --target aarch64-unknown-linux-gnu
```

### Validate

```bash
neomind extension validate ./my-extension.nep --verbose
```

### Install

```bash
neomind extension install ./my-extension.nep
neomind extension status my-extension   # verify running
neomind extension logs my-extension     # check for errors
```

## Manifest Reference (manifest.json)

```json
{
  "format": "neomind-extension-package",
  "format_version": "2.0",
  "abi_version": 3,
  "id": "my-extension",
  "name": "My Extension",
  "version": "1.0.0",
  "sdk_version": "0.6.3",
  "type": "native",
  "binaries": {
    "darwin_aarch64": "binaries/darwin_aarch64/extension.dylib",
    "linux_x86_64": "binaries/linux_x86_64/extension.so",
    "linux_aarch64": "binaries/linux_aarch64/extension.so"
  },
  "frontend": {
    "components": []
  },
  "models": "models/"
}
```

## Frontend Components (Optional)

Extensions can bundle dashboard widgets:

```json
"frontend": {
  "components": [{
    "type": "my-widget",
    "name": "My Widget",
    "description": "Widget description",
    "category": "custom",
    "bundle_path": "frontend/my-widget.umd.cjs",
    "export_name": "MyWidget",
    "global_name": "MyExtensionWidget",
    "size_constraints": {"min_w":300,"min_h":250,"default_w":400,"default_h":350,"max_w":600,"max_h":500},
    "has_data_source": true,
    "max_data_sources": 1,
    "config_schema": { "type": "object", "properties": { ... } }
  }]
}
```

Frontend widgets use the same IIFE format as custom widgets — see `neomind widget` docs.

## Common Patterns

### Stateful Extension with ML Model

```rust
pub struct YoloExtension {
    model: Mutex<Option<Model>>,  // Lazy-loaded, kept across sessions
    results: Mutex<Vec<Detection>>,
}

impl YoloExtension {
    fn ensure_model_loaded(&self) -> Result<()> {
        let mut model = self.model.lock().unwrap();
        if model.is_none() {
            *model = Some(Model::load("models/yolov8n.onnx")?);
        }
        Ok(())
    }
}
```

### Periodic Metric Collection

The platform calls `produce_metrics()` at regular intervals. Use this for:
- Polling external APIs
- Reading sensors
- Aggregating data

### Error Handling

```rust
use neomind_extension_sdk::ExtensionError;

// Built-in error constructors
ExtensionError::command_not_found("analyze")
ExtensionError::invalid_parameter("threshold", "must be > 0")
ExtensionError::internal("model loading failed")
ExtensionError::capability_denied("device_metrics_read")
```

## Extension Management Commands

| Command | Description |
|---------|-------------|
| `neomind extension list` | List all installed extensions |
| `neomind extension get <ID>` | Get extension details (alias: `info`) |
| `neomind extension status <ID>` | Get runtime status |
| `neomind extension logs <ID> [--limit <N>]` | View extension logs |
| `neomind extension reload <ID>` | Reload extension (pick up code changes) |
| `neomind extension install <PATH>` | Install from .nep package |
| `neomind extension uninstall <ID>` | Uninstall extension |

### Check Extension Status

```bash
# List all extensions and their states
neomind extension list

# Get detailed info including metrics and commands
neomind extension info my-extension

# Check if extension is running properly
neomind extension status my-extension

# View recent logs for debugging
neomind extension logs my-extension --limit 50
```

## Common Errors & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| "ABI version mismatch" | Extension compiled with wrong SDK version | Rebuild with matching SDK version |
| "Extension crashed" | Panic in extension code | Check logs with `extension logs <ID>`; add error handling |
| "Capability denied" | Extension didn't request capability | Add to metadata capabilities list |
| "Library not found" | Binary path wrong in manifest | Check `binaries` paths match actual files in `.nep` |
| "Model file not found" | Model path relative to extension dir | Use `models/` directory referenced in manifest |
| "Validation failed" | Malformed manifest or missing fields | Run `extension validate --verbose` to see details |
