use super::*;

use neomind_storage::{AgentMemory, ExecutionRecord};

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

        let outcome = truncate_to(conclusion, 300);
        let action_taken = decisions
            .iter()
            .take(5)
            .map(|d| truncate_to(&d.action, 150))
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
        success: bool,
    ) {
        // Skip if agent already has knowledge files
        if !updated_memory.knowledge_files.is_empty() {
            return;
        }

        // Must have at least one journal entry (completed an execution)
        if updated_memory.journal.records.is_empty() {
            return;
        }

        // Only auto-init on successful executions — failed runs would
        // pollute the knowledge file with error patterns
        if !success {
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
        let identity_section = agent.system_prompt.as_deref().unwrap_or(&default_identity);

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

        let content = format!(
            "# Task Understanding\n\
             \n\
             ## Role\n\
             {}\n\
             \n\
             ## Mission\n\
             {}\n\
             \n\
             ## Resources\n\
             {}\n\
             \n\
             ## Schedule\n\
             {}\n\
             \n\
             ---\n\
             Update this file as you discover thresholds, patterns, and device quirks. Append only NEW findings — never re-list previous entries.",
            identity_section,
            agent.user_prompt,
            resources_summary,
            schedule_info,
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
        updated_memory
            .knowledge_files
            .push(neomind_storage::KnowledgeFileRef {
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

    /// Pre-fetch knowledge file contents from disk for inline injection into
    /// the system prompt. Avoids wasting a tool-call round reading files the
    /// agent already knows about — especially valuable in Focused+ mode with
    /// only 3 rounds (33% of budget saved).
    pub(crate) fn prefetch_knowledge_files(
        &self,
        agent_id: &str,
        knowledge_files: &[neomind_storage::KnowledgeFileRef],
    ) -> Option<std::collections::HashMap<String, String>> {
        if knowledge_files.is_empty() {
            return None;
        }

        let store = self.memory_store.as_ref()?;

        let mut content_map = std::collections::HashMap::new();
        for f in knowledge_files {
            match store.read_agent_custom_file(agent_id, &f.name) {
                Ok(content) => {
                    // Truncate individual files to 12000 chars — scaled up from 6000
                    // to match the raised write limit (20000) so long-task context
                    // written in one execution is actually visible in the next.
                    // Still bounded per-file to avoid context bloat on small models.
                    content_map.insert(f.name.clone(), truncate_to(&content, 12000));
                }
                Err(e) => {
                    tracing::debug!(
                        agent_id = %agent_id,
                        file = %f.name,
                        error = %e,
                        "Failed to pre-fetch knowledge file, will rely on index"
                    );
                }
            }
        }

        if content_map.is_empty() {
            None
        } else {
            Some(content_map)
        }
    }
}
