# NeoTalk AI-Native Automation Implementation Plan

## 愿景目标

**核心差异**：用户只需将设备接入，不需要根据不同设备的协议去定义上层业务，一切交给AI来定义。

**解决的问题**：
- 场景碎片化：每个场景都需要手动配置
- 需求碎片化：用户需求多变，配置工作量大
- 协议碎片化：设备协议多样，适配工作复杂

---

## 实施阶段概览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        实施阶段时间线                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Phase 1: 数据理解自动化                    │
│  ├── DataPathExtractor        (3-4天)                                     │
│  ├── SemanticInference       (2-3天)                                     │
│  └── VirtualMetricGenerator  (2-3天)                                     │
│                              小计: 1-1.5周                               │
│                                                                             │
│  Phase 2: 自动化生成完整化                │
│  ├── Enhanced IntentAnalyzer (3-4天)                                   │
│  ├── NL2Automation            (4-5天)                                   │
│  └── ThresholdRecommender     (2-3天)                                   │
│                              小计: 1.5-2周                              │
│                                                                             │
│  Phase 3: 零配置设备接入                    │
│  ├── DeviceTypeGenerator      (3-4天)                                   │
│  ├── AutoDiscovery            (2-3天)                                   │
│  └── QuickImport              (1-2天)                                   │
│                              小计: 1-1.5周                               │
│                                                                             │
│  Phase 4: 测试验证                    │
│  ├── Unit Tests                (2天)                                     │
│  ├── Integration Tests         (2天)                                     │
│  └── User Acceptance           (2天)                                     │
│                              小计: 0.5-1周                               │
│                                                                             │
│                              总计: 4-6周                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: 数据理解自动化

### 1.1 DataPathExtractor

**目标**：从设备数据samples中自动提取字段路径和值

**文件创建**：
```
crates/automation/src/discovery/
├── mod.rs
├── path_extractor.rs     # DataPathExtractor
├── semantic_inference.rs # SemanticInference
└── metric_generator.rs    # VirtualMetricGenerator
```

**核心接口**：
```rust
pub struct DataPathExtractor {
    llm: Arc<dyn LlmRuntime>,
}

impl DataPathExtractor {
    /// 从samples中提取所有可访问的数据路径
    pub async fn extract_paths(
        &self,
        samples: &[serde_json::Value],
    ) -> Result<Vec<DiscoveredPath>>;

    /// 验证路径在所有samples中是否有效
    pub fn validate_path(
        &self,
        path: &str,
        samples: &[serde_json::Value],
    ) -> PathValidity;
}

pub struct DiscoveredPath {
    /// 提取的路径 (e.g., "payload.sensors[0].v")
    pub path: String,
    /// 数据类型
    pub data_type: DataType,
    /// 是否在所有samples中都存在
    pub is_consistent: bool,
    /// 示例值
    pub sample_values: Vec<serde_json::Value>,
    /// 值范围 (对于数值)
    pub value_range: Option<ValueRange>,
}
```

**影响分析**：
| 影响点 | 影响程度 | 说明 |
|--------|---------|------|
| `devices/mdl.rs` | 🟡 中 | 需要集成到MDL解析 |
| `agent/prompts.rs` | 🟡 中 | LLM prompt需要包含发现能力 |
| `automation/types.rs` | 🟢 低 | 新增类型，向后兼容 |

**风险**：
- **风险1**：LLM可能提取错误的路径
  - **缓解**：多轮验证 + 用户提供反馈
- **风险2**：复杂嵌套结构提取失败率高
  - **缓解**：递归深度限制 + 提供手动编辑选项

---

### 1.2 SemanticInference

**目标**：AI理解字段的业务语义（温度、湿度、开关等）

**核心接口**：
```rust
pub struct SemanticInference {
    llm: Arc<dyn LlmRuntime>,
}

impl SemanticInference {
    /// 推断字段的业务含义
    pub async fn infer_field_semantic(
        &self,
        field_name: &str,
        field_path: &str,
        sample_values: &[serde_json::Value],
        context: &InferenceContext,
    ) -> Result<FieldSemantic>;

    /// 推断设备类型
    pub async fn infer_device_type(
        &self,
        samples: &[serde_json::Value],
    ) -> Result<DeviceTypeInference>;
}

pub struct FieldSemantic {
    /// 推断的语义类型
    pub semantic_type: SemanticType,
    /// 标准化名称 (e.g., "temperature", "humidity")
    pub standard_name: String,
    /// 显示名称
    pub display_name: String,
    /// 推荐的单位
    pub recommended_unit: Option<String>,
    /// 置信度
    pub confidence: f32,
    /// 推理依据
    pub reasoning: String,
}

pub enum SemanticType {
    Temperature, Humidity, Pressure, Light, Motion,
    Switch, Dimmer, Color, Power, Energy,
    Co2, Pm25, Voc,
    Speed, Flow, Level,
    Status, Error, Alarm,
    Unknown,
}
```

**影响分析**：
| 影响点 | 影响程度 | 说明 |
|--------|---------|------|
| `devices/mdl.rs` | 🟡 中 | 设备类型生成时需要语义信息 |
| `agent/context/` | 🟡 中 | ResourceIndex需要语义搜索 |
| API | 🟢 低 | 新增接口，向后兼容 |

**风险**：
- **风险1**：语义推断错误（e.g., 把功率当成温度）
  - **缓解**：置信度阈值 + 用户确认
- **风险2**：字段名不规范导致推断失败
  - **缓解**：基于值范围二次验证

---

### 1.3 VirtualMetricGenerator

**目标**：自动生成虚拟指标定义（用于Simple模式设备）

**核心接口**：
```rust
pub struct VirtualMetricGenerator {
    path_extractor: DataPathExtractor,
    semantic_inference: SemanticInference,
}

impl VirtualMetricGenerator {
    /// 从samples生成完整的虚拟指标定义
    pub async fn generate_virtual_metrics(
        &self,
        device_type_id: &str,
        samples: &[serde_json::Value],
    ) -> Result<Vec<VirtualMetricDefinition>>;

    /// 生成完整的设备类型定义（Full模式）
    pub async fn generate_device_type_definition(
        &self,
        device_type_id: &str,
        device_name: &str,
        samples: &[serde_json::Value],
    ) -> Result<DeviceTypeDefinition>;
}

pub struct VirtualMetricDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub path: String,              // JSONPath表达式
    pub data_type: DataType,
    pub unit: Option<String>,
    pub value_range: Option<ValueRange>,
    pub is_readable: bool,
    pub is_writable: bool,
    pub confidence: f32,
}
```

**输出示例**：
```json
{
  "device_type": "custom_multi_sensor",
  "name": "自定义多传感器",
  "mode": "simple",
  "virtual_metrics": [
    {
      "name": "temperature",
      "display_name": "温度",
      "path": "payload.sensors[?(@.t=='temp')].v",
      "data_type": "Float",
      "unit": "°C",
      "confidence": 0.95
    },
    {
      "name": "humidity",
      "display_name": "湿度",
      "path": "payload.sensors[?(@.t=='hum')].v",
      "data_type": "Float",
      "unit": "%",
      "confidence": 0.92
    }
  ]
}
```

**影响分析**：
| 影响点 | 影响程度 | 说明 |
|--------|---------|------|
| `devices/mdl.rs` | 🔴 高 | 需要支持virtual_metrics字段 |
| `devices/service.rs` | 🟡 中 | 需要支持虚拟指标解析 |
| `devices/protocol/` | 🟡 中 | 协议映射需要支持虚拟指标 |

**风险**：
- **风险1**：虚拟指标路径复杂，解析性能问题
  - **缓解**：路径预编译 + 缓存
- **风险2**：JSONPath表达式与现有点符号不兼容
  - **缓解**：统一路径表达式格式

---

## Phase 2: 自动化生成完整化

### 2.1 Enhanced IntentAnalyzer

**目标**：从推荐类型升级为生成完整可执行的自动化

**当前限制**：
```rust
// 当前：只返回类型和理由
pub struct IntentResult {
    pub recommended_type: AutomationType,
    pub reasoning: String,
    pub suggested_automation: Option<SuggestedAutomation>,  // 实际上总是None
}
```

**改进后**：
```rust
pub struct IntentResult {
    pub recommended_type: AutomationType,
    pub confidence: u8,
    pub reasoning: String,

    // 新增：完整生成的自动化
    pub suggested_automation: Option<SuggestedAutomation>,  // 现在有值了

    // 新增：提取的实体
    pub entities: ExtractedEntities,

    // 新增：需要的额外信息
    pub missing_info: Vec<MissingInfo>,
}

pub struct ExtractedEntities {
    /// 提到的设备
    pub devices: Vec<EntityRef>,
    /// 提到的指标/数据
    pub metrics: Vec<EntityRef>,
    /// 提到的阈值
    pub thresholds: Vec<ThresholdSpec>,
    /// 提到的动作
    pub actions: Vec<ActionSpec>,
}

#[derive(Debug, Clone)]
pub struct EntityRef {
    /// 实体ID（如果已识别）
    pub id: Option<String>,
    /// 实体名称/描述
    pub name: String,
    /// 置信度
    pub confidence: f32,
    /// 需要用户确认
    pub needs_confirmation: bool,
}
```

**Prompt改进**：
```rust
// 之前：只分析类型
"Analyze the following automation description and determine whether it's better implemented as a Rule or a Workflow."

// 之后：完整提取
"Analyze the following automation description and extract all entities needed to create a complete automation:
- Devices mentioned or implied
- Metrics/data points to check
- Thresholds and conditions
- Actions to take
- Timing/delay requirements

Output a complete automation definition that can be directly executed."
```

**影响分析**：
| 影响点 | 影响程度 | 说明 |
|--------|---------|------|
| `automation/intent.rs` | 🔴 高 | 主要修改文件 |
| `automation/types.rs` | 🟡 中 | 新增EntityRef等类型 |
| `api/handlers/automations.rs` | 🟡 中 | 返回更详细的分析结果 |

**风险**：
- **风险1**：LLM提取的实体可能不准确
  - **缓解**：多轮对话澄清 + 置信度阈值
- **风险2**：生成的内容格式可能不符合要求
  - **缓解**：严格的JSON schema + 验证重试

---

### 2.2 NL2Automation Generator

**目标**：自然语言直接转换为可执行的Rule或Workflow

**核心接口**：
```rust
pub struct NL2AutomationGenerator {
    llm: Arc<dyn LlmRuntime>,
    intent_analyzer: IntentAnalyzer,
    path_extractor: DataPathExtractor,
    semantic_inference: SemanticInference,
}

impl NL2AutomationGenerator {
    /// 从自然语言生成完整自动化
    pub async fn generate(
        &self,
        description: &str,
        context: &GenerationContext,
    ) -> Result<GeneratedAutomation>;

    /// 带澄清对话的生成
    pub async fn generate_with_clarification(
        &self,
        description: &str,
        context: &GenerationContext,
    ) -> Result<ClarificationResult>;
}

pub struct GeneratedAutomation {
    /// 生成的自动化
    pub automation: Automation,
    /// 生成过程信息
    pub metadata: GenerationMetadata,
    /// 需要用户确认的模糊点
    pub confirmation_needed: Vec<ConfirmationPoint>,
}

pub struct GenerationMetadata {
    /// 使用的设备
    pub devices_resolved: Vec<DeviceResolution>,
    /// 使用的指标路径
    pub paths_resolved: Vec<PathResolution>,
    /// 推荐的阈值
    pub thresholds_suggested: Vec<ThresholdSuggestion>,
    /// 生成步骤
    pub steps: Vec<GenerationStep>,
}
```

**使用流程**：
```
用户输入: "温度超过50度时打开风扇"
    │
    ▼
1. 解析意图 → IntentAnalyzer
    │
    ▼
2. 提取实体 → devices: [温度传感器], metrics: [温度], actions: [打开风扇]
    │
    ▼
3. 澄清模糊点 → "哪个温度传感器？哪个风扇？"
    │
    ▼
4. 生成完整Rule → RuleAutomation { trigger, condition, actions }
    │
    ▼
5. 验证 → 检查语法、设备存在性、阈值合理性
    │
    ▼
6. 返回 → 用户确认 → 创建
```

**影响分析**：
| 影响点 | 影响程度 | 说明 |
|--------|---------|------|
| `automation/` | 🔴 高 | 新增核心模块 |
| `api/handlers/` | 🟡 中 | 新增NL处理endpoint |
| `agent/prompts.rs` | 🟡 中 | 更新工具描述 |
| Frontend | 🟡 中 | NL输入UI |

**风险**：
- **风险1**：生成失败率可能较高
  - **缓解**：多轮对话 + 模板回退
- **风险2**：性能问题（LLM调用多次）
  - **缓解**：并行调用 + 缓存

---

### 2.3 ThresholdRecommender

**目标**：基于历史数据AI推荐合理阈值

**核心接口**：
```rust
pub struct ThresholdRecommender {
    llm: Arc<dyn LlmRuntime>,
    telemetry: Arc<TelemetryStore>,
}

impl ThresholdRecommender {
    /// 为指标推荐阈值
    pub async fn recommend_threshold(
        &self,
        device_id: &str,
        metric_path: &str,
        goal: &ThresholdGoal,
    ) -> Result<ThresholdRecommendation>;

    /// 批量推荐多个指标的阈值
    pub async fn recommend_batch(
        &self,
        requests: Vec<ThresholdRequest>,
    ) -> Result<Vec<ThresholdRecommendation>>;
}

pub enum ThresholdGoal {
    /// 避免设备过热
    PreventOverheat,
    /// 检测异常低值
    DetectLowValue,
    /// 能源优化
    EnergyOptimization,
    /// 舒适度控制
    ComfortControl,
    /// 自定义
    Custom { description: String },
}

pub struct ThresholdRecommendation {
    /// 推荐的阈值
    pub threshold: f64,
    /// 推荐的操作符
    pub operator: ComparisonOperator,
    /// 置信度
    pub confidence: f32,
    /// 推理依据
    pub reasoning: Reasoning,
    /// 基于的数据分析
    pub data_analysis: DataAnalysis,
    /// 替代方案
    pub alternatives: Vec<AlternativeThreshold>,
}

pub struct DataAnalysis {
    /// 数据点数量
    pub sample_count: usize,
    /// 正常范围
    pub normal_range: (f64, f64),
    /// 异常值数量
    pub outlier_count: usize,
    /// 分布统计
    pub statistics: Statistics,
}

pub struct Reasoning {
    /// 主要原因
    pub primary: String,
    /// 数据支持
    pub data_points: Vec<String>,
    /// 参考依据
    pub references: Vec<String>,
}
```

**影响分析**：
| 影响点 | 影响程度 | 说明 |
|--------|---------|------|
| `automation/` | 🟢 低 | 新增独立模块 |
| `storage/telemetry.rs` | 🟡 中 | 需要历史数据查询 |
| `api/handlers/` | 🟢 低 | 新增API |

**风险**：
- **风险1**：历史数据不足时无法推荐
  - **缓解**：基于设备类型的通用推荐
- **风险2**：推荐阈值可能不符合实际业务
  - **缓解**：提供调整依据 + 人工覆盖

---

## Phase 3: 零配置设备接入

### 3.1 DeviceTypeGenerator

**目标**：从设备数据samples自动生成完整MDL定义

**核心接口**：
```rust
pub struct DeviceTypeGenerator {
    llm: Arc<dyn LlmRuntime>,
    path_extractor: DataPathExtractor,
    semantic_inference: SemanticInference,
    metric_generator: VirtualMetricGenerator,
}

impl DeviceTypeGenerator {
    /// 从samples生成设备类型定义
    pub async fn generate_from_samples(
        &self,
        device_type_id: &str,
        samples: &[DeviceSample],
    ) -> Result<GeneratedDeviceType>;

    /// 从单个数据包推断（首次接入）
    pub async fn generate_from_single_message(
        &self,
        raw_data: &[u8],
        protocol: &str,
    ) -> Result<PreliminaryDeviceType>;
}

pub struct DeviceSample {
    /// 原始数据
    pub raw_data: Vec<u8>,
    /// 解析后的JSON（如果可解析）
    pub parsed: Option<serde_json::Value>,
    /// 数据来源说明
    pub source: String,
}

pub struct GeneratedDeviceType {
    /// 设备类型ID
    pub device_type_id: String,
    /// 设备类型名称（AI生成）
    pub name: String,
    /// 设备类型描述
    pub description: String,
    /// MDL定义（Full或Simple模式）
    pub mdl_definition: DeviceTypeDefinition,
    /// 生成过程中的发现
    pub discoveries: Vec<Discovery>,
    /// 需要用户确认的内容
    pub confirmation_points: Vec<ConfirmationPoint>,
    /// 置信度评分
    pub confidence_score: f32,
}

pub enum Discovery {
    /// 发现了可提取的指标
    Metric(DiscoveredMetric),
    /// 发现了可执行的命令
    Command(DiscoveredCommand),
    /// 发现了数据编码格式
    Encoding(BinaryFormat),
    /// 发现了设备类别
    Category(DeviceCategory),
}
```

**工作流程**：
```
设备首次上报数据
    │
    ▼
1. 数据捕获 → 记录原始数据 + 时间戳
    │
    ▼
2. 格式检测 → JSON? 二进制? 编码格式?
    │
    ▼
3. 结构分析 → 嵌套层级? 数组? 字段类型?
    │
    ▼
4. 语义推断 → 字段含义? 单位? 设备类别?
    │
    ▼
5. MDL生成 → Full/Simple模式选择 + 指标定义
    │
    ▼
6. 用户确认 → 显示推断结果 → 用户调整/确认
    │
    ▼
7. 注册 → 保存到 mdl_definitions.redb
```

**影响分析**：
| 影响点 | 影响程度 | 说明 |
|--------|---------|------|
| `devices/mdl.rs` | 🔴 高 | 需要支持AI生成的MDL |
| `devices/registry.rs` | 🟡 中 | 注册流程需要支持确认 |
| `devices/adapter.rs` | 🟡 中 | 适配器需要上报原始数据 |
| `api/handlers/device_types.rs` | 🟡 中 | 新增AI生成endpoint |
| Frontend | 🟡 中 | 确认对话框UI |

**风险**：
- **风险1**：生成的MDL定义可能不准确
  - **缓解**：用户确认环节 + 支持编辑
- **风险2**：首次数据可能不够全面
  - **缓解**：增量学习 + 持续优化
- **风险3**：恶意设备可能注入错误数据
  - **缓解**：数据验证 + 权限控制

---

### 3.2 AutoDiscovery (可选)

**目标**：自动发现网络中的新设备

**核心接口**：
```rust
pub struct DeviceAutoDiscovery {
    /// MQTT自动发现
    mqtt_broker: Option<MqttClient>,
    /// Modbus扫描
    modbus_scanner: Option<ModbusScanner>,
    /// 其他协议扫描器
    scanners: HashMap<String, Box<dyn ProtocolScanner>>,
}

impl DeviceAutoDiscovery {
    /// 扫描网络中的设备
    pub async fn scan(&self, config: &ScanConfig) -> Result<Vec<DiscoveredDevice>>;

    /// 监听新设备上线
    pub async fn watch_new_devices(&self) -> Result<DeviceStream>;
}

pub struct DiscoveredDevice {
    /// 发现方式
    pub discovery_method: String,
    /// 协议类型
    pub protocol: String,
    /// 连接信息
    pub connection_info: ConnectionInfo,
    /// 初步数据样本
    pub initial_samples: Vec<DeviceSample>,
    /// 推荐的设备类型
    pub suggested_device_type: Option<PreliminaryDeviceType>,
}
```

**影响分析**：
| 影响点 | 影响程度 | 说明 |
|--------|---------|------|
| `devices/adapters/mqtt.rs` | 🟡 中 | 监听特定topic |
| 新增模块 | 🟡 中 | auto_discovery.rs |
| Frontend | 🟢 低 | 显示发现的设备 |

**风险**：
- **风险1**：网络扫描可能影响性能
  - **缓解**：限流 + 按需扫描
- **风险2**：可能发现不相关的设备
  - **缓解**：过滤规则 + 白名单

---

### 3.3 Quick Import Tools

**目标**：快速导入设备配置的工具

**功能**：
```rust
/// 从JSON/YAML导入设备类型
pub async fn import_device_type(
    file: &str,
) -> Result<DeviceTypeDefinition>;

/// 从CSV批量导入设备实例
pub async fn import_device_instances(
    file: &str,
) -> Result<Vec<DeviceInstance>>;

/// 从剪贴板快速创建
pub async fn quick_create_from_clipboard(
    content: &str,
) -> Result<QuickCreateResult>;
```

**影响分析**：
| 影响点 | 影响程度 | 说明 |
|--------|---------|------|
| `api/handlers/` | 🟢 低 | 新增endpoint |
| Frontend | 🟢 低 | 导入UI |

---

## Phase 4: 测试验证

### 测试计划

```
┌─────────────────────────────────────────────────────────────────────────┐
│  测试类型                     测试内容                      预期通过率    │
├─────────────────────────────────────────────────────────────────────────┤
│  单元测试                     每个模块的单元测试                >90%        │
│  ├── DataPathExtractor       路径提取准确性                  >85%        │
│  ├── SemanticInference       语义推断准确率                  >80%        │
│  ├── VirtualMetricGenerator  指标生成完整性                  >85%        │
│  ├── NL2Automation           自然语言转自动化准确率          >75%        │
│  └── ThresholdRecommender    阈值推荐合理性                  >70%        │
│                                                                             │
│  集成测试                     模块间协作                       >85%        │
│  ├── 设备接入→理解→生成        完整流程                       >80%        │
│  ├── 多设备场景               复杂场景处理                    >75%        │
│  └── 错误处理                 异常情况恢复                    >80%        │
│                                                                             │
│  用户验收测试                 真实场景验证                     >80%        │
│  ├── 新设备首次接入           5种不同协议设备                 >75%        │
│  ├── 自然语言创建自动化       10个不同场景                    >70%        │
│  └── AI推荐准确性             用户满意度                      >75%        │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 风险汇总

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│  风险类别              具体风险                            影响    概率    缓解措施              │
├─────────────────────────────────────────────────────────────────────────────────────┤
│  技术风险                                                                        │
│  ├── LLM准确性          提取/推断/生成可能错误              高      中    多轮验证+用户确认    │
│  ├── 性能问题           LLM调用延迟                         中      中    并行+缓存+流式     │
│  ├── 兼容性             新旧格式兼容                         中      低    版本控制+迁移工具  │
│  └── 数据质量           samples数据不完整                    高      高    增量学习+验证     │
│                                                                                 │
│  产品风险                                                                        │
│  ├── 用户期望           AI能力被高估                       高      高    明确说明+预期管理  │
│  ├── 学习成本           新功能学习曲线                      中      低    渐进式引导+示例   │
│  └── 错误容忍           生成错误的自动化                     高      中    沙箱测试+确认机制 │
│                                                                                 │
│  业务风险                                                                        │
│  ├── 实施周期           开发时间超期                       中      中    分阶段交付        │
│  ├── 资源投入           需要持续优化                        中      中    自动化闭环        │
│  └── 竞争对手           类似功能出现                        低      低    快速迭代+差异化   │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 成功指标

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│  指标类型              度量标准                              目标值                      │
├─────────────────────────────────────────────────────────────────────────────────────┤
│  技术指标                                                                        │
│  ├── 路径提取准确率      正确提取字段路径                     >85%                       │
│  ├── 语义推断准确率      正确推断字段含义                     >80%                       │
│  ├── NL转自动化成功率    生成可执行的自动化                   >75%                       │
│  ├── 阈值推荐采纳率      用户接受推荐阈值                     >60%                       │
│  └── 设备接入自动化率    无需手动定义即可使用                 >70%                       │
│                                                                                 │
│  用户体验指标                                                                  │
│  ├── 新设备接入时间      从接入到可用                       <5分钟                     │
│  ├── 自动化创建时间      从描述到执行                       <2分钟                     │
│  ├── 用户满意度          NPS评分                            >7/10                      │
│  └── 支持请求率          相关问题咨询                         <20%                       │
│                                                                                 │
│  业务指标                                                                        │
│  ├── 自动化创建数量      AI生成的自动化                     >50%                       │
│  ├── 设备类型覆盖        自动识别的设备类型                  >30种                      │
│  └── 维护成本降低        减少手动配置工作量                   >40%                       │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 实施优先级

**P0 - 必须实现 (MVP)**
1. DataPathExtractor - 数据路径提取
2. SemanticInference - 语义推断
3. Enhanced IntentAnalyzer - 完整自动化生成

**P1 - 重要增强**
4. VirtualMetricGenerator - 虚拟指标生成
5. NL2Automation - 自然语言转自动化
6. DeviceTypeGenerator - 设备类型自动生成

**P2 - 可选优化**
7. ThresholdRecommender - 阈值推荐
8. AutoDiscovery - 自动发现
9. 闭环学习 - 效果优化

---

## 文件变更清单

### 新增文件
```
crates/automation/src/discovery/
├── mod.rs                    # 模块导出
├── path_extractor.rs          # DataPathExtractor (~400行)
├── semantic_inference.rs      # SemanticInference (~350行)
├── metric_generator.rs        # VirtualMetricGenerator (~300行)
├── nl2_automation.rs          # NL2AutomationGenerator (~500行)
├── threshold_recommender.rs   # ThresholdRecommender (~300行)
├── device_type_generator.rs   # DeviceTypeGenerator (~400行)
└── types.rs                   # 共享类型定义 (~200行)

总计: ~2450行新代码
```

### 修改文件
```
crates/automation/src/
├── lib.rs                     + 模块导出
├── intent.rs                  + 增强实体提取 (~300行变更)
└── types.rs                   + 新类型定义 (~100行变更)

crates/devices/src/
├── mdl.rs                     + virtual_metrics支持 (~150行变更)
└── service.rs                 + 虚拟指标解析 (~100行变更)

crates/api/src/handlers/
├── automations.rs             + AI生成endpoint (~200行变更)
└── device_types.rs            + AI生成endpoint (~150行变更)

web/src/
├── pages/automation.tsx        + NL输入UI (~300行变更)
└── pages/devices.tsx          + 快速接入UI (~200行变更)

总计: ~1800行变更
```

---

## 下一步行动

1. **立即可做** (本周):
   - [ ] 创建 `automation/src/discovery/` 目录结构
   - [ ] 实现 `DataPathExtractor` 基础版本
   - [ ] 编写单元测试

2. **近期规划** (2-3周):
   - [ ] 完成 Phase 1 三个模块
   - [ ] 集成测试
   - [ ] 文档更新

3. **中期目标** (1-2月):
   - [ ] 完成 Phase 2
   - [ ] 完成 Phase 3
   - [ ] 用户验收测试
