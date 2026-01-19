//! 对话上下文集成测试
//!
//! 测试场景：
//! 1. 多轮对话中的实体引用
//! 2. 代词解析（"它"、"那个"）
//! 3. 模糊命令补全
//! 4. 位置/设备上下文保持

use edge_ai_agent::agent::conversation_context::{
    ConversationContext, ConversationTopic,
};

/// 测试多轮对话场景：客厅 -> 灯 -> 空调
#[test]
fn test_multi_turn_living_room_scenario() {
    let mut ctx = ConversationContext::new();

    // 第一轮：用户问客厅温度
    ctx.update("客厅温度多少", &[]);
    assert_eq!(ctx.current_location, Some("客厅".to_string()));
    assert_eq!(ctx.topic, ConversationTopic::DataQuery);

    // 第二轮：用户说"打开灯" - 应该能推断为客厅灯
    let _enhanced = ctx.enhance_input("打开灯");
    // 检查上下文摘要包含客厅信息
    let summary = ctx.get_context_summary();
    assert!(summary.contains("客厅"));

    // 第三轮：用户说"关闭它" - 应该解析为刚打开的灯
    let resolved = ctx.resolve_pronoun("它");
    // 由于之前打开了灯，"它"应该能解析到某个设备
    assert!(resolved.is_some() || ctx.current_device.is_some());
}

/// 测试代词解析功能
#[test]
fn test_pronoun_resolution() {
    let mut ctx = ConversationContext::new();

    // 先添加一个设备到上下文
    ctx.add_device("客厅空调".to_string());
    ctx.current_device = Some("客厅空调".to_string());

    // 测试各种代词
    assert_eq!(ctx.resolve_pronoun("它"), Some("客厅空调".to_string()));
    assert_eq!(ctx.resolve_pronoun("这个"), Some("客厅空调".to_string()));
    assert_eq!(ctx.resolve_pronoun("那个"), Some("客厅空调".to_string()));
}

/// 测试模糊命令补全
#[test]
fn test_ambiguous_command_completion() {
    let mut ctx = ConversationContext::new();

    // 设置当前位置为客厅
    ctx.current_location = Some("客厅".to_string());

    // "打开" -> "打开客厅的设备"
    assert_eq!(
        ctx.resolve_ambiguous_command("打开"),
        Some("打开客厅的设备".to_string())
    );

    // "关闭" -> "关闭客厅空调"（如果设置了当前设备）
    ctx.current_device = Some("客厅空调".to_string());
    assert_eq!(
        ctx.resolve_ambiguous_command("关闭"),
        Some("关闭客厅空调".to_string())
    );

    // "温度" -> "客厅的温度"
    assert_eq!(
        ctx.resolve_ambiguous_command("温度"),
        Some("客厅的温度".to_string())
    );
}

/// 测试设备提及历史管理
#[test]
fn test_device_mention_history() {
    let mut ctx = ConversationContext::new();

    // 添加多个设备
    ctx.add_device("客厅灯".to_string());
    ctx.add_device("卧室空调".to_string());
    ctx.add_device("厨房插座".to_string());

    // 添加重复设备应该更新提及时间（但不会改变位置）
    ctx.turn_count = 5;
    ctx.add_device("客厅灯".to_string());

    // 验证设备数量
    assert_eq!(ctx.mentioned_devices.len(), 3);

    // 验证客厅灯的提及时间已更新
    let living_room_light = ctx.mentioned_devices.iter().find(|d| d.name == "客厅灯").unwrap();
    assert_eq!(living_room_light.last_mentioned_turn, 5);
}

/// 测试位置上下文管理
#[test]
fn test_location_context_management() {
    let mut ctx = ConversationContext::new();

    ctx.add_location("客厅".to_string());
    ctx.add_location("卧室".to_string());
    ctx.add_location("厨房".to_string());

    assert_eq!(ctx.mentioned_locations.len(), 3);

    // 重复添加相同位置应该更新而不是新增
    ctx.turn_count = 10;
    ctx.add_location("客厅".to_string());
    assert_eq!(ctx.mentioned_locations.len(), 3);
}

/// 测试主题检测（通过 update 方法）
#[test]
fn test_topic_detection() {
    let mut ctx = ConversationContext::new();

    // 控制类
    ctx.update("打开客厅的灯", &[]);
    assert_eq!(ctx.topic, ConversationTopic::DeviceControl);

    // 查询类
    ctx.update("温度多少", &[]);
    assert_eq!(ctx.topic, ConversationTopic::DataQuery);

    // 规则创建类
    ctx.update("创建一个自动化规则", &[]);
    assert_eq!(ctx.topic, ConversationTopic::RuleCreation);

    // 工作流设计类
    ctx.update("设计一个新的工作流", &[]);
    assert_eq!(ctx.topic, ConversationTopic::WorkflowDesign);
}

/// 测试中英文混合输入
#[test]
fn test_mixed_language_input() {
    let mut ctx = ConversationContext::new();

    // 英文位置 - 通过 update 方法自动提取
    ctx.update("kitchen temperature", &[]);
    assert!(ctx.current_location.is_some() || ctx.mentioned_locations.len() > 0);

    // 中英文混合 - 测试 enhance_input 方法
    ctx.current_location = Some("客厅".to_string());
    let enhanced = ctx.enhance_input("turn on the light");
    // enhance_input 应该能处理混合语言输入
    assert!(!enhanced.is_empty());
}

/// 测试上下文摘要生成
#[test]
fn test_context_summary_generation() {
    let mut ctx = ConversationContext::new();

    ctx.current_location = Some("客厅".to_string());
    ctx.current_device = Some("客厅空调".to_string());
    ctx.add_device("客厅灯".to_string());
    ctx.add_location("卧室".to_string());

    let summary = ctx.get_context_summary();

    assert!(summary.contains("客厅"));
    assert!(summary.contains("空调"));
}

/// 测试工具结果中的实体提取
#[test]
fn test_entity_extraction_from_tool_results() {
    let mut ctx = ConversationContext::new();

    // 模拟 list_devices 工具返回结果
    let tool_results = vec![
        ("list_devices".to_string(), "客厅灯: 开启\n卧室空调: 26°C".to_string()),
    ];

    ctx.update("查看设备状态", &tool_results);

    // 应该从工具结果中提取到设备
    assert!(ctx.mentioned_devices.len() > 0);
}

/// 测试上下文重置
#[test]
fn test_context_reset() {
    let mut ctx = ConversationContext::new();

    ctx.current_location = Some("客厅".to_string());
    ctx.current_device = Some("客厅空调".to_string());
    ctx.add_device("客厅灯".to_string());
    ctx.turn_count = 10;

    ctx.reset();

    assert_eq!(ctx.current_location, None);
    assert_eq!(ctx.current_device, None);
    assert_eq!(ctx.mentioned_devices.len(), 0);
    assert_eq!(ctx.turn_count, 0);
}

/// 测试完整的对话流程
#[test]
fn test_complete_conversation_flow() {
    let mut ctx = ConversationContext::new();

    // 第一轮：查询温度
    ctx.update("客厅温度多少", &[]);
    assert_eq!(ctx.current_location, Some("客厅".to_string()));

    // 第二轮：打开灯
    let enhanced2 = ctx.enhance_input("打开灯");
    ctx.update(&enhanced2, &[]);
    // 主题应该是 DeviceControl（因为"打开"是控制关键词）
    assert_eq!(ctx.topic, ConversationTopic::DeviceControl);

    // 第三轮：设置空调温度
    let enhanced3 = ctx.enhance_input("把空调调到26度");
    ctx.update(&enhanced3, &[]);

    // 第四轮：关闭"它"
    let resolved4 = ctx.resolve_pronoun("它");
    assert!(resolved4.is_some() || ctx.current_device.is_some());

    // 验证上下文摘要包含有意义的信息
    let summary = ctx.get_context_summary();
    assert!(!summary.is_empty());
}

/// 测试位置引用检测（通过 update 方法）
#[test]
fn test_location_extraction_via_update() {
    let mut ctx = ConversationContext::new();

    // 中文位置 - 通过 update 自动提取
    ctx.update("客厅的灯打开", &[]);
    assert_eq!(ctx.current_location, Some("客厅".to_string()));

    // 英文位置
    ctx.reset();
    ctx.update("kitchen light on", &[]);
    assert!(ctx.current_location.is_some());

    // 重置后测试无位置的命令
    ctx.reset();
    ctx.update("打开所有灯", &[]);
    // 没有明确位置时，current_location 应该为 None
    assert!(ctx.current_location.is_none());
}

/// 测试设备数量限制
#[test]
fn test_device_count_limit() {
    let mut ctx = ConversationContext::new();

    // 添加超过10个设备
    for i in 0..15 {
        ctx.add_device(format!("设备{}", i));
    }

    // 应该只保留最近10个
    assert_eq!(ctx.mentioned_devices.len(), 10);

    // 最新的应该是最后添加的
    assert_eq!(ctx.mentioned_devices.last().unwrap().name, "设备14");
}

/// 测试位置数量限制
#[test]
fn test_location_count_limit() {
    let mut ctx = ConversationContext::new();

    // 添加超过5个位置
    for loc in &["客厅", "卧室", "厨房", "卫生间", "书房", "阳台", "车库"] {
        ctx.add_location(loc.to_string());
    }

    // 应该只保留最近5个
    assert_eq!(ctx.mentioned_locations.len(), 5);
}
