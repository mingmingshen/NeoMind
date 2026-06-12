//! API server for Edge AI Agent.
//!
//! This crate provides the HTTP/WebSocket API server for the Edge AI Agent system.

pub mod auth;
pub mod auth_users;
pub mod automation;
pub mod cache;
pub mod capability_providers;
pub mod config;
pub mod crypto;
pub mod event_services;
pub mod handlers;
pub mod models;

pub mod rate_limit;
pub mod server;
pub mod shutdown;
pub mod startup;
pub mod validator;

// Re-export the server entry point for the CLI binary
pub use server::run;
