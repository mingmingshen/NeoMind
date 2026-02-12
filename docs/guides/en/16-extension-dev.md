# Extension Development Guide

**Version**: 0.5.8
**Difficulty**: Medium
**Estimated Time**: 1-2 hours

## Overview

This guide will walk you through creating a NeoMind extension, from basic setup to full implementation.

## What is an Extension?

NeoMind extensions are dynamically loadable modules into NeoMind that provide:

- **Data Sources** - Fetch data from external systems (e.g., Weather API)
- **Device Adapters** - Support new device protocols (e.g., Modbus)
- **AI Tools** - Provide new capabilities to Agent
- **Alert Channels** - Send notifications to external services
- **LLM Backends** - Add custom LLM providers

## Quick Start

### 1. Create Project

```bash
# Create new library project using cargo
cargo new --lib my_neomind_extension

cd my_neomind_extension

# Add NeoMind SDK dependency
cargo add neomind-extension-sdk --path /path/to/neomind/crates/neomind-extension-sdk
```

### 2. Configure Cargo.toml

```toml
[package]
name = "my-neomind-extension"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
neomind-extension-sdk = "0.5.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Important**: `crate-type = ["cdylib"]` is required for generating dynamic libraries.

### 3. Write Extension

```rust
use neomind_extension_sdk::prelude::*;

struct MyExtension;

declare_extension!(
    MyExtension,
    metadata: ExtensionMetadata {
        name: "my.extension".to_string(),
        version: "0.1.0".to_string(),
        author: "Your Name".to_string(),
        description: "My first extension".to_string(),
    },
);

impl Extension for MyExtension {
    fn metadata(&self) -> &ExtensionMetadata {
        static METADATA: ExtensionMetadata = ExtensionMetadata {
            id: "my.extension".to_string(),
            name: "My Extension".to_string(),
            version: "0.1.0".to_string(),
            description: Some("My first NeoMind extension".to_string()),
            author: Some("Your Name".to_string()),
            homepage: Some("https://example.com".to_string()),
            license: Some("MIT".to_string()),
            file_path: None,
        };
        &METADATA
    }
}
```

### 4. Build

```bash
# macOS/Linux
cargo build --release

# Output location:
# macOS: target/release/libmy_neomind_extension.dylib
# Linux: target/release/libmy_neomind_extension.so
# Windows: target/release/my_neomind_extension.dll
```

### 5. Install

```bash
# Copy to extensions directory
mkdir -p ~/.neomind/extensions
cp target/release/libmy_neomind_extension.* ~/.neomind/extensions/

# Or register via API
curl -X POST http://localhost:9375/api/extensions \
  -H "Content-Type: application/json" \
  -d '{
    "file_path": "/path/to/libmy_neomind_extension.dylib"
  }'
```

## Complete Example: Weather Data Source

### Project Structure

```
weather-extension/
├── Cargo.toml
└── src/
    └── lib.rs
```

### Complete Code

```rust
use neomind_extension_sdk::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Weather extension state
struct WeatherExtension {
    api_key: Arc<Mutex<Option<String>>>,
    city: Arc<Mutex<String>>,
    last_update: Arc<Mutex<Option<i64>>>,
}

/// Declare extension
declare_extension!(
    WeatherExtension,
    metadata: ExtensionMetadata {
        name: "weather.extension".to_string(),
        version: "1.0.0".to_string(),
        author: "NeoMind Team".to_string(),
        description: "Weather data provider extension".to_string(),
    },
);

/// Metric definitions
const WEATHER_METRICS: &[MetricDefinition] = &[
    MetricDefinition {
        name: "temperature".to_string(),
        display_name: "Temperature".to_string(),
        data_type: MetricDataType::Float,
        unit: "°C".to_string(),
        min: Some(-50.0),
        max: Some(50.0),
        required: true,
    },
    MetricDefinition {
        name: "humidity".to_string(),
        display_name: "Humidity".to_string(),
        data_type: MetricDataType::Integer,
        unit: "%".to_string(),
        min: Some(0.0),
        max: Some(100.0),
        required: true,
    },
    MetricDefinition {
        name: "condition".to_string(),
        display_name: "Condition".to_string(),
        data_type: MetricDataType::String,
        unit: "".to_string(),
        min: None,
        max: None,
        required: true,
    },
];

/// Command definitions
const WEATHER_COMMANDS: &[ExtensionCommand] = &[
    ExtensionCommand {
        name: "set_city".to_string(),
        display_name: "Set City".to_string(),
        payload_template: "{ \"city\": \"{{city}}\" }".to_string(),
        parameters: vec![
            ParameterDefinition {
                name: "city".to_string(),
                display_name: "City".to_string(),
                description: "City name".to_string(),
                param_type: MetricDataType::String,
                required: true,
                default_value: None,
                min: None,
                max: None,
                options: vec![],
            },
        ],
        fixed_values: serde_json::Map::new(),
        llm_hints: "Set the city for weather data".to_string(),
        parameter_groups: vec![],
    },
    ExtensionCommand {
        name: "refresh".to_string(),
        display_name: "Refresh Weather".to_string(),
        payload_template: "{}".to_string(),
        parameters: vec![],
        fixed_values: serde_json::Map::new(),
        llm_hints: "Force refresh weather data".to_string(),
        parameter_groups: vec![],
    },
];

impl Extension for WeatherExtension {
    fn metadata(&self) -> &ExtensionMetadata {
        static METADATA: ExtensionMetadata = ExtensionMetadata {
            id: "weather.extension".to_string(),
            name: "Weather Extension".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Provides weather data for any city".to_string()),
            author: Some("NeoMind Team".to_string()),
            homepage: Some("https://github.com/neomind-platform".to_string()),
            license: Some("MIT".to_string()),
            file_path: None,
        };
        &METADATA
    }

    fn metrics(&self) -> &[MetricDefinition] {
        WEATHER_METRICS
    }

    fn commands(&self) -> &[ExtensionCommand] {
        WEATHER_COMMANDS
    }

    fn execute_command(&self, command: &str, args: &Value) -> Result<Value, ExtensionError> {
        match command {
            "set_city" => {
                if let Some(city) = args.get("city").and_then(|v| v.as_str()) {
                    *self.city.lock().unwrap() = city.to_string();
                    // Trigger data refresh
                    Ok(json!({
                        "status": "success",
                        "message": format!("City set to {}", city)
                    }))
                } else {
                    Err(ExtensionError::InvalidArguments(
                        "city parameter is required".to_string()
                    ))
                }
            }
            "refresh" => {
                // Trigger data refresh
                let city = self.city.lock().unwrap().clone();
                Ok(json!({
                    "status": "refreshing",
                    "city": city
                }))
            }
            _ => Err(ExtensionError::UnsupportedCommand {
                command: command.to_string(),
            })
        }
    }

    fn health_check(&self) -> Result<bool, ExtensionError> {
        // Check if API key is configured
        let api_key = self.api_key.lock().unwrap();
        Ok(api_key.is_some())
    }
}
```

## Extension Types Explained

### Data Source Extension (DataSource)

Provides external data for Agent and Rules to use:

```rust
impl Extension for MyDataSource {
    fn metrics(&self) -> &[MetricDefinition] {
        &[
            MetricDefinition {
                name: "price".to_string(),
                display_name: "Stock Price".to_string(),
                data_type: MetricDataType::Float,
                unit: "USD".to_string(),
                min: Some(0.0),
                max: None,
                required: true,
            },
        ]
    }
    // Update data in background
    // Report via ExtensionMetricsStorage
}
```

### Device Adapter Extension (DeviceAdapter)

Supports new device protocols:

```rust
impl Extension for MyDeviceAdapter {
    fn commands(&self) -> &[ExtensionCommand] {
        &[
            ExtensionCommand {
                name: "connect".to_string(),
                display_name: "Connect Device".to_string(),
                payload_template: "{ \"address\": \"{{address}}\" }".to_string(),
                parameters: vec![/* ... */],
                fixed_values: serde_json::Map::new(),
                llm_hints: "Connect to a device".to_string(),
                parameter_groups: vec![],
            },
        ]
    }
}
```

### AI Tool Extension (Tool)

Provides new capabilities to Agent:

```rust
impl Extension for MyToolExtension {
    fn commands(&self) -> &[ExtensionCommand] {
        &[
            ExtensionCommand {
                name: "calculate".to_string(),
                display_name: "Calculate".to_string(),
                payload_template: "{ \"expression\": \"{{expr}}\" }".to_string(),
                parameters: vec![/* ... */],
                fixed_values: serde_json::Map::new(),
                llm_hints: "Performs mathematical calculations".to_string(),
                parameter_groups: vec![],
            },
        ]
    }
}
```

### Alert Channel Extension (AlertChannel)

Sends notifications to external services:

```rust
use neomind_extension_sdk::types::ChannelDescriptor;

impl Extension for MyAlertChannel {
    // Return channel descriptor
    // Provided via config or other means
}
```

## Testing Extensions

### Local Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let ext = MyExtension;
        let meta = ext.metadata();
        assert_eq!(meta.id, "weather.extension");
    }

    #[test]
    fn test_execute_command() {
        let ext = MyExtension::new();
        let result = ext.execute_command("set_city", &json!({"city": "Tokyo"}));
        assert!(result.is_ok());
    }
}
```

### Integration Testing

```bash
cargo test

# Use NeoMind test framework
cargo test --package neomind-api --test extension_loader
```

## Deployment

### Manual Installation

```bash
# 1. Build
cargo build --release

# 2. Copy to extensions directory
cp target/release/libmy_extension.dylib ~/.neomind/extensions/

# 3. Restart NeoMind or use API discovery
curl -X POST http://localhost:9375/api/extensions/discover
```

### API Registration

```bash
# Register extension
curl -X POST http://localhost:9375/api/extensions \
  -H "Content-Type: application/json" \
  -d '{
    "file_path": "/absolute/path/to/libmy_extension.dylib"
  }'

# Start extension
curl -X POST http://localhost:9375/api/extensions/weather.extension/start

# Check health status
curl http://localhost:9375/api/extensions/weather.extension/health
```

## Best Practices

### 1. Error Handling

Always return meaningful error messages:

```rust
fn execute_command(&self, command: &str, args: &Value) -> Result<Value, ExtensionError> {
    match command {
        "fetch" => {
            let url = args.get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ExtensionError::InvalidArguments(
                    "url parameter is required".to_string()
                ))?;
            // ... execution logic
        }
        _ => Err(ExtensionError::UnsupportedCommand {
            command: command.to_string(),
        }),
    }
}
```

### 2. Resource Management

Use Arc<Mutex<T>> for shared state:

```rust
struct MyExtension {
    state: Arc<Mutex<ExtensionState>>,
}

impl Extension for MyExtension {
    // Safely access across threads
}
```

### 3. Configuration Management

Support runtime configuration:

```rust
impl Extension for MyExtension {
    fn execute_command(&self, command: &str, args: &Value) -> Result<Value, ExtensionError> {
        if command == "configure" {
            // Update configuration
            // Save to internal state
        }
    }
}
```

### 4. Data Reporting

Periodically report metric data:

```rust
// In extension's async task
loop {
    tokio::time::sleep(Duration::from_secs(60)).await;

    // Fetch data
    let data = fetch_weather_data().await?;

    // Report to NeoMind
    // Via ExtensionMetricsStorage API
    // Or via WebSocket
}
```

## Extension Templates

### Simple Template Project

```bash
# Clone template (if exists)
git clone https://github.com/neomind-platform/extension-template.git my-extension

# Or use cargo-generate
cargo install cargo-generate
cargo generate --git https://github.com/neomind-platform/extension-template \
  --name my-extension
```

### Minimal Cargo.toml

```toml
[package]
name = "my-extension"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
neomind-extension-sdk = { version = "0.5" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.release]
strip = true
lto = true
codegen-units = 1
opt-level = "z"
```

## Troubleshooting

### Compilation Errors

**"undefined symbol" error**:
- Ensure using `#[no_mangle]` on exported symbols
- Check function signatures are correct

**"wrong type" error**:
- Ensure using FFI-safe types (`repr(C)`)
- Avoid Rust-specific types

### Loading Errors

**"ABI version mismatch"**:
- Check `NEO_EXT_ABI_VERSION` matches server
- Ensure using latest SDK version

**"symbol not found"**:
- Ensure correct symbol names are exported
- Use `nm` or `objdump` to check exported symbols

### Runtime Errors

**"command not found"**:
- Check if `execute_command` handles the command
- Ensure command name is spelled correctly

**"health check failed"**:
- Implement or fix `health_check` method
- Ensure all dependencies are properly configured

## References

- [Extension SDK API Documentation](13-extension-sdk.md)
- [Core Module Documentation](01-core.md)
- [Migration Guide](../../architecture/plugin-migration.md)
- [Main Project Documentation](../../CLAUDE.md)

## Official Repositories

- **[NeoMind-Extensions](https://github.com/camthink-ai/NeoMind-Extensions)** - Community extension marketplace, samples, and templates
- **[NeoMind-DeviceTypes](https://github.com/camthink-ai/NeoMind-DeviceTypes)** - Supported device type definitions
- **[Example Extensions](https://github.com/neomind-platform/example-extensions)** - Example extension projects

## Next Steps

- Create your first extension
- View [example extensions](https://github.com/neomind-platform/example-extensions)
- Join NeoMind community discussion
