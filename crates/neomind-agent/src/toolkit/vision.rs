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
use crate::llm_backends::LlmBackendInstanceManager;

/// Maximum image size in bytes (10 MB). VLMs typically downsample to
/// ~448-672px anyway, so 10 MB is more than sufficient.
const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum base64 string length (~13.3 MB base64 for 10 MB raw).
const MAX_BASE64_LEN: usize = MAX_IMAGE_SIZE * 4 / 3 + 4;

/// Allowed image file extensions (lowercase). Only binary raster formats
/// that pass `detect_mime_from_bytes()` are included. SVG is excluded
/// because it cannot pass the magic-bytes validation for local files.
const ALLOWED_IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif",
];

/// Allowed MIME subtypes in data URLs (the part after "image/").
const ALLOWED_DATA_MIME_SUBTYPES: &[&str] = &[
    "png", "jpeg", "jpg", "gif", "webp", "bmp", "tiff",
];

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
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vlm_backend_id: None,
            max_tokens: 1024,
            capture_timeout_secs: 10,
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
                self.llm_manager
                    .get_runtime(&id)
                    .await
                    .map_err(|e| ToolError::Execution(format!(
                        "VLM backend '{}' unavailable: {}. Check if the model service is running.",
                        id, e
                    )))
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
        let image_lower_prefix = image.chars().take(11).collect::<String>().to_ascii_lowercase();
        let is_data_image_url = image_lower_prefix == "data:image/";
        let is_incomplete_data_url = !is_data_image_url && image.contains(";base64,")
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
                            b64.len(), MAX_BASE64_LEN
                        )));
                    }
                    let subtype = mime_suffix.to_lowercase();
                    if !ALLOWED_DATA_MIME_SUBTYPES.contains(&subtype.as_str()) {
                        return Err(ToolError::InvalidArguments(format!(
                            "Unsupported image type '{}' in data URL. Allowed: {}",
                            subtype, ALLOWED_DATA_MIME_SUBTYPES.join(", ")
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
            let mime =
                detect_mime_from_bytes(&bytes).unwrap_or("image/jpeg").to_string();
            let b64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &bytes,
            );
            return Ok((b64, mime));
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
                return Err(ToolError::InvalidArguments(
                    "Image data is empty".into(),
                ));
            }
            if image.len() > MAX_BASE64_LEN {
                return Err(ToolError::InvalidArguments(format!(
                    "Base64 data too large ({} chars, max {} chars)",
                    image.len(), MAX_BASE64_LEN
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
            let mime =
                detect_mime_from_bytes(&bytes).unwrap_or("image/jpeg").to_string();
            let b64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &bytes,
            );
            return Ok((b64, mime));
        }

        // 6. Fallback: treat as raw base64
        if image.is_empty() {
            return Err(ToolError::InvalidArguments(
                "Image data is empty".into(),
            ));
        }
        if image.len() > MAX_BASE64_LEN {
            return Err(ToolError::InvalidArguments(format!(
                "Base64 data too large ({} chars, max {} chars)",
                image.len(), MAX_BASE64_LEN
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
            "/etc/", "/proc/", "/sys/", "/dev/", "/run/", "/boot/", "/root/",
            "/var/", "/tmp/", "/opt/", "/srv/",
        ];
        if blocked_prefixes
            .iter()
            .any(|prefix| canonical_lower.starts_with(prefix))
        {
            return Err(ToolError::PermissionDenied(format!(
                "Access to system path is not allowed"
            )));
        }

        // Block hidden files (dotfiles) in home directories
        if let Some(name) = canonical.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.')
                && (canonical_lower.starts_with("/home")
                    || canonical_lower.starts_with("/users"))
            {
                return Err(ToolError::PermissionDenied(format!(
                    "Access to hidden file is not allowed"
                )));
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
                "prompt": string_property("Analysis instructions for the image. IMPORTANT: always forward the user's language preference, desired detail level, and any specific focus from the conversation. E.g. if the user speaks Chinese, write the prompt in Chinese; if they want a brief answer, include 'be concise'.")
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

/// Detect MIME type from image file magic bytes.
fn detect_mime_from_bytes(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 4 {
        return None;
    }
    match &bytes[..4] {
        [0x89, 0x50, 0x4E, 0x47] => Some("image/png"),
        [0xFF, 0xD8, 0xFF, _] => Some("image/jpeg"),
        [0x47, 0x49, 0x46, _] => Some("image/gif"),
        [0x52, 0x49, 0x46, _]
            if bytes.len() >= 12 && &bytes[8..12] == b"WEBP" =>
        {
            Some("image/webp")
        }
        // BMP: starts with "BM"
        [0x42, 0x4D, _, _] => Some("image/bmp"),
        // TIFF little-endian: II*\0
        [0x49, 0x49, 0x2A, 0x00] => Some("image/tiff"),
        // TIFF big-endian: MM\0*
        [0x4D, 0x4D, 0x00, 0x2A] => Some("image/tiff"),
        _ => None,
    }
}

/// Heuristic: does this string look like raw base64-encoded image data
/// (as opposed to a filesystem path or URL)?
///
/// We say "yes" when:
/// 1. Every character belongs to the standard base64 alphabet
///    (`A-Za-z0-9+/=`), AND
/// 2. Either the string starts with a known image magic prefix
///    (`/9j/` JPEG, `iVBORw0KGgo` PNG, `UklGR` WebP, `R0lGOD` GIF,
///    `Qk` BMP, `SUkq` TIFF) AND is at least 32 chars long, OR
/// 3. The string is long enough (>=256 chars) to plausibly be image data
///    (avoids short base64-looking strings like path components).
///
/// This MUST stay before the "local file path" branch in [`resolve_image`],
/// because JPEG base64 starts with `/9j/` and would otherwise be
/// misclassified as an absolute path.
fn looks_like_raw_base64(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Quick reject: any character outside base64 alphabet.
    // '/', '+', '=' are valid base64 chars. '.', '\', ':' are NOT — they
    // reject real paths like "/tmp/foo.jpg", "C:\\...", "/home/a:b".
    let is_pure_b64 = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=');
    if !is_pure_b64 {
        return false;
    }
    // Known image base64 magic prefixes, with minimum length to avoid
    // false positives on very short strings like "/9j/" alone (4 chars).
    const MIN_MAGIC_LEN: usize = 32;
    let has_magic = s.len() >= MIN_MAGIC_LEN
        && (s.starts_with("/9j/")         // JPEG
            || s.starts_with("iVBORw0KGgo") // PNG
            || s.starts_with("UklGR")       // WebP
            || s.starts_with("R0lGOD")      // GIF
            || s.starts_with("Qk")          // BMP
            || s.starts_with("SUkq")        // TIFF LE
            || s.starts_with("TU0"));       // TIFF BE
    if has_magic {
        return true;
    }
    // Long pure-base64 strings without magic — still likely image data,
    // but require a reasonable minimum to avoid false positives on short
    // strings like "/" or "/abc" which would be valid-looking paths.
    s.len() >= 256
}

/// Infer MIME type from the leading bytes of a base64-encoded image.
/// Thin wrapper around the shared [`crate::image_utils`] implementation
/// to keep the vision tool self-contained for downstream callers.
fn infer_mime_from_base64_prefix(s: &str) -> Option<&'static str> {
    crate::image_utils::infer_mime_from_base64_prefix(s)
}

/// Check if a hostname points to a private/local address (SSRF protection).
fn is_private_host(host: &str) -> bool {
    match host {
        "localhost" | "127.0.0.1" | "0.0.0.0" | "::1" => return true,
        _ => {}
    }

    let host_trimmed = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = host_trimmed.parse::<std::net::IpAddr>() {
        return is_private_ip(&ip);
    }

    if host.ends_with(".local")
        || host.ends_with(".localhost")
        || host == "localhost.localdomain"
    {
        return true;
    }

    false
}

/// Check if an IP address is private/local.
fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let octets = v4.octets();
            if octets[0] == 10 {
                return true;
            }
            if octets[0] == 172 && (16..=31).contains(&octets[1]) {
                return true;
            }
            if octets[0] == 192 && octets[1] == 168 {
                return true;
            }
            if octets[0] == 127 {
                return true;
            }
            if octets[0] == 169 && octets[1] == 254 {
                return true;
            }
            if octets[0] == 0 {
                return true;
            }
            if octets[0] == 100 && (64..=127).contains(&octets[1]) {
                return true;
            }
            if v4.is_broadcast() || v4.is_multicast() || v4.is_unspecified() {
                return true;
            }
        }
        std::net::IpAddr::V6(v6) => {
            if v6.is_loopback() || v6.is_multicast() || v6.is_unspecified() {
                return true;
            }
            let segments = v6.segments();
            if (segments[0] & 0xfe00) == 0xfc00 {
                return true;
            }
            if (segments[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            if let Some(v4) = v6.to_ipv4() {
                return is_private_ip(&std::net::IpAddr::V4(v4));
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_mime_jpeg() {
        assert_eq!(
            detect_mime_from_bytes(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]),
            Some("image/jpeg")
        );
    }

    #[test]
    fn test_detect_mime_png() {
        assert_eq!(
            detect_mime_from_bytes(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]),
            Some("image/png")
        );
    }

    #[test]
    fn test_detect_mime_gif() {
        assert_eq!(
            detect_mime_from_bytes(&[0x47, 0x49, 0x46, 0x38, 0x39, 0x61]),
            Some("image/gif")
        );
    }

    #[test]
    fn test_detect_mime_bmp() {
        assert_eq!(
            detect_mime_from_bytes(&[0x42, 0x4D, 0x00, 0x00]),
            Some("image/bmp")
        );
    }

    #[test]
    fn test_detect_mime_tiff_le() {
        assert_eq!(
            detect_mime_from_bytes(&[0x49, 0x49, 0x2A, 0x00, 0x00, 0x00]),
            Some("image/tiff")
        );
    }

    #[test]
    fn test_detect_mime_tiff_be() {
        assert_eq!(
            detect_mime_from_bytes(&[0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00]),
            Some("image/tiff")
        );
    }

    #[test]
    fn test_detect_mime_unknown() {
        assert_eq!(detect_mime_from_bytes(&[0x00, 0x00, 0x00, 0x00]), None);
    }

    #[test]
    fn test_detect_mime_too_small() {
        assert_eq!(detect_mime_from_bytes(&[0xFF]), None);
        assert_eq!(detect_mime_from_bytes(&[]), None);
    }

    #[test]
    fn test_vision_config_default() {
        let config = VisionConfig::default();
        assert!(config.enabled);
        assert!(config.vlm_backend_id.is_none());
        assert_eq!(config.max_tokens, 1024);
        assert_eq!(config.capture_timeout_secs, 10);
    }

    #[test]
    fn test_is_private_host_localhost() {
        assert!(is_private_host("localhost"));
        assert!(is_private_host("127.0.0.1"));
        assert!(is_private_host("0.0.0.0"));
        assert!(is_private_host("::1"));
    }

    #[test]
    fn test_is_private_host_private_ranges() {
        assert!(is_private_host("10.0.0.1"));
        assert!(is_private_host("172.16.0.1"));
        assert!(is_private_host("192.168.1.1"));
        assert!(is_private_host("169.254.169.254"));
    }

    #[test]
    fn test_is_private_host_public() {
        assert!(!is_private_host("8.8.8.8"));
        assert!(!is_private_host("1.1.1.1"));
        assert!(!is_private_host("example.com"));
    }

    #[test]
    fn test_is_private_ipv6_mapped() {
        assert!(is_private_host("::ffff:127.0.0.1"));
        assert!(is_private_host("::ffff:192.168.1.1"));
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

    // --- looks_like_raw_base64 tests ---

    #[test]
    fn test_looks_like_raw_base64_jpeg_magic() {
        // JPEG base64 always starts with "/9j/" — this used to be
        // misclassified as an absolute file path.
        assert!(looks_like_raw_base64("/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAA"));
    }

    #[test]
    fn test_looks_like_raw_base64_png_magic() {
        assert!(looks_like_raw_base64("iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB"));
    }

    #[test]
    fn test_looks_like_raw_base64_webp_magic() {
        assert!(looks_like_raw_base64("UklGRiQAAABXRUJQVlA4IBgAAAAwAQCdASoB"));
    }

    #[test]
    fn test_looks_like_raw_base64_short_magic_rejects_path_like_strings() {
        // "/9j/" alone is 4 chars and not enough evidence — should NOT match
        // (avoids misclassifying weird short paths)
        assert!(!looks_like_raw_base64("/9j/"));
    }

    #[test]
    fn test_looks_like_raw_base64_rejects_path_with_extension() {
        // Real file paths contain '.', '\', ':' etc which are NOT in base64 alphabet
        assert!(!looks_like_raw_base64("/tmp/photo.jpg"));
        assert!(!looks_like_raw_base64("/home/user/file.png"));
        assert!(!looks_like_raw_base64("./images/photo.jpeg"));
        assert!(!looks_like_raw_base64("C:\\Users\\photo.jpg"));
    }

    #[test]
    fn test_looks_like_raw_base64_rejects_short_pure_alphabet() {
        // Short pure-base64-alphabet strings are ambiguous — treat as path
        assert!(!looks_like_raw_base64("/abc"));
        assert!(!looks_like_raw_base64("ABC"));
    }

    #[test]
    fn test_looks_like_raw_base64_accepts_long_pure() {
        // 256+ chars of pure base64 alphabet without magic — still likely image
        let long: String = "ABCD".repeat(80); // 320 chars
        assert!(looks_like_raw_base64(&long));
    }

    #[test]
    fn test_looks_like_raw_base64_rejects_empty() {
        assert!(!looks_like_raw_base64(""));
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
        assert_eq!(infer_mime_from_base64_prefix("UklGRiQA"), Some("image/webp"));
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
}
