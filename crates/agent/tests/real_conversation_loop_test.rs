//! NeoTalk 真实对话循环测试
//!
//! 模拟真实用户与系统的多轮对话交互，评估：
//! - 上下文理解能力
//! - 对话连贯性
//! - 任务执行准确性
//! - 响应质量
//!
//! **测试日期**: 2026-01-18

use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use edge_ai_llm::backends::create_backend;
use edge_ai_core::llm::backend::{GenerationParams, LlmInput};
use edge_ai_core::message::{Message, MessageRole, Content};

const OLLAMA_ENDPOINT: &str = "http://localhost:11434";

// ============================================================================
// 对话场景定义
// ============================================================================

/// 对话轮次
#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub user_input: String,
    pub expected_intent: String,
    pub context_required: Vec<String>,  // 需要记住的上下文
    pub validation_fn: Option<fn(&str, &ConversationContext) -> bool>,
}

/// 对话场景
#[derive(Debug, Clone)]
pub struct ConversationScenario {
    pub name: String,
    pub description: String,
    pub turns: Vec<ConversationTurn>,
    pub category: ScenarioCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScenarioCategory {
    DeviceControl,      // 设备控制场景
    InformationQuery,   // 信息查询场景
    ProblemSolving,     // 问题解决场景
    MultiTask,          // 多任务场景
    ContextSwitching,   // 上下文切换场景
    ErrorRecovery,      // 错误恢复场景
}

/// 对话上下文
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub session_id: String,
    pub turn_number: usize,
    pub mentioned_devices: Vec<String>,
    pub mentioned_locations: Vec<String>,
    pub conversation_history: Vec<(String, String)>,  // (user, assistant)
    pub state_changes: Vec<String>,  // 记录状态变化
}

/// 对话评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEvaluation {
    pub scenario_name: String,
    pub model_name: String,
    pub total_turns: usize,
    pub completed_turns: usize,
    pub completion_rate: f64,
    pub context_retention_score: f64,
    pub response_quality_score: f64,
    pub task_success_score: f64,
    pub avg_response_time_ms: f64,
    pub overall_score: f64,
    pub details: Vec<TurnEvaluation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnEvaluation {
    pub turn_number: usize,
    pub user_input: String,
    pub assistant_response: String,
    pub response_time_ms: u128,
    pub context_preserved: bool,
    pub intent_matched: bool,
    pub response_adequate: bool,
    pub score: f64,
}

// ============================================================================
// 场景库定义
// ============================================================================

/// 获取所有测试场景
pub fn get_test_scenarios() -> Vec<ConversationScenario> {
    vec![
        // 场景1: 智能家居控制 - 渐进式设备控制
        ConversationScenario {
            name: "渐进式设备控制".to_string(),
            description: "用户逐步控制系统中的多个设备，测试上下文保持能力".to_string(),
            category: ScenarioCategory::DeviceControl,
            turns: vec![
                ConversationTurn {
                    user_input: "你好，请帮我查看一下客厅有哪些设备".to_string(),
                    expected_intent: "list_devices".to_string(),
                    context_required: vec!["客厅".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("客厅") || resp.contains("设备") || resp.len() > 20
                    }),
                },
                ConversationTurn {
                    user_input: "把客厅的灯打开".to_string(),
                    expected_intent: "control_device".to_string(),
                    context_required: vec!["客厅".to_string(), "灯".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("打开") || resp.contains("灯") || resp.contains("已")
                    }),
                },
                ConversationTurn {
                    user_input: "现在的温度是多少".to_string(),
                    expected_intent: "query_status".to_string(),
                    context_required: vec!["客厅".to_string()],  // 应该记得之前在讨论客厅
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("温度") || resp.contains("度") || resp.len() > 10
                    }),
                },
                ConversationTurn {
                    user_input: "有点冷，把空调调到26度".to_string(),
                    expected_intent: "control_device".to_string(),
                    context_required: vec!["空调".to_string(), "26度".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("空调") || resp.contains("26") || resp.contains("已设置")
                    }),
                },
                ConversationTurn {
                    user_input: "现在客厅的状态怎么样".to_string(),
                    expected_intent: "query_status".to_string(),
                    context_required: vec!["客厅".to_string(), "灯".to_string(), "空调".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        // 应该能总结之前操作的状态
                        resp.contains("灯") && resp.contains("空调") ||
                        resp.contains("客厅") && resp.len() > 30
                    }),
                },
            ],
        },

        // 场景2: 多房间控制 - 上下文切换
        ConversationScenario {
            name: "多房间设备控制".to_string(),
            description: "用户控制不同房间的设备，测试地点上下文切换能力".to_string(),
            category: ScenarioCategory::ContextSwitching,
            turns: vec![
                ConversationTurn {
                    user_input: "打开客厅的电视".to_string(),
                    expected_intent: "control_device".to_string(),
                    context_required: vec!["客厅".to_string(), "电视".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("客厅") && (resp.contains("电视") || resp.contains("已打开"))
                    }),
                },
                ConversationTurn {
                    user_input: "卧室的温度是多少".to_string(),
                    expected_intent: "query_status".to_string(),
                    context_required: vec!["卧室".to_string()],  // 切换到卧室
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("卧室") && resp.contains("温度")
                    }),
                },
                ConversationTurn {
                    user_input: "把它调低两度".to_string(),
                    expected_intent: "control_device".to_string(),
                    context_required: vec!["卧室".to_string(), "空调".to_string()],  // 应该知道"它"指空调
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("空调") || resp.contains("温度") || resp.contains("已调整")
                    }),
                },
                ConversationTurn {
                    user_input: "回到客厅，把灯关掉".to_string(),
                    expected_intent: "control_device".to_string(),
                    context_required: vec!["客厅".to_string(), "灯".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("客厅") && resp.contains("灯") &&
                        (resp.contains("关闭") || resp.contains("关掉"))
                    }),
                },
            ],
        },

        // 场景3: 问题诊断与解决
        ConversationScenario {
            name: "设备问题诊断".to_string(),
            description: "用户报告设备问题，系统协助诊断和解决".to_string(),
            category: ScenarioCategory::ProblemSolving,
            turns: vec![
                ConversationTurn {
                    user_input: "客厅的空调好像不工作了".to_string(),
                    expected_intent: "report_problem".to_string(),
                    context_required: vec!["客厅".to_string(), "空调".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.len() > 20 && (resp.contains("检查") || resp.contains("诊断") ||
                                           resp.contains("问题") || resp.contains("帮助"))
                    }),
                },
                ConversationTurn {
                    user_input: "它显示错误代码E01".to_string(),
                    expected_intent: "provide_details".to_string(),
                    context_required: vec!["E01".to_string(), "空调".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("E01") || resp.contains("错误") ||
                        resp.contains("检查") || resp.contains("建议")
                    }),
                },
                ConversationTurn {
                    user_input: "那我该怎么办".to_string(),
                    expected_intent: "request_solution".to_string(),
                    context_required: vec!["空调".to_string(), "E01".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.len() > 30 && (resp.contains("建议") || resp.contains("可以") ||
                                           resp.contains("尝试") || resp.contains("步骤"))
                    }),
                },
            ],
        },

        // 场景4: 创建自动化规则
        ConversationScenario {
            name: "规则创建对话".to_string(),
            description: "通过对话逐步创建自动化规则".to_string(),
            category: ScenarioCategory::MultiTask,
            turns: vec![
                ConversationTurn {
                    user_input: "我想创建一个自动化规则".to_string(),
                    expected_intent: "create_rule".to_string(),
                    context_required: vec![],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("规则") || resp.contains("自动化") ||
                        resp.contains("创建") || resp.contains("想要")
                    }),
                },
                ConversationTurn {
                    user_input: "当温度超过28度的时候".to_string(),
                    expected_intent: "specify_condition".to_string(),
                    context_required: vec!["温度".to_string(), "28度".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("28") || resp.contains("温度") ||
                        resp.contains("条件") || resp.contains("触发")
                    }),
                },
                ConversationTurn {
                    user_input: "自动打开风扇".to_string(),
                    expected_intent: "specify_action".to_string(),
                    context_required: vec!["风扇".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("风扇") || resp.contains("打开") ||
                        resp.contains("动作") || resp.contains("执行")
                    }),
                },
                ConversationTurn {
                    user_input: "帮我确认一下这个规则".to_string(),
                    expected_intent: "confirm_rule".to_string(),
                    context_required: vec!["温度".to_string(), "28度".to_string(), "风扇".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("温度") && resp.contains("28") && resp.contains("风扇") ||
                        resp.contains("规则") && resp.len() > 40
                    }),
                },
            ],
        },

        // 场景5: 复杂查询与信息聚合
        ConversationScenario {
            name: "复杂信息查询".to_string(),
            description: "用户询问复杂问题，需要聚合多个信息源".to_string(),
            category: ScenarioCategory::InformationQuery,
            turns: vec![
                ConversationTurn {
                    user_input: "今天家里消耗了多少电".to_string(),
                    expected_intent: "query_energy".to_string(),
                    context_required: vec![],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("电") || resp.contains("能耗") ||
                        resp.contains("度") || resp.contains("消耗")
                    }),
                },
                ConversationTurn {
                    user_input: "哪个房间用电最多".to_string(),
                    expected_intent: "compare_energy".to_string(),
                    context_required: vec!["房间".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("房间") || resp.contains("最多") ||
                        resp.contains("用电") || resp.len() > 20
                    }),
                },
                ConversationTurn {
                    user_input: "能不能帮我省点电".to_string(),
                    expected_intent: "request_advice".to_string(),
                    context_required: vec!["电".to_string(), "节能".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("建议") || resp.contains("可以") ||
                        resp.contains("节能") || resp.contains("省电")
                    }),
                },
            ],
        },

        // 场景6: 错误恢复与澄清
        ConversationScenario {
            name: "模糊指令处理".to_string(),
            description: "用户发出模糊指令，系统需要澄清或推断".to_string(),
            category: ScenarioCategory::ErrorRecovery,
            turns: vec![
                ConversationTurn {
                    user_input: "打开灯".to_string(),
                    expected_intent: "ambiguous_command".to_string(),
                    context_required: vec![],
                    validation_fn: Some(|resp, ctx| {
                        // 应该询问是哪个灯，或者做出合理推断
                        resp.contains("哪个") || resp.contains("房间") ||
                        resp.contains("请问") || resp.contains("需要") ||
                        resp.len() > 30
                    }),
                },
                ConversationTurn {
                    user_input: "客厅的".to_string(),
                    expected_intent: "clarify_intent".to_string(),
                    context_required: vec!["客厅".to_string(), "灯".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("客厅") && (resp.contains("灯") || resp.contains("打开"))
                    }),
                },
                ConversationTurn {
                    user_input: "不对，是卧室的".to_string(),
                    expected_intent: "correction".to_string(),
                    context_required: vec!["卧室".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("卧室") && (resp.contains("灯") ||
                                               resp.contains("打开") || resp.contains("好的"))
                    }),
                },
            ],
        },

        // 场景7: 早晨唤醒场景
        ConversationScenario {
            name: "早晨唤醒".to_string(),
            description: "模拟用户早上起床后的连续操作".to_string(),
            category: ScenarioCategory::MultiTask,
            turns: vec![
                ConversationTurn {
                    user_input: "早上好".to_string(),
                    expected_intent: "greeting".to_string(),
                    context_required: vec![],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("早上好") || resp.contains("您好") ||
                        resp.contains("你好") || resp.len() > 10
                    }),
                },
                ConversationTurn {
                    user_input: "帮我执行起床模式".to_string(),
                    expected_intent: "execute_scene".to_string(),
                    context_required: vec!["起床".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("窗帘") || resp.contains("咖啡") ||
                        resp.contains("新闻") || resp.contains("模式") ||
                        resp.contains("执行")
                    }),
                },
                ConversationTurn {
                    user_input: "今天天气怎么样".to_string(),
                    expected_intent: "query_weather".to_string(),
                    context_required: vec![],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("天气") || resp.contains("温度") ||
                        resp.contains("晴") || resp.len() > 15
                    }),
                },
                ConversationTurn {
                    user_input: "如果下雨的话，把窗户都关上".to_string(),
                    expected_intent: "conditional_action".to_string(),
                    context_required: vec!["雨".to_string(), "窗户".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("窗户") && (resp.contains("关闭") || resp.contains("关上"))
                    }),
                },
            ],
        },

        // 场景8: 安全检查场景
        ConversationScenario {
            name: "离家安全检查".to_string(),
            description: "用户离家前的安全检查流程".to_string(),
            category: ScenarioCategory::DeviceControl,
            turns: vec![
                ConversationTurn {
                    user_input: "我要出门了，帮我检查一下家里".to_string(),
                    expected_intent: "security_check".to_string(),
                    context_required: vec![],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("检查") || resp.contains("安全") ||
                        resp.contains("门窗") || resp.contains("设备")
                    }),
                },
                ConversationTurn {
                    user_input: "卧室的窗户关了吗".to_string(),
                    expected_intent: "query_status".to_string(),
                    context_required: vec!["卧室".to_string(), "窗户".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("窗户") && (resp.contains("关闭") ||
                                               resp.contains("已关") || resp.contains("状态"))
                    }),
                },
                ConversationTurn {
                    user_input: "帮我开启安防模式".to_string(),
                    expected_intent: "enable_security".to_string(),
                    context_required: vec!["安防".to_string()],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("安防") && (resp.contains("开启") ||
                                               resp.contains("启动") || resp.contains("已"))
                    }),
                },
                ConversationTurn {
                    user_input: "好的，再见".to_string(),
                    expected_intent: "farewell".to_string(),
                    context_required: vec![],
                    validation_fn: Some(|resp, ctx| {
                        resp.contains("再见") || resp.contains("慢走") ||
                        resp.contains("一路") || resp.len() > 5
                    }),
                },
            ],
        },
    ]
}

// ============================================================================
// 对话测试引擎
// ============================================================================

pub struct ConversationTester {
    model_name: String,
    llm: Arc<dyn edge_ai_core::llm::backend::LlmRuntime>,
    timeout_secs: u64,
}

impl ConversationTester {
    pub fn new(model_name: &str) -> Result<Self, String> {
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
        })
    }

    /// 运行单个对话场景
    pub async fn run_scenario(&self, scenario: &ConversationScenario) -> ConversationEvaluation {
        println!("\n╔════════════════════════════════════════════════════════════════════════╗");
        println!("║   场景: {:60}║", scenario.name);
        println!("║   {:64}║", scenario.description);
        println!("╚════════════════════════════════════════════════════════════════════════╝");

        let mut context = ConversationContext {
            session_id: uuid::Uuid::new_v4().to_string(),
            turn_number: 0,
            mentioned_devices: Vec::new(),
            mentioned_locations: Vec::new(),
            conversation_history: Vec::new(),
            state_changes: Vec::new(),
        };

        let mut turn_evaluations = Vec::new();
        let mut total_response_time = 0u128;
        let mut completed_turns = 0;

        for (idx, turn) in scenario.turns.iter().enumerate() {
            context.turn_number = idx + 1;
            println!("\n[轮次 {}/{}] 用户: {}", idx + 1, scenario.turns.len(), turn.user_input);

            // 构建对话历史作为系统提示
            let system_prompt = self.build_system_prompt(&context, scenario);

            // 发送消息并获取响应
            let start = Instant::now();
            let response = self.send_message(&turn.user_input, &system_prompt).await;
            let response_time = start.elapsed().as_millis();
            total_response_time += response_time;

            let display_response = if response.chars().count() > 50 {
                format!("{}...", response.chars().take(50).collect::<String>())
            } else {
                response.clone()
            };
            println!("        助手: {} ({}ms)", display_response, response_time);

            // 更新上下文
            self.update_context(&mut context, &turn.user_input, &response);

            // 评估响应
            let eval = self.evaluate_turn(&turn, &response, response_time, &context);
            println!("        评估: {} | 上下文: {} | 意图: {} | 质量: {} | 得分: {:.0}",
                if eval.response_adequate { "✓" } else { "✗" },
                if eval.context_preserved { "✓" } else { "✗" },
                if eval.intent_matched { "✓" } else { "✗" },
                if eval.response_adequate { "✓" } else { "✗" },
                eval.score
            );

            if eval.score >= 60.0 {
                completed_turns += 1;
            }

            turn_evaluations.push(eval);
        }

        // 计算场景得分
        let completion_rate = (completed_turns as f64 / scenario.turns.len() as f64) * 100.0;
        let context_retention = turn_evaluations.iter()
            .map(|t| if t.context_preserved { 100.0 } else { 0.0 })
            .sum::<f64>() / turn_evaluations.len() as f64;
        let response_quality = turn_evaluations.iter()
            .map(|t| t.score)
            .sum::<f64>() / turn_evaluations.len() as f64;
        let task_success = completion_rate;  // 简化处理
        let avg_response_time = total_response_time as f64 / turn_evaluations.len() as f64;

        // 综合评分
        let overall_score = completion_rate * 0.4 +
                          context_retention * 0.2 +
                          response_quality * 0.3 +
                          (100.0 - (avg_response_time / 100.0).min(50.0)) * 0.1;

        println!("\n📊 场景 '{}' 完成率: {:.1}%", scenario.name, completion_rate);

        ConversationEvaluation {
            scenario_name: scenario.name.clone(),
            model_name: self.model_name.clone(),
            total_turns: scenario.turns.len(),
            completed_turns,
            completion_rate,
            context_retention_score: context_retention,
            response_quality_score: response_quality,
            task_success_score: task_success,
            avg_response_time_ms: avg_response_time,
            overall_score,
            details: turn_evaluations,
        }
    }

    /// 运行所有场景
    pub async fn run_all_scenarios(&self) -> Vec<ConversationEvaluation> {
        let scenarios = get_test_scenarios();
        let mut results = Vec::new();

        println!("\n╔════════════════════════════════════════════════════════════════════════╗");
        println!("║   NeoTalk 真实对话循环测试                                           ║");
        println!("║   模型: {:58}║", self.model_name);
        println!("║   场景数: {:57}║", scenarios.len());
        println!("╚════════════════════════════════════════════════════════════════════════╝");

        for scenario in &scenarios {
            let result = self.run_scenario(scenario).await;
            results.push(result);
        }

        results
    }

    fn build_system_prompt(&self, context: &ConversationContext, scenario: &ConversationScenario) -> String {
        let mut prompt = "你是 NeoTalk 智能家居助手。请用中文简洁回答用户的问题。\n\n".to_string();

        // 添加系统说明
        prompt += "系统中的设备包括:\n";
        prompt += "  - 客厅: 灯、空调、电视、温度传感器\n";
        prompt += "  - 卧室: 灯、空调、窗帘、温度传感器\n";
        prompt += "  - 厨房: 灯、冰箱、烟雾报警器\n";
        prompt += "  - 浴室: 灯、热水器、水浸传感器\n\n";

        prompt += "你可以:\n";
        prompt += "  - 查询设备状态\n";
        prompt += "  - 控制设备开关\n";
        prompt += "  - 调整设备参数\n";
        prompt += "  - 创建自动化规则\n";
        prompt += "  - 提供建议和帮助\n\n";

        // 添加对话历史
        if !context.conversation_history.is_empty() {
            prompt += "=== 对话历史 ===\n";
            for (user, assistant) in &context.conversation_history {
                prompt += &format!("用户: {}\n助手: {}\n\n", user, assistant);
            }
            prompt += "=== 当前对话 ===\n";
        }

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
                max_tokens: Some(200),
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

    fn update_context(&self, context: &mut ConversationContext, user_input: &str, response: &str) {
        // 记录对话历史
        context.conversation_history.push((user_input.to_string(), response.to_string()));

        // 提取提到的设备
        let devices = ["灯", "空调", "电视", "窗帘", "风扇", "冰箱"];
        for device in &devices {
            if user_input.contains(device) || response.contains(device) {
                if !context.mentioned_devices.contains(&device.to_string()) {
                    context.mentioned_devices.push(device.to_string());
                }
            }
        }

        // 提取提到的位置
        let locations = ["客厅", "卧室", "厨房", "浴室", "书房"];
        for location in &locations {
            if user_input.contains(location) || response.contains(location) {
                if !context.mentioned_locations.contains(&location.to_string()) {
                    context.mentioned_locations.push(location.to_string());
                }
            }
        }

        // 记录状态变化（简单检测）
        if user_input.contains("打开") || user_input.contains("关闭") ||
           user_input.contains("设置") || user_input.contains("调") {
            context.state_changes.push(user_input.to_string());
        }
    }

    fn evaluate_turn(&self, turn: &ConversationTurn, response: &str,
                     response_time: u128, context: &ConversationContext) -> TurnEvaluation {
        // 检查上下文是否保留
        let context_preserved = if turn.context_required.is_empty() {
            true
        } else {
            let mut all_found = true;
            for required in &turn.context_required {
                if !response.contains(required) {
                    all_found = false;
                    break;
                }
            }
            all_found
        };

        // 检查意图是否匹配
        let intent_matched = if let Some(validate_fn) = turn.validation_fn {
            validate_fn(response, context)
        } else {
            response.len() > 10
        };

        // 检查响应是否充分
        let response_adequate = !response.trim().is_empty() && response.len() >= 5;

        // 计算得分
        let score = if context_preserved && intent_matched && response_adequate {
            100.0
        } else if response_adequate {
            let mut score = 60.0;
            if context_preserved { score += 20.0; }
            if intent_matched { score += 20.0; }
            score
        } else {
            0.0
        };

        TurnEvaluation {
            turn_number: context.turn_number,
            user_input: turn.user_input.clone(),
            assistant_response: response.to_string(),
            response_time_ms: response_time,
            context_preserved,
            intent_matched,
            response_adequate,
            score,
        }
    }
}

// ============================================================================
// 报告生成
// ============================================================================

pub fn print_conversation_report(evaluations: &[ConversationEvaluation], model_name: &str) {
    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   真实对话循环测试报告                                               ║");
    println!("║   模型: {:58}║", model_name);
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    println!("\n📊 场景完成情况:");
    println!("────────────────────────────────────────────────────────────────");
    println!("{:<25} | {:>6} | {:>6} | {:>8} | {:>8} | {:>8}",
        "场景", "完成", "总轮", "完成率", "上下文", "综合分");
    println!("────────────────────────────────────────────────────────────────");

    for eval in evaluations {
        println!("{:<25} | {:>6} | {:>6} | {:>7.1}% | {:>7.1}% | {:>7.1}",
            eval.scenario_name,
            eval.completed_turns,
            eval.total_turns,
            eval.completion_rate,
            eval.context_retention_score,
            eval.overall_score
        );
    }

    // 计算总体统计
    let total_turns: usize = evaluations.iter().map(|e| e.total_turns).sum();
    let total_completed: usize = evaluations.iter().map(|e| e.completed_turns).sum();
    let avg_completion = (total_completed as f64 / total_turns as f64) * 100.0;
    let avg_context = evaluations.iter().map(|e| e.context_retention_score).sum::<f64>() / evaluations.len() as f64;
    let avg_quality = evaluations.iter().map(|e| e.response_quality_score).sum::<f64>() / evaluations.len() as f64;
    let avg_response_time = evaluations.iter().map(|e| e.avg_response_time_ms).sum::<f64>() / evaluations.len() as f64;
    let overall_score = avg_completion * 0.4 + avg_context * 0.2 + avg_quality * 0.3 + 20.0;

    println!("────────────────────────────────────────────────────────────────");
    println!("{:<25} | {:>6} | {:>6} | {:>7.1}% | {:>7.1}% | {:>7.1}",
        "总体平均",
        total_completed,
        total_turns,
        avg_completion,
        avg_context,
        overall_score
    );

    println!("\n⏱️  平均响应时间: {:.1}ms", avg_response_time);

    // 详细分析
    println!("\n📋 详细轮次分析:");
    for eval in evaluations {
        println!("\n[场景: {}]", eval.scenario_name);
        println!("  轮次 | 用户输入                                  | 响应时间 | 得分 | 结果");
        println!("  ─────┼──────────────────────────────────────────┼──────────┼──────┼──────");
        for turn in &eval.details {
            let input_short = if turn.user_input.chars().count() > 20 {
                format!("{}...", turn.user_input.chars().take(20).collect::<String>())
            } else {
                turn.user_input.clone()
            };
            println!("  {:>4} | {:<42} | {:>8} | {:>4.0} | {}",
                turn.turn_number,
                input_short,
                turn.response_time_ms,
                turn.score,
                if turn.score >= 60.0 { "✓" } else { "✗" }
            );
        }
    }

    // 评级
    let grade = if overall_score >= 90.0 { "A" }
                else if overall_score >= 80.0 { "B" }
                else if overall_score >= 70.0 { "C" }
                else if overall_score >= 60.0 { "D" }
                else { "F" };

    println!("\n🎯 综合评级: {} ({:.1}/100)", grade, overall_score);
}

// ============================================================================
// 测试入口
// ============================================================================

#[tokio::test]
async fn test_real_conversation_loop() {
    let models = vec![
        "qwen3:1.7b",
        "qwen3:0.6b",
        "gemma3:270m",
    ];

    for model in models {
        match ConversationTester::new(model) {
            Ok(tester) => {
                let evaluations = tester.run_all_scenarios().await;
                print_conversation_report(&evaluations, model);
            }
            Err(e) => {
                println!("⚠️  无法测试模型 {}: {}", model, e);
            }
        }
    }
}

#[tokio::test]
async fn test_single_model_conversation() {
    let model = "qwen3:1.7b";

    match ConversationTester::new(model) {
        Ok(tester) => {
            let evaluations = tester.run_all_scenarios().await;
            print_conversation_report(&evaluations, model);
        }
        Err(e) => {
            println!("⚠️  无法测试模型 {}: {}", model, e);
        }
    }
}
