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

        // Extract pattern type — skip "info" type entirely (not meaningful enough)
        let pattern_type = match decision.decision_type.as_str() {
            "alert" => "anomaly_detection",
            "command" => "automated_control",
            _ => continue, // Skip "info" and other non-actionable types
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

    // Quality gate: filter out generic/routine patterns
    patterns.retain(is_pattern_worth_storing);

    patterns
}

/// Check if a pattern is worth storing — skip generic/routine entries.
fn is_pattern_worth_storing(pattern: &LearnedPattern) -> bool {
    let desc_lower = pattern.description.to_lowercase();
    let generic_phrases = [
        "status normal",
        "no action",
        "routine check",
        "information logging",
        "routine",
        "no anomaly",
        "within normal range",
    ];
    if generic_phrases.iter().any(|p| desc_lower.contains(p)) {
        return false;
    }
    // Skip patterns with very short descriptions (no real information)
    if pattern.description.len() < 15 {
        return false;
    }
    true
}

pub(crate) fn extract_symptom(situation_analysis: &str, decision: &Decision) -> String {
    // Try to extract actual numeric values from situation_analysis
    let numeric_re = regex::Regex::new(r"(\d+\.?\d*)\s*(?:°C|℃|度|%|℃|Pa|kPa|hPa|mmHg|V|A|W|mV)").ok();

    if !situation_analysis.is_empty() {
        // Try to extract numeric values for context-rich symptoms
        let numbers: Vec<&str> = numeric_re
            .as_ref()
            .map(|re| re.find_iter(situation_analysis).map(|m| m.as_str()).collect())
            .unwrap_or_default();

        if situation_analysis.contains("超过")
            || situation_analysis.contains("高于")
            || situation_analysis.contains("exceeds")
            || situation_analysis.contains("above")
        {
            if !numbers.is_empty() {
                return format!("Value {} exceeds threshold", numbers.join(", "));
            }
            return "Value exceeds threshold".to_string();
        }
        if situation_analysis.contains("低于") || situation_analysis.contains("below") {
            if !numbers.is_empty() {
                return format!("Value {} below threshold", numbers.join(", "));
            }
            return "Value below threshold".to_string();
        }
        if situation_analysis.contains("异常")
            || situation_analysis.contains("不正常")
            || situation_analysis.contains("abnormal")
        {
            if !numbers.is_empty() {
                return format!("Abnormal: {}", numbers.join(", "));
            }
            return "Abnormal state detected".to_string();
        }
        // For "normal/stable" — include numeric values if available
        if situation_analysis.contains("正常")
            || situation_analysis.contains("稳定")
            || situation_analysis.contains("normal")
            || situation_analysis.contains("stable")
        {
            if !numbers.is_empty() {
                return format!("{} within normal range", numbers.join(", "));
            }
            return "Status normal".to_string();
        }

        // If situation_analysis has content but no known keywords,
        // don't produce a symptom — let it fall through to decision type fallback
    }

    // Fallback to decision type
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
    let desc = &decision.description;

    // Pattern: "Temp sensor 1 shows 25 degrees" -> "Temp 25°C normal (baseline: 24.1°C)"
    if desc.contains("温度") || desc.contains("temp") {
        return format!("Temp {} - {}", symptom, decision.action);
    }
    if desc.contains("湿度") || desc.contains("humidity") {
        return format!("Humidity {} - {}", symptom, decision.action);
    }
    if desc.contains("压力") || desc.contains("pressure") {
        return format!("Pressure {} - {}", symptom, decision.action);
    }

    // Generic description — include symptom detail
    format!("{} - {}", symptom, decision.action)
}

/// Extract a concise insight from execution results using rule-based heuristics.
/// No LLM call — pure pattern matching on conclusion, decisions, and data.
fn extract_execution_insight(
    situation: &str,
    conclusion: &str,
    decisions: &[Decision],
    data: &[DataCollected],
    success: bool,
    baselines: &HashMap<String, f64>,
) -> Option<String> {
    // Rule 1: Failed execution → extract reason from conclusion
    if !success {
        let reason = truncate_to(conclusion, 120);
        return Some(format!("Execution failed: {}", reason));
    }

    // Rule 2: Alert or command decision → capture trigger and action
    if let Some(d) = decisions
        .iter()
        .find(|d| matches!(d.decision_type.as_str(), "alert" | "command"))
    {
        let desc = truncate_to(&d.description, 100);
        return Some(desc);
    }

    // Rule 3: Numeric value deviates from baseline by >20%
    for item in data {
        if let (Some(val), Some(&baseline)) = (
            item.values.get("value").and_then(|v| v.as_f64()),
            baselines.get(&item.source),
        ) {
            if baseline.abs() > 0.01 {
                let deviation = ((val - baseline) / baseline * 100.0).abs();
                if deviation > 20.0 {
                    return Some(format!(
                        "{} deviates {:.0}% from baseline: current {:.1} vs baseline {:.1}",
                        item.source, deviation, val, baseline
                    ));
                }
            }
        }
    }

    // Rule 4: Anomaly keywords detected in situation
    let lower = situation.to_lowercase();
    if lower.contains("异常") || lower.contains("abnormal") || lower.contains("anomaly") {
        return Some(truncate_to(conclusion, 120));
    }

    None // Routine execution, no special insight
}

/// Build a fingerprint for an image analysis entry.
/// Takes the first 80 chars of the entry — coarse enough to catch duplicate
/// observations of the same scene, but granular enough to distinguish changes.
fn image_analysis_fingerprint(entry: &str) -> String {
    entry.chars().take(80).collect()
}

/// Extract a concise text insight about an image from the LLM's analysis and conclusion.
///
/// Prioritizes content from the situation analysis that describes visual observations,
/// falling back to the conclusion text.  All truncation is done via
/// `clean_and_truncate_text` which operates on char boundaries — no manual byte
/// slicing.
fn extract_image_insight(situation_analysis: &str, conclusion: &str) -> String {
    let max_chars = 200;

    // Sentence-ending punctuation used to find a natural break point
    let is_sentence_end =
        |c: char| -> bool { matches!(c, '。' | '.' | '！' | '？' | '!' | '?') };

    // Try to find a visually descriptive segment from situation_analysis
    let visual_markers: &[&str] = &[
        "image shows",
        "the image",
        "visible",
        "detected",
        "camera",
        "observed",
        "图像",
        "图片",
        "画面",
        "可以看到",
        "观察到",
        "检测到",
    ];

    for marker in visual_markers {
        if let Some(pos) = situation_analysis.find(marker) {
            // Take up to max_chars characters (not bytes) after the marker position
            let segment: String = situation_analysis[pos..].chars().take(max_chars).collect();

            // Try to cut at the last sentence boundary
            let cut = segment
                .char_indices()
                .rev()
                .find(|(_, c)| is_sentence_end(*c))
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(segment.len());

            let candidate = &segment[..cut.min(segment.len())];
            if !candidate.is_empty() {
                return clean_and_truncate_text(candidate, max_chars);
            }
        }
    }

    // Fallback: use conclusion
    if !conclusion.is_empty() {
        return clean_and_truncate_text(conclusion, max_chars);
    }

    // Last resort: first part of situation_analysis
    clean_and_truncate_text(situation_analysis, max_chars)
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
        // Deduplicate by (source, insight fingerprint) to avoid filling short-term
        // memory with identical observations from an unchanged camera scene.
        let image_analyses: Vec<String> = if has_image_analysis {
            // Collect fingerprints of existing image analyses in recent memory
            let existing_fps: HashSet<String> = memory
                .short_term
                .summaries
                .iter()
                .rev()
                .take(3)
                .flat_map(|s| s.decisions.iter())
                .filter(|d| d.starts_with("[image_analysis]"))
                .map(|d| image_analysis_fingerprint(d))
                .collect();

            data.iter()
                .filter(|d| {
                    d.values
                        .get("_is_image")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                })
                .filter_map(|d| {
                    let insight = extract_image_insight(situation_analysis, conclusion);
                    let entry = format!("[image_analysis] {}: {}", d.source, insight);
                    let fp = image_analysis_fingerprint(&entry);
                    if existing_fps.contains(&fp) {
                        tracing::debug!(
                            source = %d.source,
                            "Skipping duplicate image analysis"
                        );
                        None
                    } else {
                        Some(entry)
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        // If all image analyses were deduplicated, treat as no image analysis
        let has_new_image_analysis = !image_analyses.is_empty();

        // Fingerprint-based duplicate detection: if the last 2+ short-term summaries
        // have the same conclusion fingerprint as the current one, this execution
        // carries no new information — skip writing to avoid redundant memory entries.
        // NOTE: new image analyses bypass duplicate detection.
        let current_fp = conclusion_fingerprint(conclusion, success);
        let recent_duplicate_count = if has_new_image_analysis {
            0 // never skip new image analyses as duplicates
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
            && !has_new_image_analysis;

        // Also skip non-routine executions whose conclusion is purely generic
        // (e.g., "所有设备正常" with an info-level decision)
        let conclusion_lower = conclusion.to_lowercase();
        let is_generic_conclusion = conclusion_lower.contains("正常")
            || conclusion_lower.contains("normal")
            || conclusion_lower.contains("稳定")
            || conclusion_lower.contains("stable")
            || conclusion_lower.contains("无异常")
            || conclusion_lower.contains("no anomaly")
            || conclusion_lower.contains("无需操作")
            || conclusion_lower.contains("no action");

        let is_duplicate = !has_new_image_analysis && recent_duplicate_count >= 2;

        // Skip writing if: routine success, duplicate, or generic conclusion with no real action
        let should_skip = is_routine_success
            || is_duplicate
            || (is_generic_conclusion && !has_alert_or_command && !has_anomaly && !has_new_image_analysis);

        if !should_skip {
            // Prepare decision summaries for Short-Term Memory
            let mut decision_summaries: Vec<String> = decisions
                .iter()
                .filter(|d| !d.description.is_empty())
                .map(|d| clean_and_truncate_text(&d.description, 100))
                .collect();

            // Append image analysis summaries so they persist in short-term memory
            decision_summaries.extend(image_analyses);

            // Extract rule-based insight from this execution
            let insight = extract_execution_insight(
                situation_analysis,
                conclusion,
                decisions,
                data,
                success,
                &memory.baselines,
            );

            tracing::debug!(
                agent_id = %agent.id,
                execution_id = %execution_id,
                analysis_len = cleaned_analysis.len(),
                conclusion_len = cleaned_conclusion.len(),
                decisions_count = decision_summaries.len(),
                has_image_analysis,
                has_insight = insight.is_some(),
                "About to add to short_term memory"
            );

            memory.add_to_short_term(
                execution_id.to_string(),
                cleaned_analysis,
                cleaned_conclusion,
                decision_summaries,
                success,
                insight,
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
                is_generic = is_generic_conclusion,
                recent_duplicate_count,
                "Skipping short-term/long-term memory: routine, duplicate, or generic"
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

/// Use LLM to reflect on accumulated insights and produce/update a TaskProfile.
/// Returns None if insufficient insights or if the LLM call fails.
pub(crate) async fn reflect_task_profile(
    agent_name: &str,
    user_prompt: &str,
    summaries: &[MemorySummary],
    existing_profile: Option<&TaskProfile>,
    llm: &Arc<dyn LlmRuntime + Send + Sync>,
) -> Option<TaskProfile> {
    // Collect recent summaries that have insights
    let insights: Vec<&MemorySummary> = summaries
        .iter()
        .rev()
        .filter(|s| s.insight.is_some())
        .take(10)
        .collect();
    if insights.len() < 3 {
        return None;
    }

    let insight_text = insights
        .iter()
        .map(|s| {
            let ts = chrono::DateTime::from_timestamp(s.timestamp, 0)
                .map(|dt| dt.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "??".to_string());
            format!("- [{}] {}", ts, s.insight.as_ref().unwrap())
        })
        .collect::<Vec<_>>()
        .join("\n");

    let existing_section = existing_profile
        .map(|p| {
            let ts = chrono::DateTime::from_timestamp(p.updated_at, 0)
                .map(|dt| dt.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "??".to_string());
            format!(
                "\n\nCurrent profile (v{}, {}):\n{}",
                p.version, ts, p.summary
            )
        })
        .unwrap_or_default();

    let prompt = format!(
        "You are the task reflection module for AI Agent \"{}\".\n\
         Agent's task: {}\n\n\
         Recent execution findings:\n{}\n{}\n\n\
         Synthesize these findings into an updated task knowledge summary.\n\
         Requirements:\n\
         - Each point starts with \"- \" (3-5 key points)\n\
         - Include specific numbers: value ranges, thresholds, time patterns\n\
         - Discard outdated or corrected conclusions\n\
         - Max 500 characters total\n\
         - Output ONLY the summary, nothing else",
        agent_name, user_prompt, insight_text, existing_section
    );

    let input = LlmInput::new(prompt).with_params(GenerationParams {
        temperature: Some(0.3),
        max_tokens: Some(512),
        thinking_enabled: Some(false),
        ..Default::default()
    });

    match llm.generate(input).await {
        Ok(output) => {
            let summary = output.text.trim().to_string();
            if summary.is_empty() {
                return None;
            }
            // Truncate to 500 chars instead of discarding — LLMs often ignore length limits
            let summary: String = summary.chars().take(500).collect();
            Some(TaskProfile {
                summary,
                updated_at: chrono::Utc::now().timestamp(),
                executions_reflected: existing_profile
                    .map(|p| p.executions_reflected)
                    .unwrap_or(0)
                    + insights.len() as u32,
                version: existing_profile.map(|p| p.version + 1).unwrap_or(1),
            })
        }
        Err(_) => None,
    }
}

/// Check if a reflection cycle should be triggered based on memory state.
pub(crate) fn should_trigger_reflection(memory: &AgentMemory, has_insight: bool) -> bool {
    if !has_insight {
        return false;
    }
    match &memory.task_profile {
        None => memory
            .short_term
            .summaries
            .iter()
            .filter(|s| s.insight.is_some())
            .count()
            >= 5,
        Some(p) => (chrono::Utc::now().timestamp() - p.updated_at) > 6 * 3600,
    }
}
