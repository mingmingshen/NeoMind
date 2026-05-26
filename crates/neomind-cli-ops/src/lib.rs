pub mod api_client;
pub mod auto_auth;
pub mod output;
pub mod types;

// Command modules
pub mod device;
pub mod dashboard;
pub mod rule;
pub mod transform;
pub mod extension;
pub mod agent_cmd;
pub mod message;
pub mod widget;
pub mod system;
pub mod connector;
pub mod llm;
pub mod settings;
pub mod config_cmd;
pub mod automation;
pub mod data_push;

pub use api_client::ApiClient;
pub use types::{BuildMeta, CliResponse, OutputFormat};
