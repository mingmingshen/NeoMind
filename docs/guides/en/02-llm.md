# LLM Module

**Package**: `neomind-agent`
**Version**: 0.6.1
**Completion**: 95%
**Purpose**: Provides multi-backend LLM support

## Overview

The LLM module implements a unified LLM runtime interface, supporting multiple local and cloud-based LLM backends.

## Supported Backends

| Backend | Feature | Status | Default Model |
|------|---------|------|----------|
| **Ollama** | `ollama` | ✅ Default | `qwen3-vl:2b` |
| **llama.cpp** | `llamacpp` | ✅ | (loaded at server startup) |
| **OpenAI** | `openai` | ✅ | `gpt-4o-mini` |
| **Anthropic** | `anthropic` | ✅ | `claude-3-5-sonnet-20241022` |
| **Google** | `google` | ✅ | `gemini-1.5-flash` |
| **xAI** | `xai` | ✅ | `grok-beta` |
| **Qwen (Alibaba)** | `cloud` | ✅ | `qwen-max-latest` |
| **DeepSeek** | `cloud` | ✅ | `deepseek-v3` |
| **GLM (Zhipu)** | `cloud` | ✅ | `glm-4-plus` |
| **MiniMax** | `cloud` | ✅ | `m2-1-19b` |

> **Note**: Qwen, DeepSeek, GLM, and MiniMax use OpenAI-compatible APIs and are enabled via the `cloud` feature.

## Module Structure

```
crates/neomind-agent/src/llm_backends/
├── mod.rs                      # Public interface
├── backends/
│   ├── mod.rs                  # Backend factory
│   ├── ollama.rs               # Ollama backend
│   ├── llamacpp.rs             # llama.cpp backend (auto-detect capabilities via /props)
│   └── openai.rs               # Cloud backends (OpenAI/Anthropic/Google/xAI/Qwen/DeepSeek/GLM/MiniMax)
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

## llama.cpp Backend (LlamaCppRuntime)

Direct integration with llama.cpp standalone server (llama-server). Capabilities (multimodal, tools, context size) are auto-detected via the `/props` endpoint.

### Features

- Uses OpenAI-compatible `/v1/chat/completions` endpoint
- Auto-detects multimodal support, tool calling, and context size from `/props`
- Supports streaming output
- Supports multi-modal input (vision models like llava, gemma-4, etc.)
- No API key required for local instances

### Configuration

```rust
pub struct LlamaCppConfig {
    /// Server endpoint (default: http://127.0.0.1:8080)
    pub endpoint: String,

    /// Model name (optional — model is loaded at server startup)
    pub model: String,

    /// Request timeout in seconds (default: 180, used for non-streaming only)
    pub timeout_secs: u64,

    /// Optional Bearer token for --api-key authentication
    pub api_key: Option<String>,

    /// Enable KV cache reuse via cache_prompt (default: true)
    pub cache_prompt: bool,
}
```

### Auto-Detection

On startup, NeoMind queries each llama.cpp instance's `/props` endpoint to detect:

| Property | Source | Fallback |
|----------|--------|----------|
| Multimodal (vision) | `modalities.vision` | Model name patterns (`vl`, `llava`, `vision`) |
| Tool calling | `chat_template_caps.supports_tools` | `true` |
| Context size | `default_generation_settings.n_ctx` | `4096` |

Detected capabilities are persisted to storage and kept in sync.

### Recommended Server Settings

For multimodal inference, use a larger context window:

```bash
llama-server -m model.gguf --ctx-size 32768 --port 8080
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
    Grok {
        api_key: String,
        model: String,
    },
    Qwen {
        api_key: String,
        model: String,
    },
    DeepSeek {
        api_key: String,
        model: String,
    },
    GLM {
        api_key: String,
        model: String,
    },
    MiniMax {
        api_key: String,
        model: String,
    },
    Custom {
        api_key: String,
        base_url: String,
        model: String,
    },
}
```

### Chinese LLM Providers

NeoMind natively supports major Chinese LLM providers:

| Provider | Endpoint | Default Model | Vision Support |
|----------|----------|---------------|----------------|
| **Qwen (Alibaba)** | `dashscope.aliyuncs.com` | `qwen-max-latest` | ✅ qwen-vl, qwen2.5-vl |
| **DeepSeek** | `api.deepseek.com` | `deepseek-v3` | ✅ deepseek-vl |
| **GLM (Zhipu)** | `open.bigmodel.cn` | `glm-4-plus` | ✅ glm-4v |
| **MiniMax** | `api.minimax.chat` | `m2-1-19b` | ✅ minimax-vl |

### Creating Chinese LLM Backends

```rust
use neomind_agent::{CloudConfig, CloudRuntime};

// Qwen (Alibaba)
let qwen_config = CloudConfig::qwen("your-dashscope-api-key");
let qwen_runtime = CloudRuntime::new(qwen_config)?;

// DeepSeek
let deepseek_config = CloudConfig::deepseek("your-deepseek-api-key");
let deepseek_runtime = CloudRuntime::new(deepseek_config)?;

// GLM (Zhipu)
let glm_config = CloudConfig::glm("your-zhipu-api-key");
let glm_runtime = CloudRuntime::new(glm_config)?;

// MiniMax
let minimax_config = CloudConfig::minimax("your-minimax-api-key");
let minimax_runtime = CloudRuntime::new(minimax_config)?;
```
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
use neomind_agent::{OllamaConfig, OllamaRuntime};
use neomind_agent::llm_backends::create_backend;

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
use neomind_agent::{CloudConfig, CloudProvider, CloudRuntime};

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
| xAI        | ✅        | ✅               | ✅     | ❌       |
| Qwen       | ✅        | ✅               | ✅     | ✅       |
| DeepSeek   | ✅        | ✅               | ✅     | ✅       |
| GLM        | ✅        | ✅               | ✅     | ✅       |
| MiniMax    | ✅        | ✅               | ✅     | ✅       |
```

### Vision Model Detection

NeoMind automatically detects vision capabilities based on model name patterns:

| Provider | Vision Model Patterns |
|----------|----------------------|
| OpenAI | `gpt-4o`, `gpt-4-vision`, `gpt-4-turbo` |
| Anthropic | `claude-3`, `claude-4` |
| Google | `gemini` |
| Qwen | `qwen-vl`, `qwen2.5-vl`, `qwen3-vl`, `qwen-max`, `qwen-plus` |
| DeepSeek | `deepseek-vl` |
| GLM | `glm-4v` |
| MiniMax | `minimax-vl` |
| xAI | `grok-vision` |
| Generic | Contains `vision`, `-vl`, `_vl` keywords |

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
