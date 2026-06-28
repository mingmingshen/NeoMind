//! 3-condition suspected_fallback detection (spec §7).
//! All turns empty+fast AND expectations mention a tool word.
use crate::record::TurnRecord;

const TOOL_WORDS: &[&str] = &[
    "调用", "invoke", "call", "execute", "执行", "运行", "create", "创建", "delete", "删除",
    "update", "更新", "device", "rule", "agent", "dashboard", "transform", "extension",
    "channel", "push",
];

pub fn detect_suspected_fallback(turns: &[TurnRecord], expectations: &[String]) -> bool {
    // Condition 1 + 2: ALL turns have empty tool_calls AND < 500ms.
    let all_empty_fast = turns
        .iter()
        .all(|t| t.tool_calls.is_empty() && t.processing_time_ms < 500);
    if !all_empty_fast {
        return false;
    }
    // Condition 3: at least one expectation mentions a tool word.
    expectations.iter().any(|e| {
        let lower = e.to_lowercase();
        TOOL_WORDS.iter().any(|w| lower.contains(&w.to_lowercase()))
    })
}
