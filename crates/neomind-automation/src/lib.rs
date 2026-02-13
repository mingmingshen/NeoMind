//! Edge AI Unified Automation Crate
//!
//! This crate provides a unified abstraction for transforms and rules in the NeoMind platform.
//!
//! ## Features
//!
//! - **Transform Layer**: Process raw device data into usable metrics
//! - **Unified Types**: Single `Automation` enum that wraps transforms and rules
//! - **Intent Analysis**: AI-powered recommendation of transform/rule based on natural language
//! - **Shared Resources**: Common templates, devices, and metrics for all types
//!
//! ## Example
//!
//! ```rust,no_run,ignore
//! use neomind_automation::{Automation, AutomationType, TransformScope, TransformOperation};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a transform to process sensor array data
//!     let transform = TransformAutomation::new("avg-temp", "Average Temperature", TransformScope::DeviceType("sensor".to_string()))
//!         .with_operation(TransformOperation::ArrayAggregation {
//!             json_path: "$.sensors",
//!             aggregation: AggregationFunc::Mean,
//!             value_path: Some("temp".to_string()),
//!             output_metric: "temperature_avg".to_string(),
//!         });
//!
//!     // Analyze intent to determine automation type
//!     let intent = analyzer.analyze("When temperature exceeds 30°C, send an alert").await?;
//!
//!     Ok(())
//! }
//! ```

pub mod conversion;
pub mod device_type_generator;
pub mod discovery;
pub mod error;
pub mod intent;
pub mod nl2automation;
pub mod output_registry;
pub mod store;
pub mod threshold_recommender;
pub mod transform;
pub mod types;

// Re-export all types
pub use types::*;

// Re-export core types with common aliases
pub use conversion::AutomationConverter;
pub use device_type_generator::{
    DeviceCapabilities, DeviceTypeGenerator, GenerationConfig, ValidationResult,
};
pub use error::{AutomationError, Result};
pub use nl2automation::{
    ActionEntity, ActionTypeEntity, ConditionEntity, DeviceInfo, ExtractedEntities,
    ExtractionContext, Language, MetricInfo, Nl2Automation, TimeConstraints, TriggerEntity,
    TriggerTypeEntity,
};
pub use threshold_recommender::{
    Statistics, ThresholdIntent, ThresholdRecommendation, ThresholdRecommender, ThresholdRequest,
    ThresholdValidation,
};

// Re-export discovery module
pub use discovery::{
    AutoOnboardManager, DataPathExtractor, DeviceCategory, DeviceSample, DiscoveredMetric,
    DiscoveredPath, DraftDevice, DraftDeviceStatus, FieldSemantic, GeneratedDeviceType,
    InferenceContext, ProcessingSummary, RegistrationResult, SemanticInference, SemanticType,
    TypeSignature, VirtualMetricGenerator,
};

// Re-export transform-specific types
pub use types::{
    Action, AggregationFunc, AlertSeverity, Automation, AutomationFilter, AutomationList,
    AutomationMetadata, AutomationTemplate, AutomationType, ComparisonOperator, Condition,
    ExecutionRecord, ExecutionStatus, IntentResult, LogLevel, RuleAutomation, SuggestedAutomation,
    TemplateParameter, TemplateParameterType, TimeWindow, TransformAutomation, TransformOperation,
    TransformScope, Trigger, TriggerType, TypeCounts,
};

// Re-export transform engine
pub use transform::{TransformEngine, TransformResult, TransformedMetric};

// Re-export output registry
pub use output_registry::{
    TransformDataSourceInfo, TransformDataSourcesResponse, TransformOutputInfo,
    TransformOutputRegistry, TransformOutputType,
};
