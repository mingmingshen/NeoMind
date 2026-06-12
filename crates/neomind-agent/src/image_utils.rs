//! Image data parsing utilities shared across chat, streaming, agent
//! analyzer, and data collector paths.
//!
//! Consolidates MIME type detection and base64 handling that was previously
//! duplicated across 4+ files with inconsistent behavior (missing
//! `data:image/jpg` alias in streaming, hardcoded `image/png` fallbacks,
//! `contains()` vs `starts_with()` for magic bytes, etc.).

/// Result of parsing an image data string.
#[derive(Debug, Clone, Copy)]
pub struct ParsedImage<'a> {
    /// Detected MIME type (e.g. `"image/jpeg"`, `"image/png"`).
    pub mime_type: &'static str,
    /// The raw base64 portion (no `data:...;base64,` prefix).
    pub base64: &'a str,
}

/// Parse an image data string into a [`ParsedImage`].
///
/// Accepts:
/// - `data:image/<subtype>;base64,<data>` URLs (mime from URL header)
/// - Raw base64 with recognizable magic prefix (mime inferred from prefix)
/// - Other non-empty strings: returns `image/png` as conservative fallback
///
/// Returns `None` only for empty input.
pub fn parse_image_data(s: &str) -> Option<ParsedImage<'_>> {
    if s.is_empty() {
        return None;
    }
    if let Some(parsed) = parse_data_image_url(s) {
        return Some(parsed);
    }
    // Raw base64 path — detect mime from magic prefix.
    let mime_type = infer_mime_from_base64_prefix(s).unwrap_or("image/png");
    Some(ParsedImage {
        mime_type,
        base64: s,
    })
}

/// Try to parse a `data:image/<subtype>;base64,<data>` URL.
///
/// Returns `None` if `s` is not a data URL, has no comma separator, or
/// the MIME subtype is unrecognized.
pub fn parse_data_image_url(s: &str) -> Option<ParsedImage<'_>> {
    let after_data = s.strip_prefix("data:")?;
    let (header, data) = after_data.split_once(',')?;
    if data.is_empty() {
        return None;
    }
    // Header is like "image/jpeg;base64" or "image/png;base64"
    let (mime_part, _params) = header.split_once(';').unwrap_or((header, ""));
    if !mime_part.starts_with("image/") {
        return None;
    }
    let subtype = &mime_part["image/".len()..];
    let mime_type = normalize_mime_subtype(subtype)?;
    Some(ParsedImage {
        mime_type,
        base64: data,
    })
}

/// Normalize a MIME subtype (e.g. `"jpeg"`, `"jpg"`, `"png"`) into a
/// canonical `"image/<subtype>"` string. Returns `None` for unknown subtypes.
///
/// Handles the `jpg` → `jpeg` alias consistently across all callers.
pub fn normalize_mime_subtype(subtype: &str) -> Option<&'static str> {
    match subtype.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpeg" | "jpg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "tiff" | "tif" => Some("image/tiff"),
        _ => None,
    }
}

/// Detect image MIME type from the magic prefix of a base64 string.
///
/// Uses `starts_with` (not `contains`) so a string that merely mentions
/// `/9j/` somewhere in the middle won't be misclassified as JPEG.
pub fn infer_mime_from_base64_prefix(s: &str) -> Option<&'static str> {
    if s.starts_with("/9j/") {
        Some("image/jpeg")
    } else if s.starts_with("iVBORw0KGgo") {
        Some("image/png")
    } else if s.starts_with("UklGR") {
        Some("image/webp")
    } else if s.starts_with("R0lGOD") {
        Some("image/gif")
    } else if s.starts_with("Qk") {
        Some("image/bmp")
    } else if s.starts_with("SUkq") || s.starts_with("TU0") {
        Some("image/tiff")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_data_url_jpeg() {
        let p = parse_image_data("data:image/jpeg;base64,/9j/AAAA").unwrap();
        assert_eq!(p.mime_type, "image/jpeg");
        assert_eq!(p.base64, "/9j/AAAA");
    }

    #[test]
    fn parse_data_url_jpg_alias_canonicalized() {
        // Regression for the streaming.rs bug that only checked "jpeg".
        let p = parse_image_data("data:image/jpg;base64,/9j/AAAA").unwrap();
        assert_eq!(p.mime_type, "image/jpeg");
        assert_eq!(p.base64, "/9j/AAAA");
    }

    #[test]
    fn parse_data_url_png() {
        let p = parse_image_data("data:image/png;base64,iVBORw0KGgo=").unwrap();
        assert_eq!(p.mime_type, "image/png");
        assert_eq!(p.base64, "iVBORw0KGgo=");
    }

    #[test]
    fn parse_data_url_unknown_subtype_rejected() {
        assert!(parse_data_image_url("data:image/heic;base64,AAAA").is_none());
    }

    #[test]
    fn parse_data_url_empty_data_rejected() {
        assert!(parse_data_image_url("data:image/jpeg;base64,").is_none());
    }

    #[test]
    fn parse_raw_base64_jpeg_magic() {
        let p = parse_image_data("/9j/4AAQSkZJRgABAQ").unwrap();
        assert_eq!(p.mime_type, "image/jpeg");
    }

    #[test]
    fn parse_raw_base64_png_magic() {
        let p = parse_image_data("iVBORw0KGgoAAAANSUhEUg").unwrap();
        assert_eq!(p.mime_type, "image/png");
    }

    #[test]
    fn parse_raw_base64_webp_magic() {
        let p = parse_image_data("UklGRkBAAABAQ").unwrap();
        assert_eq!(p.mime_type, "image/webp");
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(parse_image_data("").is_none());
    }

    #[test]
    fn parse_unknown_falls_back_to_png() {
        let p = parse_image_data("AAAAAaaaaa").unwrap();
        assert_eq!(p.mime_type, "image/png");
    }

    #[test]
    fn magic_in_middle_does_not_match() {
        // Regression for data_collector bug that used contains() instead of
        // starts_with(): a text string with "/9j/" in the middle must NOT
        // be misclassified as an image.
        let s = "device log: /9j/ something happened";
        assert!(infer_mime_from_base64_prefix(s).is_none());
    }

    #[test]
    fn normalize_jpg_alias() {
        assert_eq!(normalize_mime_subtype("jpg"), Some("image/jpeg"));
        assert_eq!(normalize_mime_subtype("jpeg"), Some("image/jpeg"));
        assert_eq!(normalize_mime_subtype("JPEG"), Some("image/jpeg"));
        assert_eq!(normalize_mime_subtype("heic"), None);
    }
}
