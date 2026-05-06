# LLM 模块

**包名**: `neomind-agent`
**版本**: 0.6.1
**完成度**: 95%
**用途**: 提供多后端LLM支持

## 概述

LLM模块实现了统一的LLM运行时接口，支持多种本地和云端LLM后端。

## 支持的后端

| 后端 | Feature | 状态 | 默认模型 |
|------|---------|------|----------|
| **Ollama** | `ollama` | ✅ 默认 | `qwen3.5:4b` |
| **llama.cpp** | `llamacpp` | ✅ | (服务端启动时加载) |
| **OpenAI** | `openai` | ✅ | `gpt-4o-mini` |
| **Anthropic** | `anthropic` | ✅ | `claude-3-5-sonnet-20241022` |
| **Google** | `google` | ✅ | `gemini-1.5-flash` |
| **xAI** | `xai` | ✅ | `grok-beta` |
| **Qwen (阿里云)** | `cloud` | ✅ | `qwen-max-latest` |
| **DeepSeek** | `cloud` | ✅ | `deepseek-v3` |
| **GLM (智谱)** | `cloud` | ✅ | `glm-4-plus` |
| **MiniMax** | `cloud` | ✅ | `m2-1-19b` |

> **注意**: Qwen、DeepSeek、GLM 和 MiniMax 使用 OpenAI 兼容 API，通过 `cloud` 特性启用。

## 模块结构

```
crates/neomind-agent/src/llm_backends/
├── mod.rs                      # 公开接口
├── backends/
│   ├── mod.rs                  # 后端工厂
│   ├── ollama.rs               # Ollama后端
│   ├── llamacpp.rs             # llama.cpp后端 (通过 /props 自动检测能力)
│   └── openai.rs               # 云端后端 (OpenAI/Anthropic/Google/xAI/Qwen/DeepSeek/GLM/MiniMax)
├── backend_plugin.rs           # 后端插件系统
├── config.rs                   # 配置定义
├── factories.rs                # 后端工厂
├── instance_manager.rs         # 实例管理
├── rate_limited_client.rs      # 限流客户端
└── tokenizer.rs                # 分词器包装
```

## Ollama 后端

### 特点

- 使用原生 `/api/chat` 端点（非 `/v1/chat/completions`）
- 支持 `thinking` 字段（推理模型）
- 支持流式输出
- 支持多模态输入

### 配置

```rust
pub struct OllamaConfig {
    /// 服务端点 (默认: http://localhost:11434)
    pub endpoint: String,

    /// 模型名称
    pub model: String,

    /// 请求超时（秒）
    pub timeout_secs: u64,

    /// 是否启用流式
    pub stream: bool,

    /// 额外参数
    pub options: HashMap<String, String>,
}
```

### API端点

```rust
// Ollama原生端点
POST /api/chat
Content-Type: application/json

{
    "model": "qwen3.5:4b",
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

## llama.cpp 后端 (LlamaCppRuntime)

直接集成 llama.cpp 独立服务器 (llama-server)。通过 `/props` 端点自动检测多模态、工具调用和上下文大小等能力。

### 特点

- 使用 OpenAI 兼容的 `/v1/chat/completions` 端点
- 自动从 `/props` 检测多模态支持、工具调用和上下文大小
- 支持流式输出
- 支持多模态输入（视觉模型如 llava、gemma-4 等）
- 本地实例无需 API 密钥

### 配置

```rust
pub struct LlamaCppConfig {
    /// 服务端点 (默认: http://127.0.0.1:8080)
    pub endpoint: String,

    /// 模型名称 (可选 — 模型在服务端启动时加载)
    pub model: String,

    /// 请求超时秒数 (默认: 180, 仅用于非流式请求)
    pub timeout_secs: u64,

    /// 可选的 Bearer token (--api-key 认证)
    pub api_key: Option<String>,

    /// 启用 KV 缓存复用 cache_prompt (默认: true)
    pub cache_prompt: bool,
}
```

### 自动检测

启动时，NeoMind 查询每个 llama.cpp 实例的 `/props` 端点来检测：

| 属性 | 来源 | 回退 |
|------|------|------|
| 多模态 (视觉) | `modalities.vision` | 模型名模式匹配 (`vl`、`llava`、`vision`) |
| 工具调用 | `chat_template_caps.supports_tools` | `true` |
| 上下文大小 | `default_generation_settings.n_ctx` | `4096` |

检测到的能力会持久化到存储并保持同步。

### 推荐服务端设置

多模态推理建议使用较大的上下文窗口：

```bash
llama-server -m model.gguf --ctx-size 32768 --port 8080
```

## 云端后端 (CloudRuntime)

统一的云端后端实现，支持多个提供商：

### 提供商配置

```rust
pub enum CloudProvider {
    OpenAI {
        api_key: String,
        base_url: Option<String>,  // 支持自定义端点
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

### 中国LLM提供商

NeoMind 原生支持中国主流LLM提供商：

| 提供商 | 端点 | 默认模型 | Vision支持 |
|--------|------|----------|------------|
| **Qwen (阿里云)** | `dashscope.aliyuncs.com` | `qwen-max-latest` | ✅ qwen-vl, qwen2.5-vl |
| **DeepSeek** | `api.deepseek.com` | `deepseek-v3` | ✅ deepseek-vl |
| **GLM (智谱)** | `open.bigmodel.cn` | `glm-4-plus` | ✅ glm-4v |
| **MiniMax** | `api.minimax.chat` | `m2-1-19b` | ✅ minimax-vl |

### 创建中国LLM后端

```rust
use neomind_agent::{CloudConfig, CloudProvider, CloudRuntime};

// Qwen (阿里云)
let qwen_config = CloudConfig::qwen("your-dashscope-api-key");
let qwen_runtime = CloudRuntime::new(qwen_config)?;

// DeepSeek
let deepseek_config = CloudConfig::deepseek("your-deepseek-api-key");
let deepseek_runtime = CloudRuntime::new(deepseek_config)?;

// GLM (智谱)
let glm_config = CloudConfig::glm("your-zhipu-api-key");
let glm_runtime = CloudRuntime::new(glm_config)?;

// MiniMax
let minimax_config = CloudConfig::minimax("your-minimax-api-key");
let minimax_runtime = CloudRuntime::new(minimax_config)?;
```
```

### API格式

```rust
// 统一请求格式
pub struct CloudRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<ToolDefinition>>,
}

// 统一响应格式
pub struct CloudResponse {
    pub content: String,
    pub thinking: Option<String>,  // Anthropic/Google 扩展
    pub finish_reason: String,
    pub usage: TokenUsage,
}
```

## 实例管理器

```rust
pub struct LlmBackendInstanceManager {
    // 后端实例
    instances: HashMap<BackendId, Arc<LlmBackendInstance>>,
    // 后端定义
    definitions: HashMap<String, BackendTypeDefinition>,
}
```

### 后端类型定义

```rust
pub struct BackendTypeDefinition {
    pub id: String,
    pub name: String,
    pub category: String,  // "local", "cloud"
    pub schema: serde_json::Value,  // 配置schema
    pub default_config: serde_json::Value,
}
```

## 使用示例

### 创建Ollama后端

```rust
use neomind_agent::{OllamaConfig, OllamaRuntime};
use neomind_agent::llm_backends::create_backend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 方式1: 直接创建
    let config = OllamaConfig {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen3.5:4b".to_string(),
        timeout_secs: 120,
        stream: true,
        options: Default::default(),
    };

    let runtime = OllamaRuntime::new(config)?;

    // 方式2: 使用工厂
    let runtime = create_backend("ollama", &serde_json::json!({
        "endpoint": "http://localhost:11434",
        "model": "qwen3.5:4b"
    }))?;

    Ok(())
}
```

### 创建云端后端

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

### 生成文本

```rust
use neomind-core::llm::backend::{LlmInput, LlmRuntime, GenerationParams};
use neomind-core::Message;

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

### 流式生成

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

## 后端能力

```rust
pub struct BackendCapabilities {
    /// 支持流式输出
    pub streaming: bool,

    /// 支持函数调用
    pub function_calling: bool,

    /// 支持视觉输入
    pub vision: bool,

    /// 支持thinking模式
    pub thinking: bool,
}

// 各后端能力
| 后端      | streaming | function_calling | vision | thinking |
|-----------|-----------|------------------|--------|----------|
| Ollama    | ✅        | ✅               | ✅     | ✅       |
| OpenAI    | ✅        | ✅               | ✅     | ❌       |
| Anthropic | ✅        | ✅               | ✅     | ✅       |
| Google    | ✅        | ✅               | ✅     | ✅       |
| xAI       | ✅        | ✅               | ✅     | ❌       |
| Qwen      | ✅        | ✅               | ✅     | ✅       |
| DeepSeek  | ✅        | ✅               | ✅     | ✅       |
| GLM       | ✅        | ✅               | ✅     | ✅       |
| MiniMax   | ✅        | ✅               | ✅     | ✅       |
```

### Vision模型检测

NeoMind 自动检测模型的Vision能力，支持以下模式：

| 提供商 | Vision模型模式 |
|--------|---------------|
| OpenAI | `gpt-4o`, `gpt-4-vision`, `gpt-4-turbo` |
| Anthropic | `claude-3`, `claude-4` |
| Google | `gemini` |
| Qwen | `qwen-vl`, `qwen2.5-vl`, `qwen3-vl`, `qwen3.5`, `qwen-max`, `qwen-plus` |
| DeepSeek | `deepseek-vl` |
| GLM | `glm-4v` |
| MiniMax | `minimax-vl` |
| xAI | `grok-vision` |
| 通用 | 包含 `vision`, `-vl`, `_vl` 关键词 |

## 限流

```rust
pub struct RateLimitedClient {
    inner: reqwest::Client,
    rate_limiter: RateLimiter,
}

pub struct RateLimiter {
    // 每分钟请求数
    requests_per_minute: u32,
    // 每日token数
    tokens_per_day: u64,
}
```

## 分词器

```rust
pub struct TokenizerWrapper {
    // 内部tokenizer
    inner: Option<Box<dyn Tokenizer>>,
}

pub trait Tokenizer {
    /// 计算token数
    fn count_tokens(&self, text: &str) -> usize;

    /// 计算消息token数
    fn count_message_tokens(&self, message: &Message) -> usize;
}
```

> **注意**: 当前candle分词器因依赖问题暂时禁用。

## 配置管理

```toml
# config.toml
[llm]
backend = "ollama"  # ollama, openai, anthropic, google, xai
model = "qwen3.5:4b"

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

## 环境变量

```bash
# LLM提供商
export LLM_PROVIDER=ollama

# Ollama
export OLLAMA_ENDPOINT=http://localhost:11434
export OLLAMA_MODEL=qwen3.5:4b

# OpenAI
export OPENAI_API_KEY=sk-...
export OPENAI_MODEL=gpt-4o-mini

# Anthropic
export ANTHROPIC_API_KEY=sk-ant-...
```

## 错误处理

```rust
pub enum LlmError {
    /// 请求失败
    RequestFailed(String),

    /// 响应解析失败
    ResponseParseError(String),

    /// API错误
    ApiError {
        status_code: u16,
        message: String,
    },

    /// 超时
    Timeout,

    /// 不支持的功能
    UnsupportedFeature(&'static str),

    /// 其他错误
    Other(anyhow::Error),
}
```

## API端点

```
GET  /api/llm-backends              # 列出所有后端
GET  /api/llm-backends/:id          # 获取后端详情
POST /api/llm-backends              # 创建后端
PUT  /api/llm-backends/:id          # 更新后端
DELETE /api/llm-backends/:id        # 删除后端
POST /api/llm-backends/:id/test     # 测试连接
GET  /api/llm-backends/stats        # 后端统计
GET  /api/llm-backends/types        # 可用后端类型
GET  /api/llm-backends/ollama/models # Ollama模型列表
```

## 设计原则

1. **统一接口**: 所有后端实现相同的 `LlmRuntime` trait
2. **Feature Flag**: 后端按需编译，减小二进制大小
3. **流式优先**: 默认支持流式输出
4. **错误恢复**: 自动重试和降级
5. **本地优先**: 默认使用Ollama本地部署
