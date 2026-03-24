# NeoMind 代码库精简设计

## 概述

本文档描述了 NeoMind 项目的代码精简计划，包括三个阶段：
1. 清理死代码
2. 统一重复 Trait
3. 合并 neomind-llm 到 neomind-agent

## 目标

- 减少代码冗余，提高可维护性
- 减少编译时间
- 保持现有功能不变

---

## 第一阶段：清理死代码

### 目标

删除所有 `#[allow(dead_code)]` 注解及对应的未使用代码。

### 当前状态

- 47 个 `#[allow(dead_code)]` 注解
- 主要分布在：
  - `neomind-core/src/extension/isolated/process.rs` (7个)
  - `neomind-agent/src/agent/mod.rs` (6个)
  - `neomind-agent/src/ai_agent/executor.rs` (4个)
  - 其他文件 (30个)

### 实施步骤

1. **移除注解**：删除所有 `#[allow(dead_code)]` 注解
2. **编译检测**：运行 `cargo check` 收集编译器警告
3. **代码清理**：删除编译器标记为未使用的：
   - 函数
   - 结构体
   - 枚举
   - trait
   - 模块
4. **验证**：运行测试确保功能正常

### 风险

- **低**：这些代码已被标记为未使用，删除风险较小
- 缓解措施：通过测试验证

---

## 第二阶段：统一重复 Trait

### 2.1 Tool Trait

**问题**：`Tool` trait 在两处定义

| 位置 | 使用情况 |
|------|----------|
| `neomind-core/src/tools/mod.rs` | 无直接实现 |
| `neomind-agent/src/toolkit/tool.rs` | 20+ 实现 |

**解决方案**：
1. 删除 `neomind-core/src/tools/mod.rs` 中的 `Tool` trait 定义
2. 保留 `neomind-agent/src/toolkit/tool.rs` 中的版本
3. 如需在 core 中引用，使用 re-export

**影响范围**：
- `neomind-core` 中的 `ToolFactory` 需要调整
- 可能影响依赖 core 中 Tool 的代码

### 2.2 StorageBackend Trait

**问题**：`StorageBackend` trait 在两处定义

| 位置 | 使用情况 |
|------|----------|
| `neomind-core/src/storage/mod.rs` | 被 neomind-storage 使用 |
| `neomind-storage/src/backend.rs` | 未被使用（完全重复） |

**解决方案**：
1. 删除 `neomind-storage/src/backend.rs` 中的 `StorageBackend` 定义
2. 保留 `neomind-core/src/storage/mod.rs` 中的版本
3. 确保 `neomind-storage/src/lib.rs` 正确 re-export

**影响范围**：
- 仅影响 `neomind-storage` 内部

### 2.3 ExtensionRegistryTrait

**问题**：`ExtensionRegistryTrait` 在同一 crate 的两处定义

| 位置 | 使用情况 |
|------|----------|
| `neomind-core/src/extension/registry.rs` | 有 1 个实现 |
| `neomind-core/src/extension/system.rs` | 无实现 |

**解决方案**：
1. 删除 `neomind-core/src/extension/system.rs` 中的 trait 定义
2. 保留 `neomind-core/src/extension/registry.rs` 中的版本
3. 确保 `system.rs` 中的其他代码正常工作

**影响范围**：
- 仅影响 `neomind-core` 内部

---

## 第三阶段：合并 neomind-llm 到 neomind-agent

### 当前状态

```
crates/
├── neomind-llm/           # 要合并
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── backends/
│       │   ├── mod.rs
│       │   ├── ollama.rs
│       │   └── openai.rs
│       └── ...
└── neomind-agent/         # 合并目标
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── llm.rs         # 已有一个 llm 模块
        └── ...
```

### 合并方案

1. **移动文件**
   - `neomind-llm/src/backends/` → `neomind-agent/src/llm/backends/`
   - `neomind-llm/src/` 其他文件 → `neomind-agent/src/llm/`

2. **处理冲突**
   - `neomind-agent/src/llm.rs` 已存在，需要与新内容合并
   - 将 `llm.rs` 重构为 `llm/mod.rs`，然后添加 backends 子模块

3. **更新依赖**
   - 合并 `neomind-llm/Cargo.toml` 的依赖到 `neomind-agent/Cargo.toml`
   - 更新 workspace `Cargo.toml`，移除 `neomind-llm`
   - 更新所有 `use neomind_llm::` 为 `use neomind_agent::llm::`

4. **受影响的 crate**
   - `neomind-api` - 可能直接依赖 neomind-llm
   - `neomind-core` - 可能通过 re-export 引用

### 依赖关系图

```
neomind-llm ─────┐
                 ▼
neomind-agent ◄──┤
                 │
neomind-api ─────┤ (可能直接依赖 llm)
                 │
neomind-core ────┘ (可能有 llm traits)
```

合并后：
```
neomind-agent (包含 llm)
      ▲
      │
neomind-api
      │
neomind-core
```

### 实施步骤

1. **准备工作**
   - 确认所有 neomind-llm 的公共 API
   - 检查其他 crate 对 neomind-llm 的直接依赖

2. **合并代码**
   - 在 `neomind-agent/src/` 下创建 `llm/` 目录结构
   - 移动并调整源文件

3. **更新依赖**
   - 修改 `neomind-agent/Cargo.toml`
   - 修改 workspace `Cargo.toml`
   - 更新其他 crate 的 import 语句

4. **清理**
   - 删除 `crates/neomind-llm/` 目录
   - 运行 `cargo check` 确保编译通过
   - 运行测试确保功能正常

---

## 预期结果

| 指标 | 当前 | 合并后 |
|------|------|--------|
| Crate 数量 | 12 | 11 |
| 死代码注解 | 47 | 0 |
| 重复 Trait | 3 | 0 |
| 预计减少代码 | - | 500-1000 行 |

---

## 执行顺序

1. ✅ 第一阶段：清理死代码
2. ✅ 第二阶段：统一重复 Trait
3. ✅ 第三阶段：合并 neomind-llm

每个阶段完成后运行完整测试，确保功能正常后再进入下一阶段。

---

## 风险评估

| 风险 | 级别 | 缓解措施 |
|------|------|----------|
| 删除有用代码 | 低 | 通过编译器和测试验证 |
| 合并导致循环依赖 | 中 | 仔细规划模块结构 |
| API 变更影响外部代码 | 低 | 保持公共 API 兼容 |

---

## 验证清单

- [ ] `cargo check` 无错误
- [ ] `cargo test` 全部通过
- [ ] `cargo clippy` 无新警告
- [ ] 服务可以正常启动
- [ ] 基本功能测试通过
