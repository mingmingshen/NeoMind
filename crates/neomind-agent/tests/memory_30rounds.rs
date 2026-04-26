//! 30-round memory extraction simulation
//!
//! Run: cargo test -p neomind-agent --test memory_30rounds -- --ignored --nocapture

use std::sync::Arc;
use tokio::sync::RwLock;

use neomind_agent::{ExtractionConfig, MemoryExtractor, OllamaConfig, OllamaRuntime};
use neomind_core::llm::backend::LlmRuntime;
use neomind_storage::{MarkdownMemoryStore, MemoryCategory};

fn ollama_available() -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 11434));
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(2)).is_ok()
}

/// Agent execution simulation data for each round
struct AgentRound {
    agent_name: &'static str,
    user_prompt: &'static str,
    reasoning: &'static str,
    conclusion: &'static str,
}

fn get_30_rounds() -> Vec<AgentRound> {
    vec![
        // Round 1: Temperature threshold discovery
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "监控仓库温度，超过阈值告警",
            reasoning: "1. [analyze] 读取温度传感器，当前 27.5°C\n2. [alert] 超过 25°C 阈值，但历史数据表明午后此温度正常\n3. [command] 暂不告警，但建议将阈值从 25°C 提升至 28°C 以减少误报",
            conclusion: "当前 27.5°C 未告警。经过多次观察发现 25°C 阈值在午后时段产生大量误报，建议调整为 28°C。",
        },
        // Round 2: Sensor calibration discovery
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "监控仓库温度",
            reasoning: "1. [analyze] 传感器 #3 读数 30°C，#1 读 28°C，#2 读 28.2°C\n2. [alert] 传感器 #3 持续偏高约 2°C，存在校准偏差\n3. [command] 记录偏差，后续读数需补偿",
            conclusion: "传感器 #3 存在约 +2°C 系统性偏差，建议校准或标记补偿。",
        },
        // Round 3: Humidity pattern
        AgentRound {
            agent_name: "环境监控助手",
            user_prompt: "监控仓库湿度",
            reasoning: "1. [analyze] 当前湿度 85%\n2. [alert] 超过 80% 阈值，存在结露风险\n3. [analysis] 发现每天凌晨 3-5 点湿度规律性升高",
            conclusion: "湿度 85% 已告警。发现规律：凌晨 3-5 点湿度升至 80% 以上，与通风系统定时关闭有关。",
        },
        // Round 4: User preference
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "用户说用微信通知告警",
            reasoning: "1. [analyze] 用户偏好微信通知\n2. [command] 更新通知渠道为微信",
            conclusion: "已将告警通知渠道设置为微信。用户不使用短信和邮件通知。",
        },
        // Round 5: Power consumption pattern
        AgentRound {
            agent_name: "能耗监控",
            user_prompt: "监控仓库能耗",
            reasoning: "1. [analyze] 当前功率 15kW，高于历史同期 12kW\n2. [alert] 功率异常偏高\n3. [analysis] 发现空调系统在非工作时段仍保持运行",
            conclusion: "能耗异常偏高。空调系统在 18:00-次日 8:00 非工作时段仍在运行，建议设置定时关闭。",
        },
        // Round 6: Motion detection pattern
        AgentRound {
            agent_name: "安防监控",
            user_prompt: "监控仓库人员活动",
            reasoning: "1. [analyze] 凌晨 2:30 检测到移动信号\n2. [alert] 非工作时段有移动\n3. [command] 调取摄像头确认，发现是夜班巡检人员",
            conclusion: "凌晨 2:30 移动信号为夜班巡检人员。建议将 2:00-3:00 加入巡检白名单避免误报。",
        },
        // Round 7: Strategy optimization
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "检查告警策略效果",
            reasoning: "1. [analyze] 过去一周告警 45 次，其中误报 35 次\n2. [alert] 误报率过高，达 78%\n3. [command] 采用双次确认策略：第一次超阈值后等待 5 分钟再次确认",
            conclusion: "双次确认策略实施后，误报率从 78% 降至 15%。建议推广至所有告警规则。",
        },
        // Round 8: Network latency baseline
        AgentRound {
            agent_name: "网络监控",
            user_prompt: "监控设备网络连通性",
            reasoning: "1. [analyze] 传感器 #3 平均延迟 150ms，其他 30ms\n2. [alert] #3 延迟异常高\n3. [analysis] #3 位于仓库最远端，WiFi 信号弱",
            conclusion: "传感器 #3 因距离远 WiFi 信号弱导致延迟偏高。建议增设中继器或将 #3 改为有线连接。",
        },
        // Round 9: Seasonal pattern
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "分析温度趋势",
            reasoning: "1. [analyze] 过去 30 天日均温度从 22°C 升至 28°C\n2. [analysis] 夏季升温趋势明显，需提前调整阈值\n3. [command] 建议 6-8 月阈值上调至 30°C",
            conclusion: "发现季节性升温模式：6-8 月平均温度比春秋高 6°C，阈值需随季节动态调整。",
        },
        // Round 10: Device fault detection
        AgentRound {
            agent_name: "设备健康监控",
            user_prompt: "检查传感器健康状态",
            reasoning: "1. [analyze] 传感器 #7 过去 24 小时未上报数据\n2. [alert] 设备离线\n3. [command] 尝试远程重启未成功",
            conclusion: "传感器 #7 硬件故障需更换。历史数据显示该型号传感器平均寿命约 18 个月。",
        },
        // Round 11: User language preference
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "用中文给我报告",
            reasoning: "1. [analyze] 用户要求中文报告\n2. [command] 切换输出语言为中文",
            conclusion: "已将报告语言设置为中文。用户习惯使用中文交流。",
        },
        // Round 12: Air quality baseline
        AgentRound {
            agent_name: "环境监控助手",
            user_prompt: "监控空气质量",
            reasoning: "1. [analyze] PM2.5 = 35μg/m³，CO2 = 800ppm\n2. [analysis] 办公区 CO2 浓度在 10:00-11:00 和 14:00-15:00 持续偏高\n3. [command] 建议增加这两个时段的通风",
            conclusion: "CO2 高峰时段与人员密集时段吻合（10:00-11:00，14:00-15:00），建议定时加强通风。",
        },
        // Round 13: Threshold fine-tuning
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "验证新阈值效果",
            reasoning: "1. [analyze] 28°C 阈值运行一周，告警 12 次，误报仅 2 次\n2. [analysis] 相比 25°C 阈值时的 45 次告警/35 次误报，大幅改善\n3. [command] 确认 28°C 为最优阈值",
            conclusion: "28°C 阈值验证通过：告警次数减少 73%，误报率降至 17%，为当前最优设置。",
        },
        // Round 14: Water leak detection
        AgentRound {
            agent_name: "安防监控",
            user_prompt: "检测漏水风险",
            reasoning: "1. [analyze] 地下室湿度突增至 95%，水浸传感器触发\n2. [alert] 检测到漏水\n3. [command] 紧急通知物业，自动关闭相关水阀",
            conclusion: "地下室漏水已处理。发现该区域每逢暴雨后 2 小时内易发生渗漏。",
        },
        // Round 15: Lighting optimization
        AgentRound {
            agent_name: "能耗监控",
            user_prompt: "优化照明策略",
            reasoning: "1. [analyze] 照明日均用电 50kWh，其中 30% 为无人区域照明\n2. [analysis] 走廊和卫生间照明 24 小时常开\n3. [command] 建议安装人体感应开关",
            conclusion: "无人区域照明浪费 30% 电力。走廊和卫生间建议改用人体感应照明。",
        },
        // Round 16: Prediction model improvement
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "评估温度预测准确度",
            reasoning: "1. [analyze] 过去 7 天温度预测平均误差 1.5°C\n2. [analysis] 误差主要来自午后阳光直射时段\n3. [command] 建议对 12:00-15:00 时段增加 ±2°C 容差",
            conclusion: "温度预测在午后直射时段误差较大（±2°C），建议该时段使用更宽的容差范围。",
        },
        // Round 17: Multi-device correlation
        AgentRound {
            agent_name: "环境监控助手",
            user_prompt: "分析温湿度关联",
            reasoning: "1. [analyze] 温度每升高 1°C，湿度平均下降 3%\n2. [analysis] 两者呈强负相关，需联动调整阈值\n3. [command] 建议温度和湿度阈值联动：温度高时自动放宽湿度阈值",
            conclusion: "温湿度强负相关（r=-0.87），建议联动阈值策略：温度超 30°C 时湿度阈值自动上调至 85%。",
        },
        // Round 18: Battery life prediction
        AgentRound {
            agent_name: "设备健康监控",
            user_prompt: "检查电池供电设备",
            reasoning: "1. [analyze] 传感器 #5 电池电量 15%\n2. [alert] 电量低\n3. [analysis] 该传感器已运行 14 个月，电池消耗速度比预期快",
            conclusion: "传感器 #5 电池即将耗尽。发现高温环境加速电池消耗，比常温环境快约 40%。",
        },
        // Round 19: Alert fatigue analysis
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "分析告警疲劳问题",
            reasoning: "1. [analyze] 用户过去一周忽略 60% 的告警通知\n2. [analysis] 告警过于频繁导致用户警觉性下降\n3. [command] 建议合并相似告警，每小时最多发送 1 条汇总",
            conclusion: "告警疲劳严重：60% 通知被忽略。建议采用告警聚合策略，每小时汇总发送一次。",
        },
        // Round 20: Night mode
        AgentRound {
            agent_name: "环境监控助手",
            user_prompt: "设置夜间模式",
            reasoning: "1. [analyze] 用户要求 22:00-7:00 降低告警灵敏度\n2. [command] 夜间模式阈值：温度 32°C，湿度 90%",
            conclusion: "夜间模式已启用（22:00-7:00），阈值上调。用户睡眠时间不想被打扰。",
        },
        // Round 21: Firmware update insight
        AgentRound {
            agent_name: "设备健康监控",
            user_prompt: "检查固件版本",
            reasoning: "1. [analyze] 5 个传感器运行旧固件 v2.1，最新 v2.3\n2. [analysis] v2.3 修复了 WiFi 重连问题，更新后可减少离线率\n3. [command] 建议分批更新固件",
            conclusion: "旧固件 v2.1 存在 WiFi 重连 bug，更新至 v2.3 后预计离线率降低 50%。",
        },
        // Round 22: False positive pattern
        AgentRound {
            agent_name: "安防监控",
            user_prompt: "分析误报原因",
            reasoning: "1. [analyze] 上周 20 次移动告警中 15 次为宠物触发\n2. [analysis] 仓库有猫出没，PIR 传感器无法区分\n3. [command] 建议升级为双鉴传感器（PIR+微波）",
            conclusion: "PIR 传感器被宠物频繁触发（75% 误报），建议升级双鉴传感器以过滤动物误报。",
        },
        // Round 23: Peak load pattern
        AgentRound {
            agent_name: "能耗监控",
            user_prompt: "分析用电高峰",
            reasoning: "1. [analyze] 每日 9:00-10:00 用电峰值 45kW，其余时段平均 15kW\n2. [analysis] 所有设备同时启动造成峰值\n3. [command] 建议设备错峰启动，间隔 5 分钟",
            conclusion: "用电峰值集中在上电瞬间（9:00），错峰启动可降低峰值 60% 至 18kW。",
        },
        // Round 24: Maintenance schedule
        AgentRound {
            agent_name: "设备健康监控",
            user_prompt: "制定维护计划",
            reasoning: "1. [analyze] 10 个传感器中 3 个已超 12 个月未维护\n2. [analysis] 超期未维护的传感器故障率是正常维护的 3 倍\n3. [command] 生成维护工单",
            conclusion: "建议每 6 个月校准一次传感器。超期维护的设备故障率显著升高（3x）。",
        },
        // Round 25: Communication protocol optimization
        AgentRound {
            agent_name: "网络监控",
            user_prompt: "优化数据上报频率",
            reasoning: "1. [analyze] 当前每 10 秒上报一次，数据冗余度高\n2. [analysis] 温度变化缓慢时无需高频上报\n3. [command] 建议动态频率：变化 >1°C 时 10s，否则 60s",
            conclusion: "动态上报策略可将网络负载降低 70%，且不丢失关键温度变化事件。",
        },
        // Round 26: Ventilation effectiveness
        AgentRound {
            agent_name: "环境监控助手",
            user_prompt: "评估通风效果",
            reasoning: "1. [analyze] 开启排风扇后 CO2 从 800ppm 降至 450ppm，耗时 15 分钟\n2. [analysis] 单台排风扇对 200m² 空间效率不足\n3. [command] 建议高峰时段开启两台",
            conclusion: "单台排风扇 CO2 降低速率不够，高峰时段需双台并行运行，15 分钟可恢复达标。",
        },
        // Round 27: User timezone
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "我在上海，用北京时间",
            reasoning: "1. [analyze] 用户时区 Asia/Shanghai (UTC+8)\n2. [command] 更新所有时间相关逻辑为北京时间",
            conclusion: "时区已设置为 Asia/Shanghai (UTC+8)。用户位于上海。",
        },
        // Round 28: Rain prediction correlation
        AgentRound {
            agent_name: "环境监控助手",
            user_prompt: "分析降雨与漏水关联",
            reasoning: "1. [analyze] 过去 3 次漏水均发生在降雨量 >30mm 的暴雨后\n2. [analysis] 降雨量与漏水风险正相关\n3. [command] 建议暴雨预警时提前部署防水措施",
            conclusion: "降雨量 >30mm 时漏水概率达 80%，建议接入天气预报提前预防。",
        },
        // Round 29: Energy baseline establishment
        AgentRound {
            agent_name: "能耗监控",
            user_prompt: "建立能耗基线",
            reasoning: "1. [analyze] 过去 30 天日均能耗：工作日 320kWh，周末 180kWh\n2. [analysis] 工作日基线 250-380kWh 正常，超出即为异常\n3. [command] 设置工作日 400kWh 异常告警",
            conclusion: "能耗基线已建立：工作日 250-380kWh，周末 150-220kWh。超基线 20% 触发告警。",
        },
        // Round 30: Comprehensive optimization summary
        AgentRound {
            agent_name: "温控卫士",
            user_prompt: "综合评估优化效果",
            reasoning: "1. [analyze] 经过 30 天优化：误报率从 78% 降至 15%，能耗降低 25%，传感器离线率降低 40%\n2. [analysis] 双次确认策略、动态阈值、错峰启动三项措施效果最显著\n3. [command] 形成最佳实践文档",
            conclusion: "30 天优化总结：双次确认策略减少误报 80%，28°C 动态阈值效果最优，错峰启动降低峰值 60%。",
        },
    ]
}

async fn create_extractor() -> (
    tempfile::TempDir,
    Arc<RwLock<MarkdownMemoryStore>>,
    MemoryExtractor,
) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let store = MarkdownMemoryStore::new(temp_dir.path().to_path_buf());
    store.init().unwrap();
    let store = Arc::new(RwLock::new(store));

    let config = OllamaConfig {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen3.5:4b".to_string(),
        timeout_secs: 120,
    };
    let llm: Arc<dyn LlmRuntime> = Arc::new(OllamaRuntime::new(config).unwrap());

    let extraction_config = ExtractionConfig {
        min_messages: 1,
        max_messages: 50,
        min_importance: 20,
        dedup_enabled: true,
        similarity_threshold: 0.85,
    };

    let extractor = MemoryExtractor::with_config(store.clone(), llm, extraction_config);
    (temp_dir, store, extractor)
}

#[tokio::test]
#[ignore]
async fn test_30_rounds_memory_extraction() -> anyhow::Result<()> {
    if !ollama_available() {
        eprintln!("Ollama not available, skipping");
        return Ok(());
    }

    let (_temp_dir, store, extractor) = create_extractor().await;
    let rounds = get_30_rounds();

    println!("\n============================================================");
    println!("  30-Round Memory Extraction Simulation");
    println!("============================================================\n");

    let mut total_extracted = 0;
    let mut round_results: Vec<(usize, usize)> = Vec::new(); // (round, count)

    for (i, round) in rounds.iter().enumerate() {
        let round_num = i + 1;
        println!("--- Round {}/30: {} ---", round_num, round.agent_name);

        let count = extractor
            .extract_from_agent(
                round.agent_name,
                Some(round.user_prompt),
                round.reasoning,
                round.conclusion,
            )
            .await
            .unwrap_or(0);

        total_extracted += count;
        round_results.push((round_num, count));
        println!(
            "  Extracted: {} memories (total so far: {})\n",
            count, total_extracted
        );
    }

    // Print round summary
    println!("\n============================================================");
    println!("  Round Summary");
    println!("============================================================");
    for (round, count) in &round_results {
        println!("  Round {:2}: {} memories extracted", round, count);
    }
    println!("  Total: {} memories from 30 rounds", total_extracted);

    // Print category breakdown
    println!("\n============================================================");
    println!("  Category Breakdown");
    println!("============================================================");

    let store_guard = store.read().await;
    let mut category_stats = Vec::new();

    for category in MemoryCategory::all() {
        let content = store_guard.read_category(&category).unwrap_or_default();
        let entry_count = content
            .lines()
            .filter(|l| l.trim().starts_with("- ["))
            .count();
        category_stats.push((category.display_name().to_string(), entry_count, content));
    }

    for (name, count, content) in &category_stats {
        println!("\n[{}] ({} entries)", name, count);
        for line in content.lines().take(10) {
            println!("  {}", line);
        }
        if content.lines().count() > 10 {
            println!("  ... ({} more lines)", content.lines().count() - 10);
        }
    }

    // Assertions
    println!("\n============================================================");
    println!("  Assertions");
    println!("============================================================");

    assert!(
        total_extracted > 0,
        "Should extract memories across 30 rounds"
    );
    println!("  [PASS] Total extracted > 0: {}", total_extracted);

    let se_count = category_stats
        .iter()
        .find(|(name, _, _)| name == "System Evolution")
        .map(|(_, c, _)| *c)
        .unwrap_or(0);

    assert!(
        se_count > 0,
        "system_evolution must have entries after 30 rounds"
    );
    println!("  [PASS] system_evolution has {} entries", se_count);

    let non_empty = category_stats.iter().filter(|(_, c, _)| *c > 0).count();
    assert!(
        non_empty >= 3,
        "At least 3 categories should be populated, got {}",
        non_empty
    );
    println!("  [PASS] {} out of 4 categories populated", non_empty);

    println!("\nAll 30-round tests passed!");
    Ok(())
}
