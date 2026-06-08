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

    /// Auto-initialize a knowledge file when the agent has none yet.
    /// Covers both newly-created agents (whose init happened at creation time)
    /// and legacy agents created before the init-at-creation feature was added.
    pub(crate) fn auto_init_knowledge_file(
        &self,
        agent: &AiAgent,
        updated_memory: &mut AgentMemory,
        _conclusion: &str,
    ) {
        // Skip if agent already has knowledge files
        if !updated_memory.knowledge_files.is_empty() {
            return;
        }

        // Must have at least one journal entry (completed an execution)
        if updated_memory.journal.records.is_empty() {
            return;
        }

        let Some(ref store) = self.memory_store else {
            return;
        };

        let now = chrono::Utc::now().timestamp();

        // Build resources summary
        let resources_summary = if agent.resources.is_empty() {
            "None (free mode)".to_string()
        } else {
            agent
                .resources
                .iter()
                .map(|r| format!("- {} ({})", r.name, r.resource_id))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // Build identity section
        let default_identity = format!(
            "You are an intelligent IoT agent named '{}' monitoring edge devices.",
            agent.name
        );
        let identity_section = agent
            .system_prompt
            .as_deref()
            .unwrap_or(&default_identity);

        // Build schedule info
        let schedule_info = match &agent.schedule.schedule_type {
            neomind_storage::ScheduleType::Interval => format!(
                "Interval: every {}s",
                agent.schedule.interval_seconds.unwrap_or(300)
            ),
            neomind_storage::ScheduleType::Cron => format!(
                "Cron: {}",
                agent.schedule.cron_expression.as_deref().unwrap_or("?")
            ),
            neomind_storage::ScheduleType::Event => "Event-driven".to_string(),
        };

        // Latest execution info
        let latest = updated_memory.journal.records.last();
        let latest_exec = match latest {
            Some(r) => format!(
                "- Latest execution: {} ({})",
                truncate_to(&r.outcome, 80),
                chrono::DateTime::from_timestamp(r.timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            None => "- No executions yet".to_string(),
        };

        let content = format!(
            "# Task Understanding\n\
             \n\
             ## Identity & Role\n\
             {}\n\
             \n\
             ## Mission\n\
             {}\n\
             \n\
             ## Bound Resources\n\
             {}\n\
             \n\
             ## Schedule\n\
             {}\n\
             \n\
             ## Status\n\
             - Execution mode: {:?}\n\
             - Total executions: {}\n\
             {}\n\
             - Initialized: {}\n\
             \n\
             ## Memory Commands\n\
             - Read this file: `memory(action='read', target='custom:task-understanding')`\n\
             - Update this file: `memory(action='add', target='custom:task-understanding', content='## New Section\\n...')`\n\
             - Create new knowledge file: `memory(action='create', target='custom:{{{{name}}}}', content='...')`\n\
             \n\
             ## Notes\n\
             This file was auto-created. Update it as you learn more about the environment and discover patterns.",
            identity_section,
            agent.user_prompt,
            resources_summary,
            schedule_info,
            agent.execution_mode,
            updated_memory.journal.records.len(),
            latest_exec,
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

        // Also update the per-execution handle so the MemoryTool sees it
        // Note: The handle is passed to update_memory from the caller (execute_internal)
        // For auto_init, the updated_memory.knowledge_files will be synced by the caller
        // via per_exec_knowledge_files handle after this method returns.

        tracing::info!(
            agent_id = %agent.id,
            "Auto-initialized knowledge file: task-understanding"
        );
    }
}
