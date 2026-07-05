//! Image editing tool — crop, draw shapes/text/polygons, and blur regions.
//!
//! Returns an absolute file path under `data/images/` that the `vision` tool
//! (or another `image_edit` call) can consume directly. Operations are applied
//! as an in-memory RGBA pipeline; output is written atomically only after all
//! ops succeed.
//!
//! See `docs/superpowers/specs/2026-07-05-image-edit-tool-design.md` for the
//! full design contract. Notable invariants:
//! - `path` field in the response is absolute (vision.rs:263 requires this).
//! - File is written via current_dir().join() — NOT canonicalize — to avoid
//!   the macOS `/private/tmp → /var/` blocklist trap.

use crate::toolkit::error::{Result, ToolError};
use crate::toolkit::timeouts;
use crate::toolkit::tool::{Tool, ToolOutput};
use neomind_core::tools::ToolCategory;
use serde_json::Value;

/// Maximum input image size in bytes (10 MB). Matches `vision::MAX_IMAGE_SIZE`.
#[allow(dead_code)] // Wired into pipeline in Task 14.
const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

pub struct ImageEditTool {
    #[allow(dead_code)] // Wired into pipeline in Task 14.
    data_dir: std::path::PathBuf,
    #[allow(dead_code)] // Wired into pipeline in Task 14.
    http_client: reqwest::Client,
}

impl ImageEditTool {
    pub fn new(data_dir: impl Into<std::path::PathBuf>) -> Self {
        // Reuse the centralized timeout tier (DEFAULT = 30s, see toolkit/timeouts.rs).
        // `.no_proxy()` is intentionally OMITTED so the platform's HTTP(S)_PROXY
        // env vars apply (corporate proxy support, matches vision.rs behavior).
        let http_client = reqwest::Client::builder()
            .timeout(timeouts::DEFAULT)
            .build()
            .expect("reqwest client for image_edit");
        Self {
            data_dir: data_dir.into(),
            http_client,
        }
    }
}

#[async_trait::async_trait]
impl Tool for ImageEditTool {
    fn name(&self) -> &str {
        "image_edit"
    }

    fn description(&self) -> &str {
        // Filled in by Task 14 (pipeline executor + tool description).
        "Image editing tool (crop, draw, blur). Schema populated in a later task."
    }

    fn parameters(&self) -> Value {
        // Filled in by Task 14.
        serde_json::json!({})
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, _args: Value) -> Result<ToolOutput> {
        // Pipeline implementation arrives in Task 14.
        Err(ToolError::InvalidArguments(
            "image_edit not yet implemented".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_name_and_category() {
        let t = ImageEditTool::new("/tmp");
        assert_eq!(t.name(), "image_edit");
        assert!(matches!(t.category(), ToolCategory::System));
    }

    #[test]
    fn execute_returns_not_yet_implemented() {
        // Until Task 14 lands, execute() surfaces a clear error rather than
        // panicking or silently succeeding.
        let t = ImageEditTool::new("/tmp");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(t.execute(serde_json::json!({})));
        assert!(res.is_err());
    }
}
