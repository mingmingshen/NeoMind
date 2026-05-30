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
#[derive(Default)]
pub enum MemoryAction {
    /// Append as a new memory entry
    #[default]
    Append,
    /// Merge with existing similar memories (targets contain keywords to match)
    Merge { targets: Vec<String> },
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
- domain_knowledge: Device info, protocols, environment facts from DATA (raw readings, static facts)
- task_patterns: Successful/failed approaches, workflow patterns
- system_evolution: Discovered thresholds, behavioral baselines, optimization insights, learned patterns from EXECUTION EXPERIENCE

## Rules
1. **ONE fact per entry**, max 120 characters. Never dump paragraphs.
2. **Check existing** — if similar info exists, use merge:
   "action": {{"merge": {{"targets": ["<keyword from existing>"]}}}}
3. **Append only truly new info**
4. **system_evolution for learned insights**: Use when the agent discovered something through its own execution experience:
   - Effective thresholds (e.g., "25°C threshold too low for false alerts; 28°C is better")
   - Device behavioral baselines (e.g., "Sensor typically reads 2°C higher than actual")
   - Optimized strategies (e.g., "Checking twice before alerting reduces false positives")
   - Pattern observations (e.g., "Temperature always spikes at 14:00 in warehouse")
   NOT for: raw data readings or static facts (those go to domain_knowledge)
5. **Importance**: 80-100 = critical, 50-79 = useful, below 50 = minor
6. **Language**: match the user's detected language

## Good Examples
→ {{"content":"Living room temp threshold 26°C triggers cooling","category":"task_patterns","importance":75,"action":"append"}}
→ {{"content":"Agent learned MQTT timeout should be 5s not 10s","category":"system_evolution","importance":85,"action":"append"}}
→ {{"content":"25°C alert threshold causes too many false positives, 28°C is better","category":"system_evolution","importance":90,"action":"append"}}
→ {{"content":"Warehouse temperature spikes daily at 14:00 due to sunlight","category":"system_evolution","importance":80,"action":"append"}}
→ {{"content":"Sensor #3 consistently reads 2°C higher than calibrated baseline","category":"system_evolution","importance":75,"action":"append"}}
→ {{"content":"Double-checking before alerting reduces false positives by 80%","category":"system_evolution","importance":85,"action":"append"}}
→ {{"content":"Living room sensor reads 25-26°C range","category":"domain_knowledge","importance":60,"action":{{"merge":{{"targets":["Living room sensor"]}}}}}}

## Bad Examples (DO NOT)
- "The agent checked the temperature sensor, found it was 26°C, compared against threshold of 25°C, and decided to turn on the AC" (TOO LONG)
- "Sensor reads 25°C" → this is raw data, use domain_knowledge not system_evolution
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
        let result = AgentExtractor::parse_response(json).unwrap();
        assert_eq!(result.memories.len(), 1);
        assert_eq!(result.memories[0].content, "User prefers Chinese");
        assert_eq!(result.memories[0].importance, 80);
        assert_eq!(result.memories[0].action, MemoryAction::Append);
    }

    #[test]
    fn test_parse_response_with_merge() {
        let json = r#"{"memories":[{"content":"User has 3 IoT devices","category":"domain_knowledge","importance":70,"action":{"merge":{"targets":["2 IoT devices"]}}}]}"#;
        let result = AgentExtractor::parse_response(json).unwrap();
        assert_eq!(result.memories.len(), 1);
        assert_eq!(
            result.memories[0].action,
            MemoryAction::Merge {
                targets: vec!["2 IoT devices".to_string()]
            }
        );
    }

    #[test]
    fn test_parse_response_with_noise() {
        let response = r#"
Here is the analysis:
{"memories":[{"content":"Device temperature 25C","category":"domain_knowledge","importance":60}]}
That's all.
"#;
        let result = AgentExtractor::parse_response(response).unwrap();
        assert_eq!(result.memories.len(), 1);
    }

    #[test]
    fn test_parse_response_empty() {
        let json = r#"{"memories":[]}"#;
        let result = AgentExtractor::parse_response(json).unwrap();
        assert!(result.memories.is_empty());
    }

    #[test]
    fn test_parse_response_invalid() {
        let invalid = "not json";
        assert!(AgentExtractor::parse_response(invalid).is_err());
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
