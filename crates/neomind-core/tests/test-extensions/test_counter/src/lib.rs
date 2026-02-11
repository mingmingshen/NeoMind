//! Test Counter Extension
//!
//! A simple extension for testing ExtensionOutput event publishing.
//! Provides a counter metric that can be incremented via commands.

use neomind_extension_sdk::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

/// Counter value that persists across command invocations
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Test Counter Extension
struct TestCounterExtension;

impl Extension for TestCounterExtension {
    fn metadata(&self) -> &ExtensionMetadata {
        ExtensionMetadata::new(
            "neomind.test.counter",
            "Test Counter",
            semver::Version::new(0, 1, 0),
        )
        .with_description("A simple counter extension for testing event publishing")
        .with_author("NeoMind")
    }

    fn metrics(&self) -> &[MetricDefinition] {
        &[
            MetricDefinition {
                name: "counter".to_string(),
                display_name: "Counter Value".to_string(),
                data_type: MetricDataType::Integer,
                unit: "count".to_string(),
                min: Some(0.0),
                max: None,
                required: true,
            }
        ]
    }

    fn commands(&self) -> &[ExtensionCommand] {
        &[
            ExtensionCommand {
                name: "get_counter".to_string(),
                display_name: "Get Counter".to_string(),
                payload_template: "{}".to_string(),
                parameters: vec![],
                fixed_values: serde_json::Map::new(),
                samples: vec![],
                llm_hints: "Get the current counter value".to_string(),
                parameter_groups: vec![],
            },
            ExtensionCommand {
                name: "increment".to_string(),
                display_name: "Increment Counter".to_string(),
                payload_template: "{}".to_string(),
                parameters: vec![],
                fixed_values: serde_json::Map::new(),
                samples: vec![],
                llm_hints: "Increment the counter by 1".to_string(),
                parameter_groups: vec![],
            },
            ExtensionCommand {
                name: "reset_counter".to_string(),
                display_name: "Reset Counter".to_string(),
                payload_template: "{}".to_string(),
                parameters: vec![],
                fixed_values: serde_json::Map::new(),
                samples: vec![],
                llm_hints: "Reset the counter to 0".to_string(),
                parameter_groups: vec![],
            },
        ]
    }

    fn execute_command(&self, _command: &str, _args: &Value) -> Result<Value, ExtensionError> {
        match _command {
            "get_counter" => {
                let value = COUNTER.load(Ordering::Relaxed);
                Ok(serde_json::json!({
                    "counter": value
                }))
            }
            "increment" => {
                let new_value = COUNTER.fetch_add(1, Ordering::Relaxed);
                Ok(serde_json::json!({
                    "counter": new_value
                }))
            }
            "reset_counter" => {
                COUNTER.store(0, Ordering::Relaxed);
                Ok(serde_json::json!({
                    "counter": 0
                }))
            }
            _ => {
                Err(ExtensionError::UnsupportedCommand {
                    command: _command.to_string(),
                })
            }
        }
    }

    fn health_check(&self) -> Result<bool, ExtensionError> {
        Ok(true)
    }

    fn configure(&mut self, _config: &Value) -> Result<(), ExtensionError> {
        // Reset counter when reconfigured
        COUNTER.store(0, Ordering::Relaxed);
        Ok(())
    }
}

/// Create the extension instance
#[no_mangle]
pub extern "C" fn neomind_extension_create(config: *const u8, config_len: usize) -> *mut () {
    use std::sync::Arc;
    use std::sync::RwLock;
    use tokio::runtime::Runtime;

    // Parse configuration
    let config_value = if config.is_null() || config_len == 0 {
        serde_json::Value::Object(Default::default())
    } else {
        unsafe {
            let slice = std::slice::from_raw_parts(config, config_len);
            match std::str::from_utf8(slice) {
                Ok(s) => serde_json::from_str(s).unwrap_or_default(),
                Err(_) => serde_json::Value::Object(Default::default()),
            }
        }
    };

    let extension = Arc::new(RwLock::new(TestCounterExtension));

    // Store in a static to prevent it from being dropped
    Box::leak(Box::into_raw(Arc::into_raw(extension))) as *mut ()
}

/// Cleanup the extension instance
#[no_mangle]
pub extern "C" fn neomind_extension_destroy(instance: *mut ()) {
    use std::sync::Arc;

    // Recreate the Arc to drop the extension
    let _ = unsafe { Arc::from_raw(instance as *mut _) };
}

/// Get extension metadata
#[no_mangle]
pub extern "C" fn neomind_extension_metadata() -> *mut u8 {
    use std::boxed::Box;

    let metadata = ExtensionMetadata::new(
        "neomind.test.counter",
        "Test Counter",
        semver::Version::new(0, 1, 0),
    )
    .with_description("A simple counter extension for testing event publishing");

    // Allocate the metadata on the heap (it will be copied by the loader)
    let metadata_bytes = Box::leak(serde_json::to_string(&metadata).into_bytes());

    // Return a pointer to the C string
    metadata_bytes.as_ptr() as *mut u8
}

/// Get extension ABI version
#[no_mangle]
pub extern "C" fn neomind_extension_abi_version() -> u32 {
    neomind_extension_sdk::NEO_EXT_ABI_VERSION
}
