//! Token estimation for context window management.
//!
//! Provides accurate token counting for Chinese, English, and code content.

/// Estimate token count for a text string.
///
/// This uses a heuristic approach that's more accurate than simple character division:
/// - Chinese characters: ~1.8 tokens each
/// - English words: ~0.8 tokens each
/// - Special characters/punctuation: ~1.2 tokens each
pub fn estimate_tokens(text: &str) -> usize {
    let mut tokens = 0f64;

    for line in text.lines() {
        let chinese_count = line.chars().filter(|c| is_chinese(*c)).count() as f64;
        let english_count = line.chars().filter(|c| c.is_ascii_alphabetic()).count() as f64;
        let number_count = line.chars().filter(|c| c.is_ascii_digit()).count() as f64;
        let special_count = line.chars().filter(|c| !c.is_alphanumeric()).count() as f64;

        // Chinese characters (CJK Unified Ideographs)
        tokens += chinese_count * 1.8;

        // English words (rough estimate: 4 chars = 1 word = 0.8 tokens)
        tokens += english_count * 0.25;

        // Numbers (similar to English)
        tokens += number_count * 0.3;

        // Special characters and punctuation
        tokens += special_count * 0.5;
    }

    // Add a small buffer for safety
    (tokens * 1.1).ceil() as usize
}

/// Check if a character is a Chinese/Japanese/Korean character.
fn is_chinese(c: char) -> bool {
    let cp = c as u32;
    // CJK Unified Ideographs
    (0x4E00..=0x9FFF).contains(&cp) ||
    // CJK Extension A
    (0x3400..=0x4DBF).contains(&cp) ||
    // CJK Compatibility Ideographs
    (0xF900..=0xFAFF).contains(&cp) ||
    // Fullwidth forms
    (0xFF00..=0xFFEF).contains(&cp) ||
    // Hiragana, Katakana
    (0x3040..=0x309F).contains(&cp) ||
    (0x30A0..=0x30FF).contains(&cp)
}

/// Estimate token count for a message.
pub fn estimate_message_tokens(message: &crate::agent::AgentMessage) -> usize {
    let mut tokens = estimate_tokens(&message.content);

    // Add tokens for thinking content
    if let Some(thinking) = &message.thinking {
        tokens += estimate_tokens(thinking);
    }

    // Add tokens for tool calls
    if let Some(tool_calls) = &message.tool_calls {
        for tool_call in tool_calls {
            // Tool name + arguments roughly (convert JSON to string for estimation)
            let args_str = tool_call.arguments.to_string();
            tokens += 10 + estimate_tokens(&args_str);
        }
    }

    tokens
}

/// Calculate how many messages fit within a token budget.
pub fn select_messages_within_token_limit(
    messages: &[crate::agent::AgentMessage],
    max_tokens: usize,
    min_messages: usize,
) -> Vec<&crate::agent::AgentMessage> {
    let mut selected = Vec::new();
    let mut current_tokens = 0;

    // Always include at least min_messages most recent messages
    let _min_from_end = messages.len().saturating_sub(min_messages);

    // Process in reverse (most recent first)
    for (i, msg) in messages.iter().rev().enumerate() {
        // Always keep the most recent min_messages
        if i < min_messages {
            selected.push(msg);
            current_tokens += estimate_message_tokens(msg);
            continue;
        }

        // Check if adding this message would exceed the limit
        let msg_tokens = estimate_message_tokens(msg);
        if current_tokens + msg_tokens > max_tokens {
            break;
        }

        selected.push(msg);
        current_tokens += msg_tokens;
    }

    selected.reverse();
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_chinese() {
        // Chinese text: ~1.8 tokens per character
        let tokens = estimate_tokens("你好世界");
        assert!(tokens > 4, "Chinese should count more than char count");
        assert!(tokens < 15, "Should be reasonable");
    }

    #[test]
    fn test_estimate_english() {
        // English text: ~0.25 tokens per character (4 chars = 1 token)
        let tokens = estimate_tokens("Hello world");
        assert!(tokens > 0);
        assert!(tokens < 10);
    }

    #[test]
    fn test_estimate_mixed() {
        let tokens = estimate_tokens("你好 world 你好");
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_code() {
        let code = r#"
            fn main() {
                println!("你好");
                let x = 42;
            }
        "#;
        let tokens = estimate_tokens(code);
        assert!(tokens > 0);
    }
}
