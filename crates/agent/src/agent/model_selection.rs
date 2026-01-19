//! 本地模型优先选择模块
//!
//! 功能：
//! 1. 根据任务复杂度选择合适的模型
//! 2. 优先使用本地模型（Ollama）
//! 3. 本地模型不可用时降级到云模型
//! 4. 考虑成本和性能因素

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 任务复杂度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskComplexity {
    /// 简单 - 单轮问答、简单控制
    Simple = 0,
    /// 中等 - 多轮对话、需要推理
    Medium = 1,
    /// 复杂 - 需要深度理解、多步骤
    Complex = 2,
}

/// 模型类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelType {
    /// 本地模型（Ollama）
    Local,
    /// 云端模型（OpenAI, Anthropic等）
    Cloud,
}

/// 模型能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// 支持的最大上下文长度
    pub max_context: usize,
    /// 是否支持工具调用
    pub supports_tools: bool,
    /// 是否支持视觉
    pub supports_vision: bool,
    /// 响应速度（相对值，越小越快）
    pub speed_score: u8,
    /// 推理质量（相对值，越大越好）
    pub quality_score: u8,
    /// 成本（相对值，越小越便宜）
    pub cost_score: u8,
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// 模型名称
    pub name: String,
    /// 模型类型
    pub model_type: ModelType,
    /// 模型能力
    pub capabilities: ModelCapabilities,
    /// 是否可用
    pub available: bool,
    /// 最后检查时间
    pub last_checked: Option<i64>,
}

/// 模型选择建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    /// 推荐的模型
    pub recommended_model: String,
    /// 推荐理由
    pub reason: String,
    /// 备选模型
    pub fallback_models: Vec<String>,
    /// 是否需要降级
    pub requires_fallback: bool,
}

/// 模型选择配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionConfig {
    /// 是否优先使用本地模型
    pub prefer_local: bool,
    /// 简单任务是否强制使用本地模型
    pub force_local_for_simple: bool,
    /// 本地模型不可用时是否降级到云模型
    pub allow_cloud_fallback: bool,
    /// 最大重试次数
    pub max_retries: usize,
}

impl Default for ModelSelectionConfig {
    fn default() -> Self {
        Self {
            prefer_local: true,
            force_local_for_simple: true,
            allow_cloud_fallback: true,
            max_retries: 2,
        }
    }
}

/// 模型状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    /// 模型名称
    pub model_name: String,
    /// 是否健康
    pub healthy: bool,
    /// 响应时间（毫秒）
    pub response_time_ms: Option<u64>,
    /// 错误消息（如果有）
    pub error: Option<String>,
}

/// 模型选择管理器
pub struct ModelSelectionManager {
    /// 配置
    config: ModelSelectionConfig,
    /// 已注册的模型
    models: Arc<RwLock<HashMap<String, ModelInfo>>>,
    /// 模型状态
    model_status: Arc<RwLock<HashMap<String, ModelStatus>>>,
    /// 使用统计
    usage_stats: Arc<RwLock<HashMap<String, usize>>>,
}

impl ModelSelectionManager {
    pub fn new() -> Self {
        Self {
            config: ModelSelectionConfig::default(),
            models: Arc::new(RwLock::new(HashMap::new())),
            model_status: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_config(config: ModelSelectionConfig) -> Self {
        Self {
            config,
            models: Arc::new(RwLock::new(HashMap::new())),
            model_status: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册模型
    pub async fn register_model(&self, model: ModelInfo) {
        let mut models = self.models.write().await;
        models.insert(model.name.clone(), model);
    }

    /// 批量注册模型
    pub async fn register_models(&self, models: Vec<ModelInfo>) {
        let mut model_map = self.models.write().await;
        for model in models {
            model_map.insert(model.name.clone(), model);
        }
    }

    /// 分析任务复杂度
    pub fn analyze_task_complexity(&self, input: &str, has_tools: bool) -> TaskComplexity {
        let input_lower = input.to_lowercase();

        // 复杂任务指标
        let complex_indicators = [
            "分析",
            "总结",
            "比较",
            "为什么",
            "如何",
            "步骤",
            "计划",
            "设计",
            "设置",
            "调节",
            "analyze",
            "summarize",
            "compare",
            "why",
            "how",
            "steps",
            "plan",
            "design",
            "set",
            "adjust",
        ];

        // 简单任务指标
        let simple_indicators = [
            "打开",
            "关闭",
            "开启",
            "查询",
            "状态",
            "温度",
            "湿度",
            "open",
            "close",
            "turn on",
            "turn off",
            "status",
            "temperature",
        ];

        let complex_count = complex_indicators.iter().filter(|i| input_lower.contains(*i)).count();
        let simple_count = simple_indicators.iter().filter(|i| input_lower.contains(*i)).count();

        // 需要工具调用通常增加复杂度
        let tool_complexity = if has_tools { 1 } else { 0 };

        // 输入长度也影响复杂度
        let length_complexity = if input.len() > 100 { 1 } else { 0 };

        // 计算复杂度分数，避免下溢
        // 当有工具调用时，简单指标的影响降低
        let simple_weight = if has_tools { 0 } else { simple_count };
        let complexity_score = (complex_count + tool_complexity + length_complexity)
            .saturating_sub(simple_weight);

        if complexity_score >= 2 {
            TaskComplexity::Complex
        } else if complexity_score >= 1 {
            TaskComplexity::Medium
        } else {
            TaskComplexity::Simple
        }
    }

    /// 选择模型
    pub async fn select_model(
        &self,
        input: &str,
        has_tools: bool,
        needs_vision: bool,
    ) -> ModelSelection {
        let complexity = self.analyze_task_complexity(input, has_tools);
        self.select_model_by_complexity(complexity, has_tools, needs_vision).await
    }

    /// 根据复杂度选择模型
    pub async fn select_model_by_complexity(
        &self,
        complexity: TaskComplexity,
        needs_tools: bool,
        needs_vision: bool,
    ) -> ModelSelection {
        let models = self.models.read().await;

        // 获取可用模型
        let mut available_models: Vec<_> = models
            .values()
            .filter(|m| m.available)
            .filter(|m| !needs_vision || m.capabilities.supports_vision)
            .filter(|m| !needs_tools || m.capabilities.supports_tools)
            .collect();

        // 如果没有可用模型，返回空选择
        if available_models.is_empty() {
            return ModelSelection {
                recommended_model: String::new(),
                reason: "没有可用的模型".to_string(),
                fallback_models: vec![],
                requires_fallback: true,
            };
        }

        // 排序策略：本地优先，然后根据复杂度选择
        if self.config.prefer_local {
            available_models.sort_by(|a, b| {
                // 本地模型优先
                match (&a.model_type, &b.model_type) {
                    (ModelType::Local, ModelType::Cloud) => return std::cmp::Ordering::Less,
                    (ModelType::Cloud, ModelType::Local) => return std::cmp::Ordering::Greater,
                    _ => {}
                }

                // 对于简单任务，速度优先（速度分数高的在前）
                if complexity == TaskComplexity::Simple {
                    b.capabilities.speed_score.cmp(&a.capabilities.speed_score)
                } else {
                    // 对于复杂任务，质量优先
                    b.capabilities.quality_score.cmp(&a.capabilities.quality_score)
                }
            });
        }

        // 简单任务强制使用本地模型
        if self.config.force_local_for_simple && complexity == TaskComplexity::Simple {
            let local_models: Vec<_> = available_models
                .iter()
                .filter(|m| m.model_type == ModelType::Local)
                .collect();

            if !local_models.is_empty() {
                let recommended = local_models[0];
                return ModelSelection {
                    recommended_model: recommended.name.clone(),
                    reason: "简单任务优先使用快速本地模型".to_string(),
                    fallback_models: local_models.iter().skip(1).map(|m| m.name.clone()).collect(),
                    requires_fallback: false,
                };
            }
        }

        // 选择第一个（排序后最合适的）
        let recommended = &available_models[0];
        let fallback_models: Vec<String> = available_models
            .iter()
            .skip(1)
            .take(2)
            .map(|m| m.name.clone())
            .collect();

        let requires_fallback = recommended.model_type == ModelType::Cloud
            && self.config.prefer_local
            && self.config.allow_cloud_fallback;

        let reason = match complexity {
            TaskComplexity::Simple => "简单任务，使用快速响应模型".to_string(),
            TaskComplexity::Medium => "中等复杂度任务，使用平衡模型".to_string(),
            TaskComplexity::Complex => "复杂任务，使用高质量模型".to_string(),
        };

        ModelSelection {
            recommended_model: recommended.name.clone(),
            reason,
            fallback_models,
            requires_fallback,
        }
    }

    /// 更新模型状态
    pub async fn update_model_status(&self, status: ModelStatus) {
        let mut model_status = self.model_status.write().await;
        model_status.insert(status.model_name.clone(), status);
    }

    /// 获取模型状态
    pub async fn get_model_status(&self, model_name: &str) -> Option<ModelStatus> {
        let model_status = self.model_status.read().await;
        model_status.get(model_name).cloned()
    }

    /// 检查模型健康状态
    pub async fn is_model_healthy(&self, model_name: &str) -> bool {
        if let Some(status) = self.get_model_status(model_name).await {
            status.healthy
        } else {
            // 没有状态记录时，假设可用
            true
        }
    }

    /// 记录模型使用
    pub async fn record_usage(&self, model_name: &str) {
        let mut stats = self.usage_stats.write().await;
        *stats.entry(model_name.to_string()).or_insert(0) += 1;
    }

    /// 获取使用统计
    pub async fn get_usage_stats(&self) -> HashMap<String, usize> {
        self.usage_stats.read().await.clone()
    }

    /// 获取所有模型
    pub async fn get_all_models(&self) -> Vec<ModelInfo> {
        self.models.read().await.values().cloned().collect()
    }

    /// 获取本地模型列表
    pub async fn get_local_models(&self) -> Vec<ModelInfo> {
        self.models
            .read()
            .await
            .values()
            .filter(|m| m.model_type == ModelType::Local)
            .cloned()
            .collect()
    }

    /// 获取云端模型列表
    pub async fn get_cloud_models(&self) -> Vec<ModelInfo> {
        self.models
            .read()
            .await
            .values()
            .filter(|m| m.model_type == ModelType::Cloud)
            .cloned()
            .collect()
    }

    /// 标记模型为不可用
    pub async fn mark_unavailable(&self, model_name: &str) {
        let mut models = self.models.write().await;
        if let Some(model) = models.get_mut(model_name) {
            model.available = false;
        }
    }

    /// 标记模型为可用
    pub async fn mark_available(&self, model_name: &str) {
        let mut models = self.models.write().await;
        if let Some(model) = models.get_mut(model_name) {
            model.available = true;
        }
    }

    /// 创建默认模型配置
    pub async fn setup_default_models(&self) {
        let default_models = vec![
            // Ollama 本地模型
            ModelInfo {
                name: "qwen3-vl:2b".to_string(),
                model_type: ModelType::Local,
                capabilities: ModelCapabilities {
                    max_context: 32768,
                    supports_tools: true,
                    supports_vision: true,
                    speed_score: 9,  // 快速
                    quality_score: 6, // 中等质量
                    cost_score: 10,   // 免费
                },
                available: true,
                last_checked: None,
            },
            ModelInfo {
                name: "llama3:8b".to_string(),
                model_type: ModelType::Local,
                capabilities: ModelCapabilities {
                    max_context: 8192,
                    supports_tools: true,
                    supports_vision: false,
                    speed_score: 7,
                    quality_score: 7,
                    cost_score: 10,
                },
                available: true,
                last_checked: None,
            },
            // 云端模型（备用）
            ModelInfo {
                name: "gpt-4o-mini".to_string(),
                model_type: ModelType::Cloud,
                capabilities: ModelCapabilities {
                    max_context: 128000,
                    supports_tools: true,
                    supports_vision: true,
                    speed_score: 8,
                    quality_score: 8,
                    cost_score: 5, // 付费
                },
                available: false, // 默认不可用，需要 API key
                last_checked: None,
            },
        ];

        self.register_models(default_models).await;
    }
}

impl Default for ModelSelectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_manager() -> ModelSelectionManager {
        ModelSelectionManager::new()
    }

    async fn create_manager_with_models() -> ModelSelectionManager {
        let manager = create_manager();
        let models = vec![
            ModelInfo {
                name: "qwen3-vl:2b".to_string(),
                model_type: ModelType::Local,
                capabilities: ModelCapabilities {
                    max_context: 32768,
                    supports_tools: true,
                    supports_vision: true,
                    speed_score: 9,
                    quality_score: 6,
                    cost_score: 10,
                },
                available: true,
                last_checked: None,
            },
            ModelInfo {
                name: "llama3:8b".to_string(),
                model_type: ModelType::Local,
                capabilities: ModelCapabilities {
                    max_context: 8192,
                    supports_tools: true,
                    supports_vision: false,
                    speed_score: 7,
                    quality_score: 7,
                    cost_score: 10,
                },
                available: true,
                last_checked: None,
            },
            ModelInfo {
                name: "gpt-4o-mini".to_string(),
                model_type: ModelType::Cloud,
                capabilities: ModelCapabilities {
                    max_context: 128000,
                    supports_tools: true,
                    supports_vision: true,
                    speed_score: 8,
                    quality_score: 8,
                    cost_score: 5,
                },
                available: true,
                last_checked: None,
            },
        ];
        manager.register_models(models).await;
        manager
    }

    #[test]
    fn test_task_complexity_simple() {
        let manager = create_manager();

        assert_eq!(
            manager.analyze_task_complexity("打开客厅灯", false),
            TaskComplexity::Simple
        );
        assert_eq!(
            manager.analyze_task_complexity("temperature", false),
            TaskComplexity::Simple
        );
    }

    #[test]
    fn test_task_complexity_medium() {
        let manager = create_manager();

        // 需要工具调用，但只有简单操作
        assert_eq!(
            manager.analyze_task_complexity("查询温度", true),
            TaskComplexity::Medium
        );
    }

    #[test]
    fn test_task_complexity_complex() {
        let manager = create_manager();

        assert_eq!(
            manager.analyze_task_complexity("分析最近一周的温度数据并总结趋势", true),
            TaskComplexity::Complex
        );
    }

    #[tokio::test]
    async fn test_select_local_model_for_simple_task() {
        let manager = create_manager_with_models().await;

        let selection = manager
            .select_model("打开客厅灯", false, false)
            .await;

        // 简单任务应该选择本地模型
        assert_eq!(selection.recommended_model, "qwen3-vl:2b");
        assert!(selection.reason.contains("快速") || selection.reason.contains("简单"));
    }

    #[tokio::test]
    async fn test_local_model_priority() {
        let manager = create_manager_with_models().await;

        // 本地模型应该优先于云端模型
        let selection = manager
            .select_model("简单查询", false, false)
            .await;

        assert_ne!(selection.recommended_model, "gpt-4o-mini");
    }

    #[tokio::test]
    async fn test_fallback_models() {
        let manager = create_manager_with_models().await;

        let selection = manager
            .select_model("简单查询", false, false)
            .await;

        // 应该有备选模型
        assert!(!selection.fallback_models.is_empty());
    }

    #[tokio::test]
    async fn test_vision_requirement() {
        let manager = create_manager_with_models().await;

        // 需要视觉能力时，只有支持视觉的模型会被选择
        let selection = manager
            .select_model("描述这张图片", false, true)
            .await;

        // 应该选择支持视觉的模型
        assert!(selection.recommended_model == "qwen3-vl:2b"
            || selection.recommended_model == "gpt-4o-mini");
    }

    #[tokio::test]
    async fn test_model_usage_tracking() {
        let manager = create_manager_with_models().await;

        manager.record_usage("qwen3-vl:2b").await;
        manager.record_usage("qwen3-vl:2b").await;
        manager.record_usage("llama3:8b").await;

        let stats = manager.get_usage_stats().await;
        assert_eq!(stats.get("qwen3-vl:2b"), Some(&2));
        assert_eq!(stats.get("llama3:8b"), Some(&1));
    }

    #[tokio::test]
    async fn test_mark_unavailable() {
        let manager = create_manager_with_models().await;

        manager.mark_unavailable("qwen3-vl:2b").await;

        let models = manager.get_all_models().await;
        let qwen = models.iter().find(|m| m.name == "qwen3-vl:2b");
        assert!(qwen.is_some());
        assert!(!qwen.unwrap().available);
    }

    #[tokio::test]
    async fn test_get_local_models() {
        let manager = create_manager_with_models().await;

        let local_models = manager.get_local_models().await;
        assert_eq!(local_models.len(), 2);
        assert!(local_models.iter().all(|m| m.model_type == ModelType::Local));
    }

    #[tokio::test]
    async fn test_get_cloud_models() {
        let manager = create_manager_with_models().await;

        let cloud_models = manager.get_cloud_models().await;
        assert_eq!(cloud_models.len(), 1);
        assert_eq!(cloud_models[0].model_type, ModelType::Cloud);
    }
}
