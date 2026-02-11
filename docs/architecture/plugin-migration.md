# Plugin 到 Extension 迁移分析

> NeoMind v0.5.8 (Unified Extension System)
> 更新时间: 2025-02-12

## 迁移完成状态

### 统一扩展系统 (v0.5.x)

当前分支 `feature/unified-extension-system` 已完成Plugin到Extension的统一迁移：

| 系统 | 位置 | 状态 |
|------|------|------|
| **Extension** | `neomind-core/src/extension/` | ✅ 主系统 |
| **Plugin** | `neomind-core/src/plugin/` | ⚠️ 已废弃，兼容性保留 |

---

## Extension 系统 (当前)

### 核心组件

```rust
// neomind-core/src/extension/
mod.rs           # Extension trait 和类型定义
loader/
├── mod.rs       # 加载器抽象
├── native.rs    # Native 加载器 (.so/.dylib/.dll)
└── wasm.rs      # WASM 加载器
types.rs         # ExtensionMetadata, ExtensionState 等
registry.rs      # ExtensionRegistry 生命周期管理
executor.rs      # 扩展执行器
safety.rs        # 沙箱安全检查
system.rs        # 系统扩展管理
```

### Extension Trait

```rust
pub trait Extension: Send + Sync {
    /// 获取扩展元数据
    fn metadata(&self) -> &ExtensionMetadata;

    /// 启动扩展
    fn start(&mut self) -> Result<(), ExtensionError>;

    /// 停止扩展
    fn stop(&mut self) -> Result<(), ExtensionError>;

    /// 获取当前状态
    fn state(&self) -> ExtensionState;

    /// 健康检查
    fn health(&self) -> HealthStatus;

    /// 执行命令
    fn execute_command(&mut self, cmd: &str, args: &Value) -> Result<Value>;
}
```

### 扩展类型

```rust
pub enum ExtensionType {
    /// 设备适配器
    DeviceAdapter,
    /// 数据源
    DataSource,
    /// 告警通道
    AlertChannel,
    /// LLM后端
    LlmBackend,
    /// 工具
    Tool,
    /// 通用扩展
    Generic,
}
```

---

## 扩展指标存储

### ExtensionMetricsStorage

新增 `neomind-api/src/server/extension_metrics.rs` 统一管理扩展时序数据：

```rust
pub struct ExtensionMetricsStorage {
    metrics_storage: Arc<TimeSeriesStore>,
}

impl ExtensionMetricsStorage {
    /// 存储扩展指标到 timeseries.redb
    pub async fn store_metric_value(
        &self,
        extension_id: &str,
        metric_value: &MetricValue,
    ) -> Result<()> {
        let source_id = DataSourceId::new(
            &format!("extension:{}:{}", extension_id, metric_value.name)
        )?;

        self.metrics_storage.write(
            &source_id.device_part(),  // "extension:extension_id"
            source_id.metric_part(),   // metric_name
            data_point,
        ).await?;

        Ok(())
    }
}
```

### DataSourceId 格式

扩展指标使用DataSourceId进行类型安全的存储和查询：

```
extension:{extension_id}:{metric_name}

示例:
extension:weather:temperature
extension:weather:humidity
extension:stock:price
```

---

## API 端点

### Extensions API

```
GET    /api/extensions                     # 列出扩展
POST   /api/extensions                     # 注册扩展
GET    /api/extensions/:id                 # 获取扩展详情
DELETE /api/extensions/:id                 # 注销扩展
POST   /api/extensions/:id/start           # 启动扩展
POST   /api/extensions/:id/stop            # 停止扩展
GET    /api/extensions/:id/health          # 健康检查
POST   /api/extensions/:id/command         # 执行命令
GET    /api/extensions/:id/stats           # 获取统计
POST   /api/extensions/discover            # 自动发现扩展
GET    /api/extensions/types               # 扩展类型

# 扩展指标
GET    /api/extensions/:id/metrics         # 列出扩展指标
POST   /api/extensions/:id/metrics         # 注册指标
DELETE /api/extensions/:id/metrics/:name   # 删除指标
```

### Plugins API (已废弃)

```
GET    /api/plugins                        # 重定向到 /api/extensions
POST   /api/plugins                        # 重定向到 /api/extensions
```

---

## 数据库统一

### 时序数据库

所有时序数据现在统一存储在 `data/timeseries.redb`：

| 数据类型 | device_part | metric_part |
|---------|-------------|-------------|
| 设备遥测 | `{device_id}` | `{metric_name}` |
| 扩展指标 | `extension:{ext_id}` | `{metric_name}` |
| 转换指标 | `transform:{trans_id}` | `{metric_name}` |

**重要**: AgentExecutor 现在使用 `data/timeseries.redb` 而不是 `data/timeseries_agents.redb`，这使得Agent可以访问所有设备和扩展指标。

---

## 前端集成

### 新增组件

```
web/src/components/extensions/
├── DiscoverExtensionsDialog.tsx    # 扩展发现对话框
├── ExtensionDataSourceSelector.tsx # 扩展数据源选择器
├── ExtensionDetailsDialog.tsx      # 扩展详情对话框
├── ExtensionMetricSelector.tsx     # 扩展指标选择器
├── ExtensionToolSelector.tsx       # 扩展工具选择器
├── ExtensionTransformConfig.tsx    # 扩展转换配置
└── MarketplaceDialog.tsx           # 扩展市场对话框
```

### 扩展页面

```
web/src/pages/extensions.tsx        # 统一的扩展管理页面（替代 plugins.tsx）
```

---

## 迁移指南

### 对于开发者

1. **使用Extension trait替代Plugin trait**:
   ```rust
   // 旧代码
   impl Plugin for MyPlugin { ... }

   // 新代码
   impl Extension for MyExtension { ... }
   ```

2. **更新导入路径**:
   ```rust
   // 旧代码
   use neomind_core::plugin::{Plugin, PluginRegistry};

   // 新代码
   use neomind_core::extension::{Extension, ExtensionRegistry};
   ```

3. **API 调用更新**:
   ```typescript
   // 旧代码
   await api.listPlugins()

   // 新代码
   await api.listExtensions()
   ```

---

## 当前状态总结

| 功能 | 状态 |
|------|------|
| Extension Trait | ✅ 完成 |
| Native Loader | ✅ 完成 |
| WASM Loader | 🟡 部分支持 |
| ExtensionRegistry | ✅ 完成 |
| ExtensionMetricsStorage | ✅ 完成 |
| API 端点 | ✅ 完成 |
| 前端UI | ✅ 完成 |
| Plugin兼容层 | ✅ 保留 |

---
