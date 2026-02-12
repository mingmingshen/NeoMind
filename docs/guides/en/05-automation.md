# Automation Module

**Package**: `neomind-automation`
**Version**: 0.5.8
**Completion**: 75%
**Purpose**: Data transformation, automation, and intent analysis

## Overview

The Automation module provides a data transformation engine, natural language to automation, and device type generation.

## Important Changes (v0.5.x)

### Transform Metrics Storage
Transform-generated virtual metrics are now unified in `data/timeseries.redb`:

```
DataSourceId: "transform:{transform_id}:{metric_name}"
- device_part: "transform:{transform_id}"
- metric_part: "{metric_name}"
```

Example:
```
transform:avg_temperature:temperature_avg
transform:humidity_calc:indoor_humidity
```

This makes Transform metrics accessible by Agent, Rules, and other modules.

## Module Structure

```
crates/automation/src/
├── lib.rs                      # Public interface
├── transform.rs                # Transform engine
├── types.rs                    # Type definitions
├── conversion.rs               # Type conversion
├── discovery.rs                # Data discovery
├── intent.rs                   # Intent analysis
├── nl2automation.rs            # NL2Automation
├── threshold_recommender.rs    # Threshold recommender
├── device_type_generator.rs    # Device type generator
└── store.rs                    # Storage layer
```

## Core Features

### 1. TransformEngine - Transformation Engine

```rust
pub struct TransformEngine {
    /// JS execution environment
    js_runtime: Rc<RefCell<JsRuntime>>,
}
```

```rust
impl TransformEngine {
    /// Create transform engine
    pub fn new() -> Self;

    /// Execute transform
    pub async fn execute(
        &self,
        transform: &TransformAutomation,
        input: &serde_json::Value,
    ) -> Result<TransformResult>;

    /// Validate transform
    pub fn validate(&self, transform: &TransformAutomation) -> Result<()>;
}
```

### 2. TransformAutomation - Transform Definition

```rust
pub struct TransformAutomation {
    /// Transform ID
    pub id: String,

    /// Transform name
    pub name: String,

    /// Transform scope
    pub scope: TransformScope,
}
```

```rust
pub enum TransformScope {
    /// Specific device
    Device(String),

    /// Device type
    DeviceType(String),

    /// Global
    Global,
}
```

### 3. TransformOperation - Transform Operation

```rust
pub enum TransformOperation {
    /// Field mapping
    Map {
        mappings: HashMap<String, String>,
    },

    /// Time window aggregation
    TimeWindow {
        window: TimeWindow,
        aggregation: AggregationFunc,
    },

    /// Array aggregation
    ArrayAggregation {
        json_path: String,
        aggregation: AggregationFunc,
        value_path: Option<String>,
        output_metric: String,
    },

    /// JavaScript expression
    Expression {
        code: String,
    },

    /// Pipeline
    Pipeline {
        stages: Vec<TransformOperation>,
    },

    /// Conditional branch
    If {
        condition: String,
        then_op: Box<TransformOperation>,
        else_op: Option<Box<TransformOperation>>,
    },

    /// Fork execution
    Fork {
        branches: Vec<TransformOperation>,
    },

    /// Custom WASM
    Custom {
        wasm_module: Vec<u8>,
        function_name: String,
    },
}
```

### 4. JsTransformExecutor - JS Executor

```rust
pub struct JsTransformExecutor {
    /// Boa JS runtime
    runtime: Rc<RefCell<JsRuntime>>,
}
```

```rust
impl JsTransformExecutor {
    /// Execute JS expression
    pub fn execute(
        &self,
        code: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value>;

    /// Register custom function
    pub fn register_function(
        &mut self,
        name: &str,
        func: NativeFunction,
    );
}
```

**Built-in JS Functions**:
```javascript
// Math functions
Math.abs(x)
Math.floor(x)
Math.ceil(x)
Math.round(x)

// String functions
str.toUpperCase(s)
str.toLowerCase(s)
str.substring(s, start, end)

// Array functions
arr.sum(array)
arr.avg(array)
arr.max(array)
arr.min(array)

// Time functions
time.now()
time.format(timestamp, format)
```

## Data Discovery

```rust
pub struct DataPathExtractor {
    /// JSON path extractor
    extractor: JsonPathExtractor,
}
```

```rust
impl DataPathExtractor {
    /// Extract paths from sample data
    pub fn extract_paths(
        &self,
        data: &serde_json::Value,
    ) -> Vec<DiscoveredPath>;

    /// Infer semantic type
    pub fn infer_semantic_type(
        &self,
        path: &str,
        value: &serde_json::Value,
    ) -> SemanticType;
}
```

```rust
pub enum SemanticType {
    Temperature,
    Humidity,
    Pressure,
    Boolean,
    Enum(Vec<String>),
    Unknown,
}
```

## NL2Automation

```rust
pub struct Nl2Automation {
    /// LLM runtime
    llm: Arc<dyn LlmRuntime>,
}
```

```rust
impl Nl2Automation {
    /// Generate automation from natural language
    pub async fn generate(
        &self,
        description: &str,
    ) -> Result<SuggestedAutomation>;

    /// Extract entities
    pub async fn extract_entities(
        &self,
        text: &str,
    ) -> Result<ExtractedEntities>;
}
```

```rust
pub struct ExtractedEntities {
    pub triggers: Vec<TriggerEntity>,
    pub conditions: Vec<ConditionEntity>,
    pub actions: Vec<ActionEntity>,
}
```

## Threshold Recommender

```rust
pub struct ThresholdRecommender {
    /// History data window
    window_size: usize,
}
```

```rust
impl ThresholdRecommender {
    /// Analyze data and recommend threshold
    pub async fn recommend(
        &self,
        data: &[f64],
        intent: ThresholdIntent,
    ) -> ThresholdRecommendation;

    /// Validate threshold reasonableness
    pub fn validate(
        &self,
        threshold: f64,
        statistics: &Statistics,
    ) -> ThresholdValidation;
}
```

```rust
pub enum ThresholdIntent {
    /// Detect abnormally high values
    DetectHigh,

    /// Detect abnormally low values
    DetectLow,

    /// Detect outlier values
    DetectOutliers,

    /// Detect trend changes
    DetectTrendChange,
}
```

## Device Type Generator

```rust
pub struct DeviceTypeGenerator {
    /// LLM runtime
    llm: Arc<dyn LlmRuntime>,
}
```

```rust
impl DeviceTypeGenerator {
    /// Generate device type from sample data
    pub async fn generate_from_sample(
        &self,
        sample_data: &serde_json::Value,
        device_info: &DeviceInfo,
    ) -> Result<GeneratedDeviceType>;

    /// Validate generated type
    pub fn validate(
        &self,
        device_type: &DeviceTypeTemplate,
    ) -> ValidationResult;
}
```

```rust
pub struct GeneratedDeviceType {
    pub device_type: String,
    pub name: String,
    pub description: String,
    pub metrics: Vec<MetricDefinition>,
    pub commands: Vec<CommandDefinition>,
}
```

## Auto-Onboarding

```rust
pub struct AutoOnboardManager {
    /// Device registry
    registry: Arc<DeviceRegistry>,

    /// Generator
    generator: DeviceTypeGenerator,

    /// Threshold recommender
    recommender: ThresholdRecommender,
}
```

```rust
impl AutoOnboardManager {
    /// Process draft device
    pub async fn process_draft_device(
        &self,
        draft: DraftDevice,
    ) -> Result<RegistrationResult>;

    /// Generate device type from sample
    pub async fn generate_device_type(
        &self,
        sample: &DeviceSample,
    ) -> Result<GeneratedDeviceType>;
}
```

```rust
pub struct DraftDevice {
    pub id: String,
    pub source: String,
    pub sample_data: serde_json::Value,
    pub status: DraftDeviceStatus,
}
```

```rust
pub enum DraftDeviceStatus {
    Pending,
    Processing,
    Ready,
    Failed(String),
}
```

## API Endpoints

```
# Transforms (part of unified automation API)
GET    /api/automations/transforms              # List transforms
POST   /api/automations/transforms              # Create transform
GET    /api/automations/transforms/:id          # Get transform
PUT    /api/automations/transforms/:id          # Update transform
DELETE /api/automations/transforms/:id          # Delete transform
POST   /api/automations/transforms/:id/test     # Test transform
POST   /api/automations/transforms/process      # Process data
GET    /api/automations/transforms/metrics      # Get virtual metrics

# Discovery
POST   /api/automations/analyze-intent          # Intent analysis
POST   /api/device-types/generate-from-samples  # Generate device types

# Thresholds
POST   /api/thresholds/recommend                # Recommend threshold
POST   /api/thresholds/validate                 # Validate threshold

# Draft Devices (auto-onboarding)
GET    /api/devices/drafts                      # List drafts
GET    /api/devices/drafts/:id                  # Get draft
PUT    /api/devices/drafts/:id                  # Update draft
POST   /api/devices/drafts/:id/approve          # Approve device
POST   /api/devices/drafts/:id/reject           # Reject device
POST   /api/devices/drafts/:id/analyze          # LLM analysis
POST   /api/devices/drafts/cleanup              # Cleanup drafts
```

## Usage Examples

### Create Data Transform

```rust
use neomind_automation::{TransformAutomation, TransformOperation, TransformScope, AggregationFunc};

let transform = TransformAutomation::new(
    "avg_temperature",
    "Calculate average temperature",
    TransformScope::DeviceType("sensor".to_string()),
)
.with_operation(TransformOperation::ArrayAggregation {
    json_path: "$.sensors".to_string(),
    aggregation: AggregationFunc::Mean,
    value_path: Some("temp".to_string()),
    output_metric: "temperature_avg".to_string(),
});

let result = engine.execute(&transform, &input_data).await?;
```

### JavaScript Expression

```rust
let transform = TransformAutomation::new(
    "temp_conversion",
    "Temperature unit conversion",
    TransformScope::Global,
)
.with_operation(TransformOperation::Expression {
    code: r#"
        // Fahrenheit to Celsius
        input.temp * 1.8 + 32
    "#.to_string(),
});

let result = engine.execute(&transform, &input_data).await?;
```

### Natural Language Rule Generation

```rust
use neomind_automation::Nl2Automation;

let nl2auto = Nl2Automation::new(llm);

let suggested = nl2auto.generate(
    "Send alert when temperature exceeds 30 degrees"
).await?;

// suggested contains:
// - trigger: DeviceMetric { metric: "temperature", compare: Gt, value: 30 }
// - condition: ...
// - action: SendAlert { message: "Temperature too high" }
```

### Threshold Recommendation

```rust
use neomind_automation::{ThresholdRecommender, ThresholdIntent};

let recommender = ThresholdRecommender::new(100);

let data = vec![22.5, 23.1, 22.8, 23.5, 22.9, 23.2];

let recommendation = recommender.recommend(&data, ThresholdIntent::DetectHigh).await?;

println!("Recommended threshold: {}", recommendation.threshold);
println!("Confidence: {}", recommendation.confidence);
```

## Transform Operation Status

| Operation | Status | Description |
|-----------|--------|-------------|
| Map | ✅ | Field mapping complete |
| TimeWindow | ✅ | Time window aggregation complete |
| ArrayAggregation | ✅ | Array aggregation complete |
| Expression | ✅ | JS expression execution complete |
| Pipeline | 🟡 | Basic implementation |
| Fork | 🟡 | Basic implementation |
| If | 🟡 | Basic implementation |
| Custom/WASM | ❌ | Not implemented |

## Design Principles

1. **JS-First**: Use JavaScript as transformation language
2. **Type Inference**: Automatically infer data types
3. **Natural Language**: Support automation generation from natural language
4. **Testable**: All transforms can be tested
EOF
