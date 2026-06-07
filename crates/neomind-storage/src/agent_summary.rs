//! Agent memory bridge: extract readable Markdown summaries from AgentMemory.

use crate::agents::AgentMemory;

/// Format an agent's memory into a readable Markdown summary.
/// Shows recent journal entries and knowledge file list.
/// Returns empty string if no data.
pub fn format_agent_summary(memory: &AgentMemory, _agent_name: &str, max_chars: usize) -> String {
    let mut lines: Vec<String> = Vec::new();

    // Knowledge files
    for f in &memory.knowledge_files {
        lines.push(format!("- **{}**: {}", f.name, f.description));
    }

    // Recent journal entries
    for r in memory.journal.records.iter().rev().take(5) {
        let status = if r.success { "OK" } else { "FAIL" };
        lines.push(format!("- [{}] {}", status, r.outcome));
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

    #[test]
    fn test_format_summary_with_journal() {
        let mut memory = AgentMemory::default();
        memory.journal.records.push(ExecutionRecord {
            timestamp: 1000,
            execution_id: "exec-1".into(),
            outcome: "Temperature normal at 25°C".into(),
            action_taken: "no action".into(),
            success: true,
        });
        let summary = format_agent_summary(&memory, "test-agent", 500);
        assert!(summary.contains("Temperature normal"));
    }

    #[test]
    fn test_format_summary_truncates() {
        let mut memory = AgentMemory::default();
        for i in 0..50 {
            memory.journal.records.push(ExecutionRecord {
                timestamp: 1000 + i,
                execution_id: format!("exec-{i}"),
                outcome: format!("Execution {i} with a somewhat long outcome description to test truncation behavior"),
                action_taken: "no action".into(),
                success: true,
            });
        }
        let summary = format_agent_summary(&memory, "test-agent", 200);
        assert!(summary.chars().count() <= 200, "Summary was {} chars", summary.chars().count());
    }

    #[test]
    fn test_empty_memory() {
        let memory = AgentMemory::default();
        let summary = format_agent_summary(&memory, "test-agent", 500);
        assert!(summary.is_empty());
    }
}
