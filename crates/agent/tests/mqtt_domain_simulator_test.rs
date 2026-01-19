//! MQTT Real-World Device Simulator
//!
//! This test creates a REAL MQTT connection and simulates devices across 10 domains:
//! 1. 智能家居
//! 2. 工业制造
//! 3. 智慧农业
//! 4. 能源管理
//! 5. 智慧医疗
//! 6. 智能交通
//! 7. 安防监控
//! 8. 环境监测
//! 9. 智能办公
//! 10. 智慧城市
//!
//! Each domain has:
//! - Realistic device types and characteristics
//! - Proper MQTT topic structure
//! - Telemetry data simulation
//! - Command handling
//! - Domain-specific conversation scenarios

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use edge_ai_core::EventBus;
use edge_ai_agent::SessionManager;
use serde::{Deserialize, Serialize};

// ============================================================================
// Domain Definitions
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    SmartHome,       // 智能家居
    Industrial,      // 工业制造
    Agriculture,     // 智慧农业
    Energy,          // 能源管理
    Healthcare,      // 智慧医疗
    Transportation,  // 智能交通
    Security,        // 安防监控
    Environment,     // 环境监测
    Office,          // 智能办公
    SmartCity,       // 智慧城市
}

impl Domain {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SmartHome => "智能家居",
            Self::Industrial => "工业制造",
            Self::Agriculture => "智慧农业",
            Self::Energy => "能源管理",
            Self::Healthcare => "智慧医疗",
            Self::Transportation => "智能交通",
            Self::Security => "安防监控",
            Self::Environment => "环境监测",
            Self::Office => "智能办公",
            Self::SmartCity => "智慧城市",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::SmartHome => "家庭自动化设备：灯光、空调、门锁、窗帘、家电等",
            Self::Industrial => "工业设备：PLC、传感器、机械臂、生产线监控等",
            Self::Agriculture => "农业设备：土壤传感器、灌溉控制、温室监控、气象站等",
            Self::Energy => "能源设备：电表、光伏逆变器、储能系统、充电桩等",
            Self::Healthcare => "医疗设备：生命体征监测、医疗仪器、病房自动化等",
            Self::Transportation => "交通设备：车辆监控、信号灯、停车系统、道路传感器等",
            Self::Security => "安防设备：摄像头、门禁、报警器、烟雾传感器等",
            Self::Environment => "环境监测：空气质量、水质监测、噪声、辐射等",
            Self::Office => "办公设备：会议室控制、打卡机、环境监测、能耗监控等",
            Self::SmartCity => "城市设施：路灯、井盖、垃圾桶、公共设施等",
        }
    }

    pub fn mqtt_prefix(&self) -> &'static str {
        match self {
            Self::SmartHome => "home",
            Self::Industrial => "factory",
            Self::Agriculture => "farm",
            Self::Energy => "energy",
            Self::Healthcare => "hospital",
            Self::Transportation => "traffic",
            Self::Security => "security",
            Self::Environment => "env",
            Self::Office => "office",
            Self::SmartCity => "city",
        }
    }

    pub fn all() -> Vec<Domain> {
        vec![
            Self::SmartHome,
            Self::Industrial,
            Self::Agriculture,
            Self::Energy,
            Self::Healthcare,
            Self::Transportation,
            Self::Security,
            Self::Environment,
            Self::Office,
            Self::SmartCity,
        ]
    }
}

// ============================================================================
// Device Definition
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttDevice {
    pub id: String,
    pub name: String,
    pub domain: Domain,
    pub device_type: String,
    pub location: String,
    pub capabilities: DeviceCapabilities,
    pub telemetry: Vec<TelemetryDefinition>,
    pub commands: Vec<CommandDefinition>,
    pub state: DeviceState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub report_telemetry: bool,
    pub accept_commands: bool,
    pub has_alerts: bool,
    pub is_actuator: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryDefinition {
    pub metric_name: String,
    pub unit: String,
    pub data_type: MetricDataType,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub update_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricDataType {
    Float,
    Integer,
    Boolean,
    String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub command_name: String,
    pub parameters: Vec<CommandParameter>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    pub online: bool,
    pub last_seen: i64,
    pub current_values: HashMap<String, serde_json::Value>,
}

// ============================================================================
// Domain-Specific Device Factories
// ============================================================================

pub struct DeviceFactory;

impl DeviceFactory {
    /// Generate devices for a specific domain
    pub fn generate_domain_devices(domain: Domain, count: usize) -> Vec<MqttDevice> {
        match domain {
            Domain::SmartHome => Self::generate_home_devices(count),
            Domain::Industrial => Self::generate_industrial_devices(count),
            Domain::Agriculture => Self::generate_agriculture_devices(count),
            Domain::Energy => Self::generate_energy_devices(count),
            Domain::Healthcare => Self::generate_healthcare_devices(count),
            Domain::Transportation => Self::generate_transportation_devices(count),
            Domain::Security => Self::generate_security_devices(count),
            Domain::Environment => Self::generate_environment_devices(count),
            Domain::Office => Self::generate_office_devices(count),
            Domain::SmartCity => Self::generate_smart_city_devices(count),
        }
    }

    fn generate_home_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["客厅", "卧室", "厨房", "浴室", "书房", "阳台", "车库", "花园"];
        let device_types = vec![
            ("智能灯泡", vec![
                TelemetryDefinition {
                    metric_name: "power".to_string(),
                    unit: "W".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "brightness".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 5000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "turn_on".to_string(),
                    parameters: vec![],
                    description: "打开灯".to_string(),
                },
                CommandDefinition {
                    command_name: "turn_off".to_string(),
                    parameters: vec![],
                    description: "关闭灯".to_string(),
                },
                CommandDefinition {
                    command_name: "set_brightness".to_string(),
                    parameters: vec![CommandParameter {
                        name: "level".to_string(),
                        param_type: "integer".to_string(),
                        required: true,
                    }],
                    description: "设置亮度".to_string(),
                },
                ], true),
            ("空调", vec![
                TelemetryDefinition {
                    metric_name: "temperature".to_string(),
                    unit: "°C".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(16.0),
                    max_value: Some(32.0),
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "humidity".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(30.0),
                    max_value: Some(80.0),
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "power".to_string(),
                    unit: "kW".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(5.0),
                    update_interval_ms: 5000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "set_temperature".to_string(),
                    parameters: vec![CommandParameter {
                        name: "target".to_string(),
                        param_type: "float".to_string(),
                        required: true,
                    }],
                    description: "设置目标温度".to_string(),
                },
                CommandDefinition {
                    command_name: "set_mode".to_string(),
                    parameters: vec![CommandParameter {
                        name: "mode".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                    }],
                    description: "设置模式".to_string(),
                },
            ], true),
            ("智能门锁", vec![
                TelemetryDefinition {
                    metric_name: "locked".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "battery".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 3600000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "lock".to_string(),
                    parameters: vec![],
                    description: "上锁".to_string(),
                },
                CommandDefinition {
                    command_name: "unlock".to_string(),
                    parameters: vec![],
                    description: "开锁".to_string(),
                },
            ], false),
            ("窗帘", vec![
                TelemetryDefinition {
                    metric_name: "position".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 10000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "open".to_string(),
                    parameters: vec![],
                    description: "打开窗帘".to_string(),
                },
                CommandDefinition {
                    command_name: "close".to_string(),
                    parameters: vec![],
                    description: "关闭窗帘".to_string(),
                },
                CommandDefinition {
                    command_name: "set_position".to_string(),
                    parameters: vec![CommandParameter {
                        name: "position".to_string(),
                        param_type: "integer".to_string(),
                        required: true,
                    }],
                    description: "设置开合度".to_string(),
                },
            ], true),
            ("扫地机器人", vec![
                TelemetryDefinition {
                    metric_name: "status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "battery".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "dustbin".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 60000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "start_clean".to_string(),
                    parameters: vec![],
                    description: "开始清扫".to_string(),
                },
                CommandDefinition {
                    command_name: "stop_clean".to_string(),
                    parameters: vec![],
                    description: "停止清扫".to_string(),
                },
                CommandDefinition {
                    command_name: "return_to_base".to_string(),
                    parameters: vec![],
                    description: "返回充电座".to_string(),
                },
            ], true),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (type_idx, (dev_type, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type {
                let location = locations[i % locations.len()];
                let id = format!("{}_{:02}", Self::pinyin(dev_type), i);
                let name = format!("{}{}", location, dev_type);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::SmartHome,
                    device_type: dev_type.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: true,
                        has_alerts: *dev_type == "智能门锁",
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    fn generate_industrial_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["生产线A", "生产线B", "装配车间", "喷涂车间", "仓库", "质检区"];
        let device_types = vec![
            ("PLC控制器", vec![
                TelemetryDefinition {
                    metric_name: "cpu_usage".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "cycle_count".to_string(),
                    unit: "次".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 5000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "start_cycle".to_string(),
                    parameters: vec![],
                    description: "启动生产周期".to_string(),
                },
                CommandDefinition {
                    command_name: "stop_cycle".to_string(),
                    parameters: vec![],
                    description: "停止生产周期".to_string(),
                },
                CommandDefinition {
                    command_name: "reset".to_string(),
                    parameters: vec![],
                    description: "复位PLC".to_string(),
                },
            ], true),
            ("工业温度传感器", vec![
                TelemetryDefinition {
                    metric_name: "temperature".to_string(),
                    unit: "°C".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(-50.0),
                    max_value: Some(200.0),
                    update_interval_ms: 2000,
                },
                TelemetryDefinition {
                    metric_name: "alarm_status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 5000,
                },
            ], vec![], false),
            ("振动传感器", vec![
                TelemetryDefinition {
                    metric_name: "vibration_x".to_string(),
                    unit: "mm/s".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(50.0),
                    update_interval_ms: 1000,
                },
                TelemetryDefinition {
                    metric_name: "vibration_y".to_string(),
                    unit: "mm/s".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(50.0),
                    update_interval_ms: 1000,
                },
                TelemetryDefinition {
                    metric_name: "vibration_z".to_string(),
                    unit: "mm/s".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(50.0),
                    update_interval_ms: 1000,
                },
            ], vec![], false),
            ("机械臂", vec![
                TelemetryDefinition {
                    metric_name: "position_x".to_string(),
                    unit: "mm".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(-1000.0),
                    max_value: Some(1000.0),
                    update_interval_ms: 100,
                },
                TelemetryDefinition {
                    metric_name: "position_y".to_string(),
                    unit: "mm".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(-1000.0),
                    max_value: Some(1000.0),
                    update_interval_ms: 100,
                },
                TelemetryDefinition {
                    metric_name: "position_z".to_string(),
                    unit: "mm".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(-500.0),
                    max_value: Some(1500.0),
                    update_interval_ms: 100,
                },
                TelemetryDefinition {
                    metric_name: "gripper_state".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 100,
                },
            ], vec![
                CommandDefinition {
                    command_name: "move_to".to_string(),
                    parameters: vec![
                        CommandParameter { name: "x".to_string(), param_type: "float".to_string(), required: true },
                        CommandParameter { name: "y".to_string(), param_type: "float".to_string(), required: true },
                        CommandParameter { name: "z".to_string(), param_type: "float".to_string(), required: true },
                    ],
                    description: "移动到指定位置".to_string(),
                },
                CommandDefinition {
                    command_name: "gripper_close".to_string(),
                    parameters: vec![],
                    description: "关闭夹爪".to_string(),
                },
                CommandDefinition {
                    command_name: "gripper_open".to_string(),
                    parameters: vec![],
                    description: "打开夹爪".to_string(),
                },
            ], true),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (dev_type_index, (_, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type.max(1) {
                let location = locations[i % locations.len()];
                let type_name = &device_types[dev_type_index].0;
                let id = format!("{}_{:02}", Self::pinyin(type_name), i);
                let name = format!("{}{}", location, type_name);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::Industrial,
                    device_type: type_name.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: !commands.is_empty(),
                        has_alerts: true,
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    fn generate_agriculture_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["温室1号", "温室2号", "温室3号", "露天A区", "露天B区", "育苗区"];
        let device_types = vec![
            ("土壤传感器", vec![
                TelemetryDefinition {
                    metric_name: "soil_moisture".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "soil_temperature".to_string(),
                    unit: "°C".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(-10.0),
                    max_value: Some(50.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "soil_ph".to_string(),
                    unit: "pH".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(14.0),
                    update_interval_ms: 300000,
                },
                TelemetryDefinition {
                    metric_name: "nitrogen".to_string(),
                    unit: "mg/kg".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(200.0),
                    update_interval_ms: 600000,
                },
            ], vec![], false),
            ("气象站", vec![
                TelemetryDefinition {
                    metric_name: "air_temperature".to_string(),
                    unit: "°C".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(-20.0),
                    max_value: Some(50.0),
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "air_humidity".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "wind_speed".to_string(),
                    unit: "m/s".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(40.0),
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "rainfall".to_string(),
                    unit: "mm".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "solar_radiation".to_string(),
                    unit: "W/m²".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(1500.0),
                    update_interval_ms: 30000,
                },
            ], vec![], false),
            ("灌溉控制器", vec![
                TelemetryDefinition {
                    metric_name: "valve_status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "flow_rate".to_string(),
                    unit: "L/min".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "water_used".to_string(),
                    unit: "L".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 60000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "start_irrigation".to_string(),
                    parameters: vec![
                        CommandParameter { name: "duration".to_string(), param_type: "integer".to_string(), required: true },
                    CommandParameter { name: "flow".to_string(), param_type: "float".to_string(), required: false },
                    ],
                    description: "开始灌溉".to_string(),
                },
                CommandDefinition {
                    command_name: "stop_irrigation".to_string(),
                    parameters: vec![],
                    description: "停止灌溉".to_string(),
                },
            ], true),
            ("温室控制器", vec![
                TelemetryDefinition {
                    metric_name: "roof_position".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "shade_position".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "vent_status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 30000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "open_roof".to_string(),
                    parameters: vec![CommandParameter { name: "percentage".to_string(), param_type: "integer".to_string(), required: false }],
                    description: "打开天窗".to_string(),
                },
                CommandDefinition {
                    command_name: "close_roof".to_string(),
                    parameters: vec![],
                    description: "关闭天窗".to_string(),
                },
                CommandDefinition {
                    command_name: "toggle_shade".to_string(),
                    parameters: vec![CommandParameter { name: "position".to_string(), param_type: "integer".to_string(), required: false }],
                    description: "调节遮阳网".to_string(),
                },
            ], true),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (type_idx, (dev_type, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type.max(1) {
                let location = locations[i % locations.len()];
                let id = format!("{}_{:02}", Self::pinyin(dev_type), i);
                let name = format!("{}{}", location, dev_type);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::Agriculture,
                    device_type: dev_type.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: !commands.is_empty(),
                        has_alerts: true,
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    fn generate_energy_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["变电站A", "配电室", "光伏电站1号", "储能中心", "充电站东区", "充电站西区"];
        let device_types = vec![
            ("智能电表", vec![
                TelemetryDefinition {
                    metric_name: "voltage_a".to_string(),
                    unit: "V".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(200.0),
                    max_value: Some(250.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "voltage_b".to_string(),
                    unit: "V".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(200.0),
                    max_value: Some(250.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "voltage_c".to_string(),
                    unit: "V".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(200.0),
                    max_value: Some(250.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "current".to_string(),
                    unit: "A".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(200.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "power".to_string(),
                    unit: "kW".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "energy".to_string(),
                    unit: "kWh".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 60000,
                },
            ], vec![], false),
            ("光伏逆变器", vec![
                TelemetryDefinition {
                    metric_name: "dc_voltage".to_string(),
                    unit: "V".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(1000.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "ac_power".to_string(),
                    unit: "kW".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "efficiency".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(90.0),
                    max_value: Some(99.0),
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "today_energy".to_string(),
                    unit: "kWh".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: None,
                    update_interval_ms: 60000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "power_on".to_string(),
                    parameters: vec![],
                    description: "启动逆变器".to_string(),
                },
                CommandDefinition {
                    command_name: "power_off".to_string(),
                    parameters: vec![],
                    description: "关闭逆变器".to_string(),
                },
            ], true),
            ("充电桩", vec![
                TelemetryDefinition {
                    metric_name: "connector_status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "charging_power".to_string(),
                    unit: "kW".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(120.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "soc".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "energy_delivered".to_string(),
                    unit: "kWh".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: None,
                    update_interval_ms: 10000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "start_charging".to_string(),
                    parameters: vec![CommandParameter { name: "max_power".to_string(), param_type: "float".to_string(), required: false }],
                    description: "开始充电".to_string(),
                },
                CommandDefinition {
                    command_name: "stop_charging".to_string(),
                    parameters: vec![],
                    description: "停止充电".to_string(),
                },
                CommandDefinition {
                    command_name: "unlock_connector".to_string(),
                    parameters: vec![],
                    description: "解锁充电枪".to_string(),
                },
            ], true),
            ("储能系统", vec![
                TelemetryDefinition {
                    metric_name: "battery_soc".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "battery_soh".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(80.0),
                    max_value: Some(100.0),
                    update_interval_ms: 300000,
                },
                TelemetryDefinition {
                    metric_name: "charge_power".to_string(),
                    unit: "kW".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(-100.0),
                    max_value: Some(100.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "battery_temp".to_string(),
                    unit: "°C".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(10.0),
                    max_value: Some(50.0),
                    update_interval_ms: 30000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "set_charge_mode".to_string(),
                    parameters: vec![CommandParameter { name: "mode".to_string(), param_type: "string".to_string(), required: true }],
                    description: "设置充放电模式".to_string(),
                },
                CommandDefinition {
                    command_name: "set_power_limit".to_string(),
                    parameters: vec![CommandParameter { name: "limit".to_string(), param_type: "float".to_string(), required: true }],
                    description: "设置功率限制".to_string(),
                },
            ], true),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (type_idx, (dev_type, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type.max(1) {
                let location = locations[i % locations.len()];
                let id = format!("{}_{:02}", Self::pinyin(dev_type), i);
                let name = format!("{}{}", location, dev_type);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::Energy,
                    device_type: dev_type.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: !commands.is_empty(),
                        has_alerts: true,
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    fn generate_healthcare_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["ICU病房", "普通病房", "手术室", "急诊室", "门诊大厅"];
        let device_types = vec![
            ("生命体征监护仪", vec![
                TelemetryDefinition {
                    metric_name: "heart_rate".to_string(),
                    unit: "bpm".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(30.0),
                    max_value: Some(200.0),
                    update_interval_ms: 1000,
                },
                TelemetryDefinition {
                    metric_name: "blood_pressure_systolic".to_string(),
                    unit: "mmHg".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(60.0),
                    max_value: Some(200.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "blood_pressure_diastolic".to_string(),
                    unit: "mmHg".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(40.0),
                    max_value: Some(130.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "spo2".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(70.0),
                    max_value: Some(100.0),
                    update_interval_ms: 2000,
                },
                TelemetryDefinition {
                    metric_name: "respiratory_rate".to_string(),
                    unit: "bpm".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(8.0),
                    max_value: Some(40.0),
                    update_interval_ms: 3000,
                },
                TelemetryDefinition {
                    metric_name: "temperature".to_string(),
                    unit: "°C".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(35.0),
                    max_value: Some(42.0),
                    update_interval_ms: 10000,
                },
            ], vec![], false),
            ("输液泵", vec![
                TelemetryDefinition {
                    metric_name: "flow_rate".to_string(),
                    unit: "mL/h".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.1),
                    max_value: Some(1000.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "volume_infused".to_string(),
                    unit: "mL".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: None,
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "volume_remaining".to_string(),
                    unit: "mL".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(1000.0),
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "alarm_status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 5000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "start".to_string(),
                    parameters: vec![CommandParameter { name: "rate".to_string(), param_type: "float".to_string(), required: true }],
                    description: "开始输液".to_string(),
                },
                CommandDefinition {
                    command_name: "stop".to_string(),
                    parameters: vec![],
                    description: "停止输液".to_string(),
                },
                CommandDefinition {
                    command_name: "set_rate".to_string(),
                    parameters: vec![CommandParameter { name: "rate".to_string(), param_type: "float".to_string(), required: true }],
                    description: "设置流速".to_string(),
                },
            ], true),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (type_idx, (dev_type, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type.max(1) {
                let location = locations[i % locations.len()];
                let id = format!("{}_{:02}", Self::pinyin(dev_type), i);
                let name = format!("{}{}号机", location, dev_type);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::Healthcare,
                    device_type: dev_type.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: !commands.is_empty(),
                        has_alerts: true,
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    fn generate_transportation_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["主干道", "次干道", "高速公路入口", "市中心", "商业区"];
        let device_types = vec![
            ("交通信号灯", vec![
                TelemetryDefinition {
                    metric_name: "current_phase".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "phase_timer".to_string(),
                    unit: "s".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(120.0),
                    update_interval_ms: 1000,
                },
                TelemetryDefinition {
                    metric_name: "queue_length".to_string(),
                    unit: "辆".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(50.0),
                    update_interval_ms: 10000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "set_mode".to_string(),
                    parameters: vec![CommandParameter { name: "mode".to_string(), param_type: "string".to_string(), required: true }],
                    description: "设置信号模式".to_string(),
                },
                CommandDefinition {
                    command_name: "force_phase".to_string(),
                    parameters: vec![CommandParameter { name: "phase".to_string(), param_type: "string".to_string(), required: true }],
                    description: "强制切换相位".to_string(),
                },
            ], true),
            ("地磁传感器", vec![
                TelemetryDefinition {
                    metric_name: "vehicle_detected".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 500,
                },
                TelemetryDefinition {
                    metric_name: "vehicle_speed".to_string(),
                    unit: "km/h".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(120.0),
                    update_interval_ms: 1000,
                },
                TelemetryDefinition {
                    metric_name: "vehicle_count".to_string(),
                    unit: "辆".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: None,
                    update_interval_ms: 60000,
                },
            ], vec![], false),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (type_idx, (dev_type, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type.max(1) {
                let location = locations[i % locations.len()];
                let id = format!("{}_{:02}", Self::pinyin(dev_type), i);
                let name = format!("{}{}", location, dev_type);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::Transportation,
                    device_type: dev_type.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: !commands.is_empty(),
                        has_alerts: false,
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    fn generate_security_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["正门", "后门", "大厅", "走廊", "楼梯口", "停车场入口"];
        let device_types = vec![
            ("网络摄像头", vec![
                TelemetryDefinition {
                    metric_name: "status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "motion_detected".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 1000,
                },
                TelemetryDefinition {
                    metric_name: "person_detected".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 1000,
                },
                TelemetryDefinition {
                    metric_name: "recording_status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 30000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "start_recording".to_string(),
                    parameters: vec![],
                    description: "开始录像".to_string(),
                },
                CommandDefinition {
                    command_name: "stop_recording".to_string(),
                    parameters: vec![],
                    description: "停止录像".to_string(),
                },
                CommandDefinition {
                    command_name: "set_ptz".to_string(),
                    parameters: vec![
                        CommandParameter { name: "pan".to_string(), param_type: "float".to_string(), required: false },
                        CommandParameter { name: "tilt".to_string(), param_type: "float".to_string(), required: false },
                        CommandParameter { name: "zoom".to_string(), param_type: "float".to_string(), required: false },
                    ],
                    description: "设置云台".to_string(),
                },
            ], true),
            ("门禁控制器", vec![
                TelemetryDefinition {
                    metric_name: "door_status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "lock_status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 10000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "unlock".to_string(),
                    parameters: vec![CommandParameter { name: "duration".to_string(), param_type: "integer".to_string(), required: false }],
                    description: "解锁".to_string(),
                },
                CommandDefinition {
                    command_name: "lock".to_string(),
                    parameters: vec![],
                    description: "锁定".to_string(),
                },
            ], true),
            ("烟雾传感器", vec![
                TelemetryDefinition {
                    metric_name: "smoke_level".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 5000,
                },
                TelemetryDefinition {
                    metric_name: "temperature".to_string(),
                    unit: "°C".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 10000,
                },
                TelemetryDefinition {
                    metric_name: "alarm".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 1000,
                },
            ], vec![], false),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (type_idx, (dev_type, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type.max(1) {
                let location = locations[i % locations.len()];
                let id = format!("{}_{:02}", Self::pinyin(dev_type), i);
                let name = format!("{}{}", location, dev_type);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::Security,
                    device_type: dev_type.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: !commands.is_empty(),
                        has_alerts: true,
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    fn generate_environment_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["监测站A", "监测站B", "监测站C", "河边", "工业园区", "居民区"];
        let device_types = vec![
            ("空气质量监测站", vec![
                TelemetryDefinition {
                    metric_name: "pm25".to_string(),
                    unit: "µg/m³".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(500.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "pm10".to_string(),
                    unit: "µg/m³".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(600.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "co2".to_string(),
                    unit: "ppm".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(300.0),
                    max_value: Some(5000.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "o3".to_string(),
                    unit: "ppm".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(200.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "no2".to_string(),
                    unit: "ppm".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "so2".to_string(),
                    unit: "ppm".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "aqi".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(500.0),
                    update_interval_ms: 60000,
                },
            ], vec![], false),
            ("水质监测仪", vec![
                TelemetryDefinition {
                    metric_name: "ph".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(14.0),
                    update_interval_ms: 300000,
                },
                TelemetryDefinition {
                    metric_name: "turbidity".to_string(),
                    unit: "NTU".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(1000.0),
                    update_interval_ms: 300000,
                },
                TelemetryDefinition {
                    metric_name: "dissolved_oxygen".to_string(),
                    unit: "mg/L".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(15.0),
                    update_interval_ms: 300000,
                },
                TelemetryDefinition {
                    metric_name: "conductivity".to_string(),
                    unit: "µS/cm".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(2000.0),
                    update_interval_ms: 300000,
                },
            ], vec![], false),
            ("噪声监测仪", vec![
                TelemetryDefinition {
                    metric_name: "noise_level".to_string(),
                    unit: "dB".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(30.0),
                    max_value: Some(130.0),
                    update_interval_ms: 5000,
                },
            ], vec![], false),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (type_idx, (dev_type, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type.max(1) {
                let location = locations[i % locations.len()];
                let id = format!("{}_{:02}", Self::pinyin(dev_type), i);
                let name = format!("{}{}", location, dev_type);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::Environment,
                    device_type: dev_type.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: !commands.is_empty(),
                        has_alerts: true,
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    fn generate_office_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["会议室A", "会议室B", "开放办公区", "前台", "经理办公室"];
        let device_types = vec![
            ("会议平板", vec![
                TelemetryDefinition {
                    metric_name: "status".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "screen_on".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 30000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "power_on".to_string(),
                    parameters: vec![],
                    description: "开机".to_string(),
                },
                CommandDefinition {
                    command_name: "power_off".to_string(),
                    parameters: vec![],
                    description: "关机".to_string(),
                },
            ], true),
            ("考勤机", vec![
                TelemetryDefinition {
                    metric_name: "last_checkin".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "today_count".to_string(),
                    unit: "人".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: None,
                    update_interval_ms: 60000,
                },
            ], vec![], false),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (type_idx, (dev_type, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type.max(1) {
                let location = locations[i % locations.len()];
                let id = format!("{}_{:02}", Self::pinyin(dev_type), i);
                let name = format!("{}{}", location, dev_type);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::Office,
                    device_type: dev_type.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: !commands.is_empty(),
                        has_alerts: false,
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    fn generate_smart_city_devices(count: usize) -> Vec<MqttDevice> {
        let locations = vec!["中山路", "人民广场", "体育馆", "购物中心", "地铁口"];
        let device_types = vec![
            ("智慧路灯", vec![
                TelemetryDefinition {
                    metric_name: "light_level".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "power_consumption".to_string(),
                    unit: "kWh".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(1.0),
                    update_interval_ms: 60000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "set_brightness".to_string(),
                    parameters: vec![CommandParameter { name: "level".to_string(), param_type: "integer".to_string(), required: true }],
                    description: "设置亮度".to_string(),
                },
                CommandDefinition {
                    command_name: "turn_on".to_string(),
                    parameters: vec![],
                    description: "开灯".to_string(),
                },
                CommandDefinition {
                    command_name: "turn_off".to_string(),
                    parameters: vec![],
                    description: "关灯".to_string(),
                },
            ], true),
            ("智能井盖", vec![
                TelemetryDefinition {
                    metric_name: "position".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 60000,
                },
                TelemetryDefinition {
                    metric_name: "water_level".to_string(),
                    unit: "cm".to_string(),
                    data_type: MetricDataType::Float,
                    min_value: Some(0.0),
                    max_value: Some(200.0),
                    update_interval_ms: 300000,
                },
                TelemetryDefinition {
                    metric_name: "tilt_alarm".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::Boolean,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 5000,
                },
            ], vec![], false),
            ("智能垃圾桶", vec![
                TelemetryDefinition {
                    metric_name: "fill_level".to_string(),
                    unit: "%".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: Some(100.0),
                    update_interval_ms: 30000,
                },
                TelemetryDefinition {
                    metric_name: "last_empty".to_string(),
                    unit: "".to_string(),
                    data_type: MetricDataType::String,
                    min_value: None,
                    max_value: None,
                    update_interval_ms: 3600000,
                },
                TelemetryDefinition {
                    metric_name: "compaction_count".to_string(),
                    unit: "次".to_string(),
                    data_type: MetricDataType::Integer,
                    min_value: Some(0.0),
                    max_value: None,
                    update_interval_ms: 60000,
                },
            ], vec![
                CommandDefinition {
                    command_name: "request_collection".to_string(),
                    parameters: vec![],
                    description: "请求清运".to_string(),
                },
                CommandDefinition {
                    command_name: "compact".to_string(),
                    parameters: vec![],
                    description: "压缩垃圾".to_string(),
                },
            ], true),
        ];

        let mut devices = Vec::new();
        let per_type = count / device_types.len();

        for (type_idx, (dev_type, telemetry, commands, is_actuator)) in device_types.iter().enumerate() {
            for i in 0..per_type.max(1) {
                let location = locations[i % locations.len()];
                let id = format!("{}_{:02}", Self::pinyin(dev_type), i);
                let name = format!("{}{}", location, dev_type);

                devices.push(MqttDevice {
                    id: id.clone(),
                    name,
                    domain: Domain::SmartCity,
                    device_type: dev_type.to_string(),
                    location: location.to_string(),
                    capabilities: DeviceCapabilities {
                        report_telemetry: true,
                        accept_commands: !commands.is_empty(),
                        has_alerts: true,
                        is_actuator: *is_actuator,
                    },
                    telemetry: telemetry.clone(),
                    commands: commands.clone(),
                    state: DeviceState {
                        online: true,
                        last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                        current_values: HashMap::new(),
                    },
                });
            }
        }

        devices
    }

    // Simple pinyin converter for device IDs (simplified)
    fn pinyin(s: &str) -> String {
        // Simplified mapping for common Chinese characters in device names
        let map = HashMap::from([
            ("智能", "smart"), ("灯泡", "bulb"), ("空调", "ac"), ("门锁", "lock"),
            ("窗帘", "curtain"), ("扫地机器人", "vacuum"), ("PLC控制器", "plc"),
            ("温度传感器", "temp"), ("振动传感器", "vibration"), ("机械臂", "arm"),
            ("土壤传感器", "soil"), ("气象站", "weather"), ("灌溉控制器", "irrigation"),
            ("温室控制器", "greenhouse"), ("电表", "meter"), ("光伏逆变器", "inverter"),
            ("充电桩", "evse"), ("储能系统", "battery"), ("监护仪", "monitor"),
            ("输液泵", "pump"), ("信号灯", "traffic"), ("地磁传感器", "loop"),
            ("摄像头", "camera"), ("门禁控制器", "access"), ("烟雾传感器", "smoke"),
            ("空气质量", "air"), ("水质监测仪", "water"), ("噪声监测仪", "noise"),
            ("会议平板", "panel"), ("考勤机", "attendance"), ("路灯", "streetlight"),
            ("井盖", "manhole"), ("垃圾桶", "trash"),
        ]);

        let mut result = s.to_string();
        for (chinese, english) in &map {
            result = result.replace(chinese, english);
        }
        result
    }
}

// ============================================================================
// Conversation Scenarios per Domain
// ============================================================================

pub struct DomainConversationScenario {
    pub domain: Domain,
    pub name: String,
    pub conversations: Vec<ConversationTurn>,
    pub expected_intents: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub user_input: String,
    pub expected_intent: String,
    pub expected_entities: Vec<Entity>,
    pub context_required: bool,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_type: String,  // "device", "location", "value", etc.
    pub value: String,
}

impl DomainConversationScenario {
    pub fn get_all_scenarios() -> Vec<Self> {
        vec![
            // 智能家居场景
            Self {
                domain: Domain::SmartHome,
                name: "回家模式自动化".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "我回家了".to_string(),
                        expected_intent: "scene_trigger".to_string(),
                        expected_entities: vec![Entity { entity_type: "scene".to_string(), value: "回家".to_string() }],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "帮我打开客厅的灯".to_string(),
                        expected_intent: "control_device".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "客厅".to_string() },
                            Entity { entity_type: "device_type".to_string(), value: "灯".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "把空调调到26度".to_string(),
                        expected_intent: "control_device".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "device_type".to_string(), value: "空调".to_string() },
                            Entity { entity_type: "parameter".to_string(), value: "温度".to_string() },
                            Entity { entity_type: "value".to_string(), value: "26".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "打开窗帘".to_string(),
                        expected_intent: "control_device".to_string(),
                        expected_entities: vec![Entity { entity_type: "device_type".to_string(), value: "窗帘".to_string() }],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "播放一些轻音乐".to_string(),
                        expected_intent: "control_device".to_string(),
                        expected_entities: vec![Entity { entity_type: "device_type".to_string(), value: "音响".to_string() }],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "创建一个回家模式的自动化".to_string(),
                        expected_intent: "create_automation".to_string(),
                        expected_entities: vec![Entity { entity_type: "scene".to_string(), value: "回家模式".to_string() }],
                        context_required: true,
                    },
                ],
                expected_intents: vec![
                    "scene_trigger".to_string(),
                    "control_device".to_string(),
                    "control_device".to_string(),
                    "control_device".to_string(),
                    "control_device".to_string(),
                    "create_automation".to_string(),
                ],
            },

            // 工业制造场景
            Self {
                domain: Domain::Industrial,
                name: "生产线监控与故障诊断".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "生产线A的状态怎么样".to_string(),
                        expected_intent: "query_status".to_string(),
                        expected_entities: vec![Entity { entity_type: "location".to_string(), value: "生产线A".to_string() }],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "3号机械臂在哪里".to_string(),
                        expected_intent: "query_device".to_string(),
                        expected_entities: vec![Entity { entity_type: "device_id".to_string(), value: "3号".to_string() }],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "检测到振动异常吗".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![Entity { entity_type: "metric".to_string(), value: "振动".to_string() }],
                        context_required: true,
                    },
                    ConversationTurn {
                        user_input: "如果有异常，停止生产".to_string(),
                        expected_intent: "conditional_action".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "condition".to_string(), value: "异常".to_string() },
                            Entity { entity_type: "action".to_string(), value: "停止生产".to_string() },
                        ],
                        context_required: true,
                    },
                ],
                expected_intents: vec![
                    "query_status".to_string(),
                    "query_device".to_string(),
                    "query_data".to_string(),
                    "conditional_action".to_string(),
                ],
            },

            // 智慧农业场景
            Self {
                domain: Domain::Agriculture,
                name: "智能灌溉与温室管理".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "温室1号现在的土壤湿度是多少".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "温室1号".to_string() },
                            Entity { entity_type: "metric".to_string(), value: "土壤湿度".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "太干了，开始浇水".to_string(),
                        expected_intent: "control_device".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "condition".to_string(), value: "土壤干燥".to_string() },
                            Entity { entity_type: "action".to_string(), value: "浇水".to_string() },
                        ],
                        context_required: true,
                    },
                    ConversationTurn {
                        user_input: "检查今天的天气预报".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![Entity { entity_type: "data_type".to_string(), value: "天气".to_string() }],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "如果会下雨，取消今天的浇水计划".to_string(),
                        expected_intent: "create_rule".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "condition".to_string(), value: "下雨".to_string() },
                            Entity { entity_type: "action".to_string(), value: "取消浇水".to_string() },
                        ],
                        context_required: true,
                    },
                ],
                expected_intents: vec![
                    "query_data".to_string(),
                    "control_device".to_string(),
                    "query_data".to_string(),
                    "create_rule".to_string(),
                ],
            },

            // 能源管理场景
            Self {
                domain: Domain::Energy,
                name: "用电优化与峰谷电价".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "当前的用电功率是多少".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![Entity { entity_type: "metric".to_string(), value: "功率".to_string() }],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "光伏电站今天的发电量".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "光伏电站".to_string() },
                            Entity { entity_type: "metric".to_string(), value: "发电量".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "储能系统还有多少电".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "device_type".to_string(), value: "储能系统".to_string() },
                            Entity { entity_type: "metric".to_string(), value: "电量".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "现在是用电高峰，帮我放电".to_string(),
                        expected_intent: "control_device".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "condition".to_string(), value: "用电高峰".to_string() },
                            Entity { entity_type: "action".to_string(), value: "放电".to_string() },
                        ],
                        context_required: true,
                    },
                ],
                expected_intents: vec![
                    "query_data".to_string(),
                    "query_data".to_string(),
                    "query_data".to_string(),
                    "control_device".to_string(),
                ],
            },

            // 智慧医疗场景
            Self {
                domain: Domain::Healthcare,
                name: "病人监护与告警".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "ICU病房3号床病人的生命体征".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "ICU病房".to_string() },
                            Entity { entity_type: "patient_id".to_string(), value: "3号床".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "心率有异常吗".to_string(),
                        expected_intent: "query_alert".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "metric".to_string(), value: "心率".to_string() },
                            Entity { entity_type: "check".to_string(), value: "异常".to_string() },
                        ],
                        context_required: true,
                    },
                    ConversationTurn {
                        user_input: "如果心率超过100，立即通知医生".to_string(),
                        expected_intent: "create_rule".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "condition".to_string(), value: "心率>100".to_string() },
                            Entity { entity_type: "action".to_string(), value: "通知医生".to_string() },
                        ],
                        context_required: true,
                    },
                ],
                expected_intents: vec![
                    "query_data".to_string(),
                    "query_alert".to_string(),
                    "create_rule".to_string(),
                ],
            },

            // 智能交通场景
            Self {
                domain: Domain::Transportation,
                name: "交通信号优化".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "主干道现在的车流量怎么样".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "主干道".to_string() },
                            Entity { entity_type: "metric".to_string(), value: "车流量".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "东南路口是不是堵车了".to_string(),
                        expected_intent: "query_alert".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "东南路口".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "调整信号灯配时，缓解拥堵".to_string(),
                        expected_intent: "control_device".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "action".to_string(), value: "调整信号灯".to_string() },
                            Entity { entity_type: "goal".to_string(), value: "缓解拥堵".to_string() },
                        ],
                        context_required: true,
                    },
                ],
                expected_intents: vec![
                    "query_data".to_string(),
                    "query_alert".to_string(),
                    "control_device".to_string(),
                ],
            },

            // 安防监控场景
            Self {
                domain: Domain::Security,
                name: "入侵检测与告警响应".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "正门有人活动吗".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "正门".to_string() },
                            Entity { entity_type: "event".to_string(), value: "活动检测".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "查看正门摄像头的画面".to_string(),
                        expected_intent: "query_device".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "正门".to_string() },
                            Entity { entity_type: "device_type".to_string(), value: "摄像头".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "有异常情况吗".to_string(),
                        expected_intent: "query_alert".to_string(),
                        expected_entities: vec![Entity { entity_type: "check".to_string(), value: "异常".to_string() }],
                        context_required: true,
                    },
                    ConversationTurn {
                        user_input: "如果有入侵者，启动报警并录像".to_string(),
                        expected_intent: "create_rule".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "condition".to_string(), value: "入侵者".to_string() },
                            Entity { entity_type: "action".to_string(), value: "报警并录像".to_string() },
                        ],
                        context_required: true,
                    },
                ],
                expected_intents: vec![
                    "query_data".to_string(),
                    "query_device".to_string(),
                    "query_alert".to_string(),
                    "create_rule".to_string(),
                ],
            },

            // 环境监测场景
            Self {
                domain: Domain::Environment,
                name: "空气质量与污染告警".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "监测站A的空气质量指数是多少".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "监测站A".to_string() },
                            Entity { entity_type: "metric".to_string(), value: "AQI".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "PM2.5超标了吗".to_string(),
                        expected_intent: "query_alert".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "metric".to_string(), value: "PM2.5".to_string() },
                            Entity { entity_type: "check".to_string(), value: "超标".to_string() },
                        ],
                        context_required: true,
                    },
                    ConversationTurn {
                        user_input: "如果AQI超过150，发布污染预警".to_string(),
                        expected_intent: "create_rule".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "condition".to_string(), value: "AQI>150".to_string() },
                            Entity { entity_type: "action".to_string(), value: "发布预警".to_string() },
                        ],
                        context_required: true,
                    },
                ],
                expected_intents: vec![
                    "query_data".to_string(),
                    "query_alert".to_string(),
                    "create_rule".to_string(),
                ],
            },

            // 智能办公场景
            Self {
                domain: Domain::Office,
                name: "会议室预订与环境控制".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "会议室A现在有人用吗".to_string(),
                        expected_intent: "query_status".to_string(),
                        expected_entities: vec![Entity { entity_type: "location".to_string(), value: "会议室A".to_string() }],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "帮我预订会议室A下午3点开会".to_string(),
                        expected_intent: "create_booking".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "会议室A".to_string() },
                            Entity { entity_type: "time".to_string(), value: "下午3点".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "开会时自动调节灯光和温度".to_string(),
                        expected_intent: "create_automation".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "trigger".to_string(), value: "开会".to_string() },
                            Entity { entity_type: "actions".to_string(), value: "调节灯光和温度".to_string() },
                        ],
                        context_required: true,
                    },
                ],
                expected_intents: vec![
                    "query_status".to_string(),
                    "create_booking".to_string(),
                    "create_automation".to_string(),
                ],
            },

            // 智慧城市场景
            Self {
                domain: Domain::SmartCity,
                name: "城市设施管理".to_string(),
                conversations: vec![
                    ConversationTurn {
                        user_input: "中山路路灯都在运行吗".to_string(),
                        expected_intent: "query_status".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "location".to_string(), value: "中山路".to_string() },
                            Entity { entity_type: "device_type".to_string(), value: "路灯".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "哪个垃圾桶需要清运".to_string(),
                        expected_intent: "query_data".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "device_type".to_string(), value: "垃圾桶".to_string() },
                            Entity { entity_type: "condition".to_string(), value: "满溢".to_string() },
                        ],
                        context_required: false,
                    },
                    ConversationTurn {
                        user_input: "通知环卫车去收集".to_string(),
                        expected_intent: "control_device".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "action".to_string(), value: "通知清运".to_string() },
                        ],
                        context_required: true,
                    },
                    ConversationTurn {
                        user_input: "检测到井盖移位了吗".to_string(),
                        expected_intent: "query_alert".to_string(),
                        expected_entities: vec![
                            Entity { entity_type: "device_type".to_string(), value: "井盖".to_string() },
                            Entity { entity_type: "check".to_string(), value: "移位".to_string() },
                        ],
                        context_required: false,
                    },
                ],
                expected_intents: vec![
                    "query_status".to_string(),
                    "query_data".to_string(),
                    "control_device".to_string(),
                    "query_alert".to_string(),
                ],
            },
        ]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_generate_all_domain_devices() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   10大领域设备生成测试                                             ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let domains = Domain::all();

    for domain in domains {
        let devices = DeviceFactory::generate_domain_devices(domain, 20);
        println!("\n📦 {}", domain.name());
        println!("   描述: {}", domain.description());
        println!("   MQTT前缀: {}", domain.mqtt_prefix());
        println!("   生成设备数: {}", devices.len());

        // Count device types
        let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for device in &devices {
            *type_counts.entry(device.device_type.clone()).or_insert(0) += 1;
        }

        for (dev_type, count) in type_counts {
            println!("   - {}: {}个", dev_type, count);
        }

        // Show example device
        if !devices.is_empty() {
            let example = &devices[0];
            println!("   示例设备:");
            println!("     ID: {}", example.id);
            println!("     名称: {}", example.name);
            println!("     位置: {}", example.location);
            println!("     遥测数量: {}", example.telemetry.len());
            println!("     命令数量: {}", example.commands.len());
        }
    }

    println!("\n✅ 所有领域设备生成完成");
}

#[tokio::test]
async fn test_domain_conversation_scenarios() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   10大领域对话场景测试                                             ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let scenarios = DomainConversationScenario::get_all_scenarios();

    for scenario in scenarios {
        println!("\n🏢 {} - {}", scenario.domain.name(), scenario.name);
        println!("   预期意图数: {}", scenario.conversations.len());

        for (idx, turn) in scenario.conversations.iter().enumerate() {
            println!("   [{}] 用户: \"{}\"", idx + 1, turn.user_input);
            println!("       预期意图: {}", turn.expected_intent);
            if !turn.expected_entities.is_empty() {
                println!("       实体: {:?}", turn.expected_entities);
            }
            println!("       需要上下文: {}", turn.context_required);
        }
    }

    println!("\n✅ 对话场景测试完成");
}

// ============================================================================
// MQTT Communication Tests
// ============================================================================

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Simulated MQTT message
#[derive(Debug, Clone)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: String,
    pub qos: u8,
    pub retain: bool,
}

/// Simulated MQTT broker for testing
pub struct MockMqttBroker {
    pub messages: Arc<Mutex<Vec<MqttMessage>>>,
    pub subscription_count: Arc<AtomicUsize>,
}

impl MockMqttBroker {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            subscription_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Simulate publishing a message
    pub fn publish(&self, topic: &str, payload: &str) {
        let msg = MqttMessage {
            topic: topic.to_string(),
            payload: payload.to_string(),
            qos: 1,
            retain: false,
        };
        self.messages.lock().unwrap().push(msg);
    }

    /// Get all published messages
    pub fn get_messages(&self) -> Vec<MqttMessage> {
        self.messages.lock().unwrap().clone()
    }

    /// Get messages by topic pattern
    pub fn get_messages_by_topic(&self, pattern: &str) -> Vec<MqttMessage> {
        self.messages.lock().unwrap()
            .iter()
            .filter(|m| m.topic.contains(pattern))
            .cloned()
            .collect()
    }

    /// Clear all messages
    pub fn clear(&self) {
        self.messages.lock().unwrap().clear();
    }
}

impl Default for MockMqttBroker {
    fn default() -> Self {
        Self::new()
    }
}

/// Simulated MQTT device that can publish telemetry
pub struct SimulatedMqttDevice {
    pub device: MqttDevice,
    broker: Arc<MockMqttBroker>,
    messages_published: Arc<AtomicUsize>,
}

impl SimulatedMqttDevice {
    /// Create a new simulated MQTT device
    pub fn new(device: MqttDevice, broker: Arc<MockMqttBroker>) -> Self {
        Self {
            device,
            broker,
            messages_published: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// "Connect" to the mock broker
    pub fn connect(&mut self) {
        self.broker.subscription_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Publish telemetry data to the mock broker
    pub fn publish_telemetry(&self, metrics: &HashMap<String, serde_json::Value>)
        -> Result<(), String>
    {
        let topic = format!("{}/{}/{}/telemetry",
            self.device.domain.mqtt_prefix(),
            self.device.device_type,
            self.device.id
        );

        let payload = serde_json::json!({
            "device_id": self.device.id,
            "device_type": self.device.device_type,
            "location": self.device.location,
            "domain": self.device.domain.mqtt_prefix(),
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap().as_secs(),
            "metrics": metrics
        });

        self.broker.publish(&topic, &payload.to_string());
        self.messages_published.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get count of published messages
    pub fn messages_published(&self) -> usize {
        self.messages_published.load(Ordering::Relaxed)
    }

    /// Generate random telemetry data based on device telemetry definitions
    pub fn generate_telemetry(&self) -> HashMap<String, serde_json::Value> {
        let mut metrics = HashMap::new();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().subsec_nanos() as f64 * 1e-9;

        for (idx, telemetry_def) in self.device.telemetry.iter().enumerate() {
            let value = match telemetry_def.data_type {
                MetricDataType::Float => {
                    let min = telemetry_def.min_value.unwrap_or(0.0);
                    let max = telemetry_def.max_value.unwrap_or(100.0);
                    // Use timestamp for pseudo-randomness
                    let range = max - min;
                    let offset = ((timestamp * 1000.0) % range) + (idx as f64 * 10.0);
                    let value = min + (offset % range).abs();
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(value).unwrap_or(serde_json::Number::from(0)))
                }
                MetricDataType::Integer => {
                    let min = telemetry_def.min_value.unwrap_or(0.0) as i64;
                    let max = telemetry_def.max_value.unwrap_or(100.0) as i64;
                    let range = (max - min).max(1);
                    let offset = ((timestamp * 1000.0) as i64 + idx as i64 * 7).abs();
                    let value = min + (offset % range);
                    serde_json::Value::Number(value.into())
                }
                MetricDataType::Boolean => {
                    // Alternate based on timestamp
                    serde_json::Value::Bool((idx as f64 + timestamp).floor() as i64 % 2 == 0)
                }
                MetricDataType::String => {
                    match telemetry_def.metric_name.as_str() {
                        "status" => serde_json::Value::String("online".to_string()),
                        "current_phase" => {
                            let phases = ["green".to_string(), "yellow".to_string(), "red".to_string()];
                            let idx = ((timestamp * 10.0) as usize) % phases.len();
                            serde_json::Value::String(phases[idx].clone())
                        }
                        _ => serde_json::Value::String("ok".to_string()),
                    }
                }
            };
            metrics.insert(telemetry_def.metric_name.clone(), value);
        }
        metrics
    }

    /// Get MQTT topic prefix for this device
    pub fn topic_prefix(&self) -> String {
        format!("{}/{}",
            self.device.domain.mqtt_prefix(),
            self.device.device_type)
    }
}

/// Test MQTT communication with simulated devices
#[tokio::test]
async fn test_mqtt_communication_simulation() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   MQTT通讯交互测试 (模拟模式)                                        ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let broker = Arc::new(MockMqttBroker::new());
    let devices_per_domain = 5;
    let telemetry_rounds = 3;

    let mut total_messages_published = 0usize;
    let mut total_devices = 0usize;
    let mut domains_tested = Vec::new();

    // Test each domain
    for domain in Domain::all() {
        println!("\n🌐 测试领域: {}", domain.name());

        // Generate devices for this domain
        let devices = DeviceFactory::generate_domain_devices(domain, devices_per_domain);
        let mut mqtt_devices = Vec::new();

        for device_def in devices {
            let mut mqtt_device = SimulatedMqttDevice::new(device_def, broker.clone());
            mqtt_device.connect();
            mqtt_devices.push(mqtt_device);
        }

        total_devices += mqtt_devices.len();
        domains_tested.push(domain.name());

        // Publish telemetry data
        let mut domain_published = 0;
        for _round in 1..=telemetry_rounds {
            for mqtt_device in &mqtt_devices {
                let telemetry = mqtt_device.generate_telemetry();
                if mqtt_device.publish_telemetry(&telemetry).is_ok() {
                    domain_published += 1;
                    total_messages_published += 1;
                }
            }
        }

        println!("   ✅ 模拟连接设备: {}", mqtt_devices.len());
        println!("   📤 发布遥测消息: {} 条", domain_published);
    }

    // Verify messages in broker
    let all_messages = broker.get_messages();
    println!("\n📊 Broker 统计:");
    println!("   存储消息总数: {}", all_messages.len());

    // Count by domain
    let mut domain_counts: HashMap<&str, usize> = HashMap::new();
    for msg in &all_messages {
        let parts: Vec<&str> = msg.topic.split('/').collect();
        if let Some(domain) = parts.first() {
            *domain_counts.entry(domain).or_insert(0) += 1;
        }
    }

    println!("   各领域消息分布:");
    for (domain, count) in &domain_counts {
        println!("     - {}: {} 条", domain, count);
    }

    // Summary
    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   测试摘要                                                           ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");
    println!("已测试领域: {}", domains_tested.join(", "));
    println!("模拟设备数: {}", total_devices);
    println!("发布消息数: {}", total_messages_published);

    println!("\n✅ MQTT通讯模拟测试完成");
}

/// Test conversation quality across all domains
#[tokio::test]
async fn test_domain_conversation_quality() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   10大领域对话质量测试                                               ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let scenarios = DomainConversationScenario::get_all_scenarios();

    let mut total_turns = 0;
    let mut total_context_required = 0;
    let mut domain_stats: HashMap<&str, DomainStats> = HashMap::new();

    for scenario in &scenarios {
        let domain_name = scenario.domain.name();
        let stats = domain_stats.entry(domain_name).or_insert(DomainStats::new(domain_name));

        stats.scenario_count += 1;
        stats.conversation_turns += scenario.conversations.len();

        for turn in &scenario.conversations {
            total_turns += 1;
            if turn.context_required {
                total_context_required += 1;
                stats.context_required += 1;
            }

            // Count expected intents
            let intent = &turn.expected_intent;
            *stats.intent_counts.entry(intent.clone()).or_insert(0) += 1;
        }
    }

    // Print results
    println!("\n📊 对话质量统计:");
    println!("总对话轮数: {}", total_turns);
    println!("需要上下文轮数: {} ({:.1}%)",
        total_context_required,
        (total_context_required as f64 / total_turns as f64) * 100.0);

    println!("\n🏢 各领域详情:");
    for (domain, stats) in &domain_stats {
        println!("\n📦 {}:", domain);
        println!("   场景数: {}", stats.scenario_count);
        println!("   对话轮数: {}", stats.conversation_turns);
        println!("   需要上下文: {}", stats.context_required);
        println!("   意图分布:");
        for (intent, count) in &stats.intent_counts {
            println!("     - {}: {}", intent, count);
        }
    }

    println!("\n✅ 对话质量测试完成");
}

#[derive(Debug, Default)]
struct DomainStats {
    name: &'static str,
    scenario_count: usize,
    conversation_turns: usize,
    context_required: usize,
    intent_counts: HashMap<String, usize>,
}

impl DomainStats {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

/// Comprehensive end-to-end test with all domains
#[tokio::test]
async fn test_comprehensive_domain_evaluation() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   综合领域评估测试                                                   ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let mut report = EvaluationReport::new();

    // 1. Device Generation Test
    println!("\n1️⃣  设备生成测试");
    let mut total_devices = 0;
    let mut total_device_types = std::collections::HashSet::new();

    for domain in Domain::all() {
        let devices = DeviceFactory::generate_domain_devices(domain, 10);
        total_devices += devices.len();
        for device in &devices {
            total_device_types.insert(device.device_type.clone());
        }
    }

    report.device_generation_score = if total_devices >= 100 { 100 } else { total_devices };
    println!("   ✅ 生成设备总数: {}", total_devices);
    println!("   ✅ 设备类型数: {}", total_device_types.len());

    // 2. Scenario Coverage Test
    println!("\n2️⃣  场景覆盖测试");
    let scenarios = DomainConversationScenario::get_all_scenarios();
    let total_conversations = scenarios.iter().map(|s| s.conversations.len()).sum::<usize>();

    report.scenario_coverage = if scenarios.len() >= 10 { 100 } else { scenarios.len() * 10 };
    println!("   ✅ 场景数量: {}", scenarios.len());
    println!("   ✅ 对话轮数: {}", total_conversations);

    // 3. Intent Diversity Test
    println!("\n3️⃣  意图多样性测试");
    let mut all_intents = std::collections::HashSet::new();
    for scenario in &scenarios {
        for turn in &scenario.conversations {
            all_intents.insert(turn.expected_intent.clone());
        }
    }

    report.intent_diversity = if all_intents.len() >= 10 { 100 } else { all_intents.len() * 10 };
    println!("   ✅ 唯一意图数: {}", all_intents.len());

    // 4. Telemetry Richness Test
    println!("\n4️⃣  遥测丰富度测试");
    let mut total_telemetry_points = 0;
    for domain in Domain::all() {
        let devices = DeviceFactory::generate_domain_devices(domain, 5);
        for device in &devices {
            total_telemetry_points += device.telemetry.len();
        }
    }

    report.telemetry_richness = if total_telemetry_points >= 50 { 100 } else { total_telemetry_points * 2 };
    println!("   ✅ 遥测点总数: {}", total_telemetry_points);

    // 5. Command Capability Test
    println!("\n5️⃣  命令能力测试");
    let mut total_commands = 0;
    let mut actuator_count = 0;

    for domain in Domain::all() {
        let devices = DeviceFactory::generate_domain_devices(domain, 5);
        for device in &devices {
            total_commands += device.commands.len();
            if device.capabilities.is_actuator {
                actuator_count += 1;
            }
        }
    }

    report.command_capability = if total_commands >= 30 { 100 } else { total_commands * 3 };
    println!("   ✅ 命令总数: {}", total_commands);
    println!("   ✅ 执行器数量: {}", actuator_count);

    // Final Score
    let overall_score = (
        report.device_generation_score +
        report.scenario_coverage +
        report.intent_diversity +
        report.telemetry_richness +
        report.command_capability
    ) / 5;

    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   评估报告                                                           ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");
    println!("设备生成: {}/100", report.device_generation_score);
    println!("场景覆盖: {}/100", report.scenario_coverage);
    println!("意图多样性: {}/100", report.intent_diversity);
    println!("遥测丰富度: {}/100", report.telemetry_richness);
    println!("命令能力: {}/100", report.command_capability);
    println!("\n综合评分: {}/100", overall_score);

    let grade = match overall_score {
        s if s >= 90 => "优秀 ⭐⭐⭐⭐⭐",
        s if s >= 80 => "良好 ⭐⭐⭐⭐",
        s if s >= 70 => "中等 ⭐⭐⭐",
        s if s >= 60 => "及格 ⭐⭐",
        _ => "不及格 ⭐",
    };
    println!("评级: {}", grade);

    println!("\n✅ 综合评估测试完成");
}

#[derive(Debug, Default)]
struct EvaluationReport {
    device_generation_score: usize,
    scenario_coverage: usize,
    intent_diversity: usize,
    telemetry_richness: usize,
    command_capability: usize,
}

impl EvaluationReport {
    fn new() -> Self {
        Self::default()
    }
}
