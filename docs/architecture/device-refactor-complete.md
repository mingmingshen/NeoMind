# 设备管理完整重构方案（平滑迁移）

## 一、现状分析

### 1.1 现有架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        现有系统架构                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  前端                                                                     │
│  ├── DeviceList.tsx              - 设备列表                              │
│  ├── DeviceTypeList.tsx          - 设备类型管理                          │
│  ├── DeviceTypeDialogs.tsx       - 类型对话框（已有AI生成）              │
│  ├── AddDeviceDialog.tsx         - 添加设备对话框                        │
│  └── HassDiscoveryDialog.tsx     - HASS设备发现对话框                    │
│                                                                         │
│  后端 API (crates/api/src/handlers/devices/)                            │
│  ├── crud.rs                      - 设备CRUD                              │
│  ├── types.rs                     - 设备类型API                           │
│  ├── hass.rs                      - HASS集成（15KB）                       │
│  ├── mdl.rs                       - MDL生成                               │
│  ├── discovery.rs                 - 设备发现                              │
│  └── telemetry.rs                 - 遥测数据                              │
│                                                                         │
│  核心管理 (crates/devices/src/)                                          │
│  ├── mqtt_v2.rs                   - MqttDeviceManager（核心）             │
│  ├── mdl_format.rs                - DeviceTypeDefinition                 │
│  └── telemetry.rs                 - 时序存储                              │
│                                                                         │
│  存储 (crates/storage/src/)                                              │
│  └── device_state.rs               - DeviceState（无adapter_type）        │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.2 关键发现

1. **HASS 已集成**：系统已有完整的 HASS MQTT Discovery 支持
2. **统一入口**：所有设备通过 `mqtt_device_manager` 管理
3. **前端 AI 生成**：已有 AI 辅助生成设备类型的功能
4. **设备类型耦合**：`DeviceTypeDefinition` 包含 MQTT 特定字段

---

## 二、重构目标

1. **多协议支持**：MQTT、HASS、Modbus、HTTP
2. **用户体验简单**：≤2步添加设备
3. **平滑迁移**：不破坏现有功能
4. **可扩展**：易于添加新协议

---

## 三、数据结构改动（扩展，不破坏）

### 3.1 DeviceState 扩展

```rust
// crates/storage/src/device_state.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    pub device_id: String,
    pub device_type: String,

    // ===== 新增字段（可选，有默认值） =====
    /// 适配器类型（默认 "mqtt"）
    #[serde(default = "default_adapter_type")]
    pub adapter_type: String,

    /// 连接配置（可选）
    #[serde(default)]
    pub connection_config: Option<ConnectionConfig>,

    // ===== 现有字段保持不变 =====
    pub online: bool,
    pub last_seen: i64,
    pub last_updated: i64,
    pub metrics: HashMap<String, MetricValue>,
    pub capabilities: Option<DeviceCapabilities>,
    #[serde(flatten)]
    pub properties: HashMap<String, serde_json::Value>,
}

fn default_adapter_type() -> String {
    "mqtt".to_string()
}

/// 连接配置（协议特定）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "adapter_type")]
pub enum ConnectionConfig {
    #[serde(rename = "mqtt")]
    Mqtt {
        telemetry_topic: String,
        command_topic: String,
        state_topic: Option<String>,
        #[serde(default)]
        qos: u8,
    },
    #[serde(rename = "home_assistant")]
    HomeAssistant {
        entity_id: String,
        state_topic: String,
        command_topic: String,
    },
    #[serde(rename = "modbus_tcp")]
    ModbusTcp {
        host: String,
        port: u16,
        slave_id: u8,
        start_address: u16,
    },
    #[serde(rename = "http")]
    Http {
        base_url: String,
        endpoint: String,
        poll_interval_secs: u64,
    },
}
```

### 3.2 DeviceTypeDefinition 拆分

```rust
// crates/devices/src/mdl_format.rs

/// 设备能力定义（协议无关）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapability {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub data_type: String,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// 设备类型定义（协议无关）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTypeDefinition {
    pub device_type: String,
    pub name: String,
    pub description: String,
    pub categories: Vec<String>,

    // 能力定义（协议无关）
    pub capabilities: Vec<DeviceCapability>,

    // 命令定义（协议无关）
    pub commands: Vec<CommandDefinition>,

    // ===== 兼容字段（保留但标记为 deprecated） =====
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uplink: Option<UplinkConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downlink: Option<DownlinkConfig>,
}
```

### 3.3 设备模板定义

```rust
// crates/devices/src/template.rs - 新文件

/// 设备模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: Option<String>,

    // 用户需要填写的参数
    pub user_params: Vec<TemplateParam>,

    // 能力预览
    pub capabilities: Vec<String>,  // ["温度", "湿度"]
    pub commands: Vec<String>,      // ["打开", "关闭"]

    // 协议配置
    pub adapter_type: String,
    pub adapter_config: AdapterConfigTemplate,

    // 预设映射（用于生成设备类型）
    pub capability_mappings: Vec<CapabilityMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParam {
    pub name: String,
    pub label: String,
    pub param_type: TemplateParamType,
    pub required: bool,
    pub default: Option<String>,
    pub placeholder: Option<String>,
    pub help: Option<String>,
    pub validation: Option<ValidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateParamType {
    String,
    Number,
    Boolean,
    Select { options: Vec<SelectOption> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub pattern: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// 适配器配置模板（支持变量替换）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "adapter_type")]
pub enum AdapterConfigTemplate {
    #[serde(rename = "mqtt")]
    Mqtt {
        telemetry_topic: String,  // 支持 {{param_name}}
        command_topic: String,
        json_path: Option<String>,
    },
    #[serde(rename = "home_assistant")]
    HomeAssistant {
        state_topic: String,
        command_topic: String,
    },
    #[serde(rename = "modbus_tcp")]
    ModbusTcp {
        port: u16,  // 固定值
        start_address_template: String,
    },
}

/// 能力映射（从语义到协议）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMapping {
    pub capability_name: String,  // "temperature"
    pub mapping: Mapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mapping_type")]
pub enum Mapping {
    #[serde(rename = "mqtt_topic")]
    MqttTopic {
        topic: String,
        json_path: Option<String>,
        value_transform: Option<ValueTransform>,
    },
    #[serde(rename = "hass_state")]
    HassState {
        entity_id: String,
        attribute: Option<String>,
    },
    #[serde(rename = "modbus_register")]
    ModbusRegister {
        address_offset: u16,
        register_type: String,
        data_type: String,
    },
}
```

---

## 四、内置模板（支持多协议）

```rust
// crates/devices/src/builtin_templates.rs

pub fn builtin_templates() -> Vec<DeviceTemplate> {
    vec![
        // ========== MQTT 模板 ==========
        DeviceTemplate {
            id: "mqtt-dht22".into(),
            name: "MQTT DHT22 温湿度传感器".into(),
            description: "通过 MQTT 接入的 DHT22/DHT11 传感器".into(),
            category: "sensor".into(),
            icon: Some("thermometer".into()),
            user_params: vec![
                TemplateParam {
                    name: "device_name".into(),
                    label: "设备名称".into(),
                    param_type: TemplateParamType::String,
                    required: true,
                    default: None,
                    placeholder: Some("如：客厅温度".into()),
                    help: Some("给设备起一个好记的名字".into()),
                    validation: None,
                },
                TemplateParam {
                    name: "base_topic".into(),
                    label: "MQTT 主题".into(),
                    param_type: TemplateParamType::String,
                    required: true,
                    default: None,
                    placeholder: Some("sensors/livingroom".into()),
                    help: Some("设备数据发布的主题前缀".into()),
                    validation: Some(ValidationRule {
                        pattern: Some(r"^[a-zA-Z0-9/_\-]+$".into()),
                        min: None,
                        max: None,
                    }),
                },
            ],
            capabilities: vec!["温度".into(), "湿度".into()],
            commands: vec![],
            adapter_type: "mqtt".into(),
            adapter_config: AdapterConfigTemplate::Mqtt {
                telemetry_topic: "{{base_topic}}/tele".into(),
                command_topic: "{{base_topic}}/cmd".into(),
                json_path: None,
            },
            capability_mappings: vec![
                CapabilityMapping {
                    capability_name: "temperature".into(),
                    mapping: Mapping::MqttTopic {
                        topic: "{{base_topic}}/tele".into(),
                        json_path: "$.temp".into(),
                        value_transform: None,
                    },
                },
                CapabilityMapping {
                    capability_name: "humidity".into(),
                    mapping: Mapping::MqttTopic {
                        topic: "{{base_topic}}/tele".into(),
                        json_path: "$.hum".into(),
                        value_transform: None,
                    },
                },
            ],
        },

        // ========== HASS 模板 ==========
        DeviceTemplate {
            id: "hass-light".into(),
            name: "HASS 智能灯".into(),
            description: "Home Assistant 中的智能灯具".into(),
            category: "light".into(),
            icon: Some("lightbulb".into()),
            user_params: vec![
                TemplateParam {
                    name: "device_name".into(),
                    label: "设备名称".into(),
                    param_type: TemplateParamType::String,
                    required: true,
                    default: None,
                    placeholder: Some("如：客厅灯".into()),
                    help: None,
                    validation: None,
                },
                TemplateParam {
                    name: "entity_id".into(),
                    label: "实体 ID".into(),
                    param_type: TemplateParamType::String,
                    required: true,
                    default: None,
                    placeholder: Some("light.living_room".into()),
                    help: Some("Home Assistant 中的实体 ID".into()),
                    validation: Some(ValidationRule {
                        pattern: Some(r"^[a-z_]+\.[a-z0-9_]+$".into()),
                        min: None,
                        max: None,
                    }),
                },
            ],
            capabilities: vec!["状态".into(), "亮度".into(), "颜色".into()],
            commands: vec!["打开".into(), "关闭".into(), "设置亮度".into()],
            adapter_type: "home_assistant".into(),
            adapter_config: AdapterConfigTemplate::HomeAssistant {
                state_topic: "{{entity_id}}/state".into(),
                command_topic: "{{entity_id}}/command".into(),
            },
            capability_mappings: vec![
                CapabilityMapping {
                    capability_name: "state".into(),
                    mapping: Mapping::HassState {
                        entity_id: "{{entity_id}}".into(),
                        attribute: None,
                    },
                },
                CapabilityMapping {
                    capability_name: "brightness".into(),
                    mapping: Mapping::HassState {
                        entity_id: "{{entity_id}}".into(),
                        attribute: Some("brightness".into()),
                    },
                },
            ],
        },

        // ========== Modbus 模板 ==========
        DeviceTemplate {
            id: "modbus-energy-meter".into(),
            name: "Modbus 电表".into(),
            description: "通过 Modbus TCP 接入的电表".into(),
            category: "energy".into(),
            icon: Some("zap".into()),
            user_params: vec![
                TemplateParam {
                    name: "device_name".into(),
                    label: "设备名称".into(),
                    param_type: TemplateParamType::String,
                    required: true,
                    default: None,
                    placeholder: Some("如：1号楼电表".into()),
                    help: None,
                    validation: None,
                },
                TemplateParam {
                    name: "host".into(),
                    label: "设备地址".into(),
                    param_type: TemplateParamType::String,
                    required: true,
                    default: None,
                    placeholder: Some("192.168.1.100".into()),
                    help: Some("Modbus 设备的 IP 地址".into()),
                    validation: None,
                },
                TemplateParam {
                    name: "slave_id".into(),
                    label: "从站地址".into(),
                    param_type: TemplateParamType::Number,
                    required: true,
                    default: Some("1".into()),
                    placeholder: None,
                    help: Some("Modbus 从站 ID".into()),
                    validation: Some(ValidationRule {
                        pattern: None,
                        min: Some(1.0),
                        max: Some(247.0),
                    }),
                },
            ],
            capabilities: vec!["电压".into(), "电流".into(), "功率".into(), "电能".into()],
            commands: vec![],
            adapter_type: "modbus_tcp".into(),
            adapter_config: AdapterConfigTemplate::ModbusTcp {
                port: 502,
                start_address_template: "0x0000".into(),
            },
            capability_mappings: vec![
                CapabilityMapping {
                    capability_name: "voltage".into(),
                    mapping: Mapping::ModbusRegister {
                        address_offset: 0,
                        register_type: "holding".into(),
                        data_type: "float32".into(),
                    },
                },
                // ... 更多映射
            ],
        },
    ]
}
```

---

## 五、API 改动（新增，不破坏现有）

### 5.1 新增模板 API

```rust
// crates/api/src/handlers/devices/templates.rs - 新文件

use axum::{extract::State, Json};
use crate::handlers::{HandlerResult, ServerState};
use edge_ai_devices::template::builtin_templates;

/// 获取所有模板
///
/// GET /api/devices/templates
pub async fn list_templates_handler(
    State(_state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let templates = builtin_templates();
    Ok(json!({
        "templates": templates,
        "count": templates.len(),
    }))
}

/// 获取单个模板详情
///
/// GET /api/devices/templates/:id
pub async fn get_template_handler(
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let template = builtin_templates()
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| ErrorResponse::not_found("Template"))?;

    Ok(json!(template))
}

/// 从模板创建设备
///
/// POST /api/devices/templates/:id/create
pub async fn create_from_template_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(req): Json<CreateFromTemplateRequest>,
) -> HandlerResult<serde_json::Value> {
    // 1. 获取模板
    let template = builtin_templates()
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| ErrorResponse::not_found("Template"))?;

    // 2. 验证参数
    for param in &template.user_params {
        if param.required && !req.params.contains_key(&param.name) {
            return Err(ErrorResponse::bad_request(&format!(
                "缺少必填参数: {}", param.label
            )));
        }
    }

    // 3. 替换模板变量
    let config = fill_template_vars(&template.adapter_config, &req.params);

    // 4. 生成设备类型
    let device_type_def = generate_device_type_from_template(&template, &req.params);

    // 5. 注册设备类型
    state.mqtt_device_manager
        .register_device_type(device_type_def.clone())
        .await?;

    // 6. 创建设备实例
    let device_name = req.params.get("device_name")
        .unwrap()
        .clone();
    let device_id = state.mqtt_device_manager
        .add_device(
            "".into(),  // 自动生成
            device_type_def.device_type.clone(),
            Some(device_name),
            Some(template.adapter_type.clone()),  // adapter_id
            req.params.clone(),  // config
        )
        .await?;

    Ok(json!({
        "device_id": device_id,
        "template_id": template.id,
        "created": true,
    }))
}

#[derive(Deserialize)]
pub struct CreateFromTemplateRequest {
    pub params: std::collections::HashMap<String, String>,
}
```

### 5.2 修改设备创建 API（兼容）

```rust
// crates/api/src/handlers/devices/crud.rs

/// Add a new device（保持兼容，内部适配）
pub async fn add_device_handler(
    State(state): State<ServerState>,
    Json(req): Json<AddDeviceRequest>,
) -> HandlerResult<serde_json::Value> {
    // 现有逻辑保持不变
    let device_id = if let Some(id) = req.device_id {
        id
    } else {
        let random_str: String = Uuid::new_v4()
            .to_string()
            .replace('-', "")
            .chars()
            .take(8)
            .collect();
        format!("{}_{}", req.device_type, random_str)
    };

    // 新增：设置 adapter_type 默认值
    let mut config = req.config.clone().unwrap_or_default();
    if !config.contains_key("adapter_type") {
        config.insert("adapter_type".to_string(), json!("mqtt"));
    }

    state.mqtt_device_manager
        .add_device(device_id.clone(), req.device_type, req.name.clone(), None, config)
        .await?;

    ok(json!({
        "device_id": device_id,
        "added": true,
    }))
}
```

### 5.3 路由注册

```rust
// crates/api/src/handlers/devices/mod.rs

use super::*;
use axum::{Router, routing::get};
use axum::routing::post;

pub fn routes() -> Router<Arc<ServerState>> {
    Router::new()
        // 现有路由（保持不变）
        .route("/", get(list_devices_handler).post(add_device_handler))
        .route("/:id", get(get_device_handler).delete(delete_device_handler))
        .route("/:id/command", post(send_command_handler))
        .route("/:id/telemetry", get(get_device_telemetry_handler))
        .route("/types", get(list_device_types_handler))
        // ... 其他现有路由

        // HASS 路由（保持不变）
        .route("/hass/discover", post(discover_hass_devices_handler))
        .route("/hass/discovered", get(get_hass_discovered_devices_handler))
        .route("/hass/register-aggregated", post(register_aggregated_hass_device_handler))
        // ... 其他 HASS 路由

        // 新增：模板路由
        .route("/templates", get(list_templates_handler))
        .route("/templates/:id", get(get_template_handler))
        .route("/templates/:id/create", post(create_from_template_handler))
}
```

---

## 六、前端改动（新增，渐进式）

### 6.1 新增模板选择组件

```typescript
// web/src/components/devices/TemplateSelectDialog.tsx - 新文件

import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"

interface Template {
  id: string
  name: string
  description: string
  category: string
  icon?: string
  user_params: {
    name: string
    label: string
    type: "string" | "number" | "boolean" | "select"
    required: boolean
    default?: string
    placeholder?: string
    help?: string
  }[]
  capabilities: string[]
  commands: string[]
}

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  onDeviceAdded: () => void
}

export function TemplateSelectDialog({ open, onOpenChange, onDeviceAdded }: Props) {
  const { t } = useTranslation(['common', 'devices'])
  const [templates, setTemplates] = useState<Template[]>([])
  const [selected, setSelected] = useState<Template | null>(null)
  const [step, setStep] = useState(1)
  const [params, setParams] = useState<Record<string, string>>({})
  const [adding, setAdding] = useState(false)

  useEffect(() => {
    if (open) loadTemplates()
  }, [open])

  const loadTemplates = async () => {
    const res = await fetch("/api/devices/templates")
    const data = await res.json()
    setTemplates(data.templates)
  }

  const handleAdd = async () => {
    if (!selected) return
    setAdding(true)
    try {
      await fetch(`/api/devices/templates/${selected.id}/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ params }),
      })
      onDeviceAdded()
      onOpenChange(false)
      reset()
    } finally {
      setAdding(false)
    }
  }

  const reset = () => {
    setStep(1)
    setSelected(null)
    setParams({})
  }

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) reset(); onOpenChange(v) }}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t('devices:add.title')}</DialogTitle>
        </DialogHeader>

        {step === 1 && (
          <div className="space-y-4">
            {/* 按分类分组 */}
            {["sensor", "switch", "light", "energy"].map(category => {
              const filtered = templates.filter(t => t.category === category)
              if (filtered.length === 0) return null
              return (
                <div key={category}>
                  <h3 className="text-sm font-medium text-muted-foreground mb-2 capitalize">
                    {t(`devices:category.${category}`, category)}
                  </h3>
                  <div className="grid grid-cols-2 gap-3">
                    {filtered.map(t => (
                      <button
                        key={t.id}
                        onClick={() => { setSelected(t); setStep(2) }}
                        className="p-4 border rounded-lg text-left hover:border-primary transition-colors"
                      >
                        <div className="font-medium">{t.name}</div>
                        <div className="text-sm text-muted-foreground">{t.description}</div>
                        <Badge variant="outline" className="mt-2 text-xs">
                          {t.user_params.filter(p => p.required).length} 个参数
                        </Badge>
                      </button>
                    ))}
                  </div>
                </div>
              )
            })}
          </div>
        )}

        {step === 2 && selected && (
          <div className="space-y-4">
            <Button variant="ghost" size="sm" onClick={() => setStep(1)}>
              ← {t('common:back')}
            </Button>

            <div>
              <h3 className="font-medium">{selected.name}</h3>
              <p className="text-sm text-muted-foreground">{selected.description}</p>
            </div>

            <div className="space-y-4">
              {selected.user_params.map(param => (
                <div key={param.name} className="space-y-2">
                  <Label>
                    {param.label}
                    {param.required && <span className="text-red-500"> *</span>}
                  </Label>
                  <Input
                    value={params[param.name] || ""}
                    onChange={e => setParams({ ...params, [param.name]: e.target.value })}
                    placeholder={param.placeholder}
                    required={param.required}
                  />
                  {param.help && (
                    <p className="text-xs text-muted-foreground">{param.help}</p>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('common:cancel')}
          </Button>
          {step === 2 && (
            <Button onClick={handleAdd} disabled={adding}>
              {adding ? t('common:adding') : t('common:add')}
            </Button>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
```

### 6.2 修改设备列表（渐进式）

```typescript
// web/src/pages/devices/DeviceList.tsx

// 新增状态
const [templateDialogOpen, setTemplateDialogOpen] = useState(false)
const [addMode, setAddMode] = useState<'template' | 'manual'>('template')

// 修改添加按钮区域
<div className="flex items-center gap-2">
  <Button onClick={() => setTemplateDialogOpen(true)}>
    <Plus className="mr-2 h-4 w-4" />
    {t('devices:add.title')}
  </Button>

  {/* 保留旧的手动添加入口 */}
  <Button variant="outline" onClick={() => setAddDeviceDialogOpen(true)}>
    <FileJson className="mr-2 h-4 w-4" />
    {t('devices:add.manual')}
  </Button>
</div>

{/* 新增模板对话框 */}
<TemplateSelectDialog
  open={templateDialogOpen}
  onOpenChange={setTemplateDialogOpen}
  onDeviceAdded={loadDevices}
/>

{/* 保留原有的 AddDeviceDialog */}
<AddDeviceDialog
  open={addDeviceDialogOpen}
  onOpenChange={setAddDeviceDialogOpen}
  onDeviceAdded={loadDevices}
  deviceTypes={deviceTypes}
/>

{/* 保留原有的 HASS Discovery 对话框 */}
<HassDiscoveryDialog ... />
```

### 6.3 修改设备列表显示适配器

```typescript
// web/src/pages/devices/DeviceList.tsx

// 设备表格新增适配器列
<TableHeader>
  {/* ... 现有列 ... */}
  <TableHead align="center">{t('devices:list.adapter')}</TableHead>
  <TableHead align="right">{t('devices:list.actions')}</TableHead>
</TableHeader>

<TableRow>
  {/* ... 现有列 ... */}
  <TableCell align="center">
    <Badge variant="outline" className="text-xs">
      {device.adapter_type || 'mqtt'}
    </Badge>
  </TableCell>
  <TableCell align="right">
    {/* 操作按钮 */}
  </TableCell>
</TableRow>
```

---

## 七、实施步骤

### 阶段 1：底层扩展（2天）

**目标**：扩展数据结构，不破坏现有功能

1. **DeviceState 添加字段**
   - `adapter_type`（默认 "mqtt"）
   - `connection_config`（可选）

2. **创建模板系统**
   - `template.rs`：模板定义
   - `builtin_templates.rs`：内置模板
   - 支持 MQTT、HASS 各 2 个模板

3. **设备类型兼容**
   - `DeviceTypeDefinition` 添加新字段
   - 保留 `uplink/downlink`（标记 deprecated）

### 阶段 2：API 扩展（1天）

**目标**：新增模板 API，保持现有 API

1. **模板 API**
   - `GET /api/devices/templates`
   - `GET /api/devices/templates/:id`
   - `POST /api/devices/templates/:id/create`

2. **设备创建 API 兼容**
   - 现有 API 继续工作
   - 新设置 `adapter_type` 默认值

### 阶段 3：前端扩展（2天）

**目标**：新增模板选择，保留现有界面

1. **新组件**
   - `TemplateSelectDialog.tsx`

2. **修改现有组件**
   - `DeviceList.tsx`：添加模板入口
   - 设备列表显示适配器列

3. **保留现有组件**
   - `AddDeviceDialog.tsx`
   - `HassDiscoveryDialog.tsx`

### 阶段 4：测试（1天）

**目标**：确保功能正常，兼容性良好

1. **功能测试**
   - MQTT 设备创建（新旧方式）
   - HASS 设备发现和创建

2. **兼容性测试**
   - 现有设备正常显示
   - 现有 API 继续工作

---

## 八、向后兼容性

### 8.1 数据兼容

```rust
// 现有设备自动添加 adapter_type
impl DeviceState {
    pub fn ensure_adapter_type(&mut self) {
        if self.adapter_type.is_empty() {
            self.adapter_type = "mqtt".to_string();
        }
    }
}

// 启动时迁移
pub async fn migrate_devices(store: &DeviceStateStore) -> Result<()> {
    let devices = store.list_devices().await?;
    for mut device in devices {
        if device.adapter_type.is_empty() {
            device.adapter_type = "mqtt".to_string();
            store.save_state(&device).await?;
        }
    }
    Ok(())
}
```

### 8.2 API 兼容

```
旧 API 继续可用：
POST /api/devices
{
  "device_type": "dht22",
  "name": "客厅温度"
}
→ adapter_type 默认设为 "mqtt"

新 API（推荐）：
POST /api/devices/templates/mqtt-dht22/create
{
  "params": {
    "device_name": "客厅温度",
    "base_topic": "sensors/livingroom"
  }
}
```

### 8.3 前端兼容

```
旧界面继续可用：
- 手动添加设备对话框
- HASS 发现对话框
- 设备类型管理

新界面（推荐）：
- 模板选择对话框
- 一键添加设备
```

---

## 九、工作量估算

| 阶段 | 任务 | 工作量 |
|------|------|--------|
| **阶段 1** | 底层扩展 | 2天 |
| **阶段 2** | API 扩展 | 1天 |
| **阶段 3** | 前端扩展 | 2天 |
| **阶段 4** | 测试 | 1天 |
| **总计** | | **6天** |

---

## 十、关键点总结

1. **不破坏现有功能**：所有现有 API 和前端组件继续工作
2. **渐进式迁移**：新旧方式并存，用户逐步过渡
3. **数据自动迁移**：启动时自动填充 `adapter_type`
4. **多协议支持**：模板系统支持任意协议扩展
5. **用户友好**：模板选择 ≤2 步完成设备添加
