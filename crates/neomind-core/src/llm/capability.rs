//! Model capability detection module.
//!
//! This module provides functionality to detect and query model capabilities,
//! including support for streaming, function calling, vision, audio, reasoning, etc.

use crate::llm::models::{get_model_info, ModelCapabilities};

/// Result of capability detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDetectionResult {
    /// Model name
    pub model: String,
    /// Provider type
    pub provider: crate::llm::models::ProviderType,
    /// Detected capabilities
    pub capabilities: ModelCapabilities,
    /// Whether detection was from built-in registry
    pub from_registry: bool,
}

/// Capability detector for LLM models.
pub struct CapabilityDetector {
    /// Cache of detected capabilities
    cache: std::collections::HashMap<String, CapabilityDetectionResult>,
}

impl CapabilityDetector {
    /// Create a new capability detector.
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }

    /// Detect capabilities for a model.
    ///
    /// First checks the built-in model registry, then falls back to
    /// heuristic detection based on model name patterns.
    pub fn detect(&mut self, model: &str) -> Option<CapabilityDetectionResult> {
        // Check cache first
        if let Some(cached) = self.cache.get(model) {
            return Some(cached.clone());
        }

        // Try built-in registry first
        if let Some(info) = get_model_info(model) {
            let result = CapabilityDetectionResult {
                model: model.to_string(),
                provider: info.provider,
                capabilities: info.capabilities.clone(),
                from_registry: true,
            };
            self.cache.insert(model.to_string(), result.clone());
            return Some(result);
        }

        // Fallback to heuristic detection
        let result = self.heuristic_detect(model)?;
        self.cache.insert(model.to_string(), result.clone());
        Some(result)
    }

    /// Heuristic capability detection based on model name patterns.
    ///
    /// This is used when the model is not in the built-in registry.
    fn heuristic_detect(&self, model: &str) -> Option<CapabilityDetectionResult> {
        let model_lower = model.to_lowercase();

        // Detect provider from model name
        let provider = self.detect_provider(&model_lower);

        // Detect capabilities from model name patterns
        let capabilities = ModelCapabilities {
            streaming: self.detect_streaming(&model_lower),
            function_calling: self.detect_function_calling(&model_lower),
            vision: self.detect_vision(&model_lower),
            audio: self.detect_audio(&model_lower),
            video: self.detect_video(&model_lower),
            reasoning: self.detect_reasoning(&model_lower),
            max_context: Some(self.estimate_max_context(&model_lower)),
            json_mode: self.detect_json_mode(&model_lower),
        };

        Some(CapabilityDetectionResult {
            model: model.to_string(),
            provider,
            capabilities,
            from_registry: false,
        })
    }

    /// Detect provider from model name.
    fn detect_provider(&self, model: &str) -> crate::llm::models::ProviderType {
        use crate::llm::models::ProviderType;

        if model.contains("gpt") || model.contains("o1") || model.contains("o3") {
            ProviderType::OpenAI
        } else if model.contains("claude") {
            ProviderType::Anthropic
        } else if model.contains("gemini") {
            ProviderType::Google
        } else if model.contains("grok") {
            ProviderType::XAi
        } else if model.contains("qwen") {
            ProviderType::Qwen
        } else if model.contains("deepseek") {
            ProviderType::DeepSeek
        } else if model.contains("glm") || model.contains("zhipu") {
            ProviderType::GLM
        } else if model.contains("minimax") || model.contains("m2-") || model.contains("abab") {
            ProviderType::MiniMax
        } else if model.contains("llama") || model.contains("mistral") {
            ProviderType::Ollama
        } else {
            ProviderType::Custom
        }
    }

    /// Detect streaming capability (most modern models support it).
    fn detect_streaming(&self, model: &str) -> bool {
        // Most modern LLMs support streaming
        // Older models or specific variants might not
        !model.contains("-legacy") && !model.contains("-v1-")
    }

    /// Detect function calling capability.
    fn detect_function_calling(&self, model: &str) -> bool {
        // GPT-4 and later, Claude 3+, Gemini, etc. support function calling
        model.contains("gpt-4")
            || model.contains("gpt-4o")
            || model.contains("claude-3")
            || model.contains("gemini-1.5")
            || model.contains("gemini-2.")
            || model.contains("qwen-max")
            || model.contains("qwen-plus")
            || model.contains("deepseek")
            || model.contains("glm-4")
            || model.contains("glm-5")
            || model.contains("minimax")
            || model.contains("grok")
            || model.contains("llama-3.1")
            || model.contains("llama-3.2")
            || model.contains("llama-3.3")
    }

    /// Detect vision/multimodal capability.
    pub fn detect_vision(&self, model: &str) -> bool {
        // 通用视觉关键词
        if model.contains("vision") || model.contains("-vl") || model.contains("_vl") {
            return true;
        }

        // OpenAI
        // GPT-4o 系列 (包括 gpt-4o, gpt-4o-mini)
        // GPT-4-turbo (支持 Vision)
        // GPT-4.5 (支持全模态)
        // 注意: GPT-4 (不带 turbo/vision)、GPT-3.5、o1、o3 不支持视觉
        if model.contains("-4o")
            || (model.contains("gpt-4-turbo") && !model.contains("gpt-4-turbo-preview"))
            || model.contains("gpt-4.5")
        {
            return true;
        }

        // Anthropic - Claude 3 系列和特定的 Claude 4 模型支持视觉
        // Claude 3 系列: claude-3-opus, claude-3-sonnet, claude-3-haiku, // Claude 3.5 系列: claude-3.5-sonnet, claude-3.5-haiku
        // Claude 4 系列: claude-opus-4, claude-sonnet-4, claude-haiku-4 (注意: 必须是精确匹配，不能包含 -5)
        // 例如: claude-haiku-4-5-20251001 不支持视觉
        let has_vision = model.contains("claude-3")
            || (model.contains("claude-opus-4") && !model.contains("claude-opus-4-5"))
            || (model.contains("claude-sonnet-4") && !model.contains("claude-sonnet-4-5"))
            || (model.contains("claude-haiku-4") && !model.contains("claude-haiku-4-5"));

        if has_vision {
            return true;
        }

        // Google - 所有 Gemini 原生支持多模态
        if model.contains("gemini") {
            return true;
        }

        // Qwen (阿里通义千问)
        // qwen-vl 系列: qwen-vl-max, qwen-vl-plus, qwen3-vl-plus, qwen3-vl-flash
        // qwen2.5-vl, qwen2-vl 系列
        // qvq 系列 (视觉推理): qvq-max, qvq-plus
        // qwen-omni 系列 (全模态): qwen-omni-turbo, qwen-omni-max, qwen3-omni-flash
        // qwen3.5 系列: 原生支持文本、图像、视频 (包括 qwen3.5:4b 等本地模型)
        // qwen3.5-plus (云 API)
        // 注意: qwen-turbo, qwen-long, qwen-coder 不支持视觉
        if model.contains("qwen-vl")
            || model.contains("qwen2.5-vl")
            || model.contains("qwen2-vl")
            || model.contains("qwen3-vl")
            || model.contains("qwen-omni")
            || model.contains("qwen3-omni")
            || model.contains("qvq")
            || model.contains("qwen3.5")
        {
            return true;
        }

        // DeepSeek
        // deepseek-vl 系列: deepseek-vl-7b-chat, deepseek-vl-1.3b-chat
        // 注意: deepseek-chat, deepseek-r1, deepseek-coder 不支持视觉
        if model.contains("deepseek-vl") {
            return true;
        }

        // GLM (智谱)
        // glm-4v 系列: glm-4v, glm-4v-plus, glm-4v-flash
        // 注意: glm-4-plus, glm-4-flash, glm-5, glm-z1 不支持视觉
        if model.contains("glm-4v") {
            return true;
        }

        // MiniMax
        // minimax-vl-01 (视觉语言模型)
        // m2-her (支持视觉)
        // 注意: m2-1, m2-2, minimax-text, abab 不支持视觉
        if model.contains("minimax-vl") || model.contains("m2-her") {
            return true;
        }

        // Grok (xAI)
        // grok-2-vision, grok-2-vision-latest, grok-vision-beta
        // 注意: grok-beta, grok-3, grok-4 不支持视觉
        if model.contains("grok") && model.contains("vision") {
            return true;
        }

        // Ollama/Local Models - Basic detection for common patterns
        // NOTE: For accurate detection, use Ollama's /api/show endpoint
        // which returns a "capabilities" array including "vision"
        if model.contains("-vl")
            || model.contains("_vl")
            || model.contains("llava")
            || model.contains("moondream")
            || model.contains("vision")
        {
            return true;
        }

        false
    }

    /// Detect audio capability.
    fn detect_audio(&self, model: &str) -> bool {
        model.contains("audio")
            || model.contains("tts")
            || model.contains("asr")
            || model.contains("whisper")
            || model.contains("gpt-4o")
            || model.contains("qwen-audio")
            || model.contains("qwen-tts")
            || model.contains("qwen-omni")
            || model.contains("minimax-speech")
    }

    /// Detect video capability.
    fn detect_video(&self, model: &str) -> bool {
        model.contains("video") || model.contains("qwen-video") || model.contains("qwen-omni")
    }

    /// Detect reasoning capability (o1, o3, deepseek-r1, etc).
    fn detect_reasoning(&self, model: &str) -> bool {
        model.contains("o1")
            || model.contains("o3")
            || model.contains("r1")
            || model.contains("reasoning")
            || model.contains("qwq") // Qwen reasoning models
            || model.contains("glm-z1")
    }

    /// Estimate max context length based on model name.
    fn estimate_max_context(&self, model: &str) -> usize {
        if model.contains("gpt-4") || model.contains("o1") || model.contains("o3") {
            if model.contains("turbo") {
                128000
            } else if model.contains("o1") || model.contains("o3") {
                200000
            } else {
                128000
            }
        } else if model.contains("claude") {
            if model.contains("claude-3") {
                200000
            } else {
                100000
            }
        } else if model.contains("gemini-2.") {
            1000000
        } else if model.contains("gemini-1.5") {
            if model.contains("pro") {
                1000000
            } else {
                1000000
            }
        } else if model.contains("qwen") {
            if model.contains("qwen-long") {
                1000000
            } else if model.contains("qwen-max") || model.contains("qwen-plus") {
                128000
            } else if model.contains("qwen-vl") {
                32768
            } else {
                32768
            }
        } else if model.contains("deepseek") {
            if model.contains("deepseek-r1") {
                64000
            } else if model.contains("deepseek-v3") {
                128000
            } else {
                128000
            }
        } else if model.contains("glm") {
            if model.contains("glm-5") {
                1000000
            } else if model.contains("glm-4-plus") || model.contains("glm-4-air") {
                128000
            } else if model.contains("glm-4-flash") {
                128000
            } else {
                128000
            }
        } else if model.contains("minimax") {
            if model.contains("m2-1") || model.contains("m2-her") {
                512000
            } else {
                245760
            }
        } else {
            // Default conservative estimate
            8192
        }
    }

    /// Detect JSON mode capability.
    fn detect_json_mode(&self, model: &str) -> bool {
        model.contains("gpt-4")
            || model.contains("gpt-4o")
            || model.contains("claude-3")
            || model.contains("gemini")
            || model.contains("qwen-max")
            || model.contains("qwen-plus")
            || model.contains("deepseek")
            || model.contains("glm-4")
            || model.contains("glm-5")
            || model.contains("minimax")
            || model.contains("grok")
            || model.contains("llama-3.1")
            || model.contains("llama-3.2")
            || model.contains("llama-3.3")
    }

    /// Get all cached capabilities.
    pub fn get_all_cached(&self) -> Vec<CapabilityDetectionResult> {
        self.cache.values().cloned().collect()
    }

    /// Clear the capability cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for CapabilityDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a model supports a specific capability.
///
/// This is a convenience function that creates a detector
/// and checks a single capability.
pub fn model_supports(model: &str, capability: &str) -> bool {
    let mut detector = CapabilityDetector::new();

    match capability {
        "streaming" => detector
            .detect(model)
            .map(|r| r.capabilities.streaming)
            .unwrap_or(false),
        "function_calling" | "tools" => detector
            .detect(model)
            .map(|r| r.capabilities.function_calling)
            .unwrap_or(false),
        "vision" | "vl" | "multimodal" => detector
            .detect(model)
            .map(|r| r.capabilities.vision)
            .unwrap_or(false),
        "audio" => detector
            .detect(model)
            .map(|r| r.capabilities.audio)
            .unwrap_or(false),
        "video" => detector
            .detect(model)
            .map(|r| r.capabilities.video)
            .unwrap_or(false),
        "reasoning" => detector
            .detect(model)
            .map(|r| r.capabilities.reasoning)
            .unwrap_or(false),
        "json" | "json_mode" => detector
            .detect(model)
            .map(|r| r.capabilities.json_mode)
            .unwrap_or(false),
        _ => false,
    }
}

/// Get the max context length for a model.
pub fn get_max_context(model: &str) -> usize {
    let mut detector = CapabilityDetector::new();
    detector
        .detect(model)
        .and_then(|r| r.capabilities.max_context)
        .unwrap_or(8192)
}

/// Detect vision/multimodal capability from model name.
/// This is a standalone function that can be used without creating a CapabilityDetector.
pub fn detect_vision_capability(model: &str) -> bool {
    let detector = CapabilityDetector::new();
    detector.detect_vision(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_provider() {
        let detector = CapabilityDetector::new();

        assert_eq!(
            detector.detect_provider("gpt-4o"),
            crate::llm::models::ProviderType::OpenAI
        );
        assert_eq!(
            detector.detect_provider("claude-3-5-sonnet"),
            crate::llm::models::ProviderType::Anthropic
        );
        assert_eq!(
            detector.detect_provider("gemini-2.0-flash"),
            crate::llm::models::ProviderType::Google
        );
        assert_eq!(
            detector.detect_provider("qwen-max-latest"),
            crate::llm::models::ProviderType::Qwen
        );
        assert_eq!(
            detector.detect_provider("deepseek-v3"),
            crate::llm::models::ProviderType::DeepSeek
        );
        assert_eq!(
            detector.detect_provider("glm-4-plus"),
            crate::llm::models::ProviderType::GLM
        );
        assert_eq!(
            detector.detect_provider("minimax-text-01"),
            crate::llm::models::ProviderType::MiniMax
        );
    }

    #[test]
    fn test_detect_vision() {
        let detector = CapabilityDetector::new();

        // 支持视觉的模型
        assert!(detector.detect_vision("gpt-4o"));
        assert!(detector.detect_vision("gpt-4o-mini"));
        assert!(detector.detect_vision("gpt-4-turbo"));
        assert!(detector.detect_vision("qwen-vl-max"));
        assert!(detector.detect_vision("qwen2.5-vl-7b-instruct"));
        assert!(detector.detect_vision("qwen3-vl-plus"));
        assert!(detector.detect_vision("claude-3-5-sonnet"));
        assert!(detector.detect_vision("claude-opus-4"));
        assert!(detector.detect_vision("gemini-2.0-flash"));
        assert!(detector.detect_vision("minimax-vl-01"));
        assert!(detector.detect_vision("glm-4v-plus"));
        assert!(detector.detect_vision("grok-2-vision"));

        // 不支持视觉的模型
        assert!(!detector.detect_vision("gpt-3.5-turbo"));
        assert!(!detector.detect_vision("gpt-4")); // 不带 turbo/vision 的基础版
        assert!(!detector.detect_vision("o1-preview"));
        assert!(!detector.detect_vision("o3-mini"));
        assert!(!detector.detect_vision("qwen-turbo"));
        assert!(!detector.detect_vision("qwen-coder-plus"));
        assert!(!detector.detect_vision("deepseek-chat"));
        assert!(!detector.detect_vision("deepseek-r1"));
        assert!(!detector.detect_vision("glm-4-plus"));
        assert!(!detector.detect_vision("grok-3"));
    }

    #[test]
    fn test_detect_reasoning() {
        let detector = CapabilityDetector::new();

        assert!(detector.detect_reasoning("o1-preview"));
        assert!(detector.detect_reasoning("o3-mini"));
        assert!(detector.detect_reasoning("deepseek-r1"));
        assert!(detector.detect_reasoning("qwq-32b-preview"));
        assert!(detector.detect_reasoning("glm-z1"));
        assert!(!detector.detect_reasoning("gpt-4o"));
    }

    #[test]
    fn test_model_supports() {
        assert!(model_supports("gpt-4o", "streaming"));
        assert!(model_supports("gpt-4o", "vision"));
        assert!(model_supports("gpt-4o", "function_calling"));
        assert!(model_supports("gpt-4o", "json"));

        // gpt-4-turbo 支持视觉
        assert!(model_supports("gpt-4-turbo", "vision"));

        // 不支持视觉的模型
        assert!(!model_supports("gpt-3.5-turbo", "vision"));
        assert!(!model_supports("o1-preview", "vision"));
        assert!(!model_supports("qwen-turbo", "vision"));
    }

    #[test]
    fn test_get_max_context() {
        assert_eq!(get_max_context("gpt-4o"), 128000);
        assert_eq!(get_max_context("claude-3-5-sonnet"), 200000);
        assert_eq!(get_max_context("gemini-1.5-pro"), 1000000);
        assert_eq!(get_max_context("gemini-2.0-flash"), 1000000);
        assert_eq!(get_max_context("qwen-long"), 1000000);
        assert_eq!(get_max_context("deepseek-v3"), 128000);
        assert_eq!(get_max_context("glm-5"), 128000);
    }

    #[test]
    fn test_detector_with_builtin_models() {
        let mut detector = CapabilityDetector::new();

        // Test a built-in model
        let result = detector.detect("gpt-4o").unwrap();
        assert_eq!(result.model, "gpt-4o");
        assert!(result.from_registry);
        assert!(result.capabilities.streaming);
        assert!(result.capabilities.vision);

        // Test a non-built-in model (heuristic)
        let result = detector.detect("custom-model-7b").unwrap();
        assert_eq!(result.model, "custom-model-7b");
        assert!(!result.from_registry);
    }
}
