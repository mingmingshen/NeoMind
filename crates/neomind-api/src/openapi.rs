//! OpenAPI documentation for the NeoMind API.
//!
//! This module sets up the OpenAPI/Swagger documentation for all API endpoints.
//! The documentation is available at:
//! - `/api-docs` - Swagger UI
//! - `/api/openapi.json` - OpenAPI JSON schema

use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::{basic, data};

/// Custom modifier to add server information to the OpenAPI spec.
#[derive(Default)]
struct ServerModifier;

impl Modify for ServerModifier {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // Update the info section
        openapi.info.title = "NeoMind API".to_string();
        openapi.info.version = "0.7.0".to_string();
        openapi.info.description = Some(
            "NeoMind Edge AI Agent API\n\n\
             ## Overview\n\n\
             NeoMind is an edge-deployed AI agent system with multi-backend LLM support,\n\
             device management, rule engine, and workflow automation capabilities.\n\n\
             ## Authentication\n\n\
             The API supports two authentication methods:\n\
             - **API Key**: Pass via `Authorization: Bearer <key>` header\n\
             - **JWT Token**: Obtain via `/api/auth/login`, pass via `Authorization: Bearer <token>` header\n\n\
             Public endpoints (health, read-only metadata) do not require authentication.\n\
             Protected endpoints require either an API key or JWT token.\n\
             Admin endpoints require JWT token with admin role.\n\n\
             ## Response Format\n\n\
             All endpoints return a unified response wrapper with `success`, `data`, and `meta` fields.\n\n\
             ## Pagination\n\n\
             List endpoints support `page` and `page_size` query parameters (default 20, max 100).\n\n\
             ## WebSocket\n\n\
             Real-time communication via WebSocket at `/api/chat`.\n\
             Event streaming via `/api/events/ws` or `/api/events/stream` (SSE).\n\n\
             ## Data Source IDs\n\n\
             Data sources use the format `{type}:{id}:{field}`:\n\
             - `device:sensor1:temperature` - Device metric\n\
             - `extension:weather:temp` - Extension metric\n\
             - `transform:converter:output` - Transform output\n\
             - `ai:demo:score` - AI agent metric"
                .to_string(),
        );

        // Add server info
        openapi.servers = Some(vec![utoipa::openapi::ServerBuilder::new()
            .url("/api")
            .description(Some("NeoMind API (relative)"))
            .build()]);
    }
}

/// NeoMind OpenAPI documentation.
#[derive(OpenApi)]
#[openapi(
    modifiers(&ServerModifier),
    paths(
        basic::health_handler,
        basic::health_status_handler,
        basic::liveness_handler,
        basic::readiness_handler,
        data::list_all_data_sources_handler,
        data::query_telemetry_handler,
    ),
    components(
        schemas(
            basic::HealthStatus,
            basic::ReadinessStatus,
            basic::DependencyStatus,
            data::UnifiedDataSourceInfo,
            data::ListDataSourcesQuery,
            data::ListDataSourcesResponse,
            data::TelemetryQueryParams,
        )
    ),
    tags(
        (name = "health", description = "Health check and system status"),
        (name = "devices", description = "Device management - CRUD, telemetry, commands"),
        (name = "device-types", description = "Device type templates and MDL management"),
        (name = "draft-devices", description = "Auto-onboarding draft device management"),
        (name = "extensions", description = "Extension management - dynamic plugins, metrics, commands"),
        (name = "data-sources", description = "Unified data source browsing and telemetry queries"),
        (name = "sessions", description = "Chat session management and history"),
        (name = "rules", description = "Rule engine - conditions, actions, triggers"),
        (name = "messages", description = "Alert and notification message management"),
        (name = "channels", description = "Message channel configuration (email, webhook, etc.)"),
        (name = "llm-backends", description = "LLM backend management (Ollama, llama.cpp, OpenAI)"),
        (name = "agents", description = "AI automation agents - scheduling, execution, memory"),
        (name = "automations", description = "Workflow automations and data transforms"),
        (name = "skills", description = "Agent skill templates and management"),
        (name = "memory", description = "System memory (markdown-based knowledge extraction)"),
        (name = "tools", description = "Available tools and capabilities"),
        (name = "dashboards", description = "Dashboard layout management"),
        (name = "stats", description = "System, device, and rule statistics"),
        (name = "mqtt", description = "MQTT broker management and subscriptions"),
        (name = "config", description = "Configuration import/export"),
        (name = "settings", description = "System settings (timezone, LLM generation)"),
        (name = "auth", description = "Authentication (API keys, JWT, user management)"),
        (name = "suggestions", description = "Intelligent input suggestions"),
    )
)]
pub struct ApiDoc;

/// Create the Swagger UI router for nesting.
pub fn swagger_ui() -> utoipa_swagger_ui::SwaggerUi {
    SwaggerUi::new("/api-docs{/spec}").url("/api/openapi.json", ApiDoc::openapi())
}

/// Handler that returns the OpenAPI JSON schema.
pub async fn openapi_json_handler() -> impl axum::response::IntoResponse {
    let json = ApiDoc::openapi()
        .to_pretty_json()
        .expect("Failed to serialize OpenAPI spec");

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json;charset=utf-8",
        )],
        json,
    )
}

/// Get the OpenAPI spec as JSON.
pub fn openapi_json() -> String {
    ApiDoc::openapi()
        .to_pretty_json()
        .expect("Failed to serialize OpenAPI spec")
}
