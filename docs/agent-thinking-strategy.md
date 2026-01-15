# Agent Thinking 传递策略

## 设计原则

Thinking 内容在系统中有**三个独立的用途**，必须严格分离：

```
┌─────────────────────────────────────────┐
│            LLM 输出流                │
│  (text, is_thinking)               │
└───────┬───────────────────────────────┘
          │
          ├─── 前端显示 (WebSocket/SSE) ✅
          │     用户看到完整的思考过程
          │
          ├─── 会话历史存储 (redb) ✅
          │     用于历史回顾、调试、故障排查
          │
          └─── 传递给下一轮 LLM 上下文 ❌
                绝对禁止！
```

## 核心原因

如果将 thinking 传递给 LLM，会导致：

### 1. 恶性循环
```
第1轮:
  User: "列出设备"
  Assistant: thinking(1000字) + content("有5个设备")
  → 保存: 完整 thinking

第2轮:
  User: "有几个规则？"
  → 传递给 LLM: [含1000字thinking的上一轮消息]
  → LLM 看到: "上一轮有thinking，我是不是也要thinking？"
  → LLM 产生: thinking(2000字)
  → 保存: 累积 3000字thinking

第3轮:
  → 传递给 LLM: [含3000字thinking]
  → LLM 产生: thinking(4000字)
  → ...
```

### 2. 处理逻辑混乱
LLM 可能误解上下文中的 thinking：
- "为什么上一轮有思考内容？"
- "我是不是应该模仿它的思考模式？"
- "这些thinking是给我的提示还是历史？"

### 3. 模型行为异常
- 模型会尝试"模仿"之前的thinking风格
- 导致thinking越来越长、越来越重复
- 失去简洁直接回答的能力

## 当前实现状态

### ✅ 已经正确实现的代码

查看 `crates/agent/src/agent/types.rs:267-417`：

```rust
pub fn to_core(&self) -> Message {
    match self.role.as_str() {
        "assistant" => {
            // ... 处理 tool_calls ...
            } else {
                // ⭐ 关键：只传递 content，不传递 thinking
                Message::assistant(&self.content)
            }
        },
        // ...
    }
}
```

**结论**：当前 `to_core()` **已经正确地不传递thinking**给LLM。

### 📊 验证测试

已添加测试 `test_thinking_not_passed_to_llm`：
```rust
let assistant_with_thinking = AgentMessage::assistant_with_thinking(
    "Answer text",
    "Detailed reasoning that should NOT reach LLM"
);

let core_msg = assistant_with_thinking.to_core();

// 验证：core_msg 只包含 content，不包含 thinking
assert_eq!(content.as_text(), "Answer text");
assert!(!content.as_text().contains("reasoning"));
```

## 关键架构点

### 1. AgentMessage 结构
```rust
pub struct AgentMessage {
    pub role: String,
    pub content: String,         // 最终答案，传递给 LLM ✅
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub tool_call_name: Option<String>,
    pub thinking: Option<String>,  // 思考过程，不传递给 LLM ❌
    pub timestamp: i64,
}
```

### 2. to_core() 方法
```rust
// 用途：将 AgentMessage 转换为 Message，用于传递给 LLM
pub fn to_core(&self) -> Message {
    // ⭐ 只传递 content 和 tool_calls
    // ⭐ thinking 字段被完全忽略
}
```

### 3. 消息流

```
模型输出 → AgentMessage
  ├─ thinking 内容 → 前端显示 ✅
  │                  → 保存到会话历史 ✅
  └─ content 内容 → 传递给 LLM 下一轮 ✅
```

## 优化建议

### 1. 确保系统提示清晰
```rust
// crates/agent/src/llm.rs:397
let prompt = format!(r#"你是NeoTalk物联网助手。

## 回答原则
1. 直接回答问题，不要冗长思考
2. 简单问题直接给出结果
3. 避免重复相同的词汇
"#);
```

### 2. 优化模型参数
```rust
// crates/agent/src/agent/mod.rs:195
let llm_config = ChatConfig {
    temperature: 0.3,      // 更确定性
    top_p: 0.7,           // 减少随机性
    max_tokens: 4096,      // 限制总长度
    // ...
};
```

### 3. 降低 thinking 显示限制
```rust
// crates/agent/src/agent/streaming.rs:51
max_thinking_length: 800,  // 前端显示限制（不影响LLM）
```

## 总结

| 用途 | 数据流 | 是否传递 |
|------|--------|---------|
| 前端显示 | thinking → WebSocket | ✅ |
| 会话存储 | AgentMessage.thinking → redb | ✅ |
| LLM 上下文 | AgentMessage.content → LLM | ✅ |
| LLM 上下文 | AgentMessage.thinking → LLM | ❌ 绝对禁止 |

**关键原则**：Thinking 只用于显示和存储，绝对不用于 LLM 上下文传递。

## 相关文件

- `crates/agent/src/agent/types.rs` - AgentMessage 和 to_core() 实现
- `crates/agent/src/agent/streaming.rs` - 流式处理和 thinking 事件
- `crates/agent/src/agent/mod.rs` - Agent 主逻辑
- `crates/agent/src/llm.rs` - LLM 接口调用
