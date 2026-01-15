# NeoTalk 统一插件系统重构计划

> **创建日期**: 2025-01-15
> **目标**: 统一 LLM 后端、设备适配器、扩展插件的管理接口和 UI 展示

---

## 目录

1. [背景与目标](#1-背景与目标)
2. [当前架构分析](#2-当前架构分析)
3. [目标架构设计](#3-目标架构设计)
4. [动态插件加载](#4-动态插件加载)
5. [实施计划](#5-实施计划)
6. [文件清单](#6-文件清单)
7. [API 设计](#7-api-设计)
8. [数据模型](#8-数据模型)
9. [前端设计](#9-前端设计)
10. [测试计划](#10-测试计划)
11. [风险评估](#11-风险评估)
12. [设备管理优化机会](#d1-当前设备管理代码分析)
13. [MDL 系统完善计划](#附录-e-mdl-系统完善计划)

---

## 1. 背景与目标

### 1.1 现状问题

```
当前 NeoTalk 存在三套独立的插件管理系统：

┌─────────────────────────────────────────────────────────────┐
│  LLM Backend System                                         │
│  ├─ LlmBackendInstanceManager                               │
│  ├─ BackendFactory (OllamaFactory, CloudFactory)            │
│  ├─ /api/llm-backends/*                                     │
│  └─ LLMBackendsTab (前端独立页面)                            │
├─────────────────────────────────────────────────────────────┤
│  Device Adapter System                                      │
│  ├─ DeviceAdapterPluginRegistry                             │
│  ├─ DeviceAdapter trait                                     │
│  ├─ /api/plugins/device-adapters/*                          │
│  └─ ConnectionsTab (前端独立页面)                            │
├─────────────────────────────────────────────────────────────┤
│  Extension Plugin System                                    │
│  ├─ UnifiedPluginRegistry                                   │
│  ├─ UnifiedPlugin trait                                     │
│  ├─ /api/plugins/* (扩展插件)                                │
│  └─ PluginGrid (前端统一展示)                                │
└─────────────────────────────────────────────────────────────┘
```

**核心问题**：
1. **三个独立的注册表** - 同样的功能重复实现
2. **三套 API 端点** - 前端需要调用不同的接口
3. **三种前端 UI 模式** - 维护成本高，用户体验不一致
4. **插件类型硬编码** - 添加新类型需要修改多处代码

### 1.2 重构目标

| 目标 | 描述 | 优先级 |
|------|------|--------|
| **统一插件注册表** | 单一 PluginRegistry 管理所有插件类型 | P0 |
| **统一 API 端点** | 所有插件通过 `/api/plugins/*` 操作 | P0 |
| **统一前端 UI** | 单一插件页面，Schema 驱动展示 | P0 |
| **动态插件加载** | 支持运行时加载第三方 .so/.dylib/.dll | P1 |
| **向后兼容** | 保留旧 API 直到迁移完成 | P1 |

### 1.3 不改变的内容

- ✅ `PluginCard` 组件布局（已足够通用）
- ✅ `SchemaConfigForm` 动态表单生成器
- ✅ `PluginUISchema` 类型定义
- ✅ 各插件的内部实现逻辑（MqttAdapter、OllamaRuntime 等）

---

## 2. 当前架构分析

### 2.1 后端架构对比

| 特性 | LLM Backend | Device Adapter | Extension Plugin |
|------|-------------|----------------|------------------|
| **注册表** | `LlmBackendInstanceManager` | `DeviceAdapterPluginRegistry` | `UnifiedPluginRegistry` |
| **Trait** | `LlmRuntime` | `DeviceAdapter` | `UnifiedPlugin` |
| **工厂** | `BackendFactory` | 无 | `PluginFactory` |
| **存储** | `llm_backends.redb` | `device_state` | `plugins.redb` |
| **API** | `/api/llm-backends/*` | `/api/plugins/device-adapters/*` | `/api/plugins/*` |

### 2.2 前端架构对比

| 组件 | 用途 | 数据源 | 状态 |
|------|------|--------|------|
| `PluginGrid` | 统一插件展示 | `/api/plugins` | 已实现 |
| `PluginCard` | 基础插件卡片 | 统一 Prop | 已实现 |
| `SchemaPluginCard` | Schema 驱动卡片 | Schema 定义 | 已实现 |
| `SchemaConfigForm` | 动态表单 | Schema fields | 已实现 |
| `LLMBackendsTab` | LLM 专用页面 | `/api/llm-backends/*` | 需迁移 |
| `ConnectionsTab` | 连接管理 | 多个 API | 需迁移 |

### 2.3 已有的 Schema 系统

```typescript
// web/src/types/plugin-schema.ts
interface PluginUISchema {
  id: string
  name: string
  description: string
  category: 'ai' | 'devices' | 'notify'
  icon: string
  fields: Record<string, FieldSchema>
  builtin?: boolean
  listTemplate?: {
    showConfig?: boolean
    configDisplay?: (config: any) => string
  }
}

interface FieldSchema {
  name: string
  type: 'string' | 'url' | 'password' | 'number' | 'boolean' | 'select' | 'text'
  label: string
  description?: string
  default?: any
  required?: boolean
  readonly?: boolean
  placeholder?: string
  options?: SelectOption[]
  minimum?: number
  maximum?: number
  step?: number
  group?: string
  order?: number
  hidden?: boolean
  showWhen?: ShowWhenCondition
}
```

**已有的优势**：
- 支持 8 种字段类型
- 支持字段分组
- 支持条件显示
- 支持自定义配置显示

---

## 3. 目标架构设计

### 3.1 统一插件系统架构

```
┌───────────────────────────────────────────────────────────────────┐
│                          统一插件系统                                │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    PluginRegistry                           │  │
│  │  ┌─────────────┬─────────────┬─────────────┬────────────┐  │  │
│  │  │   LLM       │  Device     │  Tool       │  Storage   │  │  │
│  │  │   Backend   │  Adapter    │  Plugin     │  Plugin    │  │  │
│  │  └─────────────┴─────────────┴─────────────┴────────────┘  │  │
│  └────────────────────────────────────────────────────────────┘  │
│                           ↓                                       │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                  PluginFactory                             │  │
│  │  • register_factory(type, factory)                         │  │
│  │  • create_instance(type, config) -> PluginInstance         │  │
│  │  • get_schema(type) -> PluginUISchema                      │  │
│  └────────────────────────────────────────────────────────────┘  │
│                           ↓                                       │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                  PluginStore (redb)                        │  │
│  │  • instances: Map<id, PluginInstance>                      │  │
│  │  • active_instance: Option<id>                             │  │
│  └────────────────────────────────────────────────────────────┘  │
│                           ↓                                       │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    API Layer                               │  │
│  │  GET    /api/plugins                    → List all         │  │
│  │  GET    /api/plugins/:id                → Get one          │  │
│  │  POST   /api/plugins                    → Create instance  │  │
│  │  PUT    /api/plugins/:id                → Update instance  │  │
│  │  DELETE /api/plugins/:id                → Delete instance  │  │
│  │  POST   /api/plugins/:id/activate       → Set active       │  │
│  │  POST   /api/plugins/:id/test           → Test connection  │  │
│  │  GET    /api/plugins/types              → List types       │  │
│  │  POST   /api/plugins/upload             → Upload .so/.dll  │  │
│  └────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
```

### 3.2 统一插件类型枚举

```rust
// crates/core/src/plugin/types.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    // AI/LLM 相关
    LlmBackend,
    Embedding,

    // 设备连接
    DeviceAdapter,
    ProtocolAdapter,

    // 功能扩展
    Tool,
    Notification,
    Storage,

    // 第三方扩展
    Custom(String),
}

impl Display for PluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginType::LlmBackend => write!(f, "llm_backend"),
            PluginType::DeviceAdapter => write!(f, "device_adapter"),
            PluginType::Tool => write!(f, "tool"),
            PluginType::Notification => write!(f, "notification"),
            PluginType::Storage => write!(f, "storage"),
            PluginType::Embedding => write!(f, "embedding"),
            PluginType::ProtocolAdapter => write!(f, "protocol_adapter"),
            PluginType::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    Ai,
    Devices,
    Tools,
    Notify,
    Storage,
}

impl PluginType {
    pub fn category(&self) -> PluginCategory {
        match self {
            PluginType::LlmBackend | PluginType::Embedding => PluginCategory::Ai,
            PluginType::DeviceAdapter | PluginType::ProtocolAdapter => PluginCategory::Devices,
            PluginType::Tool => PluginCategory::Tools,
            PluginType::Notification => PluginCategory::Notify,
            PluginType::Storage => PluginCategory::Storage,
            PluginType::Custom(_) => PluginCategory::Tools,
        }
    }
}
```

### 3.3 统一插件 Trait

```rust
// crates/core/src/plugin/unified.rs

use async_trait::async_trait;

/// 统一插件 Trait - 所有插件类型必须实现
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 插件类型标识符（唯一）
    fn plugin_type(&self) -> &'static str;

    /// 插件实例 ID
    fn instance_id(&self) -> &str;

    /// 插件显示名称
    fn display_name(&self) -> &str;

    /// 插件描述
    fn description(&self) -> &str {
        ""
    }

    /// 插件版本
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// 启动插件
    async fn start(&self) -> Result<(), PluginError>;

    /// 停止插件
    async fn stop(&self) -> Result<(), PluginError>;

    /// 获取插件状态
    fn status(&self) -> PluginStatus;

    /// 测试连接/健康检查
    async fn health_check(&self) -> Result<HealthStatus, PluginError> {
        Ok(HealthStatus::Healthy)
    }

    /// 获取插件配置 Schema
    fn config_schema(&self) -> Value {
        json!({})
    }

    /// 获取插件统计信息
    fn stats(&self) -> Value {
        json!({})
    }

    /// 处理插件特定命令
    async fn handle_command(&self, command: &str, params: Value) -> Result<Value, PluginError> {
        Err(PluginError::UnsupportedCommand(command.to_string()))
    }
}

/// 插件状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub latency_ms: Option<f64>,
    pub message: Option<String>,
    pub last_check: i64,
}
```

### 3.4 统一插件工厂

```rust
// crates/core/src/plugin/factory.rs

use std::sync::Arc;
use std::collections::HashMap;

/// 插件工厂 Trait
#[async_trait]
pub trait PluginFactory: Send + Sync {
    /// 工厂类型标识
    fn factory_type(&self) -> &str;

    /// 创建插件实例
    async fn create(&self, config: &PluginConfig) -> Result<Arc<dyn Plugin>, PluginError>;

    /// 获取默认配置
    fn default_config(&self) -> Value;

    /// 获取 UI Schema
    fn ui_schema(&self) -> PluginUISchema;

    /// 验证配置
    fn validate_config(&self, config: &Value) -> Result<(), PluginError>;

    /// 工厂是否可用
    async fn is_available(&self) -> bool {
        true
    }
}

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub id: String,
    pub name: String,
    pub plugin_type: String,
    pub enabled: bool,
    pub config: Value,
    pub created_at: i64,
    pub updated_at: i64,
}

/// UI Schema 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUISchema {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: PluginCategory,
    pub icon: String,
    pub fields: HashMap<String, FieldSchema>,
    pub groups: Option<HashMap<String, GroupSchema>>,
    pub list_template: Option<ListTemplateConfig>,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTemplateConfig {
    /// 配置显示模板
    pub config_display_format: Option<String>,

    /// 自定义统计项
    pub custom_stats: Vec<CustomStat>,

    /// 快速操作按钮
    pub quick_actions: Vec<QuickAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomStat {
    pub label: String,
    pub field: String,
    pub unit: Option<String>,
    pub badge: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickAction {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub confirm: Option<bool>,
}
```

### 3.5 统一插件注册表

```rust
// crates/core/src/plugin/registry.rs

use std::sync::{Arc, RwLock};
use std::collections::HashMap;

pub struct PluginRegistry {
    factories: RwLock<HashMap<String, Arc<dyn PluginFactory>>>,
    instances: RwLock<HashMap<String, Arc<dyn Plugin>>>,
    active_instance: RwLock<Option<String>>,
    event_tx: broadcast::Sender<PluginEvent>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            active_instance: RwLock::new(None),
            event_tx: broadcast::channel(100).0,
        }
    }

    /// 注册插件工厂
    pub fn register_factory(&self, factory: Arc<dyn PluginFactory>) -> Result<(), PluginError> {
        let type_id = factory.factory_type().to_string();
        let mut factories = self.factories.write()
            .map_err(|_| PluginError::LockError)?;
        factories.insert(type_id.clone(), factory);
        tracing::info!("Registered plugin factory: {}", type_id);
        Ok(())
    }

    /// 创建并注册插件实例
    pub async fn create_instance(&self, config: PluginConfig) -> Result<String, PluginError> {
        let factory = {
            let factories = self.factories.read()
                .map_err(|_| PluginError::LockError)?;
            factories.get(&config.plugin_type)
                .ok_or_else(|| PluginError::UnknownType(config.plugin_type.clone()))?
                .clone()
        };

        // 验证配置
        factory.validate_config(&config.config)?;

        // 创建实例
        let instance = factory.create(&config).await?;
        let instance_id = instance.instance_id().to_string();

        // 注册实例
        {
            let mut instances = self.instances.write()
                .map_err(|_| PluginError::LockError)?;
            instances.insert(instance_id.clone(), instance);
        }

        // 发布事件
        let _ = self.event_tx.send(PluginEvent::InstanceCreated {
            id: instance_id.clone(),
            plugin_type: config.plugin_type,
        });

        Ok(instance_id)
    }

    /// 获取所有实例
    pub fn list_instances(&self) -> Vec<PluginInstanceDto> {
        self.instances.read()
            .ok()
            .into_iter()
            .flat_map(|instances| instances.values().map(|p| PluginInstanceDto::from_plugin(p.as_ref())))
            .collect()
    }

    /// 获取指定类型的所有实例
    pub fn list_instances_by_type(&self, plugin_type: &str) -> Vec<PluginInstanceDto> {
        self.list_instances()
            .into_iter()
            .filter(|dto| dto.plugin_type == plugin_type)
            .collect()
    }

    /// 启动实例
    pub async fn start_instance(&self, id: &str) -> Result<(), PluginError> {
        let instance = self.get_instance(id)?;
        instance.start().await?;
        Ok(())
    }

    /// 停止实例
    pub async fn stop_instance(&self, id: &str) -> Result<(), PluginError> {
        let instance = self.get_instance(id)?;
        instance.stop().await?;
        Ok(())
    }

    /// 删除实例
    pub async fn remove_instance(&self, id: &str) -> Result<(), PluginError> {
        let instance = {
            let mut instances = self.instances.write()
                .map_err(|_| PluginError::LockError)?;
            instances.remove(id)
                .ok_or_else(|| PluginError::NotFound(id.to_string()))?
        };

        // 停止实例
        let _ = instance.stop().await;

        // 如果是活动实例，清除
        if *self.active_instance.read().unwrap().as_ref() == Some(id.to_string()) {
            *self.active_instance.write().unwrap() = None;
        }

        Ok(())
    }

    /// 设置活动实例（用于 LLM 等单选类型）
    pub async fn set_active(&self, id: &str) -> Result<(), PluginError> {
        let _ = self.get_instance(id)?;
        *self.active_instance.write().unwrap() = Some(id.to_string());
        Ok(())
    }

    /// 获取活动实例
    pub fn get_active(&self) -> Option<Arc<dyn Plugin>> {
        let id = self.active_instance.read().unwrap().as_ref()?;
        self.instances.read().ok()?.get(id).cloned()
    }

    /// 测试连接
    pub async fn test_connection(&self, id: &str) -> Result<HealthStatus, PluginError> {
        let instance = self.get_instance(id)?;
        let start = std::time::Instant::now();
        let status = instance.health_check().await?;
        let latency = start.elapsed().as_millis() as f64;
        Ok(HealthStatus {
            latency_ms: Some(latency),
            ..status
        })
    }

    /// 获取所有可用类型
    pub fn get_available_types(&self) -> Vec<PluginTypeDto> {
        self.factories.read()
            .ok()
            .into_iter()
            .flat_map(|f| f.values().map(|f| PluginTypeDto::from_factory(f.as_ref())))
            .collect()
    }

    /// 获取类型的 UI Schema
    pub fn get_schema(&self, plugin_type: &str) -> Option<PluginUISchema> {
        self.factories.read().ok()?.get(plugin_type).map(|f| f.ui_schema())
    }

    fn get_instance(&self, id: &str) -> Result<Arc<dyn Plugin>, PluginError> {
        self.instances.read()
            .map_err(|_| PluginError::LockError)?
            .get(id)
            .cloned()
            .ok_or_else(|| PluginError::NotFound(id.to_string()))
    }

    /// 订阅插件事件
    pub fn subscribe(&self) -> broadcast::Receiver<PluginEvent> {
        self.event_tx.subscribe()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    InstanceCreated { id: String, plugin_type: String },
    InstanceStarted { id: String },
    InstanceStopped { id: String },
    InstanceRemoved { id: String },
    ActiveChanged { old_id: Option<String>, new_id: Option<String> },
    HealthCheck { id: String, healthy: bool, latency_ms: Option<f64> },
}
```

---

## 4. 动态插件加载

### 4.1 概述

动态插件加载允许用户在**运行时**加载第三方编译的插件文件（`.so`/`.dylib`/`.dll`），无需重新编译 NeoTalk 核心。

```
动态插件加载流程：
┌─────────────────────────────────────────────────────────────────┐
│  1. 用户上传插件文件 (.so/.dylib/.dll)                          │
│                 ↓                                                │
│  2. 安全验证（签名、路径、权限）                                  │
│                 ↓                                                │
│  3. 使用 libloading 加载动态库                                    │
│                 ↓                                                │
│  4. 提取插件描述符（PluginDescriptor）                           │
│                 ↓                                                │
│  5. 创建插件工厂包装器（DynamicPluginFactory）                   │
│                 ↓                                                │
│  6. 注册到 PluginRegistry                                        │
│                 ↓                                                │
│  7. 前端显示为新的插件类型                                        │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 插件描述符结构

动态插件必须导出以下符号：

```rust
// 动态插件必须实现的结构

/// 插件 API 版本
pub const NEOTALK_PLUGIN_API_VERSION: u32 = 2;

/// 插件描述符（C 兼容）
#[repr(C)]
pub struct PluginDescriptor {
    /// API 版本（必须匹配 NEOTALK_PLUGIN_API_VERSION）
    pub api_version: u32,

    /// 插件类型标识符
    pub plugin_type: *const u8,
    pub plugin_type_len: usize,

    /// 插件名称
    pub name: *const u8,
    pub name_len: usize,

    /// 插件描述
    pub description: *const u8,
    pub description_len: usize,

    /// 插件版本
    pub version: *const u8,
    pub version_len: usize,

    /// 插件作者
    pub author: *const u8,
    pub author_len: usize,

    /// 插件类别
    pub category: PluginCategory,

    /// UI Schema JSON
    pub ui_schema: *const u8,
    pub ui_schema_len: usize,

    /// 默认配置 JSON
    pub default_config: *const u8,
    pub default_config_len: usize,

    /// 创建函数指针
    pub create: *const (),

    /// 销毁函数指针
    pub destroy: *const (),
}

#[repr(C)]
pub enum PluginCategory {
    Ai = 1,
    Devices = 2,
    Tools = 3,
    Notify = 4,
    Storage = 5,
}
```

### 4.3 动态插件加载器

```rust
// crates/core/src/plugin/dynamic/loader.rs

use libloading::{Library, Symbol};
use std::path::Path;
use std::sync::Arc;

pub struct DynamicPluginLoader {
    search_paths: Vec<PathBuf>,
    registry: Arc<PluginRegistry>,
}

impl DynamicPluginLoader {
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("/var/lib/neotalk/plugins"),
                PathBuf::from("~/.local/share/neotalk/plugins"),
            ],
            registry,
        }
    }

    /// 从文件加载插件
    pub async fn load_from_file(&self, path: &Path) -> Result<LoadResult, PluginError> {
        // 1. 安全验证
        self.validate_path(path)?;
        self.validate_signature(path)?;

        // 2. 加载动态库
        let library = unsafe {
            Library::new(path)
                .map_err(|e| PluginError::LoadFailed(e.to_string()))?
        };

        // 3. 获取描述符
        let descriptor: Symbol<PluginDescriptor> = unsafe {
            library.get(b"neotalk_plugin_descriptor")
                .map_err(|_| PluginError::InvalidPlugin("Missing descriptor".into()))?
        };

        let descriptor = unsafe { &*descriptor };

        // 4. 验证 API 版本
        if descriptor.api_version != NEOTALK_PLUGIN_API_VERSION {
            return Err(PluginError::VersionMismatch {
                expected: NEOTALK_PLUGIN_API_VERSION,
                found: descriptor.api_version,
            });
        }

        // 5. 提取字符串
        let plugin_type = self.extract_string(descriptor.plugin_type, descriptor.plugin_type_len)?;
        let name = self.extract_string(descriptor.name, descriptor.name_len)?;
        let description = self.extract_string(descriptor.description, descriptor.description_len)?;
        let ui_schema_json = self.extract_string(descriptor.ui_schema, descriptor.ui_schema_len)?;
        let default_config_json = self.extract_string(descriptor.default_config, descriptor.default_config_len)?;

        // 6. 解析 Schema
        let ui_schema: PluginUISchema = serde_json::from_str(ui_schema_json)
            .map_err(|e| PluginError::InvalidSchema(e.to_string()))?;

        let default_config: Value = serde_json::from_str(default_config_json)
            .unwrap_or_else(|_| json!({}));

        // 7. 创建工厂
        let factory = DynamicPluginFactory {
            plugin_type: plugin_type.clone(),
            name,
            description,
            ui_schema,
            default_config,
            library: Arc::new(library),
            create_fn: descriptor.create,
            destroy_fn: descriptor.destroy,
        };

        // 8. 注册到注册表
        self.registry.register_factory(Arc::new(factory)).await?;

        Ok(LoadResult {
            plugin_type,
            name,
            description,
        })
    }

    /// 自动发现并加载插件目录中的所有插件
    pub async fn discover_and_load(&self) -> Vec<LoadResult> {
        let mut results = Vec::new();

        for search_path in &self.search_paths {
            if let Ok(entries) = std::fs::read_dir(search_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if self.is_plugin_file(&path) {
                        match self.load_from_file(&path).await {
                            Ok(result) => {
                                tracing::info!("Loaded plugin: {}", result.name);
                                results.push(result);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to load plugin {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    fn validate_path(&self, path: &Path) -> Result<(), PluginError> {
        // 检查文件存在
        if !path.exists() {
            return Err(PluginError::NotFound(path.to_string_lossy().to_string()));
        }

        // 检查扩展名
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| PluginError::InvalidPlugin("No extension".into()))?;

        let valid = match std::env::consts::OS {
            "macos" => ext == "dylib",
            "linux" => ext == "so",
            "windows" => ext == "dll",
            _ => return Err(PluginError::UnsupportedPlatform),
        };

        if !valid {
            return Err(PluginError::InvalidPlugin(
                format!("Invalid extension: {}", ext)
            ));
        }

        // 检查路径是否在允许的搜索路径内
        let canonical = path.canonicalize()
            .map_err(|e| PluginError::InvalidPlugin(e.to_string()))?;

        let allowed = self.search_paths.iter().any(|search_path| {
            search_path.canonicalize()
                .ok()
                .map(|s| canonical.starts_with(s))
                .unwrap_or(false)
        });

        if !allowed {
            return Err(PluginError::SecurityViolation(
                "Plugin path outside allowed directories".into()
            ));
        }

        Ok(())
    }

    fn validate_signature(&self, path: &Path) -> Result<(), PluginError> {
        // TODO: 实现签名验证
        // 1. 读取插件文件
        // 2. 验证数字签名
        // 3. 检查证书链
        Ok(())
    }

    fn extract_string(&self, ptr: *const u8, len: usize) -> Result<&str, PluginError> {
        unsafe {
            let slice = std::slice::from_raw_parts(ptr, len);
            std::str::from_utf8(slice)
                .map_err(|e| PluginError::InvalidPlugin(e.to_string()))
        }
    }

    fn is_plugin_file(&self, path: &Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str());
        match std::env::consts::OS {
            "macos" => ext == Some("dylib"),
            "linux" => ext == Some("so"),
            "windows" => ext == Some("dll"),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadResult {
    pub plugin_type: String,
    pub name: String,
    pub description: String,
}
```

### 4.4 动态插件工厂

```rust
// crates/core/src/plugin/dynamic/factory.rs

use std::sync::Arc;
use libloading::Library;

pub struct DynamicPluginFactory {
    pub plugin_type: String,
    pub name: String,
    pub description: String,
    pub ui_schema: PluginUISchema,
    pub default_config: Value,
    library: Arc<Library>,
    create_fn: *const (),
    destroy_fn: *const (),
}

type PluginCreateFn = unsafe extern "C" fn(config: *const u8, config_len: usize) -> *mut ();
type PluginDestroyFn = unsafe extern "C" fn(*mut ());

#[async_trait]
impl PluginFactory for DynamicPluginFactory {
    fn factory_type(&self) -> &str {
        &self.plugin_type
    }

    async fn create(&self, config: &PluginConfig) -> Result<Arc<dyn Plugin>, PluginError> {
        // 序列化配置
        let config_json = serde_json::to_string(&config.config)
            .map_err(|e| PluginError::SerializationError(e.to_string()))?;

        // 调用插件的 create 函数
        let instance_ptr = unsafe {
            let create_fn: Symbol<PluginCreateFn> = self.library.get(self.create_fn as *const [u8])
                .map_err(|e| PluginError::LoadFailed(e.to_string()))?;
            create_fn(config_json.as_ptr(), config_json.len())
        };

        if instance_ptr.is_null() {
            return Err(PluginError::InitializationFailed("Create returned null".into()));
        }

        // 创建包装器
        let wrapper = DynamicPluginWrapper {
            instance: instance_ptr,
            plugin_type: self.plugin_type.clone(),
            instance_id: config.id.clone(),
            name: config.name.clone(),
            library: self.library.clone(),
            destroy_fn: self.destroy_fn,
        };

        Ok(Arc::new(wrapper))
    }

    fn default_config(&self) -> Value {
        self.default_config.clone()
    }

    fn ui_schema(&self) -> PluginUISchema {
        self.ui_schema.clone()
    }

    fn validate_config(&self, config: &Value) -> Result<(), PluginError> {
        // 使用 Schema 验证配置
        // TODO: 实现 JSON Schema 验证
        Ok(())
    }

    async fn is_available(&self) -> bool {
        true  // 已加载，总是可用
    }
}

/// 动态插件包装器
pub struct DynamicPluginWrapper {
    instance: *mut (),
    plugin_type: String,
    instance_id: String,
    name: String,
    library: Arc<Library>,
    destroy_fn: *const (),
}

// Drop 时调用插件的 destroy 函数
impl Drop for DynamicPluginWrapper {
    fn drop(&mut self) {
        if !self.instance.is_null() {
            unsafe {
                let destroy_fn: Symbol<PluginDestroyFn> = self.library.get(self.destroy_fn as *const [u8])
                    .unwrap_or_else(|_| std::process::abort());
                destroy_fn(self.instance);
            }
        }
    }
}

#[async_trait]
impl Plugin for DynamicPluginWrapper {
    fn plugin_type(&self) -> &'static str {
        &self.plugin_type
    }

    fn instance_id(&self) -> &str {
        &self.instance_id
    }

    fn display_name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Dynamic Plugin"
    }

    async fn start(&self) -> Result<(), PluginError> {
        // 调用插件的方法（如果定义了）
        self.call_method("start", &json!({})).await?;
        Ok(())
    }

    async fn stop(&self) -> Result<(), PluginError> {
        self.call_method("stop", &json!({})).await?;
        Ok(())
    }

    fn status(&self) -> PluginStatus {
        // 尝试获取状态
        match self.call_method_sync("get_status", &json!({})) {
            Ok(value) => {
                if value["running"].as_bool().unwrap_or(false) {
                    PluginStatus::Running
                } else {
                    PluginStatus::Stopped
                }
            }
            Err(_) => PluginStatus::Stopped,
        }
    }

    async fn health_check(&self) -> Result<HealthStatus, PluginError> {
        let start = std::time::Instant::now();
        let result = self.call_method("health_check", &json!({})).await?;
        let latency = start.elapsed().as_millis() as f64;

        Ok(HealthStatus {
            healthy: result["healthy"].as_bool().unwrap_or(true),
            latency_ms: Some(latency),
            message: result["message"].as_str().map(String::from),
            last_check: chrono::Utc::now().timestamp(),
        })
    }

    fn config_schema(&self) -> Value {
        // 返回插件的 UI Schema（从工厂获取）
        json!({})
    }

    fn stats(&self) -> Value {
        self.call_method_sync("get_stats", &json!({})).unwrap_or_else(|_| json!({}))
    }

    async fn handle_command(&self, command: &str, params: Value) -> Result<Value, PluginError> {
        self.call_method(command, &params).await
    }
}

impl DynamicPluginWrapper {
    /// 异步调用插件方法
    async fn call_method(&self, method: &str, params: &Value) -> Result<Value, PluginError> {
        let method_name = format!("neotalk_plugin_{}", method);
        let method_ptr = self.library.get(method_name.as_bytes());

        if let Ok(method_fn) = method_fn {
            // 调用方法
            unsafe {
                let params_json = serde_json::to_string(params)
                    .map_err(|e| PluginError::SerializationError(e.to_string()))?;

                type PluginMethodFn = unsafe extern "C" fn(*mut (), *const u8, usize) -> *const u8;
                let method_fn: Symbol<PluginMethodFn> = method_fn;

                let result_ptr = method_fn(self.instance, params_json.as_ptr(), params_json.len());

                if result_ptr.is_null() {
                    return Ok(json!(null));
                }

                // 解析返回值（假设返回 JSON 字符串）
                let len = *(result_ptr as *const usize).offset(-1);
                let slice = std::slice::from_raw_parts(result_ptr, len);
                let result_str = std::str::from_utf8(slice)
                    .map_err(|e| PluginError::InvalidPlugin(e.to_string()))?;

                let result = serde_json::from_str(result_str)
                    .map_err(|e| PluginError::SerializationError(e.to_string()))?;

                // 释放返回值
                type FreeFn = unsafe extern "C" fn(*const u8);
                let free_fn: Symbol<FreeFn> = self.library.get(b"neotalk_free_string").unwrap();
                free_fn(result_ptr);

                Ok(result)
            }
        } else {
            Err(PluginError::UnsupportedMethod(method.to_string()))
        }
    }

    /// 同步调用插件方法
    fn call_method_sync(&self, method: &str, params: &Value) -> Result<Value, PluginError> {
        // 类似 call_method，但同步执行
        // 简化实现，实际需要等待句柄或其他机制
        Ok(json!({}))
    }
}
```

### 4.5 动态插件 API

#### 上传插件文件

```http
POST /api/plugins/upload
Content-Type: multipart/form-data

Request:
- file: plugin.so / plugin.dylib / plugin.dll

Response:
{
  "plugin_type": "my_custom_llm",
  "name": "My Custom LLM",
  "description": "A custom LLM backend",
  "schema": { ... },
  "message": "Plugin loaded successfully"
}
```

#### 列出已加载的动态插件

```http
GET /api/plugins/dynamic

Response:
{
  "plugins": [
    {
      "plugin_type": "my_custom_llm",
      "name": "My Custom LLM",
      "version": "1.0.0",
      "author": "Developer Name",
      "loaded_at": 1705300800,
      "file_path": "/var/lib/neotalk/plugins/my_custom_llm.so"
    }
  ]
}
```

#### 卸载动态插件

```http
DELETE /api/plugins/dynamic/:plugin_type

Response:
{
  "message": "Plugin unloaded"
}
```

### 4.6 插件开发 SDK

为方便开发者创建动态插件，提供 SDK：

```rust
// neotalk-plugin-sdk/src/lib.rs

/// 创建动态插件的辅助宏
#[macro_export]
macro_rules! export_plugin {
    (
        plugin_type: $type:expr,
        name: $name:expr,
        description: $desc:expr,
        version: $version:expr,
        author: $author:expr,
        category: $cat:expr,
        ui_schema: $schema:expr,
        default_config: $config:expr,
    ) => {
        // 插件实现结构
        struct MyPlugin {
            config: serde_json::Value,
        }

        impl MyPlugin {
            fn new(config: serde_json::Value) -> Self {
                Self { config }
            }
        }

        // 导出的函数
        #[no_mangle]
        pub extern "C" fn neotalk_plugin_create(
            config_json: *const u8,
            config_len: usize,
        ) -> *mut () {
            let config_bytes = unsafe { std::slice::from_raw_parts(config_json, config_len) };
            let config_str = std::str::from_utf8(config_bytes).unwrap_or("{}");
            let config: serde_json::Value = serde_json::from_str(config_str).unwrap_or_default();

            let plugin = Box::new(MyPlugin::new(config));
            Box::into_raw(plugin) as *mut ()
        }

        #[no_mangle]
        pub extern "C" fn neotalk_plugin_destroy(plugin: *mut ()) {
            if !plugin.is_null() {
                unsafe { Box::from_raw(plugin as *mut MyPlugin) };
            }
        }

        #[no_mangle]
        pub extern "C" fn neotalk_plugin_start(
            plugin: *mut (),
            _params: *const u8,
            _params_len: usize,
        ) -> *const u8 {
            // 实现启动逻辑
            std::ptr::null()
        }

        // ... 其他方法

        // 导出描述符
        #[no_mangle]
        pub static neotalk_plugin_descriptor: $crate::PluginDescriptor = {
            $crate::PluginDescriptor {
                api_version: $crate::NEOTALK_PLUGIN_API_VERSION,
                plugin_type: concat!($type, "\0").as_ptr(),
                plugin_type_len: concat!($type, "\0").len() - 1,
                name: concat!($name, "\0").as_ptr(),
                name_len: concat!($name, "\0").len() - 1,
                description: concat!($desc, "\0").as_ptr(),
                description_len: concat!($desc, "\0").len() - 1,
                version: concat!($version, "\0").as_ptr(),
                version_len: concat!($version, "\0").len() - 1,
                author: concat!($author, "\0").as_ptr(),
                author_len: concat!($author, "\0").len() - 1,
                category: $cat,
                ui_schema: concat!($schema, "\0").as_ptr(),
                ui_schema_len: concat!($schema, "\0").len() - 1,
                default_config: concat!($config, "\0").as_ptr(),
                default_config_len: concat!($config, "\0").len() - 1,
                create: neotalk_plugin_create as *const (),
                destroy: neotalk_plugin_destroy as *const (),
            }
        };
    };
}
```

### 4.7 动态插件示例

```rust
// 示例：自定义 LLM 后端插件

use neotalk_plugin_sdk::export_plugin;

export_plugin! {
    plugin_type: "deepseek_llm",
    name: "DeepSeek LLM",
    description: "DeepSeek API integration",
    version: "1.0.0",
    author: "Your Name",
    category: PluginCategory::Ai,
    ui_schema: r#"{
        "id": "deepseek_llm",
        "name": "DeepSeek",
        "category": "ai",
        "icon": "BrainCircuit",
        "fields": {
            "api_key": {
                "type": "password",
                "label": "API Key",
                "required": true
            },
            "model": {
                "type": "string",
                "label": "Model",
                "default": "deepseek-chat"
            }
        }
    }"#,
    default_config: r#"{
        "model": "deepseek-chat"
    }"#,
}

impl MyPlugin {
    // 实现生成逻辑
    pub fn generate(&self, messages: Vec<Message>) -> Result<String, PluginError> {
        let api_key = self.config["api_key"].as_str().unwrap();
        let model = self.config["model"].as_str().unwrap();

        // 调用 DeepSeek API
        let client = reqwest::Client::new();
        let response = client.post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": model,
                "messages": messages
            }))
            .send()?
            .json::<serde_json::Value>()?;

        Ok(response["choices"][0]["message"]["content"].as_str().unwrap().to_string())
    }
}
```

### 4.8 安全考虑

| 安全问题 | 解决方案 |
|----------|----------|
| **恶意代码** | - 沙箱隔离（WASM）<br>- 权限限制<br>- 代码审计 |
| **路径遍历** | - 验证插件路径在允许目录内<br>- 禁止符号链接 |
| **版本冲突** | - API 版本检查<br>- 向后兼容性设计 |
| **内存泄漏** | - RAII 包装器<br>- Drop 时自动清理 |
| **ABI 不稳定** | - 使用 C ABI<br>- 版本化符号 |

---

## 5. 实施计划

### 阶段 1: 核心类型定义（1-2 天）

**目标**: 定义统一插件系统的核心类型和 Trait

| 任务 | 文件 | 描述 |
|------|------|------|
| 1.1 | `crates/core/src/plugin/types.rs` | 定义 PluginType、PluginCategory 枚举 |
| 1.2 | `crates/core/src/plugin/unified.rs` | 定义 Plugin trait、PluginStatus |
| 1.3 | `crates/core/src/plugin/factory.rs` | 定义 PluginFactory trait、PluginConfig |
| 1.4 | `crates/core/src/plugin/registry.rs` | 定义 PluginRegistry |
| 1.5 | `crates/core/src/plugin/error.rs` | 定义 PluginError |
| 1.6 | `crates/core/src/lib.rs` | 导出新模块 |

**验收标准**:
- [ ] 所有核心类型编译通过
- [ ] 单元测试覆盖核心类型
- [ ] 文档注释完整

### 阶段 2: 统一插件注册表实现（2-3 天）

**目标**: 实现统一的插件注册表

| 任务 | 文件 | 描述 |
|------|------|------|
| 2.1 | `crates/core/src/plugin/registry.rs` | 实现 PluginRegistry |
| 2.2 | `crates/core/src/plugin/instance_dto.rs` | 实现 DTO 类型 |
| 2.3 | `crates/core/src/plugin/mod.rs` | 模块导出 |
| 2.4 | `crates/core/src/plugin/tests.rs` | 单元测试 |

**验收标准**:
- [ ] 注册表可以注册工厂
- [ ] 可以创建、启动、停止、删除实例
- [ ] 可以设置活动实例
- [ ] 事件发布正常工作

### 阶段 3: 后端 API 实现（3-4 天）

**目标**: 实现统一的插件管理 API

| 任务 | 文件 | 描述 |
|------|------|------|
| 3.1 | `crates/api/src/handlers/plugins/unified.rs` | 统一插件 API handlers |
| 3.2 | `crates/api/src/handlers/plugins/upload.rs` | 插件上传 handler |
| 3.3 | `crates/api/src/handlers/plugins/mod.rs` | 模块导出 |
| 3.4 | `crates/api/src/routes.rs` | 注册路由 |

**API 端点**:
```
GET    /api/plugins                    # 列出所有实例
GET    /api/plugins/:id                # 获取单个实例
POST   /api/plugins                    # 创建实例
PUT    /api/plugins/:id                # 更新实例
DELETE /api/plugins/:id                # 删除实例
POST   /api/plugins/:id/start          # 启动
POST   /api/plugins/:id/stop           # 停止
POST   /api/plugins/:id/activate       # 设置为活动
POST   /api/plugins/:id/test           # 测试连接
GET    /api/plugins/types              # 列出可用类型
GET    /api/plugins/types/:id/schema   # 获取类型 schema
POST   /api/plugins/upload             # 上传插件文件
```

**验收标准**:
- [ ] 所有 API 端点可访问
- [ ] 返回正确的 JSON 格式
- [ ] 错误处理正确

### 阶段 4: 适配器实现（4-5 天）

**目标**: 将现有 LLM 后端和设备适配器适配到统一系统

| 任务 | 文件 | 描述 |
|------|------|------|
| 4.1 | `crates/llm/src/unified_factory.rs` | LlmBackendFactory 实现 |
| 4.2 | `crates/llm/src/plugin_wrapper.rs` | LlmRuntime -> Plugin wrapper |
| 4.3 | `crates/devices/src/adapter_factory.rs` | DeviceAdapterFactory 实现 |
| 4.4 | `crates/devices/src/adapter_wrapper.rs` | DeviceAdapter -> Plugin wrapper |

**适配器示例**:

```rust
// crates/llm/src/plugin_wrapper.rs

use edge_ai_core::plugin::{Plugin, PluginStatus, HealthStatus, PluginError};
use edge_ai_llm::LlmRuntime;

pub struct LlmPluginWrapper {
    inner: Arc<dyn LlmRuntime>,
    instance_id: String,
    name: String,
}

impl LlmPluginWrapper {
    pub fn new(inner: Arc<dyn LlmRuntime>, instance_id: String, name: String) -> Self {
        Self { inner, instance_id, name }
    }
}

#[async_trait]
impl Plugin for LlmPluginWrapper {
    fn plugin_type(&self) -> &'static str {
        "llm_backend"
    }

    fn instance_id(&self) -> &str {
        &self.instance_id
    }

    fn display_name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "LLM Backend"
    }

    async fn start(&self) -> Result<(), PluginError> {
        // LLM backends don't need explicit start
        Ok(())
    }

    async fn stop(&self) -> Result<(), PluginError> {
        // LLM backends don't need explicit stop
        Ok(())
    }

    fn status(&self) -> PluginStatus {
        PluginStatus::Running
    }

    async fn health_check(&self) -> Result<HealthStatus, PluginError> {
        // Try a simple generation to test
        let start = std::time::Instant::now();
        let result = self.inner.generate(edge_ai_llm::LlmInput {
            messages: vec![edge_ai_core::llm::backend::Message::user("test")],
            ..Default::default()
        }).await;
        let latency = start.elapsed().as_millis() as f64;

        Ok(HealthStatus {
            healthy: result.is_ok(),
            latency_ms: Some(latency),
            message: None,
            last_check: chrono::Utc::now().timestamp(),
        })
    }
}
```

**验收标准**:
- [ ] Ollama 可以通过统一系统管理
- [ ] OpenAI/Anthropic 可以通过统一系统管理
- [ ] MQTT 适配器可以通过统一系统管理
- [ ] HASS 适配器可以通过统一系统管理

### 阶段 5: 动态插件加载（4-5 天）

**目标**: 实现运行时加载第三方插件文件的功能

| 任务 | 文件 | 描述 |
|------|------|------|
| 5.1 | `crates/core/src/plugin/dynamic/descriptor.rs` | PluginDescriptor 定义 |
| 5.2 | `crates/core/src/plugin/dynamic/loader.rs` | DynamicPluginLoader 实现 |
| 5.3 | `crates/core/src/plugin/dynamic/factory.rs` | DynamicPluginFactory 实现 |
| 5.4 | `crates/core/src/plugin/dynamic/wrapper.rs` | DynamicPluginWrapper 实现 |
| 5.5 | `crates/core/src/plugin/dynamic/security.rs` | 签名验证 |
| 5.6 | `crates/plugin-sdk/Cargo.toml` | SDK crate 定义 |
| 5.7 | `crates/plugin-sdk/src/lib.rs` | 导出宏和类型 |
| 5.8 | `crates/plugin-sdk/examples/custom_llm/` | 示例插件 |
| 5.9 | `crates/api/src/handlers/plugins/dynamic.rs` | 动态插件 API handlers |
| 5.10 | `web/src/components/plugins/PluginUploadDialog.tsx` | 上传对话框 |
| 5.11 | `web/src/components/plugins/DynamicPluginList.tsx` | 动态插件列表 |

**关键功能**:
1. 动态库加载（.so/.dylib/.dll）
2. 安全验证（签名、路径检查）
3. 描述符提取和解析
4. 工厂注册和实例创建
5. 前端上传和管理界面

**验收标准**:
- [ ] 可以上传并加载 .so 文件
- [ ] 可以列出已加载的动态插件
- [ ] 可以创建动态插件的实例
- [ ] 可以卸载动态插件
- [ ] SDK 可以成功编译示例插件
- [ ] 安全验证正常工作

### 阶段 6: 前端 Schema 扩展（2-3 天）

**目标**: 扩展现有 Schema 系统以支持所有插件类型

| 任务 | 文件 | 描述 |
|------|------|------|
| 5.1 | `web/src/types/plugin-schema.ts` | 扩展 PluginUISchema 类型 |
| 5.2 | `web/src/components/plugins/SchemaConfigForm.tsx` | 添加新字段类型支持 |
| 5.3 | `web/src/components/plugins/UnifiedPluginCard.tsx` | 创建统一插件卡片 |

**Schema 扩展**:

```typescript
// web/src/types/plugin-schema.ts

export interface PluginUISchema {
  // 基础信息
  id: string
  name: string
  description: string
  category: 'ai' | 'devices' | 'tools' | 'notify' | 'storage'
  icon: string
  version?: string

  // 字段定义
  fields: Record<string, FieldSchema>

  // 分组定义
  groups?: Record<string, GroupSchema>

  // 内置标记
  builtin?: boolean

  // UI 扩展
  listTemplate?: {
    // 配置显示（格式字符串: "{endpoint}:{port}"）
    configDisplayFormat?: string

    // 自定义统计行
    customStats?: CustomStatDef[]

    // 快速操作按钮
    quickActions?: QuickActionDef[]

    // 状态指示器自定义
    statusIndicator?: (instance: PluginInstance) => {
      text: string
      color: string
      icon?: string
    }
  }

  // 实例模板
  instanceTemplate?: {
    defaultName: string
    config: Record<string, unknown>
  }
}

export interface CustomStatDef {
  label: string
  field: string  // 从 instance.config 或 instance.stats 中获取
  unit?: string
  badge?: boolean
  format?: 'number' | 'bytes' | 'duration'
}

export interface QuickActionDef {
  id: string
  label: string
  icon: string
  variant?: 'default' | 'outline' | 'ghost'
  confirm?: boolean | string  // true 或确认消息
  danger?: boolean
}
```

**验收标准**:
- [ ] TypeScript 类型定义正确
- [ ] Schema 验证通过

### 阶段 7: 前端统一页面实现（3-4 天）

**目标**: 实现统一的插件管理页面

| 任务 | 文件 | 描述 |
|------|------|------|
| 6.1 | `web/src/hooks/usePlugins.ts` | 统一插件数据 hook |
| 6.2 | `web/src/components/plugins/UnifiedPluginCard.tsx` | 统一插件卡片 |
| 6.3 | `web/src/components/plugins/PluginTypeGrid.tsx` | 插件类型网格 |
| 6.4 | `web/src/pages/plugins/unified.tsx` | 统一插件页面 |
| 6.5 | `web/src/pages/plugins.tsx` | 更新主页面 |

**组件结构**:

```
plugins.tsx
├── PluginTypeGrid (类型选择视图)
│   └── PluginTypeCard (Ollama, OpenAI, MQTT, ...)
└── UnifiedPluginGrid (实例列表视图)
    └── UnifiedPluginCard
        ├── PluginCardHeader (基础信息)
        ├── CustomStatsRow (自定义统计)
        └── PluginCardActions (操作按钮)
```

**验收标准**:
- [ ] 可以查看所有插件类型
- [ ] 可以创建插件实例
- [ ] 可以编辑/删除插件实例
- [ ] 可以启动/停止插件
- [ ] 可以测试连接

### 阶段 8: 数据迁移（2 天）

**目标**: 将现有插件数据迁移到统一存储

| 任务 | 文件 | 描述 |
|------|------|------|
| 7.1 | `crates/storage/src/migration.rs` | 数据迁移工具 |
| 7.2 | `crates/api/src/handlers/plugins/migrate.rs` | 迁移 API |

**迁移脚本**:

```rust
// crates/storage/src/migration.rs

pub async fn migrate_to_unified() -> Result<(), MigrationError> {
    // 1. 读取 LLM backends
    let llm_instances = LlmBackendStore::list_all()?;

    // 2. 读取 Device adapters
    let device_adapters = DeviceAdapterStore::list_all()?;

    // 3. 读取 Extension plugins
    let ext_plugins = ExtensionPluginStore::list_all()?;

    // 4. 转换为统一格式
    for llm in llm_instances {
        let plugin = PluginInstance {
            id: llm.id,
            name: llm.name,
            plugin_type: PluginType::LlmBackend,
            enabled: llm.is_active,
            config: to_unified_config(llm),
            created_at: llm.created_at,
            updated_at: llm.updated_at,
        };
        UnifiedPluginStore::insert(plugin)?;
    }

    // ... 类似处理 device_adapters 和 ext_plugins

    Ok(())
}
```

**验收标准**:
- [ ] LLM 后端数据成功迁移
- [ ] 设备适配器数据成功迁移
- [ ] 扩展插件数据成功迁移
- [ ] 回滚功能可用

### 阶段 9: 向后兼容层（2 天）

**目标**: 保留旧 API 以平滑过渡

| 任务 | 文件 | 描述 |
|------|------|------|
| 8.1 | `crates/api/src/handlers/llm_backends_legacy.rs` | LLM API 兼容层 |
| 8.2 | `crates/api/src/handlers/device_adapters_legacy.rs` | Device API 兼容层 |

**兼容层示例**:

```rust
// crates/api/src/handlers/llm_backends_legacy.rs

/// 旧版 LLM 后端列表 API - 内部转换为统一 API
pub async fn list_backends_handler_legacy(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    // 调用新的统一 API
    let plugins = state.plugin_registry.list_instances();
    let llm_backends: Vec<_> = plugins
        .into_iter()
        .filter(|p| p.plugin_type == "llm_backend")
        .map(|p| to_llm_backend_dto(p))
        .collect();

    ok(json!({
        "backends": llm_backends,
        "count": llm_backends.len(),
    }))
}
```

**验收标准**:
- [ ] 旧 API 端点仍可访问
- [ ] 数据格式与旧版本兼容
- [ ] 前端旧页面仍可工作

### 阶段 10: 测试与文档（2-3 天）

| 任务 | 描述 |
|------|------|
| 9.1 | 单元测试覆盖 |
| 9.2 | 集成测试 |
| 9.3 | API 文档更新 |
| 9.4 | 用户文档更新 |
| 9.5 | 插件开发指南 |

### 阶段 11: 上线与监控（持续）

| 任务 | 描述 |
|------|------|
| 10.1 | 灰度发布 |
| 10.2 | 监控指标 |
| 10.3 | 问题修复 |
| 10.4 | 性能优化 |

---

## 6. 文件清单

### 6.1 新增文件

#### 后端核心

```
crates/core/src/plugin/
├── mod.rs                    # 模块导出
├── types.rs                  # PluginType, PluginCategory
├── unified.rs                # Plugin trait
├── factory.rs                # PluginFactory trait
├── registry.rs               # PluginRegistry
├── error.rs                  # PluginError
├── instance_dto.rs           # DTO 类型
├── storage.rs                # PluginStore (redb)
└── tests.rs                  # 单元测试
```

#### 后端适配器

```
crates/llm/src/
├── unified_factory.rs        # LlmBackendFactory
└── plugin_wrapper.rs         # LlmRuntime -> Plugin wrapper

crates/devices/src/
├── unified_factory.rs        # DeviceAdapterFactory
└── plugin_wrapper.rs         # DeviceAdapter -> Plugin wrapper
```

#### 动态插件加载

```
crates/core/src/plugin/dynamic/
├── mod.rs                    # 模块导出
├── loader.rs                 # DynamicPluginLoader
├── factory.rs                # DynamicPluginFactory
├── wrapper.rs                # DynamicPluginWrapper
├── descriptor.rs             # PluginDescriptor 定义
└── security.rs               # 签名验证

crates/plugin-sdk/
├── Cargo.toml                # SDK crate 定义
├── src/
│   ├── lib.rs                # 导出宏和类型
│   ├── macro.rs              # export_plugin! 宏
│   └── types.rs              # PluginDescriptor 等
└── examples/
    └── custom_llm/
        ├── Cargo.toml
        └── src/lib.rs        # 示例插件
```

#### 后端 API

```
crates/api/src/handlers/plugins/
├── mod.rs                    # 模块导出
├── unified.rs                # 统一插件 handlers
├── dynamic.rs                # 动态插件 handlers (upload/list/unload)
├── schema.rs                 # Schema handlers
└── migrate.rs                # 数据迁移
```

#### 前端类型

```
web/src/types/
├── plugin-schema.ts          # 扩展 Schema 类型
└── plugin-unified.ts         # 统一插件类型
```

#### 前端组件

```
web/src/components/plugins/
├── UnifiedPluginCard.tsx     # 统一插件卡片
├── PluginTypeGrid.tsx        # 类型网格
├── PluginTypeCard.tsx        # 类型卡片
├── PluginInstanceDialog.tsx  # 实例配置对话框
├── PluginUploadDialog.tsx    # 动态插件上传对话框
└── DynamicPluginList.tsx     # 已加载动态插件列表
```

#### 前端 Hooks

```
web/src/hooks/
└── useUnifiedPlugins.ts      # 统一插件 hook
```

#### 前端页面

```
web/src/pages/
└── plugins/
    └── unified.tsx           # 统一插件页面
```

### 6.2 修改文件

#### 后端

```
crates/core/src/lib.rs        # 导出 plugin 模块
crates/api/src/routes.rs      # 添加统一路由
crates/api/src/main.rs        # 初始化 registry
```

#### 前端

```
web/src/pages/plugins.tsx     # 添加新标签
web/src/store/slices/pluginSlice.ts  # 更新状态管理
web/src/lib/api.ts            # 添加统一 API 客户端
```

### 6.3 废弃文件（保留兼容）

```
crates/api/src/handlers/llm_backends.rs      # 标记为 legacy
crates/api/src/handlers/device_adapters.rs   # 标记为 legacy
web/src/components/llm/LLMBackendsTab.tsx    # 标记为 legacy
web/src/components/connections/ConnectionsTab.tsx  # 标记为 legacy
```

---

## 7. API 设计

### 7.1 统一插件 API

#### 列出所有插件实例

```http
GET /api/plugins

Query Parameters:
  - type: string (optional)  # 按类型过滤
  - category: string (optional)  # 按分类过滤
  - active_only: boolean (optional)

Response:
{
  "plugins": [
    {
      "id": "ollama-local",
      "name": "Local Ollama",
      "plugin_type": "llm_backend",
      "category": "ai",
      "enabled": true,
      "running": true,
      "is_active": true,
      "config": {
        "endpoint": "http://localhost:11434",
        "model": "qwen3-vl:2b"
      },
      "stats": {
        "total_requests": 1234,
        "total_tokens": 1234567,
        "avg_latency_ms": 234.5
      },
      "health": {
        "healthy": true,
        "latency_ms": 45.2,
        "last_check": 1705300800
      },
      "created_at": 1705200000,
      "updated_at": 1705300000
    }
  ],
  "count": 1,
  "active_id": "ollama-local"
}
```

#### 获取单个实例

```http
GET /api/plugins/:id

Response:
{
  "plugin": { ... }
}
```

#### 创建实例

```http
POST /api/plugins

Request:
{
  "name": "My Ollama",
  "plugin_type": "ollama",
  "enabled": false,
  "config": {
    "endpoint": "http://localhost:11434",
    "model": "qwen3-vl:2b"
  }
}

Response:
{
  "id": "ollama-my-ollama-123",
  "message": "Plugin instance created"
}
```

#### 更新实例

```http
PUT /api/plugins/:id

Request:
{
  "name": "Updated Name",
  "enabled": true,
  "config": {
    "endpoint": "http://localhost:11435"
  }
}

Response:
{
  "id": "ollama-my-ollama-123",
  "message": "Plugin instance updated"
}
```

#### 删除实例

```http
DELETE /api/plugins/:id

Response:
{
  "message": "Plugin instance deleted"
}
```

#### 启动/停止实例

```http
POST /api/plugins/:id/start
POST /api/plugins/:id/stop

Response:
{
  "id": "ollama-my-ollama-123",
  "status": "running"
}
```

#### 设置活动实例

```http
POST /api/plugins/:id/activate

Response:
{
  "id": "ollama-my-ollama-123",
  "message": "Plugin activated"
}
```

#### 测试连接

```http
POST /api/plugins/:id/test

Response:
{
  "plugin_id": "ollama-my-ollama-123",
  "result": {
    "success": true,
    "latency_ms": 45.2,
    "message": "Connection successful"
  }
}
```

#### 列出可用类型

```http
GET /api/plugins/types

Response:
{
  "types": [
    {
      "id": "ollama",
      "name": "Ollama (Local LLM)",
      "description": "Run local LLM models with Ollama",
      "category": "ai",
      "icon": "Server",
      "builtin": true,
      "available": true
    },
    {
      "id": "openai",
      "name": "OpenAI",
      "description": "OpenAI API (GPT-4, GPT-3.5)",
      "category": "ai",
      "icon": "BrainCircuit",
      "builtin": true,
      "available": true
    }
  ]
}
```

#### 获取类型 Schema

```http
GET /api/plugins/types/:id/schema

Response:
{
  "schema": {
    "id": "ollama",
    "name": "Ollama",
    "description": "Local LLM runner",
    "category": "ai",
    "icon": "Server",
    "fields": {
      "endpoint": {
        "name": "endpoint",
        "type": "url",
        "label": "API Endpoint",
        "default": "http://localhost:11434"
      },
      "model": {
        "name": "model",
        "type": "string",
        "label": "Model Name",
        "default": "qwen3-vl:2b",
        "required": true
      }
    },
    "listTemplate": {
      "configDisplayFormat": "{endpoint}",
      "customStats": [
        { "label": "Latency", "field": "avg_latency_ms", "unit": "ms" }
      ],
      "quickActions": [
        { "id": "test", "label": "Test Connection", "icon": "TestTube" }
      ]
    }
  }
}
```

#### 上传插件文件

```http
POST /api/plugins/upload

Content-Type: multipart/form-data

Request:
- file: .so/.dylib/.dll file
- type: plugin type (optional)

Response:
{
  "id": "custom-plugin-123",
  "name": "Custom Plugin",
  "message": "Plugin uploaded successfully"
}
```

---

## 8. 数据模型

### 8.1 插件实例存储

```rust
// crates/storage/src/plugin_instance.rs

use redb::{TypeName, TableDefinition};

const PLUGINS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("plugins");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstance {
    pub id: String,
    pub name: String,
    pub plugin_type: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub stats: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePluginRecord {
    pub plugin_type: String,
    pub instance_id: String,
}

impl PluginInstance {
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.id.is_empty() {
            return Err(StorageError::InvalidInput("id cannot be empty".into()));
        }
        if self.name.is_empty() {
            return Err(StorageError::InvalidInput("name cannot be empty".into()));
        }
        Ok(())
    }
}

pub struct PluginStore {
    db: Arc<Database>,
}

impl PluginStore {
    pub fn insert(&self, instance: PluginInstance) -> Result<(), StorageError> {
        instance.validate()?;
        let key = instance.id.as_str();
        let value = serde_json::to_vec(&instance)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(PLUGINS_TABLE)?;
            table.insert(key, &value)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<PluginInstance>, StorageError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PLUGINS_TABLE)?;
        Ok(table.get(id)?
            .map(|value| serde_json::from_slice(value.value()))
            .transpose()?)
    }

    pub fn list_all(&self) -> Result<Vec<PluginInstance>, StorageError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PLUGINS_TABLE)?;
        let mut instances = Vec::new();
        for item in table.iter()? {
            let (_, value) = item?;
            instances.push(serde_json::from_slice(value)?);
        }
        Ok(instances)
    }

    pub fn delete(&self, id: &str) -> Result<bool, StorageError> {
        let write_txn = self.db.begin_write()?;
        let deleted = {
            let mut table = write_txn.open_table(PLUGINS_TABLE)?;
            table.remove(id)?.is_some()
        };
        write_txn.commit()?;
        Ok(deleted)
    }
}
```

### 8.2 活动插件记录

```rust
// crates/storage/src/active_plugin.rs

const ACTIVE_PLUGIN_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("active_plugin");

pub struct ActivePluginStore {
    db: Arc<Database>,
}

impl ActivePluginStore {
    pub fn set_active(&self, plugin_type: &str, instance_id: &str) -> Result<(), StorageError> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ACTIVE_PLUGIN_TABLE)?;
            table.insert(plugin_type, instance_id)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_active(&self, plugin_type: &str) -> Result<Option<String>, StorageError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ACTIVE_PLUGIN_TABLE)?;
        Ok(table.get(plugin_type)?.map(|v| v.value().to_string()))
    }
}
```

---

## 9. 前端设计

### 9.1 组件层次结构

```
UnifiedPluginsPage
├── PluginTypeSelector
│   ├── PluginTypeCard (Ollama)
│   ├── PluginTypeCard (OpenAI)
│   ├── PluginTypeCard (MQTT)
│   └── PluginTypeCard (HASS)
└── PluginInstanceList
    └── UnifiedPluginCard
        ├── PluginCardHeader
        │   ├── Icon
        │   ├── Name + Type Badge
        │   ├── Status Badge
        │   └── Actions Menu
        ├── PluginCardBody
        │   ├── Config Display
        │   ├── Stats Row
        │   └── Health Status
        └── PluginCardFooter
            ├── Enable Switch
            ├── Quick Actions
            └── Start/Stop Button
```

### 9.2 组件 Props 设计

```typescript
// web/src/components/plugins/UnifiedPluginCard.tsx

export interface UnifiedPluginCardProps {
  // 插件实例
  instance: PluginInstance

  // Schema 定义（驱动 UI）
  schema: PluginUISchema

  // 健康状态（可选）
  health?: HealthStatus

  // 是否为活动实例
  isActive?: boolean

  // 回调函数
  onToggle?: (id: string, enabled: boolean) => Promise<boolean>
  onStart?: (id: string) => Promise<boolean>
  onStop?: (id: string) => Promise<boolean>
  onEdit?: (instance: PluginInstance) => void
  onDelete?: (id: string) => Promise<boolean>
  onActivate?: (id: string) => Promise<boolean>
  onQuickAction?: (id: string, actionId: string) => Promise<void>
  onHealthCheck?: (id: string) => Promise<HealthStatus>

  // 自定义渲染（可选）
  customHeader?: (instance: PluginInstance, schema: PluginUISchema) => ReactNode
  customStats?: (instance: PluginInstance, schema: PluginUISchema) => ReactNode
  customActions?: (instance: PluginInstance, schema: PluginUISchema) => ReactNode
}

export function UnifiedPluginCard({
  instance,
  schema,
  health,
  isActive,
  onToggle,
  onStart,
  onStop,
  onEdit,
  onDelete,
  onActivate,
  onQuickAction,
  customHeader,
  customStats,
  customActions,
}: UnifiedPluginCardProps) {
  // ...
}
```

### 9.3 Schema 驱动渲染示例

```typescript
// 配置显示
function renderConfigDisplay(instance: PluginInstance, schema: PluginUISchema) {
  const format = schema.listTemplate?.configDisplayFormat
  if (!format) return null

  // 解析格式字符串 "{endpoint}:{port}"
  const display = format.replace(/\{(\w+)\}/g, (_, key) => {
    return instance.config[key] as string || ''
  })

  return <span className="font-mono text-xs">{display}</span>
}

// 自定义统计
function renderCustomStats(instance: PluginInstance, schema: PluginUISchema) {
  const stats = schema.listTemplate?.customStats
  if (!stats || stats.length === 0) return null

  return (
    <div className="flex gap-4 text-xs text-muted-foreground">
      {stats.map((stat) => {
        const value = instance.stats[stat.field] || instance.config[stat.field]
        return (
          <span key={stat.label}>
            {stat.label}: {formatValue(value, stat)}{stat.unit}
          </span>
        )
      })}
    </div>
  )
}

// 快速操作按钮
function renderQuickActions(instance: PluginInstance, schema: PluginUISchema) {
  const actions = schema.listTemplate?.quickActions
  if (!actions || actions.length === 0) return null

  return (
    <div className="flex gap-2">
      {actions.map((action) => (
        <Button
          key={action.id}
          size="sm"
          variant={action.variant || 'outline'}
          onClick={() => onQuickAction?.(instance.id, action.id)}
        >
          <Icon name={action.icon} className="mr-2 h-4 w-4" />
          {action.label}
        </Button>
      ))}
    </div>
  )
}
```

### 9.4 插件类型 Schema 定义

```typescript
// web/src/lib/plugin-schemas.ts

export const PLUGIN_SCHEMAS: Record<string, PluginUISchema> = {
  ollama: {
    id: 'ollama',
    name: 'Ollama',
    description: 'Local LLM runner',
    category: 'ai',
    icon: 'Server',
    fields: {
      endpoint: {
        name: 'endpoint',
        type: 'url',
        label: 'API Endpoint',
        default: 'http://localhost:11434',
        required: false,
      },
      model: {
        name: 'model',
        type: 'string',
        label: 'Model Name',
        default: 'qwen3-vl:2b',
        required: true,
      },
      temperature: {
        name: 'temperature',
        type: 'number',
        label: 'Temperature',
        default: 0.7,
        minimum: 0,
        maximum: 2,
        step: 0.1,
        group: 'advanced',
      },
    },
    listTemplate: {
      configDisplayFormat: '{endpoint}',
      customStats: [
        { label: 'Latency', field: 'avg_latency_ms', unit: 'ms' },
        { label: 'Requests', field: 'total_requests' },
      ],
      quickActions: [
        { id: 'test', label: 'Test Connection', icon: 'TestTube' },
      ],
    },
  },

  mqtt: {
    id: 'mqtt',
    name: 'MQTT Broker',
    description: 'MQTT message broker',
    category: 'devices',
    icon: 'Network',
    fields: {
      host: {
        name: 'host',
        type: 'string',
        label: 'Broker Host',
        default: 'localhost',
        required: true,
      },
      port: {
        name: 'port',
        type: 'number',
        label: 'Broker Port',
        default: 1883,
        minimum: 1,
        maximum: 65535,
        required: true,
      },
      username: {
        name: 'username',
        type: 'string',
        label: 'Username',
        required: false,
      },
      password: {
        name: 'password',
        type: 'password',
        label: 'Password',
        required: false,
      },
    },
    listTemplate: {
      configDisplayFormat: '{host}:{port}',
      customStats: [
        { label: 'Connections', field: 'connection_count' },
        { label: 'Messages', field: 'message_count' },
      ],
      quickActions: [
        { id: 'discover', label: 'Discover Devices', icon: 'Radar' },
      ],
    },
  },

  // ... 其他类型
}
```

### 9.5 页面状态管理

```typescript
// web/src/pages/plugins/unified.tsx

type View = 'type-select' | 'instance-list' | 'instance-detail'

export function UnifiedPluginsPage() {
  const [view, setView] = useState<View>('type-select')
  const [selectedType, setSelectedType] = useState<string | null>(null)
  const [instances, setInstances] = useState<PluginInstance[]>([])
  const [types, setTypes] = useState<PluginTypeDto[]>([])
  const [loading, setLoading] = useState(true)

  // 获取所有类型
  useEffect(() => {
    fetchAPI<{ types: PluginTypeDto[] }>('/plugins/types')
      .then(data => setTypes(data.types))
      .finally(() => setLoading(false))
  }, [])

  // 选择类型，显示该类型的实例列表
  const handleTypeSelect = (typeId: string) => {
    setSelectedType(typeId)
    fetchAPI<{ plugins: PluginInstance[] }>(`/plugins?type=${typeId}`)
      .then(data => setInstances(data.plugins))
    setView('instance-list')
  }

  // 创建新实例
  const handleCreate = (config: Record<string, unknown>) => {
    return fetchAPI('/plugins', {
      method: 'POST',
      body: JSON.stringify({
        plugin_type: selectedType,
        ...config,
      }),
    })
  }

  // 视图渲染
  if (view === 'type-select') {
    return <PluginTypeGrid types={types} onSelect={handleTypeSelect} />
  }

  if (view === 'instance-list') {
    const schema = PLUGIN_SCHEMAS[selectedType!]
    return (
      <PluginInstanceList
        schema={schema}
        instances={instances}
        onBack={() => setView('type-select')}
        onCreate={handleCreate}
      />
    )
  }

  return null
}
```

---

## 10. 测试计划

### 10.1 单元测试

#### 后端核心

```rust
// crates/core/src/plugin/tests.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_type_display() {
        assert_eq!(PluginType::LlmBackend.to_string(), "llm_backend");
        assert_eq!(PluginType::DeviceAdapter.to_string(), "device_adapter");
    }

    #[test]
    fn test_plugin_type_category() {
        assert_eq!(PluginType::LlmBackend.category(), PluginCategory::Ai);
        assert_eq!(PluginType::DeviceAdapter.category(), PluginCategory::Devices);
    }

    #[tokio::test]
    async fn test_plugin_registry() {
        let registry = PluginRegistry::new();

        // 注册工厂
        let factory = Arc::new(MockFactory::new());
        registry.register_factory(factory).unwrap();

        // 创建实例
        let config = PluginConfig {
            id: "test-1".to_string(),
            name: "Test".to_string(),
            plugin_type: "mock".to_string(),
            enabled: true,
            config: json!({}),
            created_at: 0,
            updated_at: 0,
        };
        let id = registry.create_instance(config).await.unwrap();

        // 验证实例
        let instances = registry.list_instances();
        assert_eq!(instances.len(), 1);

        // 启动实例
        registry.start_instance(&id).await.unwrap();

        // 停止实例
        registry.stop_instance(&id).await.unwrap();

        // 删除实例
        registry.remove_instance(&id).await.unwrap();
        let instances = registry.list_instances();
        assert_eq!(instances.len(), 0);
    }
}
```

#### 前端组件

```typescript
// web/src/components/plugins/__tests__/UnifiedPluginCard.test.tsx

describe('UnifiedPluginCard', () => {
  const mockInstance: PluginInstance = {
    id: 'test-1',
    name: 'Test Plugin',
    plugin_type: 'ollama',
    enabled: true,
    running: true,
    config: { endpoint: 'http://localhost:11434' },
    stats: { avg_latency_ms: 45 },
  }

  const mockSchema: PluginUISchema = {
    id: 'ollama',
    name: 'Ollama',
    category: 'ai',
    icon: 'Server',
    fields: {},
  }

  it('renders plugin name and type', () => {
    render(<UnifiedPluginCard instance={mockInstance} schema={mockSchema} />)
    expect(screen.getByText('Test Plugin')).toBeInTheDocument()
  })

  it('shows running status when running', () => {
    render(<UnifiedPluginCard instance={mockInstance} schema={mockSchema} />)
    expect(screen.getByText('运行中')).toBeInTheDocument()
  })

  it('calls onToggle when switch is clicked', async () => {
    const onToggle = jest.fn().mockResolvedValue(true)
    render(
      <UnifiedPluginCard
        instance={mockInstance}
        schema={mockSchema}
        onToggle={onToggle}
      />
    )

    fireEvent.click(screen.getByRole('switch'))
    await waitFor(() => {
      expect(onToggle).toHaveBeenCalledWith('test-1', false)
    })
  })
})
```

### 10.2 集成测试

```rust
// tests/api/plugins_integration_test.rs

#[tokio::test]
async fn test_plugin_lifecycle() {
    let app = create_test_app().await;

    // 1. 列出可用类型
    let response = app
        .oneshot(Request::builder()
            .uri("/api/plugins/types")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 2. 创建实例
    let response = app
        .oneshot(Request::builder()
            .uri("/api/plugins")
            .method(Method::POST)
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "name": "Test Ollama",
                "plugin_type": "ollama",
                "config": {
                    "endpoint": "http://localhost:11434",
                    "model": "qwen3-vl:2b"
                }
            }).to_string()))
            .unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // 3. 获取实例
    let response = app
        .oneshot(Request::builder()
            .uri("/api/plugins/ollama-test-ollama-1")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 4. 启动实例
    let response = app
        .oneshot(Request::builder()
            .uri("/api/plugins/ollama-test-ollama-1/start")
            .method(Method::POST)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 5. 测试连接
    let response = app
        .oneshot(Request::builder()
            .uri("/api/plugins/ollama-test-ollama-1/test")
            .method(Method::POST)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 6. 删除实例
    let response = app
        .oneshot(Request::builder()
            .uri("/api/plugins/ollama-test-ollama-1")
            .method(Method::DELETE)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
```

### 10.3 端到端测试

```typescript
// tests/e2e/plugins.spec.ts

test.describe('Plugin Management', () => {
  test('should display plugin types', async ({ page }) => {
    await page.goto('/plugins')

    // 应该显示 Ollama 类型卡片
    await expect(page.locator('text=Ollama')).toBeVisible()
    await expect(page.locator('text=Local LLM runner')).toBeVisible()
  })

  test('should create a plugin instance', async ({ page }) => {
    await page.goto('/plugins')

    // 点击 Ollama 类型
    await page.click('text=Ollama')

    // 点击添加实例
    await page.click('button:has-text("Add Instance")')

    // 填写表单
    await page.fill('[name="name"]', 'My Ollama')
    await page.fill('[name="model"]', 'qwen3-vl:2b')

    // 提交
    await page.click('button:has-text("Save")')

    // 验证实例创建成功
    await expect(page.locator('text=My Ollama')).toBeVisible()
  })

  test('should test plugin connection', async ({ page }) => {
    await page.goto('/plugins')
    await page.click('text=Ollama')

    // 点击测试连接按钮
    await page.click('button:has-text("Test Connection")')

    // 验证成功消息
    await expect(page.locator('text=Connection successful')).toBeVisible()
  })
})
```

---

## 11. 风险评估

### 11.1 技术风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| **Trait 对象性能问题** | 中 | 中 | - 使用 Arc 智能指针<br>- 缓存常用操作<br>- 基准测试验证 |
| **Schema 扩展不兼容** | 低 | 高 | - 保持向后兼容<br>- 保留旧 API<br>- 渐进式迁移 |
| **动态加载安全性** | 高 | 高 | - 沙箱隔离<br>- 签名验证<br>- 权限控制 |
| **数据迁移失败** | 中 | 高 | - 迁移前备份<br>- 可回滚设计<br>- 测试环境验证 |

### 11.2 业务风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| **用户体验变化** | 中 | 中 | - UI/UX 测试<br>- 用户文档<br>- 培训材料 |
| **现有集成破坏** | 低 | 高 | - API 兼容层<br>- 长期共存期<br>- 通知机制 |
| **插件生态影响** | 低 | 中 | - 提供迁移指南<br>- 兼容性测试<br>- 开发者支持 |

### 11.3 迁移风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| **配置不兼容** | 中 | 中 | - 自动转换<br>- 手动校验<br>- 回滚机制 |
| **状态丢失** | 低 | 高 | - 原子操作<br>- 事务支持<br>- 备份恢复 |
| **服务中断** | 低 | 高 | - 蓝绿部署<br>- 灰度发布<br>- 快速回滚 |

### 11.4 回滚计划

```
如果迁移失败，按以下步骤回滚：

1. 停止新版本服务
2. 恢复旧版本二进制
3. 恢复数据库备份
4. 验证旧版本功能
5. 通知用户回滚原因
6. 分析失败原因并修复

回滚时间目标 (RTO): < 30 分钟
恢复点目标 (RPO): < 5 分钟
```

---

## 附录 A: 插件开发指南

### A.1 创建 LLM 后端插件

```rust
// 1. 实现 LlmRuntime trait
pub struct MyLlmRuntime {
    config: MyConfig,
}

#[async_trait]
impl LlmRuntime for MyLlmRuntime {
    fn backend_id(&self) -> BackendId {
        BackendId::new("my_llm")
    }

    async fn generate(&self, input: LlmInput) -> Result<LlmOutput, LlmError> {
        // 实现生成逻辑
    }
}

// 2. 创建工厂
pub struct MyLlmFactory;

impl PluginFactory for MyLlmFactory {
    fn factory_type(&self) -> &str {
        "my_llm"
    }

    async fn create(&self, config: &PluginConfig) -> Result<Arc<dyn Plugin>, PluginError> {
        let runtime = MyLlmRuntime::new(config.config.clone())?;
        let wrapper = LlmPluginWrapper::new(runtime, config.id.clone(), config.name.clone());
        Ok(Arc::new(wrapper))
    }

    fn ui_schema(&self) -> PluginUISchema {
        json!({
            "id": "my_llm",
            "name": "My LLM",
            "category": "ai",
            "icon": "BrainCircuit",
            "fields": {
                "api_key": {
                    "type": "password",
                    "label": "API Key",
                    "required": true
                }
            }
        })
    }
}

// 3. 注册工厂
registry.register_factory(Arc::new(MyLlmFactory))?;
```

### A.2 创建设备适配器插件

```rust
// 1. 实现 DeviceAdapter trait
pub struct MyDeviceAdapter {
    config: MyConfig,
    event_tx: broadcast::Sender<DeviceEvent>,
}

#[async_trait]
impl DeviceAdapter for MyDeviceAdapter {
    fn name(&self) -> &str {
        "my_adapter"
    }

    async fn start(&self) -> Result<(), AdapterError> {
        // 启动适配器
    }
}

// 2. 创建工厂和包装器
pub struct MyDeviceAdapterFactory;

impl PluginFactory for MyDeviceAdapterFactory {
    fn factory_type(&self) -> &str {
        "my_device_adapter"
    }

    async fn create(&self, config: &PluginConfig) -> Result<Arc<dyn Plugin>, PluginError> {
        let adapter = Arc::new(MyDeviceAdapter::new(config.config.clone())?);
        let wrapper = DeviceAdapterPluginWrapper::new(adapter, config.id.clone());
        Ok(Arc::new(wrapper))
    }
}
```

---

## 附录 B: Schema 字段类型扩展

### B.1 新增字段类型

| 类型 | 描述 | 示例值 |
|------|------|--------|
| `json` | JSON 编辑器 | `{"key": "value"}` |
| `key-value` | 键值对列表 | `[{key: "a", value: "1"}]` |
| `file` | 文件上传 | `/path/to/file` |
| `color` | 颜色选择器 | `#ff0000` |
| `slider` | 滑块数字 | `50` |
| `tags` | 标签输入 | `["tag1", "tag2"]` |
| `code` | 代码编辑器 | `function() {}` |
| `duration` | 时长选择 | `1h 30m` |

### B.2 字段验证扩展

```typescript
interface FieldSchema {
  // ... 现有字段

  // 新增验证规则
  pattern?: string           // 正则表达式
  minLength?: number         // 最小长度
  maxLength?: number         // 最大长度
  minDate?: string           // 最小日期
  maxDate?: string           // 最大日期
  customValidator?: string   // 自定义验证器名称
}
```

---

## 设备管理优化机会

### D.1 当前设备管理代码分析

通过分析现有设备管理代码，发现以下优化机会：

| 文件 | 行数 | 问题描述 | 优化建议 |
|------|------|----------|----------|
| `mqtt_v2.rs` | 1609 | 单一文件承担过多职责 | 拆分为多个模块 |
| `adapter_manager.rs` | 762 | 与 DeviceManager 功能重叠 | 考虑合并或明确边界 |
| `manager.rs` | 428 | 功能相对简单，可合并 | 作为统一 DeviceRegistry |

### D.2 MqttDeviceManager 拆分建议

**当前问题**：
```rust
// crates/devices/src/mqtt_v2.rs (1609 lines)
pub struct MqttDeviceManager {
    // 1. MQTT 客户端管理
    mqtt_client: Arc<RwLock<Option<MqttClientInner>>>,
    connection_status: Arc<RwLock<ConnectionStatus>>,

    // 2. 设备注册表
    devices: Arc<RwLock<HashMap<String, DeviceInstance>>>,

    // 3. MDL 注册表
    mdl_registry: Arc<MdlRegistry>,

    // 4. HASS 发现功能
    hass_discovered_devices: Arc<RwLock<HashMap<String, DiscoveredHassDevice>>>,
    hass_state_topic_map: Arc<RwLock<HashMap<String, (String, String)>>>,

    // 5. 时序存储
    time_series_storage: Arc<RwLock<Option<Arc<TimeSeriesStorage>>>>,

    // 6. 指标缓存
    metric_cache: Arc<RwLock<HashMap<...>>>,
}
```

**建议拆分**：

```
crates/devices/src/mqtt/
├── mod.rs                    # 模块导出
├── client.rs                 # MQTT 客户端封装
├── manager.rs                # 设备管理器 (核心逻辑)
├── discovery.rs              # 设备发现机制
└── hass/
    ├── mod.rs                # HASS 发现模块
    ├── discovery.rs          # HASS 发现处理
    └── aggregator.rs         # HASS 设备聚合
```

### D.3 统一设备注册表

**当前重复**：

```
DeviceManager (manager.rs):
├── register_device()
├── unregister_device()
├── get_device()
└── list_devices()

AdapterManager (adapter_manager.rs):
├── register()
├── unregister()
├── get_adapter()
└── list_adapters()
```

**统一后**：

```rust
// crates/core/src/device/registry.rs

pub struct UnifiedDeviceRegistry {
    /// 设备实例 (device_id -> Device)
    devices: Arc<RwLock<HashMap<String, DynDevice>>>,

    /// 适配器实例 (adapter_id -> Adapter)
    adapters: Arc<RwLock<HashMap<String, DynAdapter>>>,

    /// 设备与适配器关系 (device_id -> adapter_id)
    device_adapter_map: Arc<RwLock<HashMap<String, String>>>,
}

impl UnifiedDeviceRegistry {
    /// 注册适配器
    pub async fn register_adapter(&self, adapter: DynAdapter) -> Result<()>;

    /// 注册设备到适配器
    pub async fn register_device(&self, adapter_id: &str, device: DynDevice) -> Result<()>;

    /// 获取设备所属适配器
    pub async fn get_device_adapter(&self, device_id: &str) -> Option<DynAdapter>;

    /// 获取适配器下的所有设备
    pub async fn get_adapter_devices(&self, adapter_id: &str) -> Vec<DynDevice>;

    /// 统一事件流
    pub fn subscribe_events(&self) -> impl Stream<Item = DeviceEvent>;
}
```

### D.4 事件流优化

**当前问题**：
- 事件类型分散 (`DeviceEvent`, `ModifiedDeviceEvent`, `ManagerEvent`)
- 事件聚合在 AdapterManager 中完成

**优化方案**：
```rust
// 统一设备事件类型
pub enum UnifiedDeviceEvent {
    // 设备级别
    DeviceDiscovered { device: DiscoveredDevice },
    DeviceOnline { device_id: String },
    DeviceOffline { device_id: String },
    DeviceMetric { device_id: String, metric: MetricValue },

    // 适配器级别
    AdapterStarted { adapter_id: String },
    AdapterStopped { adapter_id: String },
    AdapterError { adapter_id: String, error: String },
}

// 统一事件发布器
pub trait EventPublisher: Send + Sync {
    fn publish(&self, event: UnifiedDeviceEvent);
    fn subscribe(&self) -> impl Stream<Item = UnifiedDeviceEvent>;
}
```

### D.5 发现机制优化

**当前问题**：
- HASS 发现与 MQTT 设备管理紧密耦合
- 发现协议硬编码在 MqttDeviceManager 中

**优化方案**：
```rust
// 抽象发现协议
pub trait DiscoveryProtocol: Send + Sync {
    fn protocol_name(&self) -> &str;
    fn discovery_topics(&self) -> Vec<String>;
    fn parse_discovery_message(&self, topic: &str, payload: &[u8]) -> Result<DiscoveredDevice>;
}

// HASS 发现实现
pub struct HassDiscoveryProtocol;

impl DiscoveryProtocol for HassDiscoveryProtocol {
    fn protocol_name(&self) -> &str { "hass" }
    fn discovery_topics(&self) -> Vec<String> {
        vec![
            "homeassistant/+/+/config".to_string(),
            "homeassistant/+/+/+/config".to_string(),
        ]
    }
    // ...
}

// 发现管理器
pub struct DiscoveryManager {
    protocols: HashMap<String, Arc<dyn DiscoveryProtocol>>,
}
```

### D.6 实施优先级

| 优先级 | 任务 | 预计时间 | 依赖 |
|--------|------|----------|------|
| P0 | 统一插件系统核心 | 1-2 周 | 无 |
| P1 | MqttDeviceManager 拆分 | 2-3 天 | P0 |
| P1 | 统一设备注册表 | 2-3 天 | P0 |
| P2 | 事件流优化 | 1-2 天 | P1 |
| P2 | 发现机制抽象 | 1-2 天 | P1 |

---

## 附录 E: MDL 系统完善计划

### E.1 当前 MDL 系统状态分析

**MDL (Machine Description Language)** 是 NeoTalk 的设备类型定义系统，用于描述设备的数据结构和交互方式。

#### 当前能力

通过分析 `crates/devices/src/` 代码，MDL 系统**已经支持**：

| 协议 | 支持状态 | 配置方式 |
|------|----------|----------|
| **MQTT** | ✅ 完整支持 | 自动发现 + 手动配置 |
| **Home Assistant (HASS)** | ✅ 完整支持 | MQTT Discovery 自动导入 |
| **Modbus TCP** | ✅ 完整支持 | 手动注册配置 |
| **HTTP API** | ⚠️ 部分支持 | 需要自定义类型 |

#### 内置设备类型

```rust
// crates/devices/src/builtin_types.rs

pub fn builtin_device_types() -> Vec<DeviceTypeDefinition> {
    vec![
        dht22_sensor(),      // 温湿度传感器
        relay_module(),      // 继电器模块
        energy_meter(),      // 电能计量
        air_quality_sensor(), // 空气质量 (PM2.5, CO2, TVOC)
        ip_camera(),         // IPC 摄像头
        image_sensor(),      // 图像传感器
    ]
}
```

#### 当前协议映射

```rust
// 已有协议映射支持
pub fn builtin_mqtt_mappings() -> HashMap<String, MqttMapping>      // MQTT 自动发现
pub fn builtin_modbus_mappings() -> HashMap<String, ModbusMapping>  // Modbus 寄存器映射
pub fn builtin_hass_mappings() -> HashMap<String, HassMapping>      // HASS 设备转换
```

### E.2 用户便捷性问题诊断

#### 问题 1: 设备类型配置门槛高

```
现状：添加新设备类型需要手动编写 JSON

用户操作流程：
1. 了解 MDL JSON 格式规范
2. 编写设备类型定义 (50+ 行 JSON)
3. 定义 uplink (数据上报) 配置
4. 定义 downlink (命令下发) 配置
5. 通过 API 或配置文件注册
6. 验证格式正确性

用户痛点：
❌ 需要理解 JSON 结构
❌ 需要了解 MQTT Topic 规则
❌ 需要手动配置每个指标
❌ 配置错误无法直观发现
```

#### 问题 2: 常见设备无模板

```
常见家用设备缺乏开箱即用配置：
- ✅ 有：DHT22 温湿度传感器
- ❌ 无：ESP32 系列通用配置
- ❌ 无：各种开关模块
- ❌ 无：常见传感器 (光照、人体、门磁等)
- ❌ 无：窗帘控制器
- ❌ 无：空调控制器
```

#### 问题 3: HASS 设备导入不完整

```
HASS Discovery 限制：
- 只能发现通过 HASS MQTT Discovery 的设备
- 需要设备正确发布 discovery 消息
- 不支持自动配置非标准设备
- 导入后可能需要手动调整
```

### E.3 用户便捷性改进方案

#### 方案 1: 可视化设备类型编辑器

**目标**: 让用户通过 Web UI 创建和编辑设备类型，无需手写 JSON

```typescript
// web/src/components/devices/DeviceTypeEditor.tsx

export function DeviceTypeEditor() {
  // 步骤式引导
  const steps = [
    { id: 'basic', title: '基本信息' },
    { id: 'uplink', title: '数据上报' },
    { id: 'downlink', title: '命令控制' },
    { id: 'protocol', title: '协议配置' },
    { id: 'preview', title: '预览确认' },
  ]

  return (
    <Stepper steps={steps}>
      {/* 步骤 1: 基本信息 */}
      <Step id="basic">
        <FormField name="device_type" label="设备类型 ID" placeholder="如: esp32_sensor" />
        <FormField name="name" label="显示名称" placeholder="如: ESP32 传感器" />
        <TagInput name="categories" label="分类" options={['sensor', 'switch', 'camera']} />
      </Step>

      {/* 步骤 2: 数据上报配置 */}
      <Step id="uplink">
        <MetricBuilder
          metrics={uplinkMetrics}
          onAdd={addMetric}
          onEdit={editMetric}
          // 支持常见模板快速选择
          templates={['temperature', 'humidity', 'pressure', 'lux']}
        />
      </Step>

      {/* 步骤 3: 命令控制配置 */}
      <Step id="downlink">
        <CommandBuilder
          commands={downlinkCommands}
          templates={['switch', 'dimmer', 'setpoint']}
        />
      </Step>

      {/* 步骤 4: 协议选择 */}
      <Step id="protocol">
        <ProtocolSelector
          selectedProtocol={protocol}
          onProtocolChange={setProtocol}
          protocols={[
            { id: 'mqtt', name: 'MQTT', icon: 'Network' },
            { id: 'modbus', name: 'Modbus TCP', icon: 'Cpu' },
            { id: 'hass', name: 'Home Assistant', icon: 'Home' },
          ]}
        />
        <ProtocolConfigForm protocol={protocol} />
      </Step>

      {/* 步骤 5: 预览 */}
      <Step id="preview">
        <JsonPreview value={generatedMdl} />
        <TestSimulation deviceType={generatedMdl} />
      </Step>
    </Stepper>
  )
}
```

#### 方案 2: 设备类型模板库

**目标**: 提供常见设备的开箱即用配置

```typescript
// web/src/lib/device-templates.ts

export const DEVICE_TEMPLATES: DeviceTemplate[] = [
  // 传感器模板
  {
    id: 'sensor-dht',
    name: '温湿度传感器',
    icon: 'Thermometer',
    categories: ['sensor', 'environment'],
    baseType: 'dht22_sensor',
    variants: [
      { name: 'DHT11', precision: 'integer' },
      { name: 'DHT22', precision: 'decimal' },
      { name: 'BME280', adds: ['pressure'] },
    ],
    quickConfig: {
      protocol: 'mqtt',
      topic: 'tele/{device_id}/SENSOR',
      parseMode: 'json', // 或 'tasmota', 'custom'
    },
  },

  // 开关模板
  {
    id: 'switch-relay',
    name: '继电器开关',
    icon: 'Toggle',
    categories: ['switch', 'actuator'],
    baseType: 'relay_module',
    variants: [
      { name: '单路继电器', channels: 1 },
      { name: '双路继电器', channels: 2 },
      { name: '四路继电器', channels: 4 },
      { name: '八路继电器', channels: 8 },
    ],
    quickConfig: {
      protocol: 'mqtt',
      commandTopic: 'cmnd/{device_id}/POWER',
      stateTopic: 'stat/{device_id}/POWER',
      payloadOn: 'ON',
      payloadOff: 'OFF',
    },
  },

  // 人体传感器
  {
    id: 'sensor-pir',
    name: '人体移动传感器',
    icon: 'User',
    categories: ['sensor', 'security'],
    uplinkMetrics: [
      { name: 'occupancy', type: 'boolean', label: '有人' },
      { name: 'battery', type: 'integer', label: '电池电量' },
    ],
  },

  // 光照传感器
  {
    id: 'sensor-lux',
    name: '光照传感器',
    icon: 'Sun',
    categories: ['sensor', 'environment'],
    uplinkMetrics: [
      { name: 'illuminance', type: 'float', unit: 'lux', label: '照度' },
    ],
  },

  // 空调控制器
  {
    id: 'hvac-ac',
    name: '空调控制器',
    icon: 'Wind',
    categories: ['hvac', 'actuator'],
    uplinkMetrics: [
      { name: 'temperature', type: 'float', unit: '°C' },
      { name: 'mode', type: 'enum', options: ['cool', 'heat', 'auto', 'dry', 'fan_only'] },
      { name: 'fan_speed', type: 'enum', options: ['low', 'medium', 'high', 'auto'] },
    ],
    downlinkCommands: [
      { name: 'set_temperature', type: 'number', min: 16, max: 30 },
      { name: 'set_mode', type: 'enum', options: ['cool', 'heat', 'auto', 'off'] },
      { name: 'set_fan_speed', type: 'enum', options: ['low', 'medium', 'high', 'auto'] },
    ],
  },

  // 窗帘控制器
  {
    id: 'cover-blind',
    name: '窗帘/卷帘控制器',
    icon: 'ArrowsVertical',
    categories: ['cover', 'actuator'],
    uplinkMetrics: [
      { name: 'position', type: 'integer', unit: '%', range: [0, 100] },
      { name: 'state', type: 'enum', options: ['open', 'closed', 'opening', 'closing'] },
    ],
    downlinkCommands: [
      { name: 'open', type: 'action' },
      { name: 'close', type: 'action' },
      { name: 'stop', type: 'action' },
      { name: 'set_position', type: 'number', range: [0, 100] },
    ],
  },
]
```

#### 方案 3: 一键设备发现与配置

**目标**: 自动发现网络中的设备并预生成配置

```typescript
// web/src/components/devices/DeviceDiscoveryWizard.tsx

export function DeviceDiscoveryWizard() {
  return (
    <Wizard>
      {/* 1. 选择发现方式 */}
      <DiscoveryStep>
        <DiscoveryMethod
          methods={[
            { id: 'mqtt', name: 'MQTT 扫描', description: '扫描 MQTT broker 中的设备' },
            { id: 'hass', name: 'HASS 导入', description: '从 Home Assistant 导入设备' },
            { id: 'modbus', name: 'Modbus 扫描', description: '扫描 Modbus 网络设备' },
            { id: 'manual', name: '手动添加', description: '手动输入设备信息' },
          ]}
        />
      </DiscoveryStep>

      {/* 2. 发现结果 */}
      <DiscoveryResultStep>
        <DiscoveredDeviceList
          devices={discoveredDevices}
          onSelect={selectDevice}
          actions={{
            'mqtt': {
              icon: 'Zap',
              label: '快速配置',
              handler: quickConfigureMqttDevice,
            },
            'hass': {
              icon: 'Download',
              label: '导入配置',
              handler: importHassDevice,
            },
          }}
        />
      </DiscoveryResultStep>

      {/* 3. 配置确认 */}
      <ConfigureStep>
        <DeviceTypeSuggester
          device={selectedDevice}
          suggestedTypes={suggestTypes(selectedDevice)}
          onConfirm={confirmConfiguration}
        />
      </ConfigureStep>
    </Wizard>
  )
}

// 智能类型推荐
function suggestTypes(device: DiscoveredDevice): DeviceTypeSuggestion[] {
  const suggestions = []

  // 基于 MQTT Topic 模式推荐
  if (device.topic?.includes('tele') && device.topic?.includes('SENSOR')) {
    suggestions.push({
      template: 'sensor-dht',
      confidence: 0.8,
      reason: '检测到 Tasmota SENSOR 消息格式',
    })
  }

  // 基于 HASS device_type 推荐
  if (device.hassDeviceInfo?.device_type === 'sensor') {
    suggestions.push({
      template: device.hassDeviceInfo?.model || 'sensor-generic',
      confidence: 0.95,
      reason: '从 HASS 发现的传感器设备',
    })
  }

  return suggestions
}
```

#### 方案 4: 设备类型市场 + AI 生成 (Device Type Marketplace + AI Generation)

**目标**: 社区分享设备类型配置，支持 AI 辅助生成，托管于 GitHub

##### 4.1 GitHub 托管的 MDL 模板库

```
GitHub 仓库结构: neotalk-device-types

├── device-types/
│   ├── sensors/
│   │   ├── dht22.json
│   │   ├── bme280.json
│   │   ├── pir-motion.json
│   │   └── bh1750-lux.json
│   ├── switches/
│   │   ├── sonoff-basic.json
│   │   ├── shelly-1pm.json
│   │   └── tasmota-4ch.json
│   ├── covers/
│   │   └── curtain-motor.json
│   └── hvac/
│       └── ac-controller.json
├── templates/           # 预定义模板片段
│   ├── metric-templates.json
│   └── command-templates.json
├── README.md           # 贡献指南
├── schema.json         # MDL JSON Schema
└── index.json          # 设备类型索引
```

##### 4.2 AI 生成 MDL 配置

用户只需描述设备，AI 自动生成 MDL 配置：

```typescript
// web/src/components/devices/AIMdlGenerator.tsx

export function AIMdlGenerator() {
  const [description, setDescription] = useState('')
  const [generatedMdl, setGeneratedMdl] = useState<DeviceTypeDefinition | null>(null)
  const [isGenerating, setIsGenerating] = useState(false)

  const handleGenerate = async () => {
    setIsGenerating(true)
    try {
      const result = await fetchAPI('/api/device-types/ai-generate', {
        method: 'POST',
        body: JSON.stringify({
          description,
          // 可选：提供现有设备作为参考
          referenceDevice: selectedReferenceDevice,
          // 可选：指定协议
          protocol: preferredProtocol,
        }),
      })
      setGeneratedMdl(result.deviceType)
    } finally {
      setIsGenerating(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>AI 生成设备类型配置</CardTitle>
        <CardDescription>
          用自然语言描述您的设备，AI 将自动生成 MDL 配置
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Textarea
          placeholder="例如：ESP32 连接的 DHT22 温湿度传感器，通过 MQTT 上报数据，Topic 格式为 tele/sensor1/SENSOR，JSON 格式包含 TEMP 和 HUMIDITY 字段"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={4}
        />
        <Button onClick={handleGenerate} disabled={isGenerating || !description}>
          {isGenerating ? <Spinner className="mr-2" /> : <Sparkles className="mr-2" />}
          AI 生成配置
        </Button>

        {generatedMdl && (
          <GeneratedMdlPreview
            mdl={generatedMdl}
            onEdit={(mdl) => setGeneratedMdl(mdl)}
            onSave={saveToDeviceTypes}
          />
        )}
      </CardContent>
    </Card>
  )
}
```

##### 4.3 AI 生成 API 设计

```http
# AI 生成设备类型
POST /api/device-types/ai-generate
Request: {
  "description": "ESP32 连接的 DHT22 温湿度传感器...",
  "reference_device": "dht22_sensor",    # 可选：参考设备
  "protocol": "mqtt",                     # 可选：指定协议
  "examples": [                           # 可选：提供示例消息
    {
      "topic": "tele/sensor1/SENSOR",
      "payload": '{"TEMP": 25.5, "HUMIDITY": 60.2}'
    }
  ]
}

Response: {
  "deviceType": {
    "device_type": "esp32_dht22_sensor",
    "name": "ESP32 DHT22 温湿度传感器",
    "categories": ["sensor", "environment"],
    "uplink": {
      "metrics": [
        { "name": "temperature", "type": "float", "unit": "°C", ... },
        { "name": "humidity", "type": "float", "unit": "%", ... }
      ]
    },
    "protocol_mappings": {
      "mqtt": {
        "uplink_topic": "tele/{device_id}/SENSOR",
        "parse_mode": "json",
        "field_mapping": { "TEMP": "temperature", "HUMIDITY": "humidity" }
      }
    }
  },
  "confidence": 0.95,
  "warnings": ["未找到湿度单位，已使用默认值 %"]
}
```

##### 4.4 GitHub 同步接口

```http
# 从 GitHub 同步设备类型库
POST /api/device-types/sync
Request: {
  "source": "github",
  "repository": "neotalk-device-types",
  "branch": "main"  # 可选，默认 main
}
Response: {
  "synced": 25,
  "updated": 3,
  "added": 22,
  "conflicts": [],
  "last_commit": "abc123"
}

# 获取同步状态
GET /api/device-types/sync/status
Response: {
  "last_sync": "2025-01-15T10:30:00Z",
  "last_commit": "abc123",
  "total_types": 25,
  "auto_sync_enabled": true,
  "sync_interval": "24h"
}

# 提交新设备类型到 GitHub (PR)
POST /api/device-types/contribute
Request: {
  "deviceType": DeviceTypeDefinition,
  "commit_message": "添加: XYZ 设备类型",
  "pr_title": "Add XYZ device type",
  "pr_body": "测试通过，兼容协议: MQTT"
}
Response: {
  "pr_url": "https://github.com/.../pull/123",
  "pr_number": 123
}

# 搜索 GitHub 上的设备类型
GET /api/device-types/search?query=温度传感器&source=github
Response: {
  "results": [
    {
      "id": "dht22_sensor",
      "name": "DHT22 温湿度传感器",
      "repository": "neotalk-device-types",
      "file": "device-types/sensors/dht22.json",
      "description": "...",
      "compatible_protocols": ["mqtt"],
      "stars": 42  # 社区点赞数
    }
  ]
}
```

##### 4.5 后端实现：GitHub 同步服务

```rust
// crates/devices/src/github_sync.rs

use octocrab::Octocrab;
use serde_json::Value;

pub struct GitHubMdlSync {
    client: Octocrab,
    repo_owner: String,
    repo_name: String,
    branch: String,
}

impl GitHubMdlSync {
    pub fn new(token: String, repo: String) -> Result<Self, SyncError> {
        let (owner, name) = parse_repo(&repo)?;
        let client = Octocrab::builder().personal_token(token).build()?;
        Ok(Self {
            client,
            repo_owner: owner,
            repo_name: name,
            branch: "main".to_string(),
        })
    }

    /// 从 GitHub 同步设备类型
    pub async fn sync_device_types(&self) -> Result<SyncResult, SyncError> {
        let mut result = SyncResult::default();

        // 1. 获取 index.json
        let index = self.fetch_file("index.json").await?;
        let index: DeviceTypeIndex = serde_json::from_slice(&index)?;

        // 2. 遍历每个设备类型文件
        for entry in &index.device_types {
            match self.fetch_device_type(&entry.file).await {
                Ok(device_type) => {
                    // 对比本地版本，决定是否更新
                    if self.needs_update(&device_type, &entry.sha).await? {
                        self.save_device_type(&device_type).await?;
                        result.updated += 1;
                    } else {
                        result.unchanged += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to sync {}: {}", entry.file, e);
                    result.errors.push(entry.file.clone());
                }
            }
        }

        Ok(result)
    }

    /// 提交新设备类型到 GitHub (创建 PR)
    pub async fn contribute_device_type(
        &self,
        device_type: DeviceTypeDefinition,
        pr_info: PRInfo,
    ) -> Result<ContributionResult, SyncError> {
        // 1. 创建新分支
        let branch_name = format!("add-{}", device_type.device_type);
        self.create_branch(&branch_name).await?;

        // 2. 写入设备类型文件
        let file_path = format!("device-types/{}.json", device_type.device_type);
        let content = serde_json::to_vec_pretty(&device_type)?;
        self.commit_file(&file_path, &content, &pr_info.commit_message).await?;

        // 3. 更新 index.json
        let mut index = self.fetch_and_update_index(&device_type).await?;
        self.commit_file("index.json", &serde_json::to_vec(&index)?, "Update index").await?;

        // 4. 创建 Pull Request
        let pr = self.client
            .pulls(&self.repo_owner, &self.repo_name)
            .create(pr_info.title, &branch_name, "main")
            .body(pr_info.body)
            .send()
            .await?;

        Ok(ContributionResult {
            pr_number: pr.number,
            pr_url: pr.html_url.to_string(),
        })
    }

    /// 搜索 GitHub 上的设备类型
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, SyncError> {
        let results = self.client
            .search()
            .issues_and_pull_requests(&format!(
                "repo:{}/{} {} in:file",
                self.repo_owner, self.repo_name, query
            ))
            .send()
            .await?;

        // 解析搜索结果...
    }
}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PRInfo {
    pub commit_message: String,
    pub pr_title: String,
    pub pr_body: String,
}

#[derive(Debug, Clone)]
pub struct ContributionResult {
    pub pr_number: u64,
    pub pr_url: String,
}
```

##### 4.6 自动同步配置

```toml
# config.toml

[device_types]
# GitHub 仓库同步
github_repository = "neotalk/neotalk-device-types"
github_branch = "main"

# 自动同步设置
auto_sync = true
sync_interval = "24h"          # 每 24 小时同步一次
sync_on_startup = true         # 启动时同步

# AI 生成配置
ai_generation_enabled = true
ai_backend = "ollama"          # 或 "openai", "anthropic"
ai_model = "qwen3-vl:2b"
```

##### 4.7 设备类型贡献流程

```
用户贡献设备类型到 GitHub:

1. 用户在 Web UI 创建/编辑设备类型
           ↓
2. 点击"贡献到社区"按钮
           ↓
3. AI 检查配置质量，自动优化
           ↓
4. 创建 GitHub PR
           ↓
5. 社区审核和测试
           ↓
6. 合并到主分支
           ↓
7. 所有用户自动获取更新
```

##### 4.8 设备类型包格式

```json
{
  "$schema": "https://raw.githubusercontent.com/neotalk/neotalk-device-types/main/schema.json",
  "api_version": 2,
  "device_type": "esp32_dht22_sensor",
  "name": "ESP32 DHT22 温湿度传感器",
  "version": "1.0.0",
  "author": "Your Name <email@example.com>",
  "categories": ["sensor", "environment"],
  "tags": ["temperature", "humidity", "esp32", "dht22", "mqtt"],
  "description": "基于 ESP32 的 DHT22 温湿度传感器，通过 MQTT 上报数据",
  "hardware": {
    "chip": "ESP32",
    "sensor": "DHT22",
    "firmware": "Tasmota / ESPHome"
  },
  "uplink": {
    "metrics": [
      {
        "name": "temperature",
        "type": "float",
        "unit": "°C",
        "label": "温度",
        "range": [-40, 80],
        "precision": 1
      },
      {
        "name": "humidity",
        "type": "float",
        "unit": "%",
        "label": "湿度",
        "range": [0, 100],
        "precision": 1
      }
    ]
  },
  "downlink": {
    "commands": [
      {
        "name": "request_update",
        "type": "action",
        "label": "请求更新"
      }
    ]
  },
  "protocol_mappings": {
    "mqtt": {
      "uplink_topic": "tele/{device_id}/SENSOR",
      "parse_mode": "json",
      "field_mapping": {
        "TEMP": "temperature",
        "HUMIDITY": "humidity"
      },
      "discovery": {
        "topic_pattern": "tele/+/SENSOR",
        "detect_keys": ["TEMP", "HUMIDITY"]
      }
    }
  },
  "metadata": {
    "created_at": "2025-01-15T10:00:00Z",
    "updated_at": "2025-01-15T10:00:00Z",
    "tested": true,
    "tested_by": ["community"],
    "compatibility": {
      "neotalk_min_version": "1.0.0"
    },
    "images": [
      "https://raw.githubusercontent.com/.../dht22_esp32.jpg"
    ]
  }
}
```

##### 4.9 完整 API 端点

```http
# GitHub 同步相关
POST   /api/device-types/sync                      # 同步 GitHub 仓库
GET    /api/device-types/sync/status              # 获取同步状态
POST   /api/device-types/contribute               # 贡献设备类型 (创建 PR)

# AI 生成相关
POST   /api/device-types/ai-generate              # AI 生成配置
POST   /api/device-types/ai-optimize              # AI 优化现有配置
POST   /api/device-types/ai-validate              # AI 验证配置质量

# 设备类型库搜索
GET    /api/device-types/library                  # 浏览本地+GitHub
GET    /api/device-types/library/:id              # 获取设备类型详情
POST   /api/device-types/library/install          # 安装设备类型
GET    /api/device-types/search                   # 搜索设备类型
```

### E.4 API 设计

#### 设备类型管理 API

```http
# 获取所有设备类型
GET /api/device-types
Response: {
  "builtin": [...],      # 内置类型
  "custom": [...],       # 用户自定义类型
  "installed": [...]     # 已安装的第三方类型
}

# 获取单个设备类型
GET /api/device-types/:type_id

# 创建设备类型
POST /api/device-types
Request: {
  "device_type": "my_sensor",
  "name": "My Sensor",
  "categories": ["sensor"],
  "uplink": { ... },
  "downlink": { ... },
  "protocol_mappings": {
    "mqtt": { ... },
    "modbus": { ... }
  }
}

# 更新设备类型
PUT /api/device-types/:type_id

# 删除设备类型
DELETE /api/device-types/:type_id

# 导出设备类型配置
GET /api/device-types/:type_id/export

# 导入设备类型配置
POST /api/device-types/import

# 验证设备类型配置
POST /api/device-types/validate
Request: { DeviceTypeDefinition }
Response: {
  "valid": true/false,
  "errors": [...],
  "warnings": [...]
}
```

#### 设备发现 API

```http
# 启动设备发现
POST /api/devices/discover
Request: {
  "method": "mqtt" | "hass" | "modbus",
  "config": { ... }
}
Response: {
  "discovery_id": "uuid",
  "status": "running"
}

# 获取发现结果
GET /api/devices/discover/:discovery_id
Response: {
  "status": "completed",
  "devices": [
    {
      "identifier": "tasmota-socket-1",
      "type_hint": "switch",
      "discovery_method": "mqtt",
      "suggested_template": "switch-relay",
      "confidence": 0.9,
      "raw_data": { ... }
    }
  ]
}

# 从发现结果创建设备
POST /api/devices/discover/:discovery_id/create
Request: {
  "device_index": 0,
  "device_id": "socket-1",
  "name": "客厅插座",
  "template": "switch-relay"
}
```

### E.5 实施优先级

| 优先级 | 功能 | 预计时间 | 用户价值 |
|--------|------|----------|----------|
| **P0** | 设备类型模板库 (20+ 常用设备) | 2-3 天 | ⭐⭐⭐⭐⭐ |
| **P0** | Web UI 可视化编辑器 | 3-4 天 | ⭐⭐⭐⭐⭐ |
| **P1** | 一键 HASS 导入优化 | 1-2 天 | ⭐⭐⭐⭐ |
| **P1** | MQTT 设备自动发现 | 2-3 天 | ⭐⭐⭐⭐ |
| **P2** | 设备类型市场 (社区分享) | 3-5 天 | ⭐⭐⭐ |
| **P2** | 智能类型推荐 | 2-3 天 | ⭐⭐⭐ |

### E.6 总结

**MDL 系统现状澄清**：
- ✅ **已支持多种协议**: MQTT、HASS、Modbus 都有完整支持
- ✅ **已有基础设备类型**: 6 个内置类型作为示例
- ❌ **用户配置门槛高**: 需要手写 JSON，不够便捷

**改进重点**：
1. **可视化编辑**: 降低配置门槛，让非技术用户也能创建设备类型
2. **模板库**: 提供常见设备的开箱即用配置
3. **智能发现**: 自动识别设备并推荐合适的类型配置
4. **GitHub 托管**: 社区驱动的设备类型库，支持版本控制和贡献
5. **AI 生成**: 自然语言描述设备，AI 自动生成 MDL 配置

**GitHub + AI 生态**：
```
┌─────────────────────────────────────────────────────────────────┐
│                    NeoTalk 设备类型生态                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │  用户描述   │───▶│  AI 生成    │───▶│  MDL 配置   │         │
│  │  (自然语言) │    │  (LLM)      │    │  (JSON)     │         │
│  └─────────────┘    └─────────────┘    └──────┬──────┘         │
│                                                  │               │
│                                                  ▼               │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │  贡献到     │◀───│  一键提交   │◀───│  配置验证   │         │
│  │  GitHub    │    │  PR         │    │  (AI+人工)  │         │
│  └──────┬──────┘    └─────────────┘    └─────────────┘         │
│         │                                                            │
│         ▼                                                            │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │           neotalk-device-types (GitHub 仓库)                │  │
│  │  ├── device-types/sensors/ (200+ 设备类型)                   │  │
│  │  ├── device-types/switches/                                  │  │
│  │  ├── device-types/covers/                                    │  │
│  │  └── index.json (自动索引)                                    │  │
│  └─────────────────────────────────────────────────────────────┘  │
│         │                                                            │
│         ▼                                                            │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │  自动同步   │───▶│  全体用户   │───▶│  开箱即用   │         │
│  │  (24h)      │    │  获取更新   │    │  设备支持   │         │
│  └─────────────┘    └─────────────┘    └─────────────┘         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**预期效果**：
- 用户添加新设备时间从 **30 分钟降低到 2 分钟**
- 常见设备**开箱即用**，无需手动配置
- AI 生成准确率达到 **90%+**
- 社区贡献积累，设备类型库**持续增长**（目标 200+ 设备类型）

---

## 总结

本计划定义了统一插件系统的完整实施方案，包括：

1. **11 个阶段**，预计 25-35 天完成
2. **30+ 新增文件**，覆盖后端和前端
3. **15+ API 端点**，统一插件管理接口
4. **完整测试覆盖**，包括单元、集成、端到端测试
5. **设备管理优化**，代码可维护性提升
6. **MDL 系统完善**，用户便捷性大幅提升

**关键成功因素**：
- ✅ 插件类型无限制（通过 `Custom(String)` 变体）
- ✅ 保持向后兼容
- ✅ Schema 驱动的灵活 UI
- ✅ 渐进式迁移策略
- ✅ 充分的测试保障
- ✅ 设备管理代码同步优化
- ✅ 用户友好的设备类型配置体验
