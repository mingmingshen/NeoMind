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
    /// File exceeded the resolve size cap (bytes). Guards against OOM when
    /// resolving image URLs for inline base64 / LLM / external delivery.
    TooLarge(u64),
}

impl std::fmt::Display for ImageStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPathComponent(s) => write!(f, "Invalid path component: {}", s),
            Self::UnknownFileType => write!(f, "Unknown file type from magic bytes"),
            Self::IoError(e) => write!(f, "I/O error: {}", e),
            Self::TooLarge(n) => write!(f, "Image file too large: {} bytes", n),
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
        return Err(ImageStorageError::InvalidPathComponent(format!(
            "contains path traversal: {}",
            component
        )));
    }

    if component.contains('/') || component.contains('\\') {
        return Err(ImageStorageError::InvalidPathComponent(format!(
            "contains path separator: {}",
            component
        )));
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
    // Tolerate the base64 variants real devices emit: MIME-folded whitespace
    // and missing/optional padding. NE301 cameras, for example, send unpadded
    // standard-alphabet base64 (len % 4 != 0, no `=`); the strict STANDARD
    // engine rejects it ("Incorrect padding"), and the URL_SAFE_NO_PAD
    // fallback uses the wrong alphabet. Strip whitespace + padding, then try
    // the standard alphabet (no-pad) before url-safe.
    let cleaned: Vec<u8> = raw_b64
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'=')
        .collect();
    let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(&cleaned)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&cleaned))
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

/// Detect MIME type from magic bytes. Falls back to `image/jpeg`.
pub fn detect_mime_from_bytes(bytes: &[u8]) -> &'static str {
    match detect_extension(bytes) {
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tiff" => "image/tiff",
        _ => "image/jpeg",
    }
}

/// Upper bound on a single image resolved via [`read_internal_image_url`].
/// Guards against OOM when a device/extension writes an oversized file and it
/// is then resolved for inline base64 (LLM context, webhook/MQTT push).
/// Callers that want a stricter per-path cap still can, post-read.
pub const MAX_INTERNAL_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

/// Read a NeoMind internal image URL (`/api/images/<dev>/<metric>/<ts>.<ext>`)
/// back into raw bytes + MIME. Read-side counterpart to `save_image_binary`;
/// single source of truth for path construction + security guards + MIME.
///
/// Guards applied (matching the public `GET /api/images/` handler):
/// - **traversal**: rejects `..`, absolute-root, and NUL in the URL;
/// - **symlink escape**: canonicalizes the resolved path and rejects anything
///   that does not stay under `<data_dir>/images/` (a symlinked entry under
///   `images/` pointing at e.g. `/etc/shadow` is refused);
/// - **size**: stats the file and rejects above [`MAX_INTERNAL_IMAGE_BYTES`]
///   *before* reading (prevents loading a multi-GB file into RAM);
/// - **magic bytes**: rejects content whose header is not a recognized image
///   (a `.bin` payload written by `save_image_binary` for unknown bytes is
///   not returned as a fake `image/jpeg`).
pub fn read_internal_image_url(url: &str, data_dir: &Path) -> Result<(Vec<u8>, &'static str)> {
    let url_path = url.strip_prefix("/api/images/").ok_or_else(|| {
        ImageStorageError::InvalidPathComponent(format!("not a /api/images/ URL: {url}"))
    })?;

    if url_path.contains('\0')
        || std::path::Path::new(url_path).components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(ImageStorageError::InvalidPathComponent(
            "path traversal is not allowed in /api/images/ URL".to_string(),
        ));
    }

    let images_dir = data_dir.join("images");
    let image_path = images_dir.join(url_path);

    // Symlink-escape guard: resolve real paths and require the file to remain
    // under images_dir. canonicalize also requires the file to exist, so a
    // missing file surfaces here as an IoError (same outcome as before).
    let canon_images = images_dir
        .canonicalize()
        .map_err(ImageStorageError::IoError)?;
    let canon_file = image_path
        .canonicalize()
        .map_err(ImageStorageError::IoError)?;
    if !canon_file.starts_with(&canon_images) {
        return Err(ImageStorageError::InvalidPathComponent(
            "image path escapes the images directory".to_string(),
        ));
    }

    // OOM guard: stat BEFORE reading so an oversized file never enters RAM.
    let len = std::fs::metadata(&canon_file)
        .map_err(ImageStorageError::IoError)?
        .len();
    if len > MAX_INTERNAL_IMAGE_BYTES {
        return Err(ImageStorageError::TooLarge(len));
    }

    let bytes = std::fs::read(&canon_file)?;

    // Magic-byte gate: refuse non-image content (e.g. a `.bin` written by
    // save_image_binary for bytes it couldn't identify) rather than returning
    // it mislabeled as image/jpeg.
    if detect_extension(&bytes) == "bin" {
        return Err(ImageStorageError::UnknownFileType);
    }
    let mime = detect_mime_from_bytes(&bytes);
    Ok((bytes, mime))
}

/// Resolve a `/api/images/` URL to a self-contained `data:<mime>;base64,...`
/// string for delivery to consumers that can't reach the local file route
/// (external webhooks, MQTT brokers, LLM context).
///
/// Single source of truth for the url→data-url transform: data-push and the
/// transform engine both call this instead of each inlining base64+format.
/// Applies the same symlink/size/magic guards as [`read_internal_image_url`].
/// Returns `None` if `url` is not a `/api/images/` URL, the file is missing /
/// unreadable / too large / non-image — callers fall back to the raw value.
pub fn resolve_internal_image_to_data_url(url: &str, data_dir: &Path) -> Option<String> {
    use base64::Engine as _;
    let (bytes, mime) = read_internal_image_url(url, data_dir).ok()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:{mime};base64,{b64}"))
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

        let url =
            save_image_binary("camera-001", "image", 1634567890000, &jpeg_bytes, data_dir).unwrap();

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

        let url = save_image_binary(
            "sensor-02",
            "screenshot",
            1634567890001,
            &png_bytes,
            data_dir,
        )
        .unwrap();

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

        let result = save_image_binary(
            "device-001",
            "metric/../etc",
            1634567890000,
            &jpeg_bytes,
            data_dir,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_save_image_binary_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];

        save_image_binary(
            "new-device",
            "new-metric",
            1634567890000,
            &jpeg_bytes,
            data_dir,
        )
        .unwrap();

        let device_dir = data_dir.join("images/new-device");
        let metric_dir = device_dir.join("new-metric");
        assert!(device_dir.exists());
        assert!(metric_dir.exists());
    }

    #[test]
    fn test_read_internal_image_url_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();
        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        let jpg_url = save_image_binary("cam-1", "image", 1000, &jpeg_bytes, data_dir).unwrap();
        let (bytes, mime) = read_internal_image_url(&jpg_url, data_dir).unwrap();
        assert_eq!(bytes, jpeg_bytes);
        assert_eq!(mime, "image/jpeg");

        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let png_url = save_image_binary("cam-1", "image", 1001, &png_bytes, data_dir).unwrap();
        let (png_out, png_mime) = read_internal_image_url(&png_url, data_dir).unwrap();
        assert_eq!(png_out, png_bytes);
        assert_eq!(png_mime, "image/png");
    }

    #[test]
    fn test_read_internal_image_url_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let res = read_internal_image_url("/api/images/no-such/dev/m/1.jpg", temp_dir.path());
        assert!(matches!(res, Err(ImageStorageError::IoError(_))));
    }

    #[test]
    fn test_read_internal_image_url_bad_prefix() {
        let temp_dir = TempDir::new().unwrap();
        let res = read_internal_image_url("http://example.com/x.jpg", temp_dir.path());
        assert!(matches!(
            res,
            Err(ImageStorageError::InvalidPathComponent(_))
        ));
    }

    #[test]
    fn test_read_internal_image_url_rejects_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();
        let secret = data_dir.join("secret.txt");
        std::fs::write(&secret, b"TOPSECRET").unwrap();
        for evil in [
            "/api/images/../../secret.txt",
            "/api/images/dev/../../../secret.txt",
            "/api/images/dev/m/../../../../secret.txt",
        ] {
            let res = read_internal_image_url(evil, data_dir);
            assert!(
                matches!(res, Err(ImageStorageError::InvalidPathComponent(_))),
                "{evil:?} should be rejected, got {res:?}"
            );
        }
    }

    #[test]
    fn test_read_internal_image_url_rejects_non_image() {
        // save_image_binary stores unrecognized magic as <ts>.bin; the reader
        // must refuse it (UnknownFileType) rather than return it as image/jpeg.
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();
        let garbage = b"NOT_AN_IMAGE_plain_text_payload_here";
        let bin_url = save_image_binary("dev", "m", 1, garbage, data_dir).unwrap();
        assert!(bin_url.ends_with(".bin"), "expected .bin for non-image");
        let res = read_internal_image_url(&bin_url, data_dir);
        assert!(
            matches!(res, Err(ImageStorageError::UnknownFileType)),
            "non-image (.bin) content must be rejected, got {res:?}"
        );
    }

    #[test]
    fn test_read_internal_image_url_rejects_symlink_escape() {
        // A symlink planted under images/ pointing OUTSIDE must be refused by
        // the canonicalize + starts_with guard — even if the target is a valid
        // image (so the only reason for rejection is the escape).
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();
        let jpeg_bytes = [0xFFu8, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        let secret = data_dir.join("secret.jpg");
        std::fs::write(&secret, jpeg_bytes).unwrap();
        let metric_dir = data_dir.join("images").join("dev").join("m");
        std::fs::create_dir_all(&metric_dir).unwrap();
        let link = metric_dir.join("evil.jpg");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&secret, &link).unwrap();
            let res = read_internal_image_url("/api/images/dev/m/evil.jpg", data_dir);
            assert!(
                matches!(res, Err(ImageStorageError::InvalidPathComponent(_))),
                "symlink escape must be rejected, got {res:?}"
            );
        }
        #[cfg(not(unix))]
        {
            let _ = (secret, link); // symlink test is unix-only
        }
    }

    #[test]
    fn test_save_image_binary_concurrent_safe() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];

        // Simulate concurrent writes to different devices/metrics
        let url1 =
            save_image_binary("device-001", "metric-a", 1000, &jpeg_bytes, &data_dir).unwrap();
        let url2 =
            save_image_binary("device-002", "metric-b", 1000, &jpeg_bytes, &data_dir).unwrap();
        let url3 =
            save_image_binary("device-001", "metric-c", 1001, &jpeg_bytes, &data_dir).unwrap();

        // All should succeed without conflicts
        assert_eq!(url1, "/api/images/device-001/metric-a/1000.jpg");
        assert_eq!(url2, "/api/images/device-002/metric-b/1000.jpg");
        assert_eq!(url3, "/api/images/device-001/metric-c/1001.jpg");

        // Verify all files exist
        assert!(data_dir
            .join("images/device-001/metric-a/1000.jpg")
            .exists());
        assert!(data_dir
            .join("images/device-002/metric-b/1000.jpg")
            .exists());
        assert!(data_dir
            .join("images/device-001/metric-c/1001.jpg")
            .exists());
    }

    /// Regression: devices (e.g. NE301 cameras) emit standard-alphabet base64
    /// WITHOUT padding (len % 4 != 0, no `=`). The strict STANDARD engine
    /// rejects it ("Incorrect padding") and the URL_SAFE_NO_PAD fallback used
    /// the wrong alphabet — so the image was never converted to a URL and got
    /// stored as raw base64. Must now decode.
    #[test]
    fn test_try_decode_base64_image_unpadded_standard() {
        // FF D8 FF E0 = JPEG SOI + APP0 marker start.
        let bytes = [0xFF, 0xD8, 0xFF, 0xE0];
        use base64::Engine as _;
        let padded = base64::engine::general_purpose::STANDARD.encode(bytes);
        let unpadded = padded.trim_end_matches('=');
        assert_ne!(
            unpadded.len() % 4,
            0,
            "test premise: unpadded length not a multiple of 4"
        );

        let data_url = format!("data:image/jpeg;base64,{}", unpadded);
        let decoded =
            try_decode_base64_image(&data_url).expect("unpadded standard base64 must decode");
        assert_eq!(decoded, bytes);
    }

    /// Whitespace inside base64 (MIME folding) must not break decoding.
    #[test]
    fn test_try_decode_base64_image_whitespace_tolerant() {
        let bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        use base64::Engine as _;
        let folded = base64::engine::general_purpose::STANDARD
            .encode(bytes)
            .chars()
            .collect::<Vec<_>>()
            .chunks(4)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        let data_url = format!("data:image/jpeg;base64,{}", folded);
        let decoded = try_decode_base64_image(&data_url).expect("folded base64 must decode");
        assert_eq!(decoded, bytes);
    }

    /// Real-world regression: an NE301 camera payload whose `image_data` is an
    /// unpadded standard-alphabet data URL (1263 chars, len % 4 == 3). Before
    /// the fix this returned None and the image was stored as base64.
    #[test]
    fn test_try_decode_base64_image_ne301_unpadded_payload() {
        let ne301 = "data:image/jpeg;base64,/9j/2wBDAA0JCgsKCA0LCgsODg0PEyAVExISEyccHhcgLikxMC4pLSwzOko+MzZGNywtQFdBRkxOUlNSMj5aYVpQYEpRUk//2wBDAQ4ODhMREyYVFSZPNS01T09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT0//wAARCALQBQADASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwDzmiiig6wooooAKKKKACiiigAooooAKKKKACiiigAooooAKKKKACiigAopKWkAUd6KKYGlCd0SkelSd+tQWrAxADtU2aDGS1FpKPTNHagQufWkopD6EmgBc80ZpKCaAFoyemaYzqByaia5RenJpDJ6CaptdEn5RUbTO3fFMfIy8ZFHVhUT3KDpzVIkk5JpKClAtNdk/dGKhaV2HJqOlpFKKQEk9TRRRQMSiloosAlLRRQAUUUUAFFFFAH//ZAAAA";
        // Sanity: this is the failing shape (unpadded, standard alphabet).
        let b64 = ne301.split(";base64,").nth(1).unwrap();
        assert_ne!(b64.len() % 4, 0, "premise: NE301 base64 is unpadded");
        let decoded = try_decode_base64_image(ne301).expect("NE301 payload must decode");
        assert_eq!(&decoded[..3], &[0xFF, 0xD8, 0xFF], "should be a JPEG");
    }
}
