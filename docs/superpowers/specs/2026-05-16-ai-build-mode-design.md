# AI Build Mode — Design Spec

> Version: 0.8 Major Feature | Date: 2026-05-16 | Status: Approved

## 1. Overview

AI Build Mode is the flagship feature of NeoMind 0.8. It enables users to build, configure, and manage the entire platform through natural language conversations in Chat. The AI agent executes CLI commands via the existing ShellTool to perform all operations — from device onboarding to dashboard creation, rule automation, extension development, and widget component building.

### Core Principle

**CLI is the single source of truth.** All product capabilities are exposed through `neomind` CLI commands. Both humans and AI share the same command set. The AI agent uses ShellTool as its sole execution channel — no separate Tool definitions per capability domain.

### Design Goals

1. **Natural language to action** — User describes intent, AI translates to CLI commands and executes
2. **CLI as first-class citizen** — Every operation works via CLI, independent of AI
3. **Structured output** — All CLI commands support `--json` for reliable AI and frontend parsing
4. **Rich card experience** — Build results displayed as interactive cards in Chat, not raw text
5. **Incremental delivery** — CLI commands can be added one by one, each immediately usable

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Chat UI (Frontend)                     │
│                                                              │
│  User: "帮我创建一个温度监控系统"                              │
│  AI:   [calls ShellTool] → neomind device create ...         │
│  ┌───────────────────────────────────────────────┐           │
│  │  BuildCard: 5 steps completed                  │           │
│  │  ✅ device type  ✅ device  ✅ dashboard        │           │
│  │  ✅ widget       ✅ alert rule                  │           │
│  │  [查看仪表盘]  [查看规则]  [全部撤销]           │           │
│  └───────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Agent (neomind-agent)                       │
│                                                              │
│  Tools:                                                      │
│  ├── shell      (ALL operations via CLI)                     │
│  ├── think      (memory storage/retrieval)                   │
│  ├── ask_user   (clarification/confirmation)                 │
│  └── {ext}:{cmd} (extension dynamic commands)               │
│                                                              │
│  System Prompt: includes full CLI command reference          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  CLI (neomind-cli)                            │
│                                                              │
│  All commands call local API Server via HTTP                 │
│  All commands support --json for structured output           │
│  All commands support --help for AI self-discovery           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              API Server (neomind-api, Axum)                  │
│  Existing REST API — no new endpoints needed                 │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User natural language
  → LLM reasoning + CLI command reference
  → ShellTool("neomind <domain> <action> <args> --json")
  → CLI executes HTTP call to local API Server
  → JSON result: { success, data, message, build_meta? }
  → Agent returns result to frontend
  → BuildCard renders rich card based on build_meta
```

## 3. CLI Command Specification

### 3.1 Common Flags

All commands support:
- `--json` — Structured JSON output (for AI and programmatic use)
- `--help` — Detailed usage information (for AI self-discovery)

### 3.2 JSON Output Format

Success:
```json
{
  "success": true,
  "data": { ... },
  "message": "Human-readable summary"
}
```

Failure:
```json
{
  "success": false,
  "error": "Human-readable error",
  "code": "ERROR_CODE"
}
```

Build commands additionally return:
```json
{
  "success": true,
  "data": { "id": "...", ... },
  "message": "...",
  "build_meta": {
    "type": "device | dashboard | rule | extension | widget | transform",
    "action": "create | update | delete",
    "entity_id": "...",
    "entity_name": "...",
    "undo_command": "neomind device delete xxx --force"
  }
}
```

### 3.3 Device Commands

```
neomind device list [--type <type>] [--status <status>] [--json]
neomind device get <id> [--json]
neomind device create --name <name> --type <type-template-id> --adapter <mqtt|http|webhook> [--config <json>] [--json]
neomind device update <id> [--name <name>] [--config <json>] [--json]
neomind device delete <id> [--force]
neomind device latest <id> [--metric <name>] [--json]
neomind device history <id> --metric <name> --time-range <range> [--json]
neomind device control <id> --command <cmd> [--params <json>] [--confirm] [--json]
neomind device types list [--json]
neomind device types get <type-id> [--json]
neomind device types create --name <name> --metrics <json-or-file> [--commands <json-or-file>] [--json]
neomind device types update <type-id> [--name <name>] [--metrics <json-or-file>] [--commands <json-or-file>] [--json]
neomind device types delete <type-id> [--force]
```

**Device type definitions must conform to**: [NeoMind-DeviceTypes](../NeoMind-DeviceTypes) repository standards:
- Required fields: `device_type`, `name`, `metrics`
- Metric schema: `name`, `display_name`, `data_type`, `unit`, `min`, `max`, `required`
- Command schema: `name`, `display_name`, `description`, `payload_template`, `parameters`
- Dot notation for nested fields (e.g., `values.battery`)

### 3.4 Dashboard Commands

```
neomind dashboard list [--json]
neomind dashboard get <id> [--json]
neomind dashboard create --name <name> [--description <desc>] [--layout <json>] [--json]
neomind dashboard update <id> [--name <name>] [--description <desc>] [--layout <json>] [--json]
neomind dashboard delete <id>
neomind dashboard add-widget <id> --type <widget-type> [--datasource <source-id>] [--config <json>] [--json]
neomind dashboard remove-widget <id> --widget-id <widget-id>
neomind dashboard share <id> [--public] [--expires <duration>] [--json]
```

### 3.5 Widget (Dashboard Component) Commands

```
neomind widget list [--json]
neomind widget get <id> [--json]
neomind widget create --name <name> --type <chart|gauge|stat|table|image|custom> [--json]
neomind widget build [--path <dir>]
neomind widget dev [--path <dir>]
neomind widget install <path-or-url>
neomind widget uninstall <id>
neomind widget publish [--path <dir>]
```

**Widget components must conform to**: [NeoMind-Dashboard-Components](../NeoMind-Dashboard-Components) repository standards:
- Directory: `{component_id}/manifest.json` + `bundle.js` (IIFE format)
- Manifest required fields: `id`, `name` (en/zh), `description` (en/zh), `icon`, `category`, `global_name`, `export_name`
- Bundle uses `window.React` and `window.jsxRuntime` only
- Styling: CSS variables only (never hardcoded values), follow `STYLE_GUIDE.md`
- Container: must fill `w-full h-full`

### 3.6 Rule Commands

```
neomind rule list [--json]
neomind rule get <id> [--json]
neomind rule create --name <name> --trigger <json-or-file> --actions <json-or-file> [--condition <json-or-file>] [--json]
neomind rule update <id> [--name <name>] [--trigger <json-or-file>] [--actions <json-or-file>] [--condition <json-or-file>] [--json]
neomind rule delete <id>
neomind rule enable <id>
neomind rule disable <id>
neomind rule history <id> [--time-range <range>] [--json]
neomind rule test <id> --input <json> [--json]
```

### 3.7 Transform Commands

```
neomind transform list [--json]
neomind transform get <id> [--json]
neomind transform create --name <name> --input <source-id> --expression <expr> --output <field> [--json]
neomind transform update <id> [--name <name>] [--expression <expr>] [--json]
neomind transform delete <id>
```

### 3.8 Extension Commands

```
neomind extension list [--json]
neomind extension get <id> [--json]
neomind extension status <id> [--json]
neomind extension logs <id> [--follow] [--json]
neomind extension create <name> --type <type> [--output <dir>]
neomind extension build [--path <dir>] [--release] [--version <ver>]
neomind extension dev [--path <dir>]
neomind extension install <path-or-url>
neomind extension uninstall <id>
neomind extension validate <path> [--verbose]
```

**Extension code must conform to**: [NeoMind-Extensions](../NeoMind-Extensions) repository standards:
- Cargo.toml: `crate-type = ["cdylib", "rlib"]`, library name `neomind_extension_{id}`
- Use `neomind-extension-sdk` workspace dependency
- Export via `neomind_extension_sdk::neomind_export!(StructName)`
- Implement `Extension` trait: `metadata()`, `metrics()`, `commands()`, `execute_command()`, `produce_metrics()`
- Build targets: 6 platforms (darwin/linux/windows × aarch64/x86_64)
- Frontend: UMD bundle, CSS variables only

### 3.9 Agent Commands

```
neomind agent list [--json]
neomind agent get <id> [--json]
neomind agent create --name <name> --model <model> [--system-prompt <prompt>] [--tools <json>] [--execution-mode <focused|free>] [--schedule-type <event|cron|interval>] [--schedule-config <json>] [--json]
neomind agent update <id> [--name <name>] [--model <model>] [--system-prompt <prompt>] [--tools <json>] [--json]
neomind agent delete <id>
neomind agent control <id> --action <pause|resume> [--json]
neomind agent invoke <id> [--input <text>] [--json]
neomind agent memory <id> [--json]
neomind agent executions <id> [--time-range <range>] [--json]
```

### 3.10 Message Commands

```
neomind message list [--unread-only] [--level <level>] [--json]
neomind message get <id> [--json]
neomind message send --title <title> --message <body> --level <info|notice|important|urgent> [--source <source>] [--json]
neomind message read <id>
```

### 3.11 Existing Commands (unchanged)

```
neomind serve [--host <host>] [--port <port>]
neomind prompt <text> [--max-tokens <n>] [--temperature <f>]
neomind chat [--session <id>]
neomind list-models [--endpoint <url>]
neomind health
neomind logs [--tail <n>] [--follow] [--level <level>] [--since <duration>]
neomind check-update
neomind api-key create [--name <name>] [--data-dir <dir>]
neomind api-key list [--data-dir <dir>]
neomind api-key delete <name> [--data-dir <dir>]
```

## 4. Agent Integration

### 4.1 Tool Set After Migration

| Tool | Purpose | Status |
|------|---------|--------|
| `shell` | ALL platform operations via CLI | Enhanced (whitelist + timeout tiers) |
| `think` | Memory storage/retrieval | Unchanged |
| `ask_user` | User clarification/confirmation | Unchanged |
| `{ext}:{cmd}` | Extension dynamic commands | Unchanged |

**Removed** (deprecated in 0.8, deleted in 0.9):
- `device` (aggregated tool)
- `rule` (aggregated tool)
- `agent` (aggregated tool)
- `extension` (aggregated tool)
- `message` (aggregated tool)

### 4.2 ShellTool Enhancements

**Current configuration:**
```rust
pub struct ShellConfig {
    pub enabled: bool,           // default: false
    pub timeout_secs: u64,       // default: 30
    pub max_output_chars: usize, // default: 10000
}
```

**Enhanced configuration:**
```rust
pub struct ShellConfig {
    pub enabled: bool,           // default: true (changed)
    pub timeout_secs: u64,       // default: 30 (query), 120 (build), 300 (compile)
    pub max_output_chars: usize, // default: 50000 (increased)
    pub command_whitelist: Vec<String>,  // ["neomind"] — only neomind commands
    pub timeout_tiers: TimeoutTiers,
}

pub struct TimeoutTiers {
    pub query: u64,    // 10s — list, get, status
    pub mutation: u64, // 30s — create, update, delete
    pub build: u64,    // 120s — extension build, widget build
    pub compile: u64,  // 300s — full release build
}
```

Timeout is determined by parsing the CLI command keywords:
- `list|get|status|latest|history` → query tier
- `create|update|delete|install|uninstall` → mutation tier
- `build|dev` → build tier
- `build --release` → compile tier

### 4.3 System Prompt Integration

The CLI command reference is injected into the Agent's system prompt as a "Build Tools" section:

```
## Build Tools

You have access to the NeoMind CLI for creating and managing platform resources.
Execute commands using the shell tool. Always use --json for structured output.

### Quick Reference
- Device: `neomind device <action> [args]`
- Dashboard: `neomind dashboard <action> [args]`
- Widget: `neomind widget <action> [args]`
- Rule: `neomind rule <action> [args]`
- Transform: `neomind transform <action> [args]`
- Extension: `neomind extension <action> [args]`
- Agent: `neomind agent <action> [args]`
- Message: `neomind message <action> [args]`

Use `neomind <domain> <action> --help` to learn any command's parameters and examples.

### Build Workflow
1. Understand user intent
2. Query existing resources if needed (list/get)
3. Create/configure resources (create/update)
4. Verify results (get/list)
5. Present summary to user

### Conventions
- Always use --json for reliable result parsing
- For complex objects (trigger, actions, config), pass as JSON string or use file path
- Chain multiple commands for multi-step builds
- If a command fails, read the error, adjust, and retry
```

### 4.4 Multi-Step Build Example

User: "帮我创建一个温度监控系统，温度超过30度报警"

```
Round 1: neomind device types list --json
         → Check if temperature sensor type exists

Round 2: neomind device types create --name "温度传感器"
           --metrics '[{"name":"temperature","display_name":"温度","data_type":"Float","unit":"°C"}]'
           --json
         → Create device type

Round 3: neomind device create --name "temp-sensor-01"
           --type "temperature_sensor" --adapter mqtt
           --config '{"topic":"sensor/temp"}' --json
         → Create device

Round 4: neomind dashboard create --name "温度监控" --json
         → Create dashboard

Round 5: neomind dashboard add-widget <dashboard-id>
           --type chart --datasource "device:temp-sensor-01:temperature"
           --config '{"chartType":"line"}' --json
         → Add temperature chart widget

Round 6: neomind rule create --name "高温报警"
           --trigger '{"type":"device_metric","device_id":"temp-sensor-01","metric":"temperature","operator":">","value":30}'
           --actions '{"type":"notification","level":"urgent","message":"温度超过30°C！"}'
           --json
         → Create alert rule

AI: ✅ 温度监控系统已创建完成！包含：
    - 设备类型：温度传感器
    - 设备：temp-sensor-01 (MQTT)
    - 仪表盘：温度监控（含实时曲线）
    - 报警规则：温度 > 30°C 时通知
```

## 5. Frontend Rich Cards

### 5.1 Build Card Types

#### Single Command Result Card
```
┌─────────────────────────────────────────────────┐
│ 🔧 neomind device create                        │
│ ───────────────────────────────────────────────  │
│ ✅ 成功 · 0.8s                                   │
│                                                  │
│ 设备: 温度传感器-01                               │
│ ID: dev_a3f2c1                                   │
│ 类型: temperature-sensor                         │
│ 协议: MQTT (topic: sensor/temp)                  │
│                                                  │
│ [查看设备]  [撤销]                                │
└─────────────────────────────────────────────────┘
```

#### Multi-Step Build Process Card
```
┌─────────────────────────────────────────────────┐
│ 🏗️ 温度监控系统构建                               │
│ ───────────────────────────────────────────────  │
│                                                  │
│ ✅ 1. 创建设备类型              0.3s              │
│ ✅ 2. 创建设备 temp-sensor-01   0.5s              │
│ ✅ 3. 创建仪表盘 "温度监控"      0.4s              │
│ ✅ 4. 添加温度曲线组件           0.3s              │
│ ✅ 5. 创建高温报警规则           0.4s              │
│                                                  │
│ 总计: 1.9s · 5/5 步骤成功                        │
│                                                  │
│ [查看仪表盘]  [查看规则]  [全部撤销]              │
└─────────────────────────────────────────────────┘
```

#### Build Failure Card
```
┌─────────────────────────────────────────────────┐
│ ❌ neomind device create                         │
│ ───────────────────────────────────────────────  │
│ 失败 · 0.2s                                      │
│                                                  │
│ 错误: Device type 'temp-sensor' not found        │
│ 建议: 先创建设备类型，或使用已有的类型              │
│                                                  │
│ [查看可用类型]  [重试]                            │
└─────────────────────────────────────────────────┘
```

#### Code Generation Card (Extension/Widget)
```
┌─────────────────────────────────────────────────┐
│ 📦 扩展开发: weather-forecast                     │
│ ───────────────────────────────────────────────  │
│                                                  │
│ 步骤 1/3: 生成代码 ✅                             │
│   📁 src/lib.rs   (2.1 KB)                      │
│   📁 src/metadata.rs (0.8 KB)                   │
│   📁 Cargo.toml   (0.5 KB)                      │
│                                                  │
│ 步骤 2/3: 编译中... 🔄                           │
│   ▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░ 68%                       │
│                                                  │
│ [查看代码]  [取消]                                │
└─────────────────────────────────────────────────┘
```

### 5.2 Frontend Components

| Component | Responsibility |
|-----------|---------------|
| `BuildCard` | Main card container, selects sub-component by `build_meta.type` |
| `BuildResultItem` | Single command result (success/failure/running) |
| `BuildProcessTracker` | Multi-step build progress tracking |
| `BuildCodePreview` | Code generation: file list, preview, diff view |
| `BuildActionButtons` | Action buttons (view, undo, retry) |

### 5.3 Integration with Existing Chat

BuildCard integrates into the existing `ToolCallVisualization` component:

```
ToolCallVisualization
  ├── if tool_call has build_meta → BuildCard (rich card)
  └── else → existing tool call display (unchanged)
```

### 5.4 Interactive Features

- **View button**: Navigate to corresponding page (Devices/Dashboard/Rules), highlight the new entity
- **Undo button**: Execute `undo_command` from `build_meta` via ShellTool
- **Code preview**: Click to expand code diff view (similar to PR review)
- **Build progress**: Long-running builds update progress via polling or SSE

## 6. CLI Implementation

### 6.1 ApiClient Module

New module in `crates/neomind-cli/` for HTTP communication with local API Server:

```rust
// crates/neomind-cli/src/api_client.rs
pub struct ApiClient {
    base_url: String,  // http://localhost:9375/api
    client: reqwest::Client,
}

impl ApiClient {
    pub fn new() -> Self;
    pub async fn get(&self, path: &str) -> Result<ApiResponse>;
    pub async fn post(&self, path: &str, body: &Value) -> Result<ApiResponse>;
    pub async fn put(&self, path: &str, body: &Value) -> Result<ApiResponse>;
    pub async fn delete(&self, path: &str) -> Result<ApiResponse>;
}

pub struct ApiResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub code: Option<String>,
}
```

### 6.2 Output Formatting

```rust
// crates/neomind-cli/src/output.rs
pub enum OutputFormat {
    Human,  // Default: colored, readable
    Json,   // --json flag: structured
}

pub fn format_response(response: &ApiResponse, format: OutputFormat) -> String;
pub fn format_build_response(response: &ApiResponse, format: OutputFormat) -> String;
```

### 6.3 Command Module Structure

```
crates/neomind-cli/src/
├── main.rs           # CLI entry point, clap app definition
├── api_client.rs     # HTTP client for API Server
├── output.rs         # Output formatting (human/json)
├── commands/
│   ├── mod.rs
│   ├── device.rs     # neomind device *
│   ├── dashboard.rs  # neomind dashboard *
│   ├── widget.rs     # neomind widget *
│   ├── rule.rs       # neomind rule *
│   ├── transform.rs  # neomind transform *
│   ├── extension.rs  # neomind extension * (enhanced)
│   ├── agent.rs      # neomind agent *
│   ├── message.rs    # neomind message *
│   ├── serve.rs      # (existing)
│   ├── prompt.rs     # (existing)
│   ├── chat.rs       # (existing)
│   ├── health.rs     # (existing)
│   └── logs.rs       # (existing)
```

## 7. Migration Plan

### Phase 1: CLI Foundation (0.8.0)

| Task | Priority |
|------|----------|
| ApiClient module | High |
| `--json` global support + output formatting | High |
| Device command group (full CRUD + types) | High |
| Dashboard command group | High |
| Rule command group | High |
| Widget command group | High |
| ShellTool enhancement (whitelist, timeout tiers) | High |
| System Prompt CLI reference injection | High |
| Extension command enhancements (build, dev, logs) | Medium |
| Transform command group | Medium |
| Agent command group | Medium |
| Message command group | Medium |

### Phase 2: Rich Card Experience (0.8.x)

| Task | Priority |
|------|----------|
| BuildCard component family | High |
| `build_meta` in CLI JSON output | High |
| Undo mechanism (front-end) | Medium |
| Entity navigation + highlight | Medium |
| BuildProcessTracker for multi-step builds | Medium |
| BuildCodePreview for extension/widget dev | Low |

### Phase 3: Tool Consolidation (0.9)

| Task | Priority |
|------|----------|
| Remove deprecated aggregated tools | High |
| Verify CLI coverage completeness | High |
| Performance optimization | Medium |
| CLI autocomplete (shell completions) | Low |

## 8. Repository Standards

AI Build generates artifacts that must conform to existing repository standards:

### NeoMind-DeviceTypes
- JSON format in `types/` directory
- Required: `device_type`, `name`, `metrics`
- Metrics: dot notation for nested fields
- Auto-discovery from directory (no index.json update needed)

### NeoMind-Dashboard-Components
- `manifest.json` + `bundle.js` (IIFE) per component
- Required manifest fields: `id`, `name` (en/zh), `description` (en/zh), `icon`, `category`, `global_name`, `export_name`
- Uses `window.React` + `window.jsxRuntime` only
- CSS variables only (no hardcoded values)
- Must fill `w-full h-full` container

### NeoMind-Extensions
- Cargo.toml workspace with `neomind-extension-sdk`
- Library name: `neomind_extension_{id}`
- Export macro: `neomind_extension_sdk::neomind_export!(StructName)`
- Implement `Extension` trait
- Build script: `build.sh` with 6 platform targets
- Output: `{id}-{version}-{platform}.nep`

These standards are injected into the AI's System Prompt as generation constraints.

## 9. What We're Not Doing (YAGNI)

- **Sandbox/preview environment** — Direct execution with undo mechanism
- **Separate Build page** — Enhance existing Chat
- **New Agent Tools per domain** — ShellTool only
- **Dedicated Build API endpoints** — CLI calls existing API
- **Version control/rollback system** — Simple `undo_command` per operation
- **Build templates/library** — AI generates based on intent, not templates

## 10. Scope and Risk

### In Scope (0.8)
- Complete CLI command coverage for all product capabilities
- ShellTool as unified execution channel
- System Prompt CLI reference injection
- BuildCard rich card components
- CLI `--json` structured output

### Risks
| Risk | Mitigation |
|------|-----------|
| LLM generates invalid CLI commands | CLI `--help` for self-correction, retry on failure |
| CLI output parsing unreliability | Structured `--json` output with strict schema |
| Removing old tools breaks existing sessions | Gradual deprecation (0.8 mark deprecated, 0.9 remove) |
| Complex multi-step builds fail midway | Undo mechanism, partial success display |
| Extension build requires Rust toolchain | Document requirement, graceful error if missing |
