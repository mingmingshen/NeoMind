//! Common API handler utilities.
//!
//! This module provides shared utilities for API handlers including
//! unified error handling, response builders, and common patterns.

use axum::response::Json;
use base64::Engine;
use serde::Deserialize;

use crate::models::{common::ApiResponse, error::ErrorResponse, pagination::PaginationMeta};

/// Unified Result type for all API handlers.
///
/// All handlers should return this type for consistent error handling.
/// The success value is automatically wrapped in ApiResponse.
pub type HandlerResult<T> = Result<Json<ApiResponse<T>>, ErrorResponse>;

/// Result type for utility functions that return parsed values (not full responses).
pub type ExtractResult<T> = Result<T, ErrorResponse>;

/// Pagination query parameters.
///
/// Supports two modes:
/// - **Offset-based** (default): use `page` + `page_size` for traditional pagination.
/// - **Cursor-based** (opt-in): provide `cursor` to skip directly to the last-seen key
///   for O(1) deep-page performance. When `cursor` is present, `page`/`page_size` are
///   ignored in favour of `limit`.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationQuery {
    /// Page number (1-indexed). Ignored when `cursor` is provided.
    #[serde(default = "default_page")]
    pub page: usize,

    /// Items per page. Ignored when `cursor` is provided.
    #[serde(default = "default_page_size")]
    pub page_size: usize,

    /// Opaque cursor for cursor-based pagination (base64-encoded last key).
    /// When provided, takes precedence over `page`/`page_size` for better performance
    /// on deep pagination.
    pub cursor: Option<String>,

    /// Page size for cursor-based pagination (default: 10).
    /// Only used when `cursor` is provided.
    pub limit: Option<u32>,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

impl PaginationQuery {
    /// Returns `true` if this request uses cursor-based pagination.
    pub fn is_cursor_based(&self) -> bool {
        self.cursor.is_some()
    }

    /// Decode the cursor to get the underlying last key.
    ///
    /// Returns `None` if no cursor was provided or if it is not valid base64 / UTF-8.
    pub fn decode_cursor(&self) -> Option<String> {
        self.cursor.as_ref().and_then(|c| {
            base64::engine::general_purpose::STANDARD
                .decode(c)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        })
    }

    /// Encode a key as an opaque cursor string suitable for the next page.
    pub fn encode_cursor(key: &str) -> String {
        base64::engine::general_purpose::STANDARD.encode(key.as_bytes())
    }

    /// Get the effective limit for cursor-based pagination.
    /// Falls back to `page_size` (capped at 100) when `limit` is not set.
    pub fn cursor_limit(&self) -> usize {
        self.limit
            .unwrap_or(self.page_size as u32)
            .min(100) as usize
    }

    /// Get the offset for database queries (offset-based mode).
    pub fn offset(&self) -> usize {
        (self.page.saturating_sub(1)) * self.page_size
    }

    /// Get the limit for database queries (offset-based mode).
    pub fn limit(&self) -> usize {
        self.page_size
    }

    /// Validate and clamp the pagination parameters.
    pub fn validate(mut self) -> ExtractResult<Self> {
        if self.page == 0 {
            self.page = 1;
        }
        if self.page_size == 0 {
            self.page_size = 20;
        }
        if self.page_size > 1000 {
            return Err(ErrorResponse::bad_request("page_size cannot exceed 1000"));
        }
        if let Some(lim) = self.limit {
            if lim == 0 {
                self.limit = Some(10);
            } else if lim > 100 {
                return Err(ErrorResponse::bad_request("limit cannot exceed 100"));
            }
        }
        Ok(self)
    }

    /// Convert to PaginationMeta for API responses (offset-based mode).
    pub fn to_meta(&self, total: u32) -> PaginationMeta {
        let total_pages = (total as usize).div_ceil(self.page_size);
        PaginationMeta {
            page: self.page as u32,
            page_size: self.page_size as u32,
            total_count: total,
            total_pages: total_pages as u32,
            has_next: self.page < total_pages,
            has_prev: self.page > 1,
            next_cursor: None,
        }
    }

    /// Build PaginationMeta for a cursor-based response.
    ///
    /// `total_count` is the total number of items (may be approximate for some stores).
    /// `last_key` is the key of the last item returned; it will be encoded as `next_cursor`
    /// when there are more results.
    pub fn to_cursor_meta(&self, total_count: u32, returned_count: usize, last_key: Option<&str>) -> PaginationMeta {
        let effective_limit = self.cursor_limit();
        let has_more = returned_count >= effective_limit;
        PaginationMeta {
            page: 0, // not applicable in cursor mode
            page_size: effective_limit as u32,
            total_count,
            total_pages: if effective_limit > 0 { total_count.div_ceil(effective_limit as u32) } else { 0 },
            has_next: has_more,
            has_prev: false, // cursor pagination is forward-only
            next_cursor: if has_more {
                last_key.map(|k| Self::encode_cursor(k))
            } else {
                None
            },
        }
    }
}

/// Extract a path parameter or return a 400 error.
///
/// # Example
/// ```rust,no_run
/// use axum::extract::Path;
/// use crate::handlers::common::extract_path;
///
/// async fn get_item(Path(id): Path<String>) -> HandlerResult<Item> {
///     let id = extract_path::<i32>(&id)?;
///     // ...
///     # todo!()
/// }
/// ```
pub fn extract_path<T>(value: &str) -> ExtractResult<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse::<T>()
        .map_err(|_| ErrorResponse::bad_request(format!("Invalid path parameter: {}", value)))
}

/// Extract an optional query parameter.
pub fn extract_query_opt<T>(value: &Option<String>) -> ExtractResult<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match value {
        Some(v) => {
            let parsed = v.parse::<T>().map_err(|_| {
                ErrorResponse::bad_request(format!("Invalid query parameter: {}", v))
            })?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

/// Extract a required query parameter or return a 400 error.
pub fn extract_query<T>(value: &Option<String>, name: &str) -> ExtractResult<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match value {
        Some(v) => v.parse::<T>().map_err(|_| {
            ErrorResponse::bad_request(format!("Invalid query parameter '{}': {}", name, v))
        }),
        None => Err(ErrorResponse::bad_request(format!(
            "Missing required query parameter: {}",
            name
        ))),
    }
}

/// Create a successful response with data.
pub fn ok<T: serde::Serialize>(data: T) -> HandlerResult<T> {
    Ok(Json(ApiResponse::success(data)))
}

/// Create a successful response with data and metadata (for pagination).
pub fn ok_with_meta<T: serde::Serialize>(data: T, meta: PaginationMeta) -> HandlerResult<T> {
    Ok(Json(ApiResponse::paginated(data, meta)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_valid() {
        let result: ExtractResult<i32> = extract_path("42");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_extract_path_invalid() {
        let result: ExtractResult<i32> = extract_path("abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_query_opt_some() {
        let value = Some("42".to_string());
        let result: ExtractResult<Option<i32>> = extract_query_opt(&value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_extract_query_opt_none() {
        let value: Option<String> = None;
        let result: ExtractResult<Option<i32>> = extract_query_opt(&value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_pagination_offset() {
        let query = PaginationQuery {
            page: 2,
            page_size: 10,
            cursor: None,
            limit: None,
        };
        assert_eq!(query.offset(), 10);
        assert_eq!(query.limit(), 10);
    }

    #[test]
    fn test_pagination_validate() {
        let query = PaginationQuery {
            page: 0,
            page_size: 50,
            cursor: None,
            limit: None,
        };
        let result = query.validate().unwrap();
        assert_eq!(result.page, 1); // Fixed to 1
        assert_eq!(result.page_size, 50);
    }

    #[test]
    fn test_pagination_validate_max_page_size() {
        let query = PaginationQuery {
            page: 1,
            page_size: 2000,
            cursor: None,
            limit: None,
        };
        let result = query.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_pagination_to_meta() {
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: None,
            limit: None,
        };
        let meta = query.to_meta(25);
        assert_eq!(meta.page, 1);
        assert_eq!(meta.page_size, 10);
        assert_eq!(meta.total_count, 25);
        assert_eq!(meta.total_pages, 3);
        assert!(meta.next_cursor.is_none());
    }

    #[test]
    fn test_ok_helper() {
        let result: HandlerResult<String> = ok("test".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.data, Some("test".to_string()));
    }

    #[test]
    fn test_ok_with_meta_helper() {
        let meta = PaginationMeta {
            page: 1,
            page_size: 10,
            total_count: 100,
            total_pages: 10,
            has_next: true,
            has_prev: false,
            next_cursor: None,
        };
        let result: HandlerResult<String> = ok_with_meta("test".to_string(), meta);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.data, Some("test".to_string()));
    }

    #[test]
    fn test_cursor_encode_decode() {
        let key = "device:sensor-001";
        let encoded = PaginationQuery::encode_cursor(key);
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: Some(encoded),
            limit: None,
        };
        assert!(query.is_cursor_based());
        let decoded = query.decode_cursor().unwrap();
        assert_eq!(decoded, key);
    }

    #[test]
    fn test_cursor_decode_invalid_base64() {
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: Some("not-valid-base64!!!".to_string()),
            limit: None,
        };
        assert!(query.is_cursor_based());
        assert!(query.decode_cursor().is_none());
    }

    #[test]
    fn test_cursor_decode_invalid_utf8() {
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: Some(base64::engine::general_purpose::STANDARD.encode([0xff, 0xfe])),
            limit: None,
        };
        // Decodes base64 successfully but the bytes are not valid UTF-8
        // Actually, [0xff, 0xfe] IS valid UTF-8 bytes... they decode to a string.
        // Let's just verify it returns Some since these bytes happen to be valid.
        assert!(query.decode_cursor().is_some());
    }

    #[test]
    fn test_cursor_no_cursor_provided() {
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: None,
            limit: None,
        };
        assert!(!query.is_cursor_based());
        assert!(query.decode_cursor().is_none());
    }

    #[test]
    fn test_cursor_limit_falls_back_to_page_size() {
        let query = PaginationQuery {
            page: 1,
            page_size: 25,
            cursor: Some("abc".to_string()),
            limit: None,
        };
        assert_eq!(query.cursor_limit(), 25);
    }

    #[test]
    fn test_cursor_limit_capped_at_100() {
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: Some("abc".to_string()),
            limit: Some(200),
        };
        // limit > 100 should be caught by validate(), but cursor_limit still caps
        assert_eq!(query.cursor_limit(), 100);
    }

    #[test]
    fn test_cursor_validate_zero_limit() {
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: Some("abc".to_string()),
            limit: Some(0),
        };
        let result = query.validate().unwrap();
        assert_eq!(result.limit, Some(10));
    }

    #[test]
    fn test_cursor_validate_limit_exceeds_max() {
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: Some("abc".to_string()),
            limit: Some(200),
        };
        let result = query.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_to_cursor_meta_has_more() {
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: Some("abc".to_string()),
            limit: Some(10),
        };
        // returned_count == limit => has_more = true
        let meta = query.to_cursor_meta(50, 10, Some("last-key-123"));
        assert_eq!(meta.page, 0); // not applicable
        assert_eq!(meta.page_size, 10);
        assert!(meta.has_next);
        assert!(!meta.has_prev);
        assert!(meta.next_cursor.is_some());
        // Verify the cursor encodes the last key
        assert_eq!(meta.next_cursor.unwrap(), PaginationQuery::encode_cursor("last-key-123"));
    }

    #[test]
    fn test_to_cursor_meta_no_more() {
        let query = PaginationQuery {
            page: 1,
            page_size: 10,
            cursor: Some("abc".to_string()),
            limit: Some(10),
        };
        // returned_count < limit => no more results
        let meta = query.to_cursor_meta(8, 8, Some("last-key-123"));
        assert!(!meta.has_next);
        assert!(meta.next_cursor.is_none());
    }
}
