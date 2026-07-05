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

/// Individual image editing operation (crop, draw, blur, etc.).
/// Internally tagged via `type` field — {"type":"crop","x":10,"y":10,...}.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Operation {
    #[serde(rename = "crop")]
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    // other variants added in later tasks — fields inlined per variant,
    // NO #[serde(flatten)] (incompatible with internally-tagged enums).
}

/// Operation-specific errors (pure validation/pixel logic, NO I/O).
#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("crop area ({x},{y},{w},{h}) outside image bounds {iw}×{ih}")]
    CropOutOfBounds {
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        iw: u32,
        ih: u32,
    },
    #[error("zero-area crop")]
    ZeroAreaCrop,
}

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

fn apply_crop(
    img: &mut image::DynamicImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> std::result::Result<(), OpError> {
    if w == 0 || h == 0 {
        return Err(OpError::ZeroAreaCrop);
    }
    let (iw, ih) = (img.width(), img.height());
    // checked_add — `x + w` could overflow u32 in release mode and panic otherwise.
    let x2 = x
        .checked_add(w)
        .ok_or(OpError::CropOutOfBounds { x, y, w, h, iw, ih })?;
    let y2 = y
        .checked_add(h)
        .ok_or(OpError::CropOutOfBounds { x, y, w, h, iw, ih })?;
    if x2 > iw || y2 > ih {
        return Err(OpError::CropOutOfBounds { x, y, w, h, iw, ih });
    }
    let cropped = img.crop_imm(x, y, w, h);
    *img = cropped;
    Ok(())
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

#[cfg(test)]
mod op_tests {
    use super::*;
    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

    fn solid(w: u32, h: u32, c: [u8; 4]) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, Rgba(c)))
    }

    #[test]
    fn crop_happy_path() {
        let mut img = solid(200, 200, [255, 0, 0, 255]);
        apply_crop(&mut img, 10, 10, 100, 80).unwrap();
        assert_eq!(img.dimensions(), (100, 80));
    }

    #[test]
    fn crop_out_of_bounds_rejected() {
        let mut img = solid(200, 200, [255, 0, 0, 255]);
        let err = apply_crop(&mut img, 150, 150, 100, 100).unwrap_err();
        assert!(matches!(err, OpError::CropOutOfBounds { .. }));
    }

    #[test]
    fn crop_zero_area_rejected() {
        let mut img = solid(200, 200, [255, 0, 0, 255]);
        let err = apply_crop(&mut img, 10, 10, 0, 50).unwrap_err();
        assert!(matches!(err, OpError::ZeroAreaCrop));
    }

    #[test]
    fn crop_overflow_returns_error_not_panic() {
        let mut img = solid(200, 200, [255, 0, 0, 255]);
        // width = u32::MAX would overflow `x + w` if unchecked.
        let err = apply_crop(&mut img, 100, 100, u32::MAX, 10).unwrap_err();
        assert!(matches!(err, OpError::CropOutOfBounds { .. }));
    }
}
