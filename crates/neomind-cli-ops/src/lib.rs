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
pub mod broker;
pub mod help;

pub use api_client::ApiClient;
pub use types::{BuildMeta, CliResponse, OutputFormat};
