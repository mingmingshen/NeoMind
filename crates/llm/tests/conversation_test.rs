//! Test conversation flow with complex and multiple questions
//!
//! Run with: cargo test -p edge-ai-llm --test conversation_test -- --nocapture

use std::io::Write;
use std::sync::Arc;
use edge_ai_llm::{OllamaConfig, OllamaRuntime};
use edge_ai_core::{
    llm::backend::{LlmRuntime, LlmInput, GenerationParams},
    Message,
};
use futures::StreamExt;

#[tokio::test]
async fn test_complex_conversations() {
    // Initialize logging (use try_init to avoid panic if already set)
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .try_init();

    println!("\n{:=^70}", "");
    println!(" COMPLEX CONVERSATION TEST - Multiple & Long Questions");
    println!("{:=^70}\n", "");

    // Configure Ollama
    let config = OllamaConfig::new("qwen3-vl:2b")
        .with_endpoint("http://localhost:11434");

    let runtime = OllamaRuntime::new(config).expect("Failed to create runtime");
    let runtime = Arc::new(runtime);

    // Complex test cases
    let test_cases = vec![
        ("多步推理", "我有100元，买苹果花了15元，买香蕉花了8元，又买橘子花了12元。最后还剩多少钱？请详细列出计算过程。"),

        ("复杂逻辑", "一个农场有鸡和兔子共50只，共有140条腿。请用代数方法列出方程组，然后计算鸡和兔子各有多少只？"),

        ("长问题", "请帮我写一份完整的周报，包含以下内容：1. 本周完成的主要工作（至少3项）；2. 遇到的问题及解决方案；3. 下周计划（至少2项）；4. 需要协调的事项。"),

        ("多问题", "请依次回答以下问题：1. 北京是哪个国家的首都？2. 1+2+3+4+5等于多少？3. 什么动物被称为森林之王？"),

        ("数据分析", "有一个班级，语文平均分85分，数学平均分90分，英语平均分88分。如果三科权重分别是30%、40%、30%，请计算加权平均分并分析哪一科对总分影响最大。"),

        ("场景推理", "小明早上8点从家出发，步行速度是每小时5公里。他走了2小时后休息30分钟，然后骑自行车返回，骑车速度是每小时15公里。请问小明什么时候能回到家？请详细分析每个时间段。"),
    ];

    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut failed_tests = 0;

    for (test_name, user_message) in &test_cases {
        total_tests += 1;
        println!("\n{:=^70}", "");
        println!(" [{}/{}] {}", total_tests, test_cases.len(), test_name);
        println!("{:=^70}", "");
        println!("问题: {}\n", user_message);

        // Show question length
        let msg_len = user_message.chars().count();
        println!("问题长度: {} 字符\n", msg_len);

        // Build input
        let input = LlmInput {
            messages: vec![Message::user(*user_message)],
            params: GenerationParams {
                max_tokens: Some(32768),
                temperature: Some(0.4),
                ..Default::default()
            },
            model: Some("qwen3-vl:2b".to_string()),
            stream: true,
            tools: None,
        };

        // Track metrics
        let mut thinking_chars = 0usize;
        let mut content_chars = 0usize;
        let mut chunk_count = 0usize;
        let start_time = std::time::Instant::now();

        // Stream response
        match runtime.generate_stream(input).await {
            Ok(mut stream) => {
                println!("📡 接收流中...");

                loop {
                    match stream.next().await {
                        Some(chunk_result) => match chunk_result {
                            Ok((text, is_thinking)) => {
                                chunk_count += 1;
                                if is_thinking {
                                    thinking_chars += text.chars().count();
                                    if thinking_chars % 1000 == 0 && thinking_chars > 0 {
                                        print!("💭({}) ", thinking_chars);
                                        std::io::stdout().flush().unwrap();
                                    }
                                } else {
                                    content_chars += text.chars().count();
                                }
                            }
                            Err(e) => {
                                println!("\n❌ 流错误: {}", e);
                                break;
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }

                let elapsed = start_time.elapsed();

                // Summary
                println!("\n📊 统计结果:");
                println!("  ⏱️  用时: {:.2}s", elapsed.as_secs_f64());
                println!("  📦 接收块数: {}", chunk_count);
                println!("  💭 思考字符: {}", thinking_chars);
                println!("  📝 内容字符: {}", content_chars);

                // Calculate ratio
                let total = thinking_chars + content_chars;
                let thinking_ratio = if total > 0 {
                    (thinking_chars as f64 / total as f64 * 100.0) as u32
                } else {
                    0
                };
                println!("  📈 思考占比: {}% ({}% 为内容)", thinking_ratio, 100 - thinking_ratio);

                if content_chars > 50 {
                    println!("  ✅ 测试通过");
                    passed_tests += 1;
                } else if content_chars > 0 {
                    println!("  ⚠️  内容较少");
                    passed_tests += 1;
                } else {
                    println!("  ❌ 测试失败: 无内容生成");
                    failed_tests += 1;
                }
            }
            Err(e) => {
                println!("❌ 请求失败: {}", e);
                failed_tests += 1;
            }
        }
    }

    println!("\n{:=^70}", "");
    println!(" 测试汇总");
    println!("{:=^70}", "");
    println!("  总测试数: {}", total_tests);
    println!("  通过: {} ✅", passed_tests);
    println!("  失败: {} ❌", failed_tests);
    println!("  成功率: {}%", (passed_tests as f64 / total_tests as f64 * 100.0) as u32);
    println!("{:=^70}\n", "");
}

#[tokio::test]
async fn test_conversation_with_history() {
    // Initialize logging (use try_init to avoid panic if already set)
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .try_init();

    println!("\n{:=^70}", "");
    println!(" CONVERSATION WITH HISTORY TEST");
    println!("{:=^70}\n", "");

    let config = OllamaConfig::new("qwen3-vl:2b")
        .with_endpoint("http://localhost:11434");
    let runtime = Arc::new(OllamaRuntime::new(config).expect("Failed to create runtime"));

    // Simulate a multi-turn conversation
    let mut messages = vec![
        Message::user("我叫张三，今年25岁，是一名软件工程师"),
    ];

    for (turn, user_msg) in [
        "你还记得我叫什么名字吗？",
        "我今年多大？",
        "我是做什么工作的？",
        "请总结一下我的信息",
    ].iter().enumerate() {
        println!("\n{:-^70}", "");
        println!(" 第 {} 轮对话", turn + 1);
        println!("{:-^70}", "");
        println!("用户: {}", user_msg);

        messages.push(Message::user(*user_msg));

        let input = LlmInput {
            messages: messages.clone(),
            params: GenerationParams {
                max_tokens: Some(32768),
                temperature: Some(0.4),
                ..Default::default()
            },
            model: Some("qwen3-vl:2b".to_string()),
            stream: true,
            tools: None,
        };

        let mut thinking_chars = 0usize;
        let mut content_chars = 0usize;

        match runtime.generate_stream(input).await {
            Ok(mut stream) => {
                let mut response = String::new();
                loop {
                    match stream.next().await {
                        Some(Ok((text, is_thinking))) => {
                            if is_thinking {
                                thinking_chars += text.chars().count();
                            } else {
                                response.push_str(&text);
                                content_chars += text.chars().count();
                            }
                        }
                        Some(Err(e)) => {
                            println!("❌ 错误: {}", e);
                            break;
                        }
                        None => break,
                    }
                }

                // Show truncated response
                let display_response = if response.chars().count() > 200 {
                    format!("{}...", &response.chars().take(200).collect::<String>())
                } else {
                    response.clone()
                };
                println!("助手: {}", display_response);
                println!("(思考: {} 字符, 内容: {} 字符)", thinking_chars, content_chars);

                // Add assistant response to history
                messages.push(Message::assistant(&response));
            }
            Err(e) => {
                println!("❌ 请求失败: {}", e);
            }
        }
    }

    println!("\n{:=^70}", "");
    println!(" 多轮对话测试完成");
    println!("{:=^70}\n", "");
}
