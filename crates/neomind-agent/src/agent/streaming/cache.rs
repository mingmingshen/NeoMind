use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::Value;

/// Simple in-memory cache for tool results with TTL and size limit
#[derive(Debug)]
pub struct ToolResultCache {
    entries: HashMap<String, (crate::toolkit::ToolOutput, Instant)>,
    ttl: Duration,
    max_entries: usize,
}

impl ToolResultCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
            max_entries: 1000, // Prevent unbounded memory growth
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<crate::toolkit::ToolOutput> {
        self.entries.get(key).and_then(|(result, timestamp)| {
            if timestamp.elapsed() < self.ttl {
                Some(result.clone())
            } else {
                None
            }
        })
    }

    pub(crate) fn insert(&mut self, key: String, value: crate::toolkit::ToolOutput) {
        // Enforce size limit - remove oldest entry if at capacity
        if self.entries.len() >= self.max_entries {
            // Remove the oldest entry (first key in iteration)
            if let Some(oldest_key) = self.entries.keys().next().cloned() {
                self.entries.remove(&oldest_key);
            }
        }
        self.entries.insert(key, (value, Instant::now()));
    }

    pub(crate) fn cleanup_expired(&mut self) {
        self.entries
            .retain(|_, (_, timestamp)| timestamp.elapsed() < self.ttl);

        // Also enforce size limit after cleanup
        while self.entries.len() > self.max_entries {
            if let Some(oldest_key) = self.entries.keys().next().cloned() {
                self.entries.remove(&oldest_key);
            }
        }
    }

    /// Generate cache key from tool name and arguments.
    /// Sorts object keys to ensure consistent keys regardless of parameter order.
    pub fn make_key(name: &str, arguments: &Value) -> String {
        // For objects, sort keys to ensure consistent cache keys
        if let Some(obj) = arguments.as_object() {
            let mut sorted_pairs: Vec<_> = obj.iter().collect();
            sorted_pairs.sort_by(|a, b| a.0.cmp(b.0));

            let sorted_obj: serde_json::Map<String, Value> = sorted_pairs
                .into_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            format!(
                "{}:{}",
                name,
                serde_json::to_string(&sorted_obj).unwrap_or_default()
            )
        } else {
            // For non-objects (arrays, strings, numbers, etc.), use as-is
            format!("{}:{}", name, arguments)
        }
    }
}

/// Tools that should NOT be cached (e.g., commands that change state)
const NON_CACHEABLE_TOOLS: &[&str] = &[
    "send_command",
    "execute_command",
    "set_device_state",
    "toggle_device",
    "delete_device",
];

pub(crate) fn is_tool_cacheable(name: &str) -> bool {
    !NON_CACHEABLE_TOOLS.contains(&name)
}

/// Minimum size (bytes) for a result to be considered large enough to strip base64 from.
pub(crate) const BASE64_STRIP_THRESHOLD: usize = 4096;
