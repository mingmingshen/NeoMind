//! Vision tool for multi-modal image analysis using VLM backends.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use neomind_core::llm::backend::{LlmInput, LlmRuntime};
use neomind_core::message::{Content, ContentPart, Message, MessageRole};
use neomind_core::tools::ToolCategory;
use serde_json::Value;

use super::error::{Result, ToolError};
use super::tool::{object_schema, string_property, Tool, ToolOutput};
use crate::image_utils::{detect_mime_from_bytes, is_private_host, looks_like_raw_base64};
use crate::llm_backends::LlmBackendInstanceManager;

/// Maximum image size in bytes (10 MB). VLMs typically downsample to
/// ~448-672px anyway, so 10 MB is more than sufficient.
const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum base64 string length (~13.3 MB base64 for 10 MB raw).
const MAX_BASE64_LEN: usize = MAX_IMAGE_SIZE * 4 / 3 + 4;

/// Allowed image file extensions (lowercase). Only binary raster formats
/// that pass `detect_mime_from_bytes()` are included. SVG is excluded
/// because it cannot pass the magic-bytes validation for local files.
const ALLOWED_IMAGE_EXTENSIONS: &[&str] =
    &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif"];

/// Allowed MIME subtypes in data URLs (the part after "image/").
const ALLOWED_DATA_MIME_SUBTYPES: &[&str] = &["png", "jpeg", "jpg", "gif", "webp", "bmp", "tiff"];

/// Configuration for the vision tool.
#[derive(Debug, Clone)]
pub struct VisionConfig {
    /// Whether the vision tool is enabled (controlled by auto-detection).
    pub enabled: bool,
    /// Optional: explicit VLM backend ID to use.
    pub vlm_backend_id: Option<String>,
    /// Maximum tokens for VLM response (default 1024).
    pub max_tokens: u32,
    /// Timeout for HTTP image fetch in seconds (default 10).
    pub capture_timeout_secs: u64,
    /// Maximum image dimension (width/height) in pixels for VLM dispatch (default 1280).
    /// Images fetched via HTTP or read from disk are downscaled to fit within this
    /// box before base64 encoding. VLMs internally downsample to ~448-672px, so
    /// 1280 preserves full perceivable detail while cutting bandwidth 3-5x on
    /// cloud backends. Set to 0 to disable resizing (send original).
    pub max_image_dim: u32,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vlm_backend_id: None,
            max_tokens: 1024,
            capture_timeout_secs: crate::toolkit::timeouts::vision_capture().as_secs(),
            max_image_dim: 1280,
        }
    }
}

/// Vision tool — analyzes images using a vision-language model.
///
/// Accepts base64 data, data URLs, local file paths, or HTTP/HTTPS URLs.
/// Returns natural language analysis of the image content.
pub struct VisionTool {
    llm_manager: Arc<LlmBackendInstanceManager>,
    config: VisionConfig,
    http_client: reqwest::Client,
}

impl VisionTool {
    /// Create a new vision tool.
    pub fn new(config: VisionConfig, llm_manager: Arc<LlmBackendInstanceManager>) -> Self {
        // Custom redirect policy: validate each redirect target against SSRF rules,
        // matching the pattern used by WebFetchTool.
        let redirect_policy = reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url().clone();
            if let Err(e) = validate_url(&url) {
                tracing::warn!(url = %url, error = %e, "Vision: redirect blocked by SSRF check");
                return attempt.error(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("Redirect to '{}' blocked: {}", url, e),
                ));
            }
            if attempt.previous().len() >= 5 {
                return attempt.error(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Too many redirects",
                ));
            }
            attempt.follow()
        });

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.capture_timeout_secs))
            .redirect(redirect_policy)
            .no_proxy()
            .build()
            .expect("Failed to build reqwest client for vision tool");

        Self {
            llm_manager,
            config,
            http_client,
        }
    }

    /// Resolve a VLM runtime for image analysis.
    ///
    /// Priority order:
    /// 1. Explicit `vlm_backend_id` in config
    /// 2. Current active backend (if multimodal-capable)
    /// 3. First multimodal-capable instance found
    async fn resolve_vlm_runtime(&self) -> Result<Arc<dyn LlmRuntime>> {
        // 1. Try explicit backend ID
        if let Some(ref id) = self.config.vlm_backend_id {
            return self
                .llm_manager
                .get_runtime(id)
                .await
                .map_err(|e| ToolError::Execution(format!("VLM backend '{}' error: {}", id, e)));
        }

        // 2. Try active backend (the one the user is currently chatting with)
        if let Some(active) = self.llm_manager.get_active_instance() {
            if active.capabilities.supports_multimodal {
                let id = active.id.clone();
                tracing::debug!(backend_id = %id, "Using active backend for vision (multimodal-capable)");
                return self.llm_manager.get_runtime(&id).await.map_err(|e| {
                    ToolError::Execution(format!(
                        "VLM backend '{}' unavailable: {}. Check if the model service is running.",
                        id, e
                    ))
                });
            }
        }

        // 3. Fallback: first multimodal-capable instance
        let instances = self.llm_manager.list_instances();
        let vlm_instance = instances
            .iter()
            .find(|inst| inst.capabilities.supports_multimodal);

        match vlm_instance {
            Some(inst) => {
                let id = inst.id.clone();
                tracing::debug!(backend_id = %id, "Using fallback VLM backend for vision");
                self.llm_manager.get_runtime(&id).await.map_err(|e| {
                    ToolError::Execution(format!(
                        "VLM backend '{}' unavailable: {}. Check if the model service is running.",
                        id, e
                    ))
                })
            }
            None => Err(ToolError::Execution(
                "No vision model configured. Install a VLM (e.g., qwen2.5-vl) via Ollama.".into(),
            )),
        }
    }

    /// Resolve image input to (base64_data, mime_type).
    async fn resolve_image(&self, image: &str) -> Result<(String, String)> {
        // 1. Data URL: data:image/png;base64,... (case-insensitive prefix)
        //    Also handles incomplete data URLs missing the "data:" prefix
        //    (e.g. "image/jpeg;base64,/9j/...") which some callers produce.
        let image_lower_prefix = image
            .chars()
            .take(11)
            .collect::<String>()
            .to_ascii_lowercase();
        let is_data_image_url = image_lower_prefix == "data:image/";
        let is_incomplete_data_url = !is_data_image_url
            && image.contains(";base64,")
            && image_lower_prefix.starts_with("image/");

        if is_data_image_url || is_incomplete_data_url {
            let rest = if is_data_image_url {
                image.get(11..) // skip "data:image/"
            } else {
                image.get(6..) // skip "image/"
            };
            if let Some(rest) = rest {
                if let Some((mime_suffix, b64)) = rest.split_once(";base64,") {
                    if b64.is_empty() {
                        return Err(ToolError::InvalidArguments(
                            "Data URL contains empty base64 data".into(),
                        ));
                    }
                    if b64.len() > MAX_BASE64_LEN {
                        return Err(ToolError::InvalidArguments(format!(
                            "Data URL base64 data too large ({} chars, max {} chars)",
                            b64.len(),
                            MAX_BASE64_LEN
                        )));
                    }
                    let subtype = mime_suffix.to_lowercase();
                    if !ALLOWED_DATA_MIME_SUBTYPES.contains(&subtype.as_str()) {
                        return Err(ToolError::InvalidArguments(format!(
                            "Unsupported image type '{}' in data URL. Allowed: {}",
                            subtype,
                            ALLOWED_DATA_MIME_SUBTYPES.join(", ")
                        )));
                    }
                    let mime = format!("image/{}", subtype);
                    return Ok((b64.to_string(), mime));
                }
            }
        }

        // 2. HTTP/HTTPS URL
        if image.starts_with("http://") || image.starts_with("https://") {
            let bytes = self.fetch_http_image(image).await?;
            let mime = detect_mime_from_bytes(&bytes)
                .unwrap_or("image/jpeg")
                .to_string();
            return Ok(self.process_image_bytes(bytes, &mime).await);
        }

        // 3. Block non-http URL schemes with a clear error
        if image.contains("://") {
            return Err(ToolError::InvalidArguments(format!(
                "Unsupported URL scheme in '{}'. Only http:// and https:// are supported.",
                image.split("://").next().unwrap_or("")
            )));
        }

        // 4. Raw base64 detection (MUST come before local file path check).
        //
        // Why: a stripped JPEG base64 starts with "/9j/" and a PNG base64 starts
        // with "iVBORw0KGgo". The "/" prefix would otherwise be misclassified
        // as a local file path, producing "Cannot resolve path '/9j/...'" errors
        // when the LLM passes raw base64 (no data URL wrapper) into the tool.
        //
        // Heuristic: looks_like_raw_base64 returns true when the string is
        // pure base64 alphabet AND either carries an image magic prefix or is
        // long enough to plausibly be image data.
        if looks_like_raw_base64(image) {
            if image.is_empty() {
                return Err(ToolError::InvalidArguments("Image data is empty".into()));
            }
            if image.len() > MAX_BASE64_LEN {
                return Err(ToolError::InvalidArguments(format!(
                    "Base64 data too large ({} chars, max {} chars)",
                    image.len(),
                    MAX_BASE64_LEN
                )));
            }
            let mime = infer_mime_from_base64_prefix(image)
                .unwrap_or("image/jpeg")
                .to_string();
            tracing::debug!(
                len = image.len(),
                inferred_mime = %mime,
                "Treating image argument as raw base64"
            );
            return Ok((image.to_string(), mime));
        }

        // 5. Local file path
        if image.starts_with('/') || image.starts_with("./") {
            let bytes = self.read_local_image(image).await?;
            let mime = detect_mime_from_bytes(&bytes)
                .unwrap_or("image/jpeg")
                .to_string();
            return Ok(self.process_image_bytes(bytes, &mime).await);
        }

        // 6. Fallback: treat as raw base64
        if image.is_empty() {
            return Err(ToolError::InvalidArguments("Image data is empty".into()));
        }
        if image.len() > MAX_BASE64_LEN {
            return Err(ToolError::InvalidArguments(format!(
                "Base64 data too large ({} chars, max {} chars)",
                image.len(),
                MAX_BASE64_LEN
            )));
        }
        Ok((image.to_string(), "image/jpeg".to_string()))
    }

    /// Read a local image file with security checks.
    async fn read_local_image(&self, path_str: &str) -> Result<Vec<u8>> {
        let path = Path::new(path_str);

        // Block path traversal
        for component in path.components() {
            if component == std::path::Component::ParentDir {
                return Err(ToolError::PermissionDenied(
                    "Path traversal (..) is not allowed".into(),
                ));
            }
        }

        // Canonicalize to resolve symlinks, then re-check the real path
        let canonical = tokio::fs::canonicalize(path).await.map_err(|e| {
            ToolError::Execution(format!("Cannot resolve path '{}': {}", path_str, e))
        })?;
        let canonical_str = canonical.to_string_lossy();

        // Block sensitive system paths (checked against canonical path)
        let canonical_lower = canonical_str.to_lowercase();
        let blocked_prefixes = [
            "/etc/", "/proc/", "/sys/", "/dev/", "/run/", "/boot/", "/root/", "/var/", "/tmp/",
            "/opt/", "/srv/",
        ];
        if blocked_prefixes
            .iter()
            .any(|prefix| canonical_lower.starts_with(prefix))
        {
            return Err(ToolError::PermissionDenied(
                "Access to system path is not allowed".to_string(),
            ));
        }

        // Block hidden files (dotfiles) in home directories
        if let Some(name) = canonical.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.')
                && (canonical_lower.starts_with("/home") || canonical_lower.starts_with("/users"))
            {
                return Err(ToolError::PermissionDenied(
                    "Access to hidden file is not allowed".to_string(),
                ));
            }
        }

        // Validate extension looks like an image
        if let Some(ext) = canonical.extension().and_then(|e| e.to_str()) {
            if !ALLOWED_IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                return Err(ToolError::PermissionDenied(format!(
                    "File extension '.{}' is not an image format. Allowed: {}",
                    ext,
                    ALLOWED_IMAGE_EXTENSIONS.join(", ")
                )));
            }
        } else {
            // Files without extension: reject (must have a known image extension)
            return Err(ToolError::PermissionDenied(
                "File must have an image extension (e.g., .jpg, .png)".into(),
            ));
        }

        // Check file size before reading
        let metadata = tokio::fs::metadata(&canonical).await.map_err(|e| {
            ToolError::Execution(format!("Failed to stat file '{}': {}", path_str, e))
        })?;
        if metadata.len() as usize > MAX_IMAGE_SIZE {
            return Err(ToolError::Execution(format!(
                "File too large: {} bytes (max {} bytes)",
                metadata.len(),
                MAX_IMAGE_SIZE
            )));
        }

        let bytes = tokio::fs::read(&canonical).await.map_err(|e| {
            ToolError::Execution(format!("Failed to read file '{}': {}", path_str, e))
        })?;

        // Validate the file looks like an image by checking magic bytes
        if detect_mime_from_bytes(&bytes).is_none() {
            return Err(ToolError::Execution(format!(
                "File '{}' does not appear to be a valid image (unrecognized header)",
                path_str
            )));
        }

        Ok(bytes)
    }

    /// Fetch image bytes from an HTTP/HTTPS URL with SSRF protection.
    async fn fetch_http_image(&self, url: &str) -> Result<Vec<u8>> {
        // SSRF: validate URL before fetching (redirect hops are validated by custom policy)
        validate_url(
            &reqwest::Url::parse(url)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid URL: {}", e)))?,
        )?;

        let response = self
            .http_client
            .get(url)
            .header("User-Agent", "NeoMind-VisionTool/1.0")
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ToolError::Timeout
                } else {
                    ToolError::Execution(format!("HTTP fetch failed: {}", e))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(ToolError::Execution(format!(
                "HTTP {} fetching image",
                status.as_u16()
            )));
        }

        // Check Content-Length before downloading body
        if let Some(content_length) = response.headers().get("content-length") {
            if let Ok(len_str) = content_length.to_str() {
                if let Ok(len) = len_str.parse::<usize>() {
                    if len > MAX_IMAGE_SIZE {
                        return Err(ToolError::Execution(format!(
                            "Image too large (Content-Length: {} bytes, max: {} bytes)",
                            len, MAX_IMAGE_SIZE
                        )));
                    }
                }
            }
        }

        // Validate Content-Type looks like an image
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();
        let media_type = content_type.split(';').next().unwrap_or("").trim();
        if !media_type.starts_with("image/") && !media_type.is_empty() {
            return Err(ToolError::Execution(format!(
                "URL returned non-image content type: {}. Only image/* is supported.",
                content_type
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ToolError::Execution(format!("HTTP read failed: {}", e)))?;

        if bytes.len() > MAX_IMAGE_SIZE {
            return Err(ToolError::Execution(format!(
                "Image too large: {} bytes (max: {} bytes)",
                bytes.len(),
                MAX_IMAGE_SIZE
            )));
        }

        Ok(bytes.to_vec())
    }

    /// Process raw image bytes: optionally resize to `max_image_dim`, then
    /// base64-encode. The decode/resize/encode pipeline is CPU-intensive
    /// (Lanczos3 on multi-MB images can take 100-500ms), so it runs on
    /// `spawn_blocking` to avoid starving the tokio runtime.
    ///
    /// Fail-open: resize errors fall back to the original bytes. The only
    /// hard error path is `spawn_blocking` itself failing (JoinError), which
    /// is astronomically rare (would require a panic inside the image crate).
    async fn process_image_bytes(&self, bytes: Vec<u8>, detected_mime: &str) -> (String, String) {
        let max_dim = self.config.max_image_dim;
        let mime_owned = detected_mime.to_string();
        match tokio::task::spawn_blocking(move || {
            let (final_bytes, mime) = resize_image_if_needed(bytes, &mime_owned, max_dim);
            let b64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &final_bytes);
            (b64, mime)
        })
        .await
        {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(error = %e, "spawn_blocking panicked during image processing; image lost");
                // Bytes were moved into the closure and are unrecoverable on panic.
                // Return an explicit empty result so the caller surfaces a VLM error
                // rather than silently sending corrupt data.
                (String::new(), detected_mime.to_string())
            }
        }
    }

    /// Run VLM analysis on the resolved image.
    async fn analyze(&self, data: &str, mime: &str, prompt: &str) -> Result<String> {
        let runtime = self.resolve_vlm_runtime().await?;

        let msg = Message::new(
            MessageRole::User,
            Content::Parts(vec![
                ContentPart::text(prompt),
                ContentPart::image_base64(data, mime),
            ]),
        );

        let input = LlmInput {
            messages: vec![msg],
            params: neomind_core::llm::GenerationParams {
                max_tokens: Some(self.config.max_tokens as usize),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        let output = runtime
            .generate(input)
            .await
            .map_err(|e| ToolError::Execution(format!("VLM inference failed: {}", e)))?;

        if output.text.trim().is_empty() {
            return Err(ToolError::Execution(
                "VLM returned empty analysis. The image may not be processable.".into(),
            ));
        }

        Ok(output.text)
    }
}

#[async_trait]
impl Tool for VisionTool {
    fn name(&self) -> &str {
        "vision"
    }

    fn description(&self) -> &str {
        r#"Analyze images from URLs, files, or extension outputs using a vision-language model.

DO NOT use this tool for images you can already see — analyze those yourself directly. This includes:
- Images uploaded by the user in chat
- Images embedded in the current message (e.g., from bound data sources) — these are already visible to you

Only use this tool when you need to analyze images from OTHER sources:
- HTTP/HTTPS image URLs (e.g., camera snapshots, web images) — fetches automatically, private/local URLs blocked
- /path/to/file.jpg — local image file on disk (must have an image extension)
- data:image/...;base64,... — base64 data URL from extension outputs
- raw base64 string — decoded as JPEG by default"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "image": string_property("Image source: base64 data URL, raw base64, local image file path, or public HTTP/HTTPS URL"),
                "prompt": string_property("Analysis instructions for the image. Match the user's language and detail level from the conversation.")
            }),
            vec!["image".to_string(), "prompt".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let image = args["image"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("image is required".into()))?;
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("prompt is required".into()))?;

        if image.trim().is_empty() {
            return Err(ToolError::InvalidArguments("image cannot be empty".into()));
        }
        if prompt.trim().is_empty() {
            return Err(ToolError::InvalidArguments("prompt cannot be empty".into()));
        }

        tracing::info!(image_len = image.len(), "Vision tool: resolving image");

        let (data, mime) = self.resolve_image(image).await?;
        let analysis = self.analyze(&data, &mime, prompt).await?;

        Ok(ToolOutput::success(serde_json::json!({
            "analysis": analysis,
            "image_type": mime,
        })))
    }
}

// ---------------------------------------------------------------------------
// Shared helpers (SSRF protection + MIME detection)
// ---------------------------------------------------------------------------

/// Validate a reqwest::Url against SSRF rules.
fn validate_url(url: &reqwest::Url) -> Result<()> {
    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(ToolError::PermissionDenied(
                "Only http:// and https:// URLs are allowed".into(),
            ))
        }
    }

    let host = url
        .host_str()
        .ok_or_else(|| ToolError::InvalidArguments("URL has no host".into()))?;

    if is_private_host(host) {
        return Err(ToolError::PermissionDenied(format!(
            "Access to private address '{}' is not allowed",
            host
        )));
    }

    Ok(())
}

/// Resize image bytes if either dimension exceeds `max_dim`.
///
/// - If `max_dim == 0` or the image is already within bounds, returns the
///   original bytes with the detected MIME unchanged.
/// - Otherwise resizes with Lanczos3 and re-encodes as JPEG (mime → `image/jpeg`).
/// - **Fail-open**: any decode/encode error logs a warning and returns the
///   original bytes so the VLM still receives the image. This covers
///   formats the `image` crate can't decode (e.g. TIFF/BMP/HEIC when the
///   feature isn't enabled) without blocking the analysis.
fn resize_image_if_needed(bytes: Vec<u8>, detected_mime: &str, max_dim: u32) -> (Vec<u8>, String) {
    if max_dim == 0 || bytes.is_empty() {
        return (bytes, detected_mime.to_string());
    }
    let img = match image::load_from_memory(&bytes) {
        Ok(img) => img,
        Err(e) => {
            tracing::debug!(
                error = %e, mime = %detected_mime, bytes = bytes.len(),
                "Image decode failed for resize; sending original bytes"
            );
            return (bytes, detected_mime.to_string());
        }
    };
    let (w, h) = (img.width(), img.height());
    if w <= max_dim && h <= max_dim {
        return (bytes, detected_mime.to_string());
    }
    let resized = img.resize(max_dim, max_dim, image::imageops::FilterType::Lanczos3);
    let mut buf = std::io::Cursor::new(Vec::new());
    match resized.write_to(&mut buf, image::ImageFormat::Jpeg) {
        Ok(()) => {
            let new_bytes = buf.into_inner();
            tracing::info!(
                orig_w = w,
                orig_h = h,
                new_w = resized.width(),
                new_h = resized.height(),
                orig_bytes = bytes.len(),
                new_bytes = new_bytes.len(),
                "Resized image for VLM dispatch"
            );
            (new_bytes, "image/jpeg".to_string())
        }
        Err(e) => {
            tracing::warn!(error = %e, "JPEG re-encode failed after resize; sending original bytes");
            (bytes, detected_mime.to_string())
        }
    }
}

/// Infer MIME type from the leading bytes of a base64-encoded image.
/// Thin wrapper around the shared [`crate::image_utils`] implementation
/// to keep the vision tool self-contained for downstream callers.
fn infer_mime_from_base64_prefix(s: &str) -> Option<&'static str> {
    crate::image_utils::infer_mime_from_base64_prefix(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_config_default() {
        let config = VisionConfig::default();
        assert!(config.enabled);
        assert!(config.vlm_backend_id.is_none());
        assert_eq!(config.max_tokens, 1024);
        assert_eq!(config.capture_timeout_secs, 10);
    }

    #[test]
    fn test_validate_url_blocks_ftp() {
        let url = reqwest::Url::parse("ftp://example.com").unwrap();
        assert!(validate_url(&url).is_err());
    }

    #[test]
    fn test_validate_url_blocks_localhost() {
        let url = reqwest::Url::parse("http://localhost:9375").unwrap();
        assert!(validate_url(&url).is_err());
    }

    #[test]
    fn test_validate_url_allows_public() {
        let url = reqwest::Url::parse("https://example.com/photo.jpg").unwrap();
        assert!(validate_url(&url).is_ok());
    }

    // --- infer_mime_from_base64_prefix tests ---

    #[test]
    fn test_infer_mime_jpeg() {
        assert_eq!(
            infer_mime_from_base64_prefix("/9j/4AAQSkZJRg"),
            Some("image/jpeg")
        );
    }

    #[test]
    fn test_infer_mime_png() {
        assert_eq!(
            infer_mime_from_base64_prefix("iVBORw0KGgoAAAAN"),
            Some("image/png")
        );
    }

    #[test]
    fn test_infer_mime_webp() {
        assert_eq!(
            infer_mime_from_base64_prefix("UklGRiQA"),
            Some("image/webp")
        );
    }

    #[test]
    fn test_infer_mime_gif() {
        assert_eq!(infer_mime_from_base64_prefix("R0lGODlh"), Some("image/gif"));
    }

    #[test]
    fn test_infer_mime_bmp() {
        assert_eq!(infer_mime_from_base64_prefix("Qk0+AAAA"), Some("image/bmp"));
    }

    #[test]
    fn test_infer_mime_unknown_returns_none() {
        assert_eq!(infer_mime_from_base64_prefix("randomStuff"), None);
    }

    // --- resize_image_if_needed tests ---

    /// Helper: create a solid-color PNG of the given dimensions.
    fn make_test_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 50, 50]));
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .expect("write test PNG");
        buf.into_inner()
    }

    #[test]
    fn test_resize_skips_when_within_limit() {
        let bytes = make_test_png(800, 600);
        let (out, mime) = resize_image_if_needed(bytes.clone(), "image/png", 1280);
        assert_eq!(mime, "image/png", "no resize → original mime preserved");
        assert_eq!(out, bytes, "no resize → bytes unchanged");
    }

    #[test]
    fn test_resize_downscales_large_image() {
        let bytes = make_test_png(3000, 2000);
        let orig_len = bytes.len();
        let (out, mime) = resize_image_if_needed(bytes, "image/png", 1280);
        assert_eq!(mime, "image/jpeg", "resized → re-encoded as JPEG");
        let img = image::load_from_memory(&out).expect("decoded resized output");
        assert!(
            img.width() <= 1280 && img.height() <= 1280,
            "dims within 1280 box"
        );
        assert!(out.len() < orig_len, "resized output smaller than original");
    }

    #[test]
    fn test_resize_disabled_when_max_dim_zero() {
        let bytes = make_test_png(3000, 2000);
        let (out, mime) = resize_image_if_needed(bytes.clone(), "image/png", 0);
        assert_eq!(out, bytes, "max_dim=0 → no resize");
        assert_eq!(mime, "image/png");
    }

    #[test]
    fn test_resize_fail_open_on_undecodable() {
        // Garbage bytes that aren't a valid image → fail-open, return original
        let bytes = b"not-an-image-at-all!!".to_vec();
        let (out, mime) = resize_image_if_needed(bytes.clone(), "image/jpeg", 1280);
        assert_eq!(out, bytes, "decode failure → original bytes returned");
        assert_eq!(mime, "image/jpeg");
    }
}
