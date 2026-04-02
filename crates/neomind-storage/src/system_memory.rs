//! System Memory - Markdown-based memory storage
//!
//! This module provides a simple Markdown file-based memory system for AI agents.
//! Based on 2026 research (Voxos.ai, Letta), simple file storage (74% accuracy)
//! outperforms complex graph/RAG systems (68.5%).
//!
//! ## Architecture
//!
//! ```text
//! data/memory/
//! ├── system.md              # System-level memory (global)
//! ├── agents/
//! │   ├── {agent_id}.md      # Per-agent memory
//! │   └── ...
//! └── chat/
//!     ├── {session_id}.md    # Chat session memory
//!     └── ...
//! ```
//!
//! ## Memory Entry Format
//!
//! ```markdown
//! ## Patterns
//! - 2026-04-01: User prefers evening lights off [importance: 80]
//! - 2026-04-01: Daily temperature check at 10am [importance: 60]
//!
//! ## Entities
//! - Device: Living Room Light (light_001)
//! - Location: Living Room, Bedroom
//!
//! ## Preferences
//! - Temperature unit: Celsius
//! - Language: Chinese
//!
//! ## Facts
//! - 2026-04-01: System uses Clean Architecture
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{Error, Result};

/// Maximum recommended memory entries per file (based on LLM instruction limits)
pub const MAX_MEMORY_ENTRIES: usize = 30;

/// Default importance threshold for pruning
pub const DEFAULT_MIN_IMPORTANCE: u8 = 30;

/// Memory category - four types for organized storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    /// 用户画像 - User preferences, habits, focus areas
    #[default]
    UserProfile,
    /// 领域知识 - Domain knowledge, devices, environment rules
    DomainKnowledge,
    /// 任务模式 - Task patterns, agent execution experience
    TaskPatterns,
    /// 系统进化 - System evolution, self-learning records
    SystemEvolution,
}

impl MemoryCategory {
    /// Get the markdown filename for this category
    pub fn filename(&self) -> &'static str {
        match self {
            Self::UserProfile => "user_profile.md",
            Self::DomainKnowledge => "domain_knowledge.md",
            Self::TaskPatterns => "task_patterns.md",
            Self::SystemEvolution => "system_evolution.md",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::UserProfile => "User Profile",
            Self::DomainKnowledge => "Domain Knowledge",
            Self::TaskPatterns => "Task Patterns",
            Self::SystemEvolution => "System Evolution",
        }
    }

    /// Get max entries for this category
    pub fn max_entries(&self) -> usize {
        match self {
            Self::UserProfile => 50,
            Self::DomainKnowledge => 100,
            Self::TaskPatterns => 80,
            Self::SystemEvolution => 30,
        }
    }

    /// Get the markdown section name (for backward compatibility)
    pub fn section_name(&self) -> &'static str {
        self.display_name()
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            // English names
            "user_profile" | "user profile" | "userprofile" => Some(Self::UserProfile),
            "domain_knowledge" | "domain knowledge" | "domainknowledge" => {
                Some(Self::DomainKnowledge)
            }
            "task_patterns" | "task patterns" | "taskpatterns" => Some(Self::TaskPatterns),
            "system_evolution" | "system evolution" | "systemevolution" => {
                Some(Self::SystemEvolution)
            }
            // Chinese names (backward compatibility)
            "用户画像" => Some(Self::UserProfile),
            "领域知识" => Some(Self::DomainKnowledge),
            "任务模式" => Some(Self::TaskPatterns),
            "系统进化" => Some(Self::SystemEvolution),
            // Legacy aliases for backward compatibility
            "pattern" | "patterns" => Some(Self::TaskPatterns),
            "entity" | "entities" => Some(Self::DomainKnowledge),
            "preference" | "preferences" => Some(Self::UserProfile),
            "fact" | "facts" => Some(Self::DomainKnowledge),
            _ => None,
        }
    }

    /// All categories
    pub fn all() -> &'static [Self] {
        &[
            Self::UserProfile,
            Self::DomainKnowledge,
            Self::TaskPatterns,
            Self::SystemEvolution,
        ]
    }
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self)
                .unwrap_or_default()
                .trim_matches('"')
        )
    }
}

/// Category statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryStats {
    /// Number of entries in the category
    pub entry_count: usize,
    /// File size in bytes
    pub file_size: u64,
    /// Last modified timestamp (Unix seconds)
    pub modified_at: i64,
}

/// Memory source - where the memory came from
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MemorySource {
    /// Memory from an AI Agent
    Agent { id: String, name: String },
    /// Memory from a chat session
    Chat { session_id: String },
    /// System-level global memory
    System,
}

impl MemorySource {
    /// Get the file path for this source
    pub fn file_path(&self, base_path: &Path) -> PathBuf {
        match self {
            MemorySource::Agent { id, .. } => base_path.join("agents").join(format!("{}.md", id)),
            MemorySource::Chat { session_id } => {
                base_path.join("chat").join(format!("{}.md", session_id))
            }
            MemorySource::System => base_path.join("system.md"),
        }
    }

    /// Get a display name for this source
    pub fn display_name(&self) -> String {
        match self {
            MemorySource::Agent { name, .. } => name.clone(),
            MemorySource::Chat { session_id } => {
                format!("Chat {}", &session_id[..8.min(session_id.len())])
            }
            MemorySource::System => "System".to_string(),
        }
    }
}

/// A single memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// The memory content
    pub content: String,
    /// Category of this memory
    pub category: MemoryCategory,
    /// Importance score (0-100)
    pub importance: u8,
    /// When this memory was created (Unix timestamp)
    pub created_at: i64,
    /// Source of this memory
    pub source: MemorySource,
}

impl MemoryEntry {
    /// Create a new memory entry
    pub fn new(content: impl Into<String>, category: MemoryCategory, source: MemorySource) -> Self {
        Self {
            content: content.into(),
            category,
            importance: 50,
            created_at: Utc::now().timestamp(),
            source,
        }
    }

    /// Set the importance
    pub fn with_importance(mut self, importance: u8) -> Self {
        self.importance = importance.min(100);
        self
    }

    /// Set the creation timestamp
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.created_at = timestamp;
        self
    }

    /// Format as markdown list item
    pub fn to_markdown(&self) -> String {
        let date = DateTime::from_timestamp(self.created_at, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        format!(
            "- {}: {} [importance: {}]",
            date, self.content, self.importance
        )
    }

    /// Parse from markdown line
    pub fn from_markdown(
        line: &str,
        category: MemoryCategory,
        source: MemorySource,
    ) -> Option<Self> {
        // Format: "- 2026-04-01: Content here [importance: 80]"
        let line = line.trim();
        if !line.starts_with('-') {
            return None;
        }

        let line = line[1..].trim();

        // Extract importance
        let (content, importance) = if let Some(idx) = line.rfind("[importance:") {
            let content_part = line[..idx].trim();
            let importance_part = &line[idx..];
            // Extract number from "[importance: 80]"
            let importance = importance_part
                .strip_prefix("[importance:")
                .and_then(|s| s.strip_suffix(']'))
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(50);
            (content_part, importance)
        } else {
            (line, 50)
        };

        // Extract date and content
        let content = if let Some(colon_idx) = content.find(':') {
            content[colon_idx + 1..].trim().to_string()
        } else {
            content.to_string()
        };

        if content.is_empty() {
            return None;
        }

        Some(Self {
            content,
            category,
            importance,
            created_at: Utc::now().timestamp(),
            source,
        })
    }
}

/// Aggregated memory result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMemory {
    /// All memory entries
    pub entries: Vec<MemoryEntry>,
    /// Total count
    pub total: usize,
    /// Count by category
    pub by_category: HashMap<String, usize>,
    /// Count by source
    pub by_source: HashMap<String, usize>,
}

impl Default for AggregatedMemory {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            total: 0,
            by_category: HashMap::new(),
            by_source: HashMap::new(),
        }
    }
}

/// Metadata for a memory file (for UI display)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFileInfo {
    /// File identifier (agent_id, session_id, or "system")
    pub id: String,
    /// Display name
    pub name: String,
    /// Source type: "agent", "chat", or "system"
    pub source_type: String,
    /// File size in bytes
    pub size: u64,
    /// Last modified timestamp (Unix seconds)
    pub modified_at: i64,
    /// Number of entries in the file
    pub entry_count: usize,
}

/// Markdown-based memory store
#[derive(Debug, Clone)]
pub struct MarkdownMemoryStore {
    /// Base path for memory files
    base_path: PathBuf,
    /// In-memory cache
    cache: Arc<RwLock<HashMap<String, Vec<MemoryEntry>>>>,
}

impl MarkdownMemoryStore {
    /// Create a new memory store
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the directory structure
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.base_path)?;

        // Create category files if they don't exist
        for category in MemoryCategory::all() {
            let path = self.category_path(category);
            if !path.exists() {
                let content = self.default_category_content(category);
                fs::write(&path, content)?;
                info!(path = %path.display(), "Created category memory file");
            }
        }

        // Create legacy directories for backward compatibility
        fs::create_dir_all(self.base_path.join("agents"))?;
        fs::create_dir_all(self.base_path.join("chat"))?;

        // Create system.md if it doesn't exist (legacy)
        let system_path = self.base_path.join("system.md");
        if !system_path.exists() {
            let content = "# System Memory\n\n## User Profile\n\n## Domain Knowledge\n\n## Task Patterns\n\n## System Evolution\n";
            fs::write(&system_path, content)?;
            info!(path = %system_path.display(), "Created system memory file");
        }

        Ok(())
    }

    // ========================================================================
    // Category-based API (simplified for new memory system)
    // ========================================================================

    /// Get the file path for a category
    pub fn category_path(&self, category: &MemoryCategory) -> PathBuf {
        self.base_path.join(category.filename())
    }

    /// Read markdown content for a category
    pub fn read_category(&self, category: &MemoryCategory) -> Result<String> {
        let path = self.category_path(category);
        if !path.exists() {
            return Ok(self.default_category_content(category));
        }
        fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read {:?}: {}", category, e)))
    }

    /// Write markdown content for a category
    pub fn write_category(&self, category: &MemoryCategory, content: &str) -> Result<()> {
        let path = self.category_path(category);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, content)
            .map_err(|e| Error::Storage(format!("Failed to write {:?}: {}", category, e)))?;

        info!(category = ?category, size = content.len(), "Wrote category memory file");
        Ok(())
    }

    /// Get statistics for a category
    pub fn category_stats(&self, category: &MemoryCategory) -> Result<CategoryStats> {
        let path = self.category_path(category);

        let content = self.read_category(category)?;
        let entry_count = content
            .lines()
            .filter(|l| l.trim().starts_with('-'))
            .count();

        let (file_size, modified_at) = if path.exists() {
            let metadata = fs::metadata(&path)?;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            (metadata.len(), modified)
        } else {
            (content.len() as u64, 0)
        };

        Ok(CategoryStats {
            entry_count,
            file_size,
            modified_at,
        })
    }

    /// Get statistics for all categories
    pub fn all_stats(&self) -> Result<HashMap<String, CategoryStats>> {
        let mut stats = HashMap::new();
        for category in MemoryCategory::all() {
            let key = category.to_string();
            stats.insert(key, self.category_stats(category)?);
        }
        Ok(stats)
    }

    /// Export all categories as a single markdown string
    pub fn export_all(&self) -> Result<String> {
        let mut output = String::new();
        output.push_str("# NeoMind Memory Export\n\n");
        output.push_str(&format!(
            "Generated: {}\n\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        for category in MemoryCategory::all() {
            let content = self.read_category(category)?;
            output.push_str(&format!("---\n\n# {}\n\n", category.display_name()));
            output.push_str(&content);
            output.push_str("\n\n");
        }

        Ok(output)
    }

    /// Generate default content for a category file
    fn default_category_content(&self, category: &MemoryCategory) -> String {
        format!(
            "# {}\n\n> Last updated: {}\n> Total entries: 0\n\n",
            category.display_name(),
            Utc::now().format("%Y-%m-%d %H:%M")
        )
    }

    /// Get the base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Read memory entries from a source
    pub fn read(&self, source: &MemorySource) -> Result<Vec<MemoryEntry>> {
        // Check cache first
        let cache_key = self.cache_key(source);
        {
            let cache = self.cache.read().unwrap();
            if let Some(entries) = cache.get(&cache_key) {
                return Ok(entries.clone());
            }
        }

        let file_path = source.file_path(&self.base_path);
        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| Error::Storage(format!("Failed to read memory file: {}", e)))?;

        let entries = self.parse_markdown(&content, source);

        // Update cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(cache_key, entries.clone());
        }

        Ok(entries)
    }

    /// Append a memory entry
    pub fn append(&self, source: &MemorySource, entry: &MemoryEntry) -> Result<()> {
        let file_path = source.file_path(&self.base_path);

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read existing content or create new
        let mut content = if file_path.exists() {
            fs::read_to_string(&file_path)
                .map_err(|e| Error::Storage(format!("Failed to read memory file: {}", e)))?
        } else {
            self.create_empty_markdown(source)
        };

        // Find the section and append
        let section_name = entry.category.section_name();
        let section_header = format!("## {}", section_name);

        // Find the section and add entry
        if let Some(section_start) = content.find(&section_header) {
            // Find next section or end of file
            let search_start = section_start + section_header.len();
            let next_section = content[search_start..]
                .find("\n## ")
                .map(|i| search_start + i)
                .unwrap_or(content.len());

            // Insert the new entry before the next section
            let insert_pos = next_section;
            let entry_text = format!("\n{}\n", entry.to_markdown());
            content.insert_str(insert_pos, &entry_text);
        } else {
            // Section doesn't exist, append it
            content.push_str(&format!(
                "\n## {}\n\n{}\n",
                section_name,
                entry.to_markdown()
            ));
        }

        // Write back
        fs::write(&file_path, content)?;

        // Invalidate cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(&self.cache_key(source));
        }

        debug!(
            source = %source.display_name(),
            category = ?entry.category,
            content = %entry.content,
            "Appended memory entry"
        );

        Ok(())
    }

    /// Append multiple entries
    pub fn append_batch(&self, source: &MemorySource, entries: &[MemoryEntry]) -> Result<()> {
        for entry in entries {
            self.append(source, entry)?;
        }
        Ok(())
    }

    /// Write complete memory file (replaces existing)
    pub fn write(&self, source: &MemorySource, entries: &[MemoryEntry]) -> Result<()> {
        let file_path = source.file_path(&self.base_path);

        // Group by category
        let mut by_category: HashMap<MemoryCategory, Vec<&MemoryEntry>> = HashMap::new();
        for entry in entries {
            by_category.entry(entry.category).or_default().push(entry);
        }

        // Build markdown content
        let mut content = match source {
            MemorySource::Agent { name, .. } => format!("# {} Memory\n\n", name),
            MemorySource::Chat { session_id } => format!("# Chat {} Memory\n\n", session_id),
            MemorySource::System => "# System Memory\n\n".to_string(),
        };

        for category in MemoryCategory::all() {
            content.push_str(&format!("## {}\n\n", category.section_name()));
            if let Some(cat_entries) = by_category.get(category) {
                for entry in cat_entries {
                    content.push_str(&entry.to_markdown());
                    content.push('\n');
                }
            }
            content.push('\n');
        }

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&file_path, content)?;

        // Invalidate cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(&self.cache_key(source));
        }

        info!(
            source = %source.display_name(),
            count = entries.len(),
            "Wrote memory file"
        );

        Ok(())
    }

    /// Aggregate all memory from all sources
    pub fn aggregate_all(&self) -> Result<AggregatedMemory> {
        let mut result = AggregatedMemory::default();

        // Read system memory
        let system_entries = self.read(&MemorySource::System)?;
        result.entries.extend(system_entries.clone());

        // Read agent memories
        let agents_path = self.base_path.join("agents");
        if agents_path.exists() {
            for entry in fs::read_dir(&agents_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Some(agent_id) = path.file_stem().and_then(|s| s.to_str()) {
                        let source = MemorySource::Agent {
                            id: agent_id.to_string(),
                            name: agent_id.to_string(),
                        };
                        if let Ok(entries) = self.read(&source) {
                            result.entries.extend(entries);
                        }
                    }
                }
            }
        }

        // Read chat memories
        let chat_path = self.base_path.join("chat");
        if chat_path.exists() {
            for entry in fs::read_dir(&chat_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Some(session_id) = path.file_stem().and_then(|s| s.to_str()) {
                        let source = MemorySource::Chat {
                            session_id: session_id.to_string(),
                        };
                        if let Ok(entries) = self.read(&source) {
                            result.entries.extend(entries);
                        }
                    }
                }
            }
        }

        // Calculate stats
        result.total = result.entries.len();
        for entry in &result.entries {
            *result
                .by_category
                .entry(entry.category.to_string().to_lowercase())
                .or_default() += 1;
            *result
                .by_source
                .entry(entry.source.display_name())
                .or_default() += 1;
        }

        Ok(result)
    }

    /// Search memory entries (simple text matching)
    pub fn search(&self, query: &str) -> Result<Vec<MemoryEntry>> {
        let all = self.aggregate_all()?;
        let query_lower = query.to_lowercase();

        let mut matches: Vec<MemoryEntry> = all
            .entries
            .into_iter()
            .filter(|e| e.content.to_lowercase().contains(&query_lower))
            .collect();

        // Sort by importance (descending)
        matches.sort_by(|a, b| b.importance.cmp(&a.importance));

        Ok(matches)
    }

    /// Prune memory to max entries, keeping highest importance
    pub fn prune(&self, source: &MemorySource, max_items: usize) -> Result<usize> {
        let mut entries = self.read(source)?;

        if entries.len() <= max_items {
            return Ok(0);
        }

        // Sort by importance (descending)
        entries.sort_by(|a, b| b.importance.cmp(&a.importance));

        let removed = entries.len() - max_items;
        entries.truncate(max_items);

        // Rewrite file
        self.write(source, &entries)?;

        info!(
            source = %source.display_name(),
            removed = removed,
            remaining = entries.len(),
            "Pruned memory entries"
        );

        Ok(removed)
    }

    /// Clear all memory for a source
    pub fn clear(&self, source: &MemorySource) -> Result<()> {
        let file_path = source.file_path(&self.base_path);

        if file_path.exists() {
            // Write empty file with headers
            let content = self.create_empty_markdown(source);
            fs::write(&file_path, content)?;
        }

        // Invalidate cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(&self.cache_key(source));
        }

        Ok(())
    }

    /// Export all memory as a single markdown string
    pub fn export_markdown(&self) -> Result<String> {
        let all = self.aggregate_all()?;

        let mut output = String::new();
        output.push_str("# NeoMind Memory Export\n\n");
        output.push_str(&format!(
            "Generated: {}\n\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));
        output.push_str(&format!("Total entries: {}\n\n", all.total));

        // Group by category
        for category in MemoryCategory::all() {
            let cat_entries: Vec<_> = all
                .entries
                .iter()
                .filter(|e| e.category == *category)
                .collect();

            if !cat_entries.is_empty() {
                output.push_str(&format!("## {}\n\n", category.section_name()));
                for entry in cat_entries {
                    output.push_str(&format!(
                        "- **{}**: {} `[importance: {}]`\n",
                        entry.source.display_name(),
                        entry.content,
                        entry.importance
                    ));
                }
                output.push('\n');
            }
        }

        Ok(output)
    }

    // ========================================================================
    // File-based API (for UI display)
    // ========================================================================

    /// List all memory files
    pub fn list_files(&self) -> Result<Vec<MemoryFileInfo>> {
        let mut files = Vec::new();

        // System memory
        let system_path = self.base_path.join("system.md");
        if system_path.exists() {
            if let Some(info) = self.get_file_info(&system_path, "system", "System", "system")? {
                files.push(info);
            }
        }

        // Agent memories
        let agents_path = self.base_path.join("agents");
        if agents_path.exists() {
            for entry in fs::read_dir(&agents_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Some(agent_id) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Some(info) = self.get_file_info(
                            &path, agent_id,
                            agent_id, // Will be replaced with actual agent name if available
                            "agent",
                        )? {
                            files.push(info);
                        }
                    }
                }
            }
        }

        // Chat memories
        let chat_path = self.base_path.join("chat");
        if chat_path.exists() {
            for entry in fs::read_dir(&chat_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Some(session_id) = path.file_stem().and_then(|s| s.to_str()) {
                        let display_name =
                            format!("Chat {}", &session_id[..8.min(session_id.len())]);
                        if let Some(info) =
                            self.get_file_info(&path, session_id, &display_name, "chat")?
                        {
                            files.push(info);
                        }
                    }
                }
            }
        }

        // Sort by modified time, newest first
        files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

        Ok(files)
    }

    /// Get file info helper
    fn get_file_info(
        &self,
        path: &Path,
        id: &str,
        name: &str,
        source_type: &str,
    ) -> Result<Option<MemoryFileInfo>> {
        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let content = fs::read_to_string(path).unwrap_or_default();
        let entry_count = content.lines().filter(|l| l.starts_with('-')).count();

        Ok(Some(MemoryFileInfo {
            id: id.to_string(),
            name: name.to_string(),
            source_type: source_type.to_string(),
            size: metadata.len(),
            modified_at,
            entry_count,
        }))
    }

    /// Read raw markdown content from a memory file
    pub fn read_raw_markdown(&self, source_type: &str, id: &str) -> Result<String> {
        let path = match source_type {
            "agent" => self.base_path.join("agents").join(format!("{}.md", id)),
            "chat" => self.base_path.join("chat").join(format!("{}.md", id)),
            "system" | _ => self.base_path.join("system.md"),
        };

        if !path.exists() {
            return Err(Error::Storage(format!("Memory file not found: {:?}", path)));
        }

        fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read memory file: {}", e)))
    }

    /// Update raw markdown content for a memory file
    pub fn write_raw_markdown(&self, source_type: &str, id: &str, content: &str) -> Result<()> {
        let path = match source_type {
            "agent" => self.base_path.join("agents").join(format!("{}.md", id)),
            "chat" => self.base_path.join("chat").join(format!("{}.md", id)),
            "system" | _ => self.base_path.join("system.md"),
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, content)?;

        // Invalidate cache
        let source = match source_type {
            "agent" => MemorySource::Agent {
                id: id.to_string(),
                name: id.to_string(),
            },
            "chat" => MemorySource::Chat {
                session_id: id.to_string(),
            },
            _ => MemorySource::System,
        };
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(&self.cache_key(&source));
        }

        Ok(())
    }

    /// Delete a memory file
    pub fn delete_file(&self, source_type: &str, id: &str) -> Result<()> {
        let path = match source_type {
            "agent" => self.base_path.join("agents").join(format!("{}.md", id)),
            "chat" => self.base_path.join("chat").join(format!("{}.md", id)),
            "system" | _ => return Err(Error::Storage("Cannot delete system memory".to_string())),
        };

        if !path.exists() {
            return Err(Error::Storage(format!("Memory file not found: {:?}", path)));
        }

        fs::remove_file(&path)?;

        // Invalidate cache
        let source = match source_type {
            "agent" => MemorySource::Agent {
                id: id.to_string(),
                name: id.to_string(),
            },
            "chat" => MemorySource::Chat {
                session_id: id.to_string(),
            },
            _ => MemorySource::System,
        };
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(&self.cache_key(&source));
        }

        Ok(())
    }

    // Helper methods

    fn cache_key(&self, source: &MemorySource) -> String {
        match source {
            MemorySource::Agent { id, .. } => format!("agent:{}", id),
            MemorySource::Chat { session_id } => format!("chat:{}", session_id),
            MemorySource::System => "system".to_string(),
        }
    }

    fn create_empty_markdown(&self, source: &MemorySource) -> String {
        match source {
            MemorySource::Agent { name, .. } => format!(
                "# {} Memory\n\n## User Profile\n\n## Domain Knowledge\n\n## Task Patterns\n\n## System Evolution\n",
                name
            ),
            MemorySource::Chat { session_id } => format!(
                "# Chat {} Memory\n\n## User Profile\n\n## Domain Knowledge\n\n## Task Patterns\n\n## System Evolution\n",
                session_id
            ),
            MemorySource::System => {
                "# System Memory\n\n## User Profile\n\n## Domain Knowledge\n\n## Task Patterns\n\n## System Evolution\n"
                    .to_string()
            }
        }
    }

    fn parse_markdown(&self, content: &str, source: &MemorySource) -> Vec<MemoryEntry> {
        let mut entries = Vec::new();
        let mut current_category: Option<MemoryCategory> = None;

        for line in content.lines() {
            let line = line.trim();

            // Check for section headers
            if line.starts_with("## ") {
                let section = line[3..].trim();
                current_category = MemoryCategory::from_str(section);
                continue;
            }

            // Parse list items
            if line.starts_with('-') {
                if let Some(category) = current_category {
                    if let Some(entry) = MemoryEntry::from_markdown(line, category, source.clone())
                    {
                        entries.push(entry);
                    }
                }
            }
        }

        entries
    }
}

impl Default for MarkdownMemoryStore {
    fn default() -> Self {
        Self::new("data/memory")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new(
            "User prefers dark mode",
            MemoryCategory::UserProfile,
            MemorySource::System,
        )
        .with_importance(80);

        assert_eq!(entry.content, "User prefers dark mode");
        assert_eq!(entry.category, MemoryCategory::UserProfile);
        assert_eq!(entry.importance, 80);
    }

    #[test]
    fn test_memory_entry_markdown() {
        let entry = MemoryEntry::new(
            "Test content",
            MemoryCategory::TaskPatterns,
            MemorySource::System,
        )
        .with_importance(75);

        let md = entry.to_markdown();
        assert!(md.contains("Test content"));
        assert!(md.contains("75"));
    }

    #[test]
    fn test_memory_category_parsing() {
        // New names
        assert_eq!(
            MemoryCategory::from_str("user_profile"),
            Some(MemoryCategory::UserProfile)
        );
        assert_eq!(
            MemoryCategory::from_str("domain_knowledge"),
            Some(MemoryCategory::DomainKnowledge)
        );
        // Legacy aliases
        assert_eq!(
            MemoryCategory::from_str("pattern"),
            Some(MemoryCategory::TaskPatterns)
        );
        assert_eq!(
            MemoryCategory::from_str("PREFERENCES"),
            Some(MemoryCategory::UserProfile)
        );
        assert_eq!(MemoryCategory::from_str("invalid"), None);
    }

    #[test]
    fn test_new_memory_categories() {
        assert_eq!(MemoryCategory::UserProfile.filename(), "user_profile.md");
        assert_eq!(
            MemoryCategory::DomainKnowledge.filename(),
            "domain_knowledge.md"
        );
        assert_eq!(MemoryCategory::TaskPatterns.filename(), "task_patterns.md");
        assert_eq!(
            MemoryCategory::SystemEvolution.filename(),
            "system_evolution.md"
        );

        assert_eq!(MemoryCategory::UserProfile.max_entries(), 50);
        assert_eq!(MemoryCategory::DomainKnowledge.max_entries(), 100);
        assert_eq!(MemoryCategory::TaskPatterns.max_entries(), 80);
        assert_eq!(MemoryCategory::SystemEvolution.max_entries(), 30);

        assert_eq!(MemoryCategory::all().len(), 4);
    }

    #[test]
    fn test_memory_store_basic() {
        let temp_dir = TempDir::new().unwrap();
        let store = MarkdownMemoryStore::new(temp_dir.path());
        store.init().unwrap();

        let entry = MemoryEntry::new(
            "Test pattern",
            MemoryCategory::TaskPatterns,
            MemorySource::System,
        );

        store.append(&MemorySource::System, &entry).unwrap();

        let entries = store.read(&MemorySource::System).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "Test pattern");
    }

    #[test]
    fn test_memory_store_aggregation() {
        let temp_dir = TempDir::new().unwrap();
        let store = MarkdownMemoryStore::new(temp_dir.path());
        store.init().unwrap();

        // Add system memory
        store
            .append(
                &MemorySource::System,
                &MemoryEntry::new(
                    "Global fact",
                    MemoryCategory::DomainKnowledge,
                    MemorySource::System,
                ),
            )
            .unwrap();

        // Add agent memory
        let agent_source = MemorySource::Agent {
            id: "agent_1".to_string(),
            name: "TestAgent".to_string(),
        };
        store
            .append(
                &agent_source,
                &MemoryEntry::new(
                    "Agent pattern",
                    MemoryCategory::TaskPatterns,
                    agent_source.clone(),
                ),
            )
            .unwrap();

        let all = store.aggregate_all().unwrap();
        assert_eq!(all.total, 2);
        assert_eq!(all.by_category.get("domain_knowledge"), Some(&1));
        assert_eq!(all.by_category.get("task_patterns"), Some(&1));
    }

    #[test]
    fn test_memory_search() {
        let temp_dir = TempDir::new().unwrap();
        let store = MarkdownMemoryStore::new(temp_dir.path());
        store.init().unwrap();

        store
            .append(
                &MemorySource::System,
                &MemoryEntry::new(
                    "User likes Python",
                    MemoryCategory::UserProfile,
                    MemorySource::System,
                ),
            )
            .unwrap();
        store
            .append(
                &MemorySource::System,
                &MemoryEntry::new(
                    "Daily backup at midnight",
                    MemoryCategory::TaskPatterns,
                    MemorySource::System,
                ),
            )
            .unwrap();

        let results = store.search("python").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Python"));
    }

    #[test]
    fn test_memory_prune() {
        let temp_dir = TempDir::new().unwrap();
        let store = MarkdownMemoryStore::new(temp_dir.path());
        store.init().unwrap();

        // Add 5 entries with different importance
        for i in 1..=5 {
            store
                .append(
                    &MemorySource::System,
                    &MemoryEntry::new(
                        format!("Entry {}", i),
                        MemoryCategory::DomainKnowledge,
                        MemorySource::System,
                    )
                    .with_importance(i * 10),
                )
                .unwrap();
        }

        // Prune to 3
        let removed = store.prune(&MemorySource::System, 3).unwrap();
        assert_eq!(removed, 2);

        let remaining = store.read(&MemorySource::System).unwrap();
        assert_eq!(remaining.len(), 3);

        // Should keep highest importance
        let importances: Vec<u8> = remaining.iter().map(|e| e.importance).collect();
        assert!(importances.contains(&50));
        assert!(importances.contains(&40));
        assert!(importances.contains(&30));
    }

    #[test]
    fn test_category_operations() {
        let temp_dir = TempDir::new().unwrap();
        let store = MarkdownMemoryStore::new(temp_dir.path());
        store.init().unwrap();

        // Write to a category
        let content = "# User Profile\n\n## Preferences\n\n- Test preference\n";
        store
            .write_category(&MemoryCategory::UserProfile, content)
            .unwrap();

        // Read it back
        let read = store.read_category(&MemoryCategory::UserProfile).unwrap();
        assert!(read.contains("Test preference"));

        // Check stats
        let stats = store.category_stats(&MemoryCategory::UserProfile).unwrap();
        assert!(stats.file_size > 0);
        assert_eq!(stats.entry_count, 1); // One line starting with '-'

        // Check all_stats
        let all_stats = store.all_stats().unwrap();
        assert!(all_stats.contains_key("user_profile"));
    }

    #[test]
    fn test_export_all() {
        let temp_dir = TempDir::new().unwrap();
        let store = MarkdownMemoryStore::new(temp_dir.path());
        store.init().unwrap();

        // Write to multiple categories
        store
            .write_category(
                &MemoryCategory::UserProfile,
                "# User Profile\n\n- Preference 1\n",
            )
            .unwrap();
        store
            .write_category(
                &MemoryCategory::DomainKnowledge,
                "# Domain Knowledge\n\n- Knowledge 1\n",
            )
            .unwrap();

        // Export all
        let export = store.export_all().unwrap();
        assert!(export.contains("User Profile"));
        assert!(export.contains("Domain Knowledge"));
        assert!(export.contains("NeoMind Memory Export"));
    }
}
