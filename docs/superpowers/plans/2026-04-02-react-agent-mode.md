# NeoMind ReAct Agent Mode 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 增强 `execute_with_tools` 的工具调用循环，将每轮迭代拆分为 Thought → Action → Observation 三类步骤，让前端能实时展示 LLM 的推理过程，并为 data-on-demand 打下基础。

**Architecture:** 不创建新的执行函数。修改 `execute_with_tools` 内部循环，在 LLM 每次响应后提取推理文本作为 "thought"，工具调用标记为 "action"，执行结果标记为 "observation"。优化工具响应的 token 效率。通过 `AiAgent.execution_mode` 字段选择模式，默认 "standard" 保持完全向后兼容。

**Tech Stack:** Rust, serde, LLM function calling, NeoMind EventBus, React + TypeScript

---

## 实测验证

### Tool 模式（无绑定资源）— 2026-04-02 实测

Agent 自主执行了 6 轮工具调用，发现电池异常并创建告警：

```
Round 1: tool "device" → 列出2个NE101设备
Round 2: tool "device" → 查询TEST设备battery=41%
Round 3: tool "device" → 查询TEST设备batteryVoltage=4590mV
Round 4: tool "device" → 查询101 PC Test设备battery (空)
Round 5: tool "device" → 查询101 PC Test设备batteryVoltage (空)
Round 6: tool "alert"  → 创建低电量告警
```

**优点**: Agent 自主发现设备、逐步排查、主动创建告警
**问题**:
1. 所有 reasoning step 都显示 "Executed tool 'device'" — 没有 LLM 的推理文本
2. step_type 全部是 "tool_call" — 无法区分思考/行动/观察
3. 工具返回的设备列表包含完整 metric schema（~800 tokens），浪费 context
4. 前端只看到 "tool_call" 步骤，看不到 Agent "为什么" 要查 battery

### 标准模式（有绑定资源）— 2026-04-02 实测

Agent 只收集到图像数据，1 个 reasoning step，结论 "No actions required"，完全遗漏了电池异常。

**结论**: Tool 模式已经远优于标准模式。ReAct 增强的核心价值是让推理过程可见，而非改变执行逻辑。

---

## 设计原则（来自行业最佳实践）

> 来源: [Anthropic - Building Effective Agents](https://www.anthropic.com/research/building-effective-agents), [Anthropic - Writing Effective Tools](https://www.anthropic.com/engineering/writing-tools-for-agents), [Braintrust - Canonical Agent Architecture](https://www.braintrust.dev/blog/agent-while-loop)

1. **The canonical agent is a while loop**: `while (!done) { callLLM(); executeTools(); }` — Claude Code 和 OpenAI Agents SDK 都收敛于此。我们的 `execute_with_tools` 已经是了。
2. **Function calling > text ReAct**: 所有生产系统都用原生 function calling，不是文本解析。ReAct 的价值在于结构化追踪，不在于文本格式。
3. **Tool design > prompt engineering**: 工具响应占 agent 看到内容的 ~80%（67.6% tool responses + 10.7% tool definitions）。优化工具返回比优化 prompt 更有效。
4. **Context efficiency**: Claude Code 默认限制工具响应 25,000 tokens。我们的工具返回完整 JSON 没有截断。
5. **Transparency**: Anthropic 的三大原则之一是 "明确展示规划步骤"。

---

## 文件结构

| File | Change | Purpose |
|------|--------|---------|
| `crates/neomind-storage/src/agents.rs:88-91` | Add field | `execution_mode: String` 字段 |
| `crates/neomind-agent/src/ai_agent/executor.rs:1239-1255` | Modify | ReAct 模式系统提示词 |
| `crates/neomind-agent/src/ai_agent/executor.rs:1288-1290` | Add | `react_thoughts` 收集器 |
| `crates/neomind-agent/src/ai_agent/executor.rs:1319-1326` | Modify | 提取 LLM 推理文本为 thought |
| `crates/neomind-agent/src/ai_agent/executor.rs:1368-1385` | Modify | 截断工具响应（context 效率） |
| `crates/neomind-agent/src/ai_agent/executor.rs:1396-1416` | Modify | 构建 thought/action/observation 步骤 |
| `crates/neomind-api/src/handlers/agents.rs` | Add fields | API 支持 execution_mode |
| `web/src/pages/agents-components/AgentThinkingPanel.tsx:172-189` | Modify | 新增 ReAct 步骤样式 |
| `web/src/i18n/locales/{en,zh}/agents.json` | Add keys | 执行模式翻译 |

---

## Task 1: AiAgent 添加 execution_mode 字段

**Files:**
- Modify: `crates/neomind-storage/src/agents.rs`

- [ ] **Step 1: 在 AiAgent struct 中添加字段**

在 `crates/neomind-storage/src/agents.rs` 的 `AiAgent` struct 中，`error_message` 之前添加：

```rust
    /// Agent execution mode: "standard" (default) or "react"
    #[serde(default = "default_execution_mode")]
    pub execution_mode: String,
```

在文件 default helper 函数区域（`default_max_chain_depth` 附近）添加：

```rust
fn default_execution_mode() -> String {
    "standard".to_string()
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo check -p neomind-storage 2>&1 | tail -10`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-storage/src/agents.rs
git commit -m "feat(storage): add execution_mode field to AiAgent"
```

---

## Task 2: ReAct 系统提示词

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs:1239-1255`

- [ ] **Step 1: 在 execute_with_tools 中添加条件分支**

在 `execute_with_tools` 中构建 `system_prompt` 的位置（约 line 1239），将现有硬编码 prompt 改为条件分支：

```rust
let is_react_mode = agent.execution_mode == "react";

let system_prompt = if is_react_mode {
    format!(
        "You are '{name}', an IoT monitoring agent. Current time: {time}\n\
         \n## Task\n{prompt}\n\
         {resources}\
         {data}\
         \n## Reasoning Approach\n\
         Think step-by-step:\n\
         1. Analyze what you already know and what you still need\n\
         2. Call the most relevant tool to gather missing data\n\
         3. After each result, assess if you have enough information\n\
         4. When confident, provide your final analysis\n\
         \nAlways explain your reasoning BEFORE calling a tool.\n\
         \nFinal output (after tool calls):\n\
         ```json\n\
         {{\n  \"situation_analysis\": \"...\",\n  \"conclusion\": \"...\",\n  \"confidence\": 0.8\n}}\n\
         ```",
        name = agent.name,
        time = time_ctx,
        prompt = agent.user_prompt,
        resources = resource_info,
        data = current_data_section,
    )
} else {
    // 原有标准模式提示词保持不变
    format!(
        "You are an intelligent IoT agent named '{}' monitoring edge devices.\n\
         Current time: {}\n\
         Your task: {}\n{}{}\
         \nYou have access to tools for querying metrics, executing commands, and sending notifications. \
         **Always use tools to fetch the latest data before making conclusions.**\n\
         When done, provide your analysis and conclusion as plain text WITHOUT tool calls.\n\n\
         Output format for your final response (after tool calls, if any):\n\
         ```json\n\
         {{\n  \"situation_analysis\": \"...\",\n  \"conclusion\": \"...\",\n  \"confidence\": 0.8\n}}\n\
         ```",
        agent.name,
        time_ctx,
        agent.user_prompt,
        resource_info,
        current_data_section,
    )
};
```

关键设计:
- ReAct prompt 的核心差异是 "Always explain your reasoning BEFORE calling a tool"，引导 LLM 输出 thought 文本
- 标准 prompt 完全保留，不影响现有行为
- 两者都使用相同的 function calling 机制

- [ ] **Step 2: 验证编译**

Run: `cargo check -p neomind-agent 2>&1 | tail -10`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "feat(agent): add ReAct system prompt for step-by-step reasoning"
```

---

## Task 3: 增强 execute_with_tools 循环 — Thought/Action/Observation 追踪

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs:1288-1416`

这是核心改动。在工具执行循环中收集 LLM 的推理文本，并在最终构建 reasoning_steps 时分类为 thought/action/observation。

### 实测对照

当前行为（2026-04-02 实测）:
```
Step 1: "Executed tool 'device'" — step_type: "tool_call"
Step 2: "Executed tool 'device'" — step_type: "tool_call"
...全是 "tool_call"，看不到 LLM 的思考过程
```

目标行为:
```
Step 1: "I need to check what devices are available first..." — step_type: "thought"
Step 2: "List devices" — step_type: "action"
Step 3: "Found 2 NE101 cameras. Let me check battery levels..." — step_type: "observation" (tool result)
Step 4: "TEST device battery is only 41%, that's concerning..." — step_type: "thought"
Step 5: "Query battery for device 4t1vcbefzk" — step_type: "action"
Step 6: "Battery: 41%, Voltage: 4590mV" — step_type: "observation"
...
```

- [ ] **Step 1: 在循环前声明 thought 收集器**

在 `let mut all_tool_results` 声明附近（约 line 1288），添加：

```rust
let mut react_thoughts: Vec<String> = Vec::new();
```

- [ ] **Step 2: 在循环中收集 LLM 推理文本**

在 `parse_tool_calls` 之后（约 line 1320-1326），利用 `remaining_text`（LLM 响应中非工具调用部分的文本）：

```rust
// 修改前 (line 1323-1326):
if tool_calls.is_empty() {
    final_text = remaining_text;
    break;
}

// 修改后:
if tool_calls.is_empty() {
    final_text = remaining_text.clone();
    // 在 ReAct 模式下，最后一轮的推理也是 thought
    if is_react_mode && !remaining_text.trim().is_empty() {
        react_thoughts.push(remaining_text.chars().take(500).collect());
    }
    break;
}

// ReAct 模式: 记录 LLM 在工具调用前的推理文本
if is_react_mode && !remaining_text.trim().is_empty() {
    react_thoughts.push(remaining_text.chars().take(500).collect());
}
```

关键: `remaining_text` 是 `parse_tool_calls` 返回的非工具调用文本。当 LLM 先输出推理文字再输出工具调用时（ReAct 行为），这里就是 "thought"。

- [ ] **Step 3: 截断工具响应（context 效率优化）**

在工具执行结果处理处（约 line 1373），添加截断：

```rust
// 修改前 (line 1373-1377):
let result_text = match &result.result {
    Ok(output) => serde_json::to_string_pretty(&output.data)
        .unwrap_or_else(|_| "Success".to_string()),
    Err(e) => format!("Error: {}", e),
};

// 修改后:
let result_text = match &result.result {
    Ok(output) => {
        let json = serde_json::to_string_pretty(&output.data)
            .unwrap_or_else(|_| "Success".to_string());
        // 截断过长的工具响应以节省 context (Anthropic 最佳实践)
        if json.len() > 2000 {
            format!("{}...\n[truncated, {} chars total]", &json[..2000], json.len())
        } else {
            json
        }
    }
    Err(e) => format!("Error: {}", e),
};
```

实测对照: 当前 `list_devices` 工具返回完整 metric schema（~800 tokens），截断后只需 ~200 tokens。

- [ ] **Step 4: 重构 reasoning_steps 构建**

将当前（约 line 1396-1416）的工具结果 → reasoning_steps 逻辑，替换为 ReAct 感知的版本：

```rust
// 修改前:
let reasoning_steps: Vec<ReasoningStep> = all_tool_results
    .iter()
    .enumerate()
    .map(|(i, r)| {
        let (desc, conf) = match &r.result { ... };
        ReasoningStep {
            step_number: (i + 1) as u32,
            description: desc,
            step_type: "tool_call".to_string(),
            input: None,
            output: String::new(),
            confidence: conf,
        }
    })
    .collect();

// 修改后:
let mut reasoning_steps: Vec<ReasoningStep> = Vec::new();
let mut step_counter: u32 = 0;

if is_react_mode {
    // ReAct 模式: 按 thought → action → observation 分组
    let tool_count = all_tool_results.len();
    let thought_count = react_thoughts.len();

    for (round_idx, thought) in react_thoughts.iter().enumerate() {
        // Thought step — LLM 的推理过程
        step_counter += 1;
        reasoning_steps.push(ReasoningStep {
            step_number: step_counter,
            description: thought.clone(),
            step_type: "thought".to_string(),
            input: None,
            output: String::new(),
            confidence: 0.8,
        });

        // 对应的 Action + Observation（如果该 thought 后面有工具调用）
        if round_idx < tool_count {
            let r = &all_tool_results[round_idx];

            // Action step
            step_counter += 1;
            let action_conf = match &r.result {
                Ok(o) if o.success => 0.9f32,
                _ => 0.3f32,
            };
            reasoning_steps.push(ReasoningStep {
                step_number: step_counter,
                description: format!("Call tool '{}'", r.name),
                step_type: "action".to_string(),
                input: None,
                output: String::new(),
                confidence: action_conf,
            });

            // Observation step — 工具返回结果
            step_counter += 1;
            let obs_text = match &r.result {
                Ok(output) => {
                    let json = serde_json::to_string_pretty(&output.data)
                        .unwrap_or_default();
                    if json.len() > 500 {
                        format!("{}...", &json[..500])
                    } else {
                        json
                    }
                }
                Err(e) => format!("Error: {}", e),
            };
            reasoning_steps.push(ReasoningStep {
                step_number: step_counter,
                description: format!("Result from '{}'", r.name),
                step_type: "observation".to_string(),
                input: None,
                output: obs_text,
                confidence: action_conf,
            });
        }
    }
} else {
    // 标准模式: 保持原有行为
    for (i, r) in all_tool_results.iter().enumerate() {
        let (desc, conf) = match &r.result {
            Ok(output) => (
                format!("Executed tool '{}'", r.name),
                if output.success { 0.9 } else { 0.3 },
            ),
            Err(e) => (format!("Tool '{}' failed: {}", r.name, e), 0.2),
        };
        reasoning_steps.push(ReasoningStep {
            step_number: (i + 1) as u32,
            description: desc,
            step_type: "tool_call".to_string(),
            input: None,
            output: String::new(),
            confidence: conf,
        });
    }
}
```

- [ ] **Step 5: 验证编译**

Run: `cargo check -p neomind-agent 2>&1 | tail -10`

- [ ] **Step 6: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "feat(agent): enhance execute_with_tools with thought/action/observation tracking"
```

---

## Task 4: API 层支持 execution_mode

**Files:**
- Modify: `crates/neomind-api/src/handlers/agents.rs`

- [ ] **Step 1: 在 CreateAgentRequest 和 UpdateAgentRequest 添加字段**

两个 DTO 中（约 line 386 和 line 467），在 `context_window_size` 之后添加：

```rust
/// Execution mode: "standard" or "react"
pub execution_mode: Option<String>,
```

- [ ] **Step 2: 在创建和更新逻辑中使用该字段**

`create_agent` handler（约 line 954）添加：
```rust
execution_mode: request.execution_mode.unwrap_or_else(|| "standard".to_string()),
```

`update_agent` handler（约 line 1158）添加：
```rust
if let Some(mode) = request.execution_mode {
    agent.execution_mode = mode;
}
```

- [ ] **Step 3: 在 Response DTO 中添加字段**

AgentResponse DTOs（约 line 117, 150）添加：
```rust
pub execution_mode: Option<String>,
```

构建 response 处（约 line 538, 675）添加：
```rust
execution_mode: Some(agent.execution_mode.clone()),
```

- [ ] **Step 4: 验证编译**

Run: `cargo check -p neomind-api 2>&1 | tail -10`

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-api/src/handlers/agents.rs
git commit -m "feat(api): support execution_mode in agent CRUD"
```

---

## Task 5: 前端 ReAct 步骤可视化

**Files:**
- Modify: `web/src/pages/agents-components/AgentThinkingPanel.tsx`
- Modify: `web/src/i18n/locales/en/agents.json`
- Modify: `web/src/i18n/locales/zh/agents.json`

- [ ] **Step 1: 新增 ReAct 步骤类型颜色**

在 `AgentThinkingPanel.tsx` 的 `getStepTypeColor` 函数中（line 172），在 `default` 之前添加：

```typescript
case 'thought':
  return 'text-purple-500 bg-purple-500/10 border-purple-500/20'
case 'action':
  return 'text-blue-500 bg-blue-500/10 border-blue-500/20'
case 'observation':
  return 'text-green-500 bg-green-500/10 border-green-500/20'
case 'tool_call':
  return 'text-teal-500 bg-teal-500/10 border-teal-500/20'
```

- [ ] **Step 2: 添加 i18n**

`en/agents.json`:
```json
{
  "executionMode": "Execution Mode",
  "executionModeStandard": "Standard (Single Analysis)",
  "executionModeReact": "ReAct (Step-by-Step Reasoning)",
  "executionModeDescription": "ReAct mode uses Thought→Action→Observation loops for dynamic reasoning"
}
```

`zh/agents.json`:
```json
{
  "executionMode": "执行模式",
  "executionModeStandard": "标准模式（单次分析）",
  "executionModeReact": "ReAct 模式（逐步推理）",
  "executionModeDescription": "ReAct 模式使用 思考→行动→观察 循环进行动态推理"
}
```

- [ ] **Step 3: 验证前端构建**

Run: `cd web && npm run build 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/agents-components/AgentThinkingPanel.tsx web/src/i18n/
git commit -m "feat(web): add ReAct step type colors and execution mode i18n"
```

---

## Task 6: 前端 Agent 配置 UI

**Files:**
- Modify: agent 配置表单组件（需确认具体文件）

- [ ] **Step 1: 在 agent 配置表单中添加执行模式选择器**

在 agent 创建/编辑表单中，tool 配置区域附近添加：

```tsx
<SelectField
  label={t('agents:executionMode')}
  value={form.execution_mode || 'standard'}
  onChange={(v) => setForm(prev => ({ ...prev, execution_mode: v }))}
  options={[
    { value: 'standard', label: t('agents:executionModeStandard') },
    { value: 'react', label: t('agents:executionModeReact') },
  ]}
  description={t('agents:executionModeDescription')}
/>
```

- [ ] **Step 2: 验证前端构建**

Run: `cd web && npm run build 2>&1 | tail -10`

- [ ] **Step 3: Commit**

```bash
git add web/src/
git commit -m "feat(web): add execution mode selector in agent config"
```

---

## Task 7: 编译验证和测试

- [ ] **Step 1: cargo check**
- [ ] **Step 2: cargo clippy -p neomind-agent -p neomind-api**
- [ ] **Step 3: cargo test -p neomind-agent --lib**
- [ ] **Step 4: cargo fmt**
- [ ] **Step 5: cd web && npm run build**

---

## 改动影响总结

| 改动 | 影响范围 | 向后兼容 |
|------|----------|----------|
| `execution_mode` 字段 | AiAgent struct | ✅ `#[serde(default)]` |
| ReAct 系统提示词 | execute_with_tools 内部 | ✅ 仅 `execution_mode == "react"` 时生效 |
| thought/action/observation 步骤 | reasoning_steps 构建 | ✅ 仅 react 模式 |
| 工具响应截断 | execute_with_tools 循环 | ✅ 标准+react 模式都受益 |
| API DTO | 创建/更新/响应 | ✅ `Option<String>` |
| 前端步骤颜色 | AgentThinkingPanel | ✅ 新 step_type 走 default case |

## 后续优化（不在本计划范围）

1. **Data-on-demand**: ReAct 模式下跳过预采集，让 LLM 通过工具按需查询
2. **中间摘要**: 长 ReAct 对话时压缩 early rounds 的 context
3. **评估驱动**: 添加 Braintrust 风格的 agent evaluation metrics
4. **ReAct 流程图组件**: 专用可视化展示 Thought→Action→Observation 时间线

## 参考资源

- [Building Effective AI Agents - Anthropic](https://www.anthropic.com/research/building-effective-agents)
- [Writing Effective Tools for AI Agents - Anthropic Engineering](https://www.anthropic.com/engineering/writing-tools-for-agents)
- [The Canonical Agent Architecture: A While Loop With Tools - Braintrust](https://www.braintrust.dev/blog/agent-while-loop)
- [Claude Code Architecture Analysis](https://bits-bytes-nn.github.io/insights/agentic-ai/2026/03/31/claude-code-architecture-analysis.html)
- [ReAct: Synergizing Reasoning and Acting in Language Models (Yao et al., 2022)](https://arxiv.org/abs/2210.03629)
