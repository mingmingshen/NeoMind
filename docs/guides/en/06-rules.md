# Rules Module

**Package**: `neomind-rules`
**Version**: 0.8.0
**Completion**: 85%
**Purpose**: DSL rule engine with LLM-based generation

## Overview

The Rules module implements a DSL (Domain Specific Language) rule engine with support for device and extension metric conditions, LLM-based rule generation from natural language, context-aware validation, and multiple action types.

## Module Structure

```
crates/neomind-rules/src/
├── lib.rs                      # Public interface and re-exports
├── dsl.rs                      # DSL parser and types
├── engine.rs                   # Rule evaluation engine
├── generator.rs                # LLM-based rule generation from NL
├── validator.rs                # Context-aware rule validation
├── store.rs                    # Rule persistence (redb)
├── history.rs                  # Rule execution history
├── dependencies.rs             # Dependency management
├── device_integration.rs       # Device action execution
├── extension_integration.rs    # Extension action execution
├── unified_provider.rs         # Unified value provider
└── error.rs                    # Error types
```

## DSL Syntax

### Rule Structure

```neo
RULE "<name>"
[TRIGGER SCHEDULE "<cron>"]
WHEN <condition>
[FOR <duration>]
DO
    <action>
    [<action> ...]
END
```

### Complete Examples

```neo
# Simple device rule
RULE "Temperature Alert"
WHEN sensor.temperature > 50
DO
    NOTIFY "Device temperature too high: {temperature}C"
END

# Device rule with duration
RULE "Sustained High Temperature"
WHEN sensor.temperature > 30
FOR 5 minutes
DO
    NOTIFY "Temperature high for 5 minutes"
    EXECUTE device.fan(speed=100)
END

# Extension metric rule
RULE "Weather Alert"
WHEN EXTENSION weather.temperature > 30
DO
    NOTIFY "Weather too hot"
END

# Complex rule with AND/OR
RULE "Compound Alert"
WHEN (sensor.temperature > 30) AND (EXTENSION weather.humidity < 20)
DO
    NOTIFY "High temp, low humidity"
    EXECUTE device.humidifier(on=true)
END

# Range condition
RULE "Temperature Range"
WHEN sensor.temperature BETWEEN 20 AND 25
DO
    NOTIFY "Temperature in comfort range"
END

# Scheduled rule
RULE "Periodic Check"
TRIGGER SCHEDULE "0 */5 * * * *"
DO
    EXECUTE device.read_sensors()
END

# Agent trigger rule
RULE "Auto Analysis"
WHEN sensor.temperature > 40
DO
    TRIGGER_AGENT "analyzer" INPUT "Check temperature anomaly"
END
```

## Core Types

### 1. ParsedRule - Parsed Rule Definition

```rust
pub struct ParsedRule {
    /// Rule name
    pub name: String,
    /// Condition to evaluate
    pub condition: RuleCondition,
    /// Duration for condition to hold before triggering
    pub for_duration: Option<Duration>,
    /// Actions to execute
    pub actions: Vec<RuleAction>,
    /// Description (optional)
    pub description: Option<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Trigger type
    pub trigger_type: TriggerType,
}
```

### 2. TriggerType - Trigger Type

```rust
pub enum TriggerType {
    /// Triggered by device state changes (default)
    DeviceState,
    /// Triggered on a cron schedule
    Schedule { cron: String },
    /// Triggered manually via API
    Manual,
}
```

### 3. RuleCondition - Condition Definition

```rust
pub enum RuleCondition {
    /// Device condition: device.metric operator value
    Device {
        device_id: String,
        metric: String,
        operator: ComparisonOperator,
        threshold: f64,
    },
    /// Extension condition: extension.metric operator value
    Extension {
        extension_id: String,
        metric: String,
        operator: ComparisonOperator,
        threshold: f64,
    },
    /// Device range condition
    DeviceRange {
        device_id: String,
        metric: String,
        min: f64,
        max: f64,
    },
    /// Extension range condition
    ExtensionRange {
        extension_id: String,
        metric: String,
        min: f64,
        max: f64,
    },
    /// Logical AND
    And(Vec<RuleCondition>),
    /// Logical OR
    Or(Vec<RuleCondition>),
    /// Logical NOT
    Not(Box<RuleCondition>),
    /// Always true (for scheduled/manual rules)
    Always,
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

### 4. RuleAction - Action Definition

```rust
pub enum RuleAction {
    /// Send notification
    Notify {
        message: String,
        channels: Option<Vec<String>>,
    },
    /// Execute device command
    Execute {
        device_id: String,
        command: String,
        params: HashMap<String, serde_json::Value>,
    },
    /// Log message
    Log {
        level: LogLevel,
        message: String,
        severity: Option<String>,
    },
    /// Set device property
    Set {
        device_id: String,
        property: String,
        value: serde_json::Value,
    },
    /// Delay execution
    Delay { duration: Duration },
    /// Create alert
    CreateAlert {
        title: String,
        message: String,
        severity: AlertSeverity,
    },
    /// Send HTTP request
    HttpRequest {
        method: HttpMethod,
        url: String,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    },
    /// Trigger AI Agent
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

```rust
impl RuleEngine {
    /// Create rule engine
    pub fn new(value_provider: Arc<dyn ValueProvider>) -> Self;

    /// Add rule from DSL text
    pub async fn add_rule_from_dsl(&self, dsl: &str) -> Result<RuleId>;

    /// Enable/disable rule
    pub async fn set_rule_enabled(&self, id: &RuleId, enabled: bool) -> Result<()>;

    /// Evaluate all rules
    pub async fn evaluate_all(&self) -> Vec<RuleExecutionResult>;

    /// Get rule state
    pub async fn get_rule_state(&self, id: &RuleId) -> Option<RuleState>;

    /// Start evaluation loop
    pub async fn start(&self) -> Result<()>;

    /// Stop evaluation loop
    pub async fn stop(&self) -> Result<()>;
}
```

### Rule Execution Result

```rust
pub struct RuleExecutionResult {
    pub rule_id: RuleId,
    pub rule_name: String,
    pub triggered: bool,
    pub condition_met: bool,
    pub actions_executed: usize,
    pub action_results: Vec<ActionResult>,
    pub evaluation_duration: Duration,
}
```

## Rule Validation

```rust
pub struct RuleValidator {
    // Validates rules against available resources
}

pub struct ValidationContext {
    pub devices: Vec<DeviceInfo>,
    pub metrics: Vec<MetricInfo>,
    pub commands: Vec<CommandInfo>,
    pub alert_channels: Vec<AlertChannelInfo>,
}

pub struct RuleValidationResult {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub resource_summary: ResourceSummary,
}

pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}
```

## Rule History

```rust
pub struct RuleHistoryEntry {
    pub rule_id: String,
    pub rule_name: String,
    pub triggered_at: i64,
    pub condition_met: bool,
    pub actions_executed: usize,
    pub duration_ms: u64,
}

pub struct RuleHistoryStorage {
    db: Database,
}
```

## API Endpoints

```
# Rules CRUD
GET    /api/rules                           # List rules
POST   /api/rules                           # Create rule (requires {"dsl": "RULE ... END"})
GET    /api/rules/:id                       # Get rule
PUT    /api/rules/:id                       # Update rule
DELETE /api/rules/:id                       # Delete rule
POST   /api/rules/:id/enable                # Enable/disable rule

# Rule Operations
POST   /api/rules/:id/test                  # Test rule
GET    /api/rules/:id/history               # Rule execution history
POST   /api/rules/validate                  # Validate rule DSL

# Rule Import/Export
GET    /api/rules/export                    # Export all rules
POST   /api/rules/import                    # Import rules

# Rule Resources
GET    /api/rules/resources                 # Available resources for validation
```

## Usage Examples

### Create Rule via DSL

```bash
curl -X POST http://localhost:9375/api/rules \
  -H "Content-Type: application/json" \
  -d '{
    "dsl": "RULE \"Temperature Alert\" WHEN sensor.temperature > 30 DO NOTIFY \"High temperature\" END"
  }'
```

### Test Rule

```bash
curl -X POST http://localhost:9375/api/rules/test \
  -H "Content-Type: application/json" \
  -d '{
    "dsl": "RULE \"Test\" WHEN sensor.temperature > 30 DO NOTIFY \"High\" END",
    "context": {
      "sensor": {
        "temperature": 35
      }
    }
  }'
```

### Validate Rule

```bash
curl -X POST http://localhost:9375/api/rules/validate \
  -H "Content-Type: application/json" \
  -d '{
    "dsl": "RULE \"Test\" WHEN sensor.temperature > 30 DO NOTIFY \"High\" END"
  }'
```

## Design Principles

1. **DSL-First**: Human-readable rule definition language (RULE/WHEN/DO/END)
2. **Testable**: All rules can be tested with mock context
3. **Event-Driven**: Rules evaluate based on data changes
4. **Composable**: Support complex condition combinations (AND/OR/NOT)
5. **Extensible**: Support device, extension, and scheduled triggers
6. **Validated**: Context-aware validation against available resources
7. **Agent-Integrated**: Rules can trigger AI agents for complex analysis
