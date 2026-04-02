# NeoMind ReAct Agent Mode 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在现有 Agent 架构基础上，将执行模式从"先全量采集→单次分析"升级为 ReAct (Reasoning + Acting) 模式，实现 Thought → Action → Observation 迭代循环，让 Agent 具备动态推理和自适应决策能力。

**Architecture:** 基于 `execute_with_tools` 已有的工具调用循环，重构为显式的 ReAct 三阶段循环。新增 `ReActStep` 追踪结构，保留向后兼容。前端通过现有 `AgentThinking` 事件实时展示 Thought/Action/Observation 步骤。

**Tech Stack:** Rust, serde, LLM function calling, NeoMind EventBus, React + TypeScript

---

## 现状分析

### 当前执行模式 vs ReAct

```
当前模式（先采集后分析）:
  1. collect_data() → 全量采集所有数据源
  2. analyze_with_llm() → 单次 LLM 调用，一次性输出所有 reasoning_steps + decisions
  3. execute_decisions() → 执行决策
  4. [可选] execute_with_tools() → 工具调用循环（已有多轮能力）

ReAct 模式（边思考边行动）:
  1. 初始上下文 → LLM 生成 Thought（"我需要检查设备A的温度"）
  2. LLM 选择 Action → 调用 get_device_metric 工具
  3. Observation → 工具返回 "温度 85°C"
  4. LLM 基于 Observation 生成新 Thought（"温度偏高，需要检查原因"）
  5. LLM 选择新 Action → 调用 get_device_metric 查看其他指标
  6. ... 循环直到 LLM 认为任务完成
  7. 输出最终 conclusion + decisions
```

### 已有基础设施（可复用）

| 组件 | 位置 | 状态 |
|------|------|------|
| 工具调用循环 | `execute_with_tools` (executor.rs:1066) | ✅ 已有 LLM→Tool→LLM 多轮循环 |
| 工具注册表 | `toolkit/registry.rs` | ✅ 完整的工具注册/执行 |
| 工具调用解析 | `agent/tool_parser.rs` | ✅ 支持 JSON/XML 多格式 |
| Chaining 循环 | `execute_with_chaining` (executor.rs:2849) | ✅ 多轮执行 + 上下文累积 |
| ReasoningStep | `neomind-storage/src/agents.rs:840` | ✅ 灵活的 step_type 字段 |
| 事件系统 | `NeoMindEvent::AgentThinking` | ✅ 实时推送推理步骤 |
| 前端展示 | `AgentThinkingPanel.tsx` | ✅ 实时显示推理过程 |
| 内存系统 | working/short-term/long-term | ✅ 跨执行记忆 |

### 需要新增/修改的部分

| 组件 | 改动类型 | 影响范围 |
|------|----------|----------|
| 执行模式选择 | 新增逻辑 | executor.rs |
| ReAct 系统提示词 | 新增 | executor.rs |
| ReActStep 数据类型 | 新增 | neomind-storage |
| ReasoningStep.step_type 扩展 | 兼容扩展 | 无破坏性变更 |
| AgentThinking 事件扩展 | 兼容扩展 | 新增字段 |
| 前端 ReAct 步骤可视化 | 增强 | React 组件 |
| IoT 专用工具定义 | 新增 | toolkit/ |

---

## 数据结构影响评估

### 1. 现有结构（无需修改）

```rust
// ReasoningStep - 现有结构完全够用
pub struct ReasoningStep {
    pub step_number: u32,
    pub description: String,        // ← Thought/Action/Observation 的内容
    pub step_type: String,          // ← 扩展: "thought" | "action" | "observation"
    pub input: Option<String>,      // ← Action 的参数
    pub output: String,             // ← Observation 的结果
    pub confidence: f32,
}

// Decision - 无需修改
// DecisionProcess - 无需修改
// ConversationTurn - 无需修改
// TurnOutput - 无需修改
```

**结论**: 所有现有存储结构保持不变。`step_type` 字段已经是 String 类型，天然支持新值。

### 2. 需要新增的结构

```rust
// 新增: ReAct 执行轨迹记录（存储在 DecisionProcess.reasoning_steps 中）
// 不需要新的数据库表，复用现有结构
```

### 3. 前端类型影响

```typescript
// 现有 TypeScript 类型 - 无需修改
interface ReasoningStep {
  step_number: number
  description: string
  step_type: string    // ← 新增 "thought" | "action" | "observation" 值
  input?: string
  output: string
  confidence: number
}
```

**结论**: 前端类型无需修改，只是 `step_type` 有了新值。

---

## 前端改造评估

### 需要的改动

#### 1. AgentThinkingPanel.tsx — 增强步骤类型显示（小改）

当前按 `step_type` 着色：
```typescript
// 当前
const stepColors: Record<string, string> = {
  analysis: 'blue',
  evaluation: 'orange',
  planning: 'purple',
  execution: 'green',
  tool_call: 'teal',
  llm_analysis: 'indigo',
};
```

需要新增 ReAct 步骤类型的颜色/图标：
```typescript
// 新增
thought: 'purple',      // 思考 - 紫色
action: 'blue',         // 行动 - 蓝色
observation: 'green',   // 观察 - 绿色
```

#### 2. AgentExecutionTimeline.tsx — 无需修改

现有的展开式展示已经能显示所有 reasoning_steps，新增的 step_type 会自动被渲染。

#### 3. 可选增强：ReAct 循环可视化

如果想要更直观的 Thought→Action→Observation 时间线，可以新增一个小组件。但这不是必须的——现有的步骤列表已经足够清晰。

### 前端改动总结

| 文件 | 改动量 | 说明 |
|------|--------|------|
| `AgentThinkingPanel.tsx` | ~10 行 | 新增 ReAct step_type 的颜色映射 |
| `useAgentEvents.ts` | 无 | 现有事件处理已足够 |
| 新增 `ReActFlowView.tsx` | ~100 行 | 可选增强：专用 ReAct 流程图 |

---

## 实施任务

### Task 1: 定义 ReAct 执行模式常量和 IoT 专用工具

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs`
- Modify: `crates/neomind-agent/src/agent/tool_parser.rs` (如需)

- [ ] **Step 1: 在 executor.rs 中定义 ReAct 模式常量**

在文件顶部常量区域添加：
```rust
/// ReAct step types for structured Thought-Action-Observation loop
const REACT_STEP_THOUGHT: &str = "thought";
const REACT_STEP_ACTION: &str = "action";
const REACT_STEP_OBSERVATION: &str = "observation";
```

- [ ] **Step 2: 在 execute_with_tools 或新函数中添加 IoT 工具定义**

确保以下工具在工具注册时可用（检查现有工具，按需补充）：
- `get_device_data` — 获取设备最新数据（已有 device 工具）
- `get_device_history` — 获取设备历史数据
- `send_alert` — 发送告警（已有 alert 工具）
- `execute_command` — 执行设备命令
- `query_rule_status` — 查询规则状态

- [ ] **Step 3: 验证编译**

Run: `cargo check -p neomind-agent`

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "feat(agent): add ReAct step type constants and IoT tool definitions"
```

---

### Task 2: 实现 ReAct 系统提示词

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs`

- [ ] **Step 1: 新增 ReAct 系统提示词构建函数**

在 `analyze_with_llm` 附近新增函数 `build_react_system_prompt`：

```rust
/// Build system prompt for ReAct (Thought-Action-Observation) mode
fn build_react_system_prompt(
    agent: &AiAgent,
    data_summary: &str,
    tools_json: &str,
    is_chinese: bool,
) -> String {
    let time_context = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    if is_chinese {
        format!(
            r#"你是 '{agent_name}'，一个物联网自动化助手。当前时间: {time}

## 你的任务
{user_prompt}

## 可用数据
{data_summary}

## 可用工具
{tools_json}

## ReAct 工作模式
你必须严格遵循 Thought → Action → Observation 循环：

1. **Thought**: 分析当前已知信息，判断还需要什么数据或应该采取什么行动
2. **Action**: 调用一个工具来获取数据或执行操作
3. **Observation**: 系统返回工具执行结果

重复以上循环，直到你有足够信息做出最终判断。

## 输出格式
每次回复你必须输出：
- Thought: 你的推理过程（必填）
- 然后选择以下之一：
  - 调用一个工具（Action）
  - 或输出最终 JSON 结论（当你认为分析完成时）

最终结论格式:
```json
{{
  "situation_analysis": "完整分析",
  "conclusion": "结论",
  "confidence": 0.85,
  "decisions": [{{"decision_type": "alert|command|info", "description": "描述", "action": "动作", "rationale": "理由"}}]
}}
```"#,
            agent_name = agent.name,
            user_prompt = agent.user_prompt,
        )
    } else {
        format!(
            r#"You are '{agent_name}', an IoT automation assistant. Current time: {time}

## Your Task
{user_prompt}

## Available Data
{data_summary}

## Available Tools
{tools_json}

## ReAct Mode
You must follow the Thought → Action → Observation loop:

1. **Thought**: Analyze what you know and what you still need
2. **Action**: Call a tool to gather data or execute an action
3. **Observation**: System returns the tool result

Repeat until you have enough information for a final conclusion.

## Output Format
Each response must include:
- Thought: Your reasoning (required)
- Then either:
  - A tool call (Action)
  - Or a final JSON conclusion (when analysis is complete)

Final conclusion format:
```json
{{
  "situation_analysis": "Full analysis",
  "conclusion": "Conclusion",
  "confidence": 0.85,
  "decisions": [{{"decision_type": "alert|command|info", "description": "desc", "action": "action", "rationale": "reason"}}]
}}
```"#,
            agent_name = agent.name,
            user_prompt = agent.user_prompt,
        )
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo check -p neomind-agent`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "feat(agent): add ReAct system prompt builder for IoT agents"
```

---

### Task 3: 实现 ReAct 执行循环核心

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs`

- [ ] **Step 1: 新增 `execute_with_react` 函数**

这是核心改动。在 `execute_with_tools` 基础上，新增一个显式追踪 Thought/Action/Observation 的执行函数：

```rust
/// Execute agent using ReAct (Reasoning + Acting) pattern.
/// Iterates Thought → Action → Observation loop until LLM concludes.
async fn execute_with_react(
    &self,
    agent: &AiAgent,
    initial_data: &[DataCollected],
    llm_runtime: Arc<dyn LlmRuntime>,
    execution_id: &str,
) -> AgentResult<(String, Vec<neomind_storage::ReasoningStep>, Vec<neomind_storage::Decision>, String)> {
    let max_rounds = agent.max_chain_depth.max(3).min(10);
    let is_chinese = SemanticToolMapper::detect_language(&agent.user_prompt)
        .map(|l| matches!(l, crate::agent::semantic_mapper::Language::Chinese | crate::agent::semantic_mapper::Language::Mixed));

    // Get available tools
    let registry = self.tool_registry.as_ref()
        .ok_or_else(|| NeoMindError::Tool("Tool registry not available for ReAct mode".to_string()))?;
    let tool_defs = registry.definitions_json();

    // Build initial data summary
    let data_summary = Self::build_data_summary(initial_data);

    // Build system prompt
    let system_prompt = Self::build_react_system_prompt(agent, &data_summary, &tool_defs, is_chinese);

    // Build conversation messages
    let mut messages = vec![Message::system(&system_prompt)];

    // Add initial data as first user message
    let initial_context = if is_chinese {
        format!("当前采集到的数据:\n{}", data_summary)
    } else {
        format!("Current collected data:\n{}", data_summary)
    };
    messages.push(Message::user(&initial_context));

    let mut all_reasoning_steps: Vec<neomind_storage::ReasoningStep> = Vec::new();
    let mut step_counter: u32 = 0;
    let mut final_analysis = String::new();
    let mut final_conclusion = String::new();
    let mut final_confidence: f32 = 0.7;
    let mut final_decisions: Vec<neomind_storage::Decision> = Vec::new();

    for round in 0..max_rounds {
        // LLM call
        let input = LlmInput {
            messages: messages.clone(),
            tools: Some(registry.tool_definitions()),
            ..Default::default()
        };

        let output = llm_runtime.generate(input).await
            .map_err(|e| NeoMindError::Llm(format!("ReAct LLM call failed: {}", e)))?;

        let response_text = output.text.trim().to_string();

        // Parse tool calls from response
        let (remaining_text, tool_calls) = parse_tool_calls(&response_text);

        // Emit Thought step
        if !remaining_text.is_empty() {
            step_counter += 1;
            let thought_step = neomind_storage::ReasoningStep {
                step_number: step_counter,
                description: remaining_text.chars().take(500).collect(),
                step_type: REACT_STEP_THOUGHT.to_string(),
                input: None,
                output: String::new(),
                confidence: 0.8,
            };

            // Emit event for frontend
            if let Some(ref bus) = self.event_bus {
                let _ = bus.publish(NeoMindEvent::AgentThinking {
                    agent_id: agent.id.clone(),
                    execution_id: execution_id.to_string(),
                    step_number: thought_step.step_number,
                    step_type: REACT_STEP_THOUGHT.to_string(),
                    description: thought_step.description.clone(),
                    details: None,
                    timestamp: chrono::Utc::now().timestamp(),
                }).await;
            }

            all_reasoning_steps.push(thought_step);
        }

        // Check if LLM produced a final conclusion (no tool calls)
        if tool_calls.is_empty() {
            // Try to parse final JSON conclusion
            if let Some((analysis, conclusion, decisions, confidence)) =
                Self::parse_react_final_response(&response_text, is_chinese) {
                final_analysis = analysis;
                final_conclusion = conclusion;
                final_decisions = decisions;
                final_confidence = confidence;
            } else {
                // Fallback: use raw text as conclusion
                final_analysis = response_text.chars().take(500).collect();
                final_conclusion = final_analysis.clone();
            }
            break;
        }

        // Execute tool calls and record Action + Observation steps
        // Add assistant message to conversation
        messages.push(Message::assistant(&response_text));

        let results = registry.execute_parallel(tool_calls.clone()).await;

        for result in &results {
            // Action step
            step_counter += 1;
            let action_step = neomind_storage::ReasoningStep {
                step_number: step_counter,
                description: format!("Execute tool '{}'", result.name),
                step_type: REACT_STEP_ACTION.to_string(),
                input: Some(serde_json::to_string(&tool_calls.iter()
                    .find(|tc| tc.name == result.name)
                    .map(|tc| &tc.args)).unwrap_or_default()),
                output: String::new(),
                confidence: 0.9,
            };

            if let Some(ref bus) = self.event_bus {
                let _ = bus.publish(NeoMindEvent::AgentThinking {
                    agent_id: agent.id.clone(),
                    execution_id: execution_id.to_string(),
                    step_number: action_step.step_number,
                    step_type: REACT_STEP_ACTION.to_string(),
                    description: action_step.description.clone(),
                    details: Some(serde_json::to_value(&result.result).unwrap_or_default()),
                    timestamp: chrono::Utc::now().timestamp(),
                }).await;
            }

            all_reasoning_steps.push(action_step);

            // Observation step
            step_counter += 1;
            let obs_text = match &result.result {
                Ok(output) => serde_json::to_string_pretty(&output.data)
                    .unwrap_or_default()
                    .chars().take(500).collect(),
                Err(e) => format!("Error: {}", e),
            };

            let obs_step = neomind_storage::ReasoningStep {
                step_number: step_counter,
                description: format!("Tool '{}' result", result.name),
                step_type: REACT_STEP_OBSERVATION.to_string(),
                input: None,
                output: obs_text.clone(),
                confidence: 0.9,
            };

            if let Some(ref bus) = self.event_bus {
                let _ = bus.publish(NeoMindEvent::AgentThinking {
                    agent_id: agent.id.clone(),
                    execution_id: execution_id.to_string(),
                    step_number: obs_step.step_number,
                    step_type: REACT_STEP_OBSERVATION.to_string(),
                    description: obs_step.description.clone(),
                    details: Some(serde_json::Value::String(obs_text.clone())),
                    timestamp: chrono::Utc::now().timestamp(),
                }).await;
            }

            all_reasoning_steps.push(obs_step);

            // Feed result back to LLM conversation
            messages.push(Message::user(&format!(
                "Observation from '{}':\n{}",
                result.name, obs_text
            )));
        }

        // If last round, use whatever we have
        if round == max_rounds - 1 {
            final_analysis = "ReAct loop reached maximum iterations".to_string();
            final_conclusion = "Analysis incomplete - max rounds reached".to_string();
            final_confidence = 0.5;
        }
    }

    // If no steps were produced, create a fallback
    if all_reasoning_steps.is_empty() {
        all_reasoning_steps.push(neomind_storage::ReasoningStep {
            step_number: 1,
            description: "ReAct analysis completed".to_string(),
            step_type: "llm_analysis".to_string(),
            input: Some(format!("{} data sources", initial_data.len())),
            output: final_analysis.chars().take(200).collect(),
            confidence: final_confidence,
        });
    }

    Ok((final_analysis, all_reasoning_steps, final_decisions, final_conclusion))
}
```

注意：以上代码是伪代码级别的详细设计，实际实现时需要：
- 适配 `LlmInput` / `Message` 的实际 API
- 适配 `ToolRegistry::execute_parallel` 的实际签名
- 添加 `parse_react_final_response` 辅助函数
- 添加 `build_data_summary` 辅助函数

- [ ] **Step 2: 验证编译**

Run: `cargo check -p neomind-agent`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "feat(agent): implement ReAct execution loop core"
```

---

### Task 4: 集成 ReAct 模式到执行流程

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs`
- Modify: `crates/neomind-storage/src/agents.rs` (添加 execution_mode 字段)

- [ ] **Step 1: 在 AiAgent 中添加执行模式配置**

在 `crates/neomind-storage/src/agents.rs` 的 `AiAgent` 结构中添加：
```rust
/// Agent execution mode: "standard" (default) or "react"
#[serde(default = "default_execution_mode")]
pub execution_mode: String,
```

辅助函数：
```rust
fn default_execution_mode() -> String {
    "standard".to_string()
}
```

- [ ] **Step 2: 在 execute_internal 中添加模式路由**

在 `execute_internal` 函数中，在 `analyze_situation_with_intent` 调用之前添加模式判断：

```rust
// Route to ReAct mode if configured
if agent.execution_mode == "react" && self.tool_registry.is_some() {
    if let Some(ref llm) = llm_backend {
        match self.execute_with_react(agent, &data_collected, llm.clone(), &context.execution_id).await {
            Ok((analysis, steps, decisions, conclusion)) => {
                // Build DecisionProcess from ReAct result
                decision_process = DecisionProcess {
                    situation_analysis: analysis,
                    data_collected: data_collected.clone(),
                    reasoning_steps: steps,
                    decisions,
                    conclusion,
                    confidence: /* from steps */,
                };
                // Skip standard analysis
                return Ok(/* build execution result */);
            }
            Err(e) => {
                tracing::warn!(error = %e, "ReAct mode failed, falling back to standard");
                // Fall through to standard mode
            }
        }
    }
}
```

- [ ] **Step 3: 验证编译和测试**

Run: `cargo check -p neomind-agent && cargo test -p neomind-agent --lib`

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs crates/neomind-storage/src/agents.rs
git commit -m "feat(agent): integrate ReAct mode into execution flow with fallback"
```

---

### Task 5: 前端 ReAct 步骤可视化增强

**Files:**
- Modify: `web/src/pages/agents-components/AgentThinkingPanel.tsx`

- [ ] **Step 1: 新增 ReAct 步骤类型样式**

在 AgentThinkingPanel 中找到步骤类型颜色映射，添加 ReAct 类型：

```typescript
// 新增 ReAct 步骤类型的颜色和图标
const stepTypeConfig: Record<string, { color: string; icon: string; label: string }> = {
  // ... 现有类型保持不变 ...
  thought: { color: 'purple', icon: 'Lightbulb', label: '思考' },      // 思考
  action: { color: 'blue', icon: 'Zap', label: '行动' },              // 行动
  observation: { color: 'green', icon: 'Eye', label: '观察' },        // 观察
};
```

- [ ] **Step 2: 为 ReAct 步骤添加分组视觉分隔**

在步骤列表中，当检测到连续的 thought→action→observation 序列时，用视觉分组（如竖线连接或卡片边框）展示 ReAct 循环。这是可选增强，最简方案只添加颜色区分即可。

- [ ] **Step 3: 验证前端构建**

Run: `cd web && npm run build`

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/agents-components/AgentThinkingPanel.tsx
git commit -m "feat(web): add ReAct step type visualization in thinking panel"
```

---

### Task 6: API 层适配

**Files:**
- Modify: `crates/neomind-api/src/handlers/agents.rs`

- [ ] **Step 1: 在 Agent 创建/更新 API 中支持 execution_mode**

在 agent 创建和更新的 DTO 中添加 `execution_mode` 字段：
```rust
// 在 CreateAgentRequest / UpdateAgentRequest 中
pub execution_mode: Option<String>, // "standard" | "react"
```

- [ ] **Step 2: 验证 API 文档**

Run: `cargo check -p neomind-api`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-api/src/handlers/agents.rs
git commit -m "feat(api): support execution_mode field in agent CRUD"
```

---

### Task 7: 前端 Agent 设置界面

**Files:**
- Modify: `web/src/components/agents/AgentConfigForm.tsx`（或对应的 agent 配置组件）

- [ ] **Step 1: 添加执行模式选择器**

在 agent 配置表单中添加执行模式选择：

```tsx
<SelectField
  label={t('agents:executionMode')}
  options={[
    { value: 'standard', label: t('agents:executionModeStandard') },
    { value: 'react', label: t('agents:executionModeReact') },
  ]}
  description={t('agents:executionModeDescription')}
/>
```

- [ ] **Step 2: 添加 i18n 翻译**

在 `web/src/i18n/locales/en/agents.json` 和 `zh/agents.json` 中添加：
```json
{
  "executionMode": "Execution Mode",
  "executionModeStandard": "Standard (Single Analysis)",
  "executionModeReact": "ReAct (Iterative Reasoning)",
  "executionModeDescription": "ReAct mode enables Thought→Action→Observation loops for dynamic reasoning"
}
```

- [ ] **Step 3: 验证前端构建**

Run: `cd web && npm run build`

- [ ] **Step 4: Commit**

```bash
git add web/src/components/agents/ web/src/i18n/
git commit -m "feat(web): add execution mode selector in agent config"
```

---

### Task 8: 端到端测试和验证

- [ ] **Step 1: 运行完整构建验证**

Run: `cargo check && cargo test -p neomind-agent --lib && cargo fmt -- --check`

- [ ] **Step 2: 运行前端构建**

Run: `cd web && npm run build`

- [ ] **Step 3: 手动验证**

启动服务后：
1. 创建一个 agent，设置 `execution_mode: "react"`
2. 配置数据源和工具
3. 手动触发执行
4. 在前端观察 Thought→Action→Observation 步骤
5. 验证最终 conclusion 和 decisions

- [ ] **Step 4: 最终 Commit**

```bash
git add -A
git commit -m "feat(agent): complete ReAct agent mode implementation"
```

---

## 风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| LLM 不遵循 ReAct 格式 | 中 | few-shot 示例 + 解析容错 + 回退到标准模式 |
| Token 消耗增加 | 中 | max_rounds 限制 + 中间摘要 |
| 工具调用循环不终止 | 低 | 硬性 max_rounds 上限 (10) |
| 向后兼容性 | 低 | execution_mode 默认 "standard"，新字段 Option |
| 前端兼容性 | 低 | 新 step_type 值自动被现有组件渲染 |

## 参考资源

- [ReAct: Synergizing Reasoning and Acting in Language Models (Yao et al., 2022)](https://arxiv.org/abs/2210.03629)
- [ReAct Pattern Guide - Michael Brenndoerfer](https://mbrenndoerfer.com/writing/react-pattern-llm-reasoning-action-agents)
- [Agent Loop - AI-Girls Lab](https://ai-girls.org/en/2026/03/11/agent-loop-react-en/)
- [Braintrust Agent Evaluation Framework](https://www.braintrust.dev/articles/ai-agent-evaluation-framework)
