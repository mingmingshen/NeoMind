//! Extension loaders for native and WASM extensions.

mod native;
mod wasm;

pub use native::NativeExtensionLoader;
pub use wasm::WasmExtensionLoader;

use std::path::Path;

/// Check if a path is a native extension file.
pub fn is_native_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| matches!(ext, "so" | "dylib" | "dll"))
        .unwrap_or(false)
}

/// Check if a path is a WASM extension file.
pub fn is_wasm_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext == "wasm")
        .unwrap_or(false)
}
