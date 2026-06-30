//! Prompt generation utilities for the NeoMind AI Agent.
//!
//! ## Architecture
//!
//! Three-layer documentation:
//! 1. System prompt (~2,000 tokens) — core decision rules, always loaded
//! 2. CLI `--help` — command details, loaded on demand by LLM running `neomind <cmd> --help`
//! 3. Skill tool — complex workflows and error troubleshooting, loaded on demand

/// Placeholder for current UTC time in prompts.
pub const CURRENT_TIME_PLACEHOLDER: &str = "{{CURRENT_TIME}}";

/// Placeholder for current local time in prompts.
pub const LOCAL_TIME_PLACEHOLDER: &str = "{{LOCAL_TIME}}";

/// Placeholder for system timezone in prompts.
pub const TIMEZONE_PLACEHOLDER: &str = "{{TIMEZONE}}";

/// Language policy prepended to all prompts, instructing the LLM to respond in the user's language.
///
/// Content lives in `language_policy.md` next to this file so prompt edits no longer
/// require a Rust recompile. The string is `include_str!`-substituted at compile time —
/// byte-identical to the previous inline raw-string literal.
pub const LANGUAGE_POLICY: &str = include_str!("language_policy.md");

/// Enhanced prompt builder.
#[derive(Debug, Clone)]
pub struct PromptBuilder {
    /// Whether to include thinking mode instructions
    include_thinking: bool,
    /// Whether this model supports vision/multimodal input
    supports_vision: bool,
}

impl PromptBuilder {
    /// Create a new prompt builder.
    /// The prompt instructs the LLM to respond in the same language as the user's input.
    pub fn new() -> Self {
        Self {
            include_thinking: true,
            supports_vision: false,
        }
    }

    /// Enable or disable thinking mode instructions.
    pub fn with_thinking(mut self, include: bool) -> Self {
        self.include_thinking = include;
        self
    }

    /// Enable or disable vision/multimodal capability.
    /// When enabled, adds instructions for processing images.
    pub fn with_vision(mut self, supports_vision: bool) -> Self {
        self.supports_vision = supports_vision;
        self
    }

    /// Build the enhanced system prompt.
    pub fn build_system_prompt(&self) -> String {
        let mut prompt = String::with_capacity(4096);

        prompt.push_str(LANGUAGE_POLICY);
        prompt.push_str("\n\n");

        prompt.push_str(Self::IDENTITY);
        prompt.push_str("\n\n");

        if self.supports_vision {
            prompt.push_str(Self::VISION_HINT);
            prompt.push_str("\n\n");
        }

        prompt.push_str(Self::PRINCIPLES);
        prompt.push_str("\n\n");

        prompt.push_str(Self::TOOL_STRATEGY);
        prompt.push_str("\n\n");

        prompt.push_str(Self::MEMORY_USAGE);
        prompt.push('\n');

        if self.include_thinking {
            prompt.push('\n');
            prompt.push_str(Self::THINKING_GUIDELINES);
        }

        prompt
    }

    // === Static content constants ===
    //
    // Each prompt section lives in its own `.md` file next to this source so the
    // prompts can be edited without touching Rust. `include_str!` performs a
    // compile-time byte-for-byte substitution — the produced `&'static str` is
    // identical to the previous inline raw-string literals.
    //
    // Guardrail: `test_system_prompt_byte_stable` (below) asserts that the composed
    // `build_system_prompt()` hash matches a baseline captured at the refactor
    // commit. If you intentionally edit any `.md` file, update the baseline hash
    // in that test in the same commit.

    const IDENTITY: &str = include_str!("identity.md");

    const VISION_HINT: &str = include_str!("vision_hint.md");

    const PRINCIPLES: &str = include_str!("principles.md");

    const TOOL_STRATEGY: &str = include_str!("tool_strategy.md");

    const MEMORY_USAGE: &str = include_str!("memory_usage.md");

    const THINKING_GUIDELINES: &str = include_str!("thinking_guidelines.md");

    /// Get intent-specific system prompt addon.
    pub fn get_intent_prompt_addon(&self, intent: &str) -> String {
        match intent {
            "device" => "\n\n## Current Task: Device Management\nFocus on device queries and control operations.".to_string(),
            "data" => "\n\n## Current Task: Data Query and Analysis\n**MUST CALL TOOLS**: When user asks for historical data, trend analysis, or data changes, you MUST call shell tool with `neomind device history <id> --metric <name>` to get real data.\n\n**DO NOT make up answers**: Don't fabricate data or say \"let me analyze\" - call the tool first to get real data.".to_string(),
            "rule" => "\n\n## Current Task: Rule Management\nFocus on creating and modifying automation rules.".to_string(),
            "alert" | "message" => "\n\n## Current Task: Message Management\nFocus on message queries, sending, and status updates.".to_string(),
            "system" => "\n\n## Current Task: System Status\nFocus on system health checks and status queries.".to_string(),
            "help" => "\n\n## Current Task: Help & Documentation\nProvide clear usage instructions and feature overview without calling tools.".to_string(),
            _ => String::new(),
        }
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_builder_default() {
        let builder = PromptBuilder::new();
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("NeoMind"));
        assert!(prompt.contains("IoT"));
        assert!(prompt.contains("Principles"));
        assert!(!prompt.contains("Visual Understanding"));
    }

    #[test]
    fn test_prompt_without_examples() {
        let builder = PromptBuilder::new();
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("Principles"));
        assert!(!prompt.contains("Example Dialogs"));
    }

    #[test]
    fn test_prompt_without_thinking() {
        let builder = PromptBuilder::new().with_thinking(false);
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("Principles"));
        assert!(!prompt.contains("Thinking Mode"));
    }

    #[test]
    fn test_intent_addon() {
        let builder = PromptBuilder::new();
        let addon = builder.get_intent_prompt_addon("data");
        assert!(addon.contains("Data Query"));
    }

    #[test]
    fn test_language_policy_in_prompt() {
        let builder = PromptBuilder::new();
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("Language Policy"));
        assert!(prompt.contains("Highest Priority"));
        let prompt_lower = prompt.to_lowercase();
        assert!(prompt_lower.contains("same language"));
    }

    #[test]
    fn test_no_cli_reference_table() {
        let builder = PromptBuilder::new();
        let prompt = builder.build_system_prompt();
        // CLI reference table should be removed
        assert!(!prompt.contains("CLI Command Reference"));
        // But --help guidance should be present
        assert!(prompt.contains("--help"));
    }

    #[test]
    fn test_no_example_responses() {
        let builder = PromptBuilder::new();
        let prompt = builder.build_system_prompt();
        assert!(!prompt.contains("Example Dialogs"));
    }

    #[test]
    fn test_no_vision_when_disabled() {
        let builder = PromptBuilder::new().with_vision(false);
        let prompt = builder.build_system_prompt();
        assert!(!prompt.contains("Vision"));
    }

    #[test]
    fn test_vision_when_enabled() {
        let builder = PromptBuilder::new().with_vision(true);
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("Vision"));
        assert!(prompt.contains("analyze images"));
    }

    #[test]
    fn test_key_rules_preserved() {
        let builder = PromptBuilder::new();
        let prompt = builder.build_system_prompt();
        // Tier 1 rules must remain
        assert!(prompt.contains("No Hallucinated Operations"));
        assert!(prompt.contains("Complete Every Task"));
        assert!(prompt.contains("BATCH RULE"));
        assert!(prompt.contains("Domain Boundaries"));
        assert!(prompt.contains("Chinese Term Mapping"));
    }

    /// Byte-stable guardrail for the chat-agent system prompt.
    ///
    /// Captured at the `const → include_str!()` refactor commit (2026-06-30).
    /// Asserts that `build_system_prompt()` produces byte-identical output across
    /// the refactor. If you **intentionally** edit any of the `.md` files under
    /// `crates/neomind-agent/src/prompts/`, recompute the baseline length/hash
    /// below in the same commit — the failing assertion message will print the
    /// new values for you.
    ///
    /// Why both length AND hash:
    /// - length alone catches gross content drift (deleted/added sections)
    /// - hash catches single-character edits that length misses
    #[test]
    fn test_system_prompt_byte_stable() {
        let prompt = PromptBuilder::new().build_system_prompt();
        let bytes = prompt.as_bytes();

        // Baseline captured 2026-07-01 after removing rules.md, trimming
        // thinking_guidelines/principles/tool_strategy of cross-file duplicates.
        // Update ONLY when an intentional prompt change lands.
        const BASELINE_LEN: usize = 9122;
        const BASELINE_HASH: u64 = 18083644095083542179;

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(bytes, &mut hasher);
        let actual_hash = std::hash::Hasher::finish(&hasher);

        // Soft-check length first — gives a clearer diff message on failure.
        if bytes.len() != BASELINE_LEN {
            panic!(
                "system_prompt length drifted: baseline={}, actual={}. \
                 If this is intentional, update BASELINE_LEN/BASELINE_HASH in this test. \
                 New hash: {}",
                BASELINE_LEN,
                bytes.len(),
                actual_hash
            );
        }
        if actual_hash != BASELINE_HASH {
            panic!(
                "system_prompt hash drifted: baseline={}, actual={}. \
                 Length matches ({}), so a single-character edit is the likely cause. \
                 If this is intentional, update BASELINE_HASH in this test.",
                BASELINE_HASH, actual_hash, bytes.len()
            );
        }
    }
}
