//! Error types for the rules crate.

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Compilation error: {0}")]
    Compilation(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Result type for rule operations
pub type Result<T> = std::result::Result<T, RuleError>;
