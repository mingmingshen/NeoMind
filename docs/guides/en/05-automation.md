# Automation Module

**Package**: `neomind-rules` (rule generation), `neomind-devices` (auto-onboarding)
**Version**: 0.8.0
**Completion**: 85%
**Purpose**: Data transformation, rule generation from NL, auto-onboarding, and device type generation

## Overview

The Automation module provides rule generation from natural language, device type generation from samples, and auto-onboarding for discovered devices. These features are distributed across the `neomind-rules` and `neomind-devices` crates.

## Important Changes (v0.8.0)

### Unified Architecture

Automation features are now integrated into the core crates rather than a separate `neomind-automation` crate:
- **Rule Generation**: LLM-based rule generation is in `neomind-rules/src/generator.rs`
- **Auto-Onboarding**: Draft device management is in `neomind-devices/src/service.rs`
- **Device Type Generation**: LLM-based type generation is in the devices API layer
- **Rule Validation**: Context-aware validation is in `neomind-rules/src/validator.rs`

## Module Structure

```
crates/neomind-rules/src/
├── generator.rs                # LLM-based rule generation from NL
├── validator.rs                # Rule validation with context awareness
├── device_integration.rs       # Device action execution from rules
├── extension_integration.rs    # Extension action execution from rules
└── dsl.rs                      # DSL parser (RULE...WHEN...DO...END)

crates/neomind-devices/src/
├── service.rs                  # DeviceService with auto-onboarding
├── registry.rs                 # DeviceRegistry with type management
└── adapters/                   # Adapters with auto-discovery
```

## Core Features

### 1. Rule Generation from Natural Language

```rust
// In neomind-rules/src/generator.rs

pub struct GeneratorConfig {
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

/// Extracted information from natural language description
pub struct ExtractedRuleInfo {
    pub name: String,
    pub device_id: Option<String>,
    pub metric: Option<String>,
    pub operator: Option<ComparisonOperator>,
    pub threshold: Option<f64>,
    pub action_type: Option<ActionType>,
    pub message: Option<String>,
}
```

### 2. Rule Validation with Context

```rust
// In neomind-rules/src/validator.rs

pub struct RuleValidator {
    // Validates rules against available devices, metrics, commands
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
```

### 3. Auto-Onboarding Pipeline

```
Adapter Discovery -> Draft Device -> LLM Analysis -> Type Suggestion -> User Approval -> Full Device
```

The auto-onboarding flow:
1. Adapters discover new devices and emit `DeviceEvent::Discovery`
2. Discovered devices are stored as draft devices
3. LLM analyzes sample data to suggest device types
4. User reviews and approves/rejects
5. Approved drafts become full device instances

### 4. Device Type Generation from Samples

```
POST /api/device-types/generate-from-samples
```

Generates device type templates from sample data using LLM analysis.

## Rule DSL Syntax

### Basic Rule

```neo
RULE "Temperature Alert"
WHEN sensor.temperature > 50
FOR 5 minutes
DO
    NOTIFY "Device temperature too high: {temperature}C"
    EXECUTE device.fan(speed=100)
    LOG alert, severity="high"
END
```

### Extension Rule

```neo
RULE "Weather Alert"
WHEN EXTENSION weather.temperature > 30
DO
    NOTIFY "Weather too hot"
END
```

### Complex Rule with AND/OR

```neo
RULE "Compound Condition Alert"
WHEN (sensor.temperature > 30) AND (EXTENSION weather.humidity < 20)
DO
    NOTIFY "High temperature and low humidity"
    EXECUTE device.humidifier(on=true)
END
```

### Rule with Range Condition

```neo
RULE "Temperature Range Alert"
WHEN sensor.temperature BETWEEN 20 AND 25
DO
    NOTIFY "Temperature in comfort range"
END
```

### Scheduled Rule

```neo
RULE "Periodic Check"
TRIGGER SCHEDULE "0 */5 * * * *"
DO
    EXECUTE device.read_sensors()
END
```

## API Endpoints

```
# Rule Generation
POST   /api/rules/validate                  # Validate rule DSL

# Device Type Generation
POST   /api/device-types/generate-from-samples  # Generate device types from samples

# Auto-Onboarding (Drafts)
GET    /api/devices/drafts                      # List drafts
GET    /api/devices/drafts/:device_id           # Get draft
PUT    /api/devices/drafts/:device_id           # Update draft
POST   /api/devices/drafts/:device_id/approve   # Approve device
POST   /api/devices/drafts/:device_id/reject    # Reject device
POST   /api/devices/drafts/:device_id/analyze   # LLM analysis
POST   /api/devices/drafts/:device_id/enhance   # Enhance with LLM
GET    /api/devices/drafts/:device_id/suggest-types  # Suggest types
POST   /api/devices/drafts/cleanup              # Cleanup drafts
GET    /api/devices/drafts/type-signatures      # Get type signatures
GET    /api/devices/drafts/config               # Get onboard config
PUT    /api/devices/drafts/config               # Update onboard config
POST   /api/devices/drafts/upload               # Upload device data
```

## Usage Examples

### Natural Language Rule Generation

```rust
use neomind_rules::generator::GeneratorConfig;

// The generator uses LLM to convert NL descriptions to DSL rules
// Input: "Send alert when temperature exceeds 30 degrees"
// Output:
// RULE "Temperature Alert"
// WHEN sensor.temperature > 30
// DO
//     NOTIFY "Temperature too high"
// END
```

### Rule Validation

```rust
use neomind_rules::validator::{RuleValidator, ValidationContext};

let validator = RuleValidator::new();
let context = ValidationContext {
    devices: vec![/* available devices */],
    metrics: vec![/* available metrics */],
    commands: vec![/* available commands */],
    alert_channels: vec![/* configured channels */],
};

let result = validator.validate(&rule, &context);
```

## Feature Status

| Feature | Status | Description |
|---------|--------|-------------|
| DSL Rule Engine | Complete | Full DSL parser with RULE/WHEN/DO/END syntax |
| NL Rule Generation | Complete | LLM-based rule generation from natural language |
| Rule Validation | Complete | Context-aware validation against resources |
| Extension Conditions | Complete | EXTENSION metric conditions in rules |
| Device Type Generation | Complete | LLM-based type generation from samples |
| Auto-Onboarding | Complete | Full draft device pipeline |
| Agent Trigger Action | Complete | Rules can trigger AI agents |
| Transform Engine | Planned | Data transformation pipeline |

## Design Principles

1. **LLM-Powered**: Use LLM for NL-to-rule and sample-to-type generation
2. **Context-Aware**: Validate rules against available devices and metrics
3. **DSL-First**: Human-readable rule definition language
4. **Extensible**: Support device and extension conditions in rules
5. **Pipeline**: Auto-onboarding with LLM analysis and user approval
