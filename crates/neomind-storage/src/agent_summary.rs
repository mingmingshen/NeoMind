//! Agent memory bridge: extract readable Markdown summaries from AgentMemory.

use crate::agents::AgentMemory;

/// Format an agent's memory into a readable Markdown summary.
/// Filters by importance/confidence thresholds and truncates to `max_chars`.
/// Returns empty string if no significant memories.
pub fn format_agent_summary(memory: &AgentMemory, _agent_name: &str, max_chars: usize) -> String {
    let mut lines: Vec<String> = Vec::new();

    // Long-term memories (importance >= 0.7)
    for m in &memory.long_term.memories {
        if m.importance >= 0.7 {
            lines.push(format!("- {}", &m.content));
        }
    }

    // Learned patterns (confidence >= 0.8)
    for p in &memory.long_term.patterns {
        if p.confidence >= 0.8 {
            lines.push(format!("- [{}] {} ({:.0}%)", p.pattern_type, p.description, p.confidence * 100.0));
        }
    }

    if lines.is_empty() {
        return String::new();
    }

    let mut summary = lines.join("\n");

    // Truncate to max_chars at last newline boundary
    if summary.chars().count() > max_chars {
        let truncated: String = summary.chars().take(max_chars).collect();
        if let Some(pos) = truncated.rfind('\n') {
            summary = truncated[..pos].to_string();
        } else {
            summary = truncated;
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::*;

    fn make_agent_memory() -> AgentMemory {
        AgentMemory {
            long_term: LongTermMemory {
                memories: vec![ImportantMemory {
                    id: "test-1".into(),
                    memory_type: "anomaly_detection".into(),
                    content: "Temperature sensor exceeds 30 degrees regularly".into(),
                    importance: 0.8,
                    created_at: 1000,
                    last_accessed_at: 2000,
                    access_count: 5,
                    metadata: Default::default(),
                }],
                patterns: vec![LearnedPattern {
                    id: "p-1".into(),
                    pattern_type: "anomaly_detection".into(),
                    description: "Temp anomaly triggers alert".into(),
                    confidence: 0.9,
                    learned_at: 1000,
                    data: Default::default(),
                }],
                max_memories: 50,
                min_importance: 0.5,
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_format_summary_under_limit() {
        let memory = make_agent_memory();
        let summary = format_agent_summary(&memory, "test-agent", 500);
        assert!(summary.chars().count() <= 500);
        assert!(summary.contains("Temperature sensor"));
        assert!(summary.contains("anomaly_detection"));
    }

    #[test]
    fn test_format_summary_truncates() {
        let mut memory = make_agent_memory();
        for i in 0..50 {
            memory.long_term.memories.push(ImportantMemory {
                id: format!("m-{i}"),
                memory_type: "test".into(),
                content: format!("Memory entry {i} with some descriptive text that makes it longer"),
                importance: 0.9,
                created_at: 1000 + i,
                last_accessed_at: 2000,
                access_count: 1,
                metadata: Default::default(),
            });
        }
        let summary = format_agent_summary(&memory, "test-agent", 500);
        assert!(summary.chars().count() <= 500, "Summary was {} chars", summary.chars().count());
    }

    #[test]
    fn test_empty_memory() {
        let memory = AgentMemory::default();
        let summary = format_agent_summary(&memory, "test-agent", 500);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_filters_low_importance() {
        let mut memory = AgentMemory::default();
        memory.long_term.memories.push(ImportantMemory {
            id: "low".into(),
            memory_type: "test".into(),
            content: "Low importance memory".into(),
            importance: 0.3, // Below 0.7 threshold
            created_at: 1000,
            last_accessed_at: 2000,
            access_count: 1,
            metadata: Default::default(),
        });
        let summary = format_agent_summary(&memory, "test-agent", 500);
        assert!(!summary.contains("Low importance"));
    }
}
