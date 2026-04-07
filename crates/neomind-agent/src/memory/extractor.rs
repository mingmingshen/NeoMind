//! Memory extraction from Chat and Agent sources
//!
//! This module provides LLM-based extraction of memory candidates
//! from chat conversations and agent execution logs.

use neomind_storage::MemoryCategory;
use serde::{Deserialize, Serialize};

/// Default importance when LLM omits the field
const fn default_importance() -> u8 {
    50
}

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
    /// Importance score (0-100), defaults to 50 when LLM omits it
    #[serde(default = "default_importance")]
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
            r#"Extract LONG-TERM memories from the conversation. Each memory must be ONE atomic fact (max 120 chars).

## Conversation
{}

## Existing Memories
{}

## Output Format (JSON only, no extra text)
{{"memories":[{{"content":"<one fact, max 120 chars>","category":"<category>","importance":<0-100>,"action":"<append|merge>"}}]}}

## Categories
- user_profile: User preferences, habits, personal settings
- domain_knowledge: Device info, protocols, environment facts
- task_patterns: Successful approaches, common workflows

## Rules
1. **ONE fact per entry**, max 120 characters. Never dump paragraphs.
2. **Check existing first** — if similar info exists, use merge with targets:
   "action": {{"merge": {{"targets": ["<keyword from existing>"]}}}}
3. **Append only truly new info** — skip anything already covered
4. **Skip**: greetings, small talk, temporary states, questions without answers
5. **Importance**: 80-100 = critical/preference, 50-79 = useful, below 50 = minor context
6. **Language**: match the user's detected language

## Good Examples
Input: "I always want the lights dimmed after 9pm"
→ {{"content":"Lights should be dimmed after 9pm","category":"user_profile","importance":85,"action":"append"}}

Input: "My living room sensor reads 25°C" (existing: "Living room sensor reads 24°C")
→ {{"content":"Living room sensor typically reads 24-25°C","category":"domain_knowledge","importance":60,"action":{{"merge":{{"targets":["Living room sensor"]}}}}}}

## Bad Examples (DO NOT do this)
- "The user has several IoT devices including temperature sensors, motion detectors, and smart lights in the living room and bedroom" (TOO LONG, should be split)
- "User says hi" (no long-term value)
"#,
            messages, existing_section
        )
    }

    /// Parse LLM response into ExtractResult
    pub fn parse_response(response: &str) -> Result<ExtractResult, String> {
        // Strip markdown code fences (e.g., ```json ... ```)
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        // Find JSON object
        let start = cleaned.find('{').ok_or("No JSON object found")?;
        let end = cleaned.rfind('}').ok_or("No closing brace found")?;
        let json = &cleaned[start..=end];
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
            r#"Extract LONG-TERM memories from this agent execution. Each memory must be ONE atomic fact (max 120 chars).

## Agent: {}
## User Intent: {}
## Execution Process
{}

## Execution Result
{}

## Existing Memories
{}

## Output Format (JSON only, no extra text)
{{"memories":[{{"content":"<one fact, max 120 chars>","category":"<category>","importance":<0-100>,"action":"<append|merge>"}}]}}

## Categories
- user_profile: User preferences revealed during execution
- domain_knowledge: Device states, environment facts discovered
- task_patterns: Successful/failed approaches, workflow patterns
- system_evolution: Agent's own learnings, self-improvement insights

## Rules
1. **ONE fact per entry**, max 120 characters. Never dump paragraphs.
2. **Check existing** — if similar info exists, use merge:
   "action": {{"merge": {{"targets": ["<keyword from existing>"]}}}}
3. **Append only truly new info**
4. **system_evolution is ONLY for agent self-learning** (e.g. "Threshold 30°C works better than 25°C for this room")
5. **Importance**: 80-100 = critical, 50-79 = useful, below 50 = minor
6. **Language**: match the user's detected language

## Good Examples
→ {{"content":"Living room temp threshold 26°C triggers cooling","category":"task_patterns","importance":75,"action":"append"}}
→ {{"content":"Agent learned MQTT timeout should be 5s not 10s","category":"system_evolution","importance":85,"action":"append"}}
→ {{"content":"Living room sensor reads 25-26°C range","category":"domain_knowledge","importance":60,"action":{{"merge":{{"targets":["Living room sensor"]}}}}}}

## Bad Examples (DO NOT)
- "The agent checked the temperature sensor, found it was 26°C, compared against threshold of 25°C, and decided to turn on the AC" (TOO LONG)
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

/// Parse category string into enum.
/// Unknown categories are mapped to DomainKnowledge as the safest default
/// (it has the highest capacity and is the least likely to pollute user-specific data).
pub fn parse_category(s: &str) -> MemoryCategory {
    match s.to_lowercase().as_str() {
        "user_profile" => MemoryCategory::UserProfile,
        "domain_knowledge" => MemoryCategory::DomainKnowledge,
        "task_patterns" => MemoryCategory::TaskPatterns,
        "system_evolution" => MemoryCategory::SystemEvolution,
        other => {
            tracing::warn!(
                category = other,
                "Unknown memory category, falling back to DomainKnowledge"
            );
            MemoryCategory::DomainKnowledge
        }
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
        assert_eq!(parse_category("unknown"), MemoryCategory::DomainKnowledge);
    }
}
