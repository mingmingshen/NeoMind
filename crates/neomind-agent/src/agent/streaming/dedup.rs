/// Deduplicate accumulated tool results across multiple rounds.
///
/// Keeps the **latest** result for each (tool_name, key_arguments) combination.
/// When the same tool is called with the same arguments across rounds (LLM retrying),
/// only the last successful result is kept. Different arguments produce separate entries.
pub(crate) fn deduplicate_tool_results(results: &[(String, String)]) -> Vec<(String, String)> {
    // Build a key from tool name + distinguishing arguments parsed from the result JSON
    let mut seen: Vec<(String, String)> = Vec::new(); // (key, dedup_key)
    let mut deduped: Vec<(String, String)> = Vec::new();

    for (name, result) in results {
        // Create a dedup key from name + result fingerprint
        let dedup_key = make_result_dedup_key(name, result);

        if let Some(pos) = seen
            .iter()
            .position(|(k, dk)| k == name && dk == &dedup_key)
        {
            // Replace with latest result
            deduped[pos] = (name.clone(), result.clone());
        } else {
            seen.push((name.clone(), dedup_key));
            deduped.push((name.clone(), result.clone()));
        }
    }

    deduped
}

/// Create a dedup key for a tool result by extracting entity identifiers.
pub(crate) fn make_result_dedup_key(name: &str, result: &str) -> String {
    // Try to extract entity IDs from the result JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(result) {
        let mut key_parts = vec![name.to_string()];

        // Extract common entity identifiers
        for field in &["device_id", "metric", "agent_id", "rule_id", "id", "name"] {
            if let Some(val) = json.get(*field).and_then(|v| v.as_str()) {
                key_parts.push(val.to_string());
            }
        }

        // For device query results, also check nested data
        if let Some(data) = json.get("data") {
            if let Some(obj) = data.as_object() {
                for field in &["device_id", "device_name"] {
                    if let Some(val) = obj.get(*field).and_then(|v| v.as_str()) {
                        key_parts.push(val.to_string());
                    }
                }
            }
        }

        return key_parts.join("|");
    }

    // Fallback: simple hash of the result content for dedup
    let preview: String = result.chars().take(200).collect();
    let hash = preview
        .chars()
        .fold(0u64, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u64));
    format!("{}|{:016x}", name, hash)
}
