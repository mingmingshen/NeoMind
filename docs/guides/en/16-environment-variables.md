# Environment Variables Reference

Complete list of environment variables supported by NeoMind, organized by subsystem.

> **Quick start**: Most variables are optional with sensible defaults. Production deployments should set `NEOMIND_JWT_SECRET` and `NEOMIND_ENCRYPTION_KEY`.

---

## Server

| Variable | Default | Description |
|----------|---------|-------------|
| `NEOMIND_DATA_DIR` | `data` | Root directory for all database files (redb) |
| `NEOMIND_HOST` | `0.0.0.0` | Server bind host |
| `NEOMIND_PORT` | `9375` | Server bind port |
| `NEOMIND_WEB_DIR` | `/var/www/neomind` | Frontend static files directory (non-embedded mode only) |
| `NEOMIND_PUBLIC_HOST` | Auto-detected | Public hostname or IP for generating callback URLs (webhook, device onboarding). Falls back to auto-detected LAN IP |
| `NEOMIND_SERVER_URL` | `http://localhost:9375` | Server base URL used for device webhook callback generation |

## Authentication & Encryption

| Variable | Default | Description |
|----------|---------|-------------|
| `NEOMIND_JWT_SECRET` | Random (per start) | JWT signing secret. **Set this in production** to keep sessions valid across restarts |
| `NEOMIND_ENCRYPTION_KEY` | Auto-generated | Data encryption key. Stored in `data/encryption_key` if not set |
| `NEOMIND_API_KEY` | — | API key for CLI authentication |
| `NEOMIND_KEY_CIPHER` | Built-in | XOR cipher key for API key obfuscation in transit (shared with frontend). Override for production deployments |

## Logging

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error`. Accepts module filters (e.g. `neomind=debug,tower_http=info`) |
| `NEOMIND_LOG_JSON` | — | Set to `true` for JSON-formatted log output |
| `NEOMIND_COLOR` | — | Set to `true` to force colored terminal output |
| `NO_COLOR` | — | Standard env var to disable colored terminal output |
| `TZ` | System timezone | Container timezone (e.g. `Asia/Shanghai`, `America/New_York`) |

## Agent & LLM

| Variable | Default | Description |
|----------|---------|-------------|
| `LLM_PROVIDER` | `ollama` | Default LLM provider for `neomind setup`: `ollama`, `openai` |
| `LLM_MODEL` | Provider default | Model name (e.g. `qwen3.5:4b`, `gpt-4o-mini`) |
| `OLLAMA_ENDPOINT` | `http://localhost:11434` | Ollama API endpoint |
| `OPENAI_API_KEY` | — | OpenAI-compatible API key |
| `OPENAI_ENDPOINT` | `https://api.openai.com/v1` | OpenAI-compatible API endpoint |
| `NEOMIND_STREAM_TIMEOUT` | `1200` (20 min) | Agent streaming response timeout (seconds) |
| `NEOMIND_HEARTBEAT_INTERVAL` | `30` | WebSocket heartbeat interval (seconds) |
| `NEOMIND_MAX_TOOL_ITERATIONS` | `20` | Maximum tool-calling iterations per agent turn |
| `NEOMIND_ALLOWED_WRITE_DIRS` | — | Colon-separated list of directories the shell tool is allowed to write to |
| `AGENT_MAX_CONTEXT_TOKENS` | `128000` | Maximum context window size (tokens) for agent LLM calls |
| `NEOMIND_MAX_CONTEXT` | Unlimited | Global upper limit for model context length. Caps `num_ctx` (Ollama) and `max_context` capability to prevent OOM on constrained hardware |
| `AGENT_MAX_TOKENS` | `4096` | Maximum tokens to generate per LLM response |
| `AGENT_TEMPERATURE` | `0.3` | LLM sampling temperature (lower = more deterministic) |
| `AGENT_TOP_P` | `0.7` | LLM top-p (nucleus) sampling parameter |
| `AGENT_CONCURRENT_LIMIT` | `3` | Maximum concurrent LLM requests from the agent |
| `AGENT_CONTEXT_SELECTOR_TOKENS` | `4000` | Token budget for context selector (decides which context to inject) |
| `AGENT_LLM_TIMEOUT_SECS` | — | Unified LLM request timeout (seconds). When set, applies to both Ollama (default 120s) and cloud (default 60s) backends via `agent_env_vars` |
| `OLLAMA_TIMEOUT_SECS` | `120` | Ollama backend request timeout (seconds) |
| `OPENAI_TIMEOUT_SECS` | `60` | OpenAI backend request timeout (seconds) |
| `ANTHROPIC_TIMEOUT_SECS` | `60` | Anthropic backend request timeout (seconds) |
| `GOOGLE_TIMEOUT_SECS` | `60` | Google (Gemini) backend request timeout (seconds) |
| `XAI_TIMEOUT_SECS` | `60` | xAI (Grok) backend request timeout (seconds) |
| `QWEN_TIMEOUT_SECS` | `60` | Qwen (DashScope) backend request timeout (seconds) |
| `DEEPSEEK_TIMEOUT_SECS` | `60` | DeepSeek backend request timeout (seconds) |
| `GLM_TIMEOUT_SECS` | `60` | GLM (ZhipuAI) backend request timeout (seconds) |
| `MINIMAX_TIMEOUT_SECS` | `60` | MiniMax backend request timeout (seconds) |
| `LLAMACPP_TIMEOUT_SECS` | `180` | llama.cpp server backend request timeout (seconds) |

## Extension Runtime

| Variable | Default | Description |
|----------|---------|-------------|
| `NEOMIND_FFI_TIMEOUT_SECS` | `120` | IPC-level FFI call timeout in seconds (minimum 10). Extensions have three timeout layers: FFI (this var, 120s), process command (300s, via API), and agent tool (300s, hardcoded) |
| `NEOMIND_IPC_MAX_SIZE` | `10485760` (10 MB) | Maximum IPC message size in bytes |
| `NEOMIND_WASM_MEMORY_MB` | `256` | WASM linear memory limit per extension (process-level limit is 2048 MB via IsolatedExtensionConfig) |
| `NEOMIND_WASM_MAX_SIZE_MB` | `50` | Maximum WASM binary file size |
| `NEOMIND_WASM_FUEL` | `1000000` | WASM execution fuel limit (prevents infinite loops) |
| `NEOMIND_MARKET_URL` | GitHub raw URL | Dashboard component marketplace base URL. Override to use a mirror (e.g. GitHub proxy for China) |

## CLI

| Variable | Default | Description |
|----------|---------|-------------|
| `NEOMIND_API_BASE` | `http://localhost:9375/api` | API base URL for CLI commands |
| `NEOMIND_JSON` | — | Set to any value to force JSON output format |

## Frontend (Vite)

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_API_BASE_URL` | — | API base URL for cross-origin deployment (e.g. `https://api.example.com/api`). Build-time only |
| `VITE_API_TARGET` | `http://127.0.0.1:9375` | Vite dev server proxy target |

## Docker

| Variable | Default | Description |
|----------|---------|-------------|
| `NEOMIND_HTTP_PORT` | `9375` | Host port mapping for HTTP/WebSocket |
| `NEOMIND_MQTT_PORT` | `1883` | Host port mapping for MQTT broker |

---

## Docker Compose Example

```bash
# .env file
NEOMIND_HTTP_PORT=9375
NEOMIND_MQTT_PORT=1883
RUST_LOG=neomind=info
TZ=Asia/Shanghai
NEOMIND_JWT_SECRET=your-random-secret-here
NEOMIND_ENCRYPTION_KEY=0123456789abcdef0123456789abcdef
```

## Cross-Origin Deployment

When frontend and backend are on different domains:

```bash
# Build frontend with custom API URL
VITE_API_BASE_URL=https://api.example.com/api npm run build
```

## CLI Configuration

```bash
# Use CLI against a remote server
export NEOMIND_API_BASE=https://api.example.com/api
neomind device list

# JSON output for scripting
export NEOMIND_JSON=1
neomind device list | jq '.[0].name'
```

---

See also: [Installation Guide](../user-guide/en/01-installation.md) | [Docker Deployment](../../deploy/README.md)
