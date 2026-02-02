//! Agent状态机
//!
//! 定义Agent的完整生命周期状态和状态转换规则。
//!
//! ## 状态转换流程
//!
//! ```text
//!      ┌──────────┐
//!      │  Idle    │  ← 初始状态
//!      └────┬─────┘
//!           │ 用户消息
//!           ▼
//!      ┌──────────┐
//!      │Processing│ ← 处理输入
//!      └────┬─────┘
//!           │ 准备完成
//!           ▼
//!      ┌──────────┐
//!      │Generating│ ← LLM生成中
//!      └────┬─────┘
//!           │ 需要工具
//!           ▼
//!   ┌──────────────┐
//!   │ExecutingTools│ ← 执行工具
//!   └──────┬───────┘
//!          │ 完成/出错
//!          ▼
//!      ┌──────────┐
//!      │  Idle    │
//!      └──────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, Instant};

/// 处理状态（ProcessState）
///
/// 用于跟踪Agent的处理生命周期
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
    /// 空闲状态 - 等待用户输入
    Idle,

    /// 处理中 - 正在准备生成响应
    Processing,

    /// 生成中 - LLM正在生成响应
    Generating {
        /// 已生成的字符数
        chars_generated: usize,
    },

    /// 执行工具中
    ExecutingTools {
        /// 工具总数
        total_tools: usize,
        /// 已完成的工具数
        completed_tools: usize,
    },

    /// 错误状态
    Error {
        /// 错误消息
        message: String,
    },

    /// 关闭中
    Closing,

    /// 已关闭
    Closed,
}

impl ProcessState {
    /// 检查是否是活跃状态（非空闲/错误/关闭）
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ProcessState::Processing | ProcessState::Generating { .. } | ProcessState::ExecutingTools { .. }
        )
    }

    /// 检查是否是终端状态（无法再转换）
    pub fn is_terminal(&self) -> bool {
        matches!(self, ProcessState::Error { .. } | ProcessState::Closed)
    }

    /// 检查是否是错误状态
    pub fn is_error(&self) -> bool {
        matches!(self, ProcessState::Error { .. })
    }
}

impl fmt::Display for ProcessState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessState::Idle => write!(f, "Idle"),
            ProcessState::Processing => write!(f, "Processing"),
            ProcessState::Generating { chars_generated } => {
                write!(f, "Generating({} chars)", chars_generated)
            }
            ProcessState::ExecutingTools {
                total_tools,
                completed_tools,
            } => write!(f, "ExecutingTools({}/{})", completed_tools, total_tools),
            ProcessState::Error { message } => write!(f, "Error({})", message),
            ProcessState::Closing => write!(f, "Closing"),
            ProcessState::Closed => write!(f, "Closed"),
        }
    }
}

/// 状态转换事件
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransition {
    /// 开始处理
    StartProcessing,

    /// 开始生成
    StartGenerating,

    /// 生成进度更新
    GeneratingProgress { chars: usize },

    /// 开始执行工具
    StartToolExecution { total: usize },

    /// 工具执行完成
    ToolCompleted,

    /// 处理完成
    Complete,

    /// 发生错误
    Error(String),

    /// 开始关闭
    StartClosing,

    /// 关闭完成
    Closed,
}

/// Agent状态机
///
/// 管理Agent的状态转换，确保状态转换的合法性。
pub struct StateMachine {
    current: ProcessState,
    history: Vec<(ProcessState, Instant)>,
    max_history_size: usize,
}

impl StateMachine {
    /// 创建新的状态机
    pub fn new() -> Self {
        Self {
            current: ProcessState::Idle,
            history: Vec::new(),
            max_history_size: 100,
        }
    }

    /// 获取当前状态
    pub fn current(&self) -> &ProcessState {
        &self.current
    }

    /// 应用状态转换
    ///
    /// 返回转换是否成功
    pub fn transition(&mut self, event: StateTransition) -> Result<(), StateTransitionError> {
        use StateTransition::*;

        let new_state = match (&self.current, event.clone()) {
            // Idle -> Processing
            (ProcessState::Idle, StartProcessing) => ProcessState::Processing,

            // Processing -> Generating
            (ProcessState::Processing, StartGenerating) => ProcessState::Generating { chars_generated: 0 },

            // Processing -> ExecutingTools (无需LLM的工具调用)
            (ProcessState::Processing, StartToolExecution { total }) => ProcessState::ExecutingTools {
                total_tools: total,
                completed_tools: 0,
            },

            // Generating -> Generating (进度更新)
            (ProcessState::Generating { .. }, GeneratingProgress { chars }) => ProcessState::Generating {
                chars_generated: chars,
            },

            // Generating -> ExecutingTools (需要调用工具)
            (ProcessState::Generating { .. }, StartToolExecution { total }) => ProcessState::ExecutingTools {
                total_tools: total,
                completed_tools: 0,
            },

            // Generating -> Complete (直接完成)
            (ProcessState::Generating { .. }, Complete) => ProcessState::Idle,

            // ExecutingTools -> Generating (工具后继续生成)
            (ProcessState::ExecutingTools { .. }, StartGenerating) => ProcessState::Generating {
                chars_generated: 0,
            },

            // ExecutingTools -> ExecutingTools (工具进度)
            (ProcessState::ExecutingTools { total_tools, completed_tools }, ToolCompleted) => {
                if *completed_tools + 1 >= *total_tools {
                    // 所有工具完成，回到Processing进行后续处理
                    ProcessState::Processing
                } else {
                    ProcessState::ExecutingTools {
                        total_tools: *total_tools,
                        completed_tools: completed_tools + 1,
                    }
                }
            }

            // ExecutingTools -> Complete
            (ProcessState::ExecutingTools { .. }, Complete) => ProcessState::Idle,

            // Processing/Generating/ExecutingTools -> Error
            (ProcessState::Processing | ProcessState::Generating { .. } | ProcessState::ExecutingTools { .. }, Error(msg)) => {
                ProcessState::Error { message: msg }
            },

            // Any -> Closing
            (_, StartClosing) => ProcessState::Closing,

            // Closing -> Closed
            (ProcessState::Closing, Closed) => ProcessState::Closed,

            // 错误转换
            _ => {
                return Err(StateTransitionError::InvalidTransition {
                    from: self.current.clone(),
                    event,
                })
            }
        };

        // 记录历史
        self.history.push((self.current.clone(), Instant::now()));
        if self.history.len() > self.max_history_size {
            self.history.remove(0);
        }

        self.current = new_state;
        Ok(())
    }

    /// 获取当前状态的持续时间
    pub fn duration_in_state(&self) -> Duration {
        self.history
            .last()
            .map(|(_, instant)| instant.elapsed())
            .unwrap_or_else(|| Duration::ZERO)
    }

    /// 获取状态历史
    pub fn history(&self) -> &[(ProcessState, Instant)] {
        &self.history
    }

    /// 重置到空闲状态
    pub fn reset(&mut self) {
        self.current = ProcessState::Idle;
        self.history.clear();
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for StateMachine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StateMachine(current={}, history_len={})",
            self.current,
            self.history.len()
        )
    }
}

/// 状态转换错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransitionError {
    /// 无效的状态转换
    InvalidTransition {
        /// 源状态
        from: ProcessState,
        /// 尝试的事件
        event: StateTransition,
    },
}

impl fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateTransitionError::InvalidTransition { from, event } => {
                write!(
                    f,
                    "Invalid transition: {:?} + {:?}",
                    from, event
                )
            }
        }
    }
}

impl std::error::Error for StateTransitionError {}

/// 状态机配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineConfig {
    /// 状态历史最大记录数
    pub max_history_size: usize,

    /// Processing状态超时（秒）
    pub processing_timeout_secs: u64,

    /// Generating状态超时（秒）
    pub generating_timeout_secs: u64,

    /// ExecutingTools状态超时（秒）
    pub tool_execution_timeout_secs: u64,
}

impl Default for StateMachineConfig {
    fn default() -> Self {
        Self {
            max_history_size: 100,
            processing_timeout_secs: 30,
            generating_timeout_secs: 300, // 5 minutes for thinking models
            tool_execution_timeout_secs: 120,
        }
    }
}

/// 状态监控器
///
/// 监控状态机的健康状态
pub struct StateMonitor {
    config: StateMachineConfig,
}

impl StateMonitor {
    /// 创建新的状态监控器
    pub fn new(config: StateMachineConfig) -> Self {
        Self { config }
    }

    /// 创建默认配置的监控器
    pub fn default_config() -> Self {
        Self::new(StateMachineConfig::default())
    }

    /// 检查状态是否超时
    pub fn is_timeout(&self, state: &ProcessState, duration: Duration) -> bool {
        let timeout_secs = match state {
            ProcessState::Processing => self.config.processing_timeout_secs,
            ProcessState::Generating { .. } => self.config.generating_timeout_secs,
            ProcessState::ExecutingTools { .. } => self.config.tool_execution_timeout_secs,
            _ => return false,
        };

        duration.as_secs() > timeout_secs
    }

    /// 获取状态建议
    pub fn get_advice(&self, state: &ProcessState, duration: Duration) -> Option<&'static str> {
        if self.is_timeout(state, duration) {
            Some("State timeout - consider resetting")
        } else {
            None
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", ProcessState::Idle), "Idle");
        assert_eq!(
            format!("{}", ProcessState::Generating { chars_generated: 100 }),
            "Generating(100 chars)"
        );
        assert_eq!(
            format!("{}", ProcessState::ExecutingTools {
                total_tools: 3,
                completed_tools: 1
            }),
            "ExecutingTools(1/3)"
        );
    }

    #[test]
    fn test_state_machine_init() {
        let sm = StateMachine::new();
        assert_eq!(sm.current(), &ProcessState::Idle);
    }

    #[test]
    fn test_valid_transition_idle_to_processing() {
        let mut sm = StateMachine::new();
        assert!(sm.transition(StateTransition::StartProcessing).is_ok());
        assert_eq!(sm.current(), &ProcessState::Processing);
    }

    #[test]
    fn test_valid_transition_processing_to_generating() {
        let mut sm = StateMachine::new();
        sm.transition(StateTransition::StartProcessing).unwrap();
        assert!(sm.transition(StateTransition::StartGenerating).is_ok());
        assert_eq!(sm.current(), &ProcessState::Generating { chars_generated: 0 });
    }

    #[test]
    fn test_valid_transition_generating_to_complete() {
        let mut sm = StateMachine::new();
        sm.transition(StateTransition::StartProcessing).unwrap();
        sm.transition(StateTransition::StartGenerating).unwrap();
        assert!(sm.transition(StateTransition::Complete).is_ok());
        assert_eq!(sm.current(), &ProcessState::Idle);
    }

    #[test]
    fn test_invalid_transition_idle_to_complete() {
        let mut sm = StateMachine::new();
        let result = sm.transition(StateTransition::Complete);
        assert!(result.is_err());
    }

    #[test]
    fn test_generating_progress_update() {
        let mut sm = StateMachine::new();
        sm.transition(StateTransition::StartProcessing).unwrap();
        sm.transition(StateTransition::StartGenerating).unwrap();
        sm.transition(StateTransition::GeneratingProgress { chars: 50 })
            .unwrap();
        assert_eq!(
            sm.current(),
            &ProcessState::Generating { chars_generated: 50 }
        );
    }

    #[test]
    fn test_tool_execution_progress() {
        let mut sm = StateMachine::new();
        sm.transition(StateTransition::StartProcessing).unwrap();
        sm.transition(StateTransition::StartToolExecution { total: 3 })
            .unwrap();

        assert_eq!(
            sm.current(),
            &ProcessState::ExecutingTools {
                total_tools: 3,
                completed_tools: 0
            }
        );

        sm.transition(StateTransition::ToolCompleted).unwrap();
        assert_eq!(
            sm.current(),
            &ProcessState::ExecutingTools {
                total_tools: 3,
                completed_tools: 1
            }
        );

        sm.transition(StateTransition::ToolCompleted).unwrap();
        sm.transition(StateTransition::ToolCompleted).unwrap();
        // 第三次ToolCompleted后应该回到Processing
        assert_eq!(sm.current(), &ProcessState::Processing);
    }

    #[test]
    fn test_error_transition() {
        let mut sm = StateMachine::new();
        sm.transition(StateTransition::StartProcessing).unwrap();
        sm.transition(StateTransition::Error("test error".to_string()))
            .unwrap();
        assert!(sm.current().is_error());
    }

    #[test]
    fn test_state_is_active() {
        assert!(!ProcessState::Idle.is_active());
        assert!(ProcessState::Processing.is_active());
        assert!(ProcessState::Generating { chars_generated: 0 }.is_active());
        assert!(ProcessState::ExecutingTools {
            total_tools: 1,
            completed_tools: 0
        }
        .is_active());
        assert!(!ProcessState::Error {
            message: "test".to_string()
        }
        .is_active());
    }

    #[test]
    fn test_state_is_terminal() {
        assert!(!ProcessState::Idle.is_terminal());
        assert!(!ProcessState::Processing.is_terminal());
        assert!(ProcessState::Error {
            message: "test".to_string()
        }
        .is_terminal());
        assert!(ProcessState::Closed.is_terminal());
    }

    #[test]
    fn test_state_monitor_timeout() {
        let monitor = StateMonitor::default_config();

        let state = ProcessState::Generating { chars_generated: 100 };
        let duration = Duration::from_secs(400); // > 300 seconds timeout

        assert!(monitor.is_timeout(&state, duration));

        let state = ProcessState::Generating { chars_generated: 100 };
        let duration = Duration::from_secs(100); // < 300 seconds timeout

        assert!(!monitor.is_timeout(&state, duration));
    }

    #[test]
    fn test_state_machine_history() {
        let mut sm = StateMachine::new();
        sm.transition(StateTransition::StartProcessing).unwrap();
        sm.transition(StateTransition::StartGenerating).unwrap();

        assert_eq!(sm.history().len(), 2);
    }

    #[test]
    fn test_state_machine_reset() {
        let mut sm = StateMachine::new();
        sm.transition(StateTransition::StartProcessing).unwrap();
        sm.transition(StateTransition::Error("test".to_string()))
            .unwrap();

        sm.reset();
        assert_eq!(sm.current(), &ProcessState::Idle);
        assert_eq!(sm.history().len(), 0);
    }

    #[test]
    fn test_close_sequence() {
        let mut sm = StateMachine::new();
        sm.transition(StateTransition::StartProcessing).unwrap();
        sm.transition(StateTransition::StartClosing).unwrap();
        sm.transition(StateTransition::Closed).unwrap();

        assert_eq!(sm.current(), &ProcessState::Closed);
    }
}
