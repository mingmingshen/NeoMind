//! Multimodal (text + images) streaming response processing.
//!
//! Contains `process_multimodal_stream_events` for LLM interactions that
//! include images alongside text, with tool calling support.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use futures::{Stream, StreamExt};

use super::context::{build_context_window_with_summary, ToolExecutionResult};
use super::dedup::deduplicate_tool_results;
use super::result_format::format_tool_results;
use super::sanitize::sanitize_tool_result_for_prompt;
use super::stream_core::StreamSafeguards;
use super::tool_detect::detect_json_tool_calls;
use super::tool_exec::execute_tool_with_retry;
use super::resolve::resolve_cached_arguments;
use crate::agent::tool_parser::{parse_tool_calls, remove_tool_calls_from_response};
use crate::agent::types::{AgentEvent, AgentInternalState, AgentMessage, AgentMessageImage, ToolCall};
use crate::error::{NeoMindError, Result};
use crate::llm::LlmInterface;

/// Process a multimodal user message (text + images) with streaming response.
///
/// This is similar to `process_stream_events` but accepts images as base64 data URLs.
/// Images are converted to ContentPart::ImageBase64 for the LLM.
pub async fn process_multimodal_stream_events(
    llm_interface: Arc<LlmInterface>,
    internal_state: Arc<tokio::sync::RwLock<AgentInternalState>>,
    tools: Arc<crate::toolkit::ToolRegistry>,
    user_message: &str,
    images: Vec<String>, // Base64 data URLs (e.g., "data:image/png;base64,...")
) -> Result<Pin<Box<dyn Stream<Item = AgentEvent> + Send>>> {
    process_multimodal_stream_events_with_safeguards(
        llm_interface,
        internal_state,
        tools,
        user_message,
        images,
        StreamSafeguards::default(),
        None,
        None,
    )
    .await
}

/// Process multimodal message with configurable safeguards.
#[allow(clippy::too_many_arguments)]
pub async fn process_multimodal_stream_events_with_safeguards(
    llm_interface: Arc<LlmInterface>,
    internal_state: Arc<tokio::sync::RwLock<AgentInternalState>>,
    tools: Arc<crate::toolkit::ToolRegistry>,
    user_message: &str,
    images: Vec<String>,
    safeguards: StreamSafeguards,
    conversation_summary: Option<String>,
    summary_up_to_index: Option<u64>,
) -> Result<Pin<Box<dyn Stream<Item = AgentEvent> + Send>>> {
    use neomind_core::ContentPart;

    let user_message = user_message.to_string();

    // Build multimodal message content with images
    let mut parts = vec![ContentPart::text(&user_message)];

    // Add images as ContentPart
    for image_data in &images {
        if let Some(parsed) = crate::image_utils::parse_image_data(image_data) {
            parts.push(ContentPart::image_base64(parsed.base64, parsed.mime_type));
        }
    }

    // Get conversation history
    let state_guard = internal_state.read().await;
    let history_messages = state_guard.memory.clone();
    drop(state_guard);

    // Build context window — measure actual prompt overhead instead of guessing
    let max_context = llm_interface.max_context_length().await;
    let prompt_overhead = llm_interface.estimate_prompt_overhead_tokens().await;
    let effective_max = max_context
        .saturating_sub(prompt_overhead)
        .saturating_sub(1024)
        .max((max_context * 20) / 100);

    let history_for_llm: Vec<neomind_core::Message> = build_context_window_with_summary(
        &history_messages,
        effective_max,
        conversation_summary.as_deref(),
        summary_up_to_index,
    )
    .iter()
    .map(|msg| msg.to_core())
    .collect::<Vec<_>>();

    tracing::debug!(
        "Passing {} messages from history to LLM (multimodal)",
        history_for_llm.len()
    );

    // Create multimodal user message
    let multimodal_user_msg = neomind_core::Message::new(
        neomind_core::MessageRole::User,
        neomind_core::Content::Parts(parts),
    );

    // Use regular multimodal chat (with thinking enabled)
    // Thinking helps the model analyze images more thoroughly
    let stream_result = llm_interface
        .chat_stream_multimodal_with_history(multimodal_user_msg, &history_for_llm)
        .await;

    let stream = stream_result.map_err(|e| NeoMindError::Llm(e.to_string()))?;

    // Check if images are present (before moving images)
    let has_images = !images.is_empty();

    // Extract base64 data for caching before images are consumed
    let image_base64_list: Vec<String> = images
        .iter()
        .filter_map(|data_url| data_url.split(',').nth(1).map(|s| s.to_string()))
        .collect();

    // Store user message in history with images
    // Convert the image strings to AgentMessageImage
    let user_images: Vec<AgentMessageImage> = images
        .into_iter()
        .map(|data_url| {
            let mime_type = crate::image_utils::parse_image_data(&data_url)
                .map(|p| p.mime_type.to_string());
            AgentMessageImage {
                data: data_url,
                mime_type,
            }
        })
        .collect();

    let user_msg = AgentMessage::user_with_images(&user_message, user_images);
    internal_state.write().await.push_message(user_msg);

    // Cache user-uploaded images so tools can reference them via $cached:user_image
    if !image_base64_list.is_empty() {
        let mut state = internal_state.write().await;
        for (i, base64_data) in image_base64_list.iter().enumerate() {
            let cache_key = if i == 0 {
                "user_image".to_string()
            } else {
                format!("user_image_{}", i)
            };
            state.large_data_cache.store(&cache_key, base64_data);
        }
    }

    Ok(Box::pin(async_stream::stream! {
        let mut stream = stream;
        let mut buffer = String::new();
        let mut tool_calls_detected = false;
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut content_before_tools = String::new();

        // === SKILL CONTEXT: Clear transient skill context from previous turn ===
        llm_interface.clear_skill_context().await;

        let stream_start = Instant::now();
        let mut last_event_time = Instant::now();

        // Simple progress event (only for images)
        if has_images {
            yield AgentEvent::progress("正在分析图像...", "analyzing", 0);
            last_event_time = Instant::now();
        }

        // Stream the response
        while let Some(result) = StreamExt::next(&mut stream).await {
            let elapsed = stream_start.elapsed();

            if elapsed > safeguards.max_stream_duration {
                tracing::warn!("Stream timeout ({:?} elapsed)", elapsed);
                yield AgentEvent::error(format!("Request timeout ({:.1}s elapsed)", elapsed.as_secs_f64()));
                break;
            }

            // Heartbeat
            if last_event_time.elapsed() > safeguards.heartbeat_interval {
                yield AgentEvent::heartbeat();
                last_event_time = Instant::now();
            }

            match result {
                Ok((text, is_thinking)) => {
                    if text.is_empty() {
                        continue;
                    }

                    if is_thinking {
                        yield AgentEvent::thinking(text.clone());
                        last_event_time = Instant::now();
                        continue;
                    }

                    buffer.push_str(&text);
                    last_event_time = Instant::now();

                    // Check for tool calls in buffer
                    let json_tool_check = detect_json_tool_calls(&buffer);
                    if let Some((json_start, tool_json, _remaining)) = json_tool_check {
                        let before_tool = &buffer[..json_start];
                        if !before_tool.is_empty() {
                            content_before_tools.push_str(before_tool);
                            yield AgentEvent::content(before_tool);
                        }

                        if let Ok((_, calls)) = parse_tool_calls(&tool_json) {
                            tool_calls_detected = true;
                            tool_calls.extend(calls);
                        }

                        // Discard remaining hallucinated content after embedded tool calls
                        buffer.clear();
                    } else {
                        // No JSON tool calls detected - check for XML format
                        if let Some(tool_start) = buffer.find("<tool_calls>") {
                            let before_tool = &buffer[..tool_start];
                            if !before_tool.is_empty() {
                                content_before_tools.push_str(before_tool);
                                yield AgentEvent::content(before_tool);
                            }

                            if let Some(tool_end) = buffer.find("</tool_calls>") {
                                let tool_content = buffer[tool_start..tool_end + 13].to_string();
                                // Discard remaining hallucinated content after XML tool calls
                                buffer.clear();

                                if let Ok((_, calls)) = parse_tool_calls(&tool_content) {
                                    tool_calls_detected = true;
                                    tool_calls.extend(calls);
                                }
                            }
                        } else {
                            // No tool calls detected - yield content immediately for real-time streaming
                            if !text.is_empty() {
                                yield AgentEvent::content(text.clone());
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                    yield AgentEvent::error(format!("Stream error: {}", e));
                    // Save partial response on error to maintain conversation context
                    if !buffer.is_empty() || !content_before_tools.is_empty() {
                        let partial_content = if content_before_tools.is_empty() {
                            buffer.clone()
                        } else {
                            content_before_tools.clone()
                        };
                        let partial_msg = AgentMessage::assistant(&partial_content);
                        internal_state.write().await.push_message(partial_msg);
                        tracing::debug!("Saved partial multimodal response on error: {} chars", partial_content.len());
                    }
                    break;
                }
            }
        }

        // Handle tool calls if detected
        if tool_calls_detected {
            tracing::debug!("Tool calls detected in multimodal response, executing {} tools", tool_calls.len());

            let tool_calls_to_execute = tool_calls.clone();

            // Resolve cached data references in tool arguments
            let (large_cache, cache) = {
                let state = internal_state.read().await;
                (state.large_data_cache.clone(), state.tool_result_cache.clone())
            };

            // Execute tool calls with bounded concurrency (max 6 parallel)
            let tool_inputs: Vec<(String, serde_json::Value)> = tool_calls_to_execute
                .iter()
                .map(|tc| (tc.name.clone(), resolve_cached_arguments(&tc.arguments, &large_cache)))
                .collect();

            let tool_futures = futures::stream::iter(tool_inputs.into_iter().map(|(name, arguments)| {
                let tools_clone = tools.clone();
                let cache_clone = cache.clone();

                async move {
                    (name.clone(), ToolExecutionResult {
                        _name: name.clone(),
                        arguments: arguments.clone(),
                        result: execute_tool_with_retry(&tools_clone, &cache_clone, &name, arguments.clone()).await,
                    })
                }
            })).buffer_unordered(6);

            let tool_results_executed: Vec<_> = tool_futures.collect().await;

            // Process results
            let mut tool_calls_with_results: Vec<ToolCall> = Vec::new();
            let mut tool_call_results: Vec<(String, String)> = Vec::new();

            for (name, execution) in tool_results_executed {
                // Use arguments from the execution result (preserves per-call arguments for same-name tools)
                let exec_arguments = execution.arguments.clone();
                yield AgentEvent::tool_call_start(&name, exec_arguments.clone());

                match execution.result {
                    Ok(output) => {
                        let result_value = if output.success {
                            output.data.clone()
                        } else {
                            output.error.clone().map(|e| serde_json::json!({"error": e}))
                                .unwrap_or_else(|| serde_json::json!("Error"))
                        };
                        let result_str = if output.success {
                            serde_json::to_string(&output.data).unwrap_or_else(|_| "Success".to_string())
                        } else {
                            output.error.clone().unwrap_or_else(|| "Error".to_string())
                        };

                        // Sanitize base64/image data before sending to frontend or LLM
                        let display_str = sanitize_tool_result_for_prompt(&result_str);

                        tool_calls_with_results.push(ToolCall {
                            name: name.clone(),
                            id: String::new(),
                            arguments: exec_arguments,
                            result: Some(result_value.clone()),
                            round: Some(1),
                        });

                        yield AgentEvent::tool_call_end(&name, &display_str, output.success);
                        tool_call_results.push((name.clone(), display_str));
                    }
                    Err(e) => {
                        let error_msg = format!("Tool execution failed: {}", e);
                        let error_value = serde_json::json!({"error": error_msg});

                        tool_calls_with_results.push(ToolCall {
                            name: name.clone(),
                            id: String::new(),
                            arguments: exec_arguments,
                            result: Some(error_value.clone()),
                            round: Some(1),
                        });

                        yield AgentEvent::tool_call_end(&name, &error_msg, false);
                        tool_call_results.push((name.clone(), error_msg));
                    }
                }
            }

            // Save assistant message with tool calls
            let response_to_save = if content_before_tools.is_empty() {
                String::new()
            } else {
                content_before_tools.clone()
            };

            let initial_msg = AgentMessage::assistant_with_tools(&response_to_save, tool_calls_with_results.clone());
            internal_state.write().await.push_message(initial_msg);

            // Add tool result messages (large results go through cache → summary)
            for (tool_name, result_str) in &tool_call_results {
                if tool_name == "skill" {
                    llm_interface.set_skill_context(result_str.clone()).await;
                } else {
                    let mut state = internal_state.write().await;
                    let history_content = state.large_data_cache.store(tool_name, result_str);
                    let tool_result_msg = AgentMessage::tool_result(tool_name, &history_content);
                    state.push_message(tool_result_msg);
                }
            }

            // Summary fallback: ask LLM to summarize tool results (no tools, no thinking)
            // This avoids dumping raw tool JSON to the user
            let summary_history: Vec<neomind_core::Message> = {
                let state_guard = internal_state.read().await;
                let compacted = super::super::compact_tool_results(&state_guard.memory, 2);
                compacted.iter().map(|msg| msg.to_core()).collect()
            };

            let summary_prompt = "Based on the tool execution results in the conversation above, \
                provide a concise analysis and summary. Do NOT output any tool calls — \
                give a direct text response to the user's question.";

            let mut final_content = String::new();
            let summary_result = llm_interface.chat_stream_summary(
                summary_prompt,
                &summary_history,
            ).await;

            match summary_result {
                Ok(stream) => {
                    let mut pin = Box::pin(stream);
                    use futures::StreamExt;
                    while let Some(chunk) = pin.next().await {
                        match chunk {
                            Ok((text, _)) => {
                                final_content.push_str(&text);
                                yield AgentEvent::content(text);
                            }
                            Err(e) => {
                                tracing::error!("Multimodal summary stream error: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Multimodal summary call failed: {}", e);
                }
            }

            // Fallback to formatted tool results if summary is empty
            if final_content.trim().is_empty() {
                let deduped_results = deduplicate_tool_results(&tool_call_results);
                let formatted = format_tool_results(&deduped_results);
                final_content = formatted.clone();
                yield AgentEvent::content(formatted);
            }

            // Save the final content
            {
                let mut state = internal_state.write().await;
                if let Some(last_msg) = state.memory.last_mut() {
                    if last_msg.role == "assistant" && last_msg.tool_calls.is_some() {
                        last_msg.content = final_content.into();
                    } else {
                        let final_msg = AgentMessage::assistant(&final_content);
                        state.memory.push(final_msg);
                    }
                } else {
                    let final_msg = AgentMessage::assistant(&final_content);
                    state.memory.push(final_msg);
                }
            }

            tracing::debug!("Multimodal tool execution complete with summary");
        } else {
            // No tool calls - save response directly
            let raw_response = if buffer.is_empty() {
                String::new()
            } else {
                buffer.clone()
            };

            // Clean any embedded tool call JSON from response
            let response_to_save = remove_tool_calls_from_response(&raw_response);

            let initial_msg = AgentMessage::assistant(&response_to_save);
            internal_state.write().await.push_message(initial_msg);

            // Yield any remaining content
            if !buffer.is_empty() {
                yield AgentEvent::content(buffer.clone());
            }
        }

        let pt = llm_interface.take_last_prompt_tokens().await;
        match pt {
            Some(t) => yield AgentEvent::end_with_tokens(t),
            None => yield AgentEvent::end(),
        }
    }))
}
