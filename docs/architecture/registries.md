# Registry 职责划分

> NeoMind v0.4.2
> 创建时间: 2025-02-05

## 总览

NeoMind 使用多个 Registry 来管理不同类型的服务和组件。每个 Registry 有明确的职责边界。

---

## 用户可见的 Registry

### ExtensionRegistry (crate: neomind-core)

**用途**: 第三方开发者加载扩展

**类型**: 动态加载 (.so/.dylib/.dll/.wasm)

**生命周期**: 发现 → 加载 → 启动 → 停止 → 卸载

**API**: `/api/extensions/*`

**职责**:
- 管理扩展的生命周期
- 提供健康监控
- 处理 WASM 和 Native 扩展的加载
- 安全沙箱执行

**示例**:
```rust
use neomind_core::extension::{ExtensionRegistry, ExtensionType};

let registry = ExtensionRegistry::new();
let meta = registry.load_from_path(&path).await?;
registry.start(&meta.id).await?;
```

---

### LlmBackendRegistry (crate: neomind-llm)

**用途**: 用户配置 LLM 后端

**类型**: 配置驱动的运行时

**存储**: `data/llm_backends.redb`

**API**: `/api/llm-backends/*`

**职责**:
- 管理多个 LLM 后端实例 (Ollama, OpenAI, Anthropic, etc.)
- 处理后端激活/切换
- 提供后端健康检查
- 管理 API 密钥和连接配置

**示例**:
```rust
use neomind_llm::instance_manager::LlmBackendRegistry;

let registry = LlmBackendRegistry::new(storage.clone());
registry.create_backend(backend_config).await?;
registry.activate_backend("backend_id").await?;
```

---

### ToolRegistry (crate: neomind-tools)

**用途**: Agent 可用的工具函数

**类型**: 编译时注册 + 运行时动态添加

**API**: `/api/tools/*`

**职责**:
- 管理工具函数注册表
- 提供工具执行能力
- 处理工具参数验证
- 支持工具组合和链式调用

**内置工具**:
- ListDevicesTool
- QueryDataTool
- ControlDeviceTool
- ListRulesTool
- CreateRuleTool
- DeviceDiscoverTool

**示例**:
```rust
use neomind_tools::{ToolRegistry, ToolRegistryBuilder};

let registry = ToolRegistryBuilder::new()
    .with_tool(Arc::new(ListDevicesTool::new()))
    .build();

let result = registry.execute("list_devices", json!({})).await?;
```

---

### DeviceRegistry (crate: neomind-devices)

**用途**: 设备配置和类型模板

**类型**: 持久化存储

**存储**: `data/devices.redb`

**API**: `/api/devices/*`, `/api/device-types/*`

**职责**:
- 管理设备实例配置
- 存储设备类型模板 (MDL)
- 处理设备发现和自动上线
- 管理设备遥测数据

**示例**:
```rust
use neomind_devices::{DeviceRegistry, DeviceConfig};

let registry = DeviceRegistry::new();
registry.register_device(device_config).await?;
let device = registry.get_device("device_id").await?;
```

---

### UnifiedPluginRegistry (crate: neomind-core)

**用途**: 动态插件管理（内部使用）

**类型**: 运行时加载

**职责**:
- 管理插件生命周期
- 支持 WASM 和 Native 插件
- 提供 ABI 版本检查
- 处理插件依赖关系

**注意**: 此 Registry 主要被 `neomind-devices` 的插件适配器使用

---

## 内部使用的 Registry

### WasmLlmPluginRegistry (crate: neomind-sandbox)

**用途**: WASM LLM 插件执行

**可见性**: 私有，由 LlmBackendRegistry 内部使用

**职责**:
- 沙箱化 WASM LLM 插件执行
- 管理 WASM 模块实例
- 提供 Host API

---

## Registry 对比

| Registry | 外部API | 持久化 | 动态加载 | 主要用途 |
|----------|---------|--------|----------|----------|
| ExtensionRegistry | ✅ | ❌ | ✅ | 第三方扩展 |
| LlmBackendRegistry | ✅ | ✅ | ❌ | LLM 后端 |
| ToolRegistry | ✅ | ❌ | ✅ | Agent 工具 |
| DeviceRegistry | ✅ | ✅ | ❌ | 设备管理 |
| UnifiedPluginRegistry | ❌ | ❌ | ✅ | 内部插件 |
| WasmLlmPluginRegistry | ❌ | ❌ | ✅ | WASM 沙箱 |

---

## 设计原则

1. **单一职责**: 每个 Registry 只管理一种类型的资源
2. **生命周期明确**: 加载 → 注册 → 使用 → 卸载
3. **线程安全**: 所有 Registry 都是 `Send + Sync`
4. **可测试性**: 支持内存存储用于测试

---

## 未来改进

- [ ] 统一 Registry trait 接口
- [ ] 添加 Registry 事件通知机制
- [ ] 实现 Registry 热重载
