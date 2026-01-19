//! NeoTalk 任务规划与工具调用测试
//!
//! 测试评估LLM的以下能力：
//! - 任务规划能力：将复杂任务分解为步骤
//! - 工具识别能力：选择正确的工具完成任务
//! - 工具关联能力：理解工具之间的依赖关系
//! - 并行调用能力：识别可以并行执行的工具
//! - 参数提取能力：正确提取工具参数
//!
//! **测试日期**: 2026-01-18

use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use edge_ai_llm::backends::create_backend;
use edge_ai_core::llm::backend::{GenerationParams, LlmInput};
use edge_ai_core::message::{Message, MessageRole, Content};

const OLLAMA_ENDPOINT: &str = "http://localhost:11434";

// ============================================================================
// 工具定义（模拟系统中的实际工具）
// ============================================================================

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub category: ToolCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub type_: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    DeviceQuery,
    DeviceControl,
    DataQuery,
    RuleManagement,
    WorkflowManagement,
    System,
}

/// 获取可用工具列表
pub fn get_available_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_devices".to_string(),
            description: "列出所有设备，支持按类型和状态筛选".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "device_type".to_string(),
                    type_: "string".to_string(),
                    required: false,
                    description: "设备类型，如 light, sensor, switch".to_string(),
                },
                ToolParameter {
                    name: "status".to_string(),
                    type_: "string".to_string(),
                    required: false,
                    description: "设备状态，如 online, offline".to_string(),
                },
            ],
            category: ToolCategory::DeviceQuery,
        },
        ToolDefinition {
            name: "control_device".to_string(),
            description: "控制设备开关或设置参数".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "device_id".to_string(),
                    type_: "string".to_string(),
                    required: true,
                    description: "设备ID，如 living_room_light".to_string(),
                },
                ToolParameter {
                    name: "action".to_string(),
                    type_: "string".to_string(),
                    required: true,
                    description: "操作：on, off, set_value".to_string(),
                },
                ToolParameter {
                    name: "value".to_string(),
                    type_: "number".to_string(),
                    required: false,
                    description: "参数值".to_string(),
                },
            ],
            category: ToolCategory::DeviceControl,
        },
        ToolDefinition {
            name: "get_device_data".to_string(),
            description: "获取设备当前读数".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "device_id".to_string(),
                    type_: "string".to_string(),
                    required: true,
                    description: "设备ID".to_string(),
                },
            ],
            category: ToolCategory::DataQuery,
        },
        ToolDefinition {
            name: "query_history".to_string(),
            description: "查询历史数据".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "device_id".to_string(),
                    type_: "string".to_string(),
                    required: true,
                    description: "设备ID".to_string(),
                },
                ToolParameter {
                    name: "hours".to_string(),
                    type_: "number".to_string(),
                    required: false,
                    description: "查询小时数".to_string(),
                },
            ],
            category: ToolCategory::DataQuery,
        },
        ToolDefinition {
            name: "create_rule".to_string(),
            description: "创建自动化规则".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "name".to_string(),
                    type_: "string".to_string(),
                    required: true,
                    description: "规则名称".to_string(),
                },
                ToolParameter {
                    name: "condition".to_string(),
                    type_: "string".to_string(),
                    required: true,
                    description: "触发条件".to_string(),
                },
                ToolParameter {
                    name: "action".to_string(),
                    type_: "string".to_string(),
                    required: true,
                    description: "执行动作".to_string(),
                },
            ],
            category: ToolCategory::RuleManagement,
        },
        ToolDefinition {
            name: "list_rules".to_string(),
            description: "列出所有规则".to_string(),
            parameters: vec![],
            category: ToolCategory::RuleManagement,
        },
        ToolDefinition {
            name: "trigger_workflow".to_string(),
            description: "触发工作流执行".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "workflow_id".to_string(),
                    type_: "string".to_string(),
                    required: true,
                    description: "工作流ID".to_string(),
                },
            ],
            category: ToolCategory::WorkflowManagement,
        },
        ToolDefinition {
            name: "get_system_status".to_string(),
            description: "获取系统状态".to_string(),
            parameters: vec![],
            category: ToolCategory::System,
        },
    ]
}

// ============================================================================
// 任务规划测试场景
// ============================================================================

/// 语言设置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestLanguage {
    Chinese,
    English,
}

/// 任务规划测试场景
#[derive(Debug, Clone)]
pub struct TaskPlanningScenario {
    pub name: String,
    pub description: String,
    pub user_request: String,
    pub expected_plan: TaskPlan,
    pub tools_available: Vec<String>,
    pub language: TestLanguage,
}

/// 期望的任务计划
#[derive(Debug, Clone)]
pub struct TaskPlan {
    pub steps: Vec<TaskStep>,
    pub can_parallel: bool,  // 是否有可并行的步骤
    pub parallel_groups: Vec<Vec<usize>>,  // 可并行执行的步骤组
}

#[derive(Debug, Clone)]
pub struct TaskStep {
    pub step_number: usize,
    pub tool_name: String,
    pub description: String,
    pub dependencies: Vec<usize>,  // 依赖的步骤编号
    pub expected_params: Vec<(String, String)>,  // (参数名, 期望值)
}

/// 测试场景集合
pub fn get_task_planning_scenarios() -> Vec<TaskPlanningScenario> {
    vec![
        // 场景1: 简单单步任务 - 只需调用一个工具
        TaskPlanningScenario {
            name: "简单设备控制".to_string(),
            description: "用户只需控制一个设备".to_string(),
            user_request: "打开客厅的灯".to_string(),
            tools_available: vec!["list_devices".to_string(), "control_device".to_string()],
            language: TestLanguage::Chinese,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "control_device".to_string(),
                        description: "控制客厅灯打开".to_string(),
                        dependencies: vec![],
                        expected_params: vec![
                            ("device_id".to_string(), "living_room_light".to_string()),
                            ("action".to_string(), "on".to_string()),
                        ],
                    },
                ],
                can_parallel: false,
                parallel_groups: vec![],
            },
        },

        // 场景2: 独立多任务 - 多个工具无依赖，可并行执行
        TaskPlanningScenario {
            name: "多设备独立查询".to_string(),
            description: "查询多个独立设备的状态，可并行".to_string(),
            user_request: "同时查询客厅、卧室和厨房的温度".to_string(),
            tools_available: vec![
                "get_device_data".to_string(),
                "list_devices".to_string(),
            ],
            language: TestLanguage::Chinese,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "get_device_data".to_string(),
                        description: "查询客厅温度".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_id".to_string(), "living_room_temp_sensor".to_string())],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "get_device_data".to_string(),
                        description: "查询卧室温度".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_id".to_string(), "bedroom_temp_sensor".to_string())],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "get_device_data".to_string(),
                        description: "查询厨房温度".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_id".to_string(), "kitchen_temp_sensor".to_string())],
                    },
                ],
                can_parallel: true,
                parallel_groups: vec![vec![0, 1, 2]],
            },
        },

        // 场景3: 顺序依赖任务 - 后续任务依赖前序结果
        TaskPlanningScenario {
            name: "依赖式任务序列".to_string(),
            description: "需要先查询再控制的依赖任务".to_string(),
            user_request: "检查所有传感器的状态，如果温度超过30度就打开风扇".to_string(),
            tools_available: vec![
                "list_devices".to_string(),
                "get_device_data".to_string(),
                "control_device".to_string(),
            ],
            language: TestLanguage::Chinese,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_devices".to_string(),
                        description: "列出所有温度传感器".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_type".to_string(), "sensor".to_string())],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "get_device_data".to_string(),
                        description: "查询温度读数".to_string(),
                        dependencies: vec![0],  // 依赖步骤1
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "control_device".to_string(),
                        description: "控制风扇".to_string(),
                        dependencies: vec![1],  // 依赖步骤2
                        expected_params: vec![
                            ("device_id".to_string(), "fan".to_string()),
                            ("action".to_string(), "on".to_string()),
                        ],
                    },
                ],
                can_parallel: false,
                parallel_groups: vec![],
            },
        },

        // 场景4: 批量操作任务 - 需要对多个设备执行相同操作
        TaskPlanningScenario {
            name: "批量设备控制".to_string(),
            description: "关闭所有房间的灯光".to_string(),
            user_request: "关闭所有房间的灯".to_string(),
            tools_available: vec![
                "list_devices".to_string(),
                "control_device".to_string(),
            ],
            language: TestLanguage::Chinese,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_devices".to_string(),
                        description: "列出所有灯光设备".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_type".to_string(), "light".to_string())],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "control_device".to_string(),
                        description: "关闭客厅灯".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "control_device".to_string(),
                        description: "关闭卧室灯".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 4,
                        tool_name: "control_device".to_string(),
                        description: "关闭厨房灯".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                ],
                can_parallel: true,  // 步骤2-4可以并行
                parallel_groups: vec![vec![0], vec![1, 2, 3]],
            },
        },

        // 场景5: 条件分支任务 - 根据条件选择不同工具
        TaskPlanningScenario {
            name: "条件式任务规划".to_string(),
            description: "根据查询结果决定后续操作".to_string(),
            user_request: "查看当前时间，如果是晚上就开灯，如果是白天就关灯".to_string(),
            tools_available: vec![
                "get_system_status".to_string(),
                "control_device".to_string(),
            ],
            language: TestLanguage::Chinese,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "get_system_status".to_string(),
                        description: "获取系统状态（包括时间）".to_string(),
                        dependencies: vec![],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "control_device".to_string(),
                        description: "根据时间控制灯".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                ],
                can_parallel: false,
                parallel_groups: vec![],
            },
        },

        // 场景6: 混合并行任务 - 部分可并行，部分有依赖
        TaskPlanningScenario {
            name: "复杂混合任务".to_string(),
            description: "包含并行和依赖的复杂任务".to_string(),
            user_request: "同时查询所有温度和历史数据，然后根据结果决定是否创建告警规则".to_string(),
            tools_available: vec![
                "list_devices".to_string(),
                "get_device_data".to_string(),
                "query_history".to_string(),
                "create_rule".to_string(),
            ],
            language: TestLanguage::Chinese,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_devices".to_string(),
                        description: "列出温度传感器".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_type".to_string(), "sensor".to_string())],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "get_device_data".to_string(),
                        description: "获取当前温度".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "query_history".to_string(),
                        description: "查询历史数据".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 4,
                        tool_name: "create_rule".to_string(),
                        description: "创建告警规则".to_string(),
                        dependencies: vec![1, 2],  // 依赖步骤2和3
                        expected_params: vec![],
                    },
                ],
                can_parallel: true,
                parallel_groups: vec![vec![1, 2]],  // 步骤2和3可并行
            },
        },

        // 场景7: 多工具协作任务 - 多个工具配合完成一个目标
        TaskPlanningScenario {
            name: "离家模式任务".to_string(),
            description: "离家前的一整套操作".to_string(),
            user_request: "我要出门了，帮我做好离家准备".to_string(),
            tools_available: vec![
                "list_devices".to_string(),
                "get_device_data".to_string(),
                "control_device".to_string(),
                "get_system_status".to_string(),
            ],
            language: TestLanguage::Chinese,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_devices".to_string(),
                        description: "列出所有设备".to_string(),
                        dependencies: vec![],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "control_device".to_string(),
                        description: "关闭所有灯光".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "control_device".to_string(),
                        description: "关闭空调".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 4,
                        tool_name: "get_device_data".to_string(),
                        description: "检查门窗状态".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 5,
                        tool_name: "get_system_status".to_string(),
                        description: "启用安防模式".to_string(),
                        dependencies: vec![],
                        expected_params: vec![],
                    },
                ],
                can_parallel: true,
                parallel_groups: vec![vec![1, 2, 3]],  // 步骤1-3可并行
            },
        },

        // 场景8: 规则和工作流组合任务
        TaskPlanningScenario {
            name: "自动化管理任务".to_string(),
            description: "管理规则和工作流".to_string(),
            user_request: "帮我查看所有规则，然后启用高温告警规则，最后触发早晨工作流".to_string(),
            tools_available: vec![
                "list_rules".to_string(),
                "create_rule".to_string(),
                "trigger_workflow".to_string(),
            ],
            language: TestLanguage::Chinese,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_rules".to_string(),
                        description: "列出所有规则".to_string(),
                        dependencies: vec![],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "create_rule".to_string(),
                        description: "启用/更新高温告警规则".to_string(),
                        dependencies: vec![0],  // 依赖步骤1的结果
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "trigger_workflow".to_string(),
                        description: "触发早晨工作流".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("workflow_id".to_string(), "morning_routine".to_string())],
                    },
                ],
                can_parallel: true,
                parallel_groups: vec![vec![0], vec![2]],  // 步骤1和3独立
            },
        },
    ]
}

/// 获取英文测试场景
pub fn get_task_planning_scenarios_english() -> Vec<TaskPlanningScenario> {
    vec![
        // Scenario 1: Simple single-step task
        TaskPlanningScenario {
            name: "Simple Device Control".to_string(),
            description: "User only needs to control one device".to_string(),
            user_request: "Turn on the living room light".to_string(),
            tools_available: vec!["list_devices".to_string(), "control_device".to_string()],
            language: TestLanguage::English,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "control_device".to_string(),
                        description: "Turn on living room light".to_string(),
                        dependencies: vec![],
                        expected_params: vec![
                            ("device_id".to_string(), "living_room_light".to_string()),
                            ("action".to_string(), "on".to_string()),
                        ],
                    },
                ],
                can_parallel: false,
                parallel_groups: vec![],
            },
        },

        // Scenario 2: Independent multi-task
        TaskPlanningScenario {
            name: "Multi-Device Independent Query".to_string(),
            description: "Query multiple independent devices, can run in parallel".to_string(),
            user_request: "Query the temperature in living room, bedroom and kitchen at the same time".to_string(),
            tools_available: vec![
                "get_device_data".to_string(),
                "list_devices".to_string(),
            ],
            language: TestLanguage::English,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "get_device_data".to_string(),
                        description: "Query living room temperature".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_id".to_string(), "living_room_temp_sensor".to_string())],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "get_device_data".to_string(),
                        description: "Query bedroom temperature".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_id".to_string(), "bedroom_temp_sensor".to_string())],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "get_device_data".to_string(),
                        description: "Query kitchen temperature".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_id".to_string(), "kitchen_temp_sensor".to_string())],
                    },
                ],
                can_parallel: true,
                parallel_groups: vec![vec![0, 1, 2]],
            },
        },

        // Scenario 3: Sequential dependent tasks
        TaskPlanningScenario {
            name: "Dependent Task Sequence".to_string(),
            description: "Tasks that require query before control".to_string(),
            user_request: "Check all sensor status, if temperature exceeds 30 degrees then turn on the fan".to_string(),
            tools_available: vec![
                "list_devices".to_string(),
                "get_device_data".to_string(),
                "control_device".to_string(),
            ],
            language: TestLanguage::English,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_devices".to_string(),
                        description: "List all temperature sensors".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_type".to_string(), "sensor".to_string())],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "get_device_data".to_string(),
                        description: "Query temperature readings".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "control_device".to_string(),
                        description: "Control the fan".to_string(),
                        dependencies: vec![1],
                        expected_params: vec![
                            ("device_id".to_string(), "fan".to_string()),
                            ("action".to_string(), "on".to_string()),
                        ],
                    },
                ],
                can_parallel: false,
                parallel_groups: vec![],
            },
        },

        // Scenario 4: Batch operation task
        TaskPlanningScenario {
            name: "Batch Device Control".to_string(),
            description: "Turn off lights in all rooms".to_string(),
            user_request: "Turn off all the lights in the house".to_string(),
            tools_available: vec![
                "list_devices".to_string(),
                "control_device".to_string(),
            ],
            language: TestLanguage::English,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_devices".to_string(),
                        description: "List all light devices".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_type".to_string(), "light".to_string())],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "control_device".to_string(),
                        description: "Turn off living room light".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "control_device".to_string(),
                        description: "Turn off bedroom light".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 4,
                        tool_name: "control_device".to_string(),
                        description: "Turn off kitchen light".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                ],
                can_parallel: true,
                parallel_groups: vec![vec![0], vec![1, 2, 3]],
            },
        },

        // Scenario 5: Conditional task
        TaskPlanningScenario {
            name: "Conditional Task Planning".to_string(),
            description: "Decide next action based on query result".to_string(),
            user_request: "Check current time, turn on light if it's evening, turn off if it's daytime".to_string(),
            tools_available: vec![
                "get_system_status".to_string(),
                "control_device".to_string(),
            ],
            language: TestLanguage::English,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "get_system_status".to_string(),
                        description: "Get system status including time".to_string(),
                        dependencies: vec![],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "control_device".to_string(),
                        description: "Control light based on time".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                ],
                can_parallel: false,
                parallel_groups: vec![],
            },
        },

        // Scenario 6: Complex mixed task
        TaskPlanningScenario {
            name: "Complex Mixed Task".to_string(),
            description: "Complex task with parallel and dependent operations".to_string(),
            user_request: "Query all temperature and historical data simultaneously, then decide whether to create alert rule based on results".to_string(),
            tools_available: vec![
                "list_devices".to_string(),
                "get_device_data".to_string(),
                "query_history".to_string(),
                "create_rule".to_string(),
            ],
            language: TestLanguage::English,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_devices".to_string(),
                        description: "List temperature sensors".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("device_type".to_string(), "sensor".to_string())],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "get_device_data".to_string(),
                        description: "Get current temperature".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "query_history".to_string(),
                        description: "Query historical data".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 4,
                        tool_name: "create_rule".to_string(),
                        description: "Create alert rule".to_string(),
                        dependencies: vec![1, 2],
                        expected_params: vec![],
                    },
                ],
                can_parallel: true,
                parallel_groups: vec![vec![1, 2]],
            },
        },

        // Scenario 7: Multi-tool coordination
        TaskPlanningScenario {
            name: "Away Mode Task".to_string(),
            description: "A set of operations before leaving home".to_string(),
            user_request: "I'm going out, help me prepare for leaving".to_string(),
            tools_available: vec![
                "list_devices".to_string(),
                "get_device_data".to_string(),
                "control_device".to_string(),
                "get_system_status".to_string(),
            ],
            language: TestLanguage::English,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_devices".to_string(),
                        description: "List all devices".to_string(),
                        dependencies: vec![],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "control_device".to_string(),
                        description: "Turn off all lights".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "control_device".to_string(),
                        description: "Turn off air conditioning".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 4,
                        tool_name: "get_device_data".to_string(),
                        description: "Check door and window status".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 5,
                        tool_name: "get_system_status".to_string(),
                        description: "Enable security mode".to_string(),
                        dependencies: vec![],
                        expected_params: vec![],
                    },
                ],
                can_parallel: true,
                parallel_groups: vec![vec![1, 2, 3]],
            },
        },

        // Scenario 8: Rule and workflow combination
        TaskPlanningScenario {
            name: "Automation Management Task".to_string(),
            description: "Manage rules and workflows".to_string(),
            user_request: "Help me check all rules, then enable high temperature alert rule, finally trigger morning workflow".to_string(),
            tools_available: vec![
                "list_rules".to_string(),
                "create_rule".to_string(),
                "trigger_workflow".to_string(),
            ],
            language: TestLanguage::English,
            expected_plan: TaskPlan {
                steps: vec![
                    TaskStep {
                        step_number: 1,
                        tool_name: "list_rules".to_string(),
                        description: "List all rules".to_string(),
                        dependencies: vec![],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 2,
                        tool_name: "create_rule".to_string(),
                        description: "Enable/update high temperature alert rule".to_string(),
                        dependencies: vec![0],
                        expected_params: vec![],
                    },
                    TaskStep {
                        step_number: 3,
                        tool_name: "trigger_workflow".to_string(),
                        description: "Trigger morning workflow".to_string(),
                        dependencies: vec![],
                        expected_params: vec![("workflow_id".to_string(), "morning_routine".to_string())],
                    },
                ],
                can_parallel: true,
                parallel_groups: vec![vec![0], vec![2]],
            },
        },
    ]
}

// ============================================================================
// 工具调用解析结果
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParse {
    pub tool_name: String,
    pub parameters: Vec<(String, String)>,
    pub confidence: f64,  // 置信度 0-1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedToolCalls {
    pub calls: Vec<ToolCallParse>,
    pub has_parallel_calls: bool,
    pub parallel_group_count: usize,
}

// ============================================================================
// 评估器
// ============================================================================

pub struct ToolCallingEvaluator {
    model_name: String,
    llm: Arc<dyn edge_ai_core::llm::backend::LlmRuntime>,
    timeout_secs: u64,
    tools: Vec<ToolDefinition>,
    language: TestLanguage,
}

impl ToolCallingEvaluator {
    pub fn new(model_name: &str) -> Result<Self, String> {
        Self::new_with_language(model_name, TestLanguage::Chinese)
    }

    pub fn new_with_language(model_name: &str, language: TestLanguage) -> Result<Self, String> {
        let llm_config = serde_json::json!({
            "endpoint": OLLAMA_ENDPOINT,
            "model": model_name
        });

        let llm = create_backend("ollama", &llm_config)
            .map_err(|e| format!("Failed to create LLM backend: {:?}", e))?;

        Ok(Self {
            model_name: model_name.to_string(),
            llm,
            timeout_secs: 60,
            tools: get_available_tools(),
            language,
        })
    }

    /// 运行所有场景测试
    pub async fn evaluate_all(&self) -> ToolCallingEvaluationReport {
        let lang_tag = match self.language {
            TestLanguage::Chinese => "中文",
            TestLanguage::English => "English",
        };
        println!("\n╔════════════════════════════════════════════════════════════════════════╗");
        println!("║   NeoTalk Task Planning & Tool Calling Test ({})                        ║", lang_tag);
        println!("║   Model: {:58}║", self.model_name);
        println!("╚════════════════════════════════════════════════════════════════════════╝");

        let scenarios = match self.language {
            TestLanguage::Chinese => get_task_planning_scenarios(),
            TestLanguage::English => get_task_planning_scenarios_english(),
        };
        let mut results = Vec::new();

        for scenario in &scenarios {
            println!("\n📋 场景: {}", scenario.name);
            println!("   {}", scenario.description);
            println!("   请求: {}", scenario.user_request);

            let result = self.evaluate_scenario(scenario).await;
            self.print_scenario_result(&result);
            results.push(result);
        }

        self.generate_final_report(results)
    }

    /// 评估指定场景列表
    pub async fn evaluate_scenarios(&self, scenarios: &[TaskPlanningScenario]) -> ToolCallingEvaluationReport {
        let lang_tag = match self.language {
            TestLanguage::Chinese => "中文",
            TestLanguage::English => "English",
        };
        println!("\n╔════════════════════════════════════════════════════════════════════════╗");
        println!("║   NeoTalk Task Planning & Tool Calling Test ({})                        ║", lang_tag);
        println!("║   Model: {:58}║", self.model_name);
        println!("╚════════════════════════════════════════════════════════════════════════╝");

        let mut results = Vec::new();

        for scenario in scenarios {
            println!("\n📋 场景: {}", scenario.name);
            println!("   {}", scenario.description);
            println!("   请求: {}", scenario.user_request);

            let result = self.evaluate_scenario(scenario).await;
            self.print_scenario_result(&result);
            results.push(result);
        }

        self.generate_final_report(results)
    }

    /// 评估单个场景
    async fn evaluate_scenario(&self, scenario: &TaskPlanningScenario) -> ScenarioResult {
        // 构建系统提示，包含工具列表
        let system_prompt = self.build_system_prompt(&scenario.tools_available);

        let start = Instant::now();

        let response = self.send_message(&scenario.user_request, &system_prompt).await;
        let response_time = start.elapsed().as_millis();

        // 解析工具调用
        let parsed_calls = self.parse_tool_calls(&response);

        // 评估工具识别准确率
        let tool_recognition = self.evaluate_tool_recognition(&parsed_calls, &scenario.expected_plan);

        // 评估参数提取准确率
        let param_extraction = self.evaluate_param_extraction(&parsed_calls, &scenario.expected_plan);

        // 评估并行识别准确率
        let parallel_recognition = self.evaluate_parallel_recognition(&parsed_calls, &scenario.expected_plan);

        // 评估任务规划合理性
        let planning_quality = self.evaluate_planning_quality(&parsed_calls, &scenario.expected_plan);

        // 计算综合得分
        let overall_score = (tool_recognition.score * 0.3 +
                            param_extraction.score * 0.3 +
                            parallel_recognition.score * 0.2 +
                            planning_quality.score * 0.2).min(100.0);

        println!("        解析到 {} 个工具调用", parsed_calls.calls.len());
        println!("        响应时间: {}ms", response_time);

        ScenarioResult {
            scenario_name: scenario.name.clone(),
            user_request: scenario.user_request.clone(),
            llm_response: response,
            response_time_ms: response_time,
            parsed_calls,
            tool_recognition,
            param_extraction,
            parallel_recognition,
            planning_quality,
            overall_score,
        }
    }

    fn build_system_prompt(&self, available_tools: &[String]) -> String {
        match self.language {
            TestLanguage::Chinese => self.build_chinese_prompt(available_tools),
            TestLanguage::English => self.build_english_prompt(available_tools),
        }
    }

    fn build_chinese_prompt(&self, available_tools: &[String]) -> String {
        let mut prompt = "你是 NeoTalk 智能助手。你的任务是根据用户请求，选择合适的工具并正确提取参数。\n\n".to_string();

        prompt += "═══════════════════════════════════════════════════════════════\n";
        prompt += "可用工具列表\n";
        prompt += "═══════════════════════════════════════════════════════════════\n";

        for tool in &self.tools {
            if available_tools.contains(&tool.name) {
                prompt += &format!("\n【工具】: {}\n", tool.name);
                prompt += &format!("描述: {}\n", tool.description);
                if !tool.parameters.is_empty() {
                    prompt += "参数:\n";
                    for param in &tool.parameters {
                        let required = if param.required { "【必需】" } else { "【可选】" };
                        prompt += &format!("  • {}: {} {} - {}\n",
                            param.name, param.type_, required, param.description);
                    }
                }
            }
        }

        prompt += "\n═══════════════════════════════════════════════════════════════\n";
        prompt += "输出格式要求\n";
        prompt += "═══════════════════════════════════════════════════════════════\n";
        prompt += r#"
你必须严格按照以下JSON格式输出工具调用：

[
  {
    "tool": "工具名称",
    "parameters": {
      "参数名1": "参数值1",
      "参数名2": "参数值2"
    }
  }
]

重要规则：
1. 必需参数必须提供值
2. 参数值要从用户请求中提取，不要编造
3. 设备ID要从用户的描述中推断（如"客厅灯"→"living_room_light"）
4. 动作值要使用标准术语（on/off/set_value等）
5. 只输出JSON，不要有任何其他文字说明
"#;

        prompt += "\n═══════════════════════════════════════════════════════════════\n";
        prompt += "工具调用示例\n";
        prompt += "═══════════════════════════════════════════════════════════════\n";
        prompt += r#"
用户: 打开客厅的灯
输出: [{"tool": "control_device", "parameters": {"device_id": "living_room_light", "action": "on"}}]

用户: 查询客厅、卧室和厨房的温度
输出: [
  {"tool": "get_device_data", "parameters": {"device_id": "living_room_temp_sensor"}},
  {"tool": "get_device_data", "parameters": {"device_id": "bedroom_temp_sensor"}},
  {"tool": "get_device_data", "parameters": {"device_id": "kitchen_temp_sensor"}}
]

用户: 列出所有温度传感器
输出: [{"tool": "list_devices", "parameters": {"device_type": "sensor"}}]
"#;

        prompt += "\n═══════════════════════════════════════════════════════════════\n";
        prompt += "执行顺序说明\n";
        prompt += "═══════════════════════════════════════════════════════════════\n";
        prompt += r#"
• 并行执行: 如果工具之间没有依赖关系，可以同时调用
• 顺序执行: 如果后一个工具需要前一个工具的结果，按顺序列出
• 依赖关系:
  - control_device 需要 list_devices 先获取设备列表
  - get_device_data 需要知道具体的 device_id
  - create_rule 可以独立执行
"#;

        prompt
    }

    fn build_english_prompt(&self, available_tools: &[String]) -> String {
        let mut prompt = "You are NeoTalk AI Assistant. Your task is to select appropriate tools and extract parameters correctly based on user requests.\n\n".to_string();

        prompt += "═══════════════════════════════════════════════════════════════\n";
        prompt += "Available Tools\n";
        prompt += "═══════════════════════════════════════════════════════════════\n";

        for tool in &self.tools {
            if available_tools.contains(&tool.name) {
                prompt += &format!("\n[Tool]: {}\n", tool.name);
                prompt += &format!("Description: {}\n", tool.description);
                if !tool.parameters.is_empty() {
                    prompt += "Parameters:\n";
                    for param in &tool.parameters {
                        let required = if param.required { "[REQUIRED]" } else { "[OPTIONAL]" };
                        prompt += &format!("  • {}: {} {} - {}\n",
                            param.name, param.type_, required, param.description);
                    }
                }
            }
        }

        prompt += "\n═══════════════════════════════════════════════════════════════\n";
        prompt += "Output Format Requirements\n";
        prompt += "═══════════════════════════════════════════════════════════════\n";
        prompt += r#"
You MUST output tool calls in the following JSON format:

[
  {
    "tool": "tool_name",
    "parameters": {
      "param1": "value1",
      "param2": "value2"
    }
  }
]

Important Rules:
1. Required parameters MUST have values
2. Extract parameter values from user request, do not fabricate
3. Infer device IDs from user description (e.g., "living room light" → "living_room_light")
4. Use standard action terms (on/off/set_value etc.)
5. Output ONLY JSON, no additional text
"#;

        prompt += "\n═══════════════════════════════════════════════════════════════\n";
        prompt += "Tool Call Examples\n";
        prompt += "═══════════════════════════════════════════════════════════════\n";
        prompt += r#"
User: Turn on the living room light
Output: [{"tool": "control_device", "parameters": {"device_id": "living_room_light", "action": "on"}}]

User: Query the temperature in living room, bedroom and kitchen
Output: [
  {"tool": "get_device_data", "parameters": {"device_id": "living_room_temp_sensor"}},
  {"tool": "get_device_data", "parameters": {"device_id": "bedroom_temp_sensor"}},
  {"tool": "get_device_data", "parameters": {"device_id": "kitchen_temp_sensor"}}
]

User: List all temperature sensors
Output: [{"tool": "list_devices", "parameters": {"device_type": "sensor"}}]
"#;

        prompt += "\n═══════════════════════════════════════════════════════════════\n";
        prompt += "Execution Order Guidelines\n";
        prompt += "═══════════════════════════════════════════════════════════════\n";
        prompt += r#"
• Parallel execution: Tools without dependencies can be called simultaneously
• Sequential execution: Tools requiring previous results should be listed in order
• Dependencies:
  - control_device requires list_devices to get device list first
  - get_device_data needs specific device_id
  - create_rule can be executed independently
"#;

        prompt
    }

    async fn send_message(&self, user_input: &str, system_prompt: &str) -> String {
        let messages = vec![
            Message {
                role: MessageRole::System,
                content: Content::Text(system_prompt.to_string()),
                timestamp: None,
            },
            Message {
                role: MessageRole::User,
                content: Content::Text(user_input.to_string()),
                timestamp: None,
            },
        ];

        let llm_input = LlmInput {
            messages,
            params: GenerationParams {
                max_tokens: Some(800),  // 增加以支持复杂场景
                temperature: Some(0.0),  // 0温度以获得最稳定的JSON输出
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

    /// 解析工具调用（从LLM响应中提取）
    fn parse_tool_calls(&self, response: &str) -> ParsedToolCalls {
        let mut calls = Vec::new();

        // 清理响应：移除可能的markdown代码块标记
        let cleaned_response = response
            .replace("```json", "")
            .replace("```JSON", "")
            .replace("```", "")
            .trim()
            .to_string();

        // 尝试解析JSON数组格式
        let parse_result = serde_json::from_str::<Value>(&cleaned_response);

        if let Ok(json_value) = parse_result {
            if let Some(array) = json_value.as_array() {
                for item in array {
                    if let Some(obj) = item.as_object() {
                        if let Some(tool_name) = obj.get("tool").and_then(|v| v.as_str()) {
                            let mut params = Vec::new();
                            if let Some(parameters) = obj.get("parameters").and_then(|v| v.as_object()) {
                                for (key, value) in parameters {
                                    let value_str = if value.is_string() {
                                        value.as_str().unwrap().to_string()
                                    } else {
                                        value.to_string()
                                    };
                                    params.push((key.clone(), value_str));
                                }
                            }
                            calls.push(ToolCallParse {
                                tool_name: tool_name.to_string(),
                                parameters: params,
                                confidence: 1.0,
                            });
                        }
                    }
                }
            }
        }

        // 简单的关键词检测作为补充
        if calls.is_empty() {
            for tool in &self.tools {
                if cleaned_response.contains(&tool.name) {
                    calls.push(ToolCallParse {
                        tool_name: tool.name.clone(),
                        parameters: vec![],
                        confidence: 0.5,
                    });
                }
            }
        }

        // 检测是否有并行调用
        let has_parallel = calls.len() > 1;

        // 估算并行组数（简化处理：假设所有没有依赖的都可以并行）
        let parallel_group_count = if calls.len() > 1 { 1 } else { 0 };

        ParsedToolCalls {
            calls,
            has_parallel_calls: has_parallel,
            parallel_group_count,
        }
    }

    fn evaluate_tool_recognition(&self, parsed: &ParsedToolCalls, expected: &TaskPlan) -> MetricScore {
        let expected_tools: Vec<&str> = expected.steps.iter()
            .map(|s| s.tool_name.as_str())
            .collect();

        let mut recognized = 0;
        let mut total = expected_tools.len();

        for tool in &expected_tools {
            if parsed.calls.iter().any(|c| c.tool_name == *tool) {
                recognized += 1;
            }
        }

        // 额外检查是否有多余的工具调用
        let extra_calls = parsed.calls.iter()
            .filter(|c| !expected_tools.contains(&c.tool_name.as_str()))
            .count();

        let precision = if parsed.calls.is_empty() {
            0.0
        } else {
            let correct = parsed.calls.iter()
                .filter(|c| expected_tools.contains(&c.tool_name.as_str()))
                .count();
            (correct as f64 / parsed.calls.len() as f64) * 100.0
        };

        let recall = (recognized as f64 / total.max(1) as f64) * 100.0;

        let score = (precision + recall) / 2.0;

        MetricScore {
            name: "工具识别".to_string(),
            score,
            precision,
            recall,
            details: format!(
                "识别: {}/{}, 精度: {:.1}%, 召回: {:.1}%, 多余: {}",
                recognized, total, precision, recall, extra_calls
            ),
        }
    }

    fn evaluate_param_extraction(&self, parsed: &ParsedToolCalls, expected: &TaskPlan) -> MetricScore {
        let mut total_params = 0;
        let mut correct_params = 0;

        for step in &expected.steps {
            for (expected_name, expected_value) in &step.expected_params {
                total_params += 1;

                // 在解析结果中查找对应的工具调用
                if let Some(call) = parsed.calls.iter()
                    .find(|c| c.tool_name == step.tool_name)
                {
                    if call.parameters.iter().any(|(name, value)| {
                        name == expected_name &&
                        (value.contains(expected_value) || expected_value.contains(value))
                    }) {
                        correct_params += 1;
                    }
                }
            }
        }

        let score = if total_params > 0 {
            (correct_params as f64 / total_params as f64) * 100.0
        } else {
            100.0
        };

        MetricScore {
            name: "参数提取".to_string(),
            score,
            precision: score,
            recall: score,
            details: format!("正确: {}/{}, 得分: {:.1}", correct_params, total_params, score),
        }
    }

    fn evaluate_parallel_recognition(&self, parsed: &ParsedToolCalls, expected: &TaskPlan) -> MetricScore {
        let mut score: f64 = 0.0;

        // 检查是否正确识别出并行调用
        let expected_has_parallel = expected.can_parallel && expected.steps.len() > 1;
        let actual_has_parallel = parsed.calls.len() > 1;

        if expected_has_parallel && actual_has_parallel {
            score += 50.0;  // 正确识别可以并行
        } else if !expected_has_parallel && !actual_has_parallel {
            score += 50.0;  // 正确识别不能并行
        } else if expected_has_parallel && !actual_has_parallel {
            score += 20.0;  // 未识别出并行机会
        }

        // 检查并行组的数量
        if expected.can_parallel && expected.parallel_groups.len() > 0 {
            let expected_groups = expected.parallel_groups.len();
            // 简化处理：如果调用数大于等于2，认为识别了并行
            let actual_groups = if parsed.calls.len() >= 2 { 1 } else { 0 };

            if expected_groups == actual_groups {
                score += 50.0;
            } else {
                score += 25.0;
            }
        } else {
            score += 50.0;  // 不适用，给满分
        }

        MetricScore {
            name: "并行识别".to_string(),
            score: score.min(100.0),
            precision: score,
            recall: score,
            details: format!(
                "期望并行: {}, 实际并行: {}, 组数: {}/{}",
                expected_has_parallel, actual_has_parallel,
                if expected.can_parallel { expected.parallel_groups.len() } else { 0 },
                if expected.can_parallel { expected.parallel_groups.len() } else { 0 }
            ),
        }
    }

    fn evaluate_planning_quality(&self, parsed: &ParsedToolCalls, expected: &TaskPlan) -> MetricScore {
        let mut score: f64 = 0.0;

        // 检查是否按正确顺序排列（对于有依赖的任务）
        if !expected.steps.is_empty() {
            let first_step = &expected.steps[0];

            // 检查第一步是否被正确识别
            if parsed.calls.iter().any(|c| c.tool_name == first_step.tool_name) {
                score += 40.0;
            }

            // 检查依赖关系是否被正确处理
            if !expected.can_parallel {
                // 对于顺序任务，检查是否按顺序列出了工具
                let mut in_order = true;
                let mut prev_index = usize::MAX;

                for step in expected.steps.iter() {
                    if let Some(pos) = parsed.calls.iter()
                        .position(|c| c.tool_name == step.tool_name)
                    {
                        if pos < prev_index {
                            in_order = false;
                            break;
                        }
                        prev_index = pos;
                    }
                }

                if in_order && parsed.calls.len() == expected.steps.len() {
                    score += 30.0;
                } else if parsed.calls.len() >= expected.steps.len() {
                    score += 15.0;
                }
            } else {
                // 对于并行任务，检查是否识别出并行机会
                if parsed.calls.len() > 1 {
                    score += 30.0;
                } else {
                    score += 10.0;
                }
            }
        }

        // 检查是否产生了不必要的工具调用
        let expected_count = expected.steps.len();
        let actual_count = parsed.calls.len();

        if actual_count == expected_count {
            score += 30.0;
        } else if actual_count < expected_count {
            score += 15.0;  // 遗漏了一些工具
        } else {
            score += 10.0;  // 产生了多余的调用
        }

        MetricScore {
            name: "规划质量".to_string(),
            score: score.min(100.0),
            precision: score,
            recall: score,
            details: format!(
                "规划完整性: {}/{}",
                actual_count, expected_count
            ),
        }
    }

    fn print_scenario_result(&self, result: &ScenarioResult) {
        println!("\n   📊 评估结果:");
        println!("   ─────────────────────────────────────────────────────────");
        println!("   工具识别: {:.1} - {}", result.tool_recognition.score, result.tool_recognition.details);
        println!("   参数提取: {:.1} - {}", result.param_extraction.score, result.param_extraction.details);
        println!("   并行识别: {:.1} - {}", result.parallel_recognition.score, result.parallel_recognition.details);
        println!("   规划质量: {:.1} - {}", result.planning_quality.score, result.planning_quality.details);
        println!("   ─────────────────────────────────────────────────────────");
        println!("   综合得分: {:.1}/100", result.overall_score);

        println!("\n   🔍 解析到的工具调用:");
        if result.parsed_calls.calls.is_empty() {
            println!("   (无)");
        } else {
            for (i, call) in result.parsed_calls.calls.iter().enumerate() {
                println!("   {}. {} (置信度: {:.1})", i + 1, call.tool_name, call.confidence);
                if !call.parameters.is_empty() {
                    println!("      参数: {:?}", call.parameters);
                }
            }
        }
    }

    fn generate_final_report(&self, results: Vec<ScenarioResult>) -> ToolCallingEvaluationReport {
        let total_scenarios = results.len();
        let total_score: f64 = results.iter()
            .map(|r| r.overall_score)
            .sum::<f64>() / total_scenarios.max(1) as f64;

        let tool_recognition_avg = results.iter()
            .map(|r| r.tool_recognition.score)
            .sum::<f64>() / total_scenarios.max(1) as f64;

        let param_extraction_avg = results.iter()
            .map(|r| r.param_extraction.score)
            .sum::<f64>() / total_scenarios.max(1) as f64;

        let parallel_recognition_avg = results.iter()
            .map(|r| r.parallel_recognition.score)
            .sum::<f64>() / total_scenarios.max(1) as f64;

        let planning_quality_avg = results.iter()
            .map(|r| r.planning_quality.score)
            .sum::<f64>() / total_scenarios.max(1) as f64;

        let avg_response_time = results.iter()
            .map(|r| r.response_time_ms)
            .sum::<u128>() / results.len() as u128;

        ToolCallingEvaluationReport {
            model_name: self.model_name.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            total_scenarios,
            results,
            tool_recognition_avg,
            param_extraction_avg,
            parallel_recognition_avg,
            planning_quality_avg,
            overall_score: total_score,
            avg_response_time_ms: avg_response_time,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricScore {
    pub name: String,
    pub score: f64,
    pub precision: f64,
    pub recall: f64,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_name: String,
    pub user_request: String,
    pub llm_response: String,
    pub response_time_ms: u128,
    pub parsed_calls: ParsedToolCalls,
    pub tool_recognition: MetricScore,
    pub param_extraction: MetricScore,
    pub parallel_recognition: MetricScore,
    pub planning_quality: MetricScore,
    pub overall_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallingEvaluationReport {
    pub model_name: String,
    pub timestamp: i64,
    pub total_scenarios: usize,
    pub results: Vec<ScenarioResult>,
    pub tool_recognition_avg: f64,
    pub param_extraction_avg: f64,
    pub parallel_recognition_avg: f64,
    pub planning_quality_avg: f64,
    pub overall_score: f64,
    pub avg_response_time_ms: u128,
}

// ============================================================================
// 报告打印
// ============================================================================

pub fn print_tool_calling_report(reports: &[ToolCallingEvaluationReport]) {
    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   任务规划与工具调用评估报告                                           ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    println!("\n📊 模型对比:");
    println!("────────────────────────────────────────────────────────────────");
    println!("{:<20} | {:>10} | {:>10} | {:>10} | {:>10} | {:>10}",
        "模型", "工具识别", "参数提取", "并行识别", "规划质量", "综合分");
    println!("────────────────────────────────────────────────────────────────");

    let mut sorted_reports = reports.to_vec();
    sorted_reports.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap());

    for report in &sorted_reports {
        println!("{:<20} | {:>9.1} | {:>9.1} | {:>9.1} | {:>9.1} | {:>9.1}",
            report.model_name,
            report.tool_recognition_avg,
            report.param_extraction_avg,
            report.parallel_recognition_avg,
            report.planning_quality_avg,
            report.overall_score
        );
    }

    // 最佳模型分析
    if let Some(best) = sorted_reports.first() {
        println!("\n🏆 最佳模型: {}", best.model_name);
        println!("   综合得分: {:.1}/100", best.overall_score);
    }

    // 各维度最佳模型
    println!("\n📈 各维度最佳表现:");
    for dim_name in ["工具识别", "参数提取", "并行识别", "规划质量"] {
        let best = reports.iter()
            .max_by(|a, b| {
                let a_val = match dim_name {
                    "工具识别" => a.tool_recognition_avg,
                    "参数提取" => a.param_extraction_avg,
                    "并行识别" => a.parallel_recognition_avg,
                    "规划质量" => a.planning_quality_avg,
                    _ => 0.0,
                };
                let b_val = match dim_name {
                    "工具识别" => b.tool_recognition_avg,
                    "参数提取" => b.param_extraction_avg,
                    "并行识别" => b.parallel_recognition_avg,
                    "规划质量" => b.planning_quality_avg,
                    _ => 0.0,
                };
                a_val.partial_cmp(&b_val).unwrap()
            });

        if let Some(best_report) = best {
            let best_val = match dim_name {
                "工具识别" => best_report.tool_recognition_avg,
                "参数提取" => best_report.param_extraction_avg,
                "并行识别" => best_report.parallel_recognition_avg,
                "规划质量" => best_report.planning_quality_avg,
                _ => 0.0,
            };
            println!("   {}: {} ({:.1})", dim_name, best_report.model_name, best_val);
        }
    }
}

// ============================================================================
// 测试入口
// ============================================================================

#[tokio::test]
async fn test_tool_calling_capabilities() {
    let models = vec![
        "qwen3:1.7b",
        "qwen3:0.6b",
        "deepseek-r1:1.5b",
    ];

    let mut reports = Vec::new();

    for model in models {
        match ToolCallingEvaluator::new(model) {
            Ok(evaluator) => {
                let report = evaluator.evaluate_all().await;
                reports.push(report);
            }
            Err(e) => {
                println!("⚠️  无法测试模型 {}: {}", model, e);
            }
        }
    }

    print_tool_calling_report(&reports);
}

#[tokio::test]
async fn test_single_model_tool_calling() {
    let model = "qwen3:1.7b";

    match ToolCallingEvaluator::new(model) {
        Ok(evaluator) => {
            let report = evaluator.evaluate_all().await;
            print_tool_calling_report(&[report]);
        }
        Err(e) => {
            println!("⚠️  无法测试模型 {}: {}", model, e);
        }
    }
}

// ============================================================================
// 中英文对比测试
// ============================================================================

#[tokio::test]
async fn test_language_comparison() {
    let model = "qwen3:1.7b";
    let mut reports = Vec::new();

    let separator = "═".repeat(80);

    // 测试中文
    println!("\n\n{}", separator);
    println!("中文测试 / Chinese Test");
    println!("{}", separator);
    match ToolCallingEvaluator::new_with_language(model, TestLanguage::Chinese) {
        Ok(evaluator) => {
            let report = evaluator.evaluate_all().await;
            reports.push(report);
        }
        Err(e) => {
            println!("⚠️  无法测试模型 {}: {}", model, e);
        }
    }

    // 测试英文
    println!("\n\n{}", separator);
    println!("英文测试 / English Test");
    println!("{}", separator);
    match ToolCallingEvaluator::new_with_language(model, TestLanguage::English) {
        Ok(evaluator) => {
            let mut report = evaluator.evaluate_all().await;
            // 修改模型名称以区分语言
            report.model_name = format!("{} (English)", model);
            reports.push(report);
        }
        Err(e) => {
            println!("⚠️  无法测试模型 {}: {}", model, e);
        }
    }

    // 打印对比报告
    print_language_comparison_report(&reports);
}

pub fn print_language_comparison_report(reports: &[ToolCallingEvaluationReport]) {
    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   中英文对比评估报告 / Language Comparison Report                         ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    println!("\n📊 语言对比:");
    println!("────────────────────────────────────────────────────────────────");
    println!("{:<25} | {:>10} | {:>10} | {:>10} | {:>10} | {:>10}",
        "语言/Language", "工具识别", "参数提取", "并行识别", "规划质量", "综合分");
    println!("────────────────────────────────────────────────────────────────");

    for report in reports {
        println!("{:<25} | {:>9.1} | {:>9.1} | {:>9.1} | {:>9.1} | {:>9.1}",
            report.model_name,
            report.tool_recognition_avg,
            report.param_extraction_avg,
            report.parallel_recognition_avg,
            report.planning_quality_avg,
            report.overall_score
        );
    }

    // 计算差异
    if reports.len() >= 2 {
        let zh = &reports[0];
        let en = &reports[1];

        println!("\n📈 差异分析:");
        println!("────────────────────────────────────────────────────────────────");
        let tool_diff = en.tool_recognition_avg - zh.tool_recognition_avg;
        let param_diff = en.param_extraction_avg - zh.param_extraction_avg;
        let parallel_diff = en.parallel_recognition_avg - zh.parallel_recognition_avg;
        let quality_diff = en.planning_quality_avg - zh.planning_quality_avg;
        let overall_diff = en.overall_score - zh.overall_score;

        println!("工具识别: {:+.1} ({:.1} → {:.1})",
            tool_diff, zh.tool_recognition_avg, en.tool_recognition_avg);
        println!("参数提取: {:+.1} ({:.1} → {:.1})",
            param_diff, zh.param_extraction_avg, en.param_extraction_avg);
        println!("并行识别: {:+.1} ({:.1} → {:.1})",
            parallel_diff, zh.parallel_recognition_avg, en.parallel_recognition_avg);
        println!("规划质量: {:+.1} ({:.1} → {:.1})",
            quality_diff, zh.planning_quality_avg, en.planning_quality_avg);
        println!("综合得分: {:+.1} ({:.1} → {:.1})",
            overall_diff, zh.overall_score, en.overall_score);

        let better_lang = if overall_diff > 0.0 { "英文" } else { "中文" };
        println!("\n🏆 结论: {}表现更好 ({:.1}分差异)", better_lang, overall_diff.abs());
    }
}

