//! Device data discovery and AI-powered analysis module.
//!
//! This module provides capabilities for:
//! - Extracting data paths from device samples
//! - Inferring semantic meaning of fields
//! - Auto-generating virtual metrics
//! - Generating device type definitions

pub mod types;
pub mod path_extractor;
pub mod semantic_inference;
pub mod virtual_metric;

pub use types::*;
pub use path_extractor::DataPathExtractor;
pub use semantic_inference::SemanticInference;
pub use virtual_metric::{VirtualMetricGenerator, AggregationSuggestion, AggregationOperation, DerivedMetric};
