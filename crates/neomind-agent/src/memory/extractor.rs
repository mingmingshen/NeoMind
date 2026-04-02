//! Memory extraction from Chat and Agent sources
//!
//! This module provides LLM-based extraction of memory candidates
//! from chat conversations and agent execution logs.

use neomind_storage::MemoryCategory;
use serde::{Deserialize, Serialize};

/// A candidate memory entry extracted by LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidate {
    /// Memory content
    pub content: String,
    /// Target category
    pub category: String,
    /// Importance score (0-100)
    pub importance: u8,
}

/// Result of extraction operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractResult {
    /// Extracted memory candidates
    pub memories: Vec<MemoryCandidate>,
}

/// Chat conversation extractor
pub struct ChatExtractor;

impl ChatExtractor {
    /// Build LLM prompt for extracting from chat
    pub fn build_prompt(messages: &str) -> String {
        format!(
            r#"分析以下对话，提取有价值的记忆。

## 对话内容
{}

## 输出格式 (只输出 JSON)
{{"memories":[{{"content":"内容","category":"user_profile|domain_knowledge|task_patterns","importance":50}}]}}

## 规则
- 跳过闲聊和问候语
- 只提取长期有价值的信息
- importance 范围 0-100，越高越重要
"#,
            messages
        )
    }

    /// Parse LLM response into ExtractResult
    pub fn parse_response(response: &str) -> Result<ExtractResult, String> {
        // Find JSON object
        let start = response.find('{').ok_or("No JSON object found")?;
        let end = response.rfind('}').ok_or("No closing brace found")?;
        let json = &response[start..=end];
        serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))
    }
}

/// Agent execution extractor
pub struct AgentExtractor;

impl AgentExtractor {
    /// Build LLM prompt for extracting from agent execution
    pub fn build_prompt(
        agent_name: &str,
        user_prompt: Option<&str>,
        reasoning_steps: &str,
        conclusion: &str,
    ) -> String {
        format!(
            r#"分析 Agent 执行记录，提取有价值的记忆。

## Agent 名称
{}

## 用户预期（提示词）
{}

## 执行过程
{}

## 执行结果
{}

## 输出格式 (只输出 JSON)
{{"memories":[{{"content":"内容","category":"user_profile|domain_knowledge|task_patterns|system_evolution","importance":50}}]}}

## 规则
- 提取用户的偏好和习惯 -> user_profile
- 提取发现的设备状态、环境规律 -> domain_knowledge
- 提取成功的任务模式、失败原因 -> task_patterns
- 提取 Agent 学到的经验 -> system_evolution
"#,
            agent_name,
            user_prompt.unwrap_or("(无)"),
            reasoning_steps,
            conclusion
        )
    }

    /// Parse LLM response into ExtractResult
    pub fn parse_response(response: &str) -> Result<ExtractResult, String> {
        ChatExtractor::parse_response(response)
    }
}

/// Parse category string into enum
pub fn parse_category(s: &str) -> MemoryCategory {
    match s.to_lowercase().as_str() {
        "user_profile" => MemoryCategory::UserProfile,
        "domain_knowledge" => MemoryCategory::DomainKnowledge,
        "task_patterns" => MemoryCategory::TaskPatterns,
        "system_evolution" => MemoryCategory::SystemEvolution,
        _ => MemoryCategory::UserProfile, // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_extractor_prompt() {
        let messages = "User: 你好\nAssistant: 你好！有什么可以帮助你的？";
        let prompt = ChatExtractor::build_prompt(messages);
        assert!(prompt.contains("对话内容"));
        assert!(prompt.contains(messages));
    }

    #[test]
    fn test_agent_extractor_prompt() {
        let prompt = AgentExtractor::build_prompt(
            "温度监控",
            Some("监控室内温度"),
            "1. 获取温度读数\n2. 检查阈值",
            "温度正常",
        );
        assert!(prompt.contains("温度监控"));
        assert!(prompt.contains("监控室内温度"));
        assert!(prompt.contains("温度正常"));
    }

    #[test]
    fn test_parse_response_valid() {
        let json = r#"{"memories":[{"content":"用户偏好中文","category":"user_profile","importance":80}]}"#;
        let result = ChatExtractor::parse_response(json).unwrap();
        assert_eq!(result.memories.len(), 1);
        assert_eq!(result.memories[0].content, "用户偏好中文");
        assert_eq!(result.memories[0].importance, 80);
    }

    #[test]
    fn test_parse_response_with_noise() {
        let response = r#"
Here is the analysis:
{"memories":[{"content":"设备温度25度","category":"domain_knowledge","importance":60}]}
That's all.
"#;
        let result = ChatExtractor::parse_response(response).unwrap();
        assert_eq!(result.memories.len(), 1);
    }

    #[test]
    fn test_parse_response_empty() {
        let json = r#"{"memories":[]}"#;
        let result = ChatExtractor::parse_response(json).unwrap();
        assert!(result.memories.is_empty());
    }

    #[test]
    fn test_parse_response_invalid() {
        let invalid = "not json";
        assert!(ChatExtractor::parse_response(invalid).is_err());
    }

    #[test]
    fn test_parse_category() {
        assert_eq!(
            parse_category("user_profile"),
            MemoryCategory::UserProfile
        );
        assert_eq!(
            parse_category("DOMAIN_KNOWLEDGE"),
            MemoryCategory::DomainKnowledge
        );
        assert_eq!(
            parse_category("task_patterns"),
            MemoryCategory::TaskPatterns
        );
        assert_eq!(
            parse_category("system_evolution"),
            MemoryCategory::SystemEvolution
        );
        assert_eq!(parse_category("unknown"), MemoryCategory::UserProfile);
    }
}
