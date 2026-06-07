use super::*;

use neomind_storage::ExecutionRecord;

impl AgentExecutor {
    /// Update agent memory with a new execution record.
    /// Simple FIFO journal — no complex filtering or LLM reflection.
    pub(crate) async fn update_memory(
        &self,
        agent: &AiAgent,
        decisions: &[Decision],
        conclusion: &str,
        execution_id: &str,
        success: bool,
    ) -> AgentResult<AgentMemory> {
        let mut memory = agent.memory.clone();

        let outcome = truncate_to(conclusion, 100);
        let action_taken = decisions
            .iter()
            .filter(|d| matches!(d.decision_type.as_str(), "alert" | "command"))
            .map(|d| truncate_to(&d.description, 50))
            .collect::<Vec<_>>()
            .join("; ");
        let action_taken = if action_taken.is_empty() {
            "no action".to_string()
        } else {
            action_taken
        };

        memory.journal.records.push(ExecutionRecord {
            timestamp: chrono::Utc::now().timestamp(),
            execution_id: execution_id.to_string(),
            outcome,
            action_taken,
            success,
        });

        // FIFO — keep only max_records
        while memory.journal.records.len() > memory.journal.max_records {
            memory.journal.records.remove(0);
        }

        memory.updated_at = chrono::Utc::now().timestamp();

        tracing::debug!(
            agent_id = %agent.id,
            execution_id = %execution_id,
            journal_len = memory.journal.records.len(),
            success,
            "Agent memory updated"
        );

        Ok(memory)
    }

    /// Auto-initialize a knowledge file on the agent's first execution.
    /// Creates `task-understanding.md` with task summary and first execution results.
    pub(crate) fn auto_init_knowledge_file(
        &self,
        agent: &AiAgent,
        updated_memory: &mut AgentMemory,
        conclusion: &str,
    ) {
        // Only on first execution (journal had 0 records, now has 1) and no knowledge files yet
        if updated_memory.journal.records.len() != 1 || !updated_memory.knowledge_files.is_empty() {
            return;
        }

        let Some(ref store) = self.memory_store else {
            return;
        };

        let now = chrono::Utc::now().timestamp();

        // Build task understanding content
        let resources_summary = if agent.resources.is_empty() {
            "None".to_string()
        } else {
            agent
                .resources
                .iter()
                .map(|r| format!("- {} ({})", r.name, r.resource_id))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let content = format!(
            "# Task Understanding\n\
             \n\
             ## Mission\n\
             {}\n\
             \n\
             ## Bound Resources\n\
             {}\n\
             \n\
             ## First Execution\n\
             - Result: {}\n\
             - Time: {}\n\
             \n\
             ## Notes\n\
             This file was auto-created. Update it as you learn more about the environment.",
            agent.user_prompt,
            resources_summary,
            truncate_to(conclusion, 200),
            chrono::DateTime::from_timestamp(now, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        );

        // Write file to agent-scoped directory
        if let Err(e) = store.write_agent_custom_file(&agent.id, "task-understanding", &content) {
            tracing::warn!(
                agent_id = %agent.id,
                "Failed to auto-init knowledge file: {}", e
            );
            return;
        }

        // Register in knowledge_files index
        updated_memory.knowledge_files.push(neomind_storage::KnowledgeFileRef {
            name: "task-understanding".to_string(),
            description: "Auto-created task summary and first execution record".to_string(),
            created_at: now,
            updated_at: now,
        });

        // Also update the shared handle so the MemoryTool sees it
        if let Some(ref handle) = self.memory_knowledge_files_handle {
            if let Ok(mut guard) = handle.try_write() {
                *guard = updated_memory.knowledge_files.clone();
            }
        }

        tracing::info!(
            agent_id = %agent.id,
            "Auto-initialized knowledge file: task-understanding"
        );
    }
}
