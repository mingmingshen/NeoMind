use super::*;

pub(crate) fn extract_semantic_patterns(
    decisions: &[Decision],
    situation_analysis: &str,
    _data: &[DataCollected],
    baselines: &HashMap<String, f64>,
) -> Vec<LearnedPattern> {
    let mut patterns = Vec::new();
    let now = chrono::Utc::now().timestamp();

    for decision in decisions {
        if decision.decision_type.is_empty() {
            continue;
        }

        // Extract pattern type
        let pattern_type = match decision.decision_type.as_str() {
            "alert" => "anomaly_detection",
            "command" => "automated_control",
            "info" => "information_logging",
            _ => "general_pattern",
        };

        // Extract symptom (what condition triggered this)
        let symptom = extract_symptom(situation_analysis, decision);

        // Extract threshold/value if applicable
        let threshold = extract_threshold_from_data(_data, baselines);

        // Build pattern data
        let pattern_data = serde_json::json!({
            "symptom": symptom,
            "action": decision.action,
            "threshold": threshold,
            "trigger_conditions": extract_trigger_conditions(decision),
        });

        // Default confidence: higher for alerts and commands
        let confidence = match decision.decision_type.as_str() {
            "alert" | "command" => 0.9,
            _ => 0.8,
        };

        // Optimize ID allocation with pre-allocated capacity
        let id = format!("{}:{}", pattern_type, now);

        let pattern = LearnedPattern {
            id,
            pattern_type: pattern_type.to_string(),
            description: extract_semantic_description(decision, &symptom),
            confidence,
            learned_at: now,
            data: pattern_data,
        };

        patterns.push(pattern);
    }

    patterns
}

pub(crate) fn extract_symptom(situation_analysis: &str, decision: &Decision) -> String {
    // Try to extract from situation analysis - use static strings for common cases
    if !situation_analysis.is_empty() {
        // Look for key phrases indicating conditions
        if situation_analysis.contains("超过")
            || situation_analysis.contains("高于")
            || situation_analysis.contains("exceeds")
            || situation_analysis.contains("above")
        {
            return "Value exceeds threshold".to_string();
        }
        if situation_analysis.contains("低于") || situation_analysis.contains("below") {
            return "Value below threshold".to_string();
        }
        if situation_analysis.contains("异常")
            || situation_analysis.contains("不正常")
            || situation_analysis.contains("abnormal")
        {
            return "Abnormal state detected".to_string();
        }
        if situation_analysis.contains("正常")
            || situation_analysis.contains("稳定")
            || situation_analysis.contains("normal")
            || situation_analysis.contains("stable")
        {
            return "Status normal".to_string();
        }
    }

    // Fallback to decision type - use static strings
    match decision.decision_type.as_str() {
        "alert" => "Alert condition detected".to_string(),
        "command" => "Automation trigger met".to_string(),
        _ => "Routine check".to_string(),
    }
}

pub(crate) fn extract_threshold_from_data(
    data: &[DataCollected],
    baselines: &HashMap<String, f64>,
) -> Option<f64> {
    // Try to extract numeric value from decision description
    for item in data {
        if let Some(val) = item.values.get("value") {
            if let Some(num) = val.as_f64() {
                // Check if baseline exists
                if let Some(&baseline) = baselines.get(&item.source) {
                    let deviation = ((num - baseline) / baseline * 100.0).abs();
                    if deviation > 10.0 {
                        return Some(deviation);
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn extract_trigger_conditions(decision: &Decision) -> serde_json::Value {
    let mut conditions = Vec::new();

    // Use a fixed confidence since Decision doesn't have one
    conditions.push("verified_action".to_string());

    if !decision.action.is_empty() {
        conditions.push(format!("action:{}", decision.action));
    }

    serde_json::json!(conditions)
}

pub(crate) fn extract_semantic_description(decision: &Decision, symptom: &str) -> String {
    // Convert specific descriptions to abstract patterns
    let desc = &decision.description;

    // Pattern: "Temp sensor 1 shows 25 degrees" -> "Temp anomaly triggered alert"
    if desc.contains("温度") || desc.contains("temp") {
        return format!("Temp {} - {}", symptom, decision.action);
    }
    if desc.contains("湿度") || desc.contains("humidity") {
        return format!("Humidity {} - {}", symptom, decision.action);
    }
    if desc.contains("压力") || desc.contains("pressure") {
        return format!("Pressure {} - {}", symptom, decision.action);
    }

    // Generic abstract description
    format!("{} - {}", symptom, decision.action)
}

/// Extract a concise text insight about an image from the LLM's analysis and conclusion.
///
/// Prioritizes content from the situation analysis that describes visual observations,
/// falling back to the conclusion text.
fn extract_image_insight(situation_analysis: &str, conclusion: &str) -> String {
    let max_len = 200;

    // Try to find a visually descriptive segment from situation_analysis
    // Look for common visual description markers
    let visual_markers = [
        ("image shows", "image shows"),
        ("the image", "the image"),
        ("visible", "visible"),
        ("detected", "detected"),
        ("camera", "camera"),
        ("observed", "observed"),
        ("图像", "图像"),
        ("图片", "图片"),
        ("画面", "画面"),
        ("可以看到", "可以看到"),
        ("观察到", "观察到"),
        ("检测到", "检测到"),
        ("发现", "发现"),
        ("显示", "显示"),
    ];

    for (marker, _) in &visual_markers {
        if let Some(pos) = situation_analysis.find(marker) {
            let start = pos;
            // Floor to the nearest char boundary to avoid splitting multi-byte chars
            let mut end = (start + max_len).min(situation_analysis.len());
            while !situation_analysis.is_char_boundary(end) && end > start {
                end -= 1;
            }
            // Try to end at a sentence boundary
            let slice = &situation_analysis[start..end];
            if let Some(dot_pos) = slice.rfind(|c: char| c == '。' || c == '.' || c == '！' || c == '！')
            {
                let trimmed = &slice[..=dot_pos];
                if trimmed.len() >= 20 {
                    return clean_and_truncate_text(trimmed, max_len);
                }
            }
            return clean_and_truncate_text(slice, max_len);
        }
    }

    // Fallback: use conclusion (already cleaned/truncated by caller, but limit here)
    if !conclusion.is_empty() {
        return clean_and_truncate_text(conclusion, max_len);
    }

    // Last resort: first part of situation_analysis
    clean_and_truncate_text(situation_analysis, max_len)
}

impl AgentExecutor {
    pub(crate) async fn update_memory(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
        decisions: &[Decision],
        situation_analysis: &str,
        conclusion: &str,
        execution_id: &str,
        success: bool,
    ) -> AgentResult<AgentMemory> {
        let mut memory = agent.memory.clone();

        // === HIERARCHICAL MEMORY UPDATE ===

        // 1. Update Working Memory with current analysis (always, for current conversation)
        let cleaned_analysis = clean_and_truncate_text(situation_analysis, 500);
        let cleaned_conclusion = clean_and_truncate_text(conclusion, 200);
        memory.set_working_analysis(cleaned_analysis.clone(), cleaned_conclusion.clone());

        // 2. Write gating: skip short-term/long-term for routine/duplicate executions
        let has_alert_or_command = decisions
            .iter()
            .any(|d| matches!(d.decision_type.as_str(), "alert" | "command"));
        let has_anomaly = situation_analysis.to_lowercase().contains("异常")
            || situation_analysis.to_lowercase().contains("abnormal")
            || situation_analysis.to_lowercase().contains("anomaly");

        // Detect image data — each image analysis is valuable, never treat as routine
        let has_image_analysis = data.iter().any(|d| {
            d.values
                .get("_is_image")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        });

        // Build image analysis summaries from conclusion for image data
        let image_analyses: Vec<String> = if has_image_analysis {
            data.iter()
                .filter(|d| {
                    d.values
                        .get("_is_image")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                })
                .map(|d| {
                    let insight = extract_image_insight(situation_analysis, conclusion);
                    format!("[image_analysis] {}: {}", d.source, insight)
                })
                .collect()
        } else {
            Vec::new()
        };

        // Fingerprint-based duplicate detection: if the last 2+ short-term summaries
        // have the same conclusion fingerprint as the current one, this execution
        // carries no new information — skip writing to avoid redundant memory entries.
        // NOTE: image analyses bypass duplicate detection.
        let current_fp = conclusion_fingerprint(conclusion, success);
        let recent_duplicate_count = if has_image_analysis {
            0 // never skip image analyses as duplicates
        } else {
            memory
                .short_term
                .summaries
                .iter()
                .rev()
                .take(2)
                .filter(|s| conclusion_fingerprint(&s.conclusion, s.success) == current_fp)
                .count()
        };

        let is_routine_success = !has_alert_or_command
            && decisions.is_empty()
            && success
            && !has_anomaly
            && !has_image_analysis;
        let is_duplicate = !has_image_analysis && recent_duplicate_count >= 2;

        if !is_routine_success && !is_duplicate {
            // Prepare decision summaries for Short-Term Memory
            let mut decision_summaries: Vec<String> = decisions
                .iter()
                .filter(|d| !d.description.is_empty())
                .map(|d| clean_and_truncate_text(&d.description, 100))
                .collect();

            // Append image analysis summaries so they persist in short-term memory
            decision_summaries.extend(image_analyses);

            tracing::debug!(
                agent_id = %agent.id,
                execution_id = %execution_id,
                analysis_len = cleaned_analysis.len(),
                conclusion_len = cleaned_conclusion.len(),
                decisions_count = decision_summaries.len(),
                has_image_analysis,
                "About to add to short_term memory"
            );

            memory.add_to_short_term(
                execution_id.to_string(),
                cleaned_analysis,
                cleaned_conclusion,
                decision_summaries,
                success,
            );

            tracing::debug!(
                agent_id = %agent.id,
                execution_id = %execution_id,
                short_term_count = memory.short_term.summaries.len(),
                "Short-term memory updated"
            );

            // 3. Add patterns to Long-Term Memory (only for significant executions)
            if !decisions.is_empty() {
                let semantic_patterns = extract_semantic_patterns(
                    decisions,
                    situation_analysis,
                    data,
                    &memory.baselines,
                );

                for pattern in semantic_patterns {
                    memory.add_pattern(pattern);
                }
            }
        } else {
            tracing::debug!(
                agent_id = %agent.id,
                execution_id = %execution_id,
                is_routine = is_routine_success,
                is_duplicate = is_duplicate,
                recent_duplicate_count,
                "Skipping short-term/long-term memory: routine or duplicate"
            );
        }

        // === TREND AND BASELINE TRACKING ===
        let is_numeric_data =
            |data_type: &str| !matches!(data_type, "device_info" | "state" | "info");

        for data_item in data {
            if !is_numeric_data(&data_item.data_type) {
                continue;
            }

            if let Some(value) = data_item.values.get("value") {
                if let Some(num) = value.as_f64() {
                    // Add to trend data (limit to 200 points - enough for trends)
                    memory.trend_data.push(TrendPoint {
                        timestamp: data_item.timestamp,
                        metric: data_item.source.clone(),
                        value: num,
                        context: Some(serde_json::json!(data_item.data_type)),
                    });

                    if memory.trend_data.len() > 200 {
                        memory.trend_data =
                            memory.trend_data.split_off(memory.trend_data.len() - 200);
                    }

                    // Update baseline using exponential moving average
                    let baseline = memory
                        .baselines
                        .entry(data_item.source.clone())
                        .or_insert(num);
                    *baseline = *baseline * 0.9 + num * 0.1;
                }
            }
        }

        // === LEGACY STATE_VARIABLES (for backward compatibility) ===
        // Track execution count
        let execution_count = memory
            .state_variables
            .get("total_executions")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            + 1;
        memory.state_variables.insert(
            "total_executions".to_string(),
            serde_json::json!(execution_count),
        );

        // Store metrics we've seen
        for data_item in data {
            if is_numeric_data(&data_item.data_type) {
                let metrics_seen = memory
                    .state_variables
                    .entry("metrics_seen".to_string())
                    .or_insert(serde_json::json!([]));
                if let Some(arr) = metrics_seen.as_array_mut() {
                    let metric_ref = data_item.source.clone();
                    if !arr.iter().any(|v| v.as_str() == Some(&metric_ref)) {
                        arr.push(serde_json::json!(metric_ref));
                    }
                }
            }
        }

        memory.updated_at = chrono::Utc::now().timestamp();

        tracing::debug!(
            memory_usage = %memory.memory_usage_summary(),
            execution_id = %execution_id,
            success = success,
            "Agent memory updated (hierarchical)"
        );

        Ok(memory)
    }

    /// Create a conversation turn from execution results.
    pub fn create_conversation_turn(
        &self,
        execution_id: String,
        trigger_type: String,
        input_data: Vec<DataCollected>,
        event_data: Option<serde_json::Value>,
        decision_process: &DecisionProcess,
        duration_ms: u64,
        success: bool,
    ) -> ConversationTurn {
        // Clean and truncate before storing in conversation history
        // Conversation history can have up to 20 entries, so we need to be conservative
        let clean_situation = clean_and_truncate_text(&decision_process.situation_analysis, 300);
        let clean_conclusion = clean_and_truncate_text(&decision_process.conclusion, 150);

        // Also truncate reasoning step descriptions
        let cleaned_steps: Vec<neomind_storage::ReasoningStep> = decision_process
            .reasoning_steps
            .iter()
            .map(|step| neomind_storage::ReasoningStep {
                description: clean_and_truncate_text(&step.description, 100),
                ..step.clone()
            })
            .collect();

        // Truncate decision descriptions
        let cleaned_decisions: Vec<neomind_storage::Decision> = decision_process
            .decisions
            .iter()
            .map(|dec| neomind_storage::Decision {
                description: clean_and_truncate_text(&dec.description, 100),
                rationale: clean_and_truncate_text(&dec.rationale, 100),
                expected_outcome: clean_and_truncate_text(&dec.expected_outcome, 100),
                ..dec.clone()
            })
            .collect();

        ConversationTurn {
            execution_id,
            timestamp: chrono::Utc::now().timestamp(),
            trigger_type,
            input: TurnInput {
                data_collected: input_data,
                event_data,
            },
            output: TurnOutput {
                situation_analysis: clean_situation,
                reasoning_steps: cleaned_steps,
                decisions: cleaned_decisions,
                conclusion: clean_conclusion,
            },
            duration_ms,
            success,
        }
    }
}
