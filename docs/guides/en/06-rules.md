# Rules Module

**Package**: `neomind-rules`
**Version**: 0.8.0
**Purpose**: JSON-based rule engine with event-driven evaluation

## Overview

The Rules module implements a JSON-based rule engine with support for device and extension metric conditions, event-driven evaluation, context-aware validation, and three action types (notify, execute, trigger_agent).

## Module Structure

```
crates/neomind-rules/src/
├── lib.rs                      # Public interface and re-exports
├── models.rs                   # Core data models (CompiledRule, conditions, actions)
├── engine.rs                   # Rule evaluation engine
├── preview.rs                  # Human-readable preview generation (read-only)
├── validator.rs                # Rule validation
├── store.rs                    # Rule persistence (redb)
├── device_integration.rs       # Device action execution
├── extension_integration.rs    # Extension action execution
├── unified_provider.rs         # Unified value provider
└── error.rs                    # Error types
```

## Rule JSON Format

### Rule Structure

```json
{
  "name": "<rule name>",
  "description": "<optional description>",
  "enabled": true,
  "trigger": {"trigger_type": "data_change"},
  "condition": { "<condition_type>": "..." },
  "for_duration": <milliseconds, optional>,
  "cooldown": <milliseconds, default 60000>,
  "actions": [ { "<type>": "..." } ]
}
```

### Complete Examples

```bash
# Simple comparison rule (enabled by default)
neomind rule create --json '{
  "name": "Temperature Alert",
  "condition": {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 50},
  "actions": [{"type": "notify", "message": "Too hot: {value}C", "severity": "critical"}]
}'

# Rule with duration (condition must hold for 5 minutes)
neomind rule create --json '{
  "name": "Sustained High Temperature",
  "condition": {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 30},
  "for_duration": 300000,
  "actions": [
    {"type": "notify", "message": "High for 5 min", "severity": "warning"},
    {"type": "execute", "target": "fan", "target_type": "device", "command": "set_speed", "params": {"speed": 100}}
  ]
}'

# Extension metric rule
neomind rule create --json '{
  "name": "Weather Alert",
  "condition": {"condition_type": "comparison", "source": "extension:weather:temperature", "operator": "greater_than", "threshold": 30},
  "actions": [{"type": "notify", "message": "Weather too hot", "severity": "warning"}]
}'

# Complex rule with AND
neomind rule create --json '{
  "name": "Compound Alert",
  "condition": {
    "condition_type": "logical", "operator": "and",
    "conditions": [
      {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 30},
      {"condition_type": "comparison", "source": "extension:weather:humidity", "operator": "less_than", "threshold": 20}
    ]
  },
  "actions": [{"type": "notify", "message": "High temp, low humidity", "severity": "warning"}]
}'

# Range condition
neomind rule create --json '{
  "name": "Temperature Range",
  "condition": {"condition_type": "range", "source": "device:sensor:temperature", "min": 20, "max": 25},
  "actions": [{"type": "notify", "message": "Comfortable range", "severity": "info"}]
}'

# Scheduled rule (no condition needed)
neomind rule create --json '{
  "name": "Periodic Check",
  "trigger": {"trigger_type": "schedule", "cron": "0 */5 * * *"},
  "actions": [{"type": "execute", "target": "sensor-controller", "target_type": "device", "command": "read_sensors", "params": {}}]
}'

# Agent trigger rule
neomind rule create --json '{
  "name": "Auto Analysis",
  "condition": {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 40},
  "actions": [{"type": "trigger_agent", "agent_id": "analyzer", "input": "Check temperature anomaly"}]
}'
```

## Core Types

### 1. CompiledRule - Complete Rule Definition

```rust
pub struct CompiledRule {
    pub id: RuleId,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub tags: Vec<String>,
    pub trigger: RuleTrigger,
    pub condition: Option<RuleCondition>,
    pub actions: Vec<RuleAction>,
    pub cooldown: Duration,
    pub for_duration: Option<Duration>,
    pub state: RuleState,
    pub dsl_preview: String,
    pub source: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 2. RuleTrigger - Trigger Type

```rust
pub enum RuleTrigger {
    DataChange { sources: Vec<DataSourceId> },
    Schedule { cron: String },
    Manual,
}
```

### 3. RuleCondition - 3 Condition Types

```rust
pub enum RuleCondition {
    Comparison {
        source: DataSourceId,
        operator: ComparisonOperator,
        threshold: f64,
    },
    Range {
        source: DataSourceId,
        min: f64,
        max: f64,
    },
    Logical {
        operator: LogicalOperator,  // And | Or | Not
        conditions: Vec<RuleCondition>,
    },
}
```

```rust
pub enum ComparisonOperator {
    GreaterThan,    // >
    LessThan,       // <
    GreaterEqual,   // >=
    LessEqual,      // <=
    Equal,          // ==
    NotEqual,       // !=
}
```

### 4. RuleAction - 3 Action Types

```rust
pub enum RuleAction {
    Notify {
        message: String,
        severity: NotifySeverity,  // Info | Warning | Critical | Emergency
    },
    Execute {
        target: String,
        target_type: ExecuteTarget,  // Device | Extension
        command: String,
        params: serde_json::Value,
    },
    TriggerAgent {
        agent_id: String,
        input: Option<String>,
        data: Option<serde_json::Value>,
    },
}
```

## Rule Engine

```rust
pub struct RuleEngine {
    store: Arc<RuleStore>,
    value_provider: Arc<dyn ValueProvider>,
    device_executor: Option<Arc<DeviceActionExecutor>>,
    extension_executor: Option<Arc<ExtensionActionExecutor>>,
    message_manager: Option<Arc<MessageManager>>,
    agent_trigger: Option<AgentTriggerCallback>,
}
```

The engine evaluates rules on data updates (`on_data_update`), using a subscription index to map DataSourceIds to relevant rules. Cooldowns are enforced atomically via `try_claim_cooldown()` to prevent TOCTOU races.

### Rule Execution Result

```rust
pub struct RuleExecutionResult {
    pub rule_id: RuleId,
    pub rule_name: String,
    pub success: bool,
    pub actions_executed: Vec<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub triggered_at: DateTime<Utc>,
}
```

## API Endpoints

```
# Rules CRUD
GET    /api/rules                           # List rules
POST   /api/rules                           # Create rule (JSON body)
GET    /api/rules/:id                       # Get rule
PUT    /api/rules/:id                       # Update rule (JSON body)
DELETE /api/rules/:id                       # Delete rule
POST   /api/rules/:id/enable                # Enable/disable rule

# Rule Operations
POST   /api/rules/:id/test                  # Test rule
GET    /api/rules/:id/history               # Rule execution history
```

## Usage Examples

### Create Rule

```bash
curl -X POST http://localhost:9375/api/rules \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Temperature Alert",
    "condition": {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 30},
    "actions": [{"type": "notify", "message": "High temperature", "severity": "warning"}]
  }'
```

### Test Rule

```bash
curl -X POST http://localhost:9375/api/rules/<rule-id>/test \
  -H "Content-Type: application/json" \
  -d '{"test_value": 35}'
```

## Design Principles

1. **JSON-First**: Pure JSON rule definitions — no DSL parsing required
2. **Event-Driven**: Rules evaluate on data changes via subscription index
3. **Composable**: Logical conditions support AND/OR/NOT combinations
4. **Extensible**: Device, extension, and scheduled triggers
5. **Agent-Integrated**: Rules can trigger AI agents for complex analysis
6. **Cooldown-Safe**: Atomic cooldown claiming prevents concurrent trigger races
