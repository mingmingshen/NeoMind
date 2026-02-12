# Rules Module

**Package**: `neomind-rules`
**Version**: 0.5.8
**Completion**: 75%
**Purpose**: DSL rule engine

## Overview

The Rules module implements a DSL (Domain Specific Language) rule engine, supporting rule creation and management from natural language.

## Module Structure

```
crates/rules/src/
├── lib.rs                      # Public interface
├── parser/                     # Pest DSL parser
│   ├── mod.rs
│   └── grammar.pest             # Pest syntax file
├── engine/                     # Rule execution engine
│   ├── mod.rs
│   ├── executor.rs
│   └── context.rs
├── types.rs                    # Rule types
└── store.rs                    # Rule storage
```

## DSL Syntax

### Rule Structure

```neo
# Trigger
ON <trigger>
# Conditions
WHEN <conditions>
# Actions
THEN <actions>
```

### Complete Example

```neo
# Simple rule
ON device.temperature > 30
WHEN device.location == "greenhouse"
THEN send_alert("Temperature too high: {temperature}°C")

# Complex rule
ON device.temperature
WHEN device.temperature > 30
   AND device.location == "greenhouse"
   AND time.between(9, 18)
THEN device.set_fan(true)
   AND send_alert("Fan enabled")
```

# Scheduled rule
ON schedule.every(1h)
THEN device.read_all_sensors()
   AND log_temperature()

# Multi-condition rule
ON ANY(
    device.temperature > 35,
    device.humidity > 80
)
WHEN device.status == "online"
THEN send_alert("Greenhouse environment abnormal")
```
```

## Core Types

### 1. Rule - Rule Definition

```rust
pub struct Rule {
    /// Rule ID
    pub id: String,

    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// Enabled status
    pub enabled: bool,

    /// Trigger
    pub trigger: Trigger,

    /// Condition list
    pub conditions: Vec<Condition>,

    /// Action list
    pub actions: Vec<Action>,

    /// Metadata
    pub metadata: RuleMetadata,
}
```

### 2. Trigger - Trigger Definition

```rust
pub enum Trigger {
    /// Device event trigger
    Device {
        device_id: String,
        event_type: DeviceEventType,
    },

    /// Schedule trigger
    Schedule {
        schedule: ScheduleConfig,
    },

    /// Data condition trigger
    Data {
        metric: String,
        comparison: ComparisonOperator,
        value: f64,
    },

    /// Combined trigger
    Any(Vec<Trigger>),
    All(Vec<Trigger>),
}
```

```rust
pub enum ScheduleConfig {
    /// Interval execution
    Every {
        value: u64,
        unit: TimeUnit,
    },

    /// Cron expression
    Cron(String),

    /// Specific time
    At {
        hour: u8,
        minute: u8,
    },
}
```

### 3. Condition - Condition Definition

```rust
pub struct Condition {
    /// Left value
    pub left: ConditionValue,

    /// Comparison operator
    pub operator: ComparisonOperator,

    /// Right value
    pub right: ConditionValue,

    /// Logical operator
    pub logic: Option<LogicOperator>,
}
```

```rust
pub enum ComparisonOperator {
    Eq,    // ==
    Ne,    // !=
    Gt,    // >
    Ge,    // >=
    Lt,    // <
    Le,    // <=
    Contains,
    Matches, // Regex match
}
```

```rust
pub enum LogicOperator {
    And,
    Or,
    Xor,
}
```

### 4. Action - Action Definition

```rust
pub enum Action {
    /// Device control
    Device {
        device_id: String,
        command: String,
        parameters: serde_json::Value,
    },

    /// Send message
    SendMessage {
        channel: String,
        message: String,
    },

    /// Send alert
    SendAlert {
        severity: AlertSeverity,
        message: String,
    },

    /// HTTP request
    Http {
        url: String,
        method: HttpMethod,
        headers: HashMap<String, String>,
        body: Option<String>,
    },

    /// Set variable
    SetVariable {
        name: String,
        value: serde_json::Value,
    },

    /// Delay
    Delay {
        duration_ms: u64,
    },
}
```

## Rule Parser

```rust
pub struct RuleParser {
    /// Pest parser
    parser: PestParser<Rule>,
}
```

```rust
impl RuleParser {
    /// Parse rule text
    pub fn parse(&self, input: &str) -> Result<Rule>;

    /// Validate rule syntax
    pub fn validate(&self, input: &str) -> Result<()>;
}
```

### Pest Grammar

```pest
// grammar.pest
rule = { SOI ~ trigger ~ conditions? ~ actions ~ EOI }

trigger = { "ON" ~ ~condition }
condition = { comparison }
comparison = {
    ~ value ~ operator ~ value
    | "ANY(" ~ condition_list ~ ")"
    | "ALL(" ~ condition_list ~ ")"
}

operator = {
    "==" | "!=" | ">" | ">=" | "<" | "<="
    | "contains" | "matches"
}

action = {
    device_action
    | send_alert_action
    | send_message_action
    | http_action
    | delay_action
}

value = {
    string | number | boolean
    | device_ref | time_ref
}
```

## Rule Engine

```rust
pub struct RuleEngine {
    /// Rule storage
    store: Arc<RuleStore>,

    /// Event bus
    event_bus: Arc<EventBus>,

    /// Device service
    device_service: Arc<DeviceService>,

    /// Message service
    message_service: Arc<MessageService>,
}
```

```rust
impl RuleEngine {
    /// Create rule engine
    pub fn new(
        store: Arc<RuleStore>,
        event_bus: Arc<EventBus>,
    ) -> Self;

    /// Register rule
    pub async fn register_rule(&self, rule: Rule) -> Result<()>;

    /// Enable/disable rule
    pub async fn set_rule_enabled(&self, id: &str, enabled: bool) -> Result<()>;

    /// Evaluate rule
    pub async fn evaluate_rule(
        &self,
        rule_id: &str,
        context: &EvaluationContext,
    ) -> RuleResult;

    /// Execute actions
    pub async fn execute_actions(
        &self,
        actions: &[Action],
        context: &EvaluationContext,
    ) -> Result<Vec<ActionResult>>;

    /// Start engine
    pub async fn start(&self) -> Result<()>;

    /// Stop engine
    pub async fn stop(&self) -> Result<()>;
}
```

### Evaluation Context

```rust
pub struct EvaluationContext {
    /// Current timestamp
    pub timestamp: i64,

    /// Device states
    pub device_states: HashMap<String, DeviceState>,

    /// Variables
    pub variables: HashMap<String, serde_json::Value>,

    /// Trigger data
    pub trigger_data: Option<serde_json::Value>,
}
```

## Rule Execution History

```rust
pub struct RuleExecutionRecord {
    /// Execution ID
    pub id: String,

    /// Rule ID
    pub rule_id: String,

    /// Trigger timestamp
    pub triggered_at: i64,

    /// Execution result
    pub result: RuleExecutionResult,

    /// Action results
    pub action_results: Vec<ActionResult>,

    /// Duration
    pub duration_ms: u64,
}
```

```rust
pub enum RuleExecutionResult {
    /// Triggered and successful
    Triggered,

    /// Triggered but failed
    Failed { error: String },

    /// Not triggered
    NotTriggered,
}
```

## API Endpoints

```
# Rules CRUD
GET    /api/rules                           # List rules
POST   /api/rules                           # Create rule
GET    /api/rules/:id                       # Get rule
PUT    /api/rules/:id                       # Update rule
DELETE /api/rules/:id                       # Delete rule
POST   /api/rules/:id/enable                # Enable/disable rule

# Rule Operations
POST   /api/rules/:id/test                  # Test rule
GET    /api/rules/:id/history               # Rule execution history
POST   /api/rules/validate                  # Validate rule DSL

# Rule Templates
GET    /api/rules/templates                 # Rule templates
POST   /api/rules/from-nl                   # Generate rule from natural language
```

## Usage Examples

### Create Rule

```rust
use neomind_rules::{Rule, Trigger, Condition, Action, ComparisonOperator};

let rule = Rule {
    id: "temp_alert".to_string(),
    name: "Temperature Alert".to_string(),
    description: "Alert when temperature is too high".to_string(),
    enabled: true,
    trigger: Trigger::Data {
        metric: "temperature".to_string(),
        comparison: ComparisonOperator::Gt,
        value: 30.0,
    },
    conditions: vec![
        Condition {
            left: ConditionValue::DeviceField("location".to_string()),
            operator: ComparisonOperator::Eq,
            right: ConditionValue::String("greenhouse".to_string()),
            logic: None,
        },
    ],
    actions: vec![
        Action::SendAlert {
            severity: AlertSeverity::Warning,
            message: "Temperature too high: {temperature}°C".to_string(),
        },
    ],
    metadata: RuleMetadata::default(),
};

engine.register_rule(rule).await?;
```

### DSL Parsing

```rust
use neomind_rules::RuleParser;

let parser = RuleParser::new();

let rule_text = r#"
ON device.temperature > 30
WHEN device.location == "greenhouse"
THEN send_alert("High temperature")
"#;

let rule = parser.parse(rule_text)?;

engine.register_rule(rule).await?;
```

### Test Rule

```bash
curl -X POST http://localhost:3000/api/rules/test \
  -H "Content-Type: application/json" \
  -d '{
    "rule": "ON device.temperature > 30 THEN send_alert(\"High\")",
    "context": {
      "device": {
        "temperature": 32,
        "location": "greenhouse"
      }
    }
  }'
```

## Design Principles

1. **DSL-First**: Use concise DSL syntax
2. **Testable**: All rules can be tested
3. **Event-Driven**: Rules trigger via EventBus
4. **Composable**: Support complex condition combinations
EOF
