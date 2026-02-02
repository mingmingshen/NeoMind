//! Brand configuration for the application.
//!
//! This module provides centralized brand information (name, version, etc.)
//! to make it easy to rebrand the application.
//!
//! ## Rebranding
//! To rebrand the application, modify the constants in this module.
//!
//! ## Example
//! ```rust
//! use edge_ai_core::brand::APP_NAME;
//!
//! println!("Welcome to {}", APP_NAME);
//! ```

/// Full application name
pub const APP_NAME: &str = "NeoMind";

/// Short name/acronym for compact display
pub const APP_SHORT_NAME: &str = "NM";

/// Application version (from Cargo.toml)
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application homepage URL
pub const APP_HOMEPAGE: &str = "https://github.com/your-org/NeoMind";

/// Application documentation URL
pub const APP_DOCS_URL: &str = "https://docs.example.com";

/// Default user agent string for HTTP requests
pub fn user_agent() -> String {
    format!("{}/{}", APP_NAME, APP_VERSION)
}

/// Get formatted welcome message
pub fn welcome_message() -> String {
    format!("Welcome to {} {}", APP_NAME, APP_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_name() {
        assert_eq!(APP_NAME, "NeoMind");
    }

    #[test]
    fn test_short_name() {
        assert_eq!(APP_SHORT_NAME, "NM");
    }

    #[test]
    fn test_user_agent() {
        let ua = user_agent();
        assert!(ua.contains("NeoMind"));
        assert!(ua.contains('.'));
    }

    #[test]
    fn test_welcome_message() {
        let msg = welcome_message();
        assert!(msg.contains("Welcome to"));
        assert!(msg.contains("NeoMind"));
    }
}
