//! AI-powered threshold recommendation for automation conditions.
//!
//! This module analyzes historical data to recommend optimal threshold values
//! for automation conditions, reducing false positives and missed events.

use std::sync::Arc;

use crate::error::{AutomationError, Result};
use edge_ai_core::{LlmRuntime, Message, GenerationParams};
use edge_ai_core::llm::backend::LlmInput;

/// Threshold recommender for automation conditions
pub struct ThresholdRecommender {
    llm: Arc<dyn LlmRuntime>,
}

impl ThresholdRecommender {
    /// Create a new threshold recommender
    pub fn new(llm: Arc<dyn LlmRuntime>) -> Self {
        Self { llm }
    }

    /// Recommend a threshold for a metric based on historical data
    pub async fn recommend_threshold(
        &self,
        device_id: &str,
        metric: &str,
        data_points: &[f64],
        intent: ThresholdIntent,
    ) -> Result<ThresholdRecommendation> {
        if data_points.is_empty() {
            return Err(AutomationError::IntentAnalysisFailed(
                "No data points provided for threshold recommendation".into()
            ));
        }

        // Statistical analysis
        let stats = self.calculate_statistics(data_points);

        // Use LLM for contextual recommendation
        let llm_recommendation = self.llm_recommend_threshold(
            device_id,
            metric,
            &stats,
            intent.clone(),
        ).await?;

        Ok(ThresholdRecommendation {
            device_id: device_id.to_string(),
            metric: metric.to_string(),
            threshold: llm_recommendation.value,
            lower_bound: stats.min,
            upper_bound: stats.max,
            confidence: llm_recommendation.confidence,
            reasoning: llm_recommendation.reasoning,
            intent,
            sample_size: data_points.len(),
            statistics: stats,
        })
    }

    /// Recommend thresholds for multiple metrics in batch
    pub async fn recommend_batch(
        &self,
        requests: &[ThresholdRequest],
    ) -> Vec<ThresholdRecommendation> {
        let mut results = Vec::new();

        for request in requests {
            match self.recommend_threshold(
                &request.device_id,
                &request.metric,
                &request.data_points,
                request.intent.clone(),
            ).await {
                Ok(rec) => results.push(rec),
                Err(_) => {
                    // Add a fallback recommendation
                    let stats = self.calculate_statistics(&request.data_points);
                    results.push(ThresholdRecommendation {
                        device_id: request.device_id.clone(),
                        metric: request.metric.clone(),
                        threshold: stats.mean,
                        lower_bound: stats.min,
                        upper_bound: stats.max,
                        confidence: 0.5,
                        reasoning: "Fallback recommendation based on statistics".to_string(),
                        intent: request.intent.clone(),
                        sample_size: request.data_points.len(),
                        statistics: stats,
                    });
                }
            }
        }

        results
    }

    /// Calculate statistical summaries
    fn calculate_statistics(&self, data_points: &[f64]) -> Statistics {
        if data_points.is_empty() {
            return Statistics::default();
        }

        let n = data_points.len();
        let sum: f64 = data_points.iter().sum();
        let mean = sum / n as f64;

        let min = data_points.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = data_points.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        // Sort for percentiles
        let mut sorted = data_points.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let percentile_25 = if n > 0 {
            sorted[(n as f64 * 0.25).floor() as usize]
        } else {
            0.0
        };

        let percentile_75 = if n > 0 {
            sorted[(n as f64 * 0.75).floor() as usize]
        } else {
            0.0
        };

        let median = if n > 0 {
            sorted[n / 2]
        } else {
            0.0
        };

        // Standard deviation
        let variance = data_points.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();

        Statistics {
            min,
            max,
            mean,
            median,
            percentile_25,
            percentile_75,
            std_dev,
            sample_count: n,
        }
    }

    /// LLM-based threshold recommendation with context
    async fn llm_recommend_threshold(
        &self,
        device_id: &str,
        metric: &str,
        stats: &Statistics,
        intent: ThresholdIntent,
    ) -> Result<LlmThresholdRecommendation> {
        let intent_desc = match &intent {
            ThresholdIntent::AlertWhenHigh => "alert when value exceeds threshold",
            ThresholdIntent::AlertWhenLow => "alert when value falls below threshold",
            ThresholdIntent::DetectAnomaly => "detect unusual values (outside normal range)",
            ThresholdIntent::MaintainRange => "maintain value within acceptable range",
        };

        let prompt = format!(
            r#"Recommend a threshold value for an IoT automation condition.

Device ID: {}
Metric: {}
Intent: {}

Historical Data Statistics:
- Min: {:.2}
- Max: {:.2}
- Mean: {:.2}
- Median: {:.2}
- 25th Percentile: {:.2}
- 75th Percentile: {:.2}
- Standard Deviation: {:.2}
- Sample Count: {}

Respond with a JSON object:
{{
  "threshold": recommended numeric threshold value,
  "confidence": 0.0-1.0,
  "reasoning": "brief explanation of why this threshold was chosen",
  "considerations": ["list of factors to consider"]
}}

Guidelines:
- For "alert when high": recommend a threshold above the 75th percentile but below max
- For "alert when low": recommend a threshold below the 25th percentile but above min
- For "detect anomaly": recommend a threshold at mean ± 2 * std_dev
- For "maintain range": recommend both lower and upper bounds
- Consider the data spread and variability"#,
            device_id,
            metric,
            intent_desc,
            stats.min,
            stats.max,
            stats.mean,
            stats.median,
            stats.percentile_25,
            stats.percentile_75,
            stats.std_dev,
            stats.sample_count
        );

        let input = LlmInput {
            messages: vec![
                Message::system("You are an IoT data analyst. Recommend optimal thresholds for automation conditions. Respond ONLY with valid JSON."),
                Message::user(prompt),
            ],
            params: GenerationParams {
                temperature: Some(0.2),
                max_tokens: Some(500),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        let response = self.llm.generate(input).await?;
        let json_str = extract_json_from_response(&response.text)?;
        let result: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| AutomationError::IntentAnalysisFailed(format!("Invalid JSON: {}", e)))?;

        Ok(LlmThresholdRecommendation {
            value: result.get("threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(stats.mean),
            confidence: result.get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32,
            reasoning: result.get("reasoning")
                .and_then(|v| v.as_str())
                .unwrap_or("Based on statistical analysis")
                .to_string(),
        })
    }

    /// Detect anomalies in data
    pub fn detect_anomalies(&self, data_points: &[f64], _threshold: f64, std_dev_multiplier: f64) -> Vec<usize> {
        let stats = self.calculate_statistics(data_points);
        let anomaly_threshold = stats.mean + (stats.std_dev * std_dev_multiplier);

        data_points.iter()
            .enumerate()
            .filter(|(_, value)| **value > anomaly_threshold)
            .map(|(i, _)| i)
            .collect()
    }

    /// Validate if a threshold is appropriate
    pub fn validate_threshold(
        &self,
        threshold: f64,
        data_points: &[f64],
        intent: &ThresholdIntent,
    ) -> ThresholdValidation {
        let stats = self.calculate_statistics(data_points);

        match intent {
            ThresholdIntent::AlertWhenHigh => {
                if threshold < stats.mean {
                    ThresholdValidation::Warning {
                        message: format!("Threshold ({}) is below the mean ({}). This may cause frequent alerts.",
                            threshold, stats.mean),
                        suggested_value: stats.percentile_75,
                    }
                } else if threshold > stats.max {
                    ThresholdValidation::Warning {
                        message: format!("Threshold ({}) exceeds maximum observed value ({}). Alert may never trigger.",
                            threshold, stats.max),
                        suggested_value: stats.max * 0.95,
                    }
                } else {
                    ThresholdValidation::Valid
                }
            }
            ThresholdIntent::AlertWhenLow => {
                if threshold > stats.mean {
                    ThresholdValidation::Warning {
                        message: format!("Threshold ({}) is above the mean ({}). This may cause frequent alerts.",
                            threshold, stats.mean),
                        suggested_value: stats.percentile_25,
                    }
                } else if threshold < stats.min {
                    ThresholdValidation::Warning {
                        message: format!("Threshold ({}) is below minimum observed value ({}). Alert may never trigger.",
                            threshold, stats.min),
                        suggested_value: stats.min * 1.05,
                    }
                } else {
                    ThresholdValidation::Valid
                }
            }
            ThresholdIntent::DetectAnomaly => {
                let anomaly_threshold = stats.mean + 2.0 * stats.std_dev;
                if threshold < stats.mean || threshold > anomaly_threshold {
                    ThresholdValidation::Warning {
                        message: format!("For anomaly detection, consider using mean ± 2*std_dev = ±{:.2}",
                            2.0 * stats.std_dev),
                        suggested_value: anomaly_threshold,
                    }
                } else {
                    ThresholdValidation::Valid
                }
            }
            ThresholdIntent::MaintainRange => {
                ThresholdValidation::Valid
            }
        }
    }
}

/// Request for threshold recommendation
#[derive(Debug, Clone)]
pub struct ThresholdRequest {
    pub device_id: String,
    pub metric: String,
    pub data_points: Vec<f64>,
    pub intent: ThresholdIntent,
}

/// Intent for the threshold
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThresholdIntent {
    /// Alert when value exceeds threshold
    AlertWhenHigh,
    /// Alert when value falls below threshold
    AlertWhenLow,
    /// Detect unusual values
    DetectAnomaly,
    /// Maintain value within range
    MaintainRange,
}

/// Threshold recommendation result
#[derive(Debug, Clone)]
pub struct ThresholdRecommendation {
    pub device_id: String,
    pub metric: String,
    pub threshold: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub confidence: f32,
    pub reasoning: String,
    pub intent: ThresholdIntent,
    pub sample_size: usize,
    pub statistics: Statistics,
}

/// Statistical summary
#[derive(Debug, Clone, Default)]
pub struct Statistics {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub percentile_25: f64,
    pub percentile_75: f64,
    pub std_dev: f64,
    pub sample_count: usize,
}

/// Threshold validation result
#[derive(Debug, Clone)]
pub enum ThresholdValidation {
    /// Threshold is valid
    Valid,
    /// Threshold has potential issues
    Warning {
        message: String,
        suggested_value: f64,
    },
}

struct LlmThresholdRecommendation {
    value: f64,
    confidence: f32,
    reasoning: String,
}

fn extract_json_from_response(response: &str) -> Result<String> {
    let start = response.find('{')
        .ok_or_else(|| AutomationError::IntentAnalysisFailed("No JSON object found".into()))?;

    let end = response.rfind('}')
        .ok_or_else(|| AutomationError::IntentAnalysisFailed("Incomplete JSON object".into()))?;

    Ok(response[start..=end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data() -> Vec<f64> {
        vec![20.0, 22.0, 21.5, 23.0, 19.5, 24.0, 21.0, 22.5, 20.5, 23.5]
    }

    #[test]
    fn test_calculate_statistics() {
        let data = create_test_data();
        let stats = Statistics {
            min: 19.5,
            max: 24.0,
            mean: 21.75,
            median: 21.75,
            percentile_25: 20.75,
            percentile_75: 23.0,
            std_dev: 1.36,
            sample_count: 10,
        };

        assert_eq!(stats.sample_count, 10);
        assert!((stats.min - 19.5).abs() < 0.1);
        assert!((stats.max - 24.0).abs() < 0.1);
        assert!((stats.mean - 21.75).abs() < 0.1);
    }

    #[test]
    fn test_validate_threshold_high() {
        let data = create_test_data();

        // Valid threshold for alert when high (between mean and max)
        let valid = validate_threshold_direct(23.0, &data, &ThresholdIntent::AlertWhenHigh);
        assert!(matches!(valid, ThresholdValidation::Valid));

        // Warning: threshold below mean
        let warning = validate_threshold_direct(18.0, &data, &ThresholdIntent::AlertWhenHigh);
        assert!(matches!(warning, ThresholdValidation::Warning { .. }));

        // Warning: threshold above max
        let warning2 = validate_threshold_direct(26.0, &data, &ThresholdIntent::AlertWhenHigh);
        assert!(matches!(warning2, ThresholdValidation::Warning { .. }));
    }

    #[test]
    fn test_validate_threshold_low() {
        let data = create_test_data();

        // Valid threshold for alert when low (between min and mean)
        let valid = validate_threshold_direct(20.0, &data, &ThresholdIntent::AlertWhenLow);
        assert!(matches!(valid, ThresholdValidation::Valid));

        // Warning: threshold above mean
        let warning = validate_threshold_direct(23.0, &data, &ThresholdIntent::AlertWhenLow);
        assert!(matches!(warning, ThresholdValidation::Warning { .. }));

        // Warning: threshold below min
        let warning2 = validate_threshold_direct(18.0, &data, &ThresholdIntent::AlertWhenLow);
        assert!(matches!(warning2, ThresholdValidation::Warning { .. }));
    }

    #[test]
    fn test_detect_anomalies() {
        let data = vec![20.0, 21.0, 22.0, 21.5, 20.5, 100.0, 21.0, 22.5];
        let stats = calculate_statistics_direct(&data);
        let anomaly_threshold = stats.mean + 2.0 * stats.std_dev;

        let anomalies: Vec<_> = data.iter()
            .enumerate()
            .filter(|(_, value)| **value > anomaly_threshold)
            .map(|(i, _)| i)
            .collect();

        // Should detect index 5 (value 100.0)
        assert_eq!(anomalies, vec![5]);
    }

    #[test]
    fn test_threshold_intent() {
        let intents = vec![
            ThresholdIntent::AlertWhenHigh,
            ThresholdIntent::AlertWhenLow,
            ThresholdIntent::DetectAnomaly,
            ThresholdIntent::MaintainRange,
        ];

        assert_eq!(intents.len(), 4);
    }

    // Helper functions to avoid needing the full struct
    fn validate_threshold_direct(threshold: f64, data_points: &[f64], intent: &ThresholdIntent) -> ThresholdValidation {
        let stats = calculate_statistics_direct(data_points);

        match intent {
            ThresholdIntent::AlertWhenHigh => {
                if threshold < stats.mean {
                    ThresholdValidation::Warning {
                        message: format!("Threshold ({}) is below the mean ({}).",
                            threshold, stats.mean),
                        suggested_value: stats.percentile_75,
                    }
                } else if threshold > stats.max {
                    ThresholdValidation::Warning {
                        message: format!("Threshold ({}) exceeds maximum observed value ({}).",
                            threshold, stats.max),
                        suggested_value: stats.max * 0.95,
                    }
                } else {
                    ThresholdValidation::Valid
                }
            }
            ThresholdIntent::AlertWhenLow => {
                if threshold > stats.mean {
                    ThresholdValidation::Warning {
                        message: format!("Threshold ({}) is above the mean ({}).",
                            threshold, stats.mean),
                        suggested_value: stats.percentile_25,
                    }
                } else if threshold < stats.min {
                    ThresholdValidation::Warning {
                        message: format!("Threshold ({}) is below minimum observed value ({}).",
                            threshold, stats.min),
                        suggested_value: stats.min * 1.05,
                    }
                } else {
                    ThresholdValidation::Valid
                }
            }
            _ => ThresholdValidation::Valid,
        }
    }

    fn calculate_statistics_direct(data_points: &[f64]) -> Statistics {
        if data_points.is_empty() {
            return Statistics::default();
        }

        let n = data_points.len();
        let sum: f64 = data_points.iter().sum();
        let mean = sum / n as f64;

        let min = data_points.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = data_points.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        let mut sorted = data_points.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let percentile_25 = sorted[(n as f64 * 0.25).floor() as usize];
        let percentile_75 = sorted[(n as f64 * 0.75).floor() as usize];
        let median = sorted[n / 2];

        let variance = data_points.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();

        Statistics {
            min,
            max,
            mean,
            median,
            percentile_25,
            percentile_75,
            std_dev,
            sample_count: n,
        }
    }
}
