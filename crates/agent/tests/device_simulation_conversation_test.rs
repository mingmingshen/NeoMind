//! NeoTalk 设备模拟与Agent对话质量测试
//!
//! 测试目标:
//! - 模拟大规模设备环境 (100+ 设备)
//! - 测试Agent与设备交互的对话质量
//! - 评估中英文对话表现
//! - 分析多轮对话上下文保持能力
//!
//! **测试日期**: 2026-01-18

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use edge_ai_llm::backends::create_backend;
use edge_ai_core::llm::backend::{GenerationParams, LlmInput};
use edge_ai_core::message::{Message, MessageRole, Content};

const OLLAMA_ENDPOINT: &str = "http://localhost:11434";

// ============================================================================
// 设备模拟
// ============================================================================

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceType {
    Light,           // 灯光
    Sensor,          // 传感器
    Switch,          // 开关
    Thermostat,      // 温控器
    Camera,          // 摄像头
    Curtain,         // 窗帘
    Lock,            // 门锁
    Fan,             // 风扇
    AirConditioner,  // 空调
    Humidifier,      // 加湿器
}

impl DeviceType {
    pub fn all_types() -> Vec<DeviceType> {
        vec![
            DeviceType::Light,
            DeviceType::Sensor,
            DeviceType::Switch,
            DeviceType::Thermostat,
            DeviceType::Camera,
            DeviceType::Curtain,
            DeviceType::Lock,
            DeviceType::Fan,
            DeviceType::AirConditioner,
            DeviceType::Humidifier,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            DeviceType::Light => "light",
            DeviceType::Sensor => "sensor",
            DeviceType::Switch => "switch",
            DeviceType::Thermostat => "thermostat",
            DeviceType::Camera => "camera",
            DeviceType::Curtain => "curtain",
            DeviceType::Lock => "lock",
            DeviceType::Fan => "fan",
            DeviceType::AirConditioner => "aircon",
            DeviceType::Humidifier => "humidifier",
        }
    }

    pub fn cn_name(&self) -> &str {
        match self {
            DeviceType::Light => "灯光",
            DeviceType::Sensor => "传感器",
            DeviceType::Switch => "开关",
            DeviceType::Thermostat => "温控器",
            DeviceType::Camera => "摄像头",
            DeviceType::Curtain => "窗帘",
            DeviceType::Lock => "门锁",
            DeviceType::Fan => "风扇",
            DeviceType::AirConditioner => "空调",
            DeviceType::Humidifier => "加湿器",
        }
    }
}

/// 房间位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoomLocation {
    LivingRoom,
    Bedroom,
    Kitchen,
    Bathroom,
    Study,
    DiningRoom,
    Balcony,
    Garage,
    Garden,
    Corridor,
}

impl RoomLocation {
    pub fn all_locations() -> Vec<RoomLocation> {
        vec![
            RoomLocation::LivingRoom,
            RoomLocation::Bedroom,
            RoomLocation::Kitchen,
            RoomLocation::Bathroom,
            RoomLocation::Study,
            RoomLocation::DiningRoom,
            RoomLocation::Balcony,
            RoomLocation::Garage,
            RoomLocation::Garden,
            RoomLocation::Corridor,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            RoomLocation::LivingRoom => "living_room",
            RoomLocation::Bedroom => "bedroom",
            RoomLocation::Kitchen => "kitchen",
            RoomLocation::Bathroom => "bathroom",
            RoomLocation::Study => "study",
            RoomLocation::DiningRoom => "dining_room",
            RoomLocation::Balcony => "balcony",
            RoomLocation::Garage => "garage",
            RoomLocation::Garden => "garden",
            RoomLocation::Corridor => "corridor",
        }
    }

    pub fn cn_name(&self) -> &str {
        match self {
            RoomLocation::LivingRoom => "客厅",
            RoomLocation::Bedroom => "卧室",
            RoomLocation::Kitchen => "厨房",
            RoomLocation::Bathroom => "浴室",
            RoomLocation::Study => "书房",
            RoomLocation::DiningRoom => "餐厅",
            RoomLocation::Balcony => "阳台",
            RoomLocation::Garage => "车库",
            RoomLocation::Garden => "花园",
            RoomLocation::Corridor => "走廊",
        }
    }
}

/// 设备状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceState {
    On,
    Off,
    Level(u8),         // 0-100
    Temperature(f32),  // 摄氏度
    Humidity(u8),      // 百分比
    Motion(bool),      // 运动检测
    Locked(bool),      // 锁定状态
    Open(bool),        // 开关状态
}

/// 模拟设备
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedDevice {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub room: RoomLocation,
    pub state: DeviceState,
    pub online: bool,
    pub properties: HashMap<String, Value>,
}

impl SimulatedDevice {
    pub fn generate_id(device_type: DeviceType, room: RoomLocation, index: usize) -> String {
        format!("{}_{}_{}", device_type.name(), room.name(), index)
    }

    pub fn generate_name(device_type: DeviceType, room: RoomLocation, index: usize) -> String {
        let room_cn = room.cn_name();
        let type_cn = device_type.cn_name();
        if index == 1 {
            format!("{}{}", room_cn, type_cn)
        } else {
            format!("{}{}{}", room_cn, type_cn, index)
        }
    }

    pub fn new(device_type: DeviceType, room: RoomLocation, index: usize) -> Self {
        let id = Self::generate_id(device_type, room, index);
        let name = Self::generate_name(device_type, room, index);

        let state = match device_type {
            DeviceType::Light => DeviceState::Off,
            DeviceType::Sensor => DeviceState::Temperature(25.0),
            DeviceType::Switch => DeviceState::Off,
            DeviceType::Thermostat => DeviceState::Temperature(24.0),
            DeviceType::Camera => DeviceState::On,
            DeviceType::Curtain => DeviceState::Open(false),
            DeviceType::Lock => DeviceState::Locked(true),
            DeviceType::Fan => DeviceState::Off,
            DeviceType::AirConditioner => DeviceState::Off,
            DeviceType::Humidifier => DeviceState::Level(50),
        };

        let mut properties = HashMap::new();
        properties.insert("battery".to_string(), json!(85));
        properties.insert("last_update".to_string(), json!(chrono::Utc::now().timestamp()));

        SimulatedDevice {
            id,
            name,
            device_type,
            room,
            state,
            online: true,
            properties,
        }
    }

    pub fn get_status_text(&self, language: TestLanguage) -> String {
        match language {
            TestLanguage::Chinese => {
                format!("{} 状态: {}, 在线: {}",
                    self.name,
                    match &self.state {
                        DeviceState::On => "开启".to_string(),
                        DeviceState::Off => "关闭".to_string(),
                        DeviceState::Level(l) => format!("{}%", l),
                        DeviceState::Temperature(t) => format!("{}°C", t),
                        DeviceState::Humidity(h) => format!("{}%", h),
                        DeviceState::Motion(m) => if *m { "检测到运动" } else { "无运动" }.to_string(),
                        DeviceState::Locked(l) => if *l { "已锁定" } else { "已解锁" }.to_string(),
                        DeviceState::Open(o) => if *o { "开启" } else { "关闭" }.to_string(),
                    },
                    if self.online { "是" } else { "否" }
                )
            }
            TestLanguage::English => {
                format!("{} status: {}, online: {}",
                    self.name,
                    match &self.state {
                        DeviceState::On => "on".to_string(),
                        DeviceState::Off => "off".to_string(),
                        DeviceState::Level(l) => format!("{}%", l),
                        DeviceState::Temperature(t) => format!("{}°C", t),
                        DeviceState::Humidity(h) => format!("{}%", h),
                        DeviceState::Motion(m) => if *m { "motion detected" } else { "no motion" }.to_string(),
                        DeviceState::Locked(l) => if *l { "locked" } else { "unlocked" }.to_string(),
                        DeviceState::Open(o) => if *o { "open" } else { "closed" }.to_string(),
                    },
                    if self.online { "yes" } else { "no" }
                )
            }
        }
    }
}

/// 设备模拟环境
#[derive(Debug, Clone)]
pub struct DeviceSimulationEnvironment {
    pub devices: Vec<SimulatedDevice>,
    pub device_by_id: HashMap<String, SimulatedDevice>,
    pub devices_by_room: HashMap<RoomLocation, Vec<SimulatedDevice>>,
    pub devices_by_type: HashMap<DeviceType, Vec<SimulatedDevice>>,
}

impl DeviceSimulationEnvironment {
    pub fn new(device_count: usize) -> Self {
        let mut devices = Vec::new();
        let mut device_by_id = HashMap::new();
        let mut devices_by_room: HashMap<RoomLocation, Vec<SimulatedDevice>> = HashMap::new();
        let mut devices_by_type: HashMap<DeviceType, Vec<SimulatedDevice>> = HashMap::new();

        let types = DeviceType::all_types();
        let locations = RoomLocation::all_locations();

        let mut index = 1;
        for room in &locations {
            for device_type in &types {
                // 每个房间每种类型至少创建1个设备
                let device = SimulatedDevice::new(*device_type, *room, index);
                index += 1;

                device_by_id.insert(device.id.clone(), device.clone());
                devices_by_room.entry(*room).or_default().push(device.clone());
                devices_by_type.entry(*device_type).or_default().push(device.clone());
                devices.push(device.clone());

                if devices.len() >= device_count {
                    break;
                }
            }
            if devices.len() >= device_count {
                break;
            }
        }

        // 继续添加设备直到达到目标数量
        while devices.len() < device_count {
            let room = locations[index % locations.len()];
            let device_type = types[index % types.len()];
            let device = SimulatedDevice::new(device_type, room, index);
            index += 1;

            device_by_id.insert(device.id.clone(), device.clone());
            devices_by_room.entry(room).or_default().push(device.clone());
            devices_by_type.entry(device_type).or_default().push(device.clone());
            devices.push(device);
        }

        DeviceSimulationEnvironment {
            devices,
            device_by_id,
            devices_by_room,
            devices_by_type,
        }
    }

    pub fn get_device(&self, id: &str) -> Option<&SimulatedDevice> {
        self.device_by_id.get(id)
    }

    pub fn get_devices_by_room(&self, room: RoomLocation) -> Vec<&SimulatedDevice> {
        self.devices_by_room.get(&room)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<&SimulatedDevice> {
        self.devices_by_type.get(&device_type)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn find_devices_by_name_fuzzy(&self, name_pattern: &str) -> Vec<&SimulatedDevice> {
        let pattern_lower = name_pattern.to_lowercase();
        self.devices.iter()
            .filter(|d| d.name.to_lowercase().contains(&pattern_lower) ||
                       d.id.to_lowercase().contains(&pattern_lower))
            .collect()
    }

    pub fn get_device_summary(&self, language: TestLanguage) -> String {
        match language {
            TestLanguage::Chinese => {
                format!("设备环境: 共{}个设备, {}个房间, {}种设备类型",
                    self.devices.len(),
                    self.devices_by_room.len(),
                    self.devices_by_type.len()
                )
            }
            TestLanguage::English => {
                format!("Device Environment: {} devices, {} rooms, {} device types",
                    self.devices.len(),
                    self.devices_by_room.len(),
                    self.devices_by_type.len()
                )
            }
        }
    }
}

// ============================================================================
// 语言设置
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestLanguage {
    Chinese,
    English,
}

// ============================================================================
// 对话场景
// ============================================================================

#[derive(Debug, Clone)]
pub struct ConversationScenario {
    pub name: String,
    pub description: String,
    pub turns: Vec<ConversationTurn>,
    pub language: TestLanguage,
    pub expected_device_count: Option<usize>,
    pub expected_device_types: Option<Vec<DeviceType>>,
}

#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub user_message: String,
    pub expected_intents: Vec<Intent>,
    pub expected_entities: Vec<Entity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    QueryDeviceStatus,
    QueryDeviceList,
    ControlDevice,
    QueryRoomDevices,
    QueryTypeDevices,
    BatchControl,
    ConditionalQuery,
    SceneActivation,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_type: String,
    pub value: String,
    pub confidence: f64,
}

/// 获取测试场景
pub fn get_conversation_scenarios(env: &DeviceSimulationEnvironment) -> Vec<ConversationScenario> {
    let device_count = env.devices.len();

    vec![
        // 场景1: 简单查询
        ConversationScenario {
            name: "简单设备状态查询".to_string(),
            description: "查询单个设备状态".to_string(),
            language: TestLanguage::Chinese,
            expected_device_count: Some(1),
            expected_device_types: None,
            turns: vec![
                ConversationTurn {
                    user_message: "客厅灯的状态是什么？".to_string(),
                    expected_intents: vec![Intent::QueryDeviceStatus],
                    expected_entities: vec![
                        Entity { entity_type: "room".to_string(), value: "客厅".to_string(), confidence: 1.0 },
                        Entity { entity_type: "device_type".to_string(), value: "灯光".to_string(), confidence: 1.0 },
                    ],
                },
            ],
        },

        // 场景2: 房间设备查询
        ConversationScenario {
            name: "房间设备列表查询".to_string(),
            description: "查询特定房间的所有设备".to_string(),
            language: TestLanguage::Chinese,
            expected_device_count: Some(10),
            expected_device_types: None,
            turns: vec![
                ConversationTurn {
                    user_message: format!("卧室里有哪些设备？当前有{}个设备", device_count),
                    expected_intents: vec![Intent::QueryRoomDevices],
                    expected_entities: vec![
                        Entity { entity_type: "room".to_string(), value: "卧室".to_string(), confidence: 1.0 },
                    ],
                },
            ],
        },

        // 场景3: 批量控制
        ConversationScenario {
            name: "批量设备控制".to_string(),
            description: "控制多个房间的设备".to_string(),
            language: TestLanguage::Chinese,
            expected_device_count: None,
            expected_device_types: Some(vec![DeviceType::Light]),
            turns: vec![
                ConversationTurn {
                    user_message: "把所有灯都打开".to_string(),
                    expected_intents: vec![Intent::BatchControl],
                    expected_entities: vec![
                        Entity { entity_type: "device_type".to_string(), value: "灯光".to_string(), confidence: 1.0 },
                        Entity { entity_type: "action".to_string(), value: "打开".to_string(), confidence: 1.0 },
                        Entity { entity_type: "scope".to_string(), value: "所有".to_string(), confidence: 1.0 },
                    ],
                },
            ],
        },

        // 场景4: 条件查询
        ConversationScenario {
            name: "条件式设备查询".to_string(),
            description: "根据条件查询设备".to_string(),
            language: TestLanguage::Chinese,
            expected_device_count: None,
            expected_device_types: None,
            turns: vec![
                ConversationTurn {
                    user_message: "哪些传感器温度超过30度？".to_string(),
                    expected_intents: vec![Intent::ConditionalQuery],
                    expected_entities: vec![
                        Entity { entity_type: "device_type".to_string(), value: "传感器".to_string(), confidence: 1.0 },
                        Entity { entity_type: "condition".to_string(), value: "温度>30".to_string(), confidence: 1.0 },
                    ],
                },
            ],
        },

        // 场景5: 多轮对话 - 上下文保持
        ConversationScenario {
            name: "多轮上下文对话".to_string(),
            description: "测试上下文理解能力".to_string(),
            language: TestLanguage::Chinese,
            expected_device_count: None,
            expected_device_types: None,
            turns: vec![
                ConversationTurn {
                    user_message: "客厅有哪些设备？".to_string(),
                    expected_intents: vec![Intent::QueryRoomDevices],
                    expected_entities: vec![Entity { entity_type: "room".to_string(), value: "客厅".to_string(), confidence: 1.0 }],
                },
                ConversationTurn {
                    user_message: "把第一个设备打开".to_string(),
                    expected_intents: vec![Intent::ControlDevice],
                    expected_entities: vec![Entity { entity_type: "reference".to_string(), value: "第一个".to_string(), confidence: 0.8 }],
                },
                ConversationTurn {
                    user_message: "它现在是什么状态？".to_string(),
                    expected_intents: vec![Intent::QueryDeviceStatus],
                    expected_entities: vec![Entity { entity_type: "reference".to_string(), value: "它".to_string(), confidence: 0.7 }],
                },
            ],
        },

        // English scenarios
        ConversationScenario {
            name: "Simple Device Status Query".to_string(),
            description: "Query single device status".to_string(),
            language: TestLanguage::English,
            expected_device_count: Some(1),
            expected_device_types: None,
            turns: vec![
                ConversationTurn {
                    user_message: "What's the status of the living room light?".to_string(),
                    expected_intents: vec![Intent::QueryDeviceStatus],
                    expected_entities: vec![
                        Entity { entity_type: "room".to_string(), value: "living room".to_string(), confidence: 1.0 },
                        Entity { entity_type: "device_type".to_string(), value: "light".to_string(), confidence: 1.0 },
                    ],
                },
            ],
        },

        ConversationScenario {
            name: "Room Device List Query".to_string(),
            description: "Query all devices in a specific room".to_string(),
            language: TestLanguage::English,
            expected_device_count: Some(10),
            expected_device_types: None,
            turns: vec![
                ConversationTurn {
                    user_message: format!("What devices are in the bedroom? There are currently {} devices", device_count),
                    expected_intents: vec![Intent::QueryRoomDevices],
                    expected_entities: vec![
                        Entity { entity_type: "room".to_string(), value: "bedroom".to_string(), confidence: 1.0 },
                    ],
                },
            ],
        },

        ConversationScenario {
            name: "Batch Device Control".to_string(),
            description: "Control devices in multiple rooms".to_string(),
            language: TestLanguage::English,
            expected_device_count: None,
            expected_device_types: Some(vec![DeviceType::Light]),
            turns: vec![
                ConversationTurn {
                    user_message: "Turn on all the lights".to_string(),
                    expected_intents: vec![Intent::BatchControl],
                    expected_entities: vec![
                        Entity { entity_type: "device_type".to_string(), value: "light".to_string(), confidence: 1.0 },
                        Entity { entity_type: "action".to_string(), value: "turn on".to_string(), confidence: 1.0 },
                        Entity { entity_type: "scope".to_string(), value: "all".to_string(), confidence: 1.0 },
                    ],
                },
            ],
        },

        ConversationScenario {
            name: "Multi-turn Context Conversation".to_string(),
            description: "Test context understanding".to_string(),
            language: TestLanguage::English,
            expected_device_count: None,
            expected_device_types: None,
            turns: vec![
                ConversationTurn {
                    user_message: "What devices are in the living room?".to_string(),
                    expected_intents: vec![Intent::QueryRoomDevices],
                    expected_entities: vec![Entity { entity_type: "room".to_string(), value: "living room".to_string(), confidence: 1.0 }],
                },
                ConversationTurn {
                    user_message: "Turn on the first one".to_string(),
                    expected_intents: vec![Intent::ControlDevice],
                    expected_entities: vec![Entity { entity_type: "reference".to_string(), value: "first one".to_string(), confidence: 0.8 }],
                },
                ConversationTurn {
                    user_message: "What's its status now?".to_string(),
                    expected_intents: vec![Intent::QueryDeviceStatus],
                    expected_entities: vec![Entity { entity_type: "reference".to_string(), value: "its".to_string(), confidence: 0.7 }],
                },
            ],
        },
    ]
}

// ============================================================================
// Agent对话评估器
// ============================================================================

pub struct AgentConversationEvaluator {
    model_name: String,
    llm: Arc<dyn edge_ai_core::llm::backend::LlmRuntime>,
    timeout_secs: u64,
    environment: DeviceSimulationEnvironment,
}

impl AgentConversationEvaluator {
    pub fn new(model_name: &str, device_count: usize) -> Result<Self, String> {
        let llm_config = serde_json::json!({
            "endpoint": OLLAMA_ENDPOINT,
            "model": model_name
        });

        let llm = create_backend("ollama", &llm_config)
            .map_err(|e| format!("Failed to create LLM backend: {:?}", e))?;

        let environment = DeviceSimulationEnvironment::new(device_count);

        Ok(Self {
            model_name: model_name.to_string(),
            llm,
            timeout_secs: 60,
            environment,
        })
    }

    /// 运行所有对话场景测试
    pub async fn evaluate_all_scenarios(&self) -> ConversationEvaluationReport {
        let scenarios = get_conversation_scenarios(&self.environment);

        println!("\n╔════════════════════════════════════════════════════════════════════════╗");
        println!("║   NeoTalk Agent对话质量测试                                            ║");
        println!("║   模型: {:58}║", self.model_name);
        println!("║   设备数: {:57}║", self.environment.devices.len());
        println!("╚════════════════════════════════════════════════════════════════════════╝");

        let mut results = Vec::new();

        for (idx, scenario) in scenarios.iter().enumerate() {
            println!("\n📋 场景 {}/{}: {}", idx + 1, scenarios.len(), scenario.name);
            println!("   描述: {}", scenario.description);
            println!("   语言: {:?}", scenario.language);

            let result = self.evaluate_scenario(scenario).await;
            self.print_scenario_result(&result);
            results.push(result);
        }

        self.generate_final_report(results)
    }

    /// 评估单个对话场景
    async fn evaluate_scenario(&self, scenario: &ConversationScenario) -> ScenarioEvaluationResult {
        let mut messages = vec![self.build_system_message(scenario.language)];
        let mut turn_results = Vec::new();

        let start = Instant::now();

        for (turn_idx, turn) in scenario.turns.iter().enumerate() {
            println!("\n   ── 第{}轮 / Turn {} ───", turn_idx + 1, turn_idx + 1);
            println!("   用户: {}", turn.user_message);

            // 添加用户消息
            messages.push(Message {
                role: MessageRole::User,
                content: Content::Text(turn.user_message.clone()),
                timestamp: None,
            });

            let turn_start = Instant::now();

            // 构建设备上下文信息
            let device_context = self.build_device_context(scenario.language);

            // 构建完整提示
            let full_prompt = self.build_full_prompt(&messages, &device_context, scenario.language);

            // 发送请求
            let response = self.send_prompt(&full_prompt).await;
            let response_time = turn_start.elapsed().as_millis();

            println!("   Agent: {}", response.chars().take(100).collect::<String>());
            if response.len() > 100 {
                println!("   ...");
            }
            println!("   响应时间: {}ms", response_time);

            // 评估这一轮
            let turn_eval = self.evaluate_turn(
                &turn,
                &response,
                turn_idx + 1,
                scenario.language,
                response_time,
            );

            println!("   意图识别: {:.1}% | 实体提取: {:.1}% | 相关性: {:.1}% | 格式: {:.1}%",
                turn_eval.intent_recognition_score,
                turn_eval.entity_extraction_score,
                turn_eval.relevance_score,
                turn_eval.format_score);

            // 添加助手响应
            messages.push(Message {
                role: MessageRole::Assistant,
                content: Content::Text(response.clone()),
                timestamp: None,
            });

            turn_results.push(turn_eval);
        }

        let total_time = start.elapsed().as_secs();

        // 计算场景得分
        let avg_intent = turn_results.iter().map(|t| t.intent_recognition_score).sum::<f64>() / turn_results.len().max(1) as f64;
        let avg_entity = turn_results.iter().map(|t| t.entity_extraction_score).sum::<f64>() / turn_results.len().max(1) as f64;
        let avg_relevance = turn_results.iter().map(|t| t.relevance_score).sum::<f64>() / turn_results.len().max(1) as f64;
        let avg_coherence = turn_results.iter().map(|t| t.coherence_score).sum::<f64>() / turn_results.len().max(1) as f64;
        let avg_format = turn_results.iter().map(|t| t.format_score).sum::<f64>() / turn_results.len().max(1) as f64;

        // 新的评分权重: 意图20%, 实体20%, 相关性30%, 连贯性15%, 格式15%
        let scenario_score = avg_intent * 0.20 + avg_entity * 0.20 + avg_relevance * 0.30 + avg_coherence * 0.15 + avg_format * 0.15;

        ScenarioEvaluationResult {
            scenario_name: scenario.name.clone(),
            language: scenario.language,
            turn_results,
            total_time_secs: total_time,
            avg_intent_recognition: avg_intent,
            avg_entity_extraction: avg_entity,
            avg_relevance: avg_relevance,
            avg_coherence: avg_coherence,
            avg_format_score: avg_format,
            scenario_score,
        }
    }

    fn build_system_message(&self, language: TestLanguage) -> Message {
        let content = match language {
            TestLanguage::English => format!(
                "You are NeoTalk, a smart home AI assistant. You help users control and monitor their smart home devices.\n\n\
                DEVICE ENVIRONMENT:\n{}\n\n\
                RESPONSE FORMAT TEMPLATES:\n\
                ═══════════════════════════════════════════════════════════════\n\
                1. Device Status Query:\n\
                \"The [device_name] in [room] is currently [status].\"\n\
                \n\
                2. Device List Query:\n\
                \"[Room] has the following devices: [device1], [device2], [device3].\"\n\
                \n\
                3. Device Control Confirmation:\n\
                \"✓ [action] [device_name] in [room]. Status: [new_status].\"\n\
                \n\
                4. Batch Control:\n\
                \"✓ [action] [count] [device_type] devices: [room1], [room2], [room3].\"\n\
                \n\
                5. Conditional Query:\n\
                \"[count] [device_type] found: [list of matching devices].\"\n\
                ═══════════════════════════════════════════════════════════════\n\
                \n\
                INSTRUCTIONS:\n\
                - Be concise and direct in your responses\n\
                - Use the format templates above for consistent responses\n\
                - When asked about device status, provide clear status information\n\
                - When asked to control devices, confirm the action taken with ✓\n\
                - Maintain context of the conversation (use 'it', 'the device' for references)\n\
                - For batch operations, list affected rooms/devices\n\
                - If a device is not found, suggest similar devices\n\
                - Always use English device names from the environment list",
                self.build_device_context(TestLanguage::English)
            ),
            TestLanguage::Chinese => format!(
                "你是 NeoTalk 智能助手。你帮助用户控制和监控智能家居设备。\n\n\
                设备环境:\n{}\n\n\
                响应格式模板:\n\
                ═══════════════════════════════════════════════════════════════\n\
                1. 设备状态查询:\n\
                「[房间]的[设备名称]当前状态：[状态]」\n\
                \n\
                2. 设备列表查询:\n\
                「[房间]有以下设备：[设备1]、[设备2]、[设备3]」\n\
                \n\
                3. 设备控制确认:\n\
                「✓ 已[操作][房间]的[设备名称]。当前状态：[新状态]」\n\
                \n\
                4. 批量控制:\n\
                「✓ 已[操作][数量]个[设备类型]：[房间1]、[房间2]、[房间3]」\n\
                \n\
                5. 条件查询:\n\
                「找到[数量]个[设备类型]：[匹配设备列表]」\n\
                ═══════════════════════════════════════════════════════════════\n\
                \n\
                指令:\n\
                - 回答要简洁直接\n\
                - 使用上方格式模板确保响应一致\n\
                - 被问及设备状态时，提供清晰的状态信息\n\
                - 被要求控制设备时，用 ✓ 确认执行的操作\n\
                - 保持对话上下文（用「它」指代之前提到的设备）\n\
                - 批量操作时，列出受影响的房间/设备\n\
                - 如果找不到设备，建议相似的设备",
                self.build_device_context(TestLanguage::Chinese)
            ),
        };

        Message {
            role: MessageRole::System,
            content: Content::Text(content),
            timestamp: None,
        }
    }

    fn build_device_context(&self, language: TestLanguage) -> String {
        let mut context = String::new();

        match language {
            TestLanguage::Chinese => {
                context.push_str(&format!("总设备数: {}\n\n", self.environment.devices.len()));
                context.push_str("按房间分组的设备:\n");

                for (room, devices) in &self.environment.devices_by_room {
                    context.push_str(&format!("- {}: ", room.cn_name()));
                    for (i, device) in devices.iter().take(5).enumerate() {
                        if i > 0 { context.push_str(", "); }
                        context.push_str(&device.name);
                    }
                    if devices.len() > 5 {
                        context.push_str(&format!(" 等{}个", devices.len()));
                    }
                    context.push('\n');
                }
            }
            TestLanguage::English => {
                context.push_str(&format!("Total Devices: {}\n\n", self.environment.devices.len()));
                context.push_str("Devices by Room:\n");

                for (room, devices) in &self.environment.devices_by_room {
                    context.push_str(&format!("- {}: ", room.name()));
                    for (i, device) in devices.iter().take(5).enumerate() {
                        if i > 0 { context.push_str(", "); }
                        context.push_str(&device.name);
                    }
                    if devices.len() > 5 {
                        context.push_str(&format!(" ... ({} devices)", devices.len()));
                    }
                    context.push('\n');
                }
            }
        }

        context
    }

    fn build_full_prompt(&self, messages: &[Message], device_context: &str, language: TestLanguage) -> String {
        let mut prompt = String::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    prompt.push_str("[SYSTEM]\n");
                    // 只在第一次包含系统消息
                    if let Content::Text(text) = &msg.content {
                        prompt.push_str(text);
                    }
                }
                MessageRole::User => {
                    prompt.push_str("\n[USER]\n");
                    if let Content::Text(text) = &msg.content {
                        prompt.push_str(text);
                    }
                }
                MessageRole::Assistant => {
                    prompt.push_str("\n[ASSISTANT]\n");
                    if let Content::Text(text) = &msg.content {
                        prompt.push_str(text);
                    }
                }
                _ => {}
            }
        }

        prompt
    }

    async fn send_prompt(&self, prompt: &str) -> String {
        let llm_input = LlmInput {
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: Content::Text("Continue the conversation.".to_string()),
                    timestamp: None,
                },
                Message {
                    role: MessageRole::User,
                    content: Content::Text(prompt.to_string()),
                    timestamp: None,
                },
            ],
            params: GenerationParams {
                max_tokens: Some(500),
                temperature: Some(0.7),
                ..Default::default()
            },
            model: Some(self.model_name.clone()),
            stream: false,
            tools: None,
        };

        match tokio::time::timeout(
            Duration::from_secs(self.timeout_secs),
            self.llm.generate(llm_input)
        ).await {
            Ok(Ok(output)) => output.text,
            Ok(Err(_)) => String::new(),
            Err(_) => String::new(),
        }
    }

    fn evaluate_turn(
        &self,
        turn: &ConversationTurn,
        response: &str,
        turn_number: usize,
        language: TestLanguage,
        response_time: u128,
    ) -> TurnEvaluation {
        // 意图识别评估
        let intent_score = Self::evaluate_intent_recognition(turn, response, language);

        // 实体提取评估
        let entity_score = Self::evaluate_entity_extraction(turn, response, language);

        // 回答相关性评估
        let relevance_score = Self::evaluate_relevance(turn, response, language);

        // 上下文连贯性评估
        let coherence_score = if turn_number > 1 {
            Self::evaluate_coherence(response, language)
        } else {
            100.0
        };

        // 格式合规性评估
        let format_score = Self::check_format_compliance(response, language);

        TurnEvaluation {
            turn_number,
            user_message: turn.user_message.clone(),
            agent_response: response.chars().take(200).collect::<String>(),
            response_time_ms: response_time,
            intent_recognition_score: intent_score,
            entity_extraction_score: entity_score,
            relevance_score: relevance_score,
            coherence_score: coherence_score,
            format_score,
        }
    }

    fn evaluate_intent_recognition(turn: &ConversationTurn, response: &str, language: TestLanguage) -> f64 {
        let mut score = 50.0; // 基础分

        let response_lower = response.to_lowercase();

        for intent in &turn.expected_intents {
            match intent {
                Intent::QueryDeviceStatus => {
                    if response_lower.contains("status") || response_lower.contains("状态") {
                        score += 25.0;
                    }
                    if response_lower.contains("?") || response_lower.contains("是") {
                        score += 12.5;
                    }
                }
                Intent::QueryDeviceList => {
                    if response_lower.contains("device") || response_lower.contains("设备") {
                        score += 25.0;
                    }
                    if response_lower.contains("list") || response_lower.contains("列表") ||
                       response.chars().filter(|c| *c == '、' || *c == ',').count() > 1 {
                        score += 25.0;
                    }
                }
                Intent::ControlDevice => {
                    if response_lower.contains("turn") || response_lower.contains("打开") ||
                       response_lower.contains("close") || response_lower.contains("关闭") {
                        score += 25.0;
                    }
                    if response_lower.contains("done") || response_lower.contains("完成") ||
                       response_lower.contains("ok") || response_lower.contains("好的") {
                        score += 25.0;
                    }
                }
                Intent::QueryRoomDevices => {
                    if response_lower.contains("room") || response_lower.contains("房间") {
                        score += 25.0;
                    }
                    if response.chars().filter(|c| *c == '、' || *c == ',').count() >= 2 {
                        score += 25.0;
                    }
                }
                Intent::BatchControl => {
                    if response_lower.contains("all") || response_lower.contains("所有") {
                        score += 25.0;
                    }
                    if response_lower.contains("done") || response_lower.contains("完成") {
                        score += 25.0;
                    }
                }
                Intent::ConditionalQuery => {
                    if response_lower.contains("temperature") || response_lower.contains("温度") ||
                       response_lower.contains("°") {
                        score += 25.0;
                    }
                    if response_lower.contains(">") || response_lower.contains("exceed") ||
                       response_lower.contains("超过") {
                        score += 25.0;
                    }
                }
                Intent::SceneActivation => {
                    if response_lower.contains("scene") || response_lower.contains("场景") {
                        score += 50.0;
                    }
                }
                Intent::QueryTypeDevices => {
                    if response_lower.contains("type") || response_lower.contains("类型") {
                        score += 25.0;
                    }
                }
            }
        }

        (score as f64).min(100.0)
    }

    fn evaluate_entity_extraction(turn: &ConversationTurn, response: &str, language: TestLanguage) -> f64 {
        if turn.expected_entities.is_empty() {
            return 100.0;
        }

        let mut correct = 0;
        let response_lower = response.to_lowercase();

        for entity in &turn.expected_entities {
            match entity.entity_type.as_str() {
                "room" => {
                    if response_lower.contains(&entity.value.to_lowercase()) ||
                       response_lower.contains(&Self::translate_room(&entity.value, language)) {
                        correct += 1;
                    }
                }
                "device_type" => {
                    if response_lower.contains(&entity.value.to_lowercase()) ||
                       response_lower.contains(&Self::translate_device_type(&entity.value, language)) {
                        correct += 1;
                    }
                }
                "action" => {
                    if response_lower.contains(&entity.value.to_lowercase()) {
                        correct += 1;
                    }
                }
                "scope" => {
                    if response_lower.contains(&entity.value.to_lowercase()) ||
                       response_lower.contains("all") || response_lower.contains("所有") {
                        correct += 1;
                    }
                }
                "reference" => {
                    if entity.confidence < 0.8 {
                        // 代词引用，降低评分标准
                        if response_lower.contains("it") || response_lower.contains("那个") ||
                           response_lower.contains("这个") || response_lower.contains("该") {
                            correct += 1;
                        }
                    }
                }
                _ => {
                    if response_lower.contains(&entity.value.to_lowercase()) {
                        correct += 1;
                    }
                }
            }
        }

        (correct as f64 / turn.expected_entities.len() as f64) * 100.0
    }

    fn translate_room(room: &str, language: TestLanguage) -> String {
        match language {
            TestLanguage::English => {
                match room {
                    "客厅" => "living room".to_string(),
                    "卧室" => "bedroom".to_string(),
                    "厨房" => "kitchen".to_string(),
                    "浴室" => "bathroom".to_string(),
                    "书房" => "study".to_string(),
                    "餐厅" => "dining room".to_string(),
                    _ => room.to_string(),
                }
            }
            TestLanguage::Chinese => {
                match room {
                    "living room" => "客厅".to_string(),
                    "bedroom" => "卧室".to_string(),
                    "kitchen" => "厨房".to_string(),
                    "bathroom" => "浴室".to_string(),
                    "study" => "书房".to_string(),
                    "dining room" => "餐厅".to_string(),
                    _ => room.to_string(),
                }
            }
        }
    }

    fn translate_device_type(device_type: &str, language: TestLanguage) -> String {
        match language {
            TestLanguage::English => {
                match device_type {
                    "灯光" => "light".to_string(),
                    "传感器" => "sensor".to_string(),
                    "开关" => "switch".to_string(),
                    _ => device_type.to_string(),
                }
            }
            TestLanguage::Chinese => {
                match device_type {
                    "light" => "灯光".to_string(),
                    "sensor" => "传感器".to_string(),
                    "switch" => "开关".to_string(),
                    _ => device_type.to_string(),
                }
            }
        }
    }

    fn evaluate_relevance(turn: &ConversationTurn, response: &str, language: TestLanguage) -> f64 {
        if response.is_empty() {
            return 0.0;
        }

        let mut score = 0.0;
        let user_lower = turn.user_message.to_lowercase();
        let response_lower = response.to_lowercase();

        // 1. 检查意图匹配 (40分)
        let intent_score = Self::check_intent_match(&turn.expected_intents, &response_lower, language);
        score += intent_score * 0.4;

        // 2. 检查实体匹配 (30分)
        let entity_score = Self::check_entity_match(&turn.expected_entities, &response_lower, language);
        score += entity_score * 0.3;

        // 3. 检查回答完整性 (20分)
        let completeness_score = Self::check_answer_completeness(&user_lower, &response_lower, language);
        score += completeness_score * 0.2;

        // 4. 检查是否有拒绝回答 (扣分项)
        let refusal_penalty = Self::check_refusal_penalty(&response_lower, language);
        score -= refusal_penalty * 0.1;

        // 5. 检查响应格式合规性 (加分项)
        let format_bonus = Self::check_format_compliance(&response, language);
        score += format_bonus * 0.1;

        score.max(0.0).min(100.0)
    }

    fn check_intent_match(intents: &[Intent], response: &str, language: TestLanguage) -> f64 {
        if intents.is_empty() {
            return 100.0;
        }

        let mut matched = 0;
        for intent in intents {
            match intent {
                Intent::QueryDeviceStatus => {
                    if response.contains("status") || response.contains("状态") ||
                       response.contains("currently") || response.contains("当前") {
                        matched += 1;
                    }
                }
                Intent::QueryDeviceList => {
                    if response.contains("following") || response.contains("以下") ||
                       response.contains(":") || response.contains("：") {
                        matched += 1;
                    }
                }
                Intent::ControlDevice => {
                    if response.contains("✓") || response.contains("已") ||
                       response.contains("turn") || response.contains("操作") {
                        matched += 1;
                    }
                }
                Intent::QueryRoomDevices => {
                    if response.chars().filter(|c| *c == ',' || *c == '、' || *c == ' ').count() > 2 {
                        matched += 1;
                    }
                }
                Intent::BatchControl => {
                    if response.contains("✓") && (response.contains("all") || response.contains("所有") ||
                       response.chars().filter(|c| *c == ',').count() > 1) {
                        matched += 1;
                    }
                }
                Intent::ConditionalQuery => {
                    if response.contains("found") || response.contains("找到") ||
                       response.contains("matching") || response.contains("匹配") {
                        matched += 1;
                    }
                }
                Intent::SceneActivation => {
                    if response.contains("scene") || response.contains("场景") ||
                       response.contains("activated") || response.contains("已激活") {
                        matched += 1;
                    }
                }
                Intent::QueryTypeDevices => {
                    if !response.is_empty() {
                        matched += 1;
                    }
                }
            }
        }

        (matched as f64 / intents.len() as f64) * 100.0
    }

    fn check_entity_match(entities: &[Entity], response: &str, language: TestLanguage) -> f64 {
        if entities.is_empty() {
            return 100.0;
        }

        let mut matched = 0;
        for entity in entities {
            let entity_lower = entity.value.to_lowercase();
            if response.contains(&entity_lower) {
                matched += 1;
            } else {
                // 检查翻译
                match entity.entity_type.as_str() {
                    "room" => {
                        let translated = Self::translate_room(&entity.value, language);
                        if response.contains(&translated.to_lowercase()) {
                            matched += 1;
                        }
                    }
                    "device_type" => {
                        let translated = Self::translate_device_type(&entity.value, language);
                        if response.contains(&translated.to_lowercase()) {
                            matched += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        (matched as f64 / entities.len() as f64) * 100.0
    }

    fn check_answer_completeness(user: &str, response: &str, language: TestLanguage) -> f64 {
        let mut score = 50.0;

        // 检查是否包含问号的问题被回答
        if user.contains('?') || user.contains('？') {
            if !response.is_empty() && response.len() > 10 {
                score += 30.0;
            }
        }

        // 检查是否包含具体信息
        let has_specific_info = response.contains("is") ||
                              response.contains("是") ||
                              response.contains("状态") ||
                              response.contains("✓");
        if has_specific_info {
            score += 20.0;
        }

        score
    }

    fn check_refusal_penalty(response: &str, language: TestLanguage) -> f64 {
        let refusal_keywords = match language {
            TestLanguage::Chinese => &["不知道", "无法", "抱歉", "不能", "无法确定"][..],
            TestLanguage::English => &["don't know", "cannot", "sorry", "unable", "not sure", "i don't"][..],
        };

        let response_lower = response.to_lowercase();
        let mut penalty = 0.0;
        for kw in refusal_keywords {
            if response_lower.contains(&kw.to_lowercase()) {
                penalty += 20.0;
                break;
            }
        }

        (penalty as f64).min(100.0)
    }

    fn check_format_compliance(response: &str, language: TestLanguage) -> f64 {
        let mut score = 0.0;

        // 检查是否使用确认标记 ✓
        if response.contains('✓') {
            score += 30.0;
        }

        // 检查响应结构
        if response.contains(':') || response.contains('：') {
            score += 20.0;
        }

        // 检查是否简洁 (不过于冗长)
        let response_len = response.chars().count();
        if response_len >= 20 && response_len <= 150 {
            score += 30.0;
        }

        // 检查是否有清晰的结构
        if response.lines().count() >= 1 {
            score += 20.0;
        }

        score
    }

    fn evaluate_coherence(response: &str, language: TestLanguage) -> f64 {
        if response.is_empty() {
            return 0.0;
        }

        let mut score = 50.0;

        // 检查是否有上下文引用词
        let context_refs = match language {
            TestLanguage::Chinese => &["它", "那个", "该", "之前", "上面"][..],
            TestLanguage::English => &["it", "that", "the", "previous", "above", "its"][..],
        };

        let response_lower = response.to_lowercase();
        let has_context_ref = context_refs.iter().any(|&kw| response_lower.contains(kw));

        if has_context_ref {
            score += 25.0;
        }

        // 检查响应长度是否合理 (太短可能缺少上下文)
        let response_len = response.chars().count();
        if response_len >= 10 && response_len <= 200 {
            score += 25.0;
        }

        (score as f64).min(100.0)
    }

    fn print_scenario_result(&self, result: &ScenarioEvaluationResult) {
        println!("\n   📊 场景评估结果:");
        println!("   ─────────────────────────────────────────────────────────");
        println!("   意图识别: {:.1}%", result.avg_intent_recognition);
        println!("   实体提取: {:.1}%", result.avg_entity_extraction);
        println!("   回答相关性: {:.1}%", result.avg_relevance);
        println!("   上下文连贯: {:.1}%", result.avg_coherence);
        println!("   格式合规: {:.1}%", result.avg_format_score);
        println!("   ─────────────────────────────────────────────────────────");
        println!("   场景得分: {:.1}/100", result.scenario_score);
    }

    fn generate_final_report(&self, results: Vec<ScenarioEvaluationResult>) -> ConversationEvaluationReport {
        let total_scenarios = results.len();
        let avg_score = results.iter().map(|r| r.scenario_score).sum::<f64>() / total_scenarios.max(1) as f64;

        let chinese_results: Vec<_> = results.iter().filter(|r| r.language == TestLanguage::Chinese).collect();
        let english_results: Vec<_> = results.iter().filter(|r| r.language == TestLanguage::English).collect();

        let chinese_avg = chinese_results.iter().map(|r| r.scenario_score).sum::<f64>() / chinese_results.len().max(1) as f64;
        let english_avg = english_results.iter().map(|r| r.scenario_score).sum::<f64>() / english_results.len().max(1) as f64;

        ConversationEvaluationReport {
            model_name: self.model_name.clone(),
            device_count: self.environment.devices.len(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            total_scenarios,
            results,
            overall_score: avg_score,
            chinese_score: chinese_avg,
            english_score: english_avg,
            language_diff: english_avg - chinese_avg,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnEvaluation {
    pub turn_number: usize,
    pub user_message: String,
    pub agent_response: String,
    pub response_time_ms: u128,
    pub intent_recognition_score: f64,
    pub entity_extraction_score: f64,
    pub relevance_score: f64,
    pub coherence_score: f64,
    pub format_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioEvaluationResult {
    pub scenario_name: String,
    pub language: TestLanguage,
    pub turn_results: Vec<TurnEvaluation>,
    pub total_time_secs: u64,
    pub avg_intent_recognition: f64,
    pub avg_entity_extraction: f64,
    pub avg_relevance: f64,
    pub avg_coherence: f64,
    pub avg_format_score: f64,
    pub scenario_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEvaluationReport {
    pub model_name: String,
    pub device_count: usize,
    pub timestamp: i64,
    pub total_scenarios: usize,
    pub results: Vec<ScenarioEvaluationResult>,
    pub overall_score: f64,
    pub chinese_score: f64,
    pub english_score: f64,
    pub language_diff: f64,
}

pub fn print_conversation_report(report: &ConversationEvaluationReport) {
    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   设备模拟与Agent对话质量评估报告                                        ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    println!("\n📊 测试概况:");
    println!("────────────────────────────────────────────────────────────────");
    println!("模型: {}", report.model_name);
    println!("设备数量: {}", report.device_count);
    println!("测试场景数: {}", report.total_scenarios);
    println!("综合得分: {:.1}/100", report.overall_score);

    println!("\n🌍 中英文对比:");
    println!("────────────────────────────────────────────────────────────────");
    println!("中文得分: {:.1}/100", report.chinese_score);
    println!("英文得分: {:.1}/100", report.english_score);
    println!("差异: {:+.1} {}", report.language_diff,
        if report.language_diff > 0.0 { "(英文更好)" }
        else if report.language_diff < 0.0 { "(中文更好)" }
        else { "(持平)" });

    println!("\n📋 场景得分详情:");
    println!("────────────────────────────────────────────────────────────────");
    println!("{:<30} | {:>10} | {:>10} | {:>10} | {:>10} | {:>10} | {:>10}",
        "场景", "意图识别", "实体提取", "相关性", "连贯性", "格式", "综合分");
    println!("────────────────────────────────────────────────────────────────");

    for result in &report.results {
        let lang_tag = match result.language {
            TestLanguage::Chinese => "🇨🇳",
            TestLanguage::English => "🇺🇸",
        };
        println!("{:<30} | {:>9.1}% | {:>9.1}% | {:>9.1}% | {:>9.1}% | {:>9.1}% | {:>9.1}",
            format!("{} {}", lang_tag, result.scenario_name),
            result.avg_intent_recognition,
            result.avg_entity_extraction,
            result.avg_relevance,
            result.avg_coherence,
            result.avg_format_score,
            result.scenario_score
        );
    }
}

// ============================================================================
// 测试入口
// ============================================================================

#[tokio::test]
async fn test_device_conversation_small() {
    let model = "qwen3:1.7b";
    let device_count = 50;

    match AgentConversationEvaluator::new(model, device_count) {
        Ok(evaluator) => {
            let report = evaluator.evaluate_all_scenarios().await;
            print_conversation_report(&report);
        }
        Err(e) => {
            println!("⚠️  无法创建评估器: {}", e);
        }
    }
}

#[tokio::test]
async fn test_device_conversation_large() {
    let model = "qwen3:1.7b";
    let device_count = 100;

    match AgentConversationEvaluator::new(model, device_count) {
        Ok(evaluator) => {
            let report = evaluator.evaluate_all_scenarios().await;
            print_conversation_report(&report);
        }
        Err(e) => {
            println!("⚠️  无法创建评估器: {}", e);
        }
    }
}
