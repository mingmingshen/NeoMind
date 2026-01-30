//! 意图分类模块
//!
//! 功能：
//! 1. 10大核心意图分类：QueryData, AnalyzeData, ControlDevice, CreateAutomation, SendMessage, SummarizeInfo, Clarify, OutOfScope, AgentMonitor, AlertChannel
//! 2. 子类型识别
//! 3. 置信度评分
//! 4. 实体提取
//! 5. 处理策略建议

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 核心意图类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum IntentCategory {
    /// 数据查询：状态查询、历史查询
    #[serde(rename = "query_data")]
    QueryData = 0,

    /// 数据分析：趋势分析、异常检测
    #[serde(rename = "analyze_data")]
    AnalyzeData = 1,

    /// 设备控制：单设备、批量、场景
    #[serde(rename = "control_device")]
    ControlDevice = 2,

    /// 自动化创建：规则、工作流
    #[serde(rename = "create_automation")]
    CreateAutomation = 3,

    /// 消息发送：通知、报告
    #[serde(rename = "send_message")]
    SendMessage = 4,

    /// 信息汇总：数据汇总、报告生成
    #[serde(rename = "summarize_info")]
    SummarizeInfo = 5,

    /// 澄清：模糊输入需要追问
    #[serde(rename = "clarify")]
    Clarify = 6,

    /// 超出范围：能力外的请求
    #[serde(rename = "out_of_scope")]
    OutOfScope = 7,

    /// AI Agent监控：查看agent状态、执行历史、决策过程
    #[serde(rename = "agent_monitor")]
    AgentMonitor = 8,

    /// 告警通道管理：通知渠道配置、渠道测试
    #[serde(rename = "alert_channel")]
    AlertChannel = 9,
}

impl IntentCategory {
    /// 获取意图的中文描述
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::QueryData => "查询数据",
            Self::AnalyzeData => "分析数据",
            Self::ControlDevice => "控制设备",
            Self::CreateAutomation => "创建自动化",
            Self::SendMessage => "发送消息",
            Self::SummarizeInfo => "汇总信息",
            Self::Clarify => "需要澄清",
            Self::OutOfScope => "超出范围",
            Self::AgentMonitor => "Agent监控",
            Self::AlertChannel => "告警通道",
        }
    }

    /// 获取意图的英文标识
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::QueryData => "query_data",
            Self::AnalyzeData => "analyze_data",
            Self::ControlDevice => "control_device",
            Self::CreateAutomation => "create_automation",
            Self::SendMessage => "send_message",
            Self::SummarizeInfo => "summarize_info",
            Self::Clarify => "clarify",
            Self::OutOfScope => "out_of_scope",
            Self::AgentMonitor => "agent_monitor",
            Self::AlertChannel => "alert_channel",
        }
    }

    /// 从字符串解析意图
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "query_data" => Some(Self::QueryData),
            "analyze_data" => Some(Self::AnalyzeData),
            "control_device" => Some(Self::ControlDevice),
            "create_automation" => Some(Self::CreateAutomation),
            "send_message" => Some(Self::SendMessage),
            "summarize_info" => Some(Self::SummarizeInfo),
            "clarify" => Some(Self::Clarify),
            "out_of_scope" => Some(Self::OutOfScope),
            "agent_monitor" => Some(Self::AgentMonitor),
            "alert_channel" => Some(Self::AlertChannel),
            _ => None,
        }
    }
}

/// 意图子类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentSubType {
    // QueryData 子类型
    CurrentStatus,
    HistoricalQuery,
    DeviceList,
    RuleList,
    WorkflowList,

    // AnalyzeData 子类型
    TrendAnalysis,
    AnomalyDetection,
    StatisticsSummary,
    Comparison,

    // ControlDevice 子类型
    SingleDevice,
    BatchControl,
    SceneMode,

    // CreateAutomation 子类型
    SimpleRule,
    ComplexRule,
    Workflow,

    // SendMessage 子类型
    Notification,
    Report,
    Alert,

    // SummarizeInfo 子类型
    DataSummary,
    ReportGeneration,
    Insights,

    // Clarify 子类型
    MissingLocation,
    MissingDevice,
    MissingAction,
    AmbiguousInput,

    // OutOfScope 子类型
    ExternalService,
    HardwareModification,
    SystemConfiguration,

    // AgentMonitor 子类型
    AgentStatus,
    ExecutionHistory,
    DecisionProcess,
    PerformanceStats,

    // AlertChannel 子类型
    ChannelList,
    ChannelConfig,
    ChannelTest,
    ChannelEnable,

    // 未知
    Unknown,
}

/// 处理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingStrategy {
    /// 快速路径：直接响应，无需LLM
    FastPath,

    /// 标准路径：使用快速模型
    Standard,

    /// 质量优先：使用高质量模型
    Quality,

    /// 多轮对话：需要多轮交互
    MultiTurn,

    /// 降级：能力外，提供建议
    Fallback,
}

/// 提取的实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// 实体类型
    pub entity_type: EntityType,
    /// 实体值
    pub value: String,
    /// 在输入中的位置（可选）
    pub position: Option<usize>,
    /// 置信度
    pub confidence: f32,
}

/// 实体类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    /// 设备名称
    Device,
    /// 位置
    Location,
    /// 数值/参数
    Value,
    /// 时间范围
    TimeRange,
    /// 动作
    Action,
    /// 规则名称
    Rule,
    /// 工作流名称
    Workflow,
    /// 未知
    Unknown,
}

/// 意图分类结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClassification {
    /// 主要意图
    pub intent: IntentCategory,
    /// 子类型
    pub sub_type: IntentSubType,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f32,
    /// 提取的实体
    pub entities: Vec<Entity>,
    /// 处理策略建议
    pub strategy: ProcessingStrategy,
    /// 是否需要追问
    pub needs_followup: bool,
    /// 追问提示（如果需要）
    pub followup_prompt: Option<String>,
    /// 能力边界声明（如果超出范围）
    pub capability_statement: Option<String>,
}

/// 意图分类器
pub struct IntentClassifier {
    /// 启用的意图类别
    enabled_intents: Vec<IntentCategory>,
    /// 置信度阈值
    confidence_threshold: f32,
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self {
            enabled_intents: vec![
                IntentCategory::QueryData,
                IntentCategory::AnalyzeData,
                IntentCategory::ControlDevice,
                IntentCategory::CreateAutomation,
                IntentCategory::SendMessage,
                IntentCategory::SummarizeInfo,
                IntentCategory::Clarify,
                IntentCategory::OutOfScope,
                IntentCategory::AgentMonitor,
                IntentCategory::AlertChannel,
            ],
            confidence_threshold: 0.3,
        }
    }
}

impl IntentClassifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// 分类用户输入
    pub fn classify(&self, input: &str) -> IntentClassification {
        let input_lower = input.to_lowercase();

        // 1. 先检查是否是澄清场景（输入太短或模糊）
        if self.is_ambiguous(&input_lower) {
            return self.build_clarification_result(input);
        }

        // 2. 检查是否超出范围
        if let Some(statement) = self.check_out_of_scope(&input_lower) {
            return IntentClassification {
                intent: IntentCategory::OutOfScope,
                sub_type: IntentSubType::Unknown,
                confidence: 0.9,
                entities: vec![],
                strategy: ProcessingStrategy::Fallback,
                needs_followup: true,
                followup_prompt: Some("需要其他帮助吗？".to_string()),
                capability_statement: Some(statement),
            };
        }

        // 3. 计算各意图的匹配分数
        let mut scores: HashMap<IntentCategory, f32> = HashMap::new();

        scores.insert(IntentCategory::QueryData, self.score_query_data(&input_lower));
        scores.insert(IntentCategory::AnalyzeData, self.score_analyze_data(&input_lower));
        scores.insert(IntentCategory::ControlDevice, self.score_control_device(&input_lower));
        scores.insert(IntentCategory::CreateAutomation, self.score_create_automation(&input_lower));
        scores.insert(IntentCategory::SendMessage, self.score_send_message(&input_lower));
        scores.insert(IntentCategory::SummarizeInfo, self.score_summarize_info(&input_lower));
        scores.insert(IntentCategory::AgentMonitor, self.score_agent_monitor(&input_lower));
        scores.insert(IntentCategory::AlertChannel, self.score_alert_channel(&input_lower));

        // 4. 选择最高分的意图
        let (&intent, &confidence) = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        // 5. 如果置信度太低，归类为澄清
        if confidence < self.confidence_threshold {
            return self.build_clarification_result(input);
        }

        // 6. 构建分类结果
        self.build_classification_result(input, intent, confidence)
    }

    /// 检查输入是否模糊
    fn is_ambiguous(&self, input: &str) -> bool {
        let input = input.trim();

        // 输入太短
        if input.len() < 2 {
            return true;
        }

        // 只有一个词且是常见模糊词
        let ambiguous_single_words = ["打开", "关闭", "开启", "查询", "查看", "设置", "调整",
            "open", "close", "check", "show", "set", "adjust"];

        // 检查是否包含分隔符
        let has_separator = input.contains(' ') || input.contains('的') || input.contains('？') || input.contains('?');

        if !has_separator {
            // 如果是单个词，检查是否是模糊词
            for word in ambiguous_single_words {
                if input == word {
                    return true;
                }
            }
            // 如果包含数字，说明有具体的值，不算模糊
            if input.chars().any(|c| c.is_numeric()) {
                return false;
            }
        }

        false
    }

    /// 检查是否超出范围
    fn check_out_of_scope(&self, input: &str) -> Option<String> {
        // 硬件修改
        if input.contains("安装") && (input.contains("硬件") || input.contains("设备") || input.contains("传感器")) {
            return Some("硬件安装需要现场操作，我可以帮你规划但无法直接执行。".to_string());
        }

        // 系统级配置
        if input.contains("重启服务器") || input.contains("系统配置") || input.contains("修改系统") {
            return Some("系统配置修改需要管理员权限，请通过设置页面操作。".to_string());
        }

        // 外部服务
        if input.contains("打电话") || input.contains("发短信") || (input.contains("订") && input.contains("餐")) {
            return Some("我无法直接拨打电话或发送短信，但可以帮你生成提醒通知。".to_string());
        }

        None
    }

    /// 查询数据意图评分
    fn score_query_data(&self, input: &str) -> f32 {
        let mut score = 0.0;

        // 疑问词（高权重）
        let question_words = [
            "多少", "怎么样", "如何", "是不是", "在不在", "吗", "呢",
            "what", "how", "status", "temperature", "check", "show", "list",
        ];
        for word in &question_words {
            if input.contains(word) {
                score += 0.25;
            }
        }

        // 查询动词
        let query_verbs = [
            "查询", "查看", "显示", "列出", "当前", "现在", "获取",
        ];
        for word in &query_verbs {
            if input.contains(word) {
                score += 0.2;
            }
        }

        // 数据指标词（高权重）
        let metrics = ["温度", "湿度", "pm2.5", "亮度", "噪音"];
        for metric in &metrics {
            if input.contains(metric) {
                score += 0.3;
            }
        }

        // 设备相关
        if input.contains("设备") || input.contains("device") {
            score += 0.2;
        }

        // 规则相关
        if input.contains("规则") || input.contains("rule") {
            score += 0.15;
        }

        // 只有否定词且没有疑问特征时才降低分数
        let has_question_feature = question_words.iter().any(|w| input.contains(w))
            || query_verbs.iter().any(|w| input.contains(w))
            || metrics.iter().any(|m| input.contains(m));

        if !has_question_feature && (input.contains("打开") || input.contains("关闭") || input.contains("设置")) {
            score -= 0.15;
        }

        (score as f32).min(1.0).max(0.0)
    }

    /// 分析数据意图评分
    fn score_analyze_data(&self, input: &str) -> f32 {
        let mut score = 0.0;

        // 分析关键词
        let analyze_keywords = [
            "分析", "趋势", "统计", "总结", "比较", "异常", "规律", "变化",
            "analyze", "trend", "statistics", "summary", "compare", "anomaly",
        ];

        for keyword in &analyze_keywords {
            if input.contains(keyword) {
                score += 0.2;
            }
        }

        // 时间范围词
        if input.contains("最近") || input.contains("今天") || input.contains("本周") || input.contains("一周") {
            score += 0.15;
        }

        // 高级分析词
        if input.contains("预测") || input.contains("模式") || input.contains("pattern") {
            score += 0.1;
        }

        (score as f32).min(1.0).max(0.0)
    }

    /// 控制设备意图评分
    fn score_control_device(&self, input: &str) -> f32 {
        let mut score = 0.0;

        // 控制动词（高权重）
        let control_verbs = [
            "打开", "关闭", "开启", "停止", "启动",
            "open", "close", "turn on", "turn off", "start", "stop",
        ];
        for verb in &control_verbs {
            if input.contains(verb) {
                score += 0.35;
            }
        }

        // 设置/调节动词
        let adjust_verbs = [
            "设置", "调节", "调整", "切换",
            "switch", "adjust", "set", "change",
        ];
        for verb in &adjust_verbs {
            if input.contains(verb) {
                score += 0.3;
            }
        }

        // 调高/调低
        if input.contains("调高") || input.contains("调低") || input.contains("升高") || input.contains("降低") {
            score += 0.15;
        }

        // 设备类型词
        let device_types = ["灯", "空调", "窗帘", "门锁", "开关", "风扇", "电视", "heater", "cooler"];
        for device in &device_types {
            if input.contains(device) {
                score += 0.15;
            }
        }

        // 场景模式
        if input.contains("模式") || input.contains("场景") || input.contains("scene") || input.contains("mode") {
            score += 0.25;
        }

        (score as f32).min(1.0).max(0.0)
    }

    /// 创建自动化意图评分
    fn score_create_automation(&self, input: &str) -> f32 {
        let mut score = 0.0;

        // 优先检测完整的条件句式（最高权重）
        let has_when_then = (input.contains("当") && (input.contains("就") || input.contains("则") || input.contains("时")))
            || (input.contains("如果") && (input.contains("就") || input.contains("则")))
            || (input.contains("若") && (input.contains("就") || input.contains("则")));

        if has_when_then {
            score += 0.5; // 完整条件句高权重
        }

        // 检测"XX时YY"模式（如"温度高时打开空调"）
        // 这是另一种常见的条件句格式，不需要"当"
        let has_time_condition = input.contains("时")
            && (input.contains("打开") || input.contains("关闭") || input.contains("开启")
                || input.contains("启动") || input.contains("停止")
                || input.contains("open") || input.contains("close")
                || input.contains("turn on") || input.contains("turn off"));

        if has_time_condition && !has_when_then {
            score += 0.5; // "XX时YY"模式高权重
        }

        // 条件关联词（高权重）
        let condition_keywords = [
            "当", "时候", "如果", "若", "一旦",
            "when", "if", "once", "whenever",
        ];
        for keyword in &condition_keywords {
            if input.contains(keyword) {
                score += 0.2;
            }
        }

        // 结果关联词
        let result_keywords = [
            "就", "则", "那么", "then", "so",
        ];
        for keyword in &result_keywords {
            if input.contains(keyword) {
                score += 0.15;
            }
        }

        // 自动化关键词
        let automation_keywords = [
            "自动", "触发", "条件", "规则", "自动化", "工作流",
            "automatic", "trigger", "condition", "rule", "automation", "workflow",
        ];
        for keyword in &automation_keywords {
            if input.contains(keyword) {
                score += 0.15;
            }
        }

        // 序列词
        if input.contains("然后") || input.contains("接着") || input.contains("after") {
            score += 0.15;
        }

        // 创建动词
        if (input.contains("创建") || input.contains("新建") || input.contains("添加"))
            && (input.contains("规则") || input.contains("自动化") || input.contains("工作流")) {
                score += 0.3;
            }

        (score as f32).min(1.0).max(0.0)
    }

    /// 发送消息意图评分
    fn score_send_message(&self, input: &str) -> f32 {
        let mut score = 0.0;

        // 如果包含条件句式，降低分数（优先识别为自动化）
        if input.contains("如果") || input.contains("当") || input.contains("则") || input.contains("if") || input.contains("when") {
            return 0.1; // 低分，让自动化意图胜出
        }

        // 发送关键词
        let send_keywords = [
            "发送", "通知", "提醒", "告诉", "报告",
            "send", "notify", "alert", "message", "report", "email",
        ];
        for keyword in &send_keywords {
            if input.contains(keyword) {
                score += 0.15;
            }
        }

        // 接收对象
        if input.contains("给") || input.contains("到") || input.contains("to") {
            score += 0.1;
        }

        (score as f32).min(1.0).max(0.0)
    }

    /// 汇总信息意图评分
    fn score_summarize_info(&self, input: &str) -> f32 {
        let mut score = 0.0;

        // 汇总关键词
        let summary_keywords = [
            "汇总", "总结", "所有", "全部", "概览", "整体",
            "summary", "summarize", "all", "overview", "report",
        ];

        for keyword in &summary_keywords {
            if input.contains(keyword) {
                score += 0.2;
            }
        }

        // 范围词
        if input.contains("所有设备") || input.contains("全部规则") || input.contains("整体") {
            score += 0.2;
        }

        (score as f32).min(1.0).max(0.0)
    }

    /// Agent监控意图评分
    fn score_agent_monitor(&self, input: &str) -> f32 {
        let mut score = 0.0f32;

        // Agent/智能体相关关键词
        let agent_keywords = [
            "agent", "agents", "智能体", "ai代理", "ai代理",
            "agent状态", "agent执行", "agent历史",
            "监控agent", "查看agent", "agent统计",
        ];

        // 执行历史/决策过程相关
        let execution_keywords = [
            "执行历史", "决策过程", "决策记录", "推理过程",
            "execution history", "decision process", "reasoning",
            "执行记录", "操作记录",
        ];

        // 性能统计相关
        let stats_keywords = [
            "性能统计", "运行状态", "agent性能",
            "性能指标", "执行统计",
            "performance stats", "performance metrics",
        ];

        for keyword in &agent_keywords {
            if input.contains(keyword) {
                score += 0.35;
            }
        }

        for keyword in &execution_keywords {
            if input.contains(keyword) {
                score += 0.3;
            }
        }

        for keyword in &stats_keywords {
            if input.contains(keyword) {
                score += 0.3;
            }
        }

        // 检测是否提到具体agent名称
        if input.contains("monitor") || input.contains("executor") || input.contains("analyzer") {
            score += 0.25;
        }

        (score as f32).min(1.0).max(0.0)
    }

    /// 告警通道意图评分
    fn score_alert_channel(&self, input: &str) -> f32 {
        let mut score = 0.0f32;

        // 通道/渠道相关关键词
        let channel_keywords = [
            "通道", "渠道", "channel", "通知渠道", "告警通道",
            "notification channel", "alert channel",
            "邮件通知", "短信通知", "webhook",
        ];

        // 配置相关
        let config_keywords = [
            "配置通道", "设置通道", "添加通道", "删除通道",
            "configure channel", "setup channel", "add channel",
            "通道配置", "渠道设置",
        ];

        // 测试相关
        let test_keywords = [
            "测试通道", "测试通知", "发送测试",
            "test channel", "test notification", "send test",
        ];

        // 启用/禁用相关
        let enable_keywords = [
            "启用通道", "禁用通道", "开启通道", "关闭通道",
            "enable channel", "disable channel",
        ];

        for keyword in &channel_keywords {
            if input.contains(keyword) {
                score += 0.25;
            }
        }

        for keyword in &config_keywords {
            if input.contains(keyword) {
                score += 0.35;
            }
        }

        for keyword in &test_keywords {
            if input.contains(keyword) {
                score += 0.3;
            }
        }

        for keyword in &enable_keywords {
            if input.contains(keyword) {
                score += 0.25;
            }
        }

        (score as f32).min(1.0).max(0.0)
    }

    /// 构建澄清结果
    fn build_clarification_result(&self, input: &str) -> IntentClassification {
        let followup_prompt = if input.len() < 3 {
            Some("请问你想了解或操作什么？你可以问我关于设备状态、控制设备、创建自动化规则等问题。".to_string())
        } else {
            Some(format!("不太确定'{}'的具体意思，能否提供更多细节？", input.trim()))
        };

        IntentClassification {
            intent: IntentCategory::Clarify,
            sub_type: IntentSubType::AmbiguousInput,
            confidence: 0.5,
            entities: vec![],
            strategy: ProcessingStrategy::Standard,
            needs_followup: true,
            followup_prompt,
            capability_statement: None,
        }
    }

    /// 构建分类结果
    fn build_classification_result(&self, input: &str, intent: IntentCategory, confidence: f32) -> IntentClassification {
        let input_lower = input.to_lowercase();

        // 根据意图和输入内容确定子类型和策略
        let (sub_type, strategy, needs_followup, followup_prompt) = match intent {
            IntentCategory::QueryData => {
                // 检测是否是设备列表
                if input.contains("列表") || input.contains("所有") || input.contains("全部") {
                    (IntentSubType::DeviceList, ProcessingStrategy::FastPath, false, None)
                } else {
                    (IntentSubType::CurrentStatus, ProcessingStrategy::FastPath, false, None)
                }
            }
            IntentCategory::AnalyzeData => (
                IntentSubType::TrendAnalysis,
                ProcessingStrategy::Quality,
                false,
                None,
            ),
            IntentCategory::ControlDevice => {
                // 检测场景模式
                if input.contains("模式") || input.contains("场景") || input_lower.contains("mode") || input_lower.contains("scene") {
                    (IntentSubType::SceneMode, ProcessingStrategy::Standard, false, None)
                } else if input.contains("所有") || input.contains("全部") || input_lower.contains("all") {
                    (IntentSubType::BatchControl, ProcessingStrategy::Standard, false, None)
                } else {
                    (IntentSubType::SingleDevice, ProcessingStrategy::Standard, false, None)
                }
            }
            IntentCategory::CreateAutomation => (
                IntentSubType::SimpleRule,
                ProcessingStrategy::MultiTurn,
                true,
                Some("我来帮你创建自动化，需要确认几个细节。".to_string()),
            ),
            IntentCategory::SendMessage => (
                IntentSubType::Notification,
                ProcessingStrategy::Standard,
                false,
                None,
            ),
            IntentCategory::SummarizeInfo => (
                IntentSubType::DataSummary,
                ProcessingStrategy::Quality,
                false,
                None,
            ),
            IntentCategory::AgentMonitor => {
                // 检测具体子类型
                if input.contains("历史") || input_lower.contains("history") {
                    (IntentSubType::ExecutionHistory, ProcessingStrategy::FastPath, false, None)
                } else if input.contains("决策") || input_lower.contains("decision") || input_lower.contains("reasoning") {
                    (IntentSubType::DecisionProcess, ProcessingStrategy::Quality, false, None)
                } else if input.contains("性能") || input_lower.contains("performance") || input_lower.contains("stats") {
                    (IntentSubType::PerformanceStats, ProcessingStrategy::FastPath, false, None)
                } else {
                    (IntentSubType::AgentStatus, ProcessingStrategy::FastPath, false, None)
                }
            }
            IntentCategory::AlertChannel => {
                // 检测具体子类型
                if input.contains("测试") || input_lower.contains("test") {
                    (IntentSubType::ChannelTest, ProcessingStrategy::Standard, false, None)
                } else if input.contains("配置") || input_lower.contains("config") || input_lower.contains("设置") {
                    (IntentSubType::ChannelConfig, ProcessingStrategy::MultiTurn, true, Some("我来帮你配置告警通道。".to_string()))
                } else if input.contains("启用") || input.contains("禁用") || input_lower.contains("enable") || input_lower.contains("disable") {
                    (IntentSubType::ChannelEnable, ProcessingStrategy::Standard, false, None)
                } else if input.contains("列表") || input.contains("所有") || input_lower.contains("list") {
                    (IntentSubType::ChannelList, ProcessingStrategy::FastPath, false, None)
                } else {
                    (IntentSubType::ChannelList, ProcessingStrategy::FastPath, false, None)
                }
            }
            _ => (
                IntentSubType::Unknown,
                ProcessingStrategy::Standard,
                false,
                None,
            ),
        };

        // 提取实体
        let entities = self.extract_entities(input, intent);

        IntentClassification {
            intent,
            sub_type,
            confidence,
            entities,
            strategy,
            needs_followup,
            followup_prompt,
            capability_statement: None,
        }
    }

    /// 提取实体
    fn extract_entities(&self, input: &str, _intent: IntentCategory) -> Vec<Entity> {
        let mut entities = Vec::new();

        // 常见位置
        let locations = ["客厅", "卧室", "厨房", "书房", "浴室", "阳台", "车库", "garden", "living room", "bedroom", "kitchen"];
        for loc in &locations {
            if input.contains(loc) {
                entities.push(Entity {
                    entity_type: EntityType::Location,
                    value: loc.to_string(),
                    position: input.find(loc),
                    confidence: 0.9,
                });
            }
        }

        // 常见设备
        let devices = ["灯", "空调", "窗帘", "门锁", "开关", "风扇", "电视", "light", "ac", "curtain", "lock"];
        for dev in &devices {
            if input.contains(dev) {
                entities.push(Entity {
                    entity_type: EntityType::Device,
                    value: dev.to_string(),
                    position: input.find(dev),
                    confidence: 0.85,
                });
            }
        }

        // 数值提取
        if let Some(num_pos) = input.find(char::is_numeric) {
            let num_str: String = input[num_pos..]
                .chars()
                .take_while(|c| c.is_numeric() || *c == '.' || *c == '-' || *c == '°' || *c == 'C')
                .collect();
            if !num_str.is_empty() {
                entities.push(Entity {
                    entity_type: EntityType::Value,
                    value: num_str,
                    position: Some(num_pos),
                    confidence: 0.95,
                });
            }
        }

        // 时间范围
        if input.contains("最近") || input.contains("今天") {
            entities.push(Entity {
                entity_type: EntityType::TimeRange,
                value: "today".to_string(),
                position: None,
                confidence: 0.8,
            });
        }

        if input.contains("本周") || input.contains("一周") || input.contains("7天") {
            entities.push(Entity {
                entity_type: EntityType::TimeRange,
                value: "week".to_string(),
                position: None,
                confidence: 0.8,
            });
        }

        // 动作提取
        let actions = ["打开", "关闭", "开启", "停止", "调节", "open", "close", "turn on", "turn off", "adjust"];
        for action in &actions {
            if input.contains(action) {
                entities.push(Entity {
                    entity_type: EntityType::Action,
                    value: action.to_string(),
                    position: input.find(action),
                    confidence: 0.9,
                });
            }
        }

        entities
    }

    /// 获取意图处理建议
    pub fn get_processing_advice(&self, classification: &IntentClassification) -> ProcessingAdvice {
        ProcessingAdvice {
            recommended_model: match classification.strategy {
                ProcessingStrategy::FastPath => "fast_local".to_string(),
                ProcessingStrategy::Standard => "balanced_local".to_string(),
                ProcessingStrategy::Quality => "high_quality".to_string(),
                ProcessingStrategy::MultiTurn => "high_quality".to_string(),
                ProcessingStrategy::Fallback => "any".to_string(),
            },
            requires_tools: matches!(
                classification.intent,
                IntentCategory::QueryData | IntentCategory::ControlDevice | IntentCategory::CreateAutomation
            ),
            suggested_tools: self.suggest_tools(classification),
            estimated_complexity: match (classification.intent, classification.sub_type) {
                (IntentCategory::ControlDevice, IntentSubType::SingleDevice) => "simple".to_string(),
                (IntentCategory::QueryData, IntentSubType::CurrentStatus) => "simple".to_string(),
                (IntentCategory::QueryData, IntentSubType::DeviceList) => "simple".to_string(),
                _ => match classification.strategy {
                    ProcessingStrategy::FastPath => "simple".to_string(),
                    ProcessingStrategy::Standard => "medium".to_string(),
                    ProcessingStrategy::Quality => "medium".to_string(),
                    ProcessingStrategy::MultiTurn => "complex".to_string(),
                    ProcessingStrategy::Fallback => "n/a".to_string(),
                },
            },
        }
    }

    /// 建议使用的工具
    fn suggest_tools(&self, classification: &IntentClassification) -> Vec<String> {
        match classification.intent {
            IntentCategory::QueryData => vec![
                "DeviceQueryTool".to_string(),
                "ListDevicesTool".to_string(),
            ],
            IntentCategory::AnalyzeData => vec![
                "DeviceQueryTool".to_string(),
                "DeviceAnalyzeTool".to_string(),
            ],
            IntentCategory::ControlDevice => vec![
                "DeviceControlTool".to_string(),
            ],
            IntentCategory::CreateAutomation => vec![
                "CreateAutomationTool".to_string(),
            ],
            IntentCategory::SendMessage => vec![
                "SendMessageTool".to_string(),
            ],
            IntentCategory::SummarizeInfo => vec![
                "ListDevicesTool".to_string(),
                "DeviceAnalyzeTool".to_string(),
            ],
            _ => vec![],
        }
    }
}

/// 处理建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingAdvice {
    /// 推荐使用的模型
    pub recommended_model: String,
    /// 是否需要工具调用
    pub requires_tools: bool,
    /// 建议的工具列表
    pub suggested_tools: Vec<String>,
    /// 预估复杂度
    pub estimated_complexity: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_classifier() -> IntentClassifier {
        IntentClassifier::new()
    }

    #[test]
    fn test_query_data_intent() {
        let classifier = create_classifier();

        let result = classifier.classify("客厅温度多少");
        assert_eq!(result.intent, IntentCategory::QueryData);
        assert!(result.confidence > 0.3);

        let result = classifier.classify("查看设备状态");
        assert_eq!(result.intent, IntentCategory::QueryData);
    }

    #[test]
    fn test_analyze_data_intent() {
        let classifier = create_classifier();

        let result = classifier.classify("分析最近一周的温度趋势");
        assert_eq!(result.intent, IntentCategory::AnalyzeData);
        assert!(result.confidence > 0.4);
    }

    #[test]
    fn test_control_device_intent() {
        let classifier = create_classifier();

        let result = classifier.classify("打开客厅灯");
        assert_eq!(result.intent, IntentCategory::ControlDevice);

        let result = classifier.classify("关闭空调");
        assert_eq!(result.intent, IntentCategory::ControlDevice);
    }

    #[test]
    fn test_create_automation_intent() {
        let classifier = create_classifier();

        let result = classifier.classify("当温度超过28度时打开空调");
        assert_eq!(result.intent, IntentCategory::CreateAutomation);
        assert!(result.confidence > 0.4);

        let result = classifier.classify("创建一个自动化规则");
        assert_eq!(result.intent, IntentCategory::CreateAutomation);
    }

    #[test]
    fn test_summarize_info_intent() {
        let classifier = create_classifier();

        let result = classifier.classify("汇总所有设备的状态");
        assert_eq!(result.intent, IntentCategory::SummarizeInfo);
    }

    #[test]
    fn test_clarify_intent() {
        let classifier = create_classifier();

        // 太短的输入
        let result = classifier.classify("打开");
        assert_eq!(result.intent, IntentCategory::Clarify);
        assert!(result.needs_followup);

        // 单个模糊词
        let result = classifier.classify("查询");
        assert_eq!(result.intent, IntentCategory::Clarify);
    }

    #[test]
    fn test_out_of_scope_intent() {
        let classifier = create_classifier();

        let result = classifier.classify("帮我安装一个新的温度传感器");
        assert_eq!(result.intent, IntentCategory::OutOfScope);
        assert!(result.capability_statement.is_some());
    }

    #[test]
    fn test_entity_extraction() {
        let classifier = create_classifier();

        let result = classifier.classify("客厅温度是多少");
        assert!(!result.entities.is_empty());

        // 检查位置实体
        let has_location = result.entities.iter().any(|e| e.entity_type == EntityType::Location);
        assert!(has_location);
    }

    #[test]
    fn test_processing_strategy() {
        let classifier = create_classifier();

        // 查询数据应该用快速路径
        let result = classifier.classify("客厅温度");
        assert_eq!(result.strategy, ProcessingStrategy::FastPath);

        // 创建自动化需要多轮对话
        let result = classifier.classify("当温度高时打开空调");
        assert_eq!(result.strategy, ProcessingStrategy::MultiTurn);
    }

    #[test]
    fn test_intent_display_name() {
        assert_eq!(IntentCategory::QueryData.display_name(), "查询数据");
        assert_eq!(IntentCategory::ControlDevice.display_name(), "控制设备");
        assert_eq!(IntentCategory::CreateAutomation.display_name(), "创建自动化");
    }

    #[test]
    fn test_intent_from_str() {
        assert_eq!(IntentCategory::from_str("query_data"), Some(IntentCategory::QueryData));
        assert_eq!(IntentCategory::from_str("control_device"), Some(IntentCategory::ControlDevice));
        assert_eq!(IntentCategory::from_str("invalid"), None);
    }

    #[test]
    fn test_processing_advice() {
        let classifier = create_classifier();
        let result = classifier.classify("打开客厅灯");
        let advice = classifier.get_processing_advice(&result);

        assert!(advice.requires_tools);
        assert!(!advice.suggested_tools.is_empty());
        assert_eq!(advice.estimated_complexity, "simple");
    }

    #[test]
    fn test_confidence_threshold() {
        let classifier = IntentClassifier::new().with_confidence_threshold(0.8);

        // 模糊输入应该返回澄清
        let result = classifier.classify("嗯");
        assert_eq!(result.intent, IntentCategory::Clarify);
    }

    #[test]
    fn test_send_message_intent() {
        let classifier = create_classifier();

        let result = classifier.classify("发送温度报告到邮箱");
        assert_eq!(result.intent, IntentCategory::SendMessage);
    }

    #[test]
    fn test_scene_mode_detection() {
        let classifier = create_classifier();

        let result = classifier.classify("开启回家模式");
        assert_eq!(result.intent, IntentCategory::ControlDevice);
        assert_eq!(result.sub_type, IntentSubType::SceneMode);
    }

    #[test]
    fn test_time_range_extraction() {
        let classifier = create_classifier();

        let result = classifier.classify("分析今天的温度数据");
        let has_time_range = result.entities.iter().any(|e| {
            e.entity_type == EntityType::TimeRange && e.value == "today"
        });
        assert!(has_time_range);
    }

    #[test]
    fn test_value_extraction() {
        let classifier = create_classifier();

        let result = classifier.classify("设置温度为25度");
        let has_value = result.entities.iter().any(|e| {
            e.entity_type == EntityType::Value && e.value.contains("25")
        });
        assert!(has_value);
    }

    #[test]
    fn test_action_extraction() {
        let classifier = create_classifier();

        let result = classifier.classify("打开客厅灯");
        let has_action = result.entities.iter().any(|e| {
            e.entity_type == EntityType::Action && e.value == "打开"
        });
        assert!(has_action);
    }

    #[test]
    fn test_complex_automation_detection() {
        let classifier = create_classifier();

        let result = classifier.classify("如果温度超过28度且有人在，则打开空调并发送通知");
        assert_eq!(result.intent, IntentCategory::CreateAutomation);
        assert!(result.confidence > 0.5);
    }
}
