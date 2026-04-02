//! Memory extraction for Markdown-based system memory.
//!
//! Extracts memory entries from:
//! - AI Agent execution results
//! - Chat session context
//!
//! Also integrates optional LLM-based refinement.

use neomind_storage::{
    MarkdownMemoryStore, MemoryCategory, MemorySource,
    SystemMemoryEntry as MemoryEntry,
};
use std::sync::Arc;

use crate::agent::conversation_context::{ConversationContext, ConversationTopic};
use neomind_storage::agents::{AgentExecutionRecord, ExecutionStatus};

use crate::error::Result;

/// Extract memory entries from agent execution result.
pub fn extract_from_agent_execution(
    record: &AgentExecutionRecord,
    agent_name: &str,
) -> Vec<MemoryEntry> {
    let mut entries = Vec::new();
    let timestamp = record.timestamp;
    let source = MemorySource::Agent {
        id: record.agent_id.clone(),
        name: agent_name.to_string(),
    };

    // Only extract from successful executions
    if record.status == ExecutionStatus::Completed {
        let dp = &record.decision_process;

        // Extract patterns from reasoning steps
        for step in dp.reasoning_steps.iter() {
            let step_text = &step.description;
            if step_text.contains("pattern") || step_text.contains("规律") || step_text.contains("模式") {
                entries.push(MemoryEntry::new(
                    step_text.clone(),
                    MemoryCategory::TaskPatterns,
                    source.clone(),
                ).with_importance(calculate_importance_from_confidence(step.confidence))
                .with_timestamp(timestamp));
            }
        }

        // Extract facts from situation analysis (stored in conclusion field of DecisionProcess)
        let analysis = &dp.conclusion;
        if !analysis.is_empty() {
            for line in analysis.lines() {
                let trimmed = line.trim();
                if trimmed.len() > 10 && is_fact_like(trimmed) {
                    entries.push(MemoryEntry::new(
                        trimmed.to_string(),
                        MemoryCategory::DomainKnowledge,
                        source.clone(),
                    ).with_importance(40)
                    .with_timestamp(timestamp));
                }
            }
        }

        // Extract learned patterns from conclusion with high confidence
        if !dp.conclusion.is_empty() && dp.confidence > 0.7 {
            entries.push(MemoryEntry::new(
                format!(
                    "结论: {} (置信度: {:.0}%)",
                    dp.conclusion,
                    (dp.confidence * 100.0) as u32
                ),
                MemoryCategory::TaskPatterns,
                source.clone(),
            ).with_importance((dp.confidence * 80.0) as u8)
            .with_timestamp(timestamp));
        }

        // Extract entities from data collected
        for data in &dp.data_collected {
            // Extract device entities from values
            if let serde_json::Value::Object(ref obj) = data.values {
                if let (Some(device_id), Some(device_name)) = (
                    obj.get("device_id").and_then(|v| v.as_str()),
                    obj.get("device_name").and_then(|v| v.as_str()),
                ) {
                    entries.push(MemoryEntry::new(
                        format!("设备: {} ({})", device_name, device_id),
                        MemoryCategory::DomainKnowledge,
                        source.clone(),
                    ).with_importance(30)
                    .with_timestamp(timestamp));
                }
            }
        }
    }

    // Deduplicate - keep only unique entries
    let mut seen = std::collections::HashSet::new();
    entries.retain(|e| {
        let key = format!("{:?}:{}", e.category, e.content);
        seen.insert(key)
    });

    entries
}

/// Calculate importance from confidence score.
fn calculate_importance_from_confidence(confidence: f32) -> u8 {
    // Scale confidence (0.0-1.0) to importance (0-100)
    let raw = (confidence * 100.0) as u8;
    raw.min(100).max(0)
}

/// Check if text looks like a fact.
fn is_fact_like(text: &str) -> bool {
    // Facts usually contain specific values, dates, or measurements
    // Use more specific patterns to avoid false positives

    // Check for explicit value indicators
    let value_indicators = ["：", ":", "=", "等于"];
    if value_indicators.iter().any(|p| text.contains(p)) {
        return true;
    }

    // Check for number + unit patterns like "25度", "30秒", "10分"
    // Must have a digit followed by a unit character
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len().saturating_sub(1) {
        if chars[i].is_ascii_digit() {
            let next_char = chars[i + 1];
            if matches!(next_char, '度' | '秒' | '分' | '时' | '米' | '克' | '%') {
                return true;
            }
        }
    }

    false
}

/// Extract memory entries from chat session context.
pub fn extract_from_chat_context(
    context: &ConversationContext,
    session_id: &str,
    conversation_summary: Option<&str>,
) -> Vec<MemoryEntry> {
    let mut entries = Vec::new();
    let timestamp = chrono::Utc::now().timestamp();
    let source = MemorySource::Chat {
        session_id: session_id.to_string(),
    };

    // Extract topic as pattern
    if context.topic != ConversationTopic::General {
        let topic_str = match context.topic {
            ConversationTopic::DeviceControl => "设备控制对话",
            ConversationTopic::DataQuery => "数据查询对话",
            ConversationTopic::RuleCreation => "规则创建对话",
            ConversationTopic::WorkflowDesign => "工作流设计对话",
            ConversationTopic::General => "通用对话",
        };
        entries.push(MemoryEntry::new(
            format!("对话主题: {}", topic_str),
            MemoryCategory::TaskPatterns,
            source.clone(),
        ).with_importance(25)
        .with_timestamp(timestamp));
    }

    // Extract location entities
    for location in &context.mentioned_locations {
        entries.push(MemoryEntry::new(
            format!("位置: {} ({})", location.name, location.id),
            MemoryCategory::DomainKnowledge,
            source.clone(),
        ).with_importance(25)
        .with_timestamp(timestamp));
    }

    // Extract device entities
    for device in &context.mentioned_devices {
        entries.push(MemoryEntry::new(
            format!("设备: {} ({})", device.name, device.id),
            MemoryCategory::DomainKnowledge,
            source.clone(),
        ).with_importance(30)
        .with_timestamp(timestamp));
    }

    // Extract preferences from context summary
    if let Some(summary) = conversation_summary {
        let summary_lower = summary.to_lowercase();
        if summary_lower.contains("prefer")
            || summary_lower.contains("偏好")
            || summary_lower.contains("喜欢")
        {
            entries.push(MemoryEntry::new(
                format!(
                    "用户偏好: {}",
                    summary.chars().take(200).collect::<String>()
                ),
                MemoryCategory::UserProfile,
                source.clone(),
            ).with_importance(60)
            .with_timestamp(timestamp));
        }
    }

    entries
}

/// Extract and persist memory from agent execution.
pub async fn persist_agent_memory(
    memory_store: &Arc<MarkdownMemoryStore>,
    record: &AgentExecutionRecord,
    agent_name: &str,
) -> Result<()> {
    let entries = extract_from_agent_execution(record, agent_name);
    if entries.is_empty() {
        return Ok(());
    }

    let source = MemorySource::Agent {
        id: record.agent_id.clone(),
        name: agent_name.to_string(),
    };
    memory_store.append_batch(&source, &entries)?;

    tracing::debug!(
        agent_id = %record.agent_id,
        agent_name = %agent_name,
        entries_count = entries.len(),
        "Extracted and persisted agent memory"
    );

    Ok(())
}

/// Extract and persist memory from chat session.
pub async fn persist_chat_memory(
    memory_store: &Arc<MarkdownMemoryStore>,
    context: &ConversationContext,
    session_id: &str,
    conversation_summary: Option<&str>,
) -> Result<()> {
    let entries = extract_from_chat_context(context, session_id, conversation_summary);
    if entries.is_empty() {
        return Ok(());
    }

    let source = MemorySource::Chat {
        session_id: session_id.to_string(),
    };
    memory_store.append_batch(&source, &entries)?;

    tracing::debug!(
        session_id = %session_id,
        entries_count = entries.len(),
        "Extracted and persisted chat memory"
    );

    Ok(())
}

/// Prune memory - remove old/low-importance entries.
pub fn prune_memory(
    memory_store: &Arc<MarkdownMemoryStore>,
    source: &MemorySource,
    max_entries: usize,
) -> Result<()> {
    let pruned_count = memory_store.prune(source, max_entries)?;
    tracing::info!(
        max_entries = max_entries,
        pruned_count = pruned_count,
        "Pruned system memory"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use neomind_storage::agents::{DecisionProcess, ReasoningStep};

    #[test]
    fn test_extract_from_agent_execution() {
        let record = AgentExecutionRecord {
            id: "exec-001".to_string(),
            agent_id: "agent-001".to_string(),
            timestamp: 1712012400,
            trigger_type: "manual".to_string(),
            status: ExecutionStatus::Completed,
            decision_process: DecisionProcess {
                situation_analysis: "温度为25度".to_string(),
                data_collected: vec![],
                reasoning_steps: vec![ReasoningStep {
                    step_number: 1,
                    description: "发现规律: 温度持续上升".to_string(),
                    step_type: "analysis".to_string(),
                    input: None,
                    output: "温度上升".to_string(),
                    confidence: 0.85,
                }],
                decisions: vec![],
                conclusion: "温度正常".to_string(),
                confidence: 0.85,
            },
            result: None,
            duration_ms: 1000,
            error: None,
        };

        let entries = extract_from_agent_execution(&record, "温度监控");
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_extract_from_chat_context() {
        let mut context = ConversationContext::new();
        context.add_device("客厅灯".to_string());
        context.add_location("客厅".to_string());

        let entries = extract_from_chat_context(&context, "session-001", None);
        assert!(!entries.is_empty());

        // Should have device and location entities (now mapped to DomainKnowledge)
        assert!(entries
            .iter()
            .any(|e| matches!(e.category, MemoryCategory::DomainKnowledge)));
    }

    #[test]
    fn test_is_fact_like() {
        // Should match: value indicators (colon, equals)
        assert!(is_fact_like("温度: 25"));
        assert!(is_fact_like("温度：25度"));
        assert!(is_fact_like("温度=25度"));

        // Should match: number + unit
        assert!(is_fact_like("当前温度25度"));
        assert!(is_fact_like("延迟10秒"));
        assert!(is_fact_like("湿度60%"));

        // Should NOT match: casual statements without specific values
        assert!(!is_fact_like("今天天气很好"));
        assert!(!is_fact_like("温度是正常水平"));  // No specific value
    }
}
