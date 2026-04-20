use super::*;
use crate::prompts::{CONVERSATION_CONTEXT_EN, CONVERSATION_CONTEXT_ZH};

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
        if situation_analysis.contains("超过") || situation_analysis.contains("高于") || situation_analysis.contains("exceeds") || situation_analysis.contains("above") {
            return "Value exceeds threshold".to_string();
        }
        if situation_analysis.contains("低于") || situation_analysis.contains("below") {
            return "Value below threshold".to_string();
        }
        if situation_analysis.contains("异常") || situation_analysis.contains("不正常") || situation_analysis.contains("abnormal") {
            return "Abnormal state detected".to_string();
        }
        if situation_analysis.contains("正常") || situation_analysis.contains("稳定") || situation_analysis.contains("normal") || situation_analysis.contains("stable") {
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

        // 2. Write gating: skip short-term/long-term for routine successful executions
        let has_alert_or_command = decisions
            .iter()
            .any(|d| matches!(d.decision_type.as_str(), "alert" | "command"));
        let has_anomaly = situation_analysis.to_lowercase().contains("异常")
            || situation_analysis.to_lowercase().contains("abnormal")
            || situation_analysis.to_lowercase().contains("anomaly");
        let is_routine_success = !has_alert_or_command && decisions.is_empty() && success && !has_anomaly;

        if !is_routine_success {
            // Prepare decision summaries for Short-Term Memory
            let decision_summaries: Vec<String> = decisions
                .iter()
                .filter(|d| !d.description.is_empty())
                .map(|d| clean_and_truncate_text(&d.description, 100))
                .collect();

            tracing::debug!(
                agent_id = %agent.id,
                execution_id = %execution_id,
                analysis_len = cleaned_analysis.len(),
                conclusion_len = cleaned_conclusion.len(),
                decisions_count = decision_summaries.len(),
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
                let semantic_patterns =
                    extract_semantic_patterns(decisions, situation_analysis, data, &memory.baselines);

                for pattern in semantic_patterns {
                    memory.add_pattern(pattern);
                }
            }
        } else {
            tracing::debug!(
                agent_id = %agent.id,
                execution_id = %execution_id,
                "Skipping short-term/long-term memory: routine success"
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


    pub fn build_conversation_messages(
        &self,
        agent: &AiAgent,
        current_data: &[DataCollected],
        event_data: Option<serde_json::Value>,
    ) -> Vec<Message> {
        let mut messages = Vec::new();

        // Detect language from user_prompt to determine response language
        let detected_language = SemanticToolMapper::detect_language(&agent.user_prompt);
        let is_chinese = matches!(
            detected_language,
            crate::agent::semantic_mapper::Language::Chinese
                | crate::agent::semantic_mapper::Language::Mixed
        );

        // 1. Generic system prompt with conversation context
        let (role_prompt, conversation_context, task_header) = if is_chinese {
            (
                "你是一个 NeoMind 智能物联网系统的自动化助手。根据用户的指令分析数据、做出决策并执行相应操作。",
                CONVERSATION_CONTEXT_ZH,
                "## 你的任务"
            )
        } else {
            (
                "You are an automation assistant for the NeoMind IoT system. Analyze data, make decisions, and execute operations based on user instructions.",
                CONVERSATION_CONTEXT_EN,
                "## Your Task"
            )
        };

        // Get current time context for temporal understanding
        let time_context = get_time_context();

        let system_prompt = format!(
            "{}\n\n{}\n\n{}\n\n{}\n{}\n\n{}",
            LANGUAGE_POLICY, role_prompt, time_context, task_header, agent.user_prompt, conversation_context
        );
        messages.push(Message::system(system_prompt));

        // 2. Add user messages as important context - these are the user's latest instructions
        // User messages take priority over initial configuration and historical patterns
        if !agent.user_messages.is_empty() {
            let user_msgs_text: Vec<String> = agent
                .user_messages
                .iter()
                .enumerate()
                .map(|(i, msg)| {
                    let timestamp_str = chrono::DateTime::from_timestamp(msg.timestamp, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    format!("{}. [{}] {}", i + 1, timestamp_str, msg.content)
                })
                .collect();

            let formatted_msg = if is_chinese {
                format!(
                    "## ⚠️ 用户最新指令 (必须严格遵循)\n\n\
                    用户在运行期间发送了以下消息，这些消息包含对执行策略的更新。\
                    **请务必将这些指令作为最高优先级，覆盖初始配置中的任何冲突规则：**\n\n\
                    {}\n\n\
                    请在分析当前情况时，严格按照上述用户指令进行决策。",
                    user_msgs_text.join("\n")
                )
            } else {
                format!(
                    "## ⚠️ Latest User Instructions (Must Follow Strictly)\n\n\
                    The user sent the following messages during runtime, containing updates to execution strategy.\
                    **These instructions must be treated as highest priority, overriding any conflicting rules in the initial configuration:**\n\n\
                    {}\n\n\
                    When analyzing the current situation, strictly follow the above user instructions for decision-making.",
                    user_msgs_text.join("\n")
                )
            };
            messages.push(Message::system(formatted_msg));
        }

        // 3. Add conversation summary if available
        if let Some(ref summary) = agent.conversation_summary {
            let summary_header = if is_chinese {
                "## 历史对话摘要"
            } else {
                "## Conversation Summary"
            };
            messages.push(Message::system(format!(
                "{}\n\n{}",
                summary_header, summary
            )));
        }

        // 4. Add recent conversation turns as context with intelligent filtering
        // Use relevance scoring to select the most valuable conversation turns
        let context_window = agent.context_window_size;
        let current_trigger = if event_data.is_some() {
            "event"
        } else {
            "scheduled"
        };

        // Score all turns by relevance and select top N
        let mut scored_turns: Vec<_> = agent
            .conversation_history
            .iter()
            .map(|turn| {
                let score = score_turn_relevance(turn, current_data, current_trigger);
                (turn, score)
            })
            .collect();

        // Filter out turns with very low relevance (< 0.15) to save context space
        scored_turns.retain(|(_, score)| *score >= 0.15);

        // Sort by relevance score (descending) and take top N
        scored_turns.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let recent_turns: Vec<_> = scored_turns.into_iter().take(context_window).collect();

        if !recent_turns.is_empty() {
            let history_header = if is_chinese {
                format!(
                    "## 之前的执行历史 (最近 {} 次)\n\n请参考以下历史记录，避免重复告警，追踪趋势变化。",
                    recent_turns.len()
                )
            } else {
                format!(
                    "## Previous Execution History (Last {})\n\nRefer to the following history to avoid duplicate alerts and track trend changes.",
                    recent_turns.len()
                )
            };
            messages.push(Message::system(history_header));

            // Add each turn as context (in reverse order since we sorted by relevance desc)
            // recent_turns is Vec<(&ConversationTurn, f64)>
            for (i, (turn, _score)) in recent_turns.iter().rev().enumerate() {
                let timestamp_str = chrono::DateTime::from_timestamp(turn.timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let turn_context = if is_chinese {
                    format!(
                        "### 历史执行 #{} ({})\n触发方式: {}\n分析: {}\n结论: {}",
                        i + 1,
                        timestamp_str,
                        turn.trigger_type,
                        turn.output.situation_analysis,
                        turn.output.conclusion
                    )
                } else {
                    format!(
                        "### Historical Execution #{} ({})\nTrigger: {}\nAnalysis: {}\nConclusion: {}",
                        i + 1,
                        timestamp_str,
                        turn.trigger_type,
                        turn.output.situation_analysis,
                        turn.output.conclusion
                    )
                };

                messages.push(Message::system(turn_context));

                // Add decisions if any
                if !turn.output.decisions.is_empty() {
                    let decisions_summary: Vec<String> = turn
                        .output
                        .decisions
                        .iter()
                        .map(|d| format!("- {}", d.description))
                        .collect();
                    let decisions_label = if is_chinese {
                        "历史决策"
                    } else {
                        "Historical Decisions"
                    };
                    messages.push(Message::system(format!(
                        "{}:\n{}",
                        decisions_label,
                        decisions_summary.join("\n")
                    )));
                }
            }

            let current_execution_note = if is_chinese {
                "## 当前执行\n\n请参考上述历史，分析当前情况。特别注意：\n\
                - 与之前数据相比的变化趋势\n\
                - 之前报告的问题是否持续\n\
                - 避免重复相同的分析或决策"
            } else {
                "## Current Execution\n\nRefer to the history above when analyzing the current situation. Pay attention to:\n\
                - Trend changes compared to previous data\n\
                - Whether previously reported issues persist\n\
                - Avoid repeating the same analysis or decisions"
            };
            messages.push(Message::system(current_execution_note.to_string()));
        }

        // 5. Current execution data
        let data_text = if current_data.is_empty() {
            if is_chinese {
                "当前无预采集数据，请使用可用工具查询需要的设备数据".to_string()
            } else {
                "No pre-collected data available. Use available tools to query the data you need"
                    .to_string()
            }
        } else {
            current_data
                .iter()
                .map(|d| format!("- {}: {}", d.source, d.data_type))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let (
            current_data_header,
            data_sources_label,
            trigger_label,
            trigger_type_text,
            analysis_request,
        ) = if is_chinese {
            (
                "## 当前数据",
                "数据来源",
                "触发方式",
                if event_data.is_some() {
                    "事件触发"
                } else {
                    "定时/手动"
                },
                "请分析当前情况并做出决策。",
            )
        } else {
            (
                "## Current Data",
                "Data Sources",
                "Trigger",
                if event_data.is_some() {
                    "Event-triggered"
                } else {
                    "Scheduled/Manual"
                },
                "Please analyze the current situation and make decisions.",
            )
        };

        let current_input = format!(
            "{}\n\n{}:\n{}\n\n{}: {}\n\n{}",
            current_data_header,
            data_sources_label,
            data_text,
            trigger_label,
            trigger_type_text,
            analysis_request
        );

        messages.push(Message::user(current_input));

        messages
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

    /// Bridge agent execution results into system memory
    ///
    /// When an agent discovers useful patterns (high-confidence decisions, device states,
    /// threshold learnings), extract them into the shared system memory so other agents
    /// and chat sessions can benefit from the knowledge.
    pub(crate) async fn extract_to_system_memory(
        &self,
        agent: &AiAgent,
        situation_analysis: &str,
        conclusion: &str,
        decisions: &[Decision],
    ) {
        use crate::memory_extraction::MemoryExtractor;

        let Some(memory_store) = &self.memory_store else {
            return;
        };

        // Get LLM runtime for extraction (use the agent's configured backend or default)
        let llm: Arc<dyn neomind_core::llm::backend::LlmRuntime> = match &self.llm_runtime {
            Some(runtime) => runtime.clone(),
            None => {
                tracing::debug!(
                    agent_id = %agent.id,
                    "No LLM runtime available for system memory extraction"
                );
                return;
            }
        };

        // Only extract if there are meaningful decisions
        let has_high_importance = decisions.iter().any(|d| {
            matches!(d.decision_type.as_str(), "alert" | "command")
        });

        if !has_high_importance && situation_analysis.len() < 50 {
            tracing::debug!(
                agent_id = %agent.id,
                "Skipping system memory extraction: no high-importance decisions"
            );
            return;
        }

        // Build reasoning summary from decisions
        let reasoning_steps: String = decisions
            .iter()
            .map(|d| format!("- [{}] {} (action: {})", d.decision_type, d.description, d.action))
            .collect::<Vec<_>>()
            .join("\n");

        // Wrap store in RwLock as required by MemoryExtractor
        let store: Arc<tokio::sync::RwLock<MarkdownMemoryStore>> =
            Arc::new(tokio::sync::RwLock::new(MarkdownMemoryStore::clone(memory_store)));
        let extractor = MemoryExtractor::new(store, llm);

        match extractor.extract_from_agent(
            &agent.name,
            Some(&agent.user_prompt),
            &reasoning_steps,
            conclusion,
        ).await {
            Ok(count) => {
                if count > 0 {
                    tracing::info!(
                        agent_id = %agent.id,
                        agent_name = %agent.name,
                        memories_extracted = count,
                        "Bridged agent learnings to system memory"
                    );
                }
            }
            Err(e) => {
                tracing::debug!(
                    agent_id = %agent.id,
                    error = %e,
                    "Failed to extract agent results to system memory"
                );
            }
        }
    }
}
