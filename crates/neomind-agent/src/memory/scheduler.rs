//! Memory scheduler for background maintenance tasks
//!
//! Runs periodic tasks for:
//! - System resource summary (data-driven, no LLM calls)
//! - Agent memory bridge (redb → markdown)
//! - Temp file cleanup (session directories)

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use super::manager::MemoryManager;
use neomind_storage::{AgentStore, AiAgent, MarkdownMemoryStore, MemoryConfig};

/// Maximum number of agent files to keep
const MAX_AGENT_FILES: usize = 5;

/// Memory scheduler for background tasks
pub struct MemoryScheduler {
    #[allow(dead_code)]
    manager: Arc<RwLock<MemoryManager>>,
    store: Arc<RwLock<MarkdownMemoryStore>>,
    config: MemoryConfig,
    /// Optional agent store for agent memory bridge job
    agent_store: Option<Arc<AgentStore>>,
    job_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MemoryScheduler {
    /// Create a new scheduler
    pub fn new(
        manager: Arc<RwLock<MemoryManager>>,
        store: Arc<RwLock<MarkdownMemoryStore>>,
        config: MemoryConfig,
    ) -> Self {
        Self {
            manager,
            store,
            config,
            agent_store: None,
            job_handle: None,
        }
    }

    /// Set the agent store (enables Job 2: Agent Memory Bridge)
    pub fn with_agent_store(mut self, agent_store: Arc<AgentStore>) -> Self {
        self.agent_store = Some(agent_store);
        self
    }

    /// Start background jobs
    pub fn start(&mut self) {
        if !self.config.enabled {
            info!("Memory system disabled, not starting scheduler");
            return;
        }

        let store = self.store.clone();
        let config = self.config.clone();
        let agent_store = self.agent_store.clone();
        let interval_secs = config.schedule_interval_secs;

        self.job_handle = Some(tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(interval_secs));
            let mut cleanup_timer = interval(Duration::from_secs(86400)); // 24 hours

            info!(
                interval_secs = interval_secs,
                has_agent_store = agent_store.is_some(),
                "Memory scheduler started"
            );

            // Trigger first cleanup immediately
            cleanup_timer.tick().await;

            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        // Job 1: System Resource Summary
                        if let Err(e) = Self::run_system_summary_job(&store).await {
                            error!(error = %e, "System summary job failed");
                        }

                        // Job 2: Agent Memory Bridge (if agent store available)
                        if let Some(ref agent_store) = agent_store {
                            if let Err(e) = Self::run_agent_bridge_job(&store, agent_store, &config).await {
                                error!(error = %e, "Agent bridge job failed");
                            }
                        }
                    }
                    _ = cleanup_timer.tick() => {
                        // Job 3: Temp File Cleanup
                        if let Err(e) = Self::run_temp_cleanup_job(&store, &config).await {
                            error!(error = %e, "Temp cleanup job failed");
                        }
                    }
                }
            }
        }));
    }

    /// Job 1: System Resource Summary
    /// Generates a data-driven summary of system resources and updates KNOWLEDGE.md
    async fn run_system_summary_job(
        store: &Arc<RwLock<MarkdownMemoryStore>>,
    ) -> Result<(), String> {
        // TODO: Wire up real system state queries when integrating with API layer
        // For now, use placeholder values
        let (devices, rules, extensions, dashboards) = Self::get_system_counts().await;

        let summary = generate_system_summary(devices, rules, extensions, dashboards);

        let store_guard = store.read().await;
        store_guard
            .replace_section("knowledge", "System Resources", &summary)
            .await
            .map_err(|e| format!("Failed to update system summary: {}", e))?;

        info!(
            devices = devices,
            rules = rules,
            extensions = extensions,
            dashboards = dashboards,
            summary_len = summary.len(),
            "System summary updated"
        );

        Ok(())
    }

    /// Get system resource counts (stubbed for now)
    async fn get_system_counts() -> (usize, usize, usize, usize) {
        // TODO: Integrate with real system state
        // - Device count from device store
        // - Rule count from rule engine
        // - Extension count from extension registry
        // - Dashboard count from dashboard store
        (0, 0, 0, 0)
    }

    /// Job 2: Agent Memory Bridge
    /// Reads AgentMemory from redb, formats summaries, writes to markdown files
    async fn run_agent_bridge_job(
        store: &Arc<RwLock<MarkdownMemoryStore>>,
        agent_store: &Arc<AgentStore>,
        _config: &MemoryConfig,
    ) -> Result<(), String> {
        // Query all agents
        let agents = agent_store
            .query_agents(neomind_storage::AgentFilter {
                status: None,
                schedule_type: None,
                start_time: None,
                end_time: None,
                limit: None,
                offset: None,
            })
            .await
            .map_err(|e| format!("Failed to query agents: {}", e))?;

        if agents.is_empty() {
            info!("No agents found, skipping agent bridge job");
            return Ok(());
        }

        let agents_dir = {
            let store_guard = store.read().await;
            store_guard.base_path().join("agents")
        };

        // Ensure agents directory exists
        fs::create_dir_all(&agents_dir)
            .map_err(|e| format!("Failed to create agents directory: {}", e))?;

        // Process each agent
        let mut processed = 0;
        let mut errors = 0;

        for agent in &agents {
            let agent_id = &agent.id;
            let agent_name = &agent.name;

            // Format agent summary
            let summary = format_agent_summary(&agent);

            // Write to file
            let file_path = agents_dir.join(format!("{}.md", agent_id));
            if let Err(e) = fs::write(&file_path, &summary) {
                errors += 1;
                warn!(
                    agent_id = %agent_id,
                    error = %e,
                    "Failed to write agent summary file"
                );
                continue;
            }

            processed += 1;
            info!(
                agent_id = %agent_id,
                agent_name = %agent_name,
                summary_len = summary.len(),
                "Agent summary written"
            );
        }

        // Cleanup old agent files (keep only MAX_AGENT_FILES)
        Self::cleanup_old_agent_files(&agents_dir, MAX_AGENT_FILES)
            .map_err(|e| format!("Failed to cleanup old agent files: {}", e))?;

        info!(
            total_agents = agents.len(),
            processed = processed,
            errors = errors,
            max_files = MAX_AGENT_FILES,
            "Agent bridge job completed"
        );

        Ok(())
    }

    /// Cleanup old agent files, keeping only the most recent ones
    pub fn cleanup_old_agent_files(agents_dir: &PathBuf, max_files: usize) -> std::io::Result<()> {
        let mut entries: Vec<_> = fs::read_dir(agents_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
            .collect();

        // Sort by modified time (most recent first)
        entries.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        entries.reverse();

        // Remove old files beyond max_files
        if entries.len() > max_files {
            for old_entry in entries.iter().skip(max_files) {
                if let Err(e) = fs::remove_file(old_entry.path()) {
                    warn!(
                        path = %old_entry.path().display(),
                        error = %e,
                        "Failed to remove old agent file"
                    );
                }
            }
        }

        Ok(())
    }

    /// Job 3: Temp File Cleanup
    /// Deletes session directories older than temp_file_ttl_days
    async fn run_temp_cleanup_job(
        store: &Arc<RwLock<MarkdownMemoryStore>>,
        config: &MemoryConfig,
    ) -> Result<(), String> {
        let sessions_dir = {
            let store_guard = store.read().await;
            store_guard.base_path().join("sessions")
        };

        if !sessions_dir.exists() {
            info!("Sessions directory does not exist, skipping cleanup");
            return Ok(());
        }

        let ttl_secs = config.temp_file_ttl_days * 86400;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get current time: {}", e))?
            .as_secs();

        let mut deleted_count = 0;
        let mut error_count = 0;

        let entries = fs::read_dir(&sessions_dir)
            .map_err(|e| format!("Failed to read sessions directory: {}", e))?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    error_count += 1;
                    warn!(error = %e, "Failed to read session directory entry");
                    continue;
                }
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let metadata = match fs::metadata(&path) {
                Ok(m) => m,
                Err(e) => {
                    error_count += 1;
                    warn!(path = %path.display(), error = %e, "Failed to get metadata");
                    continue;
                }
            };

            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .unwrap_or_default()
                .as_secs();

            let age_secs = now.saturating_sub(modified);

            if age_secs > ttl_secs {
                match fs::remove_dir_all(&path) {
                    Ok(_) => {
                        deleted_count += 1;
                        info!(path = %path.display(), age_days = age_secs / 86400, "Deleted old session directory");
                    }
                    Err(e) => {
                        error_count += 1;
                        warn!(path = %path.display(), error = %e, "Failed to delete session directory");
                    }
                }
            }
        }

        info!(
            deleted_count = deleted_count,
            error_count = error_count,
            ttl_days = config.temp_file_ttl_days,
            "Temp cleanup job completed"
        );

        Ok(())
    }

    /// Stop background jobs
    pub fn stop(&mut self) {
        if let Some(handle) = self.job_handle.take() {
            handle.abort();
            info!("Memory scheduler stopped");
        }
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.job_handle.is_some()
    }
}

impl Drop for MemoryScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Generate a system resource summary from live state.
fn generate_system_summary(
    devices: usize,
    rules: usize,
    extensions: usize,
    dashboards: usize,
) -> String {
    format!(
        "## System Resources\n\n- Devices: {} online\n- Rules: {} active\n- Extensions: {} installed\n- Dashboards: {} configured",
        devices, rules, extensions, dashboards
    )
}

/// Format an agent's memory into a markdown summary.
fn format_agent_summary(agent: &AiAgent) -> String {
    let mut lines = vec![
        format!("# Agent: {}", agent.name),
        String::new(),
        format!("**ID**: {}", agent.id),
        format!("**Status**: {:?}", agent.status),
        format!(
            "**Created**: {}",
            chrono::DateTime::from_timestamp(agent.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        ),
        format!(
            "**Last Updated**: {}",
            chrono::DateTime::from_timestamp(agent.updated_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        ),
        String::new(),
    ];

    // Description
    if let Some(ref desc) = agent.description {
        lines.push(format!("## Description\n{}", desc));
        lines.push(String::new());
    }

    // User prompt
    lines.push("## User Prompt".to_string());
    lines.push(agent.user_prompt.clone());
    lines.push(String::new());

    // Memory sections
    let memory = &agent.memory;

    // Working memory
    if memory.working.current_analysis.is_some()
        || memory.working.current_conclusion.is_some()
        || !memory.working.pending_decisions.is_empty()
        || !memory.working.temp_data.is_empty()
    {
        lines.push("## Working Memory".to_string());
        if let Some(ref analysis) = memory.working.current_analysis {
            lines.push(format!("### Current Analysis\n{}", analysis));
        }
        if let Some(ref conclusion) = memory.working.current_conclusion {
            lines.push(format!("### Current Conclusion\n{}", conclusion));
        }
        if !memory.working.temp_data.is_empty() {
            lines.push("### Temporary Data".to_string());
            for (key, value) in &memory.working.temp_data {
                lines.push(format!("- **{}**: {}", key, value));
            }
        }
        lines.push(String::new());
    }

    // Short-term memory
    if !memory.short_term.summaries.is_empty() {
        lines.push("## Short-term Memory".to_string());
        for summary in &memory.short_term.summaries {
            lines.push(format!(
                "- **Execution {}**: {}",
                summary.execution_id, summary.conclusion
            ));
        }
        lines.push(String::new());
    }

    // Long-term memory
    if !memory.long_term.patterns.is_empty() {
        lines.push("## Long-term Memory".to_string());
        lines.push("### Patterns".to_string());
        for pattern in &memory.long_term.patterns {
            lines.push(format!(
                "- **{}** (confidence: {})",
                pattern.description, pattern.confidence
            ));
        }
        lines.push(String::new());
    }

    // Execution stats
    lines.push("## Execution Statistics".to_string());
    lines.push(format!(
        "- Total executions: {}",
        agent.stats.total_executions
    ));
    lines.push(format!(
        "- Successful executions: {}",
        agent.stats.successful_executions
    ));
    lines.push(format!(
        "- Failed executions: {}",
        agent.stats.failed_executions
    ));
    lines.push(format!(
        "- Average duration: {}ms",
        agent.stats.avg_duration_ms
    ));
    lines.push(String::new());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_system_summary() {
        let summary = generate_system_summary(5, 10, 3, 2);
        assert!(summary.contains("Devices: 5 online"));
        assert!(summary.contains("Rules: 10 active"));
        assert!(summary.contains("Extensions: 3 installed"));
        assert!(summary.contains("Dashboards: 2 configured"));
    }

    #[test]
    fn test_scheduler_creation() {
        let temp = TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();

        let manager = Arc::new(RwLock::new(MemoryManager::new(config.clone())));
        let store = Arc::new(RwLock::new(MarkdownMemoryStore::new(temp.path())));
        let scheduler = MemoryScheduler::new(manager, store, config);

        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let temp = TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();
        config.schedule_interval_secs = 60; // Long enough not to trigger

        let manager = Arc::new(RwLock::new(MemoryManager::new(config.clone())));
        let store = Arc::new(RwLock::new(MarkdownMemoryStore::new(temp.path())));
        let mut scheduler = MemoryScheduler::new(manager, store, config);

        scheduler.start();
        assert!(scheduler.is_running());

        scheduler.stop();
        assert!(!scheduler.is_running());
    }

    #[test]
    fn test_disabled_scheduler() {
        let temp = TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();
        config.enabled = false;

        let manager = Arc::new(RwLock::new(MemoryManager::new(config.clone())));
        let store = Arc::new(RwLock::new(MarkdownMemoryStore::new(temp.path())));
        let mut scheduler = MemoryScheduler::new(manager, store, config);

        scheduler.start();
        assert!(!scheduler.is_running()); // Should not start when disabled
    }

    #[test]
    fn test_cleanup_old_agent_files() {
        let temp = TempDir::new().unwrap();
        let agents_dir = temp.path().join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        // Create 7 agent files
        for i in 0..7 {
            let path = agents_dir.join(format!("agent_{}.md", i));
            fs::write(&path, format!("Agent {}", i)).unwrap();
            // Sleep a bit to ensure different modification times
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Cleanup should leave only 5 files
        super::MemoryScheduler::cleanup_old_agent_files(&agents_dir, 5).unwrap();

        let entries: Vec<_> = fs::read_dir(&agents_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
            .collect();

        assert_eq!(entries.len(), 5);
    }
}
