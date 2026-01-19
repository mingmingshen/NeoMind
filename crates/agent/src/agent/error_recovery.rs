//! 智能错误恢复模块
//!
//! 功能：
//! 1. 错误分类与友好消息转换
//! 2. 自动恢复策略
//! 3. 降级执行建议
//! 4. 错误历史追踪

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 错误类别
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// 网络错误
    Network,
    /// 设备错误
    Device,
    /// 认证/授权错误
    Auth,
    /// 资源不可用
    ResourceUnavailable,
    /// 超时错误
    Timeout,
    /// 数据格式错误
    DataFormat,
    /// LLM 错误
    Llm,
    /// 工具执行错误
    ToolExecution,
    /// 未知错误
    Unknown,
}

/// 恢复策略
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// 无需恢复，直接返回
    None,
    /// 重试操作
    Retry,
    /// 使用降级方案
    Fallback,
    /// 跳过当前操作
    Skip,
    /// 请求用户输入
    UserInput,
    /// 重启服务
    RestartService,
}

/// 恢复动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    /// 策略类型
    pub strategy: RecoveryStrategy,
    /// 动作描述
    pub description: String,
    /// 是否自动执行
    pub automatic: bool,
    /// 预计恢复时间（毫秒）
    pub estimated_duration_ms: Option<u64>,
}

/// 错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// 错误类别
    pub category: ErrorCategory,
    /// 原始错误消息
    pub raw_message: String,
    /// 友好的错误描述
    pub friendly_message: String,
    /// 建议的恢复动作
    pub recovery_action: RecoveryAction,
    /// 是否可恢复
    pub recoverable: bool,
    /// 错误代码（如果有）
    pub error_code: Option<String>,
}

/// 降级方案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackPlan {
    /// 降级方案名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 替代方案
    pub alternatives: Vec<String>,
    /// 是否影响功能
    pub affects_functionality: bool,
}

/// 错误恢复配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    /// 最大重试次数
    pub max_auto_retries: usize,
    /// 是否启用友好消息
    pub enable_friendly_messages: bool,
    /// 是否记录错误历史
    pub track_history: bool,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            max_auto_retries: 2,
            enable_friendly_messages: true,
            track_history: true,
        }
    }
}

/// 错误统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorStats {
    /// 各类别错误计数
    pub error_counts: HashMap<ErrorCategory, usize>,
    /// 总错误数
    pub total_errors: usize,
    /// 恢复成功数
    pub recovered_count: usize,
}

/// 智能错误恢复管理器
pub struct ErrorRecoveryManager {
    /// 配置
    config: ErrorRecoveryConfig,
    /// 错误统计
    stats: Arc<RwLock<ErrorStats>>,
    /// 错误历史（最近100条）
    error_history: Arc<RwLock<Vec<ErrorInfo>>>,
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        Self {
            config: ErrorRecoveryConfig::default(),
            stats: Arc::new(RwLock::new(ErrorStats::default())),
            error_history: Arc::new(RwLock::new(Vec::with_capacity(100))),
        }
    }

    pub fn with_config(config: ErrorRecoveryConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(ErrorStats::default())),
            error_history: Arc::new(RwLock::new(Vec::with_capacity(100))),
        }
    }

    /// 分析错误并生成恢复信息
    pub async fn analyze_error(&self, error: &str, context: Option<&str>) -> ErrorInfo {
        let category = self.classify_error(error);
        let friendly_message = self.generate_friendly_message(&category, error);
        let recovery_action = self.determine_recovery_action(&category, error);
        let recoverable = recovery_action.strategy != RecoveryStrategy::None;
        let error_code = self.extract_error_code(error);

        ErrorInfo {
            category,
            raw_message: error.to_string(),
            friendly_message,
            recovery_action,
            recoverable,
            error_code,
        }
    }

    /// 错误分类
    fn classify_error(&self, error: &str) -> ErrorCategory {
        let lower = error.to_lowercase();

        // 超时错误（优先检查，因为超时也可能出现在网络请求中）
        if lower.contains("timeout") || lower.contains("超时") {
            return ErrorCategory::Timeout;
        }

        // 网络错误
        if lower.contains("network")
            || lower.contains("connection")
            || lower.contains("网络")
            || lower.contains("连接")
        {
            return ErrorCategory::Network;
        }

        // 设备错误
        if lower.contains("device")
            || lower.contains("设备")
            || lower.contains("sensor")
            || lower.contains("传感器")
        {
            return ErrorCategory::Device;
        }

        // 认证错误
        if lower.contains("auth")
            || lower.contains("unauthorized")
            || lower.contains("forbidden")
            || lower.contains("认证")
            || lower.contains("未授权")
        {
            return ErrorCategory::Auth;
        }

        // 资源不可用
        if lower.contains("not found")
            || lower.contains("unavailable")
            || lower.contains("不存在")
            || lower.contains("不可用")
        {
            return ErrorCategory::ResourceUnavailable;
        }

        // 数据格式错误
        if lower.contains("parse")
            || lower.contains("format")
            || lower.contains("invalid")
            || lower.contains("格式")
            || lower.contains("解析")
        {
            return ErrorCategory::DataFormat;
        }

        // LLM 错误
        if lower.contains("llm")
            || lower.contains("model")
            || lower.contains("generation")
            || lower.contains("token")
        {
            return ErrorCategory::Llm;
        }

        // 工具执行错误
        if lower.contains("tool")
            || lower.contains("function")
            || lower.contains("execution")
        {
            return ErrorCategory::ToolExecution;
        }

        ErrorCategory::Unknown
    }

    /// 生成友好的错误消息
    fn generate_friendly_message(&self, category: &ErrorCategory, original: &str) -> String {
        match category {
            ErrorCategory::Network => {
                "网络连接出现问题。请检查网络连接或稍后重试。".to_string()
            }
            ErrorCategory::Device => {
                "设备响应异常。设备可能离线或故障，请检查设备状态。".to_string()
            }
            ErrorCategory::Auth => {
                "认证失败。请检查登录凭据或重新登录。".to_string()
            }
            ErrorCategory::ResourceUnavailable => {
                "请求的资源暂时不可用。请确认资源是否存在或稍后重试。".to_string()
            }
            ErrorCategory::Timeout => {
                "操作超时。请求处理时间过长，请稍后重试。".to_string()
            }
            ErrorCategory::DataFormat => {
                "数据格式不正确。请检查输入数据的格式。".to_string()
            }
            ErrorCategory::Llm => {
                "AI 模型处理出现问题。系统正在尝试恢复...".to_string()
            }
            ErrorCategory::ToolExecution => {
                "工具执行失败。系统正在尝试其他方法...".to_string()
            }
            ErrorCategory::Unknown => {
                format!("发生了一个错误：{}", original)
            }
        }
    }

    /// 确定恢复动作
    fn determine_recovery_action(&self, category: &ErrorCategory, _error: &str) -> RecoveryAction {
        match category {
            ErrorCategory::Network => RecoveryAction {
                strategy: RecoveryStrategy::Retry,
                description: "系统将自动重试连接".to_string(),
                automatic: true,
                estimated_duration_ms: Some(2000),
            },
            ErrorCategory::Device => RecoveryAction {
                strategy: RecoveryStrategy::Fallback,
                description: "可以尝试使用其他设备或稍后重试".to_string(),
                automatic: false,
                estimated_duration_ms: None,
            },
            ErrorCategory::Auth => RecoveryAction {
                strategy: RecoveryStrategy::UserInput,
                description: "需要重新登录或提供认证信息".to_string(),
                automatic: false,
                estimated_duration_ms: None,
            },
            ErrorCategory::ResourceUnavailable => RecoveryAction {
                strategy: RecoveryStrategy::Skip,
                description: "跳过当前操作，继续执行其他任务".to_string(),
                automatic: true,
                estimated_duration_ms: None,
            },
            ErrorCategory::Timeout => RecoveryAction {
                strategy: RecoveryStrategy::Retry,
                description: "系统将使用更长的超时时间重试".to_string(),
                automatic: true,
                estimated_duration_ms: Some(5000),
            },
            ErrorCategory::DataFormat => RecoveryAction {
                strategy: RecoveryStrategy::UserInput,
                description: "请提供正确格式的数据".to_string(),
                automatic: false,
                estimated_duration_ms: None,
            },
            ErrorCategory::Llm => RecoveryAction {
                strategy: RecoveryStrategy::Fallback,
                description: "尝试使用备用模型或简化请求".to_string(),
                automatic: true,
                estimated_duration_ms: Some(3000),
            },
            ErrorCategory::ToolExecution => RecoveryAction {
                strategy: RecoveryStrategy::Fallback,
                description: "尝试使用其他工具或手动操作".to_string(),
                automatic: true,
                estimated_duration_ms: Some(1000),
            },
            ErrorCategory::Unknown => RecoveryAction {
                strategy: RecoveryStrategy::None,
                description: "未知错误类型，请联系技术支持".to_string(),
                automatic: false,
                estimated_duration_ms: None,
            },
        }
    }

    /// 提取错误代码
    fn extract_error_code(&self, error: &str) -> Option<String> {
        // 尝试匹配错误代码模式 (E001, ERR_001, etc.)
        // 使用更精确的模式匹配
        for (pos, ch) in error.char_indices() {
            if ch == 'E' || ch == 'e' {
                let remaining = &error[pos..];
                if remaining.len() >= 4 {
                    let next_3 = remaining.chars().skip(1).take(3).collect::<String>();
                    if next_3.chars().all(|c| c.is_ascii_digit()) {
                        return Some(format!("{}{}", ch, next_3));
                    }
                }
            }
        }

        // HTTP 状态码
        if error.contains("404") {
            return Some("404".to_string());
        }
        if error.contains("500") {
            return Some("500".to_string());
        }
        if error.contains("503") {
            return Some("503".to_string());
        }

        None
    }

    /// 生成降级方案
    pub fn generate_fallback_plan(&self, error_info: &ErrorInfo) -> Option<FallbackPlan> {
        match error_info.category {
            ErrorCategory::Device => Some(FallbackPlan {
                name: "设备降级方案".to_string(),
                description: "目标设备不可用时的替代方案".to_string(),
                alternatives: vec![
                    "使用备用设备".to_string(),
                    "手动控制".to_string(),
                    "稍后重试".to_string(),
                ],
                affects_functionality: true,
            }),
            ErrorCategory::Llm => Some(FallbackPlan {
                name: "AI 降级方案".to_string(),
                description: "AI 模型不可用时的替代方案".to_string(),
                alternatives: vec![
                    "使用本地规则引擎".to_string(),
                    "使用预设回复".to_string(),
                    "切换到备用模型".to_string(),
                ],
                affects_functionality: true,
            }),
            ErrorCategory::Network => Some(FallbackPlan {
                name: "网络降级方案".to_string(),
                description: "网络不可用时的替代方案".to_string(),
                alternatives: vec![
                    "使用本地缓存数据".to_string(),
                    "启用离线模式".to_string(),
                    "等待网络恢复".to_string(),
                ],
                affects_functionality: false,
            }),
            _ => None,
        }
    }

    /// 记录错误
    pub async fn record_error(&self, error_info: ErrorInfo) {
        // 更新统计
        let mut stats = self.stats.write().await;
        *stats.error_counts.entry(error_info.category.clone()).or_insert(0) += 1;
        stats.total_errors += 1;

        // 添加到历史
        if self.config.track_history {
            let mut history = self.error_history.write().await;
            history.push(error_info);
            // 限制历史大小
            if history.len() > 100 {
                history.remove(0);
            }
        }
    }

    /// 获取错误统计
    pub async fn get_stats(&self) -> ErrorStats {
        self.stats.read().await.clone()
    }

    /// 获取错误历史
    pub async fn get_error_history(&self) -> Vec<ErrorInfo> {
        self.error_history.read().await.clone()
    }

    /// 获取特定类别的错误数
    pub async fn get_error_count(&self, category: &ErrorCategory) -> usize {
        let stats = self.stats.read().await;
        stats.error_counts.get(category).copied().unwrap_or(0)
    }

    /// 清空错误历史
    pub async fn clear_history(&self) {
        self.error_history.write().await.clear();
    }

    /// 获取最近 N 个错误
    pub async fn get_recent_errors(&self, n: usize) -> Vec<ErrorInfo> {
        let history = self.error_history.read().await;
        let start = if history.len() > n { history.len() - n } else { 0 };
        history[start..].to_vec()
    }
}

impl Default for ErrorRecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_manager() -> ErrorRecoveryManager {
        ErrorRecoveryManager::new()
    }

    #[test]
    fn test_classify_network_error() {
        let manager = create_manager();

        assert_eq!(
            manager.classify_error("Network connection failed"),
            ErrorCategory::Network
        );
        assert_eq!(manager.classify_error("网络连接失败"), ErrorCategory::Network);
    }

    #[test]
    fn test_classify_device_error() {
        let manager = create_manager();

        assert_eq!(
            manager.classify_error("Device not responding"),
            ErrorCategory::Device
        );
        assert_eq!(manager.classify_error("设备离线"), ErrorCategory::Device);
    }

    #[test]
    fn test_classify_auth_error() {
        let manager = create_manager();

        assert_eq!(
            manager.classify_error("Unauthorized access"),
            ErrorCategory::Auth
        );
        assert_eq!(manager.classify_error("认证失败"), ErrorCategory::Auth);
    }

    #[test]
    fn test_classify_timeout_error() {
        let manager = create_manager();

        assert_eq!(manager.classify_error("Request timeout"), ErrorCategory::Timeout);
        assert_eq!(manager.classify_error("操作超时"), ErrorCategory::Timeout);
    }

    #[test]
    fn test_classify_llm_error() {
        let manager = create_manager();

        assert_eq!(
            manager.classify_error("LLM generation failed"),
            ErrorCategory::Llm
        );
    }

    #[test]
    fn test_friendly_message_generation() {
        let manager = create_manager();

        let msg = manager.generate_friendly_message(&ErrorCategory::Network, "Connection failed");
        assert!(msg.contains("网络"));

        let msg = manager.generate_friendly_message(&ErrorCategory::Device, "Device offline");
        assert!(msg.contains("设备"));
    }

    #[test]
    fn test_recovery_action_determination() {
        let manager = create_manager();

        let action = manager.determine_recovery_action(&ErrorCategory::Network, "timeout");
        assert_eq!(action.strategy, RecoveryStrategy::Retry);
        assert!(action.automatic);

        let action = manager.determine_recovery_action(&ErrorCategory::Auth, "unauthorized");
        assert_eq!(action.strategy, RecoveryStrategy::UserInput);
        assert!(!action.automatic);
    }

    #[test]
    fn test_fallback_plan_generation() {
        let manager = create_manager();

        let error_info = ErrorInfo {
            category: ErrorCategory::Device,
            raw_message: "Device offline".to_string(),
            friendly_message: "设备离线".to_string(),
            recovery_action: RecoveryAction {
                strategy: RecoveryStrategy::Fallback,
                description: "降级".to_string(),
                automatic: false,
                estimated_duration_ms: None,
            },
            recoverable: true,
            error_code: None,
        };

        let plan = manager.generate_fallback_plan(&error_info);
        assert!(plan.is_some());
        assert!(plan.unwrap().alternatives.len() > 0);
    }

    #[test]
    fn test_error_code_extraction() {
        let manager = create_manager();

        assert_eq!(manager.extract_error_code("Error E001: something"), Some("E001".to_string()));
        assert_eq!(manager.extract_error_code("404 Not Found"), Some("404".to_string()));
        assert_eq!(manager.extract_error_code("Internal Server Error"), None);
    }

    #[tokio::test]
    async fn test_error_recording() {
        let manager = create_manager();

        let error_info = ErrorInfo {
            category: ErrorCategory::Network,
            raw_message: "Network error".to_string(),
            friendly_message: "网络错误".to_string(),
            recovery_action: RecoveryAction {
                strategy: RecoveryStrategy::Retry,
                description: "重试".to_string(),
                automatic: true,
                estimated_duration_ms: Some(1000),
            },
            recoverable: true,
            error_code: Some("E001".to_string()),
        };

        manager.record_error(error_info).await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_errors, 1);
        assert_eq!(manager.get_error_count(&ErrorCategory::Network).await, 1);
    }

    #[tokio::test]
    async fn test_recent_errors() {
        let manager = create_manager();

        for i in 0..5 {
            let error_info = ErrorInfo {
                category: ErrorCategory::Network,
                raw_message: format!("Error {}", i),
                friendly_message: "网络错误".to_string(),
                recovery_action: RecoveryAction {
                    strategy: RecoveryStrategy::Retry,
                    description: "重试".to_string(),
                    automatic: true,
                    estimated_duration_ms: Some(1000),
                },
                recoverable: true,
                error_code: None,
            };
            manager.record_error(error_info).await;
        }

        let recent = manager.get_recent_errors(3).await;
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_unknown_error_classification() {
        let manager = create_manager();

        assert_eq!(
            manager.classify_error("Some unknown issue"),
            ErrorCategory::Unknown
        );
    }
}
