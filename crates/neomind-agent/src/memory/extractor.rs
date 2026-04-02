//! Memory extraction from Chat and Agent sources
//!
//! This module provides LLM-based extraction of memory candidates
//! from chat conversations and agent execution logs.

use neomind_storage::MemoryCategory;
use serde::{Deserialize, Serialize};

/// Action to take when persisting a memory
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryAction {
    /// Append as a new memory entry
    Append,
    /// Merge with existing similar memories (targets contain keywords to match)
    Merge { targets: Vec<String> },
}

impl Default for MemoryAction {
    fn default() -> Self {
        Self::Append
    }
}

/// A candidate memory entry extracted by LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidate {
    /// Memory content (concise, non-redundant)
    pub content: String,
    /// Target category
    pub category: String,
    /// Importance score (0-100)
    #[serde(default)]
    pub importance: u8,
    /// Action to take (append or merge with existing)
    #[serde(default)]
    pub action: MemoryAction,
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
    pub fn build_prompt(messages: &str, existing_memories: &str) -> String {
        let existing_section = if existing_memories.trim().is_empty() {
            "(none)".to_string()
        } else {
            existing_memories.to_string()
        };

        format!(
            r#"Analyze the following conversation and extract valuable memories.

## Conversation
{}

## Existing Memories
{}

## Output Format (JSON only)
{{"memories":[{{"content":"concise content","category":"user_profile|domain_knowledge|task_patterns","importance":50,"action":"append|merge"}},{{"content":"merged content","category":"...","importance":50,"action":{{"merge":{{"targets":["keyword1","keyword2"]}}}}}}]}}

## CRITICAL Rules for Deduplication
1. **CHECK EXISTING MEMORIES FIRST** - Before extracting, check if similar info already exists
2. **MERGE when similar** - If new info relates to existing memory, use "action": {{"merge": {{"targets": ["unique keyword from existing memory"]}}}}
3. **APPEND only when truly new** - Use "action": "append" only for genuinely new information
4. **Be CONCISE** - Each memory should be ONE clear fact, not redundant details

## Content Rules
- Skip small talk and greetings
- Only extract information with long-term value
- importance range: 0-100, higher means more important
- Write content in English by default, but adapt to user's preferred language if detected
- Categories:
  - user_profile: User preferences, habits, settings
  - domain_knowledge: Device info, protocols, environment facts
  - task_patterns: Successful approaches, common workflows

## Examples
- If existing: "User has 2 IoT devices" and new info: "User's devices have 80% battery" → MERGE: "User has 2 IoT devices with battery levels at 80% and 70%"
- If existing: "User prefers Chinese" and new info: "User likes Chinese food" → APPEND (different topics)
"#,
            messages, existing_section
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
        existing_memories: &str,
    ) -> String {
        let existing_section = if existing_memories.trim().is_empty() {
            "(none)".to_string()
        } else {
            existing_memories.to_string()
        };

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

## Existing Memories
{}

## Output Format (JSON only)
{{"memories":[{{"content":"concise content","category":"user_profile|domain_knowledge|task_patterns|system_evolution","importance":50,"action":"append"}},{{"content":"merged content","category":"...","importance":50,"action":{{"merge":{{"targets":["keyword1"]}}}}}}]}}

## CRITICAL Rules for Deduplication
1. **CHECK EXISTING MEMORIES FIRST** - Before extracting, check if similar info already exists
2. **MERGE when similar** - If new info relates to existing memory, use "action": {{"merge": {{"targets": ["unique keyword from existing memory"]}}}}
3. **APPEND only when truly new** - Use "action": "append" only for genuinely new information
4. **Be CONCISE** - Each memory should be ONE clear fact, not redundant details

## Content Rules
- User preferences and habits -> user_profile
- Device states, environment patterns discovered -> domain_knowledge
- Successful task patterns, failure reasons -> task_patterns
- Lessons learned by the agent -> system_evolution
- Write content in English by default, but adapt to user's preferred language if detected
- importance range: 0-100, higher means more important

## Examples
- If existing: "Temperature sensor reads 25C" and new: "Temperature is 26C now" → MERGE: "Temperature sensor typically reads 25-26C"
- If existing: "Agent controls lights" and new: "Agent turned off lights" → MERGE with targets: ["controls lights"]
"#,
            agent_name,
            user_prompt.unwrap_or("(none)"),
            reasoning_steps,
            conclusion,
            existing_section
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
        let prompt = ChatExtractor::build_prompt(messages, "");
        assert!(prompt.contains("Conversation"));
        assert!(prompt.contains(messages));
    }

    #[test]
    fn test_chat_extractor_with_existing() {
        let messages = "User: I have 3 devices";
        let existing = "- User has 2 IoT devices";
        let prompt = ChatExtractor::build_prompt(messages, existing);
        assert!(prompt.contains("Existing Memories"));
        assert!(prompt.contains("User has 2 IoT devices"));
    }

    #[test]
    fn test_agent_extractor_prompt() {
        let prompt = AgentExtractor::build_prompt(
            "Temperature Monitor",
            Some("Monitor room temperature"),
            "1. Get temperature reading\n2. Check threshold",
            "Temperature normal",
            "",
        );
        assert!(prompt.contains("Temperature Monitor"));
        assert!(prompt.contains("Monitor room temperature"));
        assert!(prompt.contains("Temperature normal"));
    }

    #[test]
    fn test_agent_extractor_with_existing() {
        let prompt = AgentExtractor::build_prompt(
            "Temperature Monitor",
            Some("Monitor room temperature"),
            "1. Get temperature reading",
            "Temperature 26C",
            "- Temperature sensor reads 25C",
        );
        assert!(prompt.contains("Existing Memories"));
        assert!(prompt.contains("Temperature sensor reads 25C"));
    }

    #[test]
    fn test_parse_response_valid() {
        let json = r#"{"memories":[{"content":"User prefers Chinese","category":"user_profile","importance":80,"action":"append"}]}"#;
        let result = ChatExtractor::parse_response(json).unwrap();
        assert_eq!(result.memories.len(), 1);
        assert_eq!(result.memories[0].content, "User prefers Chinese");
        assert_eq!(result.memories[0].importance, 80);
        assert_eq!(result.memories[0].action, MemoryAction::Append);
    }

    #[test]
    fn test_parse_response_with_merge() {
        let json = r#"{"memories":[{"content":"User has 3 IoT devices","category":"domain_knowledge","importance":70,"action":{"merge":{"targets":["2 IoT devices"]}}}]}"#;
        let result = ChatExtractor::parse_response(json).unwrap();
        assert_eq!(result.memories.len(), 1);
        assert_eq!(result.memories[0].action, MemoryAction::Merge { targets: vec!["2 IoT devices".to_string()] });
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
