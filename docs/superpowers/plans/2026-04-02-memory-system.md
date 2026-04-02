# Memory System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 重构记忆系统，支持四类记忆（用户画像、领域知识、任务模式、系统进化），使用 LLM 进行提取和压缩。

**Architecture:** 保留现有 Markdown 文件存储，重构为按类型分类的四个文件。新增 MemoryManager 统一管理提取和压缩，复用现有 LlmBackendStore 调用 LLM。

**Tech Stack:** Rust (Axum, tokio), React + TypeScript, Markdown

---

## Dependency Graph

```
Task 1 (MemoryCategory + Config)
    │
    ├─→ Task 2 (Store Operations)
    │       │
    │       └─→ Task 3 (MemoryManager)
    │
    ├─→ Task 4 (Extractor Types)
    │
    └─→ Task 5 (Compressor Types)
            │
            └─→ Task 6 (LLM Integration)
                    │
                    ├─→ Task 7 (Scheduler)
                    │
                    └─→ Task 8 (API Handlers)
                            │
                            ├─→ Task 9 (Router + State)
                            │
                            └─→ Task 10 (Frontend)
                                    │
                                    └─→ Task 11 (Integration)
```

---

## File Structure

```
crates/neomind-storage/src/
├── system_memory.rs          # 修改：重构为四类记忆 + CategoryStats
├── memory_config.rs          # 新增：配置结构

crates/neomind-agent/src/
├── memory/                   # 新增目录
│   ├── mod.rs
│   ├── manager.rs            # MemoryManager
│   ├── extractor.rs          # 提取器（类型 + LLM 调用）
│   ├── compressor.rs         # 压缩器（类型 + LLM 调用）
│   ├── dedup.rs              # 去重逻辑
│   └── scheduler.rs          # 定时任务
├── lib.rs                    # 修改：导出 memory 模块
├── memory_extraction.rs      # 删除：合并到新 memory 模块

crates/neomind-api/src/
├── handlers/
│   ├── memory.rs             # 修改：适配新 API
│   └── mod.rs                # 修改：导出
├── server/
│   ├── router.rs             # 修改：路由
│   └── state/agent_state.rs  # 修改：添加 MemoryManager

web/src/
├── lib/api.ts                # 修改：更新 API
├── pages/agents-components/
│   ├── MemoryPanel.tsx       # 新增：主面板
│   └── SystemMemoryPanel.tsx # 删除：替换为 MemoryPanel

data/memory/
├── user_profile.md           # 用户画像
├── domain_knowledge.md       # 领域知识
├── task_patterns.md          # 任务模式
└── system_evolution.md       # 系统进化
```

---

## Task 1: Storage - MemoryCategory & CategoryStats

**Prerequisites:** None

**Files:**
- Modify: `crates/neomind-storage/src/system_memory.rs`

- [ ] **Step 1: Write failing test for new MemoryCategory**

```rust
// crates/neomind-storage/src/system_memory.rs (add to tests module)

#[test]
fn test_new_memory_categories() {
    use super::*;

    assert_eq!(MemoryCategory::UserProfile.filename(), "user_profile.md");
    assert_eq!(MemoryCategory::DomainKnowledge.filename(), "domain_knowledge.md");
    assert_eq!(MemoryCategory::TaskPatterns.filename(), "task_patterns.md");
    assert_eq!(MemoryCategory::SystemEvolution.filename(), "system_evolution.md");

    assert_eq!(MemoryCategory::UserProfile.max_entries(), 50);
    assert_eq!(MemoryCategory::DomainKnowledge.max_entries(), 100);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p neomind-storage test_new_memory_categories 2>&1`
Expected: FAIL - MemoryCategory variants don't exist

- [ ] **Step 3: Refactor MemoryCategory enum**

Replace the existing `MemoryCategory` enum in `system_memory.rs`:

```rust
/// Memory category - four types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    #[default]
    UserProfile,
    DomainKnowledge,
    TaskPatterns,
    SystemEvolution,
}

impl MemoryCategory {
    pub fn filename(&self) -> &'static str {
        match self {
            Self::UserProfile => "user_profile.md",
            Self::DomainKnowledge => "domain_knowledge.md",
            Self::TaskPatterns => "task_patterns.md",
            Self::SystemEvolution => "system_evolution.md",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::UserProfile => "用户画像",
            Self::DomainKnowledge => "领域知识",
            Self::TaskPatterns => "任务模式",
            Self::SystemEvolution => "系统进化",
        }
    }

    pub fn max_entries(&self) -> usize {
        match self {
            Self::UserProfile => 50,
            Self::DomainKnowledge => 100,
            Self::TaskPatterns => 80,
            Self::SystemEvolution => 30,
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::UserProfile, Self::DomainKnowledge, Self::TaskPatterns, Self::SystemEvolution]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "user_profile" | "用户画像" => Some(Self::UserProfile),
            "domain_knowledge" | "领域知识" => Some(Self::DomainKnowledge),
            "task_patterns" | "任务模式" => Some(Self::TaskPatterns),
            "system_evolution" | "系统进化" => Some(Self::SystemEvolution),
            _ => None,
        }
    }
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap_or_default().trim_matches('"'))
    }
}

/// Category statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryStats {
    pub entry_count: usize,
    pub file_size: u64,
    pub modified_at: i64,
}
```

- [ ] **Step 4: Update exports in system_memory.rs**

Remove old `MemorySource` enum. Update the exports at bottom of file:
```rust
// Remove MemorySource export, keep:
pub use {MarkdownMemoryStore, MemoryCategory, CategoryStats, MemoryEntry, MemoryFileInfo, ...};
```

- [ ] **Step 5: Run tests to verify**

Run: `cargo test -p neomind-storage memory_category 2>&1`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/neomind-storage/src/system_memory.rs
git commit -m "refactor(storage): simplify MemoryCategory to four types

- UserProfile, DomainKnowledge, TaskPatterns, SystemEvolution
- Add CategoryStats struct
- Remove old MemorySource enum"
```

---

## Task 2: Storage - MemoryConfig

**Prerequisites:** Task 1

**Files:**
- Create: `crates/neomind-storage/src/memory_config.rs`
- Modify: `crates/neomind-storage/src/lib.rs`

- [ ] **Step 1: Create memory_config.rs**

```rust
// crates/neomind-storage/src/memory_config.rs

//! Memory system configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_storage_path")]
    pub storage_path: String,

    #[serde(default)]
    pub extraction: ExtractionConfig,

    #[serde(default)]
    pub compression: CompressionConfig,

    #[serde(default)]
    pub llm: MemoryLlmConfig,

    #[serde(default)]
    pub schedule: ScheduleConfig,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_path: "data/memory".to_string(),
            extraction: ExtractionConfig::default(),
            compression: CompressionConfig::default(),
            llm: MemoryLlmConfig::default(),
            schedule: ScheduleConfig::default(),
        }
    }
}

fn default_enabled() -> bool { true }
fn default_storage_path() -> String { "data/memory".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self { similarity_threshold: 0.85 }
    }
}

fn default_similarity_threshold() -> f32 { 0.85 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    #[serde(default = "default_decay_days")]
    pub decay_period_days: u8,

    #[serde(default = "default_min_importance")]
    pub min_importance: u8,

    #[serde(default = "default_max_entries")]
    pub max_entries: HashMap<String, usize>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        let mut max_entries = HashMap::new();
        max_entries.insert("user_profile".to_string(), 50);
        max_entries.insert("domain_knowledge".to_string(), 100);
        max_entries.insert("task_patterns".to_string(), 80);
        max_entries.insert("system_evolution".to_string(), 30);

        Self {
            decay_period_days: 30,
            min_importance: 20,
            max_entries,
        }
    }
}

fn default_decay_days() -> u8 { 30 }
fn default_min_importance() -> u8 { 20 }
fn default_max_entries() -> HashMap<String, usize> {
    CompressionConfig::default().max_entries
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryLlmConfig {
    pub extraction_backend_id: Option<String>,
    pub compression_backend_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    #[serde(default = "default_true")]
    pub extraction_enabled: bool,

    #[serde(default = "default_extraction_interval")]
    pub extraction_interval_secs: u64,

    #[serde(default = "default_true")]
    pub compression_enabled: bool,

    #[serde(default = "default_compression_interval")]
    pub compression_interval_secs: u64,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            extraction_enabled: true,
            extraction_interval_secs: 3600,
            compression_enabled: true,
            compression_interval_secs: 86400,
        }
    }
}

fn default_true() -> bool { true }
fn default_extraction_interval() -> u64 { 3600 }
fn default_compression_interval() -> u64 { 86400 }

impl MemoryConfig {
    pub const CONFIG_FILE: &'static str = "data/memory_config.json";

    pub fn load() -> Self {
        let path = Self::CONFIG_FILE;
        if !std::path::Path::new(path).exists() {
            return Self::default();
        }
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::CONFIG_FILE, content)
    }
}
```

- [ ] **Step 2: Update lib.rs to export memory_config**

Add module declaration after `pub mod system_memory;`:
```rust
pub mod memory_config;
```

Add to exports after the `system_memory` exports:
```rust
pub use memory_config::{
    CompressionConfig, ExtractionConfig, MemoryLlmConfig, MemoryConfig,
    ScheduleConfig,
};
```

- [ ] **Step 3: Run build to verify**

Run: `cargo build -p neomind-storage 2>&1 | tail -10`
Expected: Build succeeds

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-storage/src/memory_config.rs crates/neomind-storage/src/lib.rs
git commit -m "feat(storage): add MemoryConfig with extraction/compression settings"
```

---

## Task 3: Storage - Simplified Store Operations

**Prerequisites:** Task 1, Task 2

**Files:**
- Modify: `crates/neomind-storage/src/system_memory.rs`

- [ ] **Step 1: Write test for store operations**

```rust
// Add to tests in system_memory.rs

#[test]
fn test_store_category_operations() {
    let temp = tempfile::TempDir::new().unwrap();
    let store = MarkdownMemoryStore::new(temp.path());

    store.init().unwrap();

    let content = "# 用户画像\n\n## 偏好\n- 测试\n";
    store.write_category(&MemoryCategory::UserProfile, content).unwrap();

    let read = store.read_category(&MemoryCategory::UserProfile).unwrap();
    assert!(read.contains("测试"));

    let stats = store.category_stats(&MemoryCategory::UserProfile).unwrap();
    assert_eq!(stats.file_size, content.len() as u64);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p neomind-storage test_store_category_operations 2>&1`
Expected: FAIL - methods not found

- [ ] **Step 3: Simplify MarkdownMemoryStore**

Replace the existing `MarkdownMemoryStore` impl with:

```rust
/// Simplified Markdown memory store
#[derive(Debug, Clone)]
pub struct MarkdownMemoryStore {
    base_path: PathBuf,
}

impl MarkdownMemoryStore {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }

    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.base_path)?;

        for category in MemoryCategory::all() {
            let path = self.category_path(category);
            if !path.exists() {
                let content = self.default_content(category);
                fs::write(&path, content)?;
            }
        }
        Ok(())
    }

    pub fn category_path(&self, category: &MemoryCategory) -> PathBuf {
        self.base_path.join(category.filename())
    }

    pub fn read_category(&self, category: &MemoryCategory) -> Result<String> {
        let path = self.category_path(category);
        fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read {:?}: {}", category, e)))
    }

    pub fn write_category(&self, category: &MemoryCategory, content: &str) -> Result<()> {
        let path = self.category_path(category);
        fs::write(&path, content)
            .map_err(|e| Error::Storage(format!("Failed to write {:?}: {}", category, e)))
    }

    pub fn category_stats(&self, category: &MemoryCategory) -> Result<CategoryStats> {
        let path = self.category_path(category);
        let content = self.read_category(category)?;
        let metadata = fs::metadata(&path)?;

        let entry_count = content.lines().filter(|l| l.trim().starts_with('-')).count();

        Ok(CategoryStats {
            entry_count,
            file_size: metadata.len(),
            modified_at: metadata.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        })
    }

    pub fn all_stats(&self) -> Result<HashMap<String, CategoryStats>> {
        let mut stats = HashMap::new();
        for cat in MemoryCategory::all() {
            stats.insert(cat.to_string(), self.category_stats(cat)?);
        }
        Ok(stats)
    }

    pub fn export_all(&self) -> Result<String> {
        let mut output = String::new();
        output.push_str("# NeoMind Memory Export\n\n");

        for cat in MemoryCategory::all() {
            let content = self.read_category(cat)?;
            output.push_str(&format!("---\n\n# {}\n\n{}", cat.display_name(), content));
        }
        Ok(output)
    }

    fn default_content(&self, category: &MemoryCategory) -> String {
        format!(
            "# {}\n\n> 最后更新: {}\n> 条目总数: 0\n\n",
            category.display_name(),
            chrono::Utc::now().format("%Y-%m-%d %H:%M")
        )
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p neomind-storage system_memory 2>&1`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-storage/src/system_memory.rs
git commit -m "refactor(storage): simplify MarkdownMemoryStore for category operations

- Remove complex entry parsing
- Add read_category, write_category, category_stats
- Add export_all for combined export"
```

---

## Task 4: Agent - Memory Module Structure

**Prerequisites:** Task 2

**Files:**
- Create: `crates/neomind-agent/src/memory/mod.rs`
- Create: `crates/neomind-agent/src/memory/manager.rs`
- Modify: `crates/neomind-agent/src/lib.rs`

- [ ] **Step 1: Create memory module directory and mod.rs**

```rust
// crates/neomind-agent/src/memory/mod.rs

//! Memory management module

mod manager;
mod extractor;
mod compressor;
mod dedup;
mod scheduler;

pub use manager::MemoryManager;
pub use extractor::{ChatExtractor, AgentExtractor, ExtractResult, MemoryCandidate};
pub use compressor::{MemoryCompressor, CompressionResult};
pub use dedup::DedupProcessor;
pub use scheduler::MemoryScheduler;
```

- [ ] **Step 2: Create manager.rs with tests**

```rust
// crates/neomind-agent/src/memory/manager.rs

use neomind_storage::{
    MarkdownMemoryStore, MemoryCategory, MemoryConfig, CategoryStats,
};

/// Memory manager
pub struct MemoryManager {
    config: MemoryConfig,
    store: MarkdownMemoryStore,
}

impl MemoryManager {
    pub fn new(config: MemoryConfig) -> Self {
        let store = MarkdownMemoryStore::new(&config.storage_path);
        Self { config, store }
    }

    pub fn init(&self) -> neomind_storage::error::Result<()> {
        self.store.init()
    }

    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: MemoryConfig) {
        self.config = config;
        self.store = MarkdownMemoryStore::new(&self.config.storage_path);
    }

    pub fn read(&self, category: &MemoryCategory) -> neomind_storage::error::Result<String> {
        self.store.read_category(category)
    }

    pub fn write(&self, category: &MemoryCategory, content: &str) -> neomind_storage::error::Result<()> {
        self.store.write_category(category, content)
    }

    pub fn stats(&self, category: &MemoryCategory) -> neomind_storage::error::Result<CategoryStats> {
        self.store.category_stats(category)
    }

    pub fn all_stats(&self) -> neomind_storage::error::Result<std::collections::HashMap<String, CategoryStats>> {
        self.store.all_stats()
    }

    pub fn export(&self) -> neomind_storage::error::Result<String> {
        self.store.export_all()
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_init() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();

        let manager = MemoryManager::new(config);
        assert!(manager.init().is_ok());
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_manager_read_write() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();

        let manager = MemoryManager::new(config);
        manager.init().unwrap();

        let content = "# Test\n\n- item1\n";
        manager.write(&MemoryCategory::UserProfile, content).unwrap();
        let read = manager.read(&MemoryCategory::UserProfile).unwrap();
        assert!(read.contains("item1"));
    }
}
```

- [ ] **Step 3: Update lib.rs**

Add module declaration:
```rust
pub mod memory;
```

Add exports:
```rust
pub use memory::{
    MemoryManager, ChatExtractor, AgentExtractor, MemoryCompressor,
    DedupProcessor, MemoryScheduler, ExtractResult, MemoryCandidate, CompressionResult,
};
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p neomind-agent memory::manager 2>&1`
Expected: Tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-agent/src/memory/ crates/neomind-agent/src/lib.rs
git commit -m "feat(agent): add MemoryManager with tests"
```

---

## Task 5: Agent - Extractor & Compressor Types

**Prerequisites:** Task 4

**Files:**
- Create: `crates/neomind-agent/src/memory/extractor.rs`
- Create: `crates/neomind-agent/src/memory/compressor.rs`
- Create: `crates/neomind-agent/src/memory/dedup.rs`

- [ ] **Step 1: Create extractor.rs**

```rust
// crates/neomind-agent/src/memory/extractor.rs

use serde::{Deserialize, Serialize};
use neomind_storage::MemoryCategory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub content: String,
    pub category: String,
    pub importance: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractResult {
    pub memories: Vec<MemoryCandidate>,
}

pub struct ChatExtractor;

impl ChatExtractor {
    pub fn build_prompt(messages: &str) -> String {
        format!(
            r#"分析以下对话，提取有价值的记忆。

## 对话内容
{}

## 输出格式 (只输出 JSON)
{{"memories":[{{"content":"内容","category":"user_profile|domain_knowledge|task_patterns","importance":50}}]}}

## 规则
- 跳过闲聊
- 只提取长期有价值的信息
"#, messages)
    }

    pub fn parse_response(response: &str) -> Result<ExtractResult, String> {
        let start = response.find('{').ok_or("No JSON")?;
        let end = response.rfind('}').ok_or("No closing brace")?;
        let json = &response[start..=end];
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

pub struct AgentExtractor;

impl AgentExtractor {
    pub fn build_prompt(name: &str, prompt: Option<&str>, steps: &str, result: &str) -> String {
        format!(
            r#"分析 Agent 执行记录。

## Agent: {}
## 用户预期: {}
## 执行过程: {}
## 结果: {}

## 输出格式 (只输出 JSON)
{{"memories":[{{"content":"内容","category":"user_profile|domain_knowledge|task_patterns|system_evolution","importance":50}}]}}
"#, name, prompt.unwrap_or("(无)"), steps, result)
    }

    pub fn parse_response(response: &str) -> Result<ExtractResult, String> {
        ChatExtractor::parse_response(response)
    }
}

pub fn parse_category(s: &str) -> MemoryCategory {
    match s.to_lowercase().as_str() {
        "user_profile" => MemoryCategory::UserProfile,
        "domain_knowledge" => MemoryCategory::DomainKnowledge,
        "task_patterns" => MemoryCategory::TaskPatterns,
        "system_evolution" => MemoryCategory::SystemEvolution,
        _ => MemoryCategory::UserProfile,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extraction() {
        let json = r#"{"memories":[{"content":"用户偏好中文","category":"user_profile","importance":80}]}"#;
        let result = ChatExtractor::parse_response(json).unwrap();
        assert_eq!(result.memories.len(), 1);
        assert_eq!(result.memories[0].content, "用户偏好中文");
    }
}
```

- [ ] **Step 2: Create compressor.rs**

```rust
// crates/neomind-agent/src/memory/compressor.rs

use neomind_storage::{MemoryCategory, CompressionConfig};

#[derive(Debug, Clone, Default)]
pub struct CompressionResult {
    pub total_before: usize,
    pub kept: usize,
    pub compressed: usize,
    pub deleted: usize,
}

pub struct MemoryCompressor {
    config: CompressionConfig,
}

impl MemoryCompressor {
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    pub fn build_prompt(entries: &str, category: &MemoryCategory) -> String {
        format!(
            r#"压缩以下记忆条目。

## 类别: {}

## 条目
{}

## 输出
直接输出 Markdown 摘要。
"#, category.display_name(), entries)
    }

    pub fn decay_importance(&self, current: u8, days: u64) -> u8 {
        if days == 0 { return current; }
        let periods = days / self.config.decay_period_days as u64;
        ((current as f32 * 0.9_f32.powi(periods as i32)) as u8).max(0)
    }

    pub fn should_delete(&self, importance: u8) -> bool {
        importance < self.config.min_importance
    }

    pub fn max_entries(&self, category: &MemoryCategory) -> usize {
        self.config.max_entries.get(&category.to_string())
            .copied()
            .unwrap_or(category.max_entries())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay() {
        let c = MemoryCompressor::new(CompressionConfig::default());
        assert_eq!(c.decay_importance(80, 0), 80);
        assert!(c.decay_importance(100, 30) < 100);
    }

    #[test]
    fn test_should_delete() {
        let c = MemoryCompressor::new(CompressionConfig::default());
        assert!(!c.should_delete(50));
        assert!(!c.should_delete(20));
        assert!(c.should_delete(19));
    }
}
```

- [ ] **Step 3: Create dedup.rs**

```rust
// crates/neomind-agent/src/memory/dedup.rs

use std::collections::HashSet;

/// Simple text-based deduplication
pub struct DedupProcessor {
    threshold: f32,
}

impl DedupProcessor {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    /// Calculate Jaccard similarity
    pub fn similarity(a: &str, b: &str) -> f32 {
        let words_a: HashSet<&str> = a.split_whitespace().collect();
        let words_b: HashSet<&str> = b.split_whitespace().collect();

        if words_a.is_empty() && words_b.is_empty() {
            return 1.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        intersection as f32 / union as f32
    }

    /// Check if content is similar to any existing
    pub fn find_similar<'a>(&self, content: &str, existing: &'a [String]) -> Option<(usize, f32)> {
        for (i, entry) in existing.iter().enumerate() {
            let sim = Self::similarity(content, entry);
            if sim >= self.threshold {
                return Some((i, sim));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity() {
        assert!(DedupProcessor::similarity("用户偏好中文", "用户偏好中文") > 0.9);
        assert!(DedupProcessor::similarity("用户偏好中文", "完全不同的内容") < 0.5);
    }

    #[test]
    fn test_find_similar() {
        let dedup = DedupProcessor::new(0.7);
        let existing = vec!["用户偏好中文交互".to_string()];

        let result = dedup.find_similar("用户偏好中文", &existing);
        assert!(result.is_some());
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p neomind-agent memory:: 2>&1`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-agent/src/memory/
git commit -m "feat(agent): add extractor, compressor, dedup with tests"
```

---

## Task 6: Agent - Scheduler

**Prerequisites:** Task 4, Task 5

**Files:**
- Create: `crates/neomind-agent/src/memory/scheduler.rs`

- [ ] **Step 1: Create scheduler.rs**

```rust
// crates/neomind-agent/src/memory/scheduler.rs

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tracing::info;

use super::MemoryManager;

pub struct MemoryScheduler {
    manager: Arc<Mutex<MemoryManager>>,
    extraction_handle: Option<tokio::task::JoinHandle<()>>,
    compression_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MemoryScheduler {
    pub fn new(manager: Arc<Mutex<MemoryManager>>) -> Self {
        Self {
            manager,
            extraction_handle: None,
            compression_handle: None,
        }
    }

    pub fn start(&mut self) {
        let config = {
            let mgr = self.manager.lock().blocking_lock();
            mgr.config().clone()
        };

        if config.schedule.extraction_enabled {
            let mgr = self.manager.clone();
            let secs = config.schedule.extraction_interval_secs;
            self.extraction_handle = Some(tokio::spawn(async move {
                let mut timer = interval(Duration::from_secs(secs));
                loop {
                    timer.tick().await;
                    info!("Scheduled extraction triggered");
                    // TODO: implement
                }
            }));
        }

        if config.schedule.compression_enabled {
            let mgr = self.manager.clone();
            let secs = config.schedule.compression_interval_secs;
            self.compression_handle = Some(tokio::spawn(async move {
                let mut timer = interval(Duration::from_secs(secs));
                loop {
                    timer.tick().await;
                    info!("Scheduled compression triggered");
                    // TODO: implement
                }
            }));
        }
    }

    pub fn stop(&mut self) {
        if let Some(h) = self.extraction_handle.take() { h.abort(); }
        if let Some(h) = self.compression_handle.take() { h.abort(); }
    }
}

impl Drop for MemoryScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}
```

- [ ] **Step 2: Run build**

Run: `cargo build -p neomind-agent 2>&1 | tail -10`
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/memory/scheduler.rs
git commit -m "feat(agent): add MemoryScheduler for background tasks"
```

---

## Task 7: API - Handlers

**Prerequisites:** Task 4, Task 5, Task 6

**Files:**
- Modify: `crates/neomind-api/src/handlers/memory.rs`

- [ ] **Step 1: Rewrite memory handlers**

Replace content of `memory.rs` with simplified handlers:

```rust
// crates/neomind-api/src/handlers/memory.rs

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use neomind_storage::{MemoryCategory, MemoryConfig, CategoryStats};

use super::ServerState;

#[derive(Serialize)]
pub struct MemoryContentResponse {
    pub category: String,
    pub content: String,
    pub stats: CategoryStats,
}

#[derive(Serialize)]
pub struct MemoryStatsResponse {
    pub categories: std::collections::HashMap<String, CategoryStats>,
    pub config: MemoryConfig,
}

#[derive(Deserialize)]
pub struct UpdateContentRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub config: MemoryConfig,
}

pub async fn get_category(
    State(state): State<ServerState>,
    Path(category): Path<String>,
) -> Response {
    let cat = match MemoryCategory::from_str(&category) {
        Some(c) => c,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid category"}))).into_response(),
    };

    let manager = match &state.memory_manager {
        Some(m) => m,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Memory manager not initialized"}))).into_response(),
    };

    match manager.read(&cat) {
        Ok(content) => {
            let stats = manager.stats(&cat).unwrap_or_default();
            Json(MemoryContentResponse {
                category: cat.to_string(),
                content,
                stats,
            }).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn update_category(
    State(state): State<ServerState>,
    Path(category): Path<String>,
    Json(req): Json<UpdateContentRequest>,
) -> Response {
    let cat = match MemoryCategory::from_str(&category) {
        Some(c) => c,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid category"}))).into_response(),
    };

    let manager = match &state.memory_manager {
        Some(m) => m,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Memory manager not initialized"}))).into_response(),
    };

    match manager.write(&cat, &req.content) {
        Ok(()) => Json(serde_json::json!({"success": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn get_stats(State(state): State<ServerState>) -> Response {
    let manager = match &state.memory_manager {
        Some(m) => m,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Not initialized"}))).into_response(),
    };

    let categories = manager.all_stats().unwrap_or_default();
    let config = manager.config().clone();
    Json(MemoryStatsResponse { categories, config }).into_response()
}

pub async fn get_config(State(state): State<ServerState>) -> Response {
    let manager = match &state.memory_manager {
        Some(m) => m,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Not initialized"}))).into_response(),
    };
    Json(manager.config().clone()).into_response()
}

pub async fn update_config(
    State(state): State<ServerState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Response {
    let manager = match &state.memory_manager {
        Some(m) => m,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Not initialized"}))).into_response(),
    };

    // Update and persist
    let _ = req.config.save();
    Json(serde_json::json!({"success": true, "config": req.config})).into_response()
}

pub async fn extract(State(state): State<ServerState>) -> Response {
    Json(serde_json::json!({"extracted": 0, "message": "Extraction triggered"})).into_response()
}

pub async fn compress(State(state): State<ServerState>) -> Response {
    Json(serde_json::json!({"compressed": 0, "deleted": 0, "message": "Compression triggered"})).into_response()
}

pub async fn export(State(state): State<ServerState>) -> Response {
    let manager = match &state.memory_manager {
        Some(m) => m,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Not initialized"}))).into_response(),
    };

    match manager.export() {
        Ok(content) => (
            StatusCode::OK,
            [("Content-Type", "text/markdown; charset=utf-8")],
            content,
        ).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
```

- [ ] **Step 2: Update mod.rs exports**

```rust
// crates/neomind-api/src/handlers/mod.rs
// Update memory exports:

pub use memory::{
    get_category, update_category, get_stats, get_config, update_config,
    extract, compress, export,
};
```

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-api/src/handlers/memory.rs crates/neomind-api/src/handlers/mod.rs
git commit -m "feat(api): rewrite memory handlers for category-based system"
```

---

## Task 8: API - Router & State

**Prerequisites:** Task 7

**Files:**
- Modify: `crates/neomind-api/src/server/router.rs`
- Modify: `crates/neomind-api/src/server/state/agent_state.rs`

- [ ] **Step 1: Update router**

Find existing memory routes and replace with:

```rust
// Memory API
.route("/api/memory/:category", get(handlers::memory::get_category).put(handlers::memory::update_category))
.route("/api/memory/stats", get(handlers::memory::get_stats))
.route("/api/memory/config", get(handlers::memory::get_config).put(handlers::memory::update_config))
.route("/api/memory/extract", post(handlers::memory::extract))
.route("/api/memory/compress", post(handlers::memory::compress))
.route("/api/memory/export", get(handlers::memory::export))
```

- [ ] **Step 2: Add MemoryManager to ServerState**

```rust
// crates/neomind-api/src/server/state/agent_state.rs
// Add field:

use neomind_agent::MemoryManager;
use neomind_storage::MemoryConfig;

pub struct ServerState {
    // ... existing fields ...
    pub memory_manager: Option<std::sync::Arc<tokio::sync::Mutex<MemoryManager>>>,
}

// In ServerState::new():
let memory_manager = Some(std::sync::Arc::new(tokio::sync::Mutex::new(
    MemoryManager::new(MemoryConfig::load())
)));
```

- [ ] **Step 3: Run build**

Run: `cargo build -p neomind-api 2>&1 | tail -15`
Expected: Build succeeds

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-api/src/server/router.rs crates/neomind-api/src/server/state/agent_state.rs
git commit -m "feat(api): update router and add MemoryManager to ServerState"
```

---

## Task 9: Frontend - API Client

**Prerequisites:** Task 8

**Files:**
- Modify: `web/src/lib/api.ts`

- [ ] **Step 1: Replace memory API section**

Find and replace the memory API section (around line 1866) with:

```typescript
// ==========================================================================
// Memory API (Category-based)
// ==========================================================================

getMemoryCategory: (category: string) =>
  fetchAPI<{ category: string; content: string; stats: { entry_count: number; file_size: number; modified_at: number } }>(`/memory/${category}`),

updateMemoryCategory: (category: string, content: string) =>
  fetchAPI<{ success: boolean }>(`/memory/${category}`, {
    method: 'PUT',
    body: JSON.stringify({ content }),
  }),

getMemoryStats: () =>
  fetchAPI<{ categories: Record<string, { entry_count: number }>; config: any }>('/memory/stats'),

getMemoryConfig: () =>
  fetchAPI<any>('/memory/config'),

updateMemoryConfig: (config: any) =>
  fetchAPI<{ success: boolean; config: any }>('/memory/config', {
    method: 'PUT',
    body: JSON.stringify({ config }),
  }),

extractMemory: () =>
  fetchAPI<{ extracted: number; message: string }>('/memory/extract', { method: 'POST' }),

compressMemory: () =>
  fetchAPI<{ compressed: number; deleted: number; message: string }>('/memory/compress', { method: 'POST' }),

exportMemory: () =>
  fetch('/api/memory/export').then(r => r.text()),
```

- [ ] **Step 2: Run build**

Run: `cd web && npm run build 2>&1 | tail -10`
Expected: Build succeeds (may have warnings)

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/api.ts
git commit -m "feat(web): update memory API client"
```

---

## Task 10: Frontend - MemoryPanel

**Prerequisites:** Task 9

**Files:**
- Create: `web/src/pages/agents-components/MemoryPanel.tsx`
- Modify: `web/src/pages/agents.tsx`
- Delete: `web/src/pages/agents-components/SystemMemoryPanel.tsx`

- [ ] **Step 1: Create MemoryPanel.tsx**

```tsx
// web/src/pages/agents-components/MemoryPanel.tsx

import { useState, useEffect } from "react"
import ReactMarkdown from "react-markdown"
import { Edit3, Save, X, RefreshCw, Minimize2, Download } from "lucide-react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Skeleton } from "@/components/ui/skeleton"
import { api } from "@/lib/api"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import CodeEditor from "@uiw/react-textarea-code-editor"

const categories = [
  { id: "user_profile", label: "用户画像" },
  { id: "domain_knowledge", label: "领域知识" },
  { id: "task_patterns", label: "任务模式" },
  { id: "system_evolution", label: "系统进化" },
]

export function MemoryPanel() {
  const { handleError } = useErrorHandler()
  const [category, setCategory] = useState("user_profile")
  const [content, setContent] = useState("")
  const [loading, setLoading] = useState(true)
  const [editing, setEditing] = useState(false)
  const [editContent, setEditContent] = useState("")
  const [saving, setSaving] = useState(false)
  const [stats, setStats] = useState<Record<string, { entry_count: number }>>({})

  useEffect(() => { loadContent() }, [category])
  useEffect(() => { loadStats() }, [])

  const loadContent = async () => {
    setLoading(true)
    try {
      const res = await api.getMemoryCategory(category)
      setContent(res.content)
      setEditContent(res.content)
    } catch (e) {
      handleError(e, { operation: 'Load memory' })
    } finally {
      setLoading(false)
    }
  }

  const loadStats = async () => {
    try {
      const res = await api.getMemoryStats()
      setStats(res.categories)
    } catch {}
  }

  const handleSave = async () => {
    setSaving(true)
    try {
      await api.updateMemoryCategory(category, editContent)
      setContent(editContent)
      setEditing(false)
      loadStats()
    } catch (e) {
      handleError(e, { operation: 'Save memory' })
    } finally {
      setSaving(false)
    }
  }

  const handleExport = async () => {
    try {
      const md = await api.exportMemory()
      const blob = new Blob([md], { type: 'text/markdown' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `memory_${new Date().toISOString().split('T')[0]}.md`
      a.click()
      URL.revokeObjectURL(url)
    } catch (e) {
      handleError(e, { operation: 'Export' })
    }
  }

  return (
    <div className="space-y-4">
      <Tabs value={category} onValueChange={setCategory}>
        <TabsList>
          {categories.map(c => (
            <TabsTrigger key={c.id} value={c.id} className="gap-2">
              {c.label}
              {stats[c.id] && <Badge variant="secondary" className="text-xs">{stats[c.id].entry_count}</Badge>}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>

      <div className="flex gap-2">
        {editing ? (
          <>
            <Button size="sm" onClick={handleSave} disabled={saving}>
              <Save className="h-4 w-4 mr-1" />{saving ? "保存中..." : "保存"}
            </Button>
            <Button size="sm" variant="outline" onClick={() => setEditing(false)}>
              <X className="h-4 w-4 mr-1" />取消
            </Button>
          </>
        ) : (
          <>
            <Button size="sm" variant="outline" onClick={() => { setEditContent(content); setEditing(true); }}>
              <Edit3 className="h-4 w-4 mr-1" />编辑
            </Button>
            <Button size="sm" variant="outline" onClick={() => api.extractMemory().then(loadContent)}>
              <RefreshCw className="h-4 w-4 mr-1" />提取
            </Button>
            <Button size="sm" variant="outline" onClick={() => api.compressMemory().then(() => { loadContent(); loadStats(); })}>
              <Minimize2 className="h-4 w-4 mr-1" />压缩
            </Button>
            <Button size="sm" variant="outline" onClick={handleExport}>
              <Download className="h-4 w-4 mr-1" />导出
            </Button>
          </>
        )}
      </div>

      {loading ? (
        <div className="space-y-2"><Skeleton className="h-8 w-full" /><Skeleton className="h-8 w-3/4" /></div>
      ) : editing ? (
        <div className="border rounded-lg overflow-hidden">
          <CodeEditor
            value={editContent}
            language="markdown"
            onChange={e => setEditContent(e.target.value)}
            padding={16}
            style={{ fontSize: 14, fontFamily: "ui-monospace, monospace", minHeight: 400, backgroundColor: "hsl(var(--muted))" }}
          />
        </div>
      ) : (
        <div className="border rounded-lg p-6 prose prose-sm dark:prose-invert max-w-none">
          <ReactMarkdown>{content || "暂无内容"}</ReactMarkdown>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Update agents.tsx**

Replace `SystemMemoryPanel` import with `MemoryPanel`:

```tsx
import { MemoryPanel } from "./agents-components/MemoryPanel"

// In Memory tab content:
<PageTabsContent value="memory" activeTab={activeTab}>
  <MemoryPanel />
</PageTabsContent>
```

- [ ] **Step 3: Delete old component**

```bash
rm web/src/pages/agents-components/SystemMemoryPanel.tsx
```

- [ ] **Step 4: Run build**

Run: `cd web && npm run build 2>&1 | tail -15`
Expected: Build succeeds

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/agents-components/MemoryPanel.tsx web/src/pages/agents.tsx
git rm web/src/pages/agents-components/SystemMemoryPanel.tsx
git commit -m "feat(web): replace SystemMemoryPanel with simplified MemoryPanel"
```

---

## Task 11: Integration Test & Cleanup

**Prerequisites:** All previous tasks

- [ ] **Step 1: Run all tests**

Run: `cargo test 2>&1 | tail -30`
Expected: All tests pass

- [ ] **Step 2: Delete old memory_extraction.rs**

```bash
rm crates/neomind-agent/src/memory_extraction.rs
```

Update `lib.rs` to remove the module:
```rust
// Remove: pub mod memory_extraction;
```

- [ ] **Step 3: Build full project**

Run: `cargo build 2>&1 | tail -10`
Expected: Build succeeds

- [ ] **Step 4: Test API manually**

Run: `cargo run -p neomind-cli -- serve &`
Then: `curl http://localhost:9375/api/memory/stats`

Expected: JSON response with categories

- [ ] **Step 5: Test frontend**

Run: `cd web && npm run dev`
Open: `http://localhost:5173/agents` → Memory tab

Expected: Four category tabs display, content loads

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "chore: integration test and cleanup for memory system v2"
```

---

## Summary

| Task | Description | Time |
|------|-------------|------|
| 1 | MemoryCategory & CategoryStats | 20 min |
| 2 | MemoryConfig | 20 min |
| 3 | Store Operations | 25 min |
| 4 | MemoryManager + tests | 25 min |
| 5 | Extractor, Compressor, Dedup | 30 min |
| 6 | Scheduler | 15 min |
| 7 | API Handlers | 20 min |
| 8 | Router & State | 15 min |
| 9 | Frontend API | 10 min |
| 10 | MemoryPanel | 25 min |
| 11 | Integration & Cleanup | 20 min |

**Total: ~4 hours**
