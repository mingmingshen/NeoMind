use serde::{Deserialize, Serialize};

/// Structured output for all CLI commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_meta: Option<BuildMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMeta {
    pub r#type: String,       // "device" | "dashboard" | "rule" | ...
    pub action: String,       // "create" | "update" | "delete"
    pub entity_id: String,
    pub entity_name: Option<String>,
    pub undo_command: String,
}

impl CliResponse {
    pub fn success(data: serde_json::Value, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.into()),
            error: None,
            code: None,
            build_meta: None,
        }
    }

    pub fn success_with_meta(
        data: serde_json::Value,
        message: impl Into<String>,
        meta: BuildMeta,
    ) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.into()),
            error: None,
            code: None,
            build_meta: Some(meta),
        }
    }

    pub fn error(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(error.into()),
            code: Some(code.into()),
            build_meta: None,
        }
    }
}

/// Output format control
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
}
