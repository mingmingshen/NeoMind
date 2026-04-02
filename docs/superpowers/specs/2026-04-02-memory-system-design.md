# Memory System Design

## Overview

基于 LLM 的智能记忆系统，从 Chat 对话和 Agent 执行中提取有价值的记忆，支持用户画像、领域知识、任务模式和系统进化四类记忆。

## Goals

1. **用户偏好学习** - 记住用户习惯、偏好、常用操作
2. **知识积累** - 从对话和执行中提取有价值信息
3. **上下文压缩** - 用 LLM 定期总结，减少 token 消耗
4. **系统自进化** - 记录系统学习经验，持续优化

## Memory Categories

| Category | File | Description | Max Entries |
|----------|------|-------------|-------------|
| 用户画像 | `user_profile.md` | 用户偏好、习惯、关注点 | 50 |
| 领域知识 | `domain_knowledge.md` | 设备、环境、业务规则 | 100 |
| 任务模式 | `task_patterns.md` | Agent 执行经验、工作流 | 80 |
| 系统进化 | `system_evolution.md` | 自学习、优化记录 | 30 |

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│                     MemoryManager                           │
│                                                            │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────┐   │
│  │  extract()  │  │  compress() │  │  get/put_md()    │   │
│  │  轻量 LLM   │  │  强力 LLM   │  │  读写 MD 文件    │   │
│  └─────────────┘  └─────────────┘  └──────────────────┘   │
│                          ↓                                 │
│  ┌──────────────────────────────────────────────────────┐ │
│  │            Markdown Files (四类文件)                 │ │
│  │  user_profile.md | domain_knowledge.md               │ │
│  │  task_patterns.md | system_evolution.md              │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
         ↑                              ↑
         │                              │
┌────────┴────────┐           ┌────────┴────────┐
│ LlmBackendStore │           │ 定时任务调度器   │
│ (复用现有配置)   │           │ (tokio async)   │
└─────────────────┘           └─────────────────┘
```

## Storage Format

**文件结构**:

```
data/memory/
├── user_profile.md       # 用户画像
├── domain_knowledge.md   # 领域知识
├── task_patterns.md      # 任务模式
└── system_evolution.md   # 系统进化
```

**Markdown 文件格式**:

```markdown
# 用户画像

> 最后更新: 2026-04-02 14:30
> 条目总数: 12

## 偏好设置

- [2026-03-15] 用户偏好中文交互 [重要度: 85] [来源: chat#abc123]
- [2026-03-20] 偏好简洁回复，避免冗长解释 [重要度: 75] [来源: agent#temp-monitor]
- [2026-04-01] 喜欢看到 Agent 的推理过程 [重要度: 60] [来源: chat#def456]

## 关注点

- [2026-03-10] 关注能源消耗，希望优化用电 [重要度: 85] [来源: chat#energy]
- [2026-03-28] 对温度监控特别敏感，阈值设为 28°C [重要度: 80] [来源: agent#temp-monitor]

---

## 压缩摘要

> 由 LLM 于 2026-03-28 生成：

**交互风格**: 中文、简洁、显示推理过程

**生活规律**:
- 作息: 23:00 就寝，8:30 周末起床
- 关注点: 能源优化、温度舒适度

**典型设备使用模式**: 晚间自动化灯光控制，早晨咖啡机
```

## Data Model

```rust
/// 记忆类型
pub enum MemoryCategory {
    UserProfile,      // 用户画像
    DomainKnowledge,  // 领域知识
    TaskPatterns,     // 任务模式
    SystemEvolution,  // 系统进化
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

    pub fn max_entries(&self) -> usize {
        match self {
            Self::UserProfile => 50,
            Self::DomainKnowledge => 100,
            Self::TaskPatterns => 80,
            Self::SystemEvolution => 30,
        }
    }
}

/// 记忆条目（内存中表示，用于提取和压缩处理）
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: MemoryCategory,
    pub importance: u8,           // 0-100
    pub sources: Vec<MemorySource>,
    pub created_at: i64,
    pub updated_at: i64,
    pub occurrence_count: u32,    // 相似内容合并时累加
}

pub struct MemorySource {
    pub source_type: MemorySourceType,  // Chat | Agent | Manual
    pub id: String,
}
```

## API Design (Simplified)

```
# 读写 Markdown 内容
GET    /api/memory/:category           # 获取 MD 内容
PUT    /api/memory/:category           # 保存 MD 内容

# 批量操作
POST   /api/memory/extract             # 手动触发提取
POST   /api/memory/compress            # 手动触发压缩

# 配置
GET    /api/memory/config              # 获取配置
PUT    /api/memory/config              # 更新配置

# 统计
GET    /api/memory/stats               # 获取统计信息
```

**API Types**:

```rust
/// GET /api/memory/:category 响应
#[derive(Serialize)]
pub struct MemoryContentResponse {
    pub category: String,
    pub content: String,        // 原始 Markdown 内容
    pub stats: CategoryStats,
}

#[derive(Serialize)]
pub struct CategoryStats {
    pub entry_count: usize,
    pub last_updated: i64,
    pub file_size: u64,
}

/// POST /api/memory/extract 请求
#[derive(Deserialize)]
pub struct ExtractRequest {
    pub scope: ExtractScope,
}

pub enum ExtractScope {
    All,
    Chats { since: Option<i64> },
    Agents { since: Option<i64> },
}

/// POST /api/memory/compress 请求
#[derive(Deserialize)]
pub struct CompressRequest {
    pub category: Option<String>,   // None = 全部
    pub force: bool,                // 强制压缩，忽略条目数检查
}
```

## Extraction Flow

### 触发方式

1. **定时批量** - 每小时扫描未处理的 Chat/Agent 记录
2. **手动触发** - API 调用

### 提取管道

```
Collect → Extract (LLM) → Dedupe → Append to MD
   │            │            │           │
   │            │            │           └── 追加到 MD 文件
   │            │            └── 相似度检测，增量更新
   │            └── 调用轻量 LLM
   └── 收集原始数据
```

### Chat 提取 Prompt

```
分析以下对话，提取有价值的记忆信息。

## 对话内容
{messages}

## 提取规则
1. **用户画像** - 用户的偏好、习惯、关注点
2. **领域知识** - 设备、环境、业务规则
3. **任务模式** - 用户想解决的问题、常用工作流

## 输出格式 (JSON)
{
  "memories": [
    {
      "content": "记忆内容（简洁的一句话）",
      "category": "user_profile|domain_knowledge|task_patterns",
      "importance": 0-100
    }
  ]
}

## 注意
- 跳过问候语、闲聊
- 只提取长期有价值的信息
```

### Agent 提取 Prompt

```
分析以下 Agent 执行记录，提取有价值的记忆信息。

## Agent 信息
- 名称: {agent_name}
- 类型: {agent_type}

## 用户预期（提示词）
{user_prompt}

## 执行过程
{reasoning_steps}

## 执行结果
{conclusion}

## 提取规则
1. **用户画像** - 用户对这个 Agent 的期望
2. **领域知识** - 发现的设备状态、环境规律
3. **任务模式** - 成功模式、失败原因
4. **系统进化** - Agent 学到的经验

## 输出格式 (JSON)
{
  "memories": [
    {
      "content": "记忆内容",
      "category": "user_profile|domain_knowledge|task_patterns|system_evolution",
      "importance": 0-100
    }
  ]
}
```

### 去重逻辑

```rust
impl MemoryManager {
    /// 写入前去重
    fn append_with_dedup(&self, category: MemoryCategory, entries: Vec<MemoryEntry>) {
        let existing = self.parse_md_entries(&category);

        for entry in entries {
            // 查找相似条目
            if let Some(similar) = find_similar(&entry, &existing, threshold: 0.85) {
                // 增量更新：增加出现次数，更新时间
                self.update_entry_line(&similar, |line| {
                    line.occurrence_count += 1;
                    line.updated_at = now();
                });
            } else {
                // 追加新条目
                self.append_to_md(&category, &entry);
            }
        }
    }
}
```

## Compression Flow

### 触发方式

1. **定时** - 每天凌晨
2. **手动** - API 调用
3. **条目超限** - 某类别超过 max_entries

### 压缩管道

```
Read MD → Parse Entries → Decay → Select → Compress (LLM) → Write MD
    │           │           │        │           │            │
    │           │           │        │           │            └── 更新 MD
    │           │           │        │           └── 生成摘要
    │           │           │        └── 选择要压缩的条目
    │           │           └── 重要性衰减
    │           └── 解析 MD 中的条目
    └── 读取文件
```

### 重要性衰减

```rust
// 每 30 天衰减 10%
fn apply_decay(entry: &mut MemoryEntry) -> bool {
    let days = days_since(entry.updated_at);
    let periods = days / 30;
    entry.importance = (entry.importance as f32 * 0.9_f32.powi(periods)) as u8;

    // 低于 20 则删除
    entry.importance >= 20
}
```

### LLM 压缩

**压缩 Prompt**:

```
将以下记忆条目压缩成简洁的摘要。

## 原始条目
{entries}

## 压缩规则
1. 合并相似内容
2. 提取通用模式
3. 保留关键数值和阈值
4. 去除冗余

## 输出格式
直接输出 Markdown 格式的摘要内容，包含：
- 简短的标题
- 关键点列表
- 发现的模式（如有）
```

## Configuration

```rust
/// 记忆系统配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub enabled: bool,

    /// 存储路径
    #[serde(default = "default_storage_path")]
    pub storage_path: String,

    /// 提取配置
    pub extraction: ExtractionConfig,

    /// 压缩配置
    pub compression: CompressionConfig,

    /// LLM 后端配置
    pub llm: LlmConfig,

    /// 定时配置
    pub schedule: ScheduleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub similarity_threshold: f32,  // 0.85
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub decay_period_days: u8,      // 30
    pub min_importance: u8,         // 20
    pub max_entries: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// 提取用 LLM（轻量）
    pub extraction_backend_id: Option<String>,
    /// 压缩用 LLM（强力）
    pub compression_backend_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub extraction_enabled: bool,
    pub extraction_interval_secs: u64,  // 3600
    pub compression_enabled: bool,
    pub compression_interval_secs: u64, // 86400
}
```

**配置文件**: `data/memory_config.json`

```json
{
  "enabled": true,
  "storage_path": "data/memory",
  "extraction": {
    "similarity_threshold": 0.85
  },
  "compression": {
    "decay_period_days": 30,
    "min_importance": 20,
    "max_entries": {
      "user_profile": 50,
      "domain_knowledge": 100,
      "task_patterns": 80,
      "system_evolution": 30
    }
  },
  "llm": {
    "extraction_backend_id": null,
    "compression_backend_id": null
  },
  "schedule": {
    "extraction_enabled": true,
    "extraction_interval_secs": 3600,
    "compression_enabled": true,
    "compression_interval_secs": 86400
  }
}
```

## UI Design

**极简方案：直接渲染 Markdown**

```
┌─────────────────────────────────────────────────────────┐
│ [用户画像] [领域知识] [任务模式] [系统进化]              │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │                                                  │   │
│  │   Markdown 渲染区 (ReactMarkdown)               │   │
│  │                                                  │   │
│  │   # 用户画像                                     │   │
│  │   > 最后更新: 2026-04-02                         │   │
│  │                                                  │   │
│  │   ## 偏好设置                                    │   │
│  │   - [2026-03-15] 用户偏好中文交互...             │   │
│  │                                                  │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  [编辑] [提取] [压缩] [导出]                            │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**交互**:
- Tab 切换四个类别
- 默认显示渲染后的 Markdown
- 点击"编辑"切换到 Monaco Editor 编辑源码
- "提取"和"压缩"调用 API
- "导出"下载当前 MD 文件

**组件**:

```tsx
// MemoryPanel.tsx
export function MemoryPanel() {
  const [category, setCategory] = useState("user_profile")
  const [editing, setEditing] = useState(false)
  const [content, setContent] = useState("")

  return (
    <div>
      <Tabs value={category} onValueChange={setCategory}>
        <TabsList>
          <TabsTrigger value="user_profile">用户画像</TabsTrigger>
          <TabsTrigger value="domain_knowledge">领域知识</TabsTrigger>
          <TabsTrigger value="task_patterns">任务模式</TabsTrigger>
          <TabsTrigger value="system_evolution">系统进化</TabsTrigger>
        </TabsList>
      </Tabs>

      {editing ? (
        <MonacoEditor value={content} onChange={setContent} />
      ) : (
        <ReactMarkdown>{content}</ReactMarkdown>
      )}

      <div className="flex gap-2">
        <Button onClick={() => setEditing(!editing)}>
          {editing ? "预览" : "编辑"}
        </Button>
        {editing && <Button onClick={handleSave}>保存</Button>}
        <Button variant="outline" onClick={handleExtract}>提取</Button>
        <Button variant="outline" onClick={handleCompress}>压缩</Button>
        <Button variant="outline" onClick={handleExport}>导出</Button>
      </div>
    </div>
  )
}
```

## File Structure

```
crates/neomind-storage/src/
├── memory/
│   ├── mod.rs
│   ├── config.rs          # MemoryConfig
│   ├── category.rs        # MemoryCategory
│   └── store.rs           # MD 文件读写

crates/neomind-agent/src/
├── memory/
│   ├── mod.rs
│   ├── manager.rs         # MemoryManager
│   ├── extractor.rs       # 提取器
│   ├── compressor.rs      # 压缩器
│   └── scheduler.rs       # 定时任务

crates/neomind-api/src/handlers/
├── memory.rs              # API handlers

web/src/pages/agents-components/
├── MemoryPanel.tsx        # 主面板
```

## Implementation Phases

### Phase 1: Core (1-2 days)
- [ ] MemoryConfig 配置
- [ ] MemoryCategory 枚举
- [ ] MD 文件读写（store.rs）

### Phase 2: API (1 day)
- [ ] GET/PUT /api/memory/:category
- [ ] POST /api/memory/extract
- [ ] POST /api/memory/compress
- [ ] GET /api/memory/config
- [ ] GET /api/memory/stats

### Phase 3: Extraction (2 days)
- [ ] ChatExtractor
- [ ] AgentExtractor
- [ ] 去重逻辑
- [ ] 追加到 MD 文件

### Phase 4: Compression (1-2 days)
- [ ] 重要性衰减
- [ ] LlmCompressor
- [ ] 更新 MD 文件

### Phase 5: Scheduler (1 day)
- [ ] 定时任务
- [ ] 与 ServerState 集成

### Phase 6: Frontend (1 day)
- [ ] MemoryPanel 组件
- [ ] 集成到 Agents 页面

## Open Questions

1. **Embedding 支持** - 未来是否需要 vector embedding 提升搜索？
2. **多用户隔离** - 是否需要支持多用户？
3. **记忆注入** - Agent 执行时如何高效检索相关记忆？
