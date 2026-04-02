//! Memory extraction from Chat and Agent sources
//!
//! This module provides LLM-based extraction of memory candidates
//! from chat conversations and agent execution logs.

use neomind_storage::MemoryCategory;
use serde::{Deserialize, Serialize};

/// A candidate memory entry extracted by LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidate {
    /// Memory content (in English by default, adapt to user's language if needed)
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
            r#"Analyze the following conversation and extract valuable memories.

## Conversation
{}

## Output Format (JSON only)
{{"memories":[{{"content":"content","category":"user_profile|domain_knowledge|task_patterns","importance":50}}]}}

## Rules
- Skip small talk and greetings
- Only extract information with long-term value
- importance range: 0-100, higher means more important
- Write content in English by default, but adapt to user's preferred language if detected
- Categories:
  - user_profile: User preferences, habits, settings
  - domain_knowledge: Device info, protocols, environment facts
  - task_patterns: Successful approaches, common workflows
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
            r#"Analyze the agent execution log and extract valuable memories.

## Agent Name
{}

## User Intent (Prompt)
{}

## Execution Process
{}

## Execution Result
{}

## Output Format (JSON only)
{{"memories":[{{"content":"content","category":"user_profile|domain_knowledge|task_patterns|system_evolution","importance":50}}]}}

## Rules
- User preferences and habits -> user_profile
- Device states, environment patterns discovered -> domain_knowledge
- Successful task patterns, failure reasons -> task_patterns
- Lessons learned by the agent -> system_evolution
- Write content in English by default, but adapt to user's preferred language if detected
- importance range: 0-100, higher means more important
"#,
            agent_name,
            user_prompt.unwrap_or("(none)"),
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
        let messages = "User: Hello\nAssistant: Hi! How can I help you?";
        let prompt = ChatExtractor::build_prompt(messages);
        assert!(prompt.contains("Conversation"));
        assert!(prompt.contains(messages));
    }

    #[test]
    fn test_agent_extractor_prompt() {
        let prompt = AgentExtractor::build_prompt(
            "Temperature Monitor",
            Some("Monitor room temperature"),
            "1. Get temperature reading\n2. Check threshold",
            "Temperature normal",
        );
        assert!(prompt.contains("Temperature Monitor"));
        assert!(prompt.contains("Monitor room temperature"));
        assert!(prompt.contains("Temperature normal"));
    }

    #[test]
    fn test_parse_response_valid() {
        let json = r#"{"memories":[{"content":"User prefers Chinese","category":"user_profile","importance":80}]}"#;
        let result = ChatExtractor::parse_response(json).unwrap();
        assert_eq!(result.memories.len(), 1);
        assert_eq!(result.memories[0].content, "User prefers Chinese");
        assert_eq!(result.memories[0].importance, 80);
    }

    #[test]
    fn test_parse_response_with_noise() {
        let response = r#"
Here is the analysis:
{"memories":[{"content":"Device temperature 25C","category":"domain_knowledge","importance":60}]}
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
        assert_eq!(parse_category("user_profile"), MemoryCategory::UserProfile);
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
