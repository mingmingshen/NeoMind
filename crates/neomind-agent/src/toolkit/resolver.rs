//! Generic entity name/ID resolution for LLM tool parameters.
//!
//! When LLMs call tools, they often pass human-readable names instead of
//! internal IDs (especially UUIDs). This module provides fuzzy matching
//! to resolve names to IDs, reducing the number of tool round-trips.

/// A generic entity resolver that matches user input against a list of
/// `(id, name)` candidates using progressively looser strategies.
pub struct EntityResolver;

impl EntityResolver {
    /// Resolve user input to an entity ID.
    ///
    /// Matching strategy (in order):
    /// 1. Exact ID match
    /// 2. Exact name match (case-insensitive)
    /// 3. Substring match on name or ID (case-insensitive)
    ///
    /// Returns the matched ID, or an error with helpful suggestions.
    pub fn resolve(
        input: &str,
        candidates: &[(String, String)],
        entity_type: &str,
    ) -> Result<String, String> {
        if input.is_empty() {
            return Err(format!(
                "Empty {}. Please provide a name or ID.",
                entity_type
            ));
        }

        let input_lower = input.to_lowercase();

        // 1. Exact ID match
        for (id, _name) in candidates {
            if id == input {
                return Ok(id.clone());
            }
        }

        // 2. Exact name match (case-insensitive)
        for (id, name) in candidates {
            if name.to_lowercase() == input_lower {
                return Ok(id.clone());
            }
        }

        // 3. Substring match on name or ID
        let matched: Vec<_> = candidates
            .iter()
            .filter(|(id, name)| {
                id.to_lowercase().contains(&input_lower)
                    || name.to_lowercase().contains(&input_lower)
            })
            .collect();

        match matched.len() {
            0 => Err(format!(
                "未找到 {} '{}'。请先调用 {entity_type}(action: 'list') 查看可用项。",
                entity_type, input
            )),
            1 => Ok(matched[0].0.clone()),
            _ => {
                let list: Vec<String> = matched
                    .iter()
                    .map(|(id, name)| format!("{} ({})", name, id))
                    .collect();
                Err(format!(
                    "找到多个匹配 '{}' 的{}，请指定更明确的名称: {}",
                    input,
                    entity_type,
                    list.join(", ")
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candidates() -> Vec<(String, String)> {
        vec![
            ("550e8400-aaaa-bbbb-cccc-111111111111".to_string(), "Temperature Monitor".to_string()),
            ("550e8400-aaaa-bbbb-cccc-222222222222".to_string(), "Humidity Sensor".to_string()),
            ("550e8400-aaaa-bbbb-cccc-333333333333".to_string(), "Temp Alert Agent".to_string()),
            ("ne101".to_string(), "Living Room Light".to_string()),
        ]
    }

    #[test]
    fn test_exact_id_match() {
        let result = EntityResolver::resolve(
            "550e8400-aaaa-bbbb-cccc-111111111111",
            &sample_candidates(),
            "agent",
        );
        assert_eq!(result.unwrap(), "550e8400-aaaa-bbbb-cccc-111111111111");
    }

    #[test]
    fn test_exact_id_short() {
        let result = EntityResolver::resolve("ne101", &sample_candidates(), "device");
        assert_eq!(result.unwrap(), "ne101");
    }

    #[test]
    fn test_exact_name_match() {
        let result = EntityResolver::resolve(
            "Temperature Monitor",
            &sample_candidates(),
            "agent",
        );
        assert_eq!(result.unwrap(), "550e8400-aaaa-bbbb-cccc-111111111111");
    }

    #[test]
    fn test_case_insensitive_name() {
        let result = EntityResolver::resolve(
            "temperature monitor",
            &sample_candidates(),
            "agent",
        );
        assert_eq!(result.unwrap(), "550e8400-aaaa-bbbb-cccc-111111111111");
    }

    #[test]
    fn test_substring_name_match() {
        let result = EntityResolver::resolve("Humidity", &sample_candidates(), "agent");
        assert_eq!(result.unwrap(), "550e8400-aaaa-bbbb-cccc-222222222222");
    }

    #[test]
    fn test_substring_id_match() {
        let result = EntityResolver::resolve("ne101", &sample_candidates(), "device");
        assert_eq!(result.unwrap(), "ne101");
    }

    #[test]
    fn test_substring_name_partial() {
        // "Living" matches "Living Room Light"
        let result = EntityResolver::resolve("Living", &sample_candidates(), "device");
        assert_eq!(result.unwrap(), "ne101");
    }

    #[test]
    fn test_ambiguous_returns_error_with_candidates() {
        let result = EntityResolver::resolve(
            "Temp",
            &sample_candidates(),
            "agent",
        );
        let err = result.unwrap_err();
        assert!(err.contains("多个匹配"));
        assert!(err.contains("Temperature Monitor"));
        assert!(err.contains("Temp Alert Agent"));
    }

    #[test]
    fn test_no_match_returns_error_with_hint() {
        let result = EntityResolver::resolve(
            "nonexistent",
            &sample_candidates(),
            "agent",
        );
        let err = result.unwrap_err();
        assert!(err.contains("未找到"));
        assert!(err.contains("list"));
    }

    #[test]
    fn test_empty_input_returns_error() {
        let result = EntityResolver::resolve("", &sample_candidates(), "agent");
        assert!(result.unwrap_err().contains("Empty"));
    }

    #[test]
    fn test_empty_candidates() {
        let result = EntityResolver::resolve(
            "anything",
            &[],
            "agent",
        );
        assert!(result.unwrap_err().contains("未找到"));
    }
}
