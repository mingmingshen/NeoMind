# 环境变量参考

NeoMind 支持的完整环境变量列表，按子系统分类。

> **快速上手**：大多数变量为可选，有合理默认值。生产环境请务必设置 `NEOMIND_JWT_SECRET` 和 `NEOMIND_ENCRYPTION_KEY`。

---

## 服务器

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `NEOMIND_DATA_DIR` | `data` | 所有数据库文件的根目录（redb） |
| `NEOMIND_HOST` | `0.0.0.0` | 服务器绑定主机 |
| `NEOMIND_PORT` | `9375` | 服务器绑定端口 |
| `NEOMIND_WEB_DIR` | `/var/www/neomind` | 前端静态文件目录（仅非嵌入模式） |
| `NEOMIND_PUBLIC_HOST` | 自动检测 | 公网主机名或 IP，用于生成回调 URL（Webhook、设备接入）。未设置时自动检测局域网 IP |
| `NEOMIND_SERVER_URL` | `http://localhost:9375` | 服务器基础 URL，用于设备 Webhook 回调地址生成 |

## 认证与加密

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `NEOMIND_JWT_SECRET` | 随机（每次启动） | JWT 签名密钥。**生产环境务必设置**，确保重启后会话有效 |
| `NEOMIND_ENCRYPTION_KEY` | 自动生成 | 数据加密密钥。未设置时存储在 `data/encryption_key` |
| `NEOMIND_API_KEY` | — | CLI 认证 API 密钥 |
| `NEOMIND_KEY_CIPHER` | 内置 | API 密钥传输混淆的 XOR 密钥（与前端共享）。生产环境建议自定义 |

## 日志

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RUST_LOG` | `info` | 日志级别：`trace`、`debug`、`info`、`warn`、`error`。支持模块过滤（如 `neomind=debug,tower_http=info`） |
| `NEOMIND_LOG_JSON` | — | 设为 `true` 启用 JSON 格式日志 |
| `NEOMIND_COLOR` | — | 设为 `true` 强制启用彩色终端输出 |
| `NO_COLOR` | — | 标准环境变量，禁用彩色终端输出 |
| `TZ` | 系统时区 | 容器时区（如 `Asia/Shanghai`、`America/New_York`） |

## 智能体与 LLM

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `LLM_PROVIDER` | `ollama` | `neomind setup` 默认 LLM 提供商：`ollama`、`openai` |
| `LLM_MODEL` | 提供商默认 | 模型名称（如 `qwen3.5:4b`、`gpt-4o-mini`） |
| `OLLAMA_ENDPOINT` | `http://localhost:11434` | Ollama API 端点 |
| `OPENAI_API_KEY` | — | OpenAI 兼容 API 密钥 |
| `OPENAI_ENDPOINT` | `https://api.openai.com/v1` | OpenAI 兼容 API 端点 |
| `NEOMIND_STREAM_TIMEOUT` | `1200`（20 分钟） | 智能体流式响应超时（秒） |
| `NEOMIND_HEARTBEAT_INTERVAL` | `30` | WebSocket 心跳间隔（秒） |
| `NEOMIND_MAX_TOOL_ITERATIONS` | `20` | 每次智能体轮次最大工具调用迭代次数 |
| `NEOMIND_ALLOWED_WRITE_DIRS` | — | Shell 工具允许写入的目录列表（冒号分隔） |
| `AGENT_MAX_CONTEXT_TOKENS` | `128000` | 智能体 LLM 调用的最大上下文窗口大小（token 数） |
| `NEOMIND_MAX_CONTEXT` | 无限制 | 模型上下文长度全局上限。限制 Ollama `num_ctx` 和 `max_context` 能力值，防止低内存设备 OOM |
| `AGENT_MAX_TOKENS` | `4096` | 每次 LLM 响应最大生成 token 数 |
| `AGENT_TEMPERATURE` | `0.3` | LLM 采样温度（越低越确定性） |
| `AGENT_TOP_P` | `0.7` | LLM top-p（核采样）参数 |
| `AGENT_CONCURRENT_LIMIT` | `3` | 智能体最大并发 LLM 请求数 |
| `AGENT_CONTEXT_SELECTOR_TOKENS` | `4000` | 上下文选择器的 token 预算（决定注入哪些上下文） |
| `AGENT_LLM_TIMEOUT_SECS` | — | 统一 LLM 请求超时（秒）。设置后同时应用于 Ollama（默认 120s）和云端（默认 60s）后端（通过 `agent_env_vars`） |
| `OLLAMA_TIMEOUT_SECS` | `120` | Ollama 后端请求超时（秒） |
| `OPENAI_TIMEOUT_SECS` | `60` | OpenAI 后端请求超时（秒） |
| `ANTHROPIC_TIMEOUT_SECS` | `60` | Anthropic 后端请求超时（秒） |
| `GOOGLE_TIMEOUT_SECS` | `60` | Google（Gemini）后端请求超时（秒） |
| `XAI_TIMEOUT_SECS` | `60` | xAI（Grok）后端请求超时（秒） |
| `QWEN_TIMEOUT_SECS` | `60` | Qwen（通义千问）后端请求超时（秒） |
| `DEEPSEEK_TIMEOUT_SECS` | `60` | DeepSeek 后端请求超时（秒） |
| `GLM_TIMEOUT_SECS` | `60` | GLM（智谱 AI）后端请求超时（秒） |
| `MINIMAX_TIMEOUT_SECS` | `60` | MiniMax 后端请求超时（秒） |
| `LLAMACPP_TIMEOUT_SECS` | `180` | llama.cpp server 后端请求超时（秒） |

## 扩展运行时

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `NEOMIND_FFI_TIMEOUT_SECS` | `120` | IPC 层 FFI 调用超时（秒，最小值 10）。扩展超时分层：FFI（此变量，120s）、进程命令（300s，通过 API）、智能体工具（300s，硬编码） |
| `NEOMIND_IPC_MAX_SIZE` | `10485760`（10 MB） | IPC 消息最大字节数 |
| `NEOMIND_WASM_MEMORY_MB` | `256` | 每个扩展的 WASM 线性内存限制（进程级限制为 2048 MB，通过 IsolatedExtensionConfig 配置） |
| `NEOMIND_WASM_MAX_SIZE_MB` | `50` | WASM 二进制文件最大大小 |
| `NEOMIND_WASM_FUEL` | `1000000` | WASM 执行燃料限制（防止无限循环） |
| `NEOMIND_MARKET_URL` | GitHub raw URL | 仪表板组件市场基础 URL。可设为镜像地址（如国内 GitHub 代理） |

## CLI

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `NEOMIND_API_BASE` | `http://localhost:9375/api` | CLI 命令的 API 基础 URL |
| `NEOMIND_JSON` | — | 设为任意值强制 JSON 输出格式 |

## 前端（Vite）

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `VITE_API_BASE_URL` | — | 跨域部署时的 API 基础 URL（如 `https://api.example.com/api`）。仅构建时生效 |
| `VITE_API_TARGET` | `http://127.0.0.1:9375` | Vite 开发服务器代理目标 |

## Docker

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `NEOMIND_HTTP_PORT` | `9375` | 主机端口映射（HTTP/WebSocket） |
| `NEOMIND_MQTT_PORT` | `1883` | 主机端口映射（MQTT Broker） |

---

## Docker Compose 示例

```bash
# .env 文件
NEOMIND_HTTP_PORT=9375
NEOMIND_MQTT_PORT=1883
RUST_LOG=neomind=info
TZ=Asia/Shanghai
NEOMIND_JWT_SECRET=your-random-secret-here
NEOMIND_ENCRYPTION_KEY=0123456789abcdef0123456789abcdef
```

## 跨域部署

前端与后端在不同域名时：

```bash
# 构建前端时指定 API URL
VITE_API_BASE_URL=https://api.example.com/api npm run build
```

## CLI 配置

```bash
# 连接远程服务器
export NEOMIND_API_BASE=https://api.example.com/api
neomind device list

# JSON 输出（用于脚本）
export NEOMIND_JSON=1
neomind device list | jq '.[0].name'
```

---

参见：[安装指南](../user-guide/zh/01-installation.md) | [Docker 部署](../../deploy/README.md)
