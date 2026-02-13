//! Extension loaders for native and WASM extensions.

pub mod native;
pub mod wasm;

pub use native::{LoadedNativeExtension, NativeExtensionLoader};
pub use wasm::WasmExtensionLoader;
