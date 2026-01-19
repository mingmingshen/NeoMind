//! Semantic inference for device data fields.
//!
//! This module uses AI to analyze field names, values, and patterns
//! to infer the semantic meaning of device data fields.

use crate::discovery::types::*;
use edge_ai_core::{LlmRuntime, Message, GenerationParams, llm::backend::LlmInput};
use std::collections::HashMap;
use std::sync::Arc;

/// AI-powered semantic inference for device fields
pub struct SemanticInference {
    llm: Arc<dyn LlmRuntime>,
}

impl SemanticInference {
    /// Create a new semantic inference engine
    pub fn new(llm: Arc<dyn LlmRuntime>) -> Self {
        Self { llm }
    }

    /// Infer semantic meaning for a single field
    pub async fn infer_field_semantic(
        &self,
        field_name: &str,
        field_values: &[serde_json::Value],
        context: &InferenceContext,
    ) -> FieldSemantic {
        // First try rule-based inference
        let first_value = field_values.first().cloned();
        let rule_based = SemanticType::infer_from_context(field_name, &first_value);

        // If rule-based is confident enough, use it
        if rule_based != SemanticType::Unknown {
            let semantic = FieldSemantic::new(
                rule_based.clone(),
                Self::standardize_name(field_name),
                Self::display_name_for_type(&rule_based),
            )
            .with_confidence(0.7)
            .with_source_field(field_name.to_string());

            // For high-confidence rule-based results, return directly
            if matches!(rule_based,
                SemanticType::Temperature | SemanticType::Humidity |
                SemanticType::Battery | SemanticType::Switch |
                SemanticType::Light | SemanticType::Motion)
            {
                return semantic;
            }
        }

        // Otherwise, use LLM for more sophisticated inference
        self.llm_field_inference(field_name, field_values, context).await
            .unwrap_or_else(|| {
                // Fallback to rule-based
                FieldSemantic::new(
                    rule_based,
                    Self::standardize_name(field_name),
                    field_name.to_string(),
                )
                .with_confidence(0.5)
                .with_source_field(field_name.to_string())
            })
    }

    /// Infer semantics for multiple fields in batch
    pub async fn infer_fields_batch(
        &self,
        fields: &HashMap<String, Vec<serde_json::Value>>,
        context: &InferenceContext,
    ) -> HashMap<String, FieldSemantic> {
        let mut results = HashMap::new();

        // Use LLM for batch inference (more efficient)
        let llm_result = self.llm_batch_inference(fields, context).await;

        // Merge results with rule-based fallbacks
        for (field_name, values) in fields {
            let semantic = if let Some(inferred) = llm_result.get(field_name) {
                inferred.clone()
            } else {
                // Fallback to rule-based
                let semantic_type = SemanticType::infer_from_context(field_name, &values.first().cloned());
                FieldSemantic::new(
                    semantic_type,
                    Self::standardize_name(field_name),
                    field_name.to_string(),
                )
                .with_source_field(field_name.to_string())
            };
            results.insert(field_name.clone(), semantic);
        }

        results
    }

    /// Analyze a discovered path and enhance it with semantic information
    pub async fn enhance_path(
        &self,
        path: &DiscoveredPath,
        context: &InferenceContext,
    ) -> DiscoveredMetric {
        // Extract field name from path
        let field_name = Self::extract_field_name(&path.path);

        let semantic = self.infer_field_semantic(
            &field_name,
            &path.sample_values,
            context,
        ).await;

        DiscoveredMetric {
            name: semantic.standard_name.clone(),
            display_name: semantic.display_name.clone(),
            description: semantic.reasoning.clone(),
            path: path.path.clone(),
            data_type: path.data_type.clone(),
            semantic_type: semantic.semantic_type.clone(),
            unit: semantic.recommended_unit,
            value_range: path.value_range.clone(),
            is_readable: true,
            is_writable: false,
            confidence: semantic.confidence,
        }
    }

    /// LLM-based field inference
    async fn llm_field_inference(
        &self,
        field_name: &str,
        field_values: &[serde_json::Value],
        context: &InferenceContext,
    ) -> Option<FieldSemantic> {
        // Prepare sample values for the prompt
        let sample_values: String = field_values.iter()
            .take(5)
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let device_hint = context.device_type_hint
            .as_deref()
            .unwrap_or("unknown device");

        let prompt = format!(
            "Analyze this field from an IoT device and determine its semantic meaning.\n\
            \n\
            Device Type: {}\n\
            Field Name: {}\n\
            Sample Values: {}\n\
            \n\
            Respond with a JSON object in this exact format:\n\
            {{\n\
              \"semantic_type\": \"temperature|humidity|pressure|light|motion|switch|dimmer|color|power|energy|co2|pm25|voc|speed|flow|level|status|error|alarm|battery|rssi|unknown\",\n\
              \"standard_name\": \"standardized English name (e.g., 'temperature', 'humidity')\",\n\
              \"display_name\": \"Chinese display name (e.g., '温度', '湿度')\",\n\
              \"unit\": \"recommended unit or null\",\n\
              \"confidence\": 0.0-1.0,\n\
              \"reasoning\": \"brief explanation\"\n\
            }}",
            device_hint, field_name, sample_values
        );

        let input = LlmInput {
            messages: vec![
                Message::system("You are an IoT data analyst. Analyze device fields and determine their semantic meaning. \
                              Respond ONLY with valid JSON, no additional text."),
                Message::user(prompt),
            ],
            params: GenerationParams {
                temperature: Some(0.1),
                max_tokens: Some(300),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        match self.llm.generate(input).await {
            Ok(output) => {
                // Parse LLM response
                let response = output.text.trim().trim_start_matches("```json").trim_start_matches("```").trim();
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response) {
                    return Some(Self::parse_llm_semantic_result(field_name, parsed));
                }
                None
            }
            Err(_) => None,
        }
    }

    /// LLM-based batch field inference
    async fn llm_batch_inference(
        &self,
        fields: &HashMap<String, Vec<serde_json::Value>>,
        context: &InferenceContext,
    ) -> HashMap<String, FieldSemantic> {
        let mut results = HashMap::new();

        // Limit batch size to avoid overwhelming the LLM
        let batch: Vec<_> = fields.iter().take(10).collect();

        let fields_desc = batch.iter()
            .map(|(name, values)| {
                let samples = values.iter()
                    .take(3)
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("- {}: [{}]", name, samples)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let device_hint = context.device_type_hint
            .as_deref()
            .unwrap_or("unknown device");

        let prompt = format!(
            "Analyze these fields from an IoT device and determine their semantic meanings.\n\
            \n\
            Device Type: {}\n\
            Fields:\n\
            {}\n\
            \n\
            Respond with a JSON object mapping each field name to its semantic analysis:\n\
            {{\n\
              \"field_name\": {{\n\
                \"semantic_type\": \"temperature|humidity|...\",\n\
                \"standard_name\": \"standardized name\",\n\
                \"display_name\": \"Chinese name\",\n\
                \"unit\": \"unit or null\",\n\
                \"confidence\": 0.0-1.0\n\
              }}\n\
            }}",
            device_hint, fields_desc
        );

        let input = LlmInput {
            messages: vec![
                Message::system("You are an IoT data analyst. Analyze device fields. Respond ONLY with valid JSON."),
                Message::user(prompt),
            ],
            params: GenerationParams {
                temperature: Some(0.1),
                max_tokens: Some(800),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        if let Ok(output) = self.llm.generate(input).await {
            let response = output.text.trim().trim_start_matches("```json").trim_start_matches("```").trim();
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response) {
                if let Some(obj) = parsed.as_object() {
                    for (field_name, semantic_data) in obj {
                        let semantic = Self::parse_llm_semantic_result(field_name, semantic_data.clone());
                        results.insert(field_name.clone(), semantic);
                    }
                }
            }
        }

        results
    }

    /// Parse LLM semantic inference result
    fn parse_llm_semantic_result(field_name: &str, value: serde_json::Value) -> FieldSemantic {
        let semantic_type_str = value.get("semantic_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let semantic_type = match semantic_type_str {
            "temperature" => SemanticType::Temperature,
            "humidity" => SemanticType::Humidity,
            "pressure" => SemanticType::Pressure,
            "light" => SemanticType::Light,
            "motion" => SemanticType::Motion,
            "switch" => SemanticType::Switch,
            "dimmer" => SemanticType::Dimmer,
            "color" => SemanticType::Color,
            "power" => SemanticType::Power,
            "energy" => SemanticType::Energy,
            "co2" => SemanticType::Co2,
            "pm25" => SemanticType::Pm25,
            "voc" => SemanticType::Voc,
            "speed" => SemanticType::Speed,
            "flow" => SemanticType::Flow,
            "level" => SemanticType::Level,
            "status" => SemanticType::Status,
            "error" => SemanticType::Error,
            "alarm" => SemanticType::Alarm,
            "battery" => SemanticType::Battery,
            "rssi" => SemanticType::Rssi,
            _ => SemanticType::Unknown,
        };

        let standard_name = value.get("standard_name")
            .and_then(|v| v.as_str())
            .unwrap_or(&Self::standardize_name(field_name))
            .to_string();

        let display_name = value.get("display_name")
            .and_then(|v| v.as_str())
            .unwrap_or(field_name)
            .to_string();

        let confidence = value.get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        let reasoning = value.get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("AI inference")
            .to_string();

        let recommended_unit = value.get("unit")
            .and_then(|v| {
                if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) }
            });

        FieldSemantic {
            semantic_type,
            standard_name,
            display_name,
            recommended_unit,
            confidence,
            reasoning,
            source_fields: vec![field_name.to_string()],
        }
    }

    /// Extract field name from a JSON path
    fn extract_field_name(path: &str) -> String {
        // Get the last component of the path
        let parts: Vec<&str> = path.split('.').collect();
        let last = parts.last().unwrap_or(&path);

        // Remove array indices
        last.split('[')
            .next()
            .unwrap_or(last)
            .to_string()
    }

    /// Standardize a field name
    fn standardize_name(name: &str) -> String {
        name.to_lowercase()
            .replace([' ', '-', '_'], "_")
            .trim_matches('_')
            .to_string()
    }

    /// Get display name for a semantic type
    fn display_name_for_type(semantic_type: &SemanticType) -> String {
        semantic_type.display_name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_field_name() {
        assert_eq!(
            SemanticInference::extract_field_name("payload.sensors[0].v"),
            "v"
        );
        assert_eq!(
            SemanticInference::extract_field_name("temperature"),
            "temperature"
        );
        assert_eq!(
            SemanticInference::extract_field_name("data.values[0]"),
            "values"
        );
    }

    #[test]
    fn test_standardize_name() {
        assert_eq!(
            SemanticInference::standardize_name("Temperature Sensor"),
            "temperature_sensor"
        );
        assert_eq!(
            SemanticInference::standardize_name("humidity-value"),
            "humidity_value"
        );
        assert_eq!(
            SemanticInference::standardize_name("  device_status  "),
            "device_status"
        );
    }

    #[test]
    fn test_parse_llm_semantic_result() {
        let json = serde_json::json!({
            "semantic_type": "temperature",
            "standard_name": "temperature",
            "display_name": "温度",
            "unit": "°C",
            "confidence": 0.9,
            "reasoning": "Field name contains 'temp'"
        });

        let result = SemanticInference::parse_llm_semantic_result("temp_celsius", json);
        assert_eq!(result.semantic_type, SemanticType::Temperature);
        assert_eq!(result.standard_name, "temperature");
        assert_eq!(result.display_name, "温度");
        assert_eq!(result.recommended_unit, Some("°C".to_string()));
        assert_eq!(result.confidence, 0.9);
    }
}
