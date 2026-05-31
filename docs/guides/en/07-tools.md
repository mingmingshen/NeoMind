# Tools Module

**Package**: `neomind-agent` (toolkit and tools submodules)
**Version**: 0.8.0
**Completion**: 90%
**Purpose**: AI function calling tools

## Overview

The Tools module implements a tool system for AI-callable functions, including device control, rule management, data analysis, and more. Tools are organized into two submodules within `neomind-agent`: `tools/` for agent-specific tool wrappers (event integration, interaction) and `toolkit/` for core tool implementations (shell, skills, extensions, metrics).

## Module Structure

```
crates/neomind-agent/src/
├── toolkit/                            # Core tool implementations
│   ├── mod.rs                          # Public interface and re-exports
│   ├── tool.rs                         # Tool trait and ToolDefinition
│   ├── registry.rs                     # ToolRegistry and ToolRegistryBuilder
│   ├── resolver.rs                     # EntityResolver (fuzzy name/ID matching)
│   ├── shell.rs                        # Shell tool (neomind CLI execution)
│   ├── skill_tool.rs                   # Skill management tool
│   ├── extension_tools.rs              # Extension tool generator and executor
│   ├── session_search.rs              # Session history search tool
│   ├── time_utils.rs                  # Time range parsing utilities
│   └── error.rs                        # Tool error types
├── tools/                              # Agent tool wrappers
│   ├── mod.rs                          # Public interface and re-exports
│   ├── event_integration.rs            # Tool execution with event bus tracking
│   ├── interaction.rs                  # AskUser, ClarifyIntent, ConfirmAction
│   ├── mapper.rs                       # Tool name mapping and parameter resolution
│   ├── think.rs                        # ThinkTool for reasoning
│   └── tool_search.rs                  # ToolSearchTool for tool lookup
```

## Core Trait

### Tool - Tool Interface

```rust
pub trait Tool: Send + Sync {
    /// Get tool definition
    fn definition(&self) -> &ToolDefinition;

    /// Execute tool
    fn execute(&self, input: &serde_json::Value) -> Result<ToolOutput>;

    /// Validate input
    fn validate(&self, input: &serde_json::Value) -> Result<()> {
        // Default implementation: schema-based validation
    }

    /// Get tool schema (for LLM)
    fn schema(&self) -> serde_json::Value {
        // Returns OpenAI function calling format
    }
}
```

### ToolDefinition - Tool Definition

```rust
pub struct ToolDefinition {
    /// Tool name (unique identifier)
    pub name: String,

    /// Display name
    pub display_name: String,

    /// Tool description (for AI)
    pub description: String,

    /// Parameter definitions
    pub parameters: Vec<Parameter>,

    /// Return value description
    pub returns: Option<String>,

    /// Usage examples
    pub examples: Vec<ToolExample>,
}
```

```rust
pub struct Parameter {
    /// Parameter name
    pub name: String,

    /// Parameter type
    pub param_type: ParameterType,

    /// Description
    pub description: String,

    /// Required
    pub required: bool,

    /// Default value
    pub default: Option<serde_json::Value>,

    /// Enum values (for selection)
    pub enum_values: Option<Vec<String>>,
}
```

```rust
pub enum ParameterType {
    String,
    Number,
    Integer,
    Boolean,
    Array,
    Object,
}
```

### ToolOutput - Tool Output

```rust
pub struct ToolOutput {
    /// Success status
    pub success: bool,

    /// Output data
    pub data: serde_json::Value,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}
```

## Tool Registry

```rust
pub struct ToolRegistry {
    /// Tool mapping
    tools: HashMap<String, Arc<dyn Tool>>,

    /// Execution history
    history: Arc<RwLock<ToolExecutionHistory>>,

    /// Config
    config: RegistryConfig,
}
```

```rust
impl ToolRegistry {
    /// Create empty registry
    pub fn new() -> Self;

    /// Add tool
    pub fn register(&mut self, tool: Arc<dyn Tool>);

    /// Get tool
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// List all tools
    pub fn list(&self) -> Vec<String>;

    /// Execute tool
    pub async fn execute(
        &self,
        name: &str,
        input: &serde_json::Value,
    ) -> Result<ToolOutput>;
}
```

## Built-in Tools

### Shell Tool (CLI-driven)

The primary tool interface uses the `shell` tool to execute `neomind` CLI commands. The LLM constructs CLI commands and the shell tool routes them to in-process CLI operations.

```rust
/// Shell tool for neomind CLI execution
pub struct ShellTool;

// 10 CLI domains: device, dashboard, rule, extension, widget,
//   transform, agent, message, system, data-push
// Input: { "command": "neomind device list --type sensor" }
// Output: { "success": true, "data": {...}, "suggestion": null }
```

**CLI Command Reference** (embedded in tool description for LLM discoverability):
| Domain | Actions |
|--------|---------|
| `device` | list, get, create, update, delete, control |
| `dashboard` | list, get, create, update, delete |
| `rule` | list, get, create, update, delete, enable, disable |
| `extension` | list, get, install, uninstall, enable, disable |
| `widget` | list, get, create, update, delete |
| `transform` | list, get, create, update, delete |
| `agent` | list, get, create, update, delete, status, executions |
| `message` | list, get, send, channel-list, channel-create, channel-update, channel-delete |
| `system` | info |
| `data-push` | list, get, create, update, delete |

### Skill Tool

```rust
/// User-defined skill management
pub struct SkillTool;

// Actions: search, list, get, create, update, delete
// Input: { "action": "search", "keyword": "temperature" }
```

### Extension Tools

```rust
/// Extension tool generator and executor
pub struct ExtensionTool;
pub struct ExtensionToolGenerator;

// Dynamically generates tool definitions from extension manifests
// Executes extension tool calls via the extension runner
```

### Session Search Tool

```rust
/// Session history search
pub struct SessionSearchTool;

// Search across conversation history
// Input: { "query": "temperature alert", "limit": 5 }
```

### Device Tools

```rust
/// List devices
pub struct ListDevicesTool {
    device_service: Arc<DeviceService>,
}

// Input: { "device_type": "sensor" } (optional filter)
// Output: { "devices": [...] }
```

```rust
/// Query device data
pub struct QueryDataTool {
    device_service: Arc<DeviceService>,
    time_series: Arc<TimeSeriesStore>,
}

// Input: { "device_id": "sensor_1", "metric": "temperature" }
// Output: { "current": 25.5, "history": [...] }
```

```rust
/// Control device
pub struct ControlDeviceTool {
    device_service: Arc<DeviceService>,
}

// Input: { "device_id": "relay_1", "command": "turn_on" }
// Output: { "success": true, "result": ... }
```

### Rule Tools

```rust
/// List rules
pub struct ListRulesTool {
    rule_service: Arc<RuleService>,
}
```

```rust
/// Create rule
pub struct CreateRuleTool {
    rule_service: Arc<RuleService>,
    parser: RuleParser,
}
```

### Agent Tools

```rust
/// List agents
pub struct ListAgentsTool {
    agent_service: Arc<AgentService>,
}
```

### Interaction Tools

NeoMind provides interaction tools for user communication during agent execution:

#### AskUserTool

Prompt the user for input during execution:

```rust
/// Ask user for input
pub struct AskUserTool;

// Input: { "question": "Which device to control?", "options": ["device_1", "device_2"] }
// Output: { "type": "ask_user", "awaiting_user_response": true }
```

#### ConfirmActionTool

Request confirmation for dangerous operations:

```rust
/// Confirm action before execution
pub struct ConfirmActionTool;

// Input: { "action": "delete all devices", "risk_level": "high" }
// Output: { "type": "confirm_action", "awaiting_confirmation": true }
```

**Dangerous Action Detection**: The system automatically detects dangerous operations in both English and Chinese:

| English Keywords | Chinese Keywords |
|-----------------|------------------|
| delete, remove, clear | 删除, 移除, 清空 |
| reset, format | 重置, 格式化 |
| close all, turn off all | 关闭所有 |
| delete all, batch delete | 删除所有, 批量删除 |

**Example**:
```rust
// These will trigger confirmation:
tool.requires_confirmation("delete all rules");      // English
tool.requires_confirmation("关闭所有设备");           // Chinese
tool.requires_confirmation("删除所有自动化规则");    // Chinese

// These will NOT trigger confirmation:
tool.requires_confirmation("show temperature");
tool.requires_confirmation("获取温度");
```

### System Tools

```rust
/// System info — aggregates MQTT status, network info, webhook URL
pub struct SystemInfoTool;

// Accessible via: neomind system info
// Output: { "version": "...", "uptime": "...", "mqtt_status": "...", "webhook_url": "..." }
```

### Think Tool

```rust
/// Reasoning and thought recording
pub struct ThinkTool;

// Input: { "thought": "User wants to control device, need to find device ID first..." }
// Output: { "type": "think", "recorded": true }
```

### Tool Search Tool

```rust
/// Tool lookup and discovery
pub struct ToolSearchTool;

// Input: { "query": "temperature", "category": "device" }
// Output: { "results": [...] }
```

## Tool Execution History

```rust
pub struct ToolExecutionHistory {
    /// Execution records
    records: Vec<ToolExecutionRecord>,
}

pub struct ToolExecutionRecord {
    /// Execution ID
    pub id: String,
    /// Tool name
    pub tool_name: String,
    /// Input parameters
    pub input: serde_json::Value,
    /// Output result
    pub output: Option<ToolOutput>,
    /// Execution timestamp
    pub executed_at: i64,
    /// Duration (ms)
    pub duration_ms: u64,
    /// Success status
    pub success: bool,
}

pub struct ToolExecutionStats {
    /// Total executions
    pub total_executions: usize,
    /// Success count
    pub success_count: usize,
    /// Failure count
    pub failure_count: usize,
    /// Average duration
    pub avg_duration_ms: f64,
    /// Most used tools
    pub most_used_tools: Vec<(String, usize)>,
}
```

## LLM Format

```rust
pub struct LlmToolDefinition {
    /// Tool name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Description
    pub description: String,
    /// Parameters (JSON Schema format)
    pub parameters: serde_json::Value,
}
```

## Usage Examples

### Create Tool Registry

```rust
use neomind_agent::toolkit::{ToolRegistryBuilder, ShellTool, SkillTool};
use std::sync::Arc;

let registry = ToolRegistryBuilder::new()
    .with_tool(Arc::new(ShellTool::new(shell_config)))
    .with_tool(Arc::new(SkillTool::new(skill_registry)))
    .build();
```

### Execute Tool via Shell

```rust
use neomind_agent::toolkit::ToolRegistry;

let result = registry.execute(
    "shell",
    serde_json::json!({
        "command": "neomind device list --type sensor"
    }),
).await?;

if result.success {
    println!("Result: {}", result.data);
} else {
    println!("Error: {}", result.error.unwrap());
}
```

## Design Principles

1. **Unified Interface**: All tools implement the same Trait
2. **Type Safety**: Strongly typed input/output
3. **CLI-First**: Tools use `neomind` CLI commands via shell tool for consistency
4. **LLM-Friendly**: Standardized function calling format with embedded CLI reference
5. **Traceable**: Full execution history tracking via event bus integration
6. **Error Recovery**: Failed CLI commands return domain-specific recovery hints
