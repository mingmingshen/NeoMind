use super::*;
use base64::Engine;

/// Result of situation analysis, branching by execution path.
///
/// - `Free`: Tool-calling mode — the LLM autonomously called tools and
///   produced a full `DecisionProcess` + `ExecutionResult`.  The caller should
///   return these directly, skipping Focused post-processing (execute_decisions,
///   report generation, truncation).
/// - `Focused`: Standard single-pass LLM or rule-based analysis — returns the four
///   classical fields that `execute_internal` assembles into `DecisionProcess`.
pub(crate) enum AnalysisResult {
    Focused {
        situation_analysis: String,
        reasoning_steps: Vec<ReasoningStep>,
        decisions: Vec<Decision>,
        conclusion: String,
    },
    Free {
        decision_process: DecisionProcess,
        execution_result: neomind_storage::ExecutionResult,
    },
}

impl AgentExecutor {
    pub(crate) fn build_available_commands_description(agent: &AiAgent) -> String {
        let mut device_commands: std::collections::HashMap<String, Vec<&AgentResource>> =
            std::collections::HashMap::new();
        let mut extension_commands: std::collections::HashMap<String, Vec<&AgentResource>> =
            std::collections::HashMap::new();

        // Group commands by device or extension
        for resource in &agent.resources {
            match resource.resource_type {
                ResourceType::Command => {
                    // Parse device_id from resource_id (format: "device_id:command_name")
                    let parts: Vec<&str> = resource.resource_id.split(':').collect();
                    let device_id = if !parts.is_empty() {
                        parts[0]
                    } else {
                        "unknown"
                    };

                    device_commands
                        .entry(device_id.to_string())
                        .or_default()
                        .push(resource);
                }
                ResourceType::ExtensionTool => {
                    // Parse extension_id from resource_id (format: "extension:extension_id:command_name")
                    let parts: Vec<&str> = resource.resource_id.split(':').collect();
                    let ext_id = if parts.len() >= 2 {
                        parts[1]
                    } else {
                        "unknown"
                    };

                    extension_commands
                        .entry(ext_id.to_string())
                        .or_default()
                        .push(resource);
                }
                _ => {}
            }
        }

        if device_commands.is_empty() && extension_commands.is_empty() {
            return "无可用命令".to_string();
        }

        let mut descriptions = Vec::new();

        // Add device commands
        if !device_commands.is_empty() {
            descriptions.push("## 可用设备命令\n".to_string());

            for (device_id, commands) in &device_commands {
                descriptions.push(format!("### 设备: {}", device_id));

                for cmd in commands {
                    // Extract command name from resource_id
                    let parts: Vec<&str> = cmd.resource_id.split(':').collect();
                    let command_name = if parts.len() >= 2 {
                        parts[1]
                    } else {
                        &cmd.resource_id
                    };

                    // Get display name or use command name
                    let display_name = if !cmd.name.is_empty() {
                        &cmd.name
                    } else {
                        command_name
                    };

                    // Format: "device_id:command_name" - display_name
                    descriptions.push(format!(
                        "- `{}:{}` - {}",
                        device_id, command_name, display_name
                    ));

                    // Add parameters info if available
                    if let Some(params) = cmd.config.get("parameters").and_then(|v| v.as_array()) {
                        let param_names: Vec<_> = params
                            .iter()
                            .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
                            .collect();
                        if !param_names.is_empty() {
                            descriptions.push(format!("  参数: {}", param_names.join(", ")));
                        }
                    }
                }

                descriptions.push(String::new()); // Empty line between devices
            }
        }

        // Add extension commands
        if !extension_commands.is_empty() {
            descriptions.push("## 可用扩展工具\n".to_string());

            for (ext_id, commands) in &extension_commands {
                descriptions.push(format!("### 扩展: {}", ext_id));

                for cmd in commands {
                    // Extract command name from resource_id (format: "extension:ext_id:command_name")
                    let parts: Vec<&str> = cmd.resource_id.split(':').collect();
                    let command_name = if parts.len() >= 3 {
                        parts[2]
                    } else {
                        &cmd.resource_id
                    };

                    // Get display name or use command name
                    let display_name = if !cmd.name.is_empty() {
                        &cmd.name
                    } else {
                        command_name
                    };

                    // Format: "extension:ext_id:command_name" - display_name
                    descriptions.push(format!(
                        "- `extension:{}:{}` - {}",
                        ext_id, command_name, display_name
                    ));

                    // Add parameters info if available
                    if let Some(params) = cmd.config.get("parameters").and_then(|v| v.as_array()) {
                        let param_names: Vec<_> = params
                            .iter()
                            .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
                            .collect();
                        if !param_names.is_empty() {
                            descriptions.push(format!("  参数: {}", param_names.join(", ")));
                        }
                    }
                }

                descriptions.push(String::new()); // Empty line between extensions
            }
        }

        // Add usage instructions
        descriptions.push(
            "### 命令执行说明\n\
             在 decisions 中，如需执行命令，请使用以下格式：\n\
             - 设备命令: action: \"device_id:command_name\" (例如: \"light1:turn_on\")\n\
             - 扩展工具: action: \"extension:ext_id:command_name\" (例如: \"extension:weather:get_forecast\")\n\
             - decision_type: \"command\"\n\
             - description: 命令描述\n\
             - rationale: 执行原因".to_string()
        );

        descriptions.join("\n")
    }

    /// Build available data sources description for LLM.
    pub(crate) fn build_available_data_sources_description(agent: &AiAgent) -> String {
        let mut device_metrics: std::collections::HashMap<String, Vec<&AgentResource>> =
            std::collections::HashMap::new();
        let mut extension_metrics: std::collections::HashMap<String, Vec<&AgentResource>> =
            std::collections::HashMap::new();
        let mut device_resources: Vec<&AgentResource> = Vec::new();

        // Group data sources by type
        for resource in &agent.resources {
            match resource.resource_type {
                ResourceType::Metric => {
                    // Parse device_id from resource_id (format: "device_id:metric_name")
                    let parts: Vec<&str> = resource.resource_id.split(':').collect();
                    let device_id = if !parts.is_empty() {
                        parts[0]
                    } else {
                        "unknown"
                    };

                    device_metrics
                        .entry(device_id.to_string())
                        .or_default()
                        .push(resource);
                }
                ResourceType::ExtensionMetric => {
                    // Parse extension_id from resource_id (format: "extension:extension_id:metric")
                    let parts: Vec<&str> = resource.resource_id.split(':').collect();
                    let ext_id = if parts.len() >= 2 {
                        parts[1]
                    } else {
                        "unknown"
                    };

                    extension_metrics
                        .entry(ext_id.to_string())
                        .or_default()
                        .push(resource);
                }
                ResourceType::Device => {
                    device_resources.push(resource);
                }
                _ => {}
            }
        }

        if device_metrics.is_empty() && extension_metrics.is_empty() && device_resources.is_empty()
        {
            return String::new();
        }

        let mut descriptions = Vec::new();

        // Add device metrics
        if !device_metrics.is_empty() {
            descriptions.push("## 可用设备数据源\n".to_string());

            for (device_id, metrics) in &device_metrics {
                descriptions.push(format!("### 设备: {}", device_id));

                for metric in metrics {
                    // Extract metric name from resource_id
                    let parts: Vec<&str> = metric.resource_id.split(':').collect();
                    let metric_name = if parts.len() >= 2 {
                        parts[1]
                    } else {
                        &metric.resource_id
                    };

                    // Get display name or use metric name
                    let display_name = if !metric.name.is_empty() {
                        &metric.name
                    } else {
                        metric_name
                    };

                    // Get data type and unit from config
                    let data_type = metric
                        .config
                        .get("data_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("number");
                    let unit = metric.config.get("unit").and_then(|v| v.as_str());

                    let unit_str = if let Some(u) = unit {
                        format!(" ({})", u)
                    } else {
                        String::new()
                    };

                    descriptions.push(format!(
                        "- `{}:{}` - {}{} [{}]",
                        device_id, metric_name, display_name, unit_str, data_type
                    ));
                }

                descriptions.push(String::new()); // Empty line between devices
            }
        }

        // Add device resources (full device data)
        if !device_resources.is_empty() {
            descriptions.push("## 可用设备\n".to_string());

            for resource in device_resources {
                let display_name = if !resource.name.is_empty() {
                    &resource.name
                } else {
                    &resource.resource_id
                };
                descriptions.push(format!("- `{}` - {}", resource.resource_id, display_name));
            }

            descriptions.push(String::new());
        }

        // Add extension metrics
        if !extension_metrics.is_empty() {
            descriptions.push("## 可用扩展数据源\n".to_string());

            for (ext_id, metrics) in &extension_metrics {
                descriptions.push(format!("### 扩展: {}", ext_id));

                for metric in metrics {
                    // Extract metric name from resource_id (format: "extension:ext_id:metric" or "extension:ext_id:command:field")
                    let parts: Vec<&str> = metric.resource_id.split(':').collect();
                    let metric_path = if parts.len() >= 3 {
                        parts[2..].join(":")
                    } else if parts.len() >= 3 {
                        parts[2].to_string()
                    } else {
                        metric.resource_id.clone()
                    };

                    let display_name = if !metric.name.is_empty() {
                        &metric.name
                    } else {
                        &metric_path
                    };

                    descriptions.push(format!("- `{}:{}` - {}", ext_id, metric_path, display_name));
                }

                descriptions.push(String::new()); // Empty line between extensions
            }
        }

        // Add usage instructions
        if !descriptions.is_empty() {
            descriptions.push(
                "### 数据查询说明\n\
                 - 当前数据值会在下方「当前数据」部分显示\n\
                 - 如果显示「No data available」，表示数据源暂时没有最新数据\n\
                 - 如需查询特定时间范围的数据，在 decision 的 action 中使用格式：\n\
                   * `query:device_id:metric:1h` - 查询最近1小时\n\
                   * `query:device_id:metric:24h` - 查询最近24小时\n\
                   * `query:device_id:metric:7d` - 查询最近7天\n\
                   * `query:device_id:metric:yesterday` - 查询昨天\n\
                   * `query:device_id:metric:last_week` - 查询上周\n\
                 - 支持的时间单位: m(分钟), h(小时), d(天), w(周)\n\
                 - 示例: `query:sensor1:temperature:24h` 查询传感器最近24小时温度"
                    .to_string(),
            );
        }

        descriptions.join("\n")
    }

    /// Build structured data table for Focused Mode prompt.
    /// Returns markdown tables of current data and available commands.
    pub(crate) fn build_focused_data_table(agent: &AiAgent, data: &[DataCollected]) -> String {
        let mut sections = Vec::new();

        // --- Current Data Table ---
        let data_entries: Vec<&DataCollected> = data
            .iter()
            .filter(|d| {
                d.source != "system"
                    && !d
                        .values
                        .get("_is_image")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
            })
            .take(15)
            .collect();

        if !data_entries.is_empty() {
            sections.push("## Current Data (live from bound resources)".to_string());
            sections.push("| Resource | Type | Value |".to_string());
            sections.push("|----------|------|-------|".to_string());
            for d in &data_entries {
                let value = if let Some(v) = d.values.get("value") {
                    format!("{}", v)
                } else {
                    let json_str = serde_json::to_string(&d.values).unwrap_or_default();
                    if json_str.len() > 100 {
                        json_str[..100].to_string() + "..."
                    } else {
                        json_str
                    }
                };
                sections.push(format!("| {} | {} | {} |", d.source, d.data_type, value));
            }
        }

        // --- Available Commands Table ---
        let commands: Vec<&AgentResource> = agent
            .resources
            .iter()
            .filter(|r| {
                matches!(
                    r.resource_type,
                    ResourceType::Command | ResourceType::ExtensionTool
                )
            })
            .collect();

        if !commands.is_empty() {
            sections.push(String::new());
            sections.push("## Available Commands (only execute when needed)".to_string());
            sections.push("| Name | Action Value |".to_string());
            sections.push("|------|-------------|".to_string());
            for cmd in &commands {
                let display_name = if cmd.name.is_empty() {
                    &cmd.resource_id
                } else {
                    &cmd.name
                };
                sections.push(format!("| {} | `{}` |", display_name, cmd.resource_id));
            }
        }

        // --- Decision Template ---
        sections.push(String::new());
        sections.push("## Decision Format".to_string());
        if !commands.is_empty() {
            sections.push("Execute a command:".to_string());
            sections.push("`{\"decision_type\": \"command\", \"action\": \"<copy Action Value>\", \"description\": \"<reason>\"}`".to_string());
        }
        sections.push("Send a notification:".to_string());
        sections.push("`{\"decision_type\": \"alert\", \"description\": \"<message>\"}` or `\"warning\"` / `\"critical\"`".to_string());
        sections.push("No action needed:".to_string());
        sections.push("`\"decisions\": []`".to_string());

        sections.join("\n")
    }

    pub(crate) async fn analyze_situation_with_intent(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
        parsed_intent: Option<&neomind_storage::ParsedIntent>,
        execution_id: &str,
        invocation_input: Option<&super::super::AgentInput>,
    ) -> AgentResult<AnalysisResult> {
        tracing::info!(
            agent_id = %agent.id,
            agent_name = %agent.name,
            data_count = data.len(),
            execution_id = %execution_id,
            "[ANALYZE] Starting situation analysis"
        );

        match self.get_llm_runtime_for_agent(agent).await {
            Ok(Some(llm)) => {
                tracing::info!(
                    agent_id = %agent.id,
                    "LLM runtime available, performing LLM-based analysis"
                );

                // Check if tool/function-calling mode should be used
                if self.should_use_tools(agent, &llm) {
                    tracing::info!(
                        agent_id = %agent.id,
                        "Tool mode enabled - using function calling"
                    );
                    match self
                        .execute_with_tools(
                            agent,
                            data,
                            llm.clone(),
                            execution_id,
                            invocation_input,
                        )
                        .await
                    {
                        Ok((dp, exec_result)) => {
                            tracing::info!(
                                agent_id = %agent.id,
                                "Tool-based analysis completed successfully"
                            );
                            // Free path: return full (DP, ER) — caller uses them directly
                            return Ok(AnalysisResult::Free {
                                decision_process: dp,
                                execution_result: exec_result,
                            });
                        }
                        Err(e) => {
                            tracing::warn!(
                                agent_id = %agent.id,
                                error = %e,
                                "Tool-based analysis failed, falling back to LLM analysis"
                            );
                        }
                    }
                }

                // Standard LLM-based analysis (Focused path)
                match self
                    .analyze_with_llm(llm, agent, data, parsed_intent, execution_id)
                    .await
                {
                    Ok((situation_analysis, reasoning_steps, decisions, conclusion)) => {
                        tracing::info!(
                            agent_id = %agent.id,
                            "LLM-based analysis completed successfully"
                        );
                        return Ok(AnalysisResult::Focused {
                            situation_analysis,
                            reasoning_steps,
                            decisions,
                            conclusion,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            agent_id = %agent.id,
                            error = %e,
                            "LLM analysis failed, falling back to rule-based"
                        );
                    }
                }
            }
            Ok(None) => {
                tracing::warn!(
                    agent_id = %agent.id,
                    "No LLM runtime configured, falling back to rule-based analysis"
                );
            }
            Err(e) => {
                tracing::warn!(
                    agent_id = %agent.id,
                    error = %e,
                    "Failed to get LLM runtime, falling back to rule-based"
                );
            }
        }

        // Fall back to rule-based logic (Focused path)
        let (situation_analysis, reasoning_steps, decisions, conclusion) =
            self.analyze_rule_based(agent, data, parsed_intent).await?;
        Ok(AnalysisResult::Focused {
            situation_analysis,
            reasoning_steps,
            decisions,
            conclusion,
        })
    }

    pub(crate) async fn analyze_with_llm(
        &self,
        llm: Arc<dyn neomind_core::llm::backend::LlmRuntime + Send + Sync>,
        agent: &AiAgent,
        data: &[DataCollected],
        parsed_intent: Option<&neomind_storage::ParsedIntent>,
        execution_id: &str,
    ) -> AgentResult<(String, Vec<ReasoningStep>, Vec<Decision>, String)> {
        use neomind_core::llm::backend::{GenerationParams, LlmInput};

        let current_time = chrono::Utc::now();
        let time_str = current_time.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let _timestamp = current_time.timestamp();

        tracing::info!(
            agent_id = %agent.id,
            data_count = data.len(),
            execution_id,
            current_time = %time_str,
            "Calling LLM for situation analysis..."
        );

        // Check if any data contains images
        let _has_images = data.iter().any(|d| {
            d.values
                .get("_is_image")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        });

        // Collect image parts directly from data_collected
        // Images are already collected in data_collected, no need to re-query storage
        let mut image_parts = Vec::new();
        let mut image_sources_info = Vec::new(); // Track image sources for text summary

        for d in data.iter() {
            let is_image = d
                .values
                .get("_is_image")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !is_image {
                continue;
            }

            // Record image source info for text summary
            image_sources_info.push(format!(
                "[图像数据: {}, 格式: {}]",
                d.source,
                d.values
                    .get("image_mime_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            ));

            // Try to get image URL first
            if let Some(url) = d.values.get("image_url").and_then(|v| v.as_str()) {
                if !url.is_empty() {
                    image_parts.push((
                        d.source.clone(),
                        d.data_type.clone(),
                        ImageContent::Url(url.to_string()),
                    ));
                    continue;
                }
            }

            // Fall back to base64 data
            if let Some(base64) = d.values.get("image_base64").and_then(|v| v.as_str()) {
                if !base64.is_empty() {
                    let mime = d
                        .values
                        .get("image_mime_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("image/jpeg");
                    // Clean base64: strip whitespace/newlines that can cause "illegal base64 data" errors
                    let cleaned_base64: String = base64
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect();
                    // Skip if base64 is invalid after cleaning
                    let is_valid = base64::engine::general_purpose::STANDARD
                        .decode(&cleaned_base64)
                        .is_ok();
                    if !is_valid {
                        tracing::warn!(
                            source = %d.source,
                            len = cleaned_base64.len(),
                            "Skipping invalid base64 image data"
                        );
                        continue;
                    }
                    image_parts.push((
                        d.source.clone(),
                        d.data_type.clone(),
                        ImageContent::Base64(cleaned_base64, mime.to_string()),
                    ));
                }
            }
        }

        // Check if LLM supports vision/multimodal
        let llm_supports_vision = llm.capabilities().supports_images;

        // Only use multimodal mode if we have valid images AND LLM supports vision
        let has_valid_images = !image_parts.is_empty() && llm_supports_vision;

        // Log when images are available but LLM doesn't support vision
        if !image_parts.is_empty() && !llm_supports_vision {
            tracing::warn!(
                agent_id = %agent.id,
                image_count = image_parts.len(),
                "Agent has image data but LLM doesn't support vision. Images will be ignored."
            );
        }

        // Build text data summary for non-image data
        // IMPORTANT: Filter out memory-related data to avoid confusing small models
        let max_metrics = 15;
        let text_data_summary: Vec<_> = data
            .iter()
            .filter(|d| {
                // Exclude images
                if d.values
                    .get("_is_image")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    return false;
                }
                // Exclude memory-related data types (confuses small models)
                let data_type_lower = d.data_type.to_lowercase();
                if matches!(
                    data_type_lower.as_str(),
                    "summary" | "memory" | "state_variables" | "baselines" | "patterns"
                ) {
                    return false;
                }
                // Exclude placeholder data from collect_data
                // When no data sources are bound, collect_data adds a placeholder with guidance
                // This placeholder should NOT be treated as real sensor data
                if d.source == "system"
                    && d.values
                        .get("message")
                        .and_then(|v| v.as_str())
                        .map(|s| s.contains("No pre-collected data"))
                        .unwrap_or(false)
                {
                    return false;
                }
                true
            })
            .take(max_metrics)
            .map(|d| {
                // Create a more compact representation of values
                let value_str = if let Some(v) = d.values.get("value") {
                    format!("{}", v) // Compact value representation
                } else if let Some(v) = d.values.get("history") {
                    format!(
                        "[历史数据: {}个点]",
                        v.as_array().map(|a| a.len()).unwrap_or(0)
                    )
                } else {
                    // Fallback to compact JSON - use character-safe truncation
                    let json_str = serde_json::to_string(&d.values).unwrap_or_default();
                    if json_str.chars().count() > 200 {
                        // Truncate at character boundary, not byte boundary
                        json_str.chars().take(200).collect::<String>() + "..."
                    } else {
                        json_str
                    }
                };
                format!("- {}: {} = {}", d.source, d.data_type, value_str)
            })
            .collect();

        // Build intent context
        let _intent_context = if let Some(intent) = parsed_intent.or(agent.parsed_intent.as_ref()) {
            format!(
                "\n意图类型: {:?}\n目标指标: {:?}\n条件: {:?}\n动作: {:?}",
                intent.intent_type, intent.target_metrics, intent.conditions, intent.actions
            )
        } else {
            "".to_string()
        };

        // Build history context from conversation turns and memory
        let mut history_parts = Vec::new();

        // Add memory summary if available
        if !agent.memory.state_variables.is_empty() {
            // Get recent analyses from memory
            if let Some(analyses) = agent
                .memory
                .state_variables
                .get("recent_analyses")
                .and_then(|v| v.as_array())
            {
                if !analyses.is_empty() {
                    let summary: Vec<_> = analyses
                        .iter()
                        .take(1) // Reduced to 1 for small models
                        .filter_map(|a| {
                            a.get("analysis").and_then(|an| an.as_str()).map(|txt| {
                                let conclusion =
                                    a.get("conclusion").and_then(|c| c.as_str()).unwrap_or("");
                                if !conclusion.is_empty() {
                                    format!("- 分析: {} | 结论: {}", txt, conclusion)
                                } else {
                                    format!("- 分析: {}", txt)
                                }
                            })
                        })
                        .collect();

                    if !summary.is_empty() {
                        history_parts.push(format!(
                            "\n## 历史分析 (最近{}次)\n{}",
                            summary.len(),
                            summary.join("\n")
                        ));
                    }
                }
            }

            // === SEMANTIC PATTERNS (Long-term memory) ===
            // Use learned_patterns instead of raw decision_patterns
            // Organized by pattern_type for better context
            if !agent.memory.learned_patterns.is_empty() {
                // Group patterns by type and show only the best from each category
                let mut pattern_groups: std::collections::HashMap<&str, Vec<&LearnedPattern>> =
                    std::collections::HashMap::new();
                for pattern in &agent.memory.learned_patterns {
                    pattern_groups
                        .entry(pattern.pattern_type.as_str())
                        .or_default()
                        .push(pattern);
                }

                // Take only high-confidence patterns (>= 0.7) from each category
                let mut semantic_patterns = Vec::new();
                for (category, patterns) in pattern_groups.iter() {
                    if let Some(&best) =
                        patterns
                            .iter()
                            .filter(|p| p.confidence >= 0.7)
                            .max_by(|a, b| {
                                a.confidence
                                    .partial_cmp(&b.confidence)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                    {
                        semantic_patterns.push(format!(
                            "- [{}] {} (置信度: {:.0}%)",
                            category,
                            best.description,
                            best.confidence * 100.0
                        ));
                    }
                }

                if !semantic_patterns.is_empty() {
                    history_parts.push(format!(
                        "\n## 已验证的决策模式\n{}",
                        semantic_patterns.join("\n")
                    ));
                }
            }

            // === BASELINES (Reference values) ===
            if !agent.memory.baselines.is_empty() {
                let baseline_info: Vec<_> = agent
                    .memory
                    .baselines
                    .iter()
                    .take(3) // Reduced for small models
                    .map(|(metric, value)| format!("- {}: 基线值 {:.2}", metric, value))
                    .collect();
                history_parts.push(format!("\n## 指标基线\n{}", baseline_info.join("\n")));
            }
        }

        // === CONTEXT MANAGEMENT ===
        // === HISTORY CONTEXT - DISABLED for small models ===
        // The compressed history context is NOT used to avoid confusing qwen3:1.7b
        let _history_context = ""; // Intentionally unused

        // === USER MESSAGES (用户发送的消息) ===
        // Build user messages context for adding to user message (not system message)
        let user_messages_for_user_msg = if !agent.user_messages.is_empty() {
            let user_msgs_text: Vec<String> = agent
                .user_messages
                .iter()
                .enumerate()
                .map(|(i, msg)| {
                    let timestamp_str = chrono::DateTime::from_timestamp(msg.timestamp, 0)
                        .map(|dt| dt.format("%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "??".to_string());
                    format!("{}. [{}] {}", i + 1, timestamp_str, msg.content)
                })
                .collect();

            Some(format!(
                "## ⚠️ 用户最新指令 (必须严格遵循)\n\n\
                用户在运行期间发送了以下消息，这些消息包含对执行策略的更新。\
                **请务必将这些指令作为最高优先级，覆盖初始配置中的任何冲突规则：**\n\n\
                {}\n",
                user_msgs_text.join("\n")
            ))
        } else {
            None
        };

        // === SEMANTIC MEMORY CONTEXT ===
        // Compress memory into meaning-preserving format that small models can understand
        let memory_context = {
            let mut parts = Vec::new();

            // 1. Recent success pattern (learned from what works)
            if !agent.memory.short_term.summaries.is_empty() {
                let last_3: Vec<_> = agent
                    .memory
                    .short_term
                    .summaries
                    .iter()
                    .rev()
                    .take(3)
                    .collect();
                let success_rate =
                    last_3.iter().filter(|s| s.success).count() as f32 / last_3.len() as f32;

                if success_rate >= 0.8 {
                    parts.push("Recent: Success pattern established".to_string());
                } else if success_rate <= 0.3 {
                    parts.push("Recent: Multiple failures, needs new approach".to_string());
                }
            }

            // 2. Action patterns (what actions typically work)
            if !agent.memory.learned_patterns.is_empty() {
                let high_confidence: Vec<_> = agent
                    .memory
                    .learned_patterns
                    .iter()
                    .filter(|p| p.confidence >= 0.75)
                    .collect();

                if !high_confidence.is_empty() {
                    let pattern_summary = high_confidence
                        .iter()
                        .map(|p| truncate_to(&p.description, 20))
                        .collect::<Vec<_>>()
                        .join(", ");
                    parts.push(format!("Patterns: {}", pattern_summary));
                }
            }

            // 3. Baseline anomalies (if current values deviate significantly)
            if !agent.memory.baselines.is_empty() && !data.is_empty() {
                // Check if any current data significantly deviates from baseline
                for (metric, baseline) in agent.memory.baselines.iter().take(2) {
                    for d in data.iter().take(3) {
                        if let Some(val) = d.values.get("value").and_then(|v| v.as_f64()) {
                            if (val - baseline).abs() / baseline.abs().max(0.1) > 0.3 {
                                parts.push(format!("Anomaly: {} changed significantly", metric));
                                break;
                            }
                        }
                    }
                }
            }

            if parts.is_empty() {
                String::new()
            } else {
                format!("[Memory: {}]", parts.join(" | "))
            }
        };

        // === SYSTEM PROMPT - Restore original working structure ===
        // This was the proven working format - don't over-engineer it

        // Detect language from user_prompt to determine response language
        let detected_language = SemanticToolMapper::detect_language(&agent.user_prompt);
        let is_chinese = matches!(
            detected_language,
            crate::agent::semantic_mapper::Language::Chinese
                | crate::agent::semantic_mapper::Language::Mixed
        );

        let role_prompt = if is_chinese {
            "你是一个物联网自动化助手。只输出有效的JSON格式，不要输出其他任何文字。"
        } else {
            "You are an IoT automation assistant. Output ONLY valid JSON. No other text."
        };

        // Get current time context for temporal understanding
        let _time_context = get_time_context();

        // Build resources info based on execution mode
        let resources_info = match agent.execution_mode {
            neomind_storage::agents::ExecutionMode::Focused => {
                Self::build_focused_data_table(agent, data)
            }
            neomind_storage::agents::ExecutionMode::Free => {
                let available_commands = Self::build_available_commands_description(agent);
                let available_data_sources = Self::build_available_data_sources_description(agent);
                if available_data_sources.is_empty() {
                    available_commands
                } else {
                    format!("{}\n\n{}", available_commands, available_data_sources)
                }
            }
        };

        // Language-specific templates
        let (output_format_header, user_instruction_header) = if is_chinese {
            (
                "# 输出格式 - 仅输出JSON，不要输出其他任何文字",
                "# 用户指令",
            )
        } else {
            (
                "# Output Format - Output ONLY valid JSON, no other text",
                "# User Instruction",
            )
        };

        let system_prompt = if has_valid_images {
            if is_chinese {
                format!(
                    "{}\n\n{}\n\n{}\n{{\n  \"situation_analysis\": \"图像内容描述\",\n  \"reasoning_steps\": [{{\"step\": 1, \"description\": \"分析步骤\", \"result\": \"该步骤的具体发现\", \"confidence\": 0.9}}],\n  \"decisions\": [{{\"decision_type\": \"info|alert|command\", \"description\": \"描述\", \"action\": \"log或device:command\", \"rationale\": \"理由\", \"confidence\": 0.8}}],\n  \"conclusion\": \"结论\"\n}}\n\n{}\n{}",
                    role_prompt, resources_info, output_format_header, user_instruction_header, agent.user_prompt
                )
            } else {
                format!(
                    "{}\n\n{}\n\n{}\n{{\n  \"situation_analysis\": \"Image content description\",\n  \"reasoning_steps\": [{{\"step\": 1, \"description\": \"Analysis step\", \"result\": \"Specific finding from this step\", \"confidence\": 0.9}}],\n  \"decisions\": [{{\"decision_type\": \"info|alert|command\", \"description\": \"Description\", \"action\": \"log or device:command\", \"rationale\": \"Rationale\", \"confidence\": 0.8}}],\n  \"conclusion\": \"Conclusion\"\n}}\n\n{}\n{}",
                    role_prompt, resources_info, output_format_header, user_instruction_header, agent.user_prompt
                )
            }
        } else if is_chinese {
            format!(
                "{}\n\n{}\n\n{}\n{{\n  \"situation_analysis\": \"情况分析\",\n  \"reasoning_steps\": [{{\"step\": 1, \"description\": \"步骤\", \"result\": \"该步骤的具体发现\", \"confidence\": 0.9}}],\n  \"decisions\": [{{\"decision_type\": \"info|alert|command\", \"description\": \"描述\", \"action\": \"log或device:command\", \"rationale\": \"理由\", \"confidence\": 0.8}}],\n  \"conclusion\": \"结论\"\n}}\n\n{}\n{}",
                role_prompt, resources_info, output_format_header, user_instruction_header, agent.user_prompt
            )
        } else {
            format!(
                "{}\n\n{}\n\n{}\n{{\n  \"situation_analysis\": \"Situation analysis\",\n  \"reasoning_steps\": [{{\"step\": 1, \"description\": \"Step\", \"result\": \"Specific finding from this step\", \"confidence\": 0.9}}],\n  \"decisions\": [{{\"decision_type\": \"info|alert|command\", \"description\": \"Description\", \"action\": \"log or device:command\", \"rationale\": \"Rationale\", \"confidence\": 0.8}}],\n  \"conclusion\": \"Conclusion\"\n}}\n\n{}\n{}",
                role_prompt, resources_info, output_format_header, user_instruction_header, agent.user_prompt
            )
        };

        // === CONTEXT MANAGEMENT ===
        // For image analysis, include minimal memory context
        let memory_context_for_msg = if !memory_context.is_empty() {
            let history_header = if is_chinese {
                "# 历史参考"
            } else {
                "# Historical Reference"
            };
            format!("\n\n{}\n{}", history_header, memory_context)
        } else {
            String::new()
        };

        // Build messages - multimodal if images present
        let messages = if has_valid_images {
            let (current_data_header, important_note, image_only_text) = if is_chinese {
                (
                    "## 当前数据",
                    "重要：只输出JSON格式，不要有任何其他文字。",
                    "仅有图像数据",
                )
            } else {
                (
                    "## Current Data",
                    "Important: Output ONLY JSON format, no other text.",
                    "Image data only",
                )
            };

            let mut parts = vec![ContentPart::text(format!(
                "{}\n{}\n\n{}",
                current_data_header,
                if text_data_summary.is_empty() {
                    // Show image sources info instead of generic "image only" text
                    if !image_sources_info.is_empty() {
                        format!("{}\n{}", image_only_text, image_sources_info.join("\n"))
                    } else {
                        image_only_text.to_string()
                    }
                } else {
                    text_data_summary.join("\n")
                },
                important_note
            ))];

            // Add images
            for (source, _data_type, image_content) in &image_parts {
                match image_content {
                    ImageContent::Base64(data, mime) => {
                        parts.push(ContentPart::image_base64(data.clone(), mime.clone()));
                        tracing::debug!(source = %source, mime = %mime, "Adding base64 image to LLM message");
                    }
                    ImageContent::Url(url) => {
                        parts.push(ContentPart::image_url(url.clone()));
                        tracing::debug!(source = %source, url = %url, "Adding URL image to LLM message");
                    }
                }
            }

            // Add memory context and user messages
            if !memory_context_for_msg.is_empty() {
                parts.push(ContentPart::text(memory_context_for_msg));
            }
            if let Some(ref user_msgs) = user_messages_for_user_msg {
                parts.push(ContentPart::text(format!("\n\n{}", user_msgs)));
            }

            vec![
                Message::new(MessageRole::System, Content::text(system_prompt)),
                Message::from_parts(MessageRole::User, parts),
            ]
        } else {
            // Text-only message
            let data_summary = if text_data_summary.is_empty() {
                // Check if we have image data that couldn't be displayed
                if !image_sources_info.is_empty() {
                    if is_chinese {
                        format!(
                            "当前只有图像数据（LLM 不支持视觉）：\n{}",
                            image_sources_info.join("\n")
                        )
                    } else {
                        format!(
                            "Image data only (LLM doesn't support vision):\n{}",
                            image_sources_info.join("\n")
                        )
                    }
                } else if is_chinese {
                    "当前无预采集的传感器数据。请基于用户指令和已知模式进行分析，如需设备数据请建议用户绑定数据源。".to_string()
                } else {
                    "No pre-collected sensor data available. Analyze based on the user's instructions and known patterns. If device data is needed, suggest the user bind data sources.".to_string()
                }
            } else {
                text_data_summary.join("\n")
            };

            let (current_data_header, json_only_note) = if is_chinese {
                ("## 当前数据", "只输出JSON，不要有其他文字。")
            } else {
                ("## Current Data", "Output ONLY JSON, no other text.")
            };

            let mut user_msg_content = format!(
                "{}\n{}\n\n{}",
                current_data_header, data_summary, json_only_note
            );

            if !memory_context_for_msg.is_empty() {
                user_msg_content = format!("{}\n\n{}", user_msg_content, memory_context_for_msg);
            }
            if let Some(ref user_msgs) = user_messages_for_user_msg {
                user_msg_content = format!("{}\n\n{}", user_msg_content, user_msgs);
            }

            vec![
                Message::new(MessageRole::System, Content::text(system_prompt)),
                Message::new(MessageRole::User, Content::text(user_msg_content)),
            ]
        };

        let input = LlmInput {
            messages,
            params: GenerationParams {
                temperature: Some(0.7),
                max_tokens: Some(5000), // Balanced for speed and completeness
                thinking_enabled: Some(false), // Disable thinking — analyzer needs strict JSON output
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        // Add timeout for LLM generation (5 minutes max)
        const LLM_TIMEOUT_SECS: u64 = 300;
        let llm_result = match tokio::time::timeout(
            std::time::Duration::from_secs(LLM_TIMEOUT_SECS),
            llm.generate(input),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!(
                    agent_id = %agent.id,
                    "LLM generation timed out after {}s",
                    LLM_TIMEOUT_SECS
                );
                return Err(NeoMindError::Llm(format!(
                    "LLM timeout after {}s",
                    LLM_TIMEOUT_SECS
                )));
            }
        };

        match llm_result {
            Ok(output) => {
                let json_str = output.text.trim();
                // Extract JSON if wrapped in markdown
                let json_str = extract_json_from_codeblock(json_str).unwrap_or(json_str);

                // Sanitize control characters that may break JSON parsing
                let sanitized_json = sanitize_json_string(json_str);
                let json_str = sanitized_json.as_str();

                // Parse the LLM response
                // Note: situation_analysis and conclusion can be either String or Object
                // depending on LLM output format, so we use Value and convert later
                #[derive(serde::Deserialize)]
                struct LlmResponse {
                    #[serde(default)]
                    situation_analysis: serde_json::Value,
                    #[serde(default)]
                    reasoning_steps: Vec<ReasoningFromLlm>,
                    #[serde(default)]
                    decisions: Vec<DecisionFromLlm>,
                    #[serde(default)]
                    conclusion: serde_json::Value,
                }

                impl LlmResponse {
                    fn situation_analysis_string(&self) -> String {
                        json_value_to_string(&self.situation_analysis)
                    }
                    fn conclusion_string(&self) -> String {
                        json_value_to_string(&self.conclusion)
                    }
                }

                #[derive(serde::Deserialize)]
                struct ReasoningFromLlm {
                    #[serde(alias = "step_number", default)]
                    step: serde_json::Value,
                    #[serde(alias = "output", default)]
                    description: Option<String>,
                    /// Step-specific result/finding (distinct from the overall situation_analysis)
                    #[serde(default)]
                    result: Option<String>,
                    #[serde(default)]
                    confidence: f32,
                }

                // Helper to extract step number from either string or number
                fn extract_step_number(value: &serde_json::Value, default: u32) -> u32 {
                    match value {
                        serde_json::Value::Number(n) => n.as_u64().unwrap_or(default as u64) as u32,
                        serde_json::Value::String(s) => s.parse().unwrap_or(default),
                        _ => default,
                    }
                }

                #[derive(serde::Deserialize)]
                struct DecisionFromLlm {
                    #[serde(default)]
                    decision_type: Option<String>,
                    #[serde(default)]
                    description: Option<String>,
                    #[serde(default)]
                    action: Option<String>,
                    #[serde(default)]
                    rationale: Option<String>,
                    #[serde(default)]
                    confidence: f32,
                }

                match serde_json::from_str::<LlmResponse>(json_str) {
                    Ok(response) => {
                        let situation_analysis = response.situation_analysis_string();
                        let conclusion = response.conclusion_string();
                        let reasoning_steps: Vec<neomind_storage::ReasoningStep> = response
                            .reasoning_steps
                            .into_iter()
                            .enumerate()
                            .map(|(_i, step)| {
                                let desc = step.description.clone().unwrap_or_default();
                                neomind_storage::ReasoningStep {
                                    step_number: extract_step_number(&step.step, (_i + 1) as u32),
                                    description: desc.clone(),
                                    step_type: "llm_analysis".to_string(),
                                    input: Some(text_data_summary.join("\n")),
                                    output: step.result.or_else(|| Some(desc)).unwrap_or_default(),
                                    confidence: step.confidence,
                                }
                            })
                            .collect();

                        let decisions: Vec<neomind_storage::Decision> = response
                            .decisions
                            .into_iter()
                            .map(|d| neomind_storage::Decision {
                                decision_type: d.decision_type.unwrap_or_default(),
                                description: d.description.unwrap_or_default(),
                                action: d.action.unwrap_or_default(),
                                rationale: d.rationale.unwrap_or_default(),
                                expected_outcome: conclusion.clone(),
                            })
                            .collect();

                        // Emit AgentThinking events for each reasoning step
                        if let Some(ref bus) = self.event_bus {
                            let event_timestamp = chrono::Utc::now().timestamp();
                            for step in &reasoning_steps {
                                let _ = bus
                                    .publish(NeoMindEvent::AgentThinking {
                                        agent_id: agent.id.clone(),
                                        execution_id: execution_id.to_string(),
                                        step_number: step.step_number,
                                        step_type: step.step_type.clone(),
                                        description: step.description.clone(),
                                        details: None,
                                        timestamp: event_timestamp,
                                    })
                                    .await;
                            }

                            // Emit AgentDecision events for each decision
                            for decision in &decisions {
                                let _ = bus
                                    .publish(NeoMindEvent::AgentDecision {
                                        agent_id: agent.id.clone(),
                                        execution_id: execution_id.to_string(),
                                        description: decision.description.clone(),
                                        rationale: decision.rationale.clone(),
                                        action: decision.action.clone(),
                                        confidence: 0.8_f32,
                                        timestamp: event_timestamp,
                                    })
                                    .await;
                            }
                        }

                        Ok((situation_analysis, reasoning_steps, decisions, conclusion))
                    }
                    Err(parse_error) => {
                        // Convert error to string safely to avoid UTF-8 boundary panics
                        let error_str = parse_error.to_string();
                        // Truncate error message safely using char boundaries
                        let error_preview: String = error_str.chars().take(200).collect();

                        tracing::warn!(
                            error = %error_preview,
                            response_preview = %json_str.chars().take(500).collect::<String>(),
                            "Failed to parse LLM JSON response, attempting recovery"
                        );

                        // STEP 1: Try to extract JSON from mixed text (model output text before JSON)
                        if let Some(extracted_json) = extract_json_from_mixed_text(json_str) {
                            tracing::info!(
                                agent_id = %agent.id,
                                extracted_len = extracted_json.len(),
                                "Successfully extracted JSON from mixed text response"
                            );
                            match serde_json::from_str::<LlmResponse>(&extracted_json) {
                                Ok(response) => {
                                    let situation_analysis = response.situation_analysis_string();
                                    let conclusion = response.conclusion_string();
                                    let reasoning_steps: Vec<neomind_storage::ReasoningStep> =
                                        response
                                            .reasoning_steps
                                            .into_iter()
                                            .enumerate()
                                            .map(|(_i, step)| {
                                                let desc =
                                                    step.description.clone().unwrap_or_default();
                                                neomind_storage::ReasoningStep {
                                                    step_number: extract_step_number(
                                                        &step.step,
                                                        (_i + 1) as u32,
                                                    ),
                                                    description: desc.clone(),
                                                    step_type: "llm_analysis".to_string(),
                                                    input: Some(text_data_summary.join("\n")),
                                                    output: step
                                                        .result
                                                        .or_else(|| Some(desc))
                                                        .unwrap_or_default(),
                                                    confidence: step.confidence,
                                                }
                                            })
                                            .collect();

                                    let decisions: Vec<neomind_storage::Decision> = response
                                        .decisions
                                        .into_iter()
                                        .map(|decision| neomind_storage::Decision {
                                            decision_type: decision
                                                .decision_type
                                                .unwrap_or_default(),
                                            description: decision.description.unwrap_or_default(),
                                            action: decision.action.unwrap_or_default(),
                                            rationale: decision.rationale.unwrap_or_default(),
                                            expected_outcome: format!(
                                                "Confidence: {:.0}%",
                                                decision.confidence * 100.0
                                            ),
                                        })
                                        .collect();

                                    return Ok((
                                        situation_analysis,
                                        reasoning_steps,
                                        decisions,
                                        conclusion,
                                    ));
                                }
                                Err(_) => {
                                    tracing::warn!("Extracted JSON failed to parse as LlmResponse");
                                }
                            }
                        }

                        // STEP 2: Try to recover truncated JSON by finding the last complete object
                        let recovered = try_recover_truncated_json(json_str);

                        if let Some((recovered_json, was_truncated)) = recovered {
                            if was_truncated {
                                tracing::info!(
                                    agent_id = %agent.id,
                                    "Successfully recovered truncated JSON response"
                                );
                            }
                            match serde_json::from_str::<LlmResponse>(&recovered_json) {
                                Ok(response) => {
                                    let situation_analysis = response.situation_analysis_string();
                                    let conclusion = response.conclusion_string();
                                    let reasoning_steps: Vec<neomind_storage::ReasoningStep> =
                                        response
                                            .reasoning_steps
                                            .into_iter()
                                            .enumerate()
                                            .map(|(_i, step)| {
                                                let desc =
                                                    step.description.clone().unwrap_or_default();
                                                neomind_storage::ReasoningStep {
                                                    step_number: extract_step_number(
                                                        &step.step,
                                                        (_i + 1) as u32,
                                                    ),
                                                    description: desc.clone(),
                                                    step_type: "llm_analysis".to_string(),
                                                    input: Some(text_data_summary.join("\n")),
                                                    output: step
                                                        .result
                                                        .or_else(|| Some(desc))
                                                        .unwrap_or_default(),
                                                    confidence: step.confidence,
                                                }
                                            })
                                            .collect();

                                    let decisions: Vec<neomind_storage::Decision> = response
                                        .decisions
                                        .into_iter()
                                        .map(|decision| neomind_storage::Decision {
                                            decision_type: decision
                                                .decision_type
                                                .unwrap_or_default(),
                                            description: decision.description.unwrap_or_default(),
                                            action: decision.action.unwrap_or_default(),
                                            rationale: decision.rationale.unwrap_or_default(),
                                            expected_outcome: format!(
                                                "Confidence: {:.0}%",
                                                decision.confidence * 100.0
                                            ),
                                        })
                                        .collect();

                                    return Ok((
                                        situation_analysis,
                                        reasoning_steps,
                                        decisions,
                                        if was_truncated {
                                            format!(
                                                "{} (Response was truncated, some content may be incomplete)",
                                                conclusion
                                            )
                                        } else {
                                            conclusion
                                        },
                                    ));
                                }
                                Err(e) => {
                                    tracing::debug!(error = %e, "Recovered JSON still failed to parse, trying lenient extraction");
                                }
                            }
                        }

                        // Lenient extraction: parse as Value and extract fields (handles different LLM JSON shapes)
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                            if let Some(obj) = value.as_object() {
                                // Use extract_string_field to handle both string and nested object types
                                let situation_analysis =
                                    extract_string_field(obj, "situation_analysis");
                                let conclusion = extract_string_field(obj, "conclusion");
                                let mut reasoning_steps = Vec::new();
                                if let Some(arr) =
                                    obj.get("reasoning_steps").and_then(|v| v.as_array())
                                {
                                    for (i, item) in arr.iter().enumerate() {
                                        let step_num = (i + 1) as u32;
                                        let description: String = item
                                            .get("description")
                                            .and_then(|v| v.as_str())
                                            .or_else(|| item.get("output").and_then(|v| v.as_str()))
                                            .unwrap_or("")
                                            .to_string();
                                        if description.is_empty() {
                                            continue;
                                        }
                                        let confidence = item
                                            .get("confidence")
                                            .and_then(|v| v.as_f64())
                                            .unwrap_or(0.8)
                                            as f32;
                                        let step_result = item
                                            .get("result")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string())
                                            .or_else(|| Some(description.clone()))
                                            .unwrap_or_default();
                                        reasoning_steps.push(neomind_storage::ReasoningStep {
                                            step_number: step_num,
                                            description,
                                            step_type: "llm_analysis".to_string(),
                                            input: Some(text_data_summary.join("\n")),
                                            output: step_result,
                                            confidence,
                                        });
                                    }
                                }
                                let mut decisions = Vec::new();
                                if let Some(arr) = obj.get("decisions").and_then(|v| v.as_array()) {
                                    for item in arr {
                                        let decision_type = item
                                            .get("decision_type")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("analysis")
                                            .to_string();
                                        let description = item
                                            .get("description")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let action = item
                                            .get("action")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("review")
                                            .to_string();
                                        let rationale = item
                                            .get("rationale")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        decisions.push(neomind_storage::Decision {
                                            decision_type,
                                            description,
                                            action,
                                            rationale,
                                            expected_outcome: conclusion.clone(),
                                        });
                                    }
                                }
                                if !situation_analysis.is_empty() || !conclusion.is_empty() {
                                    tracing::info!(
                                        agent_id = %agent.id,
                                        "Extracted decision process from JSON via lenient parsing"
                                    );
                                    return Ok((
                                        if situation_analysis.is_empty() {
                                            conclusion.chars().take(500).collect::<String>()
                                        } else {
                                            situation_analysis.clone()
                                        },
                                        if reasoning_steps.is_empty() {
                                            vec![neomind_storage::ReasoningStep {
                                                step_number: 1,
                                                description: "LLM analysis completed".to_string(),
                                                step_type: "llm_analysis".to_string(),
                                                input: Some(format!("{} data sources", data.len())),
                                                output: situation_analysis
                                                    .chars()
                                                    .take(200)
                                                    .collect::<String>(),
                                                confidence: 0.7,
                                            }]
                                        } else {
                                            reasoning_steps
                                        },
                                        if decisions.is_empty() {
                                            vec![neomind_storage::Decision {
                                                decision_type: "analysis".to_string(),
                                                description: "See situation analysis for details"
                                                    .to_string(),
                                                action: "review".to_string(),
                                                rationale: "LLM provided structured analysis"
                                                    .to_string(),
                                                expected_outcome: conclusion.clone(),
                                            }]
                                        } else {
                                            decisions
                                        },
                                        if conclusion.is_empty() {
                                            "Analysis complete.".to_string()
                                        } else {
                                            conclusion
                                        },
                                    ));
                                } else {
                                    // Both situation_analysis and conclusion are empty, fall through to final fallback
                                    tracing::debug!("JSON was valid but contained no useful data, falling back to raw text");
                                }
                            }
                        } // Close if let Ok(value)

                        // Final fallback: use raw text - show actual content, not placeholder
                        let raw_text = output.text.trim();
                        let situation_analysis = if raw_text.chars().count() > 1000 {
                            raw_text.chars().take(1000).collect::<String>() + "..."
                        } else {
                            raw_text.to_string()
                        };
                        let char_count = raw_text.chars().count();
                        let conclusion = if char_count > 500 {
                            raw_text
                                .chars()
                                .skip(char_count.saturating_sub(500))
                                .collect::<String>()
                                + "..."
                        } else {
                            raw_text.to_string()
                        };

                        let reasoning_steps = vec![neomind_storage::ReasoningStep {
                            step_number: 1,
                            description: if situation_analysis.chars().count() > 200 {
                                situation_analysis.chars().take(200).collect::<String>() + "..."
                            } else {
                                situation_analysis.clone()
                            },
                            step_type: "llm_analysis".to_string(),
                            input: Some(format!("{} data sources", data.len())),
                            output: situation_analysis.clone(),
                            confidence: 0.7,
                        }];

                        let decisions = vec![neomind_storage::Decision {
                            decision_type: "analysis".to_string(),
                            description: "See situation analysis for details".to_string(),
                            action: "review".to_string(),
                            rationale: "LLM provided text response instead of structured JSON"
                                .to_string(),
                            expected_outcome: "Manual review of analysis recommended".to_string(),
                        }];

                        tracing::info!(
                            agent_id = %agent.id,
                            raw_response_length = raw_text.len(),
                            "Using raw LLM response as fallback (content preserved)"
                        );

                        Ok((situation_analysis, reasoning_steps, decisions, conclusion))
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    agent_id = %agent.id,
                    error = %e,
                    error_details = ?e,
                    "LLM generation failed - check LLM backend configuration and connectivity"
                );
                Err(NeoMindError::Llm(format!("LLM generation failed: {}", e)))
            }
        }
    }

    pub(crate) async fn analyze_rule_based(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
        parsed_intent: Option<&neomind_storage::ParsedIntent>,
    ) -> AgentResult<(String, Vec<ReasoningStep>, Vec<Decision>, String)> {
        let mut reasoning_steps = Vec::new();
        let mut decisions = Vec::new();

        // Step 1: Understand the situation
        let situation_analysis = if data.is_empty() {
            format!(
                "No pre-collected data for agent '{}'. Analyzing based on user instructions and configured rules.",
                agent.name
            )
        } else {
            format!(
                "Analyzing {} data points for agent '{}'",
                data.len(),
                agent.name
            )
        };

        reasoning_steps.push(ReasoningStep {
            step_number: 1,
            description: "Collect and analyze input data".to_string(),
            step_type: "data_collection".to_string(),
            input: Some(format!("{} data sources", data.len())),
            output: format!("Data collected from {} sources", data.len()),
            confidence: 1.0,
        });

        // Step 2: Evaluate conditions based on parsed intent
        let intent = parsed_intent.or(agent.parsed_intent.as_ref());
        if let Some(intent) = intent {
            for condition in &intent.conditions {
                let result = self.evaluate_condition(condition, data).await;

                reasoning_steps.push(ReasoningStep {
                    step_number: reasoning_steps.len() as u32 + 1,
                    description: format!("Evaluate condition: {}", condition),
                    step_type: "condition_eval".to_string(),
                    input: Some(condition.clone()),
                    output: format!("Condition result: {}", result),
                    confidence: 0.8,
                });

                if result {
                    decisions.push(Decision {
                        decision_type: "condition_met".to_string(),
                        description: format!("Condition '{}' is met", condition),
                        action: "trigger_actions".to_string(),
                        rationale: format!("The condition '{}' evaluated to true", condition),
                        expected_outcome: "Execute defined actions".to_string(),
                    });
                }
            }
        }

        // Step 3: Determine actions
        let empty_actions = vec![];
        let actions = intent.map(|i| &i.actions).unwrap_or(&empty_actions);
        if !decisions.is_empty() {
            for action in actions {
                reasoning_steps.push(ReasoningStep {
                    step_number: reasoning_steps.len() as u32 + 1,
                    description: format!("Plan action: {}", action),
                    step_type: "action_planning".to_string(),
                    input: Some(action.clone()),
                    output: format!("Action '{}' queued for execution", action),
                    confidence: 0.7,
                });
            }
        }

        let conclusion = if decisions.is_empty() {
            if data.is_empty() {
                "No data available and no conditions met. Consider binding data sources or using an LLM-powered agent for flexible analysis.".to_string()
            } else {
                "No actions required - conditions not met".to_string()
            }
        } else {
            format!("{} action(s) to be executed", decisions.len())
        };

        Ok((situation_analysis, reasoning_steps, decisions, conclusion))
    }

    pub(crate) async fn evaluate_condition(&self, condition: &str, data: &[DataCollected]) -> bool {
        let condition_lower = condition.to_lowercase();

        // Check if any data meets the condition
        for data_item in data {
            if let Some(value) = data_item.values.get("value") {
                if let Some(num) = value.as_f64() {
                    if condition_lower.contains("大于")
                        || condition_lower.contains(">")
                        || condition_lower.contains("超过")
                    {
                        if let Some(threshold) = extract_threshold(&condition_lower) {
                            return num > threshold;
                        }
                    } else if condition_lower.contains("小于")
                        || condition_lower.contains("<")
                        || condition_lower.contains("低于")
                    {
                        if let Some(threshold) = extract_threshold(&condition_lower) {
                            return num < threshold;
                        }
                    }
                }
            }
        }

        false
    }
}
