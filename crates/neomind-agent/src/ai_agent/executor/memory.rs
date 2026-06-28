use super::*;

use neomind_storage::{AgentMemory, ExecutionRecord};

/// Hard cap on the number of knowledge files an agent may accumulate.
///
/// The MemoryTool can append arbitrary new files; without a cap a
/// runaway or long-lived agent bloats both storage and the system
/// prompt — `prefetch_knowledge_files` injects ALL file contents
/// into context on every execution. Same FIFO-trim pattern as
/// `journal.records` and `user_messages` (storage
/// `MAX_USER_MESSAGES=50`).
pub(crate) const MAX_KNOWLEDGE_FILES: usize = 20;

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
        // Reload the latest memory from the store rather than reusing the
        // in-memory snapshot on `agent`. The snapshot was taken when the agent
        // was loaded and may be stale if a concurrent path (e.g. event-trigger
        // retry's failure branch) wrote a journal entry in the meantime. Using
        // the stale snapshot here would overwrite that entry, silently erasing
        // failure patterns the agent is supposed to learn from (gotcha #10).
        let mut memory = match self.store.get_agent(&agent.id).await {
            Ok(Some(data)) => data.memory,
            _ => agent.memory.clone(),
        };

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
    ///
    /// **Total budget constraint**: in addition to the per-file adaptive cap,
    /// the cumulative chars across all inlined files are capped at 40% of the
    /// `context_window_size` (estimated as chars/4 ≈ tokens). Files are loaded
    /// in order until the budget is exhausted; the remaining files fall back
    /// to index-only mode (name + description). This prevents a runaway agent
    /// with 20 knowledge files from consuming 84%+ of the context budget,
    /// starving journal/recent-messages/tool results.
    pub(crate) fn prefetch_knowledge_files(
        &self,
        agent_id: &str,
        knowledge_files: &[neomind_storage::KnowledgeFileRef],
        context_window_size: usize,
    ) -> Option<std::collections::HashMap<String, String>> {
        if knowledge_files.is_empty() {
            return None;
        }

        let store = self.memory_store.as_ref()?;

        // Per-file cap + total budget cap (see `compute_prefetch_budget`).
        let (per_file_limit, total_budget_chars) = compute_prefetch_budget(context_window_size);

        let mut content_map = std::collections::HashMap::new();
        let mut accumulated_chars: usize = 0;
        let mut budget_exhausted = false;

        for f in knowledge_files {
            // Once budget is exhausted, leave remaining files for the index
            // fallback path (caller's `build_history_context` will render them
            // as name+description only).
            if budget_exhausted {
                tracing::info!(
                    agent_id = %agent_id,
                    skipped_file = %f.name,
                    accumulated_chars,
                    total_budget_chars,
                    "Skipping knowledge file prefetch (total budget exhausted)"
                );
                continue;
            }

            match store.read_agent_custom_file(agent_id, &f.name) {
                Ok(content) => {
                    let capped = truncate_to(&content, per_file_limit);
                    accumulated_chars += capped.chars().count();

                    // If adding this file blew the budget, keep it (already
                    // truncated) but signal budget exhaustion so subsequent
                    // files skip prefetch. This avoids partial-file edge cases
                    // and keeps the most-recently-loaded file complete.
                    if accumulated_chars > total_budget_chars {
                        budget_exhausted = true;
                        tracing::info!(
                            agent_id = %agent_id,
                            last_file = %f.name,
                            accumulated_chars,
                            total_budget_chars,
                            "Knowledge prefetch budget reached after this file"
                        );
                    }

                    content_map.insert(f.name.clone(), capped);
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

/// Compute `(per_file_limit, total_budget_chars)` for `prefetch_knowledge_files`.
///
/// Pure function extracted for testability — the budget decision is the only
/// nontrivial logic in the prefetch path (file IO + HashMap building is
/// mechanical). Keeping it standalone lets us verify the tier boundaries and
/// the budget-floor behaviour without spinning up a full `AgentExecutor`.
///
/// - `per_file_limit`: per-file char cap. Tiers: >64K context → 20000,
///   >16K → 16000, else 8000.
/// - `total_budget_chars`: cumulative char budget across all inlined files.
///   Computed as `context_window_size * 0.40 * 4` (~4 chars/token), floored
///   at 16K chars so small-context models still get usable content. Files
///   past the budget fall back to index-only rendering in `build_history_context`.
pub(crate) fn compute_prefetch_budget(context_window_size: usize) -> (usize, usize) {
    let per_file_limit = if context_window_size > 64000 {
        20000
    } else if context_window_size > 16000 {
        16000
    } else {
        8000
    };

    let total_budget_chars = ((context_window_size as f64 * 0.40) * 4.0) as usize;
    let total_budget_chars = total_budget_chars.max(16_000);

    (per_file_limit, total_budget_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefetch_budget_small_context_uses_floor() {
        // 4K context → 4K * 0.4 * 4 = 6400 chars, below the 16K floor
        let (per_file, total) = compute_prefetch_budget(4096);
        assert_eq!(per_file, 8000);
        assert_eq!(total, 16_000);
    }

    #[test]
    fn prefetch_budget_mid_context() {
        // 32K context → per_file 16K, total = floor(32768 * 0.4 * 4) = 52428 chars
        let (per_file, total) = compute_prefetch_budget(32_768);
        assert_eq!(per_file, 16_000);
        assert_eq!(total, 52_428);
    }

    #[test]
    fn prefetch_budget_large_context() {
        // 128K context → per_file 20K, total = floor(131072 * 0.4 * 4) = 209715 chars.
        // For a 128K-token window, ~210K chars ≈ 52K tokens — about 40% of
        // the context, leaving ~76K tokens for actual conversation. With
        // MAX_KNOWLEDGE_FILES=20 × 20K chars/file = 400K raw, this cap
        // cuts off at ~half — preventing knowledge files alone from
        // consuming the majority of the history budget.
        let (per_file, total) = compute_prefetch_budget(131_072);
        assert_eq!(per_file, 20_000);
        assert_eq!(total, 209_715);
    }

    #[test]
    fn prefetch_budget_tier_boundary_16k() {
        // Exactly 16K is the "small" tier (per_file 8K) — the >16K branch
        // starts at 16381+
        let (per_file_at_16k, _) = compute_prefetch_budget(16_000);
        let (per_file_above, _) = compute_prefetch_budget(16_381);
        assert_eq!(per_file_at_16k, 8_000);
        assert_eq!(per_file_above, 16_000);
    }

    #[test]
    fn prefetch_budget_tier_boundary_64k() {
        let (per_file_at_64k, _) = compute_prefetch_budget(64_000);
        let (per_file_above, _) = compute_prefetch_budget(64_001);
        assert_eq!(per_file_at_64k, 16_000);
        assert_eq!(per_file_above, 20_000);
    }
}
