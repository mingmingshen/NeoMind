//! Example NeoTalk Plugin
//!
//! This is a simple example plugin that demonstrates how to use the NeoTalk Plugin SDK.
//!
//! # Features
//!
//! - Echo command: Returns the input as-is
//! - Reverse command: Reverses the input string
//! - Uppercase command: Converts input to uppercase
//! - Get info: Returns plugin information

use neotalk_plugin_sdk::{PluginError, PluginResult, PluginType};
use serde_json::json;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

/// Plugin instance state
struct ExamplePlugin {
    /// Plugin configuration
    config: serde_json::Value,

    /// Request counter
    request_count: Arc<AtomicU64>,
}

// FFI-safe wrapper for the plugin
struct PluginWrapper {
    plugin: Option<ExamplePlugin>,
}

// Global plugin state
static mut PLUGIN_STATE: Option<PluginWrapper> = None;
static INIT: std::sync::Once = std::sync::Once::new();

impl ExamplePlugin {
    /// Create a new plugin instance
    fn new(config: &serde_json::Value) -> PluginResult<Self> {
        Ok(Self {
            config: config.clone(),
            request_count: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Handle a command
    fn handle(&self, command: &str, args: &serde_json::Value) -> PluginResult<serde_json::Value> {
        let count = self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        match command {
            "echo" => {
                let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
                Ok(json!({
                    "result": input,
                    "request_number": count,
                }))
            }

            "reverse" => {
                let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
                Ok(json!({
                    "result": input.chars().rev().collect::<String>(),
                    "request_number": count,
                }))
            }

            "uppercase" => {
                let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
                Ok(json!({
                    "result": input.to_uppercase(),
                    "request_number": count,
                }))
            }

            "lowercase" => {
                let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
                Ok(json!({
                    "result": input.to_lowercase(),
                    "request_number": count,
                }))
            }

            "get_info" => Ok(json!({
                "plugin_id": "example-plugin",
                "plugin_name": "Example Plugin",
                "version": "0.1.0",
                "description": "A simple example plugin that demonstrates the NeoTalk Plugin SDK",
                "request_count": count,
                "config": self.config,
            })),

            "get_stats" => Ok(json!({
                "total_requests": count,
            })),

            _ => Err(PluginError::ExecutionFailed(format!(
                "Unknown command: {}",
                command
            ))),
        }
    }
}

/// Create a plugin instance
fn create_plugin(config: &serde_json::Value) -> PluginResult<()> {
    let plugin = ExamplePlugin::new(config)?;
    unsafe {
        PLUGIN_STATE = Some(PluginWrapper { plugin: Some(plugin) });
    }
    Ok(())
}

/// Get the plugin instance
unsafe fn get_plugin() -> Option<&'static ExamplePlugin> {
    unsafe { PLUGIN_STATE.as_ref()?.plugin.as_ref() }
}

/// Plugin descriptor (static)
const PLUGIN_ID: &str = "example-plugin";
const PLUGIN_NAME: &str = "Example Plugin";
const PLUGIN_VERSION: &str = "0.1.0";
const PLUGIN_TYPE: &str = "tool";
const PLUGIN_DESCRIPTION: &str = "A simple example plugin that demonstrates the NeoTalk Plugin SDK";
const PLUGIN_AUTHOR: &str = "NeoTalk Contributors";

// Get the length of a string at compile time
const fn str_len(s: &str) -> usize {
    s.as_bytes().len()
}

// Export the descriptor
#[no_mangle]
pub static neotalk_plugin_descriptor: CPluginDescriptor = CPluginDescriptor {
    abi_version: 1,
    plugin_type: PLUGIN_TYPE.as_ptr(),
    plugin_type_len: str_len(PLUGIN_TYPE),
    id: PLUGIN_ID.as_ptr(),
    id_len: str_len(PLUGIN_ID),
    name: PLUGIN_NAME.as_ptr(),
    name_len: str_len(PLUGIN_NAME),
    version: PLUGIN_VERSION.as_ptr(),
    version_len: str_len(PLUGIN_VERSION),
    description: PLUGIN_DESCRIPTION.as_ptr(),
    description_len: str_len(PLUGIN_DESCRIPTION),
    required_neotalk: ">=1.0.0".as_ptr(),
    required_neotalk_len: 7,
    author: PLUGIN_AUTHOR.as_ptr(),
    author_len: str_len(PLUGIN_AUTHOR),
    homepage: std::ptr::null(),
    homepage_len: 0,
    repository: std::ptr::null(),
    repository_len: 0,
    license: std::ptr::null(),
    license_len: 0,
    create_fn: neotalk_plugin_create as *const (),
    destroy_fn: neotalk_plugin_destroy as *const (),
    config_schema: std::ptr::null(),
    config_schema_len: 0,
    capabilities: 0x84, // ASYNC | THREAD_SAFE
};

/// C-compatible plugin descriptor
#[repr(C)]
pub struct CPluginDescriptor {
    pub abi_version: u32,
    pub plugin_type: *const u8,
    pub plugin_type_len: usize,
    pub id: *const u8,
    pub id_len: usize,
    pub name: *const u8,
    pub name_len: usize,
    pub version: *const u8,
    pub version_len: usize,
    pub description: *const u8,
    pub description_len: usize,
    pub required_neotalk: *const u8,
    pub required_neotalk_len: usize,
    pub author: *const u8,
    pub author_len: usize,
    pub homepage: *const u8,
    pub homepage_len: usize,
    pub repository: *const u8,
    pub repository_len: usize,
    pub license: *const u8,
    pub license_len: usize,
    pub create_fn: *const (),
    pub destroy_fn: *const (),
    pub config_schema: *const u8,
    pub config_schema_len: usize,
    pub capabilities: u64,
}

// SAFETY: The descriptor is only read, never modified
unsafe impl Sync for CPluginDescriptor {}

/// Create function
#[no_mangle]
pub extern "C" fn neotalk_plugin_create(
    config_json: *const u8,
    config_len: usize,
) -> *mut () {
    use std::ptr;

    // Parse the config
    let config_str = if config_json.is_null() || config_len == 0 {
        "{}"
    } else {
        unsafe {
            let slice = std::slice::from_raw_parts(config_json, config_len);
            match std::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => return ptr::null_mut(),
            }
        }
    };

    let config: serde_json::Value = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => return ptr::null_mut(),
    };

    // Initialize the plugin
    INIT.call_once(|| {
        if let Err(_) = create_plugin(&config) {
            eprintln!("Failed to initialize plugin");
        }
    });

    // Return a non-null pointer to indicate success
    0x1 as *mut ()
}

/// Destroy function
#[no_mangle]
pub extern "C" fn neotalk_plugin_destroy(_instance: *mut ()) {
    unsafe {
        PLUGIN_STATE = None;
    }
}

/// Get info function (exported for testing)
#[no_mangle]
pub extern "C" fn neotalk_plugin_get_info() -> *const u8 {
    // Return JSON info as a string
    static INFO: &str = r#"{"id":"example-plugin","name":"Example Plugin","version":"0.1.0"}"#;
    INFO.as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let config = serde_json::json!({});
        let plugin = ExamplePlugin::new(&config);
        assert!(plugin.is_ok());
    }

    #[test]
    fn test_echo_command() {
        let config = serde_json::json!({});
        let plugin = ExamplePlugin::new(&config).unwrap();

        let args = serde_json::json!({"input": "hello"});
        let result = plugin.handle("echo", &args);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result["result"], "hello");
        assert_eq!(result["request_number"], 0);
    }

    #[test]
    fn test_reverse_command() {
        let config = serde_json::json!({});
        let plugin = ExamplePlugin::new(&config).unwrap();

        let args = serde_json::json!({"input": "hello"});
        let result = plugin.handle("reverse", &args);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result["result"], "olleh");
    }

    #[test]
    fn test_uppercase_command() {
        let config = serde_json::json!({});
        let plugin = ExamplePlugin::new(&config).unwrap();

        let args = serde_json::json!({"input": "hello"});
        let result = plugin.handle("uppercase", &args);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result["result"], "HELLO");
    }

    #[test]
    fn test_unknown_command() {
        let config = serde_json::json!({});
        let plugin = ExamplePlugin::new(&config).unwrap();

        let args = serde_json::json!({});
        let result = plugin.handle("unknown", &args);

        assert!(result.is_err());
    }
}
