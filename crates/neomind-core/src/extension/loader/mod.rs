//! Extension loaders for native and WASM extensions.

pub mod native;
pub mod wasm;

pub use native::{NativeExtensionLoader, LoadedNativeExtension};
pub use wasm::WasmExtensionLoader;
