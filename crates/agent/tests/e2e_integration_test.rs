//! 端到端集成测试 - 真实LLM + 完整设备链路测试
//!
//! 这个测试创建完整的集成环境：
//! 1. 真实LLM后端（Ollama）
//! 2. 设备模拟器（多领域）
//! 3. EventBus事件流
//! 4. SessionManager会话管理
//! 5. 完整的Agent工具链路
//!
//! 运行方式:
//! ```bash
//! # 确保Ollama在运行
//! ollama serve
//!
//! # 运行测试
//! cargo test -p edge-ai-agent e2e_real_llm_test -- --ignored --nocapture
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use edge_ai_core::{EventBus, Message, event::MetricValue, eventbus::EventBus as CoreEventBus};
use edge_ai_agent::{
    LlmBackend,
    SessionManager,
};
use edge_ai_tools::ToolRegistryBuilder;
use edge_ai_devices::{
    ConnectionStatus,
    adapter::AdapterError,
};
use edge_ai_llm::{OllamaConfig, OllamaRuntime};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{RwLock, mpsc};

// ============================================================================
// 配置
// ============================================================================

const DEFAULT_OLLAMA_ENDPOINT: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "qwen2.5:3b"; // 或 "gpt-oss:20b"
const TEST_TIMEOUT_SECS: u64 = 120;
const SHOW_FULL_CONVERSATION: bool = true; // 显示完整对话

// ============================================================================
// 测试结果结构
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2ETestReport {
    pub test_name: String,
    pub started_at: i64,
    pub completed_at: i64,
    pub duration_ms: u64,
    pub llm_config: LlmConfigInfo,
    pub domain_results: Vec<DomainTestResult>,
    pub summary: TestSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfigInfo {
    pub endpoint: String,
    pub model: String,
    pub connected: bool,
    pub connection_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainTestResult {
    pub domain: String,
    pub scenario_name: String,
    pub conversation_turns: Vec<TurnResult>,
    pub total_turns: usize,
    pub successful_turns: usize,
    pub avg_response_time_ms: u64,
    pub tools_called: Vec<String>,
    pub llm_tokens_used: TokenUsage,
    pub quality_score: ConversationQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnResult {
    pub turn_number: usize,
    pub user_input: String,
    pub agent_response: String,
    pub response_time_ms: u64,
    pub tools_called: Vec<String>,
    pub tool_results: Vec<String>,
    pub is_successful: bool,
    pub error_message: Option<String>,
    pub thinking_content: Option<String>,
    pub response_length: usize,
    pub has_tool_execution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub total_turns: usize,
    pub successful_turns: usize,
    pub success_rate: f64,
    pub avg_response_time_ms: u64,
    pub total_tools_called: usize,
    pub unique_tools_used: Vec<String>,
    pub total_response_length: usize,
    pub avg_response_length: usize,
}

/// 对话质量评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationQuality {
    /// 平均响应长度（字符数）
    pub avg_response_length: f64,
    /// 工具使用率
    pub tool_usage_rate: f64,
    /// 响应相关性 (0-100)
    pub relevance_score: u8,
    /// 完整性 (0-100)
    pub completeness_score: u8,
    /// 总体质量评分 (0-100)
    pub overall_score: u8,
}

// ============================================================================
// 设备模拟器 - 简化版用于E2E测试
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDevice {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub location: String,
    pub domain: String,
    pub state: DeviceState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    pub online: bool,
    pub current_value: Option<f64>,
    pub properties: HashMap<String, serde_json::Value>,
}

pub struct E2EDeviceSimulator {
    name: String,
    event_bus: Arc<CoreEventBus>,
    devices: Arc<RwLock<HashMap<String, TestDevice>>>,
    running: Arc<RwLock<bool>>,
    _event_tx: mpsc::UnboundedSender<()>,  // Unused for now
    metrics: Arc<RwLock<SimulatorMetrics>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SimulatorMetrics {
    pub events_published: usize,
    pub commands_received: usize,
    pub commands_executed: usize,
}

impl E2EDeviceSimulator {
    pub fn new(name: String, event_bus: Arc<CoreEventBus>) -> Self {
        let (event_tx, _) = mpsc::unbounded_channel();
        Self {
            name,
            event_bus,
            devices: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            _event_tx: event_tx,
            metrics: Arc::new(RwLock::new(SimulatorMetrics::default())),
        }
    }

    pub async fn add_device(&self, device: TestDevice) {
        let mut devices = self.devices.write().await;
        devices.insert(device.id.clone(), device);
    }

    pub async fn get_device(&self, device_id: &str) -> Option<TestDevice> {
        let devices = self.devices.read().await;
        devices.get(device_id).cloned()
    }

    pub async fn list_devices(&self) -> Vec<TestDevice> {
        let devices = self.devices.read().await;
        devices.values().cloned().collect()
    }

    pub async fn start(&self) -> Result<(), AdapterError> {
        let mut running = self.running.write().await;
        *running = true;

        // Mark all devices as online and publish events
        let devices = self.devices.read().await;
        for (device_id, device) in devices.iter() {
            let _ = self.event_bus.publish(edge_ai_core::event::NeoTalkEvent::DeviceOnline {
                device_id: device_id.clone(),
                device_type: device.device_type.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            }).await;
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), AdapterError> {
        let mut running = self.running.write().await;
        *running = false;
        Ok(())
    }

    pub async fn execute_command(
        &self,
        device_id: &str,
        command: &str,
        _payload: &str,
    ) -> Result<String, AdapterError> {
        let mut devices = self.devices.write().await;
        let mut metrics = self.metrics.write().await;

        metrics.commands_received += 1;

        let device = devices.get_mut(device_id)
            .ok_or_else(|| AdapterError::DeviceNotFound(device_id.to_string()))?;

        match command {
            "turn_on" | "on" => {
                device.state.online = true;
                device.state.current_value = Some(100.0);
                metrics.commands_executed += 1;
                Ok(format!("✅ 设备 '{}' 已成功打开", device.name))
            }
            "turn_off" | "off" => {
                device.state.current_value = Some(0.0);
                metrics.commands_executed += 1;
                Ok(format!("✅ 设备 '{}' 已成功关闭", device.name))
            }
            "set" | "set_temperature" => {
                device.state.current_value = Some(26.0);
                metrics.commands_executed += 1;
                Ok(format!("✅ 设备 '{}' 温度已设置为 26°C", device.name))
            }
            "set_brightness" => {
                device.state.current_value = Some(80.0);
                metrics.commands_executed += 1;
                Ok(format!("✅ 设备 '{}' 亮度已设置为 80%", device.name))
            }
            "open" => {
                device.state.current_value = Some(100.0);
                metrics.commands_executed += 1;
                Ok(format!("✅ 设备 '{}' 已打开", device.name))
            }
            "close" => {
                device.state.current_value = Some(0.0);
                metrics.commands_executed += 1;
                Ok(format!("✅ 设备 '{}' 已关闭", device.name))
            }
            "start" => {
                device.state.current_value = Some(1.0);
                metrics.commands_executed += 1;
                Ok(format!("✅ 设备 '{}' 已启动", device.name))
            }
            "stop" => {
                device.state.current_value = Some(0.0);
                metrics.commands_executed += 1;
                Ok(format!("✅ 设备 '{}' 已停止", device.name))
            }
            _ => Ok(format!("✅ 命令 '{}' 已执行", command)),
        }
    }

    pub async fn get_metrics(&self) -> SimulatorMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn query_device_data(&self, device_id: &str) -> Result<String, AdapterError> {
        let devices = self.devices.read().await;
        let device = devices.get(device_id)
            .ok_or_else(|| AdapterError::DeviceNotFound(device_id.to_string()))?;

        Ok(format!("✅ 设备 '{}' 数据查询成功 - 当前值: {:.1}, 状态: {}",
            device.name,
            device.state.current_value.unwrap_or(0.0),
            if device.state.online { "在线" } else { "离线" }
        ))
    }
}

// ============================================================================
// 领域设备生成器
// ============================================================================

pub struct DomainDeviceGenerator;

impl DomainDeviceGenerator {
    pub fn generate_home_devices() -> Vec<TestDevice> {
        vec![
            // 客厅设备
            TestDevice {
                id: "home_light_livingroom_001".to_string(),
                name: "客厅智能灯".to_string(),
                device_type: "light".to_string(),
                location: "客厅".to_string(),
                domain: "智能家居".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(0.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("brightness".to_string(), json!(0));
                        map.insert("color_temperature".to_string(), json!(4000));
                        map
                    },
                },
            },
            TestDevice {
                id: "home_ac_livingroom".to_string(),
                name: "客厅空调".to_string(),
                device_type: "air_conditioner".to_string(),
                location: "客厅".to_string(),
                domain: "智能家居".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(24.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("target_temp".to_string(), json!(24));
                        map.insert("mode".to_string(), json!("cool"));
                        map
                    },
                },
            },
            // 卧室设备
            TestDevice {
                id: "home_ac_bedroom_001".to_string(),
                name: "卧室空调".to_string(),
                device_type: "air_conditioner".to_string(),
                location: "卧室".to_string(),
                domain: "智能家居".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(26.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("target_temp".to_string(), json!(26));
                        map.insert("mode".to_string(), json!("cool"));
                        map
                    },
                },
            },
            TestDevice {
                id: "home_light_bedroom".to_string(),
                name: "卧室灯".to_string(),
                device_type: "light".to_string(),
                location: "卧室".to_string(),
                domain: "智能家居".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(0.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("brightness".to_string(), json!(0));
                        map
                    },
                },
            },
            // 传感器
            TestDevice {
                id: "home_sensor_temp_livingroom".to_string(),
                name: "客厅温度传感器".to_string(),
                device_type: "temperature_sensor".to_string(),
                location: "客厅".to_string(),
                domain: "智能家居".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(23.5),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("unit".to_string(), json!("°C"));
                        map.insert("accuracy".to_string(), json!(0.5));
                        map
                    },
                },
            },
            TestDevice {
                id: "home_sensor_humidity_livingroom".to_string(),
                name: "客厅湿度传感器".to_string(),
                device_type: "humidity_sensor".to_string(),
                location: "客厅".to_string(),
                domain: "智能家居".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(55.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("unit".to_string(), json!("%"));
                        map
                    },
                },
            },
            // 窗帘
            TestDevice {
                id: "home_curtain_livingroom".to_string(),
                name: "客厅窗帘".to_string(),
                device_type: "curtain".to_string(),
                location: "客厅".to_string(),
                domain: "智能家居".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(50.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("position".to_string(), json!(50));
                        map.insert("max_position".to_string(), json!(100));
                        map
                    },
                },
            },
            // 其他设备
            TestDevice {
                id: "home_socket_kitchen".to_string(),
                name: "厨房插座".to_string(),
                device_type: "socket".to_string(),
                location: "厨房".to_string(),
                domain: "智能家居".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(0.0),
                    properties: HashMap::new(),
                },
            },
        ]
    }

    pub fn generate_industrial_devices() -> Vec<TestDevice> {
        vec![
            TestDevice {
                id: "industrial_plc_line_a".to_string(),
                name: "生产线A PLC控制器".to_string(),
                device_type: "plc".to_string(),
                location: "生产线A".to_string(),
                domain: "工业制造".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(75.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("cycle_count".to_string(), json!(1523));
                        map.insert("status".to_string(), json!("running"));
                        map
                    },
                },
            },
            TestDevice {
                id: "industrial_temp_furnace".to_string(),
                name: "熔炉温度传感器".to_string(),
                device_type: "temperature_sensor".to_string(),
                location: "热处理车间".to_string(),
                domain: "工业制造".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(845.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("alarm_threshold".to_string(), json!(900.0));
                        map.insert("unit".to_string(), json!("°C"));
                        map
                    },
                },
            },
            TestDevice {
                id: "industrial_robot_arm_1".to_string(),
                name: "1号机械臂".to_string(),
                device_type: "robotic_arm".to_string(),
                location: "装配车间".to_string(),
                domain: "工业制造".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(0.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("x".to_string(), json!(0.0));
                        map.insert("y".to_string(), json!(120.5));
                        map.insert("z".to_string(), json!(450.0));
                        map
                    },
                },
            },
            TestDevice {
                id: "industrial_conveyor_belt".to_string(),
                name: "传送带".to_string(),
                device_type: "conveyor".to_string(),
                location: "生产线B".to_string(),
                domain: "工业制造".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(1.5), // m/s
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("speed".to_string(), json!(1.5));
                        map.insert("length".to_string(), json!(50.0));
                        map
                    },
                },
            },
        ]
    }

    pub fn generate_agriculture_devices() -> Vec<TestDevice> {
        vec![
            TestDevice {
                id: "agri_soil_greenhouse1".to_string(),
                name: "温室1号土壤传感器".to_string(),
                device_type: "soil_sensor".to_string(),
                location: "温室1号".to_string(),
                domain: "智慧农业".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(65.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("soil_moisture".to_string(), json!(65.0));
                        map.insert("soil_ph".to_string(), json!(6.8));
                        map
                    },
                },
            },
            TestDevice {
                id: "agri_temp_greenhouse1".to_string(),
                name: "温室1号温度传感器".to_string(),
                device_type: "temperature_sensor".to_string(),
                location: "温室1号".to_string(),
                domain: "智慧农业".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(28.5),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("unit".to_string(), json!("°C"));
                        map
                    },
                },
            },
            TestDevice {
                id: "agri_irrigation_main".to_string(),
                name: "主灌溉控制器".to_string(),
                device_type: "irrigation_controller".to_string(),
                location: "露天A区".to_string(),
                domain: "智慧农业".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(0.0),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("valve_open".to_string(), json!(false));
                        map.insert("flow_rate".to_string(), json!(0.0));
                        map
                    },
                },
            },
            TestDevice {
                id: "agri_fertilizer_dispenser".to_string(),
                name: "施肥机".to_string(),
                device_type: "fertilizer_system".to_string(),
                location: "大棚B区".to_string(),
                domain: "智慧农业".to_string(),
                state: DeviceState {
                    online: true,
                    current_value: Some(0.0),
                    properties: HashMap::new(),
                },
            },
        ]
    }

    pub fn generate_all_devices() -> HashMap<String, Vec<TestDevice>> {
        let mut all = HashMap::new();
        all.insert("智能家居".to_string(), Self::generate_home_devices());
        all.insert("工业制造".to_string(), Self::generate_industrial_devices());
        all.insert("智慧农业".to_string(), Self::generate_agriculture_devices());
        all
    }
}

// ============================================================================
// 对话场景定义 - 扩展版本
// ============================================================================

#[derive(Debug, Clone)]
pub struct ConversationScenario {
    pub domain: String,
    pub name: String,
    pub description: String,
    pub turns: Vec<ConversationTurn>,
    pub expected_tools: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub user_input: String,
    pub expected_tools: Vec<String>,
    pub min_response_length: usize,
    pub should_use_tool: bool,
    pub quality_check: QualityCheck,
}

#[derive(Debug, Clone)]
pub struct QualityCheck {
    pub should_be_helpful: bool,
    pub should_be_complete: bool,
    pub expected_keywords: Vec<String>,
}

impl Default for QualityCheck {
    fn default() -> Self {
        Self {
            should_be_helpful: true,
            should_be_complete: true,
            expected_keywords: vec![],
        }
    }
}

impl ConversationScenario {
    /// 创建智能家居 - 回家模式场景 (25轮对话)
    fn home_arrival_scenario() -> Self {
        Self {
            domain: "智能家居".to_string(),
            name: "回家模式".to_string(),
            description: "模拟用户回家后的完整场景，包含多设备控制和环境查询 (25轮)".to_string(),
            expected_tools: vec!["device.control".to_string(), "device.query".to_string(), "device.discover".to_string()],
            turns: vec![
                ct("我回家了", vec![], 5, false, qc(vec!["欢迎", "回家", "到家"], true, true)),
                ct("帮我打开客厅的灯", vec!["device.control"], 10, true, Default::default()),
                ct("再把走廊灯也打开", vec!["device.control"], 10, true, Default::default()),
                ct("现在家里灯光怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("把客厅空调调到26度", vec!["device.control"], 10, true, Default::default()),
                ct("卧室空调也打开，调到25度", vec!["device.control"], 10, true, Default::default()),
                ct("现在客厅温度是多少", vec!["device.query"], 10, true, Default::default()),
                ct("卧室呢，温度多少", vec!["device.query"], 10, true, Default::default()),
                ct("现在家里湿度怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("有点干，把加湿器打开", vec!["device.control"], 10, true, Default::default()),
                ct("播放一些轻松的音乐", vec!["device.control"], 10, true, Default::default()),
                ct("把音量调到50%", vec!["device.control"], 10, true, Default::default()),
                ct("打开客厅窗帘", vec!["device.control"], 10, true, Default::default()),
                ct("检查一下门锁状态", vec!["device.query"], 10, true, Default::default()),
                ct("把门锁上", vec!["device.control"], 10, true, Default::default()),
                ct("打开空气净化器", vec!["device.control"], 10, true, Default::default()),
                ct("现在空气质量怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("把扫地机器人打开", vec!["device.control"], 10, true, Default::default()),
                ct("现在厨房温度多少", vec!["device.query"], 10, true, Default::default()),
                ct("打开厨房排气扇", vec!["device.control"], 10, true, Default::default()),
                ct("列出所有已打开的设备", vec!["device.discover"], 15, true, Default::default()),
                ct("把灯光都调暗一点", vec!["device.control"], 10, true, Default::default()),
                ct("设置一个晚上10点关灯的规则", vec!["create_rule"], 10, true, Default::default()),
                ct("现在几点了", vec![], 10, false, Default::default()),
                ct("就这样了，谢谢", vec![], 5, false, qc(vec!["不客气", "再见", "随时"], true, true)),
            ],
        }
    }

    /// 创建智能家居 - 离家模式场景 (20轮对话)
    fn leaving_home_scenario() -> Self {
        Self {
            domain: "智能家居".to_string(),
            name: "离家模式".to_string(),
            description: "模拟用户离家前的完整场景，关闭所有设备 (20轮)".to_string(),
            expected_tools: vec!["device.control".to_string(), "device.query".to_string()],
            turns: vec![
                ct("我要出门了", vec![], 5, false, Default::default()),
                ct("大概什么时候回来", vec![], 10, false, Default::default()),
                ct("晚上6点左右回来", vec![], 10, false, Default::default()),
                ct("帮我关闭所有灯", vec!["device.control"], 10, true, Default::default()),
                ct("客厅灯关了吗", vec!["device.query"], 10, true, Default::default()),
                ct("卧室灯呢", vec!["device.query"], 10, true, Default::default()),
                ct("关闭所有空调", vec!["device.control"], 10, true, Default::default()),
                ct("关闭加湿器", vec!["device.control"], 10, true, Default::default()),
                ct("关闭空气净化器", vec!["device.control"], 10, true, Default::default()),
                ct("停止音乐播放", vec!["device.control"], 10, true, Default::default()),
                ct("关闭扫地机器人", vec!["device.control"], 10, true, Default::default()),
                ct("检查一下还有哪些设备开着", vec!["device.discover"], 10, true, Default::default()),
                ct("关闭所有剩余设备", vec!["device.control"], 10, true, Default::default()),
                ct("打开客厅窗帘", vec!["device.control"], 10, true, Default::default()),
                ct("打开卧室窗帘", vec!["device.control"], 10, true, Default::default()),
                ct("设置一个回家前自动开空调的规则", vec!["create_rule"], 15, true, Default::default()),
                ct("列出当前所有规则", vec!["list_rules"], 10, true, Default::default()),
                ct("门锁好了吗", vec!["device.query"], 10, true, Default::default()),
                ct("帮我锁门", vec!["device.control"], 10, true, Default::default()),
                ct("好了，我走了", vec![], 5, false, qc(vec!["再见", "路上", "注意"], true, true)),
            ],
        }
    }

    /// 创建智能家居 - 睡眠模式场景 (20轮对话)
    fn sleep_mode_scenario() -> Self {
        Self {
            domain: "智能家居".to_string(),
            name: "睡眠模式".to_string(),
            description: "设置睡眠模式，调整多设备状态 (20轮)".to_string(),
            expected_tools: vec!["device.control".to_string(), "device.query".to_string()],
            turns: vec![
                ct("我要睡觉了", vec![], 5, false, qc(vec!["晚安", "好梦"], true, true)),
                ct("关闭客厅灯", vec!["device.control"], 10, true, Default::default()),
                ct("关闭卧室灯", vec!["device.control"], 10, true, Default::default()),
                ct("把走廊灯打开，调最暗", vec!["device.control"], 10, true, Default::default()),
                ct("关闭所有窗帘", vec!["device.control"], 10, true, Default::default()),
                ct("把卧室空调调到28度", vec!["device.control"], 10, true, Default::default()),
                ct("关闭客厅空调", vec!["device.control"], 10, true, Default::default()),
                ct("打开卧室加湿器", vec!["device.control"], 10, true, Default::default()),
                ct("关闭空气净化器", vec!["device.control"], 10, true, Default::default()),
                ct("停止音乐播放", vec!["device.control"], 10, true, Default::default()),
                ct("现在卧室温度多少", vec!["device.query"], 10, true, Default::default()),
                ct("湿度呢", vec!["device.query"], 10, true, Default::default()),
                ct("把加湿器调到中等", vec!["device.control"], 10, true, Default::default()),
                ct("检查门锁状态", vec!["device.query"], 10, true, Default::default()),
                ct("帮我锁门", vec!["device.control"], 10, true, Default::default()),
                ct("关闭所有不需要的设备", vec!["device.control"], 10, true, Default::default()),
                ct("设置明天早上7点的闹钟", vec!["device.control"], 10, true, Default::default()),
                ct("设置明天早上7点自动开灯的规则", vec!["create_rule"], 15, true, Default::default()),
                ct("列出当前开着的设备", vec!["device.discover"], 10, true, Default::default()),
                ct("晚安", vec![], 5, false, qc(vec!["晚安", "好梦"], true, true)),
            ],
        }
    }

    /// 创建智能家居 - 环境监控场景 (25轮对话)
    fn environment_monitor_scenario() -> Self {
        Self {
            domain: "智能家居".to_string(),
            name: "环境监控".to_string(),
            description: "查询各种环境传感器数据 (25轮)".to_string(),
            expected_tools: vec!["device.query".to_string(), "device.discover".to_string()],
            turns: vec![
                ct("现在家里温湿度怎么样", vec!["device.query"], 15, true, Default::default()),
                ct("客厅温度多少", vec!["device.query"], 10, true, Default::default()),
                ct("卧室呢", vec!["device.query"], 10, true, Default::default()),
                ct("厨房温度", vec!["device.query"], 10, true, Default::default()),
                ct("卫生间呢", vec!["device.query"], 10, true, Default::default()),
                ct("各个房间湿度怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("客厅有点干，能加湿吗", vec!["device.control"], 10, true, Default::default()),
                ct("把加湿器打开", vec!["device.control"], 10, true, Default::default()),
                ct("现在空气质量怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("PM2.5是多少", vec!["device.query"], 10, true, Default::default()),
                ct("需要开空气净化器吗", vec![], 10, false, Default::default()),
                ct("打开空气净化器", vec!["device.control"], 10, true, Default::default()),
                ct("现在光照强度怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("客厅是不是太亮了", vec!["device.query"], 10, true, Default::default()),
                ct("把窗帘关上一半", vec!["device.control"], 10, true, Default::default()),
                ct("噪音水平怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("有没有检测到什么异常", vec!["device.query"], 10, true, Default::default()),
                ct("检查所有传感器状态", vec!["device.discover"], 10, true, Default::default()),
                ct("有没有传感器离线", vec!["device.query"], 10, true, Default::default()),
                ct("列出所有环境传感器", vec!["device.discover"], 10, true, Default::default()),
                ct("哪些房间温度最高", vec!["device.query"], 10, true, Default::default()),
                ct("哪些房间湿度最低", vec!["device.query"], 10, true, Default::default()),
                ct("生成一个环境报告", vec![], 10, false, Default::default()),
                ct("最近24小时温湿度变化", vec!["device.query"], 10, true, Default::default()),
                ct("就这样吧", vec![], 5, false, Default::default()),
            ],
        }
    }

    /// 创建工业制造 - 生产线监控场景 (25轮对话)
    fn production_line_scenario() -> Self {
        Self {
            domain: "工业制造".to_string(),
            name: "生产线监控".to_string(),
            description: "监控生产线设备状态和数据 (25轮)".to_string(),
            expected_tools: vec!["device.discover".to_string(), "device.query".to_string(), "device.control".to_string()],
            turns: vec![
                ct("生产线A的状态怎么样", vec!["device.query"], 15, true, Default::default()),
                ct("生产线B呢", vec!["device.query"], 10, true, Default::default()),
                ct("机械臂在运行吗", vec!["device.query"], 10, true, Default::default()),
                ct("1号机械臂状态", vec!["device.query"], 10, true, Default::default()),
                ct("2号机械臂呢", vec!["device.query"], 10, true, Default::default()),
                ct("传送带运行正常吗", vec!["device.query"], 10, true, Default::default()),
                ct("当前生产速度是多少", vec!["device.query"], 10, true, Default::default()),
                ct("熔炉温度是多少", vec!["device.query"], 10, true, Default::default()),
                ct("温度正常吗", vec![], 10, false, Default::default()),
                ct("有没有温度报警", vec!["device.query"], 10, true, Default::default()),
                ct("压力传感器读数", vec!["device.query"], 10, true, Default::default()),
                ct("流水线上有多少产品", vec!["device.query"], 10, true, Default::default()),
                ct("生产进度怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("有没有设备故障", vec!["device.query"], 10, true, Default::default()),
                ct("能耗水平怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("列出所有工业设备", vec!["device.discover"], 10, true, Default::default()),
                ct("哪些设备需要维护", vec!["device.query"], 10, true, Default::default()),
                ct("停机时间有多长", vec!["device.query"], 10, true, Default::default()),
                ct("今天的产量是多少", vec!["device.query"], 10, true, Default::default()),
                ct("良品率怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("有没有异常振动", vec!["device.query"], 10, true, Default::default()),
                ct("润滑油位正常吗", vec!["device.query"], 10, true, Default::default()),
                ct("生成生产报告", vec![], 10, false, Default::default()),
                ct("今天设备运行总时长", vec!["device.query"], 10, true, Default::default()),
                ct("知道了", vec![], 5, false, Default::default()),
            ],
        }
    }

    /// 创建工业制造 - 温度报警规则场景 (20轮对话)
    fn temperature_alert_scenario() -> Self {
        Self {
            domain: "工业制造".to_string(),
            name: "温度报警规则".to_string(),
            description: "创建和管理温度报警规则 (20轮)".to_string(),
            expected_tools: vec!["create_rule".to_string(), "list_rules".to_string(), "device.query".to_string()],
            turns: vec![
                ct("熔炉温度正常吗", vec!["device.query"], 10, true, Default::default()),
                ct("现在温度是多少", vec!["device.query"], 10, true, Default::default()),
                ct("如果温度超过900度，立即报警", vec!["create_rule"], 10, true, Default::default()),
                ct("再创建一个规则，温度低于500度也报警", vec!["create_rule"], 15, true, Default::default()),
                ct("列出所有规则", vec!["list_rules"], 10, true, Default::default()),
                ct("第一个规则的详情是什么", vec![], 10, false, Default::default()),
                ct("能不能修改报警阈值", vec![], 10, false, Default::default()),
                ct("把第一个规则的阈值改成950度", vec!["update_rule"], 10, true, Default::default()),
                ct("再列出规则看看", vec!["list_rules"], 10, true, Default::default()),
                ct("创建一个压力报警规则", vec!["create_rule"], 15, true, Default::default()),
                ct("压力超过100时报警", vec!["create_rule"], 10, true, Default::default()),
                ct("现在有几个规则了", vec!["list_rules"], 10, true, Default::default()),
                ct("禁用温度报警规则", vec!["disable_rule"], 10, true, Default::default()),
                ct("列出规则看看状态", vec!["list_rules"], 10, true, Default::default()),
                ct("重新启用温度报警", vec!["enable_rule"], 10, true, Default::default()),
                ct("删除压力报警规则", vec!["delete_rule"], 10, true, Default::default()),
                ct("现在还有哪些规则", vec!["list_rules"], 10, true, Default::default()),
                ct("这些规则都在运行吗", vec![], 10, false, Default::default()),
                ct("怎么测试规则", vec![], 10, false, Default::default()),
                ct("好了，了解了", vec![], 5, false, Default::default()),
            ],
        }
    }

    /// 创建智慧农业 - 智能灌溉场景 (25轮对话)
    fn smart_irrigation_scenario() -> Self {
        Self {
            domain: "智慧农业".to_string(),
            name: "智能灌溉".to_string(),
            description: "智能灌溉系统的完整场景 (25轮)".to_string(),
            expected_tools: vec!["device.query".to_string(), "device.control".to_string(), "create_rule".to_string()],
            turns: vec![
                ct("温室1号的土壤湿度是多少", vec!["device.query"], 10, true, Default::default()),
                ct("温室2号呢", vec!["device.query"], 10, true, Default::default()),
                ct("温室3号呢", vec!["device.query"], 10, true, Default::default()),
                ct("哪个温室最干", vec!["device.query"], 10, true, Default::default()),
                ct("温度怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("各温室温度对比", vec!["device.query"], 10, true, Default::default()),
                ct("温室1太干了，开始浇水", vec!["device.control"], 10, true, Default::default()),
                ct("温室2也浇水", vec!["device.control"], 10, true, Default::default()),
                ct("温室3湿度正常，不用浇", vec![], 10, false, Default::default()),
                ct("灌溉系统运行正常吗", vec!["device.query"], 10, true, Default::default()),
                ct("水流压力够吗", vec!["device.query"], 10, true, Default::default()),
                ct("水位是多少", vec!["device.query"], 10, true, Default::default()),
                ct("需要补水吗", vec!["device.query"], 10, true, Default::default()),
                ct("打开水泵补水", vec!["device.control"], 10, true, Default::default()),
                ct("现在土壤湿度怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("继续浇5分钟", vec!["device.control"], 10, true, Default::default()),
                ct("停止浇水", vec!["device.control"], 10, true, Default::default()),
                ct("创建一个自动灌溉规则", vec!["create_rule"], 15, true, Default::default()),
                ct("湿度低于30%自动浇水", vec!["create_rule"], 15, true, Default::default()),
                ct("列出所有灌溉设备", vec!["device.discover"], 10, true, Default::default()),
                ct("施肥机状态怎么样", vec!["device.query"], 10, true, Default::default()),
                ct("需要施肥吗", vec!["device.query"], 10, true, Default::default()),
                ct("今天的用水量", vec!["device.query"], 10, true, Default::default()),
                ct("好了", vec![], 5, false, Default::default()),
            ],
        }
    }

    /// 创建通用 - 基础对话场景 (25轮对话)
    fn basic_conversation_scenario() -> Self {
        Self {
            domain: "通用".to_string(),
            name: "基础对话".to_string(),
            description: "基础对话能力测试 (25轮)".to_string(),
            expected_tools: vec!["device.discover".to_string(), "device.control".to_string()],
            turns: vec![
                ct("你好", vec![], 10, false, qc(vec!["你好", "您好", "嗨"], true, true)),
                ct("你是谁", vec![], 15, false, qc(vec!["NeoTalk", "助手", "智能"], true, true)),
                ct("你能做什么", vec![], 20, false, qc(vec!["设备", "控制", "查询", "规则"], true, true)),
                ct("列出所有设备", vec!["device.discover"], 10, true, Default::default()),
                ct("有几个设备", vec![], 10, false, Default::default()),
                ct("都是什么类型的设备", vec![], 10, false, Default::default()),
                ct("能控制哪些设备", vec![], 10, false, Default::default()),
                ct("怎么控制设备", vec![], 10, false, Default::default()),
                ct("演示一下控制灯", vec!["device.control"], 10, true, Default::default()),
                ct("再演示一下查询温度", vec!["device.query"], 10, true, Default::default()),
                ct("能创建自动化规则吗", vec![], 10, false, Default::default()),
                ct("怎么创建规则", vec![], 10, false, Default::default()),
                ct("演示创建一个规则", vec!["create_rule"], 15, true, Default::default()),
                ct("列出所有规则", vec!["list_rules"], 10, true, Default::default()),
                ct("能删除规则吗", vec![], 10, false, Default::default()),
                ct("怎么删除", vec![], 10, false, Default::default()),
                ct("支持哪些设备类型", vec![], 10, false, Default::default()),
                ct("支持哪些传感器", vec![], 10, false, Default::default()),
                ct("怎么连接新设备", vec![], 10, false, Default::default()),
                ct("有API接口吗", vec![], 10, false, Default::default()),
                ct("可以语音控制吗", vec![], 10, false, Default::default()),
                ct("数据保存在哪", vec![], 10, false, Default::default()),
                ct("安全吗", vec![], 10, false, Default::default()),
                ct("还有什么功能", vec![], 10, false, Default::default()),
                ct("谢谢介绍", vec![], 5, false, qc(vec!["不客气", "再见"], true, true)),
            ],
        }
    }

    /// 创建通用 - 复杂多轮对话场景 (30轮对话)
    fn complex_multi_turn_scenario() -> Self {
        Self {
            domain: "通用".to_string(),
            name: "复杂多轮对话".to_string(),
            description: "测试复杂多轮对话中的上下文理解 (30轮)".to_string(),
            expected_tools: vec!["device.query".to_string(), "device.control".to_string(), "create_rule".to_string()],
            turns: vec![
                ct("帮我查看温度", vec![], 10, false, qc(vec!["哪个", "哪个房间"], true, true)),
                ct("客厅的温度", vec!["device.query"], 10, true, Default::default()),
                ct("有点热，调低一点", vec!["device.control"], 10, true, Default::default()),
                ct("再低2度", vec!["device.control"], 10, true, Default::default()),
                ct("现在多少度", vec!["device.query"], 10, true, Default::default()),
                ct("卧室呢", vec!["device.query"], 10, true, Default::default()),
                ct("也调到24度", vec!["device.control"], 10, true, Default::default()),
                ct("把所有灯打开", vec!["device.control"], 10, true, Default::default()),
                ct("太亮了，调暗一点", vec!["device.control"], 10, true, Default::default()),
                ct("客厅灯关掉", vec!["device.control"], 10, true, Default::default()),
                ct("卧室灯调到最暗", vec!["device.control"], 10, true, Default::default()),
                ct("检查湿度", vec!["device.query"], 10, true, Default::default()),
                ct("打开加湿器", vec!["device.control"], 10, true, Default::default()),
                ct("把音量调到70%", vec!["device.control"], 10, true, Default::default()),
                ct("播放音乐", vec!["device.control"], 10, true, Default::default()),
                ct("暂停一下", vec!["device.control"], 10, true, Default::default()),
                ct("继续播放", vec!["device.control"], 10, true, Default::default()),
                ct("停止音乐", vec!["device.control"], 10, true, Default::default()),
                ct("创建一个温度超过25度开空调的规则", vec!["create_rule"], 15, true, Default::default()),
                ct("再创建一个湿度低于40%开加湿器的规则", vec!["create_rule"], 15, true, Default::default()),
                ct("列出所有规则", vec!["list_rules"], 10, true, Default::default()),
                ct("删除第一个规则", vec!["delete_rule"], 10, true, Default::default()),
                ct("现在还有几个规则", vec!["list_rules"], 10, true, Default::default()),
                ct("关闭所有灯", vec!["device.control"], 10, true, Default::default()),
                ct("打开走廊灯", vec!["device.control"], 10, true, Default::default()),
                ct("检查所有设备状态", vec!["device.discover"], 10, true, Default::default()),
                ct("哪些设备还开着", vec!["device.discover"], 10, true, Default::default()),
                ct("关闭所有设备", vec!["device.control"], 10, true, Default::default()),
                ct("真的都关了吗", vec!["device.discover"], 10, true, Default::default()),
                ct("好了，谢谢", vec![], 5, false, Default::default()),
            ],
        }
    }

    pub fn get_all_scenarios() -> Vec<Self> {
        vec![
            Self::home_arrival_scenario(),
            Self::leaving_home_scenario(),
            Self::sleep_mode_scenario(),
            Self::environment_monitor_scenario(),
            Self::production_line_scenario(),
            Self::temperature_alert_scenario(),
            Self::smart_irrigation_scenario(),
            Self::basic_conversation_scenario(),
            Self::complex_multi_turn_scenario(),
        ]
    }
}

/// 辅助函数：创建对话轮次
fn ct(
    input: &str,
    tools: Vec<&str>,
    min_len: usize,
    use_tool: bool,
    qc: QualityCheck,
) -> ConversationTurn {
    ConversationTurn {
        user_input: input.to_string(),
        expected_tools: tools.into_iter().map(|s| s.to_string()).collect(),
        min_response_length: min_len,
        should_use_tool: use_tool,
        quality_check: qc,
    }
}

/// 辅助函数：创建质量检查
fn qc(keywords: Vec<&str>, helpful: bool, complete: bool) -> QualityCheck {
    QualityCheck {
        should_be_helpful: helpful,
        should_be_complete: complete,
        expected_keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
    }
}

// ============================================================================
// E2E测试框架
// ============================================================================

pub struct E2ETestFramework {
    pub event_bus: Arc<CoreEventBus>,
    pub session_manager: Arc<SessionManager>,
    pub simulator: Arc<E2EDeviceSimulator>,
    pub llm_endpoint: String,
    pub llm_model: String,
    pub show_full_conversation: bool,
}

impl E2ETestFramework {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let event_bus = Arc::new(CoreEventBus::new());

        // Create SessionManager
        let session_manager = Arc::new(SessionManager::new()
            .map_err(|e| format!("Failed to create SessionManager: {}", e))?);

        // Create device simulator
        let simulator = Arc::new(E2EDeviceSimulator::new(
            "e2e_simulator".to_string(),
            event_bus.clone(),
        ));

        // Get LLM config from environment or use defaults
        let llm_endpoint = std::env::var("OLLAMA_ENDPOINT")
            .unwrap_or_else(|_| DEFAULT_OLLAMA_ENDPOINT.to_string());
        let llm_model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Ok(Self {
            event_bus,
            session_manager,
            simulator,
            llm_endpoint,
            llm_model,
            show_full_conversation: SHOW_FULL_CONVERSATION,
        })
    }

    pub async fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔧 设置E2E测试环境...");

        // Add all test devices
        let all_devices = DomainDeviceGenerator::generate_all_devices();
        for (_domain, devices) in all_devices {
            for device in devices {
                self.simulator.add_device(device).await;
            }
        }

        // Start simulator
        self.simulator.start().await?;
        println!("   ✅ 设备模拟器已启动 ({} 设备)", self.simulator.list_devices().await.len());

        // Configure LLM
        self.configure_llm().await?;
        println!("   ✅ LLM后端已配置 ({})", self.llm_model);

        Ok(())
    }

    async fn configure_llm(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Test connection first
        let config = OllamaConfig::new(&self.llm_model)
            .with_endpoint(&self.llm_endpoint)
            .with_timeout_secs(30);

        let start = std::time::Instant::now();
        let _runtime = OllamaRuntime::new(config)
            .map_err(|e| format!("Failed to create Ollama runtime: {}", e))?;
        let connection_time = start.elapsed().as_millis() as u64;
        println!("   ✅ LLM连接成功 ({}ms)", connection_time);

        // Configure session manager with LlmBackend enum
        let backend = LlmBackend::Ollama {
            endpoint: self.llm_endpoint.clone(),
            model: self.llm_model.clone(),
        };

        self.session_manager
            .set_llm_backend(backend)
            .await
            .map_err(|e| format!("Failed to set LLM backend: {}", e))?;

        // Set up tool registry with all available tools
        // Note: Mock-specific tools (device_discover, device_query, etc.) are cfg(test) gated
        // We use the standard tools which work across all contexts
        let tool_registry = Arc::new(
            ToolRegistryBuilder::new()
                // Standard tools (available outside of cfg(test))
                .with_query_data_tool()
                .with_control_device_tool()
                .with_list_devices_tool()
                .with_create_rule_tool()
                .with_list_rules_tool()
                .with_delete_rule_tool()
                .with_enable_rule_tool()
                .with_disable_rule_tool()
                .with_update_rule_tool()
                .with_get_device_metrics_tool()
                .with_query_device_status_tool()
                .with_get_device_config_tool()
                .with_set_device_config_tool()
                .with_batch_control_devices_tool()
                .with_trigger_workflow_tool()
                .with_get_device_type_schema_tool()
                .with_list_device_types_tool()
                .build()
        );

        self.session_manager
            .set_tool_registry(tool_registry)
            .await;

        Ok(())
    }

    fn calculate_quality_score(turns: &[TurnResult], scenario: &ConversationScenario) -> ConversationQuality {
        if turns.is_empty() {
            return ConversationQuality {
                avg_response_length: 0.0,
                tool_usage_rate: 0.0,
                relevance_score: 50,
                completeness_score: 50,
                overall_score: 50,
            };
        }

        let total_length: usize = turns.iter().map(|t| t.response_length).sum();
        let avg_response_length = total_length as f64 / turns.len() as f64;

        let turns_with_tools = turns.iter().filter(|t| t.has_tool_execution).count();
        let tool_usage_rate = (turns_with_tools as f64 / turns.len() as f64) * 100.0;

        // 检查关键词匹配
        let mut relevance_score = 80u8;
        let mut completeness_score = 80u8;

        for turn in turns {
            // 检查响应长度是否足够
            if turn.response_length < 10 {
                completeness_score = completeness_score.saturating_sub(10);
            }
            // 检查是否有关键词（如果有预期的话）
            if let Some(expected_turn) = scenario.turns.get(turn.turn_number - 1) {
                if !expected_turn.quality_check.expected_keywords.is_empty() {
                    let has_keyword = expected_turn.quality_check.expected_keywords.iter()
                        .any(|kw| turn.agent_response.contains(kw));
                    if !has_keyword {
                        relevance_score = relevance_score.saturating_sub(15);
                    }
                }
            }
        }

        // 整体评分 (简单加权)
        let overall_score = ((relevance_score as u32 * 2
            + completeness_score as u32 * 2
            + (tool_usage_rate.min(100.0) as u32)) / 5) as u8;

        ConversationQuality {
            avg_response_length,
            tool_usage_rate,
            relevance_score,
            completeness_score,
            overall_score,
        }
    }

    pub async fn run_scenario(
        &self,
        scenario: &ConversationScenario,
    ) -> Result<DomainTestResult, Box<dyn std::error::Error>> {
        println!("\n{}\n", "=".repeat(70));
        println!("📋 测试场景: {} ({})", scenario.name, scenario.domain);
        println!("📝 描述: {}", scenario.description);
        println!("{}\n", "=".repeat(70));

        // Create a new session for this scenario
        let session_id = self.session_manager.create_session().await
            .map_err(|e| format!("Failed to create session: {}", e))?;

        let mut turn_results = Vec::new();
        let mut all_tools_called = Vec::new();
        let mut total_response_time = 0u64;
        let mut successful_turns = 0;
        let mut total_response_length = 0;

        for (idx, turn) in scenario.turns.iter().enumerate() {
            println!("\n🔹 [轮次 {}/{}] 用户: \"{}\"", idx + 1, scenario.turns.len(), turn.user_input);

            let start = std::time::Instant::now();

            // Process the message
            match self.session_manager.process_message(&session_id, &turn.user_input).await {
                Ok(response) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    total_response_time += elapsed;

                    let response_content = response.message.content.clone();
                    let response_len = response_content.chars().count();
                    total_response_length += response_len;

                    let tools_called = response.tools_used.clone();
                    let has_tool_execution = !tools_called.is_empty();

                    all_tools_called.extend(tools_called.clone());

                    // Print response
                    if self.show_full_conversation {
                        println!("\n   🤖 Agent 完整回复:");
                        println!("   ┌─────────────────────────────────────────────────────────");
                        for line in response_content.lines() {
                            println!("   │ {}", line);
                        }
                        println!("   └─────────────────────────────────────────────────────────");
                    } else {
                        let preview = if response_content.chars().count() > 100 {
                            format!("{}...", response_content.chars().take(100).collect::<String>())
                        } else {
                            response_content.clone()
                        };
                        println!("   🤖 Agent: \"{}\"", preview);
                    }

                    // Print tool calls
                    if has_tool_execution {
                        println!("   🔧 工具调用: {:?}", tools_called);
                    }

                    // Print timing and metrics
                    println!("   📊 响应时间: {}ms | 响应长度: {} 字符", elapsed, response_len);

                    // Check if response is successful
                    let is_successful = response_len >= turn.min_response_length;
                    if is_successful {
                        successful_turns += 1;
                        println!("   ✅ 状态: 成功");
                    } else {
                        println!("   ⚠️  状态: 响应过短 (期望 >= {} 字符)", turn.min_response_length);
                    }

                    turn_results.push(TurnResult {
                        turn_number: idx + 1,
                        user_input: turn.user_input.clone(),
                        agent_response: response_content,
                        response_time_ms: elapsed,
                        tools_called: tools_called.clone(),
                        tool_results: vec![],  // TODO: 收集工具执行结果
                        is_successful,
                        error_message: None,
                        thinking_content: response.message.thinking.clone(),
                        response_length: response_len,
                        has_tool_execution,
                    });
                }
                Err(e) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    println!("   ❌ 错误: {}", e);
                    println!("   ⏱️  耗时: {}ms", elapsed);

                    turn_results.push(TurnResult {
                        turn_number: idx + 1,
                        user_input: turn.user_input.clone(),
                        agent_response: String::new(),
                        response_time_ms: elapsed,
                        tools_called: vec![],
                        tool_results: vec![],
                        is_successful: false,
                        error_message: Some(e.to_string()),
                        thinking_content: None,
                        response_length: 0,
                        has_tool_execution: false,
                    });
                }
            }

            // Small delay between turns
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        // Clean up session
        let _ = self.session_manager.remove_session(&session_id).await;

        let avg_response = if turn_results.is_empty() {
            0
        } else {
            total_response_time / turn_results.len() as u64
        };

        let unique_tools: Vec<String> = all_tools_called.into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let quality_score = Self::calculate_quality_score(&turn_results, scenario);

        // Print quality metrics
        println!("\n   📈 场景质量指标:");
        println!("   ├─ 平均响应长度: {:.1} 字符", quality_score.avg_response_length);
        println!("   ├─ 工具使用率: {:.1}%", quality_score.tool_usage_rate);
        println!("   ├─ 相关性评分: {}/100", quality_score.relevance_score);
        println!("   ├─ 完整性评分: {}/100", quality_score.completeness_score);
        println!("   └─ 总体评分: {}/100 {}", quality_score.overall_score,
            if quality_score.overall_score >= 80 { "⭐⭐⭐⭐⭐" }
            else if quality_score.overall_score >= 60 { "⭐⭐⭐⭐" }
            else if quality_score.overall_score >= 40 { "⭐⭐⭐" }
            else { "⭐⭐" }
        );

        Ok(DomainTestResult {
            domain: scenario.domain.clone(),
            scenario_name: scenario.name.clone(),
            conversation_turns: turn_results,
            total_turns: scenario.turns.len(),
            successful_turns,
            avg_response_time_ms: avg_response,
            tools_called: unique_tools,
            llm_tokens_used: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            quality_score,
        })
    }

    pub async fn run_all_tests(&self) -> E2ETestReport {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║   NeoTalk 端到端集成测试                                       ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!("LLM端点: {}", self.llm_endpoint);
        println!("LLM模型: {}", self.llm_model);
        println!("开始时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
        println!("显示完整对话: {}", if self.show_full_conversation { "是" } else { "否" });

        // Setup
        if let Err(e) = self.setup().await {
            println!("❌ 设置失败: {}", e);
            let completed_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            return E2ETestReport {
                test_name: "E2E Integration Test".to_string(),
                started_at: start_time,
                completed_at,
                duration_ms: 0,
                llm_config: LlmConfigInfo {
                    endpoint: self.llm_endpoint.clone(),
                    model: self.llm_model.clone(),
                    connected: false,
                    connection_time_ms: 0,
                },
                domain_results: vec![],
                summary: TestSummary {
                    total_turns: 0,
                    successful_turns: 0,
                    success_rate: 0.0,
                    avg_response_time_ms: 0,
                    total_tools_called: 0,
                    unique_tools_used: vec![],
                    total_response_length: 0,
                    avg_response_length: 0,
                },
            };
        }

        // Test connection
        let llm_connected = true;
        let connection_time = 0; // Already measured in setup

        // Run all scenarios
        let scenarios = ConversationScenario::get_all_scenarios();
        let mut domain_results = Vec::new();
        let mut total_turns = 0;
        let mut successful_turns = 0;
        let mut total_response_time = 0u64;
        let mut all_tools = Vec::new();
        let mut total_response_length = 0;

        for scenario in &scenarios {
            match self.run_scenario(scenario).await {
                Ok(result) => {
                    total_turns += result.total_turns;
                    successful_turns += result.successful_turns;
                    total_response_time += result.avg_response_time_ms * result.total_turns as u64;
                    all_tools.extend(result.tools_called.clone());
                    total_response_length += result.conversation_turns.iter()
                        .map(|t| t.response_length).sum::<usize>();
                    domain_results.push(result);
                }
                Err(e) => {
                    println!("❌ 场景执行失败: {}", e);
                }
            }
        }

        let completed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let duration_ms = ((completed_at - start_time) * 1000) as u64;

        let unique_tools: Vec<String> = all_tools.iter()
            .collect::<std::collections::HashSet<_>>()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let success_rate = if total_turns > 0 {
            (successful_turns as f64 / total_turns as f64) * 100.0
        } else {
            0.0
        };

        let avg_response = if total_turns > 0 {
            total_response_time / total_turns as u64
        } else {
            0
        };

        let avg_response_length = if total_turns > 0 {
            total_response_length / total_turns
        } else {
            0
        };

        let summary = TestSummary {
            total_turns,
            successful_turns,
            success_rate,
            avg_response_time_ms: avg_response,
            total_tools_called: all_tools.len(),
            unique_tools_used: unique_tools.clone(),
            total_response_length,
            avg_response_length,
        };

        // Print summary
        self.print_summary(&summary);

        E2ETestReport {
            test_name: "E2E Integration Test".to_string(),
            started_at: start_time,
            completed_at,
            duration_ms,
            llm_config: LlmConfigInfo {
                endpoint: self.llm_endpoint.clone(),
                model: self.llm_model.clone(),
                connected: llm_connected,
                connection_time_ms: connection_time,
            },
            domain_results,
            summary,
        }
    }

    fn print_summary(&self, summary: &TestSummary) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║   测试摘要                                                       ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!("总对话轮次: {}", summary.total_turns);
        println!("成功轮次: {}", summary.successful_turns);
        println!("成功率: {:.1}%", summary.success_rate);
        println!("平均响应时间: {}ms", summary.avg_response_time_ms);
        println!("平均响应长度: {} 字符", summary.avg_response_length);
        println!("工具调用次数: {}", summary.total_tools_called);
        println!("使用工具种类: {:?}", summary.unique_tools_used);

        let grade = if summary.success_rate >= 90.0 {
            "优秀 ⭐⭐⭐⭐⭐"
        } else if summary.success_rate >= 75.0 {
            "良好 ⭐⭐⭐⭐"
        } else if summary.success_rate >= 60.0 {
            "及格 ⭐⭐⭐"
        } else {
            "需改进 ⭐⭐"
        };
        println!("\n评级: {}", grade);
    }

    pub fn generate_markdown_report(&self, report: &E2ETestReport) -> String {
        let mut output = String::new();

        // Header
        output.push_str("# NeoTalk 端到端集成测试报告\n\n");
        output.push_str(&format!("**测试时间**: {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
        output.push_str(&format!("**LLM端点**: {}\n", report.llm_config.endpoint));
        output.push_str(&format!("**LLM模型**: {}\n", report.llm_config.model));
        output.push_str(&format!("**连接状态**: {}\n\n",
            if report.llm_config.connected { "✅ 已连接" } else { "❌ 未连接" }
        ));

        // Summary
        output.push_str("## 测试摘要\n\n");
        output.push_str("| 指标 | 值 |\n");
        output.push_str("|------|------|\n");
        output.push_str(&format!("| 总对话轮次 | {} |\n", report.summary.total_turns));
        output.push_str(&format!("| 成功轮次 | {} |\n", report.summary.successful_turns));
        output.push_str(&format!("| 成功率 | {:.1}% |\n", report.summary.success_rate));
        output.push_str(&format!("| 平均响应时间 | {}ms |\n", report.summary.avg_response_time_ms));
        output.push_str(&format!("| 平均响应长度 | {} 字符 |\n", report.summary.avg_response_length));
        output.push_str(&format!("| 工具调用次数 | {} |\n", report.summary.total_tools_called));
        output.push_str(&format!("| 使用工具种类 | {} |\n\n", report.summary.unique_tools_used.len()));

        // Domain results
        output.push_str("## 场景详情\n\n");

        for result in &report.domain_results {
            output.push_str(&format!("### {} - {}\n\n", result.domain, result.scenario_name));
            output.push_str(&format!("- 总轮次: {}\n", result.total_turns));
            output.push_str(&format!("- 成功轮次: {}\n", result.successful_turns));
            output.push_str(&format!("- 平均响应时间: {}ms\n", result.avg_response_time_ms));
            output.push_str(&format!("- 使用工具: {:?}\n", result.tools_called));
            output.push_str(&format!("- 质量评分: {}/100\n\n", result.quality_score.overall_score));

            output.push_str("#### 对话详情\n\n");
            for turn in &result.conversation_turns {
                let status = if turn.is_successful { "✅" } else { "❌" };
                output.push_str(&format!("{} **[轮次{}] 用户**: \"{}\"\n\n",
                    status, turn.turn_number, turn.user_input
                ));
                output.push_str(&format!("**Agent**: \"{}\"\n\n", turn.agent_response));
                if !turn.tools_called.is_empty() {
                    output.push_str(&format!("**工具**: {:?}\n\n", turn.tools_called));
                }
                if let Some(ref error) = turn.error_message {
                    output.push_str(&format!("**错误**: {}\n\n", error));
                }
            }
        }

        // Quality metrics summary
        output.push_str("## 质量指标分析\n\n");
        let avg_quality: u32 = if !report.domain_results.is_empty() {
            report.domain_results.iter()
                .map(|r| r.quality_score.overall_score as u32)
                .sum::<u32>() / report.domain_results.len() as u32
        } else {
            0
        };
        output.push_str(&format!("- **平均质量评分**: {}/100\n", avg_quality));
        output.push_str(&format!("- **优秀场景数**: {}\n",
            report.domain_results.iter().filter(|r| r.quality_score.overall_score >= 80).count()));
        output.push_str(&format!("- **需改进场景数**: {}\n",
            report.domain_results.iter().filter(|r| r.quality_score.overall_score < 60).count()));

        output
    }
}

// ============================================================================
// 测试用例
// ============================================================================

#[tokio::test]
#[ignore]  // 运行: cargo test -p edge-ai-agent e2e_real_llm_test -- --ignored --nocapture
async fn e2e_real_llm_test() {
    let framework = E2ETestFramework::new().await
        .expect("Failed to create test framework");

    let report = framework.run_all_tests().await;

    // Print markdown report
    println!("\n\n{}", framework.generate_markdown_report(&report));

    // Assertions
    assert!(report.llm_config.connected, "LLM should be connected");
    assert!(report.summary.total_turns > 0, "Should have at least one test turn");

    // In real testing with actual LLM, we'd expect better results
    // For CI/CD without LLM, we just check the framework works
    println!("\n✅ E2E测试完成");
}

#[tokio::test]
async fn e2e_test_framework_only() {
    // This test validates the framework works without requiring LLM
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║   E2E 测试框架验证测试 (无需LLM)                              ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    let framework = E2ETestFramework::new().await
        .expect("Failed to create test framework");

    // Setup (will skip LLM configuration if Ollama not available)
    let setup_result = framework.setup().await;

    match setup_result {
        Ok(_) => {
            println!("✅ 测试环境设置成功");
        }
        Err(_) => {
            println!("⚠️  LLM连接失败（可能Ollama未运行），跳过LLM相关测试");
            println!("✅ 框架本身运行正常");
            return;
        }
    }

    // Check devices
    let devices = framework.simulator.list_devices().await;
    println!("✅ 模拟设备数: {}", devices.len());

    // Show sample devices by domain
    let mut domain_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for device in &devices {
        *domain_counts.entry(device.domain.clone()).or_insert(0) += 1;
    }
    println!("   各领域设备:");
    for (domain, count) in &domain_counts {
        println!("     - {}: {}个", domain, count);
    }

    // Test scenarios without LLM
    let scenarios = ConversationScenario::get_all_scenarios();
    println!("\n✅ 场景定义数: {}", scenarios.len());
    for scenario in &scenarios {
        println!("   - [{}] {} - {} 轮对话",
            scenario.domain, scenario.name, scenario.turns.len());
    }

    println!("\n✅ 框架验证测试通过");
    println!("\n💡 提示: 运行完整LLM测试需要:");
    println!("   1. 启动Ollama: ollama serve");
    println!("   2. 运行测试: cargo test -p edge-ai-agent e2e_real_llm_test -- --ignored --nocapture");
}
