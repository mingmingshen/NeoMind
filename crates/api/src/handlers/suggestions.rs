//! Suggestions API handlers.
//!
//! Provides intelligent, context-aware suggestions for user input.

use axum::{
    extract::{Query, State},
    Json,
    response::Json as ResponseJson,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::ServerState;
use edge_ai_storage::{AgentFilter, AiAgent};

/// Suggestion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionItem {
    pub id: String,
    pub label: String,
    pub prompt: String,
    pub icon: String,  // Icon name (e.g., "Cpu", "Zap", "AlertTriangle")
    pub category: String,
}

/// Suggestions response
#[derive(Debug, Serialize)]
pub struct SuggestionsResponse {
    pub suggestions: Vec<SuggestionItem>,
    pub context: SuggestionContext,
}

/// Context information about the suggestions
#[derive(Debug, Serialize)]
pub struct SuggestionContext {
    pub timestamp: i64,
    pub learned_patterns_count: usize,
}

/// Query parameters for suggestions
#[derive(Debug, Deserialize)]
pub struct SuggestionsQuery {
    pub input: Option<String>,
    pub category: Option<String>,
    pub limit: Option<usize>,
}

/// Generate dynamic suggestions based on system state
pub async fn get_suggestions_handler(
    State(state): State<ServerState>,
    Query(params): Query<SuggestionsQuery>,
) -> ResponseJson<SuggestionsResponse> {
    let input = params.input.unwrap_or_default();
    let category = params.category;
    let limit = params.limit.unwrap_or(20);

    // Generate base suggestions
    let mut suggestions = generate_all_suggestions();

    // Get learned patterns from agents and generate pattern-based suggestions
    let pattern_suggestions = generate_pattern_based_suggestions(&state).await;
    let learned_patterns_count = pattern_suggestions.len();

    // Add pattern suggestions at the beginning (highest priority)
    if !pattern_suggestions.is_empty() {
        suggestions.splice(0..0, pattern_suggestions);
    }

    // Filter by category if specified
    if let Some(cat) = category {
        suggestions.retain(|s| s.category == cat);
    }

    // Filter by input if provided
    if !input.is_empty() {
        let input_lower = input.to_lowercase();
        suggestions.retain(|s| {
            s.label.to_lowercase().contains(&input_lower)
                || s.prompt.to_lowercase().contains(&input_lower)
        });
    }

    // Limit results
    suggestions.truncate(limit);

    let context = SuggestionContext {
        timestamp: chrono::Utc::now().timestamp(),
        learned_patterns_count,
    };

    ResponseJson(SuggestionsResponse {
        suggestions,
        context,
    })
}

/// Generate suggestions based on learned patterns from agents
async fn generate_pattern_based_suggestions(state: &ServerState) -> Vec<SuggestionItem> {
    let mut pattern_suggestions = Vec::new();

    // Get agents from the store
    let store = &state.agent_store;
    let agents = match store.query_agents(AgentFilter::default()).await {
        Ok(a) => a,
        Err(_) => return pattern_suggestions,
    };

    // Collect high-confidence learned patterns from all agents
    let mut high_confidence_patterns: Vec<(String, f32)> = Vec::new();

    for agent in agents {
        for pattern in &agent.memory.learned_patterns {
            // Only use patterns with high confidence (> 0.7)
            if pattern.confidence > 0.7 {
                // Extract action from pattern data
                if let Some(action) = pattern.data.get("action") {
                    if let Some(action_str) = action.as_str() {
                        high_confidence_patterns.push((
                            format!("类似: {}", action_str),
                            pattern.confidence,
                        ));
                    }
                }
                // Also use description as suggestion
                if !pattern.description.is_empty() {
                    high_confidence_patterns.push((
                        pattern.description.clone(),
                        pattern.confidence,
                    ));
                }
            }
        }
    }

    // Sort by confidence and deduplicate
    high_confidence_patterns.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Deduplicate by keeping first occurrence of each description
    let mut seen = std::collections::HashSet::new();
    high_confidence_patterns.retain(|(desc, _)| seen.insert(desc.clone()));

    // Convert to suggestion items (max 3 pattern-based suggestions)
    for (description, _confidence) in high_confidence_patterns.into_iter().take(3) {
        pattern_suggestions.push(SuggestionItem {
            id: format!("pattern-{}", pattern_suggestions.len()),
            label: description.clone(),
            prompt: description.clone(),
            icon: "History".to_string(),
            category: "agent".to_string(),
        });
    }

    pattern_suggestions
}

/// Generate all available suggestions
fn generate_all_suggestions() -> Vec<SuggestionItem> {
    vec![
        // Device suggestions
        SuggestionItem {
            id: "device-list".to_string(),
            label: "查看所有设备".to_string(),
            prompt: "查看所有设备状态".to_string(),
            icon: "Cpu".to_string(),
            category: "device".to_string(),
        },
        SuggestionItem {
            id: "device-online".to_string(),
            label: "查看在线设备".to_string(),
            prompt: "哪些设备当前在线".to_string(),
            icon: "Cpu".to_string(),
            category: "device".to_string(),
        },
        SuggestionItem {
            id: "device-temp".to_string(),
            label: "查看温度传感器".to_string(),
            prompt: "查看所有温度传感器的读数".to_string(),
            icon: "Cpu".to_string(),
            category: "device".to_string(),
        },
        // Automation suggestions
        SuggestionItem {
            id: "automation-list".to_string(),
            label: "查看自动化规则".to_string(),
            prompt: "查看所有自动化规则".to_string(),
            icon: "Zap".to_string(),
            category: "automation".to_string(),
        },
        SuggestionItem {
            id: "automation-create".to_string(),
            label: "创建自动化规则".to_string(),
            prompt: "创建新的自动化规则".to_string(),
            icon: "Zap".to_string(),
            category: "automation".to_string(),
        },
        SuggestionItem {
            id: "workflow-list".to_string(),
            label: "查看工作流".to_string(),
            prompt: "查看所有工作流".to_string(),
            icon: "Zap".to_string(),
            category: "automation".to_string(),
        },
        // Alert suggestions
        SuggestionItem {
            id: "alert-list".to_string(),
            label: "查看告警".to_string(),
            prompt: "查看当前告警".to_string(),
            icon: "AlertTriangle".to_string(),
            category: "alert".to_string(),
        },
        SuggestionItem {
            id: "alert-create".to_string(),
            label: "创建告警规则".to_string(),
            prompt: "创建新的告警规则".to_string(),
            icon: "AlertTriangle".to_string(),
            category: "alert".to_string(),
        },
        // Analytics suggestions
        SuggestionItem {
            id: "analytics-temp".to_string(),
            label: "温度数据分析".to_string(),
            prompt: "分析最近24小时的温度数据".to_string(),
            icon: "TrendingUp".to_string(),
            category: "analytics".to_string(),
        },
        SuggestionItem {
            id: "analytics-trend".to_string(),
            label: "查看数据趋势".to_string(),
            prompt: "查看设备数据趋势".to_string(),
            icon: "TrendingUp".to_string(),
            category: "analytics".to_string(),
        },
        // Agent suggestions
        SuggestionItem {
            id: "agent-status".to_string(),
            label: "查看Agent状态".to_string(),
            prompt: "查看所有Agent的运行状态".to_string(),
            icon: "Bot".to_string(),
            category: "agent".to_string(),
        },
        SuggestionItem {
            id: "agent-history".to_string(),
            label: "查看Agent执行历史".to_string(),
            prompt: "显示最近的Agent执行记录".to_string(),
            icon: "History".to_string(),
            category: "agent".to_string(),
        },
        // Settings suggestions
        SuggestionItem {
            id: "settings-llm".to_string(),
            label: "LLM设置".to_string(),
            prompt: "查看LLM后端配置".to_string(),
            icon: "Settings".to_string(),
            category: "settings".to_string(),
        },
        // General suggestions
        SuggestionItem {
            id: "help".to_string(),
            label: "帮助".to_string(),
            prompt: "你能做什么".to_string(),
            icon: "Lightbulb".to_string(),
            category: "general".to_string(),
        },
    ]
}

/// Get suggestions categories (for UI filtering)
pub async fn get_suggestions_categories_handler() -> ResponseJson<Vec<String>> {
    ResponseJson(vec![
        "device".to_string(),
        "automation".to_string(),
        "alert".to_string(),
        "analytics".to_string(),
        "agent".to_string(),
        "settings".to_string(),
        "general".to_string(),
    ])
}

