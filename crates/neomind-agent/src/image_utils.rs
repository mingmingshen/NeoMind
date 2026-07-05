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

// ---------------------------------------------------------------------------
// I/O helpers moved from vision.rs (SSRF protection + MIME detection)
// ---------------------------------------------------------------------------

/// Detect MIME type from image file magic bytes.
pub fn detect_mime_from_bytes(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 4 {
        return None;
    }
    match &bytes[..4] {
        [0x89, 0x50, 0x4E, 0x47] => Some("image/png"),
        [0xFF, 0xD8, 0xFF, _] => Some("image/jpeg"),
        [0x47, 0x49, 0x46, _] => Some("image/gif"),
        [0x52, 0x49, 0x46, _] if bytes.len() >= 12 && &bytes[8..12] == b"WEBP" => {
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
/// This MUST stay before the "local file path" branch in callers that
/// resolve image inputs, because JPEG base64 starts with `/9j/` and would
/// otherwise be misclassified as an absolute path.
pub fn looks_like_raw_base64(s: &str) -> bool {
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
            || s.starts_with("TU0")); // TIFF BE
    if has_magic {
        return true;
    }
    // Long pure-base64 strings without magic — still likely image data,
    // but require a reasonable minimum to avoid false positives on hash-like
    // or token-like strings (e.g. JWT fragments, 256-char base64 tokens).
    s.len() >= 1024
}

/// Check if a hostname points to a private/local address (SSRF protection).
pub fn is_private_host(host: &str) -> bool {
    match host {
        "localhost" | "127.0.0.1" | "0.0.0.0" | "::1" => return true,
        _ => {}
    }

    let host_trimmed = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = host_trimmed.parse::<std::net::IpAddr>() {
        return is_private_ip(&ip);
    }

    if host.ends_with(".local") || host.ends_with(".localhost") || host == "localhost.localdomain" {
        return true;
    }

    false
}

/// Check if an IP address is private/local.
pub fn is_private_ip(ip: &std::net::IpAddr) -> bool {
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

    // --- detect_mime_from_bytes tests ---

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
        assert!(looks_like_raw_base64(
            "UklGRiQAAABXRUJQVlA4IBgAAAAwAQCdASoB"
        ));
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
        // 1024+ chars of pure base64 alphabet without magic — still likely image
        let long: String = "ABCD".repeat(300); // 1200 chars
        assert!(looks_like_raw_base64(&long));
    }

    #[test]
    fn test_looks_like_raw_base64_rejects_medium_token_like() {
        // 320 chars without magic — could be a JWT/hash fragment, reject
        let medium: String = "ABCD".repeat(80); // 320 chars
        assert!(!looks_like_raw_base64(&medium));
    }

    #[test]
    fn test_looks_like_raw_base64_rejects_empty() {
        assert!(!looks_like_raw_base64(""));
    }

    // --- is_private_host tests ---

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
}
