//! 工具执行置信度模块
//!
//! 功能：
//! 1. 评估工具调用结果的可信度
//! 2. 低置信度时自动重试
//! 3. 结果一致性验证
//! 4. 异常结果检测

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// 置信度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    /// 极低 (0-20%) - 不可信，需要重试或使用其他方法
    VeryLow = 0,
    /// 低 (20-40%) - 可能有问题，建议验证
    Low = 1,
    /// 中等 (40-60%) - 基本可信
    Medium = 2,
    /// 高 (60-80%) - 比较可信
    High = 3,
    /// 极高 (80-100%) - 完全可信
    VeryHigh = 4,
}

impl ConfidenceLevel {
    /// 从数值转换为置信度等级
    pub fn from_score(score: f32) -> Self {
        if score < 0.2 {
            Self::VeryLow
        } else if score < 0.4 {
            Self::Low
        } else if score < 0.6 {
            Self::Medium
        } else if score < 0.8 {
            Self::High
        } else {
            Self::VeryHigh
        }
    }

    /// 转换为数值
    pub fn as_score(&self) -> f32 {
        match self {
            Self::VeryLow => 0.1,
            Self::Low => 0.3,
            Self::Medium => 0.5,
            Self::High => 0.7,
            Self::VeryHigh => 0.9,
        }
    }

    /// 是否可接受（大于等于中等）
    pub fn is_acceptable(&self) -> bool {
        *self >= Self::Medium
    }
}

/// 工具执行状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// 成功
    Success,
    /// 失败但可重试
    RetryableFailure,
    /// 失败且不可重试
    PermanentFailure,
    /// 超时
    Timeout,
    /// 结果异常
    AbnormalResult,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// 工具名称
    pub tool_name: String,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 置信度
    pub confidence: ConfidenceLevel,
    /// 置信度分数 (0-1)
    pub confidence_score: f32,
    /// 结果内容
    pub content: String,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 错误信息（如果有）
    pub error_message: Option<String>,
    /// 是否为重试结果
    pub is_retry: bool,
    /// 重试次数
    pub retry_count: usize,
}

/// 置信度评估配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceConfig {
    /// 最大重试次数
    pub max_retries: usize,
    /// 重试延迟（毫秒）
    pub retry_delay_ms: u64,
    /// 超时阈值（毫秒）
    pub timeout_threshold_ms: u64,
    /// 是否启用结果验证
    pub enable_validation: bool,
}

impl Default for ConfidenceConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            retry_delay_ms: 100,
            timeout_threshold_ms: 5000,
            enable_validation: true,
        }
    }
}

/// 工具置信度管理器
pub struct ToolConfidenceManager {
    /// 配置
    config: ConfidenceConfig,
    /// 工具历史记录（用于一致性检测）
    tool_history: HashMap<String, Vec<ToolExecutionResult>>,
    /// 最大历史记录数
    max_history_size: usize,
}

impl ToolConfidenceManager {
    pub fn new() -> Self {
        Self {
            config: ConfidenceConfig::default(),
            tool_history: HashMap::new(),
            max_history_size: 10,
        }
    }

    pub fn with_config(config: ConfidenceConfig) -> Self {
        Self {
            config,
            tool_history: HashMap::new(),
            max_history_size: 10,
        }
    }

    /// 评估工具执行结果的置信度
    pub fn evaluate_result(
        &mut self,
        tool_name: &str,
        result_content: &str,
        duration_ms: u64,
        error: Option<&str>,
    ) -> ToolExecutionResult {
        let mut score = 0.5f32; // 基础分数
        let mut status = ExecutionStatus::Success;

        // 1. 检查是否有错误
        if let Some(err) = error {
            score -= 0.5;
            status = if self.is_retryable_error(err) {
                ExecutionStatus::RetryableFailure
            } else {
                ExecutionStatus::PermanentFailure
            };
        }

        // 2. 检查超时
        if duration_ms > self.config.timeout_threshold_ms {
            score -= 0.2;
            status = ExecutionStatus::Timeout;
        }

        // 3. 检查结果内容的完整性
        score += self.evaluate_content_completeness(tool_name, result_content);

        // 4. 检查结果格式
        score += self.evaluate_content_format(result_content);

        // 5. 检查异常模式
        if self.has_abnormal_patterns(result_content) {
            score -= 0.3;
            status = ExecutionStatus::AbnormalResult;
        }

        // 6. 与历史结果比较（一致性检查）
        score += self.evaluate_consistency(tool_name, result_content);

        // 限制分数在 0-1 范围内
        score = score.clamp(0.0, 1.0);

        let confidence = ConfidenceLevel::from_score(score);

        ToolExecutionResult {
            tool_name: tool_name.to_string(),
            status,
            confidence,
            confidence_score: score,
            content: result_content.to_string(),
            duration_ms,
            error_message: error.map(|e| e.to_string()),
            is_retry: false,
            retry_count: 0,
        }
    }

    /// 判断错误是否可重试
    fn is_retryable_error(&self, error: &str) -> bool {
        let retryable_keywords = [
            "timeout",
            "超时",
            "连接失败",
            "connection",
            "network",
            "网络",
            "临时",
            "temporary",
            "unavailable",
            "不可用",
        ];

        let lower = error.to_lowercase();
        retryable_keywords.iter().any(|keyword| lower.contains(keyword))
    }

    /// 评估内容完整性
    fn evaluate_content_completeness(&self, tool_name: &str, content: &str) -> f32 {
        let mut score = 0.0;

        // 检查内容是否为空
        if content.trim().is_empty() {
            return -0.5;
        }

        // 检查内容长度
        if content.len() > 10 {
            score += 0.1;
        }
        if content.len() > 50 {
            score += 0.1;
        }

        // 特定工具的完整性检查
        match tool_name {
            "list_devices" => {
                // 设备列表应该包含设备名称
                if content.contains("设备") || content.contains("Device") || content.contains(":") {
                    score += 0.2;
                }
            }
            "query_data" | "get_device" => {
                // 数据查询应该包含数值
                if content.chars().any(|c| c.is_ascii_digit()) {
                    score += 0.2;
                }
                // 检查是否包含单位
                if content.contains("°") || content.contains("%") || content.contains("℃") {
                    score += 0.1;
                }
            }
            _ => {}
        }

        score
    }

    /// 评估内容格式
    fn evaluate_content_format(&self, content: &str) -> f32 {
        let mut score = 0.0;

        // 检查是否为有效 JSON
        if serde_json::from_str::<serde_json::Value>(content).is_ok() {
            return 0.3; // JSON 格式加分
        }

        // 检查是否为结构化文本（包含分隔符）
        if content.contains(":") || content.contains("=") || content.contains("：") {
            score += 0.1;
        }

        // 检查是否为错误信息（降低分数）
        if content.starts_with("Error:") || content.starts_with("错误") {
            score -= 0.3;
        }

        score
    }

    /// 检查异常模式
    fn has_abnormal_patterns(&self, content: &str) -> bool {
        let abnormal_patterns = [
            "null",
            "undefined",
            "n/a",
            "错误",
            "error",
            "failed",
            "失败",
            "[object Object]",
            "未找到",
            "not found",
        ];

        let lower = content.to_lowercase();
        // 如果只包含异常模式且内容很少，认为是异常
        if content.len() < 50 {
            abnormal_patterns.iter().any(|p| lower.contains(p))
        } else {
            false
        }
    }

    /// 评估结果一致性（与历史记录比较）
    fn evaluate_consistency(&mut self, tool_name: &str, content: &str) -> f32 {
        if let Some(history) = self.tool_history.get(tool_name)
            && !history.is_empty() {
                let last_result = &history[history.len() - 1];

                // 如果结果完全相同，认为是稳定的
                if last_result.content == content {
                    return 0.1;
                }

                // 如果内容长度差异太大，可能是异常
                let length_diff = (last_result.content.len() as i64 - content.len() as i64).abs();
                if length_diff > last_result.content.len() as i64 / 2 {
                    return -0.1;
                }
            }
        0.0
    }

    /// 记录工具执行结果到历史
    pub fn record_result(&mut self, result: ToolExecutionResult) {
        let tool_name = result.tool_name.clone();
        let history = self.tool_history.entry(tool_name).or_default();

        history.push(result);
        // 限制历史记录大小
        if history.len() > self.max_history_size {
            history.remove(0);
        }
    }

    /// 获取工具的历史成功率
    pub fn get_success_rate(&self, tool_name: &str) -> Option<f32> {
        if let Some(history) = self.tool_history.get(tool_name) {
            if history.is_empty() {
                return None;
            }

            let success_count = history
                .iter()
                .filter(|r| r.status == ExecutionStatus::Success)
                .count();

            Some(success_count as f32 / history.len() as f32)
        } else {
            None
        }
    }

    /// 判断是否应该重试
    pub fn should_retry(&self, result: &ToolExecutionResult) -> bool {
        if result.retry_count >= self.config.max_retries {
            return false;
        }

        match result.status {
            ExecutionStatus::RetryableFailure => true,
            ExecutionStatus::Timeout => true,
            ExecutionStatus::AbnormalResult => true,
            ExecutionStatus::Success => result.confidence < ConfidenceLevel::Medium,
            ExecutionStatus::PermanentFailure => false,
        }
    }

    /// 获取重试延迟
    pub fn get_retry_delay(&self, retry_count: usize) -> Duration {
        // 指数退避
        let delay_ms = self.config.retry_delay_ms * 2_u64.pow(retry_count as u32);
        Duration::from_millis(delay_ms.min(5000)) // 最多 5 秒
    }

    /// 创建带重试标记的结果
    pub fn create_retry_result(&self, original: &ToolExecutionResult) -> ToolExecutionResult {
        ToolExecutionResult {
            tool_name: original.tool_name.clone(),
            status: original.status.clone(),
            confidence: original.confidence,
            confidence_score: original.confidence_score,
            content: original.content.clone(),
            duration_ms: original.duration_ms,
            error_message: original.error_message.clone(),
            is_retry: true,
            retry_count: original.retry_count + 1,
        }
    }

    /// 清空历史记录
    pub fn clear_history(&mut self) {
        self.tool_history.clear();
    }

    /// 清空特定工具的历史记录
    pub fn clear_tool_history(&mut self, tool_name: &str) {
        self.tool_history.remove(tool_name);
    }
}

impl Default for ToolConfidenceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_manager() -> ToolConfidenceManager {
        ToolConfidenceManager::new()
    }

    #[test]
    fn test_confidence_level_from_score() {
        assert_eq!(ConfidenceLevel::from_score(0.1), ConfidenceLevel::VeryLow);
        assert_eq!(ConfidenceLevel::from_score(0.3), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.5), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.7), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(0.9), ConfidenceLevel::VeryHigh);
    }

    #[test]
    fn test_confidence_level_is_acceptable() {
        assert!(!ConfidenceLevel::VeryLow.is_acceptable());
        assert!(!ConfidenceLevel::Low.is_acceptable());
        assert!(ConfidenceLevel::Medium.is_acceptable());
        assert!(ConfidenceLevel::High.is_acceptable());
        assert!(ConfidenceLevel::VeryHigh.is_acceptable());
    }

    #[test]
    fn test_evaluate_success_result() {
        let mut manager = create_manager();

        let result = manager.evaluate_result(
            "list_devices",
            "客厅灯: 开\n卧室空调: 关",
            100,
            None,
        );

        assert_eq!(result.status, ExecutionStatus::Success);
        assert!(result.confidence >= ConfidenceLevel::Medium);
    }

    #[test]
    fn test_evaluate_error_result() {
        let mut manager = create_manager();

        let result = manager.evaluate_result(
            "list_devices",
            "",
            100,
            Some("连接失败"),
        );

        assert_eq!(result.status, ExecutionStatus::RetryableFailure);
        assert!(result.confidence < ConfidenceLevel::Medium);
    }

    #[test]
    fn test_evaluate_timeout() {
        let mut manager = create_manager();

        let result = manager.evaluate_result(
            "query_data",
            "timeout",
            6000, // 超过默认阈值 5000ms
            None,
        );

        assert_eq!(result.status, ExecutionStatus::Timeout);
    }

    #[test]
    fn test_evaluate_empty_result() {
        let mut manager = create_manager();

        let result = manager.evaluate_result(
            "get_device",
            "",
            100,
            None,
        );

        assert!(result.confidence_score < 0.5);
    }

    #[test]
    fn test_should_retry() {
        let manager = create_manager();

        // 可重试的失败
        let retryable = ToolExecutionResult {
            tool_name: "test".to_string(),
            status: ExecutionStatus::RetryableFailure,
            confidence: ConfidenceLevel::VeryLow,
            confidence_score: 0.1,
            content: "".to_string(),
            duration_ms: 100,
            error_message: Some("timeout".to_string()),
            is_retry: false,
            retry_count: 0,
        };
        assert!(manager.should_retry(&retryable));

        // 永久失败
        let permanent = ToolExecutionResult {
            status: ExecutionStatus::PermanentFailure,
            ..retryable.clone()
        };
        assert!(!manager.should_retry(&permanent));

        // 超过最大重试次数
        let exceeded = ToolExecutionResult {
            retry_count: 3,
            ..retryable
        };
        assert!(!manager.should_retry(&exceeded));
    }

    #[test]
    fn test_retry_delay_exponential_backoff() {
        let manager = create_manager();

        assert_eq!(manager.get_retry_delay(0).as_millis(), 100);
        assert_eq!(manager.get_retry_delay(1).as_millis(), 200);
        assert_eq!(manager.get_retry_delay(2).as_millis(), 400);
        assert_eq!(manager.get_retry_delay(3).as_millis(), 800); // 不超过 5000
    }

    #[test]
    fn test_success_rate_tracking() {
        let mut manager = create_manager();

        // 记录多次成功
        for _ in 0..3 {
            let result = ToolExecutionResult {
                tool_name: "test_tool".to_string(),
                status: ExecutionStatus::Success,
                confidence: ConfidenceLevel::High,
                confidence_score: 0.8,
                content: "success".to_string(),
                duration_ms: 100,
                error_message: None,
                is_retry: false,
                retry_count: 0,
            };
            manager.record_result(result);
        }

        // 记录一次失败
        let fail_result = ToolExecutionResult {
            tool_name: "test_tool".to_string(),
            status: ExecutionStatus::PermanentFailure,
            confidence: ConfidenceLevel::VeryLow,
            confidence_score: 0.1,
            content: "".to_string(),
            duration_ms: 100,
            error_message: Some("error".to_string()),
            is_retry: false,
            retry_count: 0,
        };
        manager.record_result(fail_result);

        assert_eq!(manager.get_success_rate("test_tool"), Some(0.75));
    }

    #[test]
    fn test_abnormal_pattern_detection() {
        let manager = create_manager();

        // 短内容包含异常模式
        assert!(manager.has_abnormal_patterns("Error: device not found"));
        assert!(manager.has_abnormal_patterns("undefined"));
        assert!(manager.has_abnormal_patterns("N/A"));

        // 长内容即使包含异常模式也不算异常（超过50字符）
        let long_content = "Temperature: 25°C, Humidity: 60%, Status: OK, Sensor: working correctly, Error: none";
        assert!(!manager.has_abnormal_patterns(long_content));
    }

    #[test]
    fn test_content_format_evaluation() {
        let manager = create_manager();

        // JSON 格式
        assert!(manager.evaluate_content_format(r#"{"status": "ok"}"#) > 0.0);

        // 错误格式
        assert!(manager.evaluate_content_format("Error: something went wrong") < 0.0);

        // 结构化文本
        assert!(manager.evaluate_content_format("status: ok") > 0.0);
    }
}
