//! WebSocket 处理相关模块
//!
//! 包含连接状态管理、心跳检测等功能

pub mod connection_state;

pub use connection_state::{
    ConnectionMetadata, ConnectionState, ConnectionStateRef, HeartbeatState,
    create_connection_metadata,
};
