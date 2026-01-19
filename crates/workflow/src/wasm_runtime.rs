//! WASM runtime for executing user-defined code

use crate::error::{Result, WorkflowError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wasmtime::*;

/// WASM configuration
#[derive(Debug, Clone)]
pub struct WasmConfig {
    /// Memory limit in MB
    pub memory_limit: usize,
    /// Maximum execution time in seconds
    pub max_execution_time_seconds: u64,
    /// Enable WASI
    pub enable_wasi: bool,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            memory_limit: 64,
            max_execution_time_seconds: 30,
            enable_wasi: true,
        }
    }
}

/// WASM module
#[derive(Clone)]
pub struct WasmModule {
    /// Module bytes
    pub bytes: Vec<u8>,
    /// Module metadata
    pub metadata: ModuleMetadata,
}

/// Module metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleMetadata {
    /// Module ID
    pub id: String,
    /// Module name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Source language
    #[serde(default)]
    pub source_language: String,
    /// Created at
    pub created_at: i64,
}

/// Execution result from WASM function
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Return value (if any)
    pub return_value: Option<serde_json::Value>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Memory used in bytes
    pub memory_used: Option<usize>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Host functions state available to WASM modules
#[derive(Clone)]
struct HostState {
    /// Device metric getter - would be connected to actual device system
    _get_metric: Arc<dyn Fn(&str, &str) -> Option<f64> + Send + Sync>,
    /// Command sender - would be connected to actual command system
    _send_command: Arc<dyn Fn(&str, &str, &str) -> Result<serde_json::Value> + Send + Sync>,
}

impl HostState {
    fn new() -> Self {
        Self {
            _get_metric: Arc::new(|_device_id: &str, _metric: &str| None),
            _send_command: Arc::new(|_device_id: &str, _command: &str, _params: &str| {
                Ok(serde_json::json!({"status": "queued"}))
            }),
        }
    }
}

impl Default for HostState {
    fn default() -> Self {
        Self::new()
    }
}

/// WASM runtime
pub struct WasmRuntime {
    engine: Engine,
    config: WasmConfig,
    modules: Arc<RwLock<HashMap<String, WasmModule>>>,
    _host_state: HostState,
}

impl WasmRuntime {
    /// Create a new WASM runtime
    pub fn new() -> Result<Self> {
        Self::with_config(WasmConfig::default())
    }

    /// Create a WASM runtime with custom configuration
    pub fn with_config(config: WasmConfig) -> Result<Self> {
        let mut engine_config = Config::new();
        engine_config.wasm_component_model(true);
        engine_config.async_support(true);
        engine_config.consume_fuel(true);

        // Configure memory limits
        engine_config.max_wasm_stack(1024 * 1024); // 1MB max stack

        let engine =
            Engine::new(&engine_config).map_err(|e| WorkflowError::WasmError(e.to_string()))?;

        Ok(Self {
            engine,
            config,
            modules: Arc::new(RwLock::new(HashMap::new())),
            _host_state: HostState::new(),
        })
    }

    /// Load a WASM module
    pub async fn load_module(
        &self,
        id: String,
        bytes: Vec<u8>,
        metadata: ModuleMetadata,
    ) -> Result<()> {
        // Validate the module
        self.validate_module(&bytes)?;

        let module = WasmModule { bytes, metadata };
        let mut modules = self.modules.write().await;
        modules.insert(id.clone(), module);
        Ok(())
    }

    /// Get a module
    pub async fn get_module(&self, id: &str) -> Option<WasmModule> {
        let modules = self.modules.read().await;
        modules.get(id).cloned()
    }

    /// List all module IDs
    pub async fn list_modules(&self) -> Vec<String> {
        let modules = self.modules.read().await;
        modules.keys().cloned().collect()
    }

    /// Remove a module
    pub async fn remove_module(&self, id: &str) -> bool {
        let mut modules = self.modules.write().await;
        modules.remove(id).is_some()
    }

    /// Execute a WASM function with proper instantiation
    pub async fn execute(
        &self,
        module_id: &str,
        function_name: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<WasmExecutionResult> {
        let start = std::time::Instant::now();

        let wasm_module = self
            .get_module(module_id)
            .await
            .ok_or_else(|| WorkflowError::WasmError(format!("Module not found: {}", module_id)))?;

        // Create a linker (host functions can be added later as needed)
        let linker = Linker::new(&self.engine);

        // Compile and instantiate the module
        let module = Module::from_binary(&self.engine, &wasm_module.bytes)
            .map_err(|e| WorkflowError::WasmError(format!("Failed to compile module: {}", e)))?;

        let mut store = Store::new(&self.engine, HostState::new());

        // Set fuel limit for timeout protection
        store
            .set_fuel(self.config.max_execution_time_seconds * 1_000_000_000)
            .map_err(|e| WorkflowError::WasmError(format!("Failed to set fuel: {}", e)))?;

        // Pre-instantiate the module
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| WorkflowError::WasmError(format!("Failed to instantiate module: {}", e)))?;

        // Get the exported function
        let func = instance
            .get_func(&mut store, function_name)
            .ok_or_else(|| {
                WorkflowError::WasmError(format!("Function '{}' not found in module", function_name))
            })?;

        // Convert JSON args to WASM values
        let mut wasm_args = Vec::new();
        for arg in args {
            match arg {
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        wasm_args.push(Val::I64(i));
                    } else if let Some(f) = n.as_f64() {
                        wasm_args.push(Val::F64(f.to_bits()));
                    }
                }
                serde_json::Value::Bool(b) => {
                    wasm_args.push(Val::I32(if b { 1 } else { 0 }));
                }
                serde_json::Value::String(_) => {
                    // Strings would need memory allocation
                    wasm_args.push(Val::I32(0));
                }
                serde_json::Value::Null => {
                    wasm_args.push(Val::I32(0));
                }
                _ => {}
            }
        }

        // Get function type to determine number of results
        let func_ty = func.ty(&store);
        let num_results = func_ty.results().len();

        // Prepare result buffer - use I32(0) as placeholder for each result
        let mut results = vec![Val::I32(0); num_results];

        // Call the function
        func.call(&mut store, &wasm_args, &mut results)
            .map_err(|e| WorkflowError::WasmError(format!("Function call failed: {}", e)))?;

        // Get fuel consumed for execution metrics
        // Note: fuel tracking APIs vary between wasmtime versions
        // Use a placeholder for memory tracking
        let fuel_consumed = 0u64; // TODO: Track actual execution cost

        // Convert result to JSON
        let return_value = if !results.is_empty() {
            Some(convert_wasm_val_to_json(&results[0]))
        } else {
            None
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(WasmExecutionResult {
            success: true,
            return_value,
            duration_ms,
            memory_used: Some(fuel_consumed as usize),
            error: None,
        })
    }

    /// Execute a WASM function with timeout protection
    pub async fn execute_with_timeout(
        &self,
        module_id: &str,
        function_name: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<WasmExecutionResult> {
        let timeout = tokio::time::Duration::from_secs(self.config.max_execution_time_seconds);

        match tokio::time::timeout(timeout, self.execute(module_id, function_name, args)).await {
            Ok(result) => result,
            Err(_) => Ok(WasmExecutionResult {
                success: false,
                return_value: None,
                duration_ms: self.config.max_execution_time_seconds * 1000,
                memory_used: None,
                error: Some("Execution timeout".to_string()),
            }),
        }
    }

    /// Validate a WASM module
    fn validate_module(&self, bytes: &[u8]) -> Result<()> {
        // Try to compile to validate
        Module::from_binary(&self.engine, bytes)
            .map_err(|e| WorkflowError::WasmError(format!("Invalid WASM module: {}", e)))?;
        Ok(())
    }

    /// Compile from Wat (WebAssembly Text format) - requires wat feature
    #[cfg(feature = "wat")]
    pub fn compile_wat(&self, wat: &str) -> Result<Vec<u8>> {
        wat::parse_str(wat)
            .map_err(|e| WorkflowError::CompilationError(format!("Failed to parse Wat: {}", e)))
    }
}

/// Convert a WasmValue to JSON
fn convert_wasm_val_to_json(val: &Val) -> serde_json::Value {
    match val {
        Val::I32(i) => serde_json::json!(i),
        Val::I64(i) => serde_json::json!(i),
        Val::F32(f) => {
            // F32 stores IEEE 754 bits as u32, interpret as float
            serde_json::json!(f32::from_bits(*f))
        }
        Val::F64(f) => {
            // F64 stores IEEE 754 bits as u64, interpret as float
            serde_json::json!(f64::from_bits(*f))
        }
        Val::V128(v) => {
            // V128 is a 128-bit vector, serialize as string representation
            serde_json::json!(format!("{:?}", v))
        }
        Val::FuncRef(_) => serde_json::json!(null),
        Val::ExternRef(_) => serde_json::json!(null),
        Val::AnyRef(_) => serde_json::json!(null),
        _ => serde_json::json!(null),
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback if runtime creation fails
            let engine_config = Config::new();
            let engine = Engine::new(&engine_config).unwrap();
            Self {
                engine,
                config: WasmConfig::default(),
                modules: Arc::new(RwLock::new(HashMap::new())),
                _host_state: HostState::new(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_config_default() {
        let config = WasmConfig::default();
        assert_eq!(config.memory_limit, 64);
        assert_eq!(config.max_execution_time_seconds, 30);
        assert!(config.enable_wasi);
    }

    #[tokio::test]
    async fn test_wasm_runtime_creation() {
        let runtime = WasmRuntime::new().unwrap();
        assert_eq!(runtime.list_modules().await.len(), 0);
    }

    #[test]
    fn test_convert_wasm_val_to_json() {
        assert_eq!(convert_wasm_val_to_json(&Val::I32(42)), serde_json::json!(42));
        assert_eq!(convert_wasm_val_to_json(&Val::I64(-100)), serde_json::json!(-100));
        // F32 stores IEEE 754 bits directly
        assert_eq!(
            convert_wasm_val_to_json(&Val::F32(3.14_f32.to_bits())),
            serde_json::json!(3.14_f32)
        );
    }
}
