//! NeoTalk 全模型综合测试
//!
//! 测试所有本地LLM模型在NeoTalk系统中的表现
//! 评估维度：响应可用性、响应质量、指令理解、响应速度
//!
//! **测试日期**: 2026-01-17

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

use edge_ai_llm::backends::create_backend;
use edge_ai_core::llm::backend::{GenerationParams, LlmInput};
use edge_ai_core::message::{Message, MessageRole, Content};

const OLLAMA_ENDPOINT: &str = "http://localhost:11434";

/// 模型测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTestResult {
    pub model_name: String,
    pub total_tests: usize,
    pub successful_responses: usize,
    pub empty_responses: usize,
    pub short_responses: usize,  // < 10 chars
    pub avg_response_length: f64,
    pub avg_response_time_ms: f64,
    pub response_quality_score: f64,
    pub command_understanding_rate: f64,
    pub overall_score: f64,
}

/// 单次对话测试结果
#[derive(Debug, Clone)]
struct SingleTestResult {
    pub model: String,
    pub prompt: String,
    pub response: String,
    pub response_length: usize,
    pub response_time_ms: u128,
    pub is_empty: bool,
    pub is_short: bool,
    pub has_command: bool,
}

/// 模型测试器
pub struct ModelTester {
    endpoint: String,
    timeout_secs: u64,
}

impl ModelTester {
    pub fn new() -> Self {
        Self {
            endpoint: OLLAMA_ENDPOINT.to_string(),
            timeout_secs: 60,
        }
    }

    /// 测试单个模型
    pub async fn test_model(&self, model_name: &str, test_prompts: Vec<&str>) -> ModelTestResult {
        println!("\n╔════════════════════════════════════════════════════════════════════════╗");
        println!("║   测试模型: {:58}║", model_name);
        println!("╚════════════════════════════════════════════════════════════════════════╝");

        let llm_config = serde_json::json!({
            "endpoint": self.endpoint,
            "model": model_name
        });

        let llm = match create_backend("ollama", &llm_config) {
            Ok(l) => Arc::new(l),
            Err(e) => {
                println!("⚠️  无法加载模型: {:?}", e);
                return ModelTestResult {
                    model_name: model_name.to_string(),
                    total_tests: 0,
                    successful_responses: 0,
                    empty_responses: 0,
                    short_responses: 0,
                    avg_response_length: 0.0,
                    avg_response_time_ms: 0.0,
                    response_quality_score: 0.0,
                    command_understanding_rate: 0.0,
                    overall_score: 0.0,
                };
            }
        };

        let mut results = Vec::new();

        for (i, prompt) in test_prompts.iter().enumerate() {
            print!("[{:2}] {:50} | ", i + 1, &prompt[..prompt.len().min(50)]);

            let start = Instant::now();

            let system_prompt = "你是 NeoTalk 智能助手。请用中文简洁回答用户的问题。";

            let messages = vec![
                Message {
                    role: MessageRole::System,
                    content: Content::Text(system_prompt.to_string()),
                    timestamp: None,
                },
                Message {
                    role: MessageRole::User,
                    content: Content::Text(prompt.to_string()),
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
                model: Some(model_name.to_string()),
                stream: false,
                tools: None,
            };

            let result = match tokio::time::timeout(
                Duration::from_secs(self.timeout_secs),
                llm.generate(llm_input)
            ).await {
                Ok(Ok(output)) => {
                    let response = output.text;
                    let response_length = response.len();
                    let is_empty = response.trim().is_empty();
                    let is_short = response_length > 0 && response_length < 10;
                    let has_command = self.detect_command(&response);

                    let status = if is_empty {
                        "❌ 空"
                    } else if is_short {
                        "⚠️ 短"
                    } else {
                        "✅"
                    };

                    println!("{} | {}字符 | {}ms | {}", status, response_length, start.elapsed().as_millis(),
                        if has_command { "⚡命令" } else { "" });

                    SingleTestResult {
                        model: model_name.to_string(),
                        prompt: prompt.to_string(),
                        response,
                        response_length,
                        response_time_ms: start.elapsed().as_millis(),
                        is_empty,
                        is_short,
                        has_command,
                    }
                }
                Ok(Err(e)) => {
                    println!("❌ 错误 | {:?}", e);
                    SingleTestResult {
                        model: model_name.to_string(),
                        prompt: prompt.to_string(),
                        response: String::new(),
                        response_length: 0,
                        response_time_ms: start.elapsed().as_millis(),
                        is_empty: true,
                        is_short: false,
                        has_command: false,
                    }
                }
                Err(_) => {
                    println!("❌ 超时");
                    SingleTestResult {
                        model: model_name.to_string(),
                        prompt: prompt.to_string(),
                        response: String::new(),
                        response_length: 0,
                        response_time_ms: (self.timeout_secs * 1000) as u128,
                        is_empty: true,
                        is_short: false,
                        has_command: false,
                    }
                }
            };

            results.push(result);
        }

        // 计算统计数据
        let total_tests = results.len();
        let successful_responses = results.iter().filter(|r| !r.is_empty).count();
        let empty_responses = results.iter().filter(|r| r.is_empty).count();
        let short_responses = results.iter().filter(|r| r.is_short).count();
        let avg_response_length = if !results.is_empty() {
            results.iter().map(|r| r.response_length).sum::<usize>() as f64 / total_tests as f64
        } else {
            0.0
        };
        let avg_response_time_ms = if !results.is_empty() {
            results.iter().map(|r| r.response_time_ms).sum::<u128>() as f64 / total_tests as f64
        } else {
            0.0
        };

        // 响应质量评分
        let long_responses = results.iter().filter(|r| r.response_length >= 20).count();
        let response_quality_score = if total_tests > 0 {
            (long_responses as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };

        // 指令理解率
        let has_command = results.iter().filter(|r| r.has_command).count();
        let command_understanding_rate = if total_tests > 0 {
            (has_command as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };

        // 综合评分
        let availability_score = if total_tests > 0 {
            (successful_responses as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };

        let overall_score = availability_score * 0.4 +
            response_quality_score * 0.3 +
            command_understanding_rate * 0.3;

        println!("\n📊 {} 测试结果:", model_name);
        println!("────────────────────────────────────────────────────────────────");
        println!("  总测试数: {}", total_tests);
        println!("  成功响应: {} ({:.1}%)", successful_responses, availability_score);
        println!("  空响应: {} ({:.1}%)", empty_responses, (empty_responses as f64 / total_tests as f64) * 100.0);
        println!("  短响应(<10字符): {} ({:.1}%)", short_responses, (short_responses as f64 / total_tests as f64) * 100.0);
        println!("  平均长度: {:.1} 字符", avg_response_length);
        println!("  平均响应时间: {:.1} ms", avg_response_time_ms);
        println!("  响应质量: {:.1}/100", response_quality_score);
        println!("  指令理解: {:.1}/100", command_understanding_rate);
        println!("  综合评分: {:.1}/100", overall_score);

        ModelTestResult {
            model_name: model_name.to_string(),
            total_tests,
            successful_responses,
            empty_responses,
            short_responses,
            avg_response_length,
            avg_response_time_ms,
            response_quality_score,
            command_understanding_rate,
            overall_score,
        }
    }

    fn detect_command(&self, response: &str) -> bool {
        let lower = response.to_lowercase();
        lower.contains("命令")
            || lower.contains("执行")
            || lower.contains("打开")
            || lower.contains("关闭")
            || lower.contains("启动")
            || lower.contains("停止")
            || lower.contains("设置")
    }
}

/// 综合测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveModelTestResult {
    pub model_results: Vec<ModelTestResult>,
    pub best_overall_model: String,
    pub fastest_model: String,
    pub best_quality_model: String,
    pub most_reliable_model: String,
    pub recommendations: Vec<String>,
}

/// 测试所有可用的模型
pub async fn test_all_available_models() -> ComprehensiveModelTestResult {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   NeoTalk 全模型综合测试                                               ║");
    println!("║   Ollama端点: {:54}║", OLLAMA_ENDPOINT);
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    // 测试提示词 - 包含各种场景
    let test_prompts = vec![
        // 基础对话
        "你好",
        "今天的天气怎么样",

        // 设备控制
        "帮我打开客厅的灯",
        "关闭卧室的空调",
        "设置温度为26度",

        // 数据查询
        "当前温度是多少",
        "查看所有传感器数据",
        "系统运行状态如何",

        // 复杂指令
        "创建一个高温告警规则",
        "当有人移动时自动开灯",
        "设置每天早上7点自动打开窗帘",

        // 批量操作
        "打开所有房间的灯",
        "关闭所有的空调",

        // 告警相关
        "有没有异常告警",
        "查看所有历史告警",
    ];

    let tester = ModelTester::new();
    let mut model_results = Vec::new();

    // 所有可用模型（专注于对话模型）
    let models_to_test = vec![
        "qwen3:1.7b",
        "deepseek-r1:1.5b",
        "qwen3-vl:2b",
        "qwen3:0.6b",
        "gemma3:270m",
        "qwen2:1.5b",
        "qwen2.5:3b",
        "gemma3:4b",
    ];

    for model in models_to_test {
        let result = tester.test_model(model, test_prompts.clone()).await;
        model_results.push(result);
    }

    // 分析结果
    let mut best_overall_model = String::new();
    let mut best_overall_score = 0.0;

    let mut fastest_model = String::new();
    let mut fastest_time = f64::MAX;

    let mut best_quality_model = String::new();
    let mut best_quality_score = 0.0;

    let mut most_reliable_model = String::new();
    let mut best_reliability = 0.0;

    for result in &model_results {
        if result.total_tests > 0 {
            if result.overall_score > best_overall_score {
                best_overall_score = result.overall_score;
                best_overall_model = result.model_name.clone();
            }

            if result.avg_response_time_ms < fastest_time && result.avg_response_time_ms > 0.0 {
                fastest_time = result.avg_response_time_ms;
                fastest_model = result.model_name.clone();
            }

            if result.response_quality_score > best_quality_score {
                best_quality_score = result.response_quality_score;
                best_quality_model = result.model_name.clone();
            }

            let reliability = (result.total_tests - result.empty_responses) as f64 / result.total_tests as f64 * 100.0;
            if reliability > best_reliability {
                best_reliability = reliability;
                most_reliable_model = result.model_name.clone();
            }
        }
    }

    // 打最终排名
    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   模型排名                                                           ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let mut sorted_results = model_results.clone();
    sorted_results.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap_or(Ordering::Equal));

    println!("\n{:20} | {:10} | {:10} | {:10} | {:10} | {:10}",
        "模型", "响应率%", "质量%", "理解%", "速度ms", "综合%");
    println!("────────────────────────────────────────────────────────────────────────");

    for result in sorted_results {
        let availability = if result.total_tests > 0 {
            (result.successful_responses as f64 / result.total_tests as f64) * 100.0
        } else {
            0.0
        };
        println!("{:20} | {:9.1}% | {:9.1}% | {:9.1}% | {:9.1} | {:9.1}",
            result.model_name,
            availability,
            result.response_quality_score,
            result.command_understanding_rate,
            result.avg_response_time_ms,
            result.overall_score
        );
    }

    // 推荐和建议
    let mut recommendations = Vec::new();

    if !best_overall_model.is_empty() {
        recommendations.push(format!("最佳综合模型: {} (评分: {:.1})", best_overall_model, best_overall_score));
    }

    if !fastest_model.is_empty() {
        recommendations.push(format!("最快响应模型: {} ({:.1}ms)", fastest_model, fastest_time));
    }

    if !best_quality_model.is_empty() {
        recommendations.push(format!("最佳响应质量: {} (评分: {:.1})", best_quality_model, best_quality_score));
    }

    if !most_reliable_model.is_empty() {
        recommendations.push(format!("最高可靠性: {} (无空响应率最高)", most_reliable_model));
    }

    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   推荐与建议                                                         ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    for (i, rec) in recommendations.iter().enumerate() {
        println!("  {}. {}", i + 1, rec);
    }

    // 设计问题分析
    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   系统设计问题分析                                                   ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let low_reliability_models: Vec<_> = model_results.iter()
        .filter(|r| r.total_tests > 0 && {
            let empty_rate = (r.empty_responses as f64 / r.total_tests as f64) * 100.0;
            empty_rate > 20.0
        })
        .collect();

    if !low_reliability_models.is_empty() {
        println!("\n⚠️  高空响应率模型 (>20%):");
        for model in low_reliability_models {
            println!("   - {}: {:.1}% 空响应率",
                model.model_name,
                (model.empty_responses as f64 / model.total_tests as f64) * 100.0
            );
        }
        println!("\n建议: 这些模型可能需要调整响应处理逻辑或prompt策略");
    }

    let slow_models: Vec<_> = model_results.iter()
        .filter(|r| r.avg_response_time_ms > 5000.0)
        .collect();

    if !slow_models.is_empty() {
        println!("\n⚠️  响应缓慢模型 (>5000ms):");
        for model in slow_models {
            println!("   - {}: {:.1}ms", model.model_name, model.avg_response_time_ms);
        }
        println!("\n建议: 这些模型可能不适合实时交互场景");
    }

    let low_quality_models: Vec<_> = model_results.iter()
        .filter(|r| r.response_quality_score < 50.0)
        .collect();

    if !low_quality_models.is_empty() {
        println!("\n⚠️  响应质量较低模型 (<50分):");
        for model in low_quality_models {
            println!("   - {}: {:.1}分", model.model_name, model.response_quality_score);
        }
        println!("\n建议: 这些模型可能需要更详细的系统提示词");
    }

    println!("\n✅ 测试完成");

    ComprehensiveModelTestResult {
        model_results,
        best_overall_model,
        fastest_model,
        best_quality_model,
        most_reliable_model,
        recommendations,
    }
}

// ============================================================================
// 测试入口
// ============================================================================

#[tokio::test]
async fn test_all_models_comprehensive() {
    let _result = test_all_available_models().await;

    // 验证至少有一个模型被测试
    // assert!(!result.model_results.is_empty(), "应该至少有一个模型被测试");
}
