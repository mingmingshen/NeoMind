//! Skill matcher: scores and selects relevant skills for a user message.

use super::registry::SkillRegistry;
use super::types::*;

/// Score a single skill against user input.
fn score_skill(skill: &Skill, user_input: &str) -> f32 {
    let input_lower = user_input.to_lowercase();
    let mut score: f32 = 0.0;

    // Keyword matching (+0.4 per exact match)
    for keyword in &skill.metadata.triggers.keywords {
        let kw_lower = keyword.to_lowercase();
        if input_lower.contains(&kw_lower) {
            score += 0.4;
        }
    }

    // Tool-action matching (+0.5 for tool+action match)
    for target in &skill.metadata.triggers.tool_target {
        let tool_lower = target.tool.to_lowercase();
        if input_lower.contains(&tool_lower) {
            // Check if any action keyword matches
            let action_match = target.actions.iter().any(|action| {
                let action_lower = action.to_lowercase();
                input_lower.contains(&action_lower)
            });
            if action_match {
                score += 0.5;
            } else {
                // Tool name matched but no action
                score += 0.2;
            }
        }
    }

    // Anti-trigger exclusion (-1.0 if any anti-trigger keyword matches)
    for anti_kw in &skill.metadata.anti_triggers.keywords {
        let anti_lower = anti_kw.to_lowercase();
        if input_lower.contains(&anti_lower) {
            score -= 1.0;
        }
    }

    // Priority weight (0-0.1 based on priority)
    score += (skill.metadata.priority as f32 / 100.0) * 0.1;

    score
}

/// Match skills against user input and return scored results within token budget.
pub fn match_skills(
    registry: &SkillRegistry,
    user_input: &str,
    budget: TokenBudgetConfig,
) -> Vec<SkillMatch> {
    let mut candidates: Vec<SkillMatch> = Vec::new();

    for skill in registry.list() {
        let score = score_skill(skill, user_input);
        if score > 0.0 {
            let body = skill.body_within_budget();
            let token_count = body.len() / 4;
            candidates.push(SkillMatch {
                skill_id: skill.metadata.id.clone(),
                skill_name: skill.metadata.name.clone(),
                score,
                body,
                token_count,
            });
        }
    }

    // Sort by score descending
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Apply token budget
    let mut result = Vec::new();
    let mut used_tokens = 0;

    for candidate in candidates {
        if used_tokens + candidate.token_count <= budget.max_tokens {
            used_tokens += candidate.token_count;
            result.push(candidate);
        }
        // Skip skills that don't fit in budget rather than truncating further
    }

    result
}

/// Format matched skills into a prompt section for injection.
pub fn format_skill_matches(matches: &[SkillMatch]) -> String {
    if matches.is_empty() {
        return String::new();
    }

    let mut output = String::from("\n## Skill Guides\n");
    output.push_str("Follow these guides when performing the relevant operations:\n\n");

    for m in matches {
        output.push_str(&format!("### {}\n", m.skill_name));
        output.push_str(&m.body);
        output.push_str("\n\n");
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_registry() -> SkillRegistry {
        let mut registry = SkillRegistry::new();
        // Add test skills inline
        let rule_mgmt = r#"---
id: rule-management
name: Rule Management
category: rule
priority: 85
token_budget: 800
triggers:
  keywords: [delete rule, remove rule, 删除规则, 修改规则, 更新规则]
  tool_target:
    tool: rule
    actions: [delete, update, enable]
anti_triggers:
  keywords: [create rule, 创建规则, 新建规则]
---

# Rule Management

Step 1: list to get rule_id
Step 2: ONE action (delete/update/enable)"#;
        registry.add_user_skill(rule_mgmt).unwrap();
        registry
    }

    #[test]
    fn test_keyword_match_scores_high() {
        let registry = make_test_registry();
        let budget = TokenBudgetConfig::for_context(8000);
        let matches = match_skills(&registry, "删除规则 rule-001", budget);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].skill_id, "rule-management");
    }

    #[test]
    fn test_anti_trigger_excludes() {
        let registry = make_test_registry();
        let budget = TokenBudgetConfig::for_context(8000);
        let matches = match_skills(&registry, "创建规则 temperature-rule", budget);
        let has_mgmt = matches.iter().any(|m| m.skill_id == "rule-management");
        assert!(!has_mgmt, "Anti-trigger should exclude rule-management");
    }

    #[test]
    fn test_no_keyword_match_returns_low_score() {
        let registry = make_test_registry();
        let budget = TokenBudgetConfig::for_context(8000);
        let matches = match_skills(&registry, "天气怎么样", budget);
        // Priority weight alone produces a low score; no strong match
        for m in &matches {
            assert!(
                m.score < 0.2,
                "Unrelated query should have low score, got {}",
                m.score
            );
        }
    }

    #[test]
    fn test_token_budget_respected() {
        let registry = make_test_registry();
        let budget = TokenBudgetConfig { max_tokens: 100 };
        let matches = match_skills(&registry, "删除规则", budget);
        let total_tokens: usize = matches.iter().map(|m| m.token_count).sum();
        assert!(total_tokens <= 100, "Total tokens should respect budget");
    }

    #[test]
    fn test_format_skill_matches() {
        let matches = vec![SkillMatch {
            skill_id: "test".into(),
            skill_name: "Test Skill".into(),
            score: 0.5,
            body: "Do this thing.".into(),
            token_count: 4,
        }];
        let formatted = format_skill_matches(&matches);
        assert!(formatted.contains("## Skill Guides"));
        assert!(formatted.contains("### Test Skill"));
        assert!(formatted.contains("Do this thing."));
    }

    #[test]
    fn test_empty_match_returns_empty_string() {
        let formatted = format_skill_matches(&[]);
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_context_size_budgets() {
        assert_eq!(TokenBudgetConfig::for_context(3000).max_tokens, 400);
        assert_eq!(TokenBudgetConfig::for_context(4000).max_tokens, 400);
        assert_eq!(TokenBudgetConfig::for_context(5000).max_tokens, 800);
        assert_eq!(TokenBudgetConfig::for_context(8000).max_tokens, 800);
        assert_eq!(TokenBudgetConfig::for_context(10000).max_tokens, 1500);
    }

    #[test]
    fn test_update_rule_match() {
        let registry = make_test_registry();
        let budget = TokenBudgetConfig::for_context(8000);
        let matches = match_skills(&registry, "修改规则 temperature-rule", budget);
        assert!(
            matches.iter().any(|m| m.skill_id == "rule-management"),
            "Should match rule-management"
        );
    }

    #[test]
    fn test_empty_registry_returns_empty() {
        let registry = SkillRegistry::new();
        let budget = TokenBudgetConfig::for_context(8000);
        let matches = match_skills(&registry, "删除规则", budget);
        assert!(matches.is_empty());
    }
}
