//! Edge AI Unified Automation Crate
//!
//! This crate provides a unified abstraction for transforms and rules in the NeoMind platform.
//!
//! ## Features
//!
//! - **Transform Layer**: Process raw device data into usable metrics
//! - **Unified Types**: Single `Automation` enum that wraps transforms and rules
//! - **Shared Resources**: Common templates, devices, and metrics for all types

pub mod conversion;
pub mod device_type_generator;
pub mod discovery;
pub mod error;
pub mod output_registry;
pub mod store;
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
