# LLM Module

**Package**: `neomind-llm`
**Version**: 0.5.8
**Completion**: 90%
**Purpose**: Provides multi-backend LLM support

## Overview

The LLM module implements a unified LLM runtime interface, supporting multiple local and cloud-based LLM backends.

## Supported Backends

| Backend | Feature | Status | Default Model |
|------|---------|------|----------|
| **Ollama** | `ollama` | ✅ Default | `qwen3-vl:2b` |
| **OpenAI** | `openai` | ✅ | `gpt-4o-mini` |
| **Anthropic** | `anthropic` | ✅ | `claude-3-5-sonnet-20241022` |
| **Google** | `google` | ✅ | `gemini-2.0-flash` |
| **xAI** | `xai` | ✅ | `grok-beta` |

## Module Structure

```
crates/llm/src/
├── lib.rs                      # Public interface
├── backends/
│   ├── mod.rs                  # Backend factory
│   ├── ollama.rs               # Ollama backend
│   └── openai.rs               # Cloud backends (OpenAI/Anthropic/Google/xAI)
├── backend_plugin.rs           # Backend plugin system
├── config.rs                   # Configuration definitions
├── factories.rs                # Backend factories
├── instance_manager.rs         # Instance manager
├── rate_limited_client.rs      # Rate-limited client
└── tokenizer.rs                # Tokenizer wrapper
```

## Ollama Backend

### Features

- Uses native `/api/chat` endpoint (NOT `/v1/chat/completions`)
- Supports `thinking` field (for reasoning models)
- Supports streaming output
- Supports multi-modal input

### Configuration

```rust
pub struct OllamaConfig {
    /// Server endpoint (default: http://localhost:11434)
    pub endpoint: String,

    /// Model name
    pub model: String,

    /// Request timeout (seconds)
    pub timeout_secs: u64,

    /// Enable streaming
    pub stream: bool,

    /// Additional options
    pub options: HashMap<String, String>,
}
```

### API Endpoint

```rust
// Ollama native endpoint
POST /api/chat
Content-Type: application/json

{
    "model": "qwen3-vl:2b",
    "messages": [
        { "role": "user", "content": "Hello" }
    ],
    "stream": true,
    "options": {
        "temperature": 0.7,
        "num_predict": 2000
    }
}
```

## Cloud Backend (CloudRuntime)

Unified cloud backend implementation, supporting multiple providers:

### Provider Configuration

```rust
pub enum CloudProvider {
    OpenAI {
        api_key: String,
        base_url: Option<String>,  // Supports custom endpoint
        model: String,
    },
    Anthropic {
        api_key: String,
        model: String,
    },
    Google {
        api_key: String,
        model: String,
    },
    Xai {
        api_key: String,
        model: String,
    },
}
```

### API Format

```rust
// Unified request format
pub struct CloudRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<ToolDefinition>>,
}

// Unified response format
pub struct CloudResponse {
    pub content: String,
    pub thinking: Option<String>,  // Anthropic/Google extension
    pub finish_reason: String,
    pub usage: TokenUsage,
}
```

## Instance Manager

```rust
pub struct LlmBackendInstanceManager {
    // Backend instances
    instances: HashMap<BackendId, Arc<LlmBackendInstance>>,
    // Backend definitions
    definitions: HashMap<String, BackendTypeDefinition>,
}
```

### Backend Type Definition

```rust
pub struct BackendTypeDefinition {
    pub id: String,
    pub name: String,
    pub category: String,  // "local", "cloud"
    pub schema: serde_json::Value,  // Configuration schema
    pub default_config: serde_json::Value,
}
```

## Usage Examples

### Creating Ollama Backend

```rust
use neomind_llm::{OllamaConfig, OllamaRuntime, create_backend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Method 1: Direct creation
    let config = OllamaConfig {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen3-vl:2b".to_string(),
        timeout_secs: 120,
        stream: true,
        options: Default::default(),
    };

    let runtime = OllamaRuntime::new(config)?;

    // Method 2: Using factory
    let runtime = create_backend("ollama", &serde_json::json!({
        "endpoint": "http://localhost:11434",
        "model": "qwen3-vl:2b"
    }))?;

    Ok(())
}
```

### Creating Cloud Backend

```rust
use neomind_llm::{CloudConfig, CloudProvider, CloudRuntime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CloudConfig {
        provider: CloudProvider::OpenAI {
            api_key: "sk-...".to_string(),
            base_url: None,
            model: "gpt-4o-mini".to_string(),
        },
        timeout_secs: 60,
    };

    let runtime = CloudRuntime::new(config)?;
    Ok(())
}
```

### Generating Text

```rust
use neomind_core::llm::backend::{LlmInput, LlmRuntime, GenerationParams};
use neomind_core::Message;

async fn generate(runtime: &dyn LlmRuntime, prompt: &str) -> Result<String> {
    let input = LlmInput {
        messages: vec![Message::user(prompt)],
        params: GenerationParams {
            temperature: Some(0.7),
            max_tokens: Some(2000),
            ..Default::default()
        },
        model: None,
    };

    let output = runtime.generate(&input)?;
    Ok(output.text)
}
```

### Streaming Generation

```rust
use futures::StreamExt;

async fn generate_stream(runtime: &dyn LlmRuntime, prompt: &str) -> Result<String> {
    let input = LlmInput {
        messages: vec![Message::user(prompt)],
        params: GenerationParams::default(),
        model: None,
    };

    let mut stream = runtime.generate_stream(&input)?;
    let mut full_text = String::new();

    while let Some(chunk) = stream.next().await {
        let (content, is_thinking) = chunk?;
        if !is_thinking {
            full_text.push_str(&content);
            print!("{}", content);
            std::io::stdout().flush()?;
        }
    }

    Ok(full_text)
}
```

## Backend Capabilities

```rust
pub struct BackendCapabilities {
    /// Supports streaming output
    pub streaming: bool,

    /// Supports function calling
    pub function_calling: bool,

    /// Supports vision input
    pub vision: bool,

    /// Supports thinking mode
    pub thinking: bool,
}

// Capability matrix by backend
| Backend   | streaming | function_calling | vision | thinking |
|-----------|-----------|------------------|--------|----------|
| Ollama     | ✅        | ✅               | ✅     | ✅       |
| OpenAI     | ✅        | ✅               | ✅     | ❌       |
| Anthropic  | ✅        | ✅               | ✅     | ✅       |
| Google     | ✅        | ✅               | ✅     | ✅       |
| xAI        | ✅        | ✅               | ❌     | ❌       |
```

## Rate Limiting

```rust
pub struct RateLimitedClient {
    inner: reqwest::Client,
    rate_limiter: RateLimiter,
}

pub struct RateLimiter {
    // Requests per minute
    pub requests_per_minute: u32,
    // Tokens per day
    pub tokens_per_day: u64,
}
```

## Tokenizer

```rust
pub struct TokenizerWrapper {
    // Internal tokenizer
    inner: Option<Box<dyn Tokenizer>>,
}

pub trait Tokenizer {
    /// Count tokens
    fn count_tokens(&self, text: &str) -> usize;

    /// Count message tokens
    fn count_message_tokens(&self, message: &Message) -> usize;
}
```

> **Note**: The candle tokenizer is currently disabled due to dependency issues.

## Configuration Management

```toml
# config.toml
[llm]
backend = "ollama"  # ollama, openai, anthropic, google, xai
model = "qwen3-vl:2b"

[llm.ollama]
endpoint = "http://localhost:11434"

[llm.openai]
api_key = "sk-..."
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"

[llm.anthropic]
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"
```

## Environment Variables

```bash
# LLM Provider
export LLM_PROVIDER=ollama

# Ollama
export OLLAMA_ENDPOINT=http://localhost:11434
export OLLAMA_MODEL=qwen3-vl:2b

# OpenAI
export OPENAI_API_KEY=sk-...
export OPENAI_MODEL=gpt-4o-mini

# Anthropic
export ANTHROPIC_API_KEY=sk-ant-...
```

## Error Handling

```rust
pub enum LlmError {
    /// Request failed
    RequestFailed(String),

    /// Response parsing failed
    ResponseParseError(String),

    /// API error
    ApiError {
        status_code: u16,
        message: String,
    },

    /// Timeout
    Timeout,

    /// Unsupported feature
    UnsupportedFeature(&'static str),

    /// Other error
    Other(anyhow::Error),
}
```

## API Endpoints

```
GET  /api/llm-backends              # List backends
GET  /api/llm-backends/:id          # Get backend details
POST /api/llm-backends              # Create backend
PUT  /api/llm-backends/:id          # Update backend
DELETE /api/llm-backends/:id        # Delete backend
POST /api/llm-backends/:id/test     # Test connection
GET  /api/llm-backends/stats        # Backend statistics
GET  /api/llm-backends/types        # Available backend types
GET  /api/llm-backends/ollama/models # Ollama model list
```

## Design Principles

1. **Unified Interface**: All backends implement the same `LlmRuntime` trait
2. **Feature Flag**: Backends compiled by features to reduce binary size
3. **Streaming-First**: Streaming enabled by default
4. **Error Recovery**: Automatic retry and fallback
5. **Local-First**: Ollama local deployment by default
