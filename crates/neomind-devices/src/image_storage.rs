//! Image binary storage for device telemetry.
//!
//! Provides functionality to store image metrics as binary files
//! instead of base64-encoded strings in the telemetry database.
//! This reduces database size and improves query performance.
//!
//! ## Path Structure
//!
//! Images are stored at:
//! ```text
//! <data_dir>/images/<device_id>/<metric>/<timestamp>.<ext>
//! ```
//!
//! And served via:
//! ```text
//! GET /api/images/<device_id>/<metric>/<timestamp>.<ext>
//! ```

use std::path::Path;

/// Error types for image storage operations.
#[derive(Debug)]
pub enum ImageStorageError {
    /// Invalid device_id or metric name (path traversal attempt).
    InvalidPathComponent(String),
    /// Unable to detect file type from magic bytes.
    UnknownFileType,
    /// I/O error during file write.
    IoError(std::io::Error),
}

impl std::fmt::Display for ImageStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPathComponent(s) => write!(f, "Invalid path component: {}", s),
            Self::UnknownFileType => write!(f, "Unknown file type from magic bytes"),
            Self::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for ImageStorageError {}

impl From<std::io::Error> for ImageStorageError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

/// Result type for image storage operations.
pub type Result<T> = std::result::Result<T, ImageStorageError>;

/// Detect file extension from magic bytes (first 8 bytes of file data).
///
/// Returns the lowercase file extension without a dot, or "bin" if unknown.
///
/// ## Supported Formats
///
/// | Format | Magic Bytes | Extension |
/// |--------|-------------|-----------|
/// | JPEG   | `FF D8 FF` | jpg |
/// | PNG    | `89 50 4E 47` | png |
/// | GIF    | `47 49 46 38` | gif |
/// | WebP   | `52 49 46 46 ... 57 45 42 50` | webp |
///
/// # Examples
///
/// ```rust
/// use neomind_devices::image_storage::detect_extension;
///
/// let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
/// assert_eq!(detect_extension(&jpeg_bytes), "jpg");
/// ```
pub fn detect_extension(bytes: &[u8]) -> &'static str {
    if bytes.len() < 4 {
        return "bin";
    }

    // JPEG: FF D8 FF
    if bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return "jpg";
    }

    // PNG: 89 50 4E 47
    if bytes[0] == 0x89 && bytes[1] == 0x50 && bytes[2] == 0x4E && bytes[3] == 0x47 {
        return "png";
    }

    // GIF: 47 49 46 38
    if bytes[0] == 0x47 && bytes[1] == 0x49 && bytes[2] == 0x46 && bytes[3] == 0x38 {
        return "gif";
    }

    // WebP: RIFF....WEBP (needs at least 12 bytes: "RIFF" + 4 bytes size + "WEBP")
    if bytes.len() >= 12
        && bytes[0] == 0x52
        && bytes[1] == 0x49
        && bytes[2] == 0x46
        && bytes[3] == 0x46
        && bytes[8] == 0x57
        && bytes[9] == 0x45
        && bytes[10] == 0x42
        && bytes[11] == 0x50
    {
        return "webp";
    }

    // BMP: 42 4D (BM)
    if bytes.len() >= 2 && bytes[0] == 0x42 && bytes[1] == 0x4D {
        return "bmp";
    }

    // TIFF: 49 49 2A 00 (little-endian) or 4D 4D 00 2A (big-endian)
    if bytes.len() >= 4 {
        if bytes[0] == 0x49 && bytes[1] == 0x49 && bytes[2] == 0x2A && bytes[3] == 0x00 {
            return "tiff";
        }
        if bytes[0] == 0x4D && bytes[1] == 0x4D && bytes[2] == 0x00 && bytes[3] == 0x2A {
            return "tiff";
        }
    }

    "bin"
}

/// Validate and sanitize a path component (device_id or metric name).
///
/// Rejects:
/// - Empty strings
/// - Strings containing `..` (parent directory reference)
/// - Strings containing `/` or `\` (path separators)
/// - Strings longer than 255 characters (filesystem limit)
///
/// # Returns
///
/// Returns the sanitized string if safe, or an error if rejected.
///
/// # Examples
///
/// ```rust
/// use neomind_devices::image_storage::validate_path_component;
///
/// assert!(validate_path_component("device-001").is_ok());
/// assert!(validate_path_component("temperature").is_ok());
/// assert!(validate_path_component("../etc").is_err());
/// assert!(validate_path_component("device/001").is_err());
/// ```
pub fn validate_path_component(component: &str) -> Result<String> {
    if component.is_empty() {
        return Err(ImageStorageError::InvalidPathComponent(
            "empty string".to_string(),
        ));
    }

    if component.contains("..") {
        return Err(ImageStorageError::InvalidPathComponent(
            format!("contains path traversal: {}", component),
        ));
    }

    if component.contains('/') || component.contains('\\') {
        return Err(ImageStorageError::InvalidPathComponent(
            format!("contains path separator: {}", component),
        ));
    }

    if component.contains('\0') {
        return Err(ImageStorageError::InvalidPathComponent(
            "contains null byte".to_string(),
        ));
    }

    if component.len() > 255 {
        return Err(ImageStorageError::InvalidPathComponent(
            "too long (>255 chars)".to_string(),
        ));
    }

    Ok(component.to_string())
}

/// Save image binary data to disk and return the URL path.
///
/// This function stores image metrics as binary files instead of
/// base64-encoded strings in the telemetry database, reducing storage
/// size and improving query performance.
///
/// # Arguments
///
/// * `device_id` - Device identifier (sanitized to prevent path traversal)
/// * `metric` - Metric name (sanitized to prevent path traversal)
/// * `timestamp` - Unix timestamp in milliseconds (used as filename)
/// * `bytes` - Image binary data
/// * `data_dir` - Root data directory path
///
/// # Returns
///
/// Returns the URL path that can be used to serve the image:
/// ```text
/// /api/images/<device_id>/<metric>/<timestamp>.<ext>
/// ```
///
/// # Path Structure
///
/// Files are stored at:
/// ```text
/// <data_dir>/images/<device_id>/<metric>/<timestamp>.<ext>
/// ```
///
/// # Example
///
/// ```rust
/// use neomind_devices::image_storage::save_image_binary;
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let device_id = "camera-001";
/// let metric = "image";
/// let timestamp = 1634567890000i64;
/// let bytes = b"\xFF\xD8\xFF\xE0\x00\x10\x4A\x46..."; // JPEG bytes
/// let data_dir = PathBuf::from("/data");
///
/// let url = save_image_binary(device_id, metric, timestamp, &bytes, &data_dir)?;
/// assert_eq!(url, "/api/images/camera-001/image/1634567890000.jpg");
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns `ImageStorageError` if:
/// - `device_id` or `metric` contain invalid characters (path traversal)
/// - File I/O fails (disk full, permissions, etc.)
/// Try to decode a string as base64-encoded image data.
/// Handles data URLs (`data:image/png;base64,...`) and raw base64.
/// Returns decoded bytes if it looks like an image, None otherwise.
pub fn try_decode_base64_image(s: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;
    let raw_b64 = if s.starts_with("data:image/") {
        s.split(";base64,").nth(1)?
    } else if s.len() > 100 {
        s
    } else {
        return None;
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw_b64)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(raw_b64))
        .ok()?;
    if detect_extension(&decoded) != "bin" {
        Some(decoded)
    } else {
        None
    }
}

pub fn save_image_binary(
    device_id: &str,
    metric: &str,
    timestamp: i64,
    bytes: &[u8],
    data_dir: &Path,
) -> Result<String> {
    // 1. Validate and sanitize path components
    let safe_device_id = validate_path_component(device_id)?;
    let safe_metric = validate_path_component(metric)?;

    // 2. Detect file extension from magic bytes
    let ext = detect_extension(bytes);

    // 3. Build file path: <data_dir>/images/<device_id>/<metric>/<timestamp>.<ext>
    let images_dir = data_dir.join("images");
    let device_dir = images_dir.join(&safe_device_id);
    let metric_dir = device_dir.join(&safe_metric);
    let filename = format!("{}.{}", timestamp, ext);
    let file_path = metric_dir.join(&filename);

    // 4. Create parent directories if they don't exist
    std::fs::create_dir_all(&metric_dir)?;

    // 5. Write file atomically (write to temp file, then rename)
    let temp_path = metric_dir.join(format!(".tmp.{}", timestamp));
    std::fs::write(&temp_path, bytes)?;
    std::fs::rename(&temp_path, &file_path)?;

    // 6. Return URL path
    let url_path = format!(
        "/api/images/{}/{}/{}.{}",
        safe_device_id, safe_metric, timestamp, ext
    );

    Ok(url_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_extension_jpeg() {
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(detect_extension(&jpeg), "jpg");
    }

    #[test]
    fn test_detect_extension_png() {
        let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_extension(&png), "png");
    }

    #[test]
    fn test_detect_extension_gif() {
        let gif = [0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00];
        assert_eq!(detect_extension(&gif), "gif");
    }

    #[test]
    fn test_detect_extension_webp() {
        let mut webp = [0u8; 16];
        webp[0] = 0x52; // R
        webp[1] = 0x49; // I
        webp[2] = 0x46; // F
        webp[3] = 0x46; // F
        webp[8] = 0x57; // W
        webp[9] = 0x45; // E
        webp[10] = 0x42; // B
        webp[11] = 0x50; // P
        assert_eq!(detect_extension(&webp), "webp");
    }

    #[test]
    fn test_detect_extension_bmp() {
        let bmp = [0x42, 0x4D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_extension(&bmp), "bmp");
    }

    #[test]
    fn test_detect_extension_unknown() {
        let unknown = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert_eq!(detect_extension(&unknown), "bin");
    }

    #[test]
    fn test_detect_extension_too_short() {
        let short = [0xFF, 0xD8];
        assert_eq!(detect_extension(&short), "bin");
    }

    #[test]
    fn test_validate_path_component_valid() {
        assert!(validate_path_component("device-001").is_ok());
        assert!(validate_path_component("temperature").is_ok());
        assert!(validate_path_component("image_metric").is_ok());
        assert!(validate_path_component("cam-01").is_ok());
    }

    #[test]
    fn test_validate_path_component_empty() {
        assert!(validate_path_component("").is_err());
    }

    #[test]
    fn test_validate_path_component_traversal() {
        assert!(validate_path_component("../etc").is_err());
        assert!(validate_path_component("..").is_err());
        assert!(validate_path_component("device/../etc").is_err());
    }

    #[test]
    fn test_validate_path_component_separators() {
        assert!(validate_path_component("device/001").is_err());
        assert!(validate_path_component("device\\001").is_err());
    }

    #[test]
    fn test_validate_path_component_null() {
        assert!(validate_path_component("device\x00").is_err());
    }

    #[test]
    fn test_validate_path_component_too_long() {
        let long = "a".repeat(256);
        assert!(validate_path_component(&long).is_err());
    }

    #[test]
    fn test_save_image_binary_jpeg() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        // JPEG header + minimal JPEG data
        let jpeg_bytes = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
        ];

        let url = save_image_binary("camera-001", "image", 1634567890000, &jpeg_bytes, data_dir).unwrap();

        assert_eq!(url, "/api/images/camera-001/image/1634567890000.jpg");

        // Verify file exists
        let file_path = data_dir.join("images/camera-001/image/1634567890000.jpg");
        assert!(file_path.exists());

        // Verify file contents
        let saved_bytes = std::fs::read(&file_path).unwrap();
        assert_eq!(saved_bytes, jpeg_bytes);
    }

    #[test]
    fn test_save_image_binary_png() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        // PNG header
        let png_bytes = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        ];

        let url = save_image_binary("sensor-02", "screenshot", 1634567890001, &png_bytes, data_dir).unwrap();

        assert_eq!(url, "/api/images/sensor-02/screenshot/1634567890001.png");

        let file_path = data_dir.join("images/sensor-02/screenshot/1634567890001.png");
        assert!(file_path.exists());
    }

    #[test]
    fn test_save_image_binary_invalid_device_id() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];

        let result = save_image_binary("../etc", "image", 1634567890000, &jpeg_bytes, data_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_image_binary_invalid_metric() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];

        let result = save_image_binary("device-001", "metric/../etc", 1634567890000, &jpeg_bytes, data_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_image_binary_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];

        save_image_binary("new-device", "new-metric", 1634567890000, &jpeg_bytes, data_dir).unwrap();

        let device_dir = data_dir.join("images/new-device");
        let metric_dir = device_dir.join("new-metric");
        assert!(device_dir.exists());
        assert!(metric_dir.exists());
    }

    #[test]
    fn test_save_image_binary_concurrent_safe() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];

        // Simulate concurrent writes to different devices/metrics
        let url1 = save_image_binary("device-001", "metric-a", 1000, &jpeg_bytes, &data_dir).unwrap();
        let url2 = save_image_binary("device-002", "metric-b", 1000, &jpeg_bytes, &data_dir).unwrap();
        let url3 = save_image_binary("device-001", "metric-c", 1001, &jpeg_bytes, &data_dir).unwrap();

        // All should succeed without conflicts
        assert_eq!(url1, "/api/images/device-001/metric-a/1000.jpg");
        assert_eq!(url2, "/api/images/device-002/metric-b/1000.jpg");
        assert_eq!(url3, "/api/images/device-001/metric-c/1001.jpg");

        // Verify all files exist
        assert!(data_dir.join("images/device-001/metric-a/1000.jpg").exists());
        assert!(data_dir.join("images/device-002/metric-b/1000.jpg").exists());
        assert!(data_dir.join("images/device-001/metric-c/1001.jpg").exists());
    }
}
