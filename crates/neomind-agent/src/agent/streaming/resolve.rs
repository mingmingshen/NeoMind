use super::super::types::LargeDataCache;

/// Argument names that typically hold image/base64 data.
const IMAGE_ARG_NAMES: &[&str] = &["image", "image_base64", "base64_data", "image_data", "img"];

/// Resolve `$cached:tool_name` references in tool arguments by replacing them
/// with the full cached data. Also **auto-injects** cached image data for any
/// image-related argument — the LLM cannot reliably pass binary image data, so
/// whenever cached image data exists it takes precedence over the LLM's value.
///
/// Only HTTP(S) URLs are passed through (they may point to a real image resource).
pub(crate) fn resolve_cached_arguments(
    arguments: &serde_json::Value,
    cache: &LargeDataCache,
) -> serde_json::Value {
    match arguments {
        // Explicit $cached: reference → resolve
        serde_json::Value::String(s) if s.starts_with("$cached:") => {
            if let Some(resolved) = cache.resolve_reference(s) {
                tracing::info!(
                    reference = %s,
                    resolved_bytes = resolved.len(),
                    "Resolved cached data reference in tool arguments"
                );
                serde_json::Value::String(resolved)
            } else {
                tracing::warn!(reference = %s, "Cached data reference not found, using as-is");
                arguments.clone()
            }
        }
        serde_json::Value::Object(map) => {
            let resolved: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| {
                    let resolved_val = resolve_cached_arguments(v, cache);
                    // Auto-injection for image arguments:
                    // The LLM cannot reliably pass binary image data — it will copy
                    // truncated previews, output MIME types, or invent values.
                    // If we have cached image data, always prefer it over the LLM's value.
                    if IMAGE_ARG_NAMES.contains(&k.as_str()) {
                        if let serde_json::Value::String(ref s) = resolved_val {
                            // Pass through valid HTTP(S) URLs — those are legitimate references
                            if !s.starts_with("http://") && !s.starts_with("https://") {
                                if let Some((image_data, source)) = cache.get_latest_image() {
                                    tracing::info!(
                                        arg_name = %k,
                                        original_preview = %&s[..s.len().min(80)],
                                        source = %source,
                                        injected_bytes = image_data.len(),
                                        "Auto-injected cached image data (LLM cannot pass binary data)"
                                    );
                                    return (k.clone(), serde_json::Value::String(image_data));
                                }
                            }
                        }
                    }
                    (k.clone(), resolved_val)
                })
                .collect();
            serde_json::Value::Object(resolved)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.iter()
                .map(|v| resolve_cached_arguments(v, cache))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Map simplified tool names to real tool names.
///
/// Simplified names are used in LLM prompts (e.g., "device.discover")
/// while real names are used in ToolRegistry (e.g., "list_devices").
///
/// NOTE: This now uses the unified ToolNameMapper to ensure consistency.
pub(crate) fn resolve_tool_name(simplified_name: &str) -> String {
    crate::tools::resolve_tool_name(simplified_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_cached_arguments_passthrough() {
        let cache = LargeDataCache::new();
        let args = serde_json::json!({"name": "test", "count": 42});
        let resolved = resolve_cached_arguments(&args, &cache);
        assert_eq!(resolved["name"], "test");
        assert_eq!(resolved["count"], 42);
    }

    #[test]
    fn test_resolve_cached_arguments_missing_ref() {
        let cache = LargeDataCache::new();
        let args = serde_json::json!("$cached:nonexistent");
        let resolved = resolve_cached_arguments(&args, &cache);
        // Should pass through as-is when ref not found
        assert_eq!(resolved, serde_json::json!("$cached:nonexistent"));
    }

    #[test]
    fn test_http_urls_passed_through() {
        let cache = LargeDataCache::new();
        let args = serde_json::json!({"image": "https://example.com/img.png"});
        let resolved = resolve_cached_arguments(&args, &cache);
        assert_eq!(resolved["image"], "https://example.com/img.png");
    }

    #[test]
    fn test_resolve_tool_name_passthrough() {
        // Unknown simplified names should pass through unchanged
        let result = resolve_tool_name("unknown_tool");
        assert_eq!(result, "unknown_tool");
    }
}
