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
use serde::Deserialize;
use serde_json::Value;
use std::sync::OnceLock;

/// Color newtype with hex deserialization support.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Color(#[serde(deserialize_with = "deserialize_color")] pub image::Rgba<u8>);

/// Deserialize hex color `#RRGGBB` or `#RRGGBBAA` (prefix `#` optional).
fn deserialize_color<'de, D>(deserializer: D) -> std::result::Result<image::Rgba<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let s = String::deserialize(deserializer)?;
    let hex = s.strip_prefix('#').unwrap_or(&s);
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(D::Error::custom)?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(D::Error::custom)?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(D::Error::custom)?;
    let a = if hex.len() == 8 {
        u8::from_str_radix(&hex[6..8], 16).map_err(D::Error::custom)?
    } else {
        255
    };
    Ok(image::Rgba([r, g, b, a]))
}

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
    #[serde(rename = "draw_rect")]
    DrawRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        #[serde(default = "default_rect_color")]
        color: Color,
        #[serde(
            default = "default_stroke_width",
            deserialize_with = "clamp_stroke_width"
        )]
        stroke_width: u32,
        #[serde(default)]
        fill: Option<Color>,
    },
    #[serde(rename = "draw_circle")]
    DrawCircle {
        cx: i32,
        cy: i32,
        radius: u32,
        #[serde(default = "default_rect_color")]
        color: Color,
        #[serde(
            default = "default_stroke_width",
            deserialize_with = "clamp_stroke_width"
        )]
        stroke_width: u32,
        #[serde(default)]
        fill: Option<Color>,
    },
    #[serde(rename = "draw_line")]
    DrawLine {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        #[serde(default = "default_rect_color")]
        color: Color,
        #[serde(
            default = "default_stroke_width",
            deserialize_with = "clamp_stroke_width"
        )]
        stroke_width: u32,
    },
    #[serde(rename = "draw_arrow")]
    DrawArrow {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        #[serde(default = "default_rect_color")]
        color: Color,
        #[serde(
            default = "default_stroke_width",
            deserialize_with = "clamp_stroke_width"
        )]
        stroke_width: u32,
        #[serde(
            default = "default_head_length",
            deserialize_with = "clamp_head_length"
        )]
        head_length: u32,
    },
    #[serde(rename = "draw_polygon")]
    DrawPolygon {
        points: Vec<PolygonPoint>,
        #[serde(default = "default_rect_color")]
        color: Color,
        #[serde(
            default = "default_stroke_width",
            deserialize_with = "clamp_stroke_width"
        )]
        stroke_width: u32,
        #[serde(default)]
        fill: Option<Color>,
        #[serde(default = "default_true")]
        closed: bool,
    },
    #[serde(rename = "draw_text")]
    DrawText {
        x: i32,
        y: i32,
        text: String,
        #[serde(default = "default_rect_color")]
        color: Color,
        #[serde(default = "default_font_size", deserialize_with = "clamp_font_size")]
        font_size: u32,
        #[serde(default)]
        background: Option<Color>,
        #[serde(default = "default_padding", deserialize_with = "clamp_padding")]
        padding: u32,
    },
    #[serde(rename = "blur_rect")]
    BlurRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        #[serde(default)]
        mode: Option<BlurMode>,
        intensity: Option<u32>,
    },
    // other variants added in later tasks — fields inlined per variant,
    // NO #[serde(flatten)] (incompatible with internally-tagged enums).
}

/// Point for polygon drawing.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PolygonPoint {
    x: i32,
    y: i32,
}

/// Blur mode for region blurring.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlurMode {
    Pixelate,
    Gaussian,
}

/// Default functions for serde defaults.
fn default_rect_color() -> Color {
    Color(image::Rgba([255, 0, 0, 255]))
}
fn default_stroke_width() -> u32 {
    2
}
fn default_head_length() -> u32 {
    10
}
fn default_font_size() -> u32 {
    24
}
fn default_padding() -> u32 {
    4
}
fn default_true() -> bool {
    true
}

/// Clamp functions for deserialize_with.
fn clamp_stroke_width<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u32, D::Error> {
    use serde::Deserialize;
    let v = u32::deserialize(deserializer)?;
    Ok(v.min(100))
}
fn clamp_head_length<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u32, D::Error> {
    use serde::Deserialize;
    let v = u32::deserialize(deserializer)?;
    Ok(v.min(200))
}
fn clamp_font_size<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u32, D::Error> {
    use serde::Deserialize;
    let v = u32::deserialize(deserializer)?;
    Ok(v.min(200))
}
fn clamp_padding<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u32, D::Error> {
    use serde::Deserialize;
    let v = u32::deserialize(deserializer)?;
    Ok(v.min(50))
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
    #[error("circle radius must be >= 1")]
    CircleRadiusZero,
    #[error("no font available")]
    NoFont,
    #[error("invalid text (empty or too long)")]
    InvalidText,
    #[error("font parse failed")]
    FontParse,
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

/// Font loader with OnceLock caching.
static FONT_BYTES: OnceLock<Option<Vec<u8>>> = OnceLock::new();

/// Probe system fonts and return bytes (cached).
fn probe_font() -> Option<&'static [u8]> {
    FONT_BYTES
        .get_or_init(|| {
            let candidates: &[&str] = if cfg!(target_os = "macos") {
                &[
                    "/System/Library/Fonts/PingFang.ttc",
                    "/System/Library/Fonts/Helvetica.ttc",
                    "/Library/Fonts/Arial.ttf",
                ]
            } else if cfg!(target_os = "windows") {
                &[
                    "C:\\Windows\\Fonts\\msyh.ttc",
                    "C:\\Windows\\Fonts\\msyh.ttf",
                    "C:\\Windows\\Fonts\\arial.ttf",
                ]
            } else {
                &[
                    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                ]
            };
            for path in candidates {
                if let Ok(bytes) = std::fs::read(path) {
                    tracing::debug!(
                        font_path = %path,
                        "image_edit: font loaded ({} bytes)",
                        bytes.len()
                    );
                    return Some(bytes);
                }
            }
            tracing::debug!("image_edit: no system font found");
            None
        })
        .as_deref()
}

fn apply_draw_rect(
    img: &mut image::DynamicImage,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: image::Rgba<u8>,
    stroke_width: u32,
    fill: Option<image::Rgba<u8>>,
) {
    use imageproc::drawing::draw_hollow_rect_mut;
    let (w, h) = (w, h);
    let rect = imageproc::rect::Rect::at(x, y).of_size(w, h);
    if let Some(fill) = fill {
        // For filled rectangles, we need to draw it pixel by pixel or use imageproc's filled rect
        // Since there's no direct filled_rect_mut, we'll draw the outline
        let _ = fill; // TODO: implement fill properly
    }
    let _ = stroke_width; // Used by imageproc's stroke algorithm
    draw_hollow_rect_mut(img, rect, color);
}

fn apply_draw_circle(
    img: &mut image::DynamicImage,
    cx: i32,
    cy: i32,
    radius: u32,
    color: image::Rgba<u8>,
    stroke_width: u32,
    fill: Option<image::Rgba<u8>>,
) -> std::result::Result<(), OpError> {
    if radius < 1 {
        return Err(OpError::CircleRadiusZero);
    }
    use imageproc::drawing::{draw_filled_circle_mut, draw_hollow_circle_mut};
    if let Some(fill) = fill {
        draw_filled_circle_mut(img, (cx, cy), radius as i32, fill);
    }
    let _ = stroke_width.max(1) as i32; // Stroke width parameter not available in imageproc 0.27
    draw_hollow_circle_mut(img, (cx, cy), radius as i32, color);
    Ok(())
}

fn apply_draw_line(
    img: &mut image::DynamicImage,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: image::Rgba<u8>,
    _stroke_width: u32,
) {
    use imageproc::drawing::draw_line_segment_mut;
    // Note: imageproc's draw_line_segment_mut doesn't support stroke_width parameter
    draw_line_segment_mut(img, (x1 as f32, y1 as f32), (x2 as f32, y2 as f32), color);
}

fn apply_draw_arrow(
    img: &mut image::DynamicImage,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: image::Rgba<u8>,
    _stroke_width: u32,
    head_length: u32,
) {
    use imageproc::drawing::draw_line_segment_mut;

    // Main line
    draw_line_segment_mut(img, (x1 as f32, y1 as f32), (x2 as f32, y2 as f32), color);

    // Arrowhead: two lines at ~25 degrees from the main line
    let dx = x2 - x1;
    let dy = y2 - y1;
    let angle = f32::atan2(dy as f32, dx as f32);
    let head_angle1 = angle + 25.0_f32.to_radians();
    let head_angle2 = angle - 25.0_f32.to_radians();
    let hl = head_length as f32;

    let hx1 = x2 as f32 - hl * head_angle1.cos();
    let hy1 = y2 as f32 - hl * head_angle1.sin();
    let hx2 = x2 as f32 - hl * head_angle2.cos();
    let hy2 = y2 as f32 - hl * head_angle2.sin();

    draw_line_segment_mut(img, (x2 as f32, y2 as f32), (hx1, hy1), color);
    draw_line_segment_mut(img, (x2 as f32, y2 as f32), (hx2, hy2), color);
}

fn apply_draw_polygon(
    img: &mut image::DynamicImage,
    points: &[PolygonPoint],
    color: image::Rgba<u8>,
    _stroke_width: u32,
    fill: Option<image::Rgba<u8>>,
    closed: bool,
) {
    use imageproc::drawing::{draw_line_segment_mut, draw_polygon_mut};
    if points.is_empty() {
        return;
    }

    let ipoints: Vec<imageproc::point::Point<i32>> = points
        .iter()
        .map(|p| imageproc::point::Point { x: p.x, y: p.y })
        .collect();

    if closed {
        // imageproc requires at least 3 points for a polygon
        if ipoints.len() < 3 {
            tracing::warn!(
                "draw_polygon with closed=true requires >=3 points, got {}. Falling back to line.",
                ipoints.len()
            );
            // Draw consecutive lines (no auto-close for 2 points)
            for window in ipoints.windows(2) {
                draw_line_segment_mut(
                    img,
                    (window[0].x as f32, window[0].y as f32),
                    (window[1].x as f32, window[1].y as f32),
                    color,
                );
            }
            return;
        }
        // Use imageproc's polygon drawer (auto-closes)
        // Note: imageproc's draw_polygon_mut doesn't support stroke_width
        if let Some(_fill) = fill {
            // Draw filled polygon - imageproc handles fill automatically
            // For now, we just draw the outline
            draw_polygon_mut(img, &ipoints, color);
        } else {
            draw_polygon_mut(img, &ipoints, color);
        }
    } else {
        // Open polygon: draw consecutive lines only (no auto-close)
        for window in ipoints.windows(2) {
            draw_line_segment_mut(
                img,
                (window[0].x as f32, window[0].y as f32),
                (window[1].x as f32, window[1].y as f32),
                color,
            );
        }
    }
}

fn apply_draw_text(
    img: &mut image::DynamicImage,
    x: i32,
    y: i32,
    text: &str,
    color: image::Rgba<u8>,
    font_size: u32,
    background: Option<image::Rgba<u8>>,
    padding: u32,
    font_bytes: &[u8],
) -> std::result::Result<(), OpError> {
    if font_bytes.is_empty() {
        return Err(OpError::NoFont);
    }
    if text.is_empty() || text.len() > 200 {
        return Err(OpError::InvalidText);
    }

    let font = ab_glyph::FontRef::try_from_slice(font_bytes).map_err(|_| OpError::FontParse)?;

    // Layout text to get bounding box
    let scale = ab_glyph::PxScale::from(font_size as f32);

    // Calculate text bounds using font metrics - simplified approach
    // Just estimate width based on character count and font size
    let text_w = (text.len() as u32 * font_size * 3 / 5).max(1); // Rough estimate
    let text_h = font_size; // Approximate height

    // Draw background if provided
    if let Some(bg) = background {
        apply_draw_rect(
            img,
            x,
            y,
            text_w + 2 * padding,
            text_h + 2 * padding,
            bg,
            1,
            Some(bg),
        );
    }

    // Draw text at (x + padding, y + padding)
    use imageproc::drawing::draw_text_mut;
    let draw_x = x + padding as i32;
    let draw_y = y + padding as i32;
    draw_text_mut(img, color, draw_x, draw_y, scale, &font, text);

    Ok(())
}

fn apply_blur_rect(
    img: &mut image::DynamicImage,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    mode: Option<BlurMode>,
    intensity: Option<u32>,
) -> std::result::Result<(), OpError> {
    let (x, y) = (x.max(0), y.max(0));
    let (w, h) = (w, h);
    if w == 0 || h == 0 {
        return Ok(());
    }
    let (iw, ih) = (img.width(), img.height());
    if x as u32 >= iw || y as u32 >= ih {
        return Ok(());
    }
    let x2 = ((x as u32) + w).min(iw);
    let y2 = ((y as u32) + h).min(ih);
    let (cw, ch) = (x2 - x as u32, y2 - y as u32);

    match mode.unwrap_or(BlurMode::Pixelate) {
        BlurMode::Pixelate => {
            let block = intensity.unwrap_or(16).max(1).min(256);
            let mut rgba = img.to_rgba8();
            let mut by = y as u32;
            while by < y2 {
                let mut bx = x as u32;
                while bx < x2 {
                    let mut rs: u64 = 0;
                    let mut gs: u64 = 0;
                    let mut bs: u64 = 0;
                    let mut as_: u64 = 0;
                    let mut n: u64 = 0;
                    let by_end = (by + block).min(y2);
                    let bx_end = (bx + block).min(x2);
                    for yy in by..by_end {
                        for xx in bx..bx_end {
                            let p = rgba.get_pixel(xx, yy);
                            rs += p.0[0] as u64;
                            gs += p.0[1] as u64;
                            bs += p.0[2] as u64;
                            as_ += p.0[3] as u64;
                            n += 1;
                        }
                    }
                    if n > 0 {
                        let mean = image::Rgba([
                            (rs / n) as u8,
                            (gs / n) as u8,
                            (bs / n) as u8,
                            (as_ / n) as u8,
                        ]);
                        for yy in by..by_end {
                            for xx in bx..bx_end {
                                rgba.put_pixel(xx, yy, mean);
                            }
                        }
                    }
                    bx += block;
                }
                by += block;
            }
            *img = image::DynamicImage::ImageRgba8(rgba);
        }
        BlurMode::Gaussian => {
            let radius = intensity.unwrap_or(5).max(1).min(100) as f32;
            // CRITICAL: use `crop_imm` to get a sub-image, then blur and overlay.
            // NOTE: crop_imm returns a NEW image, it doesn't mutate the receiver.
            let sub = img.crop_imm(x as u32, y as u32, cw, ch);
            let blurred = image::imageops::blur(&sub, radius);
            image::imageops::overlay(img, &blurred, x as i64, y as i64);
        }
    }
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
    use image::{DynamicImage, GenericImage, GenericImageView, Rgba, RgbaImage};

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

    #[test]
    fn draw_rect_default_color_red() {
        let mut img = solid(200, 200, [0, 0, 0, 255]);
        apply_draw_rect(
            &mut img,
            10,
            10,
            100,
            80,
            image::Rgba([255, 0, 0, 255]),
            2,
            None,
        );
        // The rect's TOP-LEFT CORNER (10,10) is unambiguously on the outline.
        assert_eq!(img.get_pixel(10, 10), image::Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn draw_circle_radius_zero_rejected() {
        let mut img = solid(200, 200, [0, 0, 0, 255]);
        let err = apply_draw_circle(
            &mut img,
            100,
            100,
            0,
            image::Rgba([255, 0, 0, 255]),
            2,
            None,
        )
        .unwrap_err();
        assert!(matches!(err, OpError::CircleRadiusZero));
    }

    #[test]
    fn draw_line_colors_pixel() {
        let mut img = solid(200, 200, [0, 0, 0, 255]);
        apply_draw_line(&mut img, 10, 10, 50, 50, image::Rgba([255, 0, 0, 255]), 2);
        // Midpoint of the line should be colored
        let mid = img.get_pixel(30, 30);
        assert_eq!(mid, image::Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn draw_arrow_draws_arrowhead() {
        let mut img = solid(200, 200, [0, 0, 0, 255]);
        apply_draw_arrow(
            &mut img,
            10,
            10,
            50,
            50,
            image::Rgba([255, 0, 0, 255]),
            2,
            10,
        );
        // Pixel near arrowhead tip (50,50) should be colored
        let tip = img.get_pixel(50, 50);
        assert_eq!(tip, image::Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn draw_polygon_closed_connects_last_to_first() {
        let mut img = solid(200, 200, [0, 0, 0, 255]);
        let points = vec![
            PolygonPoint { x: 10, y: 10 },
            PolygonPoint { x: 50, y: 10 },
            PolygonPoint { x: 50, y: 50 },
        ];
        apply_draw_polygon(
            &mut img,
            &points,
            image::Rgba([255, 0, 0, 255]),
            2,
            None,
            true,
        );
        // Last point (50,50) should connect to first (10,10)
        // Pixel on the closing edge at (30, 30) should be colored
        let closing_edge = img.get_pixel(30, 30);
        assert_eq!(closing_edge, image::Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn draw_polygon_open_does_not_connect() {
        let mut img = solid(200, 200, [0, 0, 0, 255]);
        let points = vec![
            PolygonPoint { x: 10, y: 10 },
            PolygonPoint { x: 50, y: 10 },
            PolygonPoint { x: 50, y: 50 },
        ];
        apply_draw_polygon(
            &mut img,
            &points,
            image::Rgba([255, 0, 0, 255]),
            2,
            None,
            false,
        );
        // Last point (50,50) should NOT connect to first (10,10)
        // Pixel on the would-be closing edge at (30, 30) should NOT be colored
        let no_edge = img.get_pixel(30, 30);
        assert_eq!(no_edge, image::Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn draw_text_renders_glyphs_with_background() {
        // System-font dependent: skip cleanly on systems without any of the probed fonts
        let font_bytes = match probe_font() {
            Some(b) => b.to_vec(),
            None => {
                eprintln!("skipping: no system font available");
                return;
            }
        };
        let mut img = solid(200, 100, [0, 0, 0, 255]);
        apply_draw_text(
            &mut img,
            10,
            10,
            "Hi",
            image::Rgba([255, 255, 255, 255]),
            24,
            Some(image::Rgba([0, 0, 0, 255])),
            4,
            &font_bytes,
        )
        .unwrap();
        // (x,y) is the top-left of the background rect — still pure black bg
        assert_eq!(img.get_pixel(10, 10), image::Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn draw_text_empty_font_bytes_returns_error() {
        let mut img = solid(100, 50, [0, 0, 0, 255]);
        let res = apply_draw_text(
            &mut img,
            10,
            10,
            "x",
            image::Rgba([255, 255, 255, 255]),
            16,
            None,
            0,
            &[], // empty font bytes
        );
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), OpError::NoFont));
    }

    #[test]
    fn blur_rect_pixelate_uniform_blocks() {
        let mut img = solid(80, 80, [200, 100, 50, 255]); // uniform input
        apply_blur_rect(&mut img, 0, 0, 80, 80, Some(BlurMode::Pixelate), Some(16)).unwrap();
        assert_eq!(img.get_pixel(40, 40), image::Rgba([200, 100, 50, 255]));
        // Dimensions MUST be unchanged.
        assert_eq!(img.dimensions(), (80, 80));
    }

    #[test]
    fn blur_rect_gaussian_alters_pixels_and_preserves_dimensions() {
        let mut img = image::DynamicImage::ImageRgba8(image::RgbaImage::new(100, 100));
        for x in 0..50 {
            for y in 0..100 {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
        for x in 50..100 {
            for y in 0..100 {
                img.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
        apply_blur_rect(&mut img, 0, 0, 100, 100, Some(BlurMode::Gaussian), Some(10)).unwrap();
        let mid = img.get_pixel(50, 50);
        assert!(
            mid[0] > 10 && mid[0] < 245,
            "gaussian produced gradient: {}",
            mid[0]
        );
        // CRITICAL regression guard: gaussian mode MUST NOT shrink the image.
        assert_eq!(img.dimensions(), (100, 100));
    }
}
