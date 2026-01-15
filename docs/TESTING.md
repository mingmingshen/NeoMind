# NeoTalk 测试指南

本文档描述 NeoTalk 项目的测试策略、结构和运行方法。

## 测试概览

| 测试类型 | 位置 | 说明 |
|---------|------|------|
| 单元测试 | `src/*/tests.rs` 或 `tests/` | 测试单个模块/函数 |
| 集成测试 | `crates/*/tests/` | 测试多个模块协作 |
| 性能测试 | `tests/*_performance.rs` | 测试性能指标 |

## 运行测试

### 运行所有测试

```bash
cargo test --workspace
```

### 运行单个 crate 的测试

```bash
# 核心库
cargo test -p edge-ai-core

# Agent
cargo test -p edge-ai-agent

# 设备管理
cargo test -p edge-ai-devices

# API 服务器
cargo test -p edge-ai-api
```

### 运行特定测试

```bash
# 按名称过滤
cargo test -- test_tool_parser

# 按模块过滤
cargo test --package edge-ai-agent --lib
```

## 测试覆盖范围

### edge-ai-core

- [x] 事件总线 (EventBus)
- [x] 消息类型 (Message)
- [x] 工具宏 (macros)

### edge-ai-agent

- [x] Agent 配置和初始化
- [x] 流式处理 (streaming)
- [x] 工具调用解析 (tool_parser)
- [x] 会话管理 (session)

### edge-ai-devices

- [x] MQTT 设备
- [x] Modbus 设备
- [x] 设备注册表 (registry)
- [x] 设备服务 (service)

### edge-ai-tools

- [x] 工具注册表
- [x] 内置工具
- [x] 工具执行

### edge-ai-api

- [x] REST API 处理器
- [x] WebSocket 处理
- [x] 会话管理

## 测试结果

### 当前状态

```
test result: ok. 156 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

所有单元测试通过（156/156）。

### 已知问题

部分集成测试需要 Ollama 服务器运行：
- `extended_conversation` 测试
- `multi_turn_conversation` 测试
- `agent_performance_test` 测试

这些测试在 CI 环境中会被跳过，因为它们依赖外部服务。

## 添加新测试

### 单元测试示例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        let result = function_to_test();
        assert_eq!(result, expected_value);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = async_function().await.unwrap();
        assert!(result);
    }
}
```

### 集成测试示例

```rust
// crates/my_crate/tests/integration_test.rs
use edge_ai_core::EventBus;

#[tokio::test]
async fn test_integration() {
    let bus = EventBus::new();
    // 测试多个组件协作
}
```

## 测试最佳实践

1. **独立性**: 每个测试应该独立运行，不依赖其他测试
2. **清晰性**: 测试名称应清楚描述测试内容
3. **隔离**: 使用 mock/stub 避免依赖外部服务
4. **异步**: 使用 `#[tokio::test]` 进行异步测试
5. **断言**: 使用清晰的断言消息

## 性能测试

性能测试位于 `tests/agent_performance_test.rs`：

```bash
cargo test --test agent_performance_test -- --nocapture
```

## CI/CD

测试在 CI 中自动运行。确保：
- 所有 PR 都通过测试
- 新功能包含测试
- 测试覆盖率保持高水平
