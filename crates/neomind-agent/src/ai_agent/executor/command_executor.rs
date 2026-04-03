use super::*;

impl AgentExecutor {
    async fn execute_single_command(
        &self,
        agent: &AiAgent,
        device_id: &str,
        command_name: &str,
        decision: &Decision,
    ) -> Option<neomind_storage::ActionExecuted> {
        let device_service = self.device_service.as_ref()?;

        // Find the command resource to get parameters
        let command_resource_id = format!("{}:{}", device_id, command_name);
        let resource = agent.resources.iter().find(|r| {
            r.resource_type == ResourceType::Command && r.resource_id == command_resource_id
        });

        let parameters = if let Some(res) = resource {
            res.config
                .get("parameters")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default()
        } else {
            serde_json::Map::new()
        };

        // Clone parameters for DeviceService (will be consumed)
        let params_map: std::collections::HashMap<String, serde_json::Value> = parameters
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        tracing::info!(
            agent_id = %agent.id,
            device_id = %device_id,
            command = %command_name,
            decision_action = %decision.action,
            "Executing command from LLM decision"
        );

        // Execute the command via DeviceService
        let execution_result = device_service
            .send_command(device_id, command_name, params_map)
            .await;

        let (success, result) = match execution_result {
            Ok(_) => (true, Some("Command sent successfully".to_string())),
            Err(e) => {
                tracing::warn!(
                    agent_id = %agent.id,
                    device_id = %device_id,
                    command = %command_name,
                    error = %e,
                    "Failed to send command"
                );
                (false, Some(format!("Failed: {}", e)))
            }
        };

        Some(neomind_storage::ActionExecuted {
            action_type: "device_command".to_string(),
            description: format!(
                "Execute {} on {} (reason: {})",
                command_name, device_id, decision.rationale
            ),
            target: device_id.to_string(),
            parameters: serde_json::to_value(&parameters).unwrap_or_default(),
            success,
            result,
        })
    }


    async fn execute_extension_command_for_agent(
        &self,
        agent: &AiAgent,
        extension_id: &str,
        command_name: &str,
        decision: &Decision,
    ) -> Option<neomind_storage::ActionExecuted> {
        let extension_registry = self.extension_registry.as_ref()?;

        tracing::info!(
            agent_id = %agent.id,
            extension_id = %extension_id,
            command = %command_name,
            decision_action = %decision.action,
            "Executing extension command from LLM decision"
        );

        // Build parameters from resource config or decision
        let command_args = decision.rationale.clone();
        let args_value = if command_args.is_empty() {
            serde_json::json!({})
        } else {
            // Try to parse as JSON, otherwise wrap as string
            serde_json::from_str(&command_args)
                .unwrap_or_else(|_| serde_json::json!({ "reason": command_args }))
        };

        // Execute the extension command
        let execution_result = extension_registry
            .execute_command(extension_id, command_name, &args_value)
            .await;

        let (success, result) = match execution_result {
            Ok(resp) => (true, Some(format!("Success: {}", resp))),
            Err(e) => {
                tracing::warn!(
                    agent_id = %agent.id,
                    extension_id = %extension_id,
                    command = %command_name,
                    error = %e,
                    "Failed to execute extension command"
                );
                (false, Some(format!("Failed: {}", e)))
            }
        };

        Some(neomind_storage::ActionExecuted {
            action_type: "extension_command".to_string(),
            description: format!(
                "Execute {} on extension {} (reason: {})",
                command_name, extension_id, decision.rationale
            ),
            target: extension_id.to_string(),
            parameters: args_value,
            success,
            result,
        })
    }

    /// Parse command from decision.action field.
    ///
    /// Expected formats:
    /// - "device_id:command_name" -> device command
    /// - "extension:ext_id:command_name" -> extension command
    ///

    fn parse_command_from_action(action: &str) -> Option<(String, String, String)> {
        let action = action.trim();

        // Try to parse as "prefix:id:command_name"
        if let Some(colon_pos) = action.find(':') {
            let prefix = &action[..colon_pos];
            let rest = &action[colon_pos + 1..];

            // Check if it's "extension:ext_id:command_name"
            if prefix == "extension" || prefix == "ext" {
                if let Some(second_colon) = rest.find(':') {
                    let ext_id = &rest[..second_colon];
                    let command_name = &rest[second_colon + 1..];
                    if !ext_id.is_empty() && !command_name.is_empty() {
                        return Some((
                            "extension".to_string(),
                            ext_id.trim().to_string(),
                            command_name.trim().to_string(),
                        ));
                    }
                }
            }

            // Otherwise treat as "device_id:command_name"
            if !prefix.is_empty() && !rest.is_empty() {
                return Some((
                    "device".to_string(),
                    prefix.trim().to_string(),
                    rest.trim().to_string(),
                ));
            }
        }

        // Try to parse as "device:command" (common format)
        if action.contains("device:") || action.contains("设备:") {
            // Extract device:command pattern using regex-like parsing
            let parts: Vec<&str> = action.split([':', '：']).map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                // Format: "device:xxx:command" or similar
                let cmd_keyword_idx = parts
                    .iter()
                    .position(|&p| p == "device" || p == "设备" || p == "command" || p == "指令");
                if let Some(idx) = cmd_keyword_idx {
                    if idx + 1 < parts.len() {
                        return Some((
                            "device".to_string(),
                            parts[idx + 1].to_string(),
                            parts[idx + 2].to_string(),
                        ));
                    }
                }
            }
        }

        None
    }


    async fn execute_decisions(
        &self,
        agent: &AiAgent,
        decisions: &[Decision],
    ) -> AgentResult<(
        Vec<neomind_storage::ActionExecuted>,
        Vec<neomind_storage::NotificationSent>,
    )> {
        let mut actions_executed = Vec::new();
        let mut notifications_sent = Vec::new();

        for decision in decisions {
            // === Handle query decisions - Agent requesting specific time range data ===
            // Format: query:device_id:metric:time_range (e.g., query:sensor1:temperature:24h)
            if decision.action.starts_with("query:") {
                let parts: Vec<&str> = decision.action.split(':').collect();
                if parts.len() >= 4 {
                    let _device_id = parts[1];
                    let _metric = parts[2];
                    let time_spec = parts[3];

                    // Parse time specification and log the request
                    // Note: This is a informational action - the actual time range
                    // adjustment needs to happen in the data collection phase
                    tracing::info!(
                        agent_id = %agent.id,
                        time_spec = %time_spec,
                        decision_action = %decision.action,
                        "Agent requested data with specific time range"
                    );

                    // Record as a "query" action for tracking
                    actions_executed.push(neomind_storage::ActionExecuted {
                        action_type: "data_query".to_string(),
                        description: format!("Query data with time range: {}", time_spec),
                        target: format!("{}:{}", parts[1], parts[2]),
                        parameters: serde_json::json!({"time_spec": time_spec}),
                        success: true,
                        result: Some(format!("Time range request noted: {}", time_spec)),
                    });
                }
            }

            // === NEW: Handle LLM-driven command decisions ===
            // When LLM returns decision_type == "command", parse the action field
            // and execute only that specific command
            if decision.decision_type == "command" {
                if let Some((cmd_type, target_id, command_name)) =
                    Self::parse_command_from_action(&decision.action)
                {
                    tracing::info!(
                        agent_id = %agent.id,
                        cmd_type = %cmd_type,
                        target_id = %target_id,
                        command_name = %command_name,
                        decision_action = %decision.action,
                        "Executing LLM-specified command"
                    );

                    match cmd_type.as_str() {
                        "extension" => {
                            // Execute extension command
                            if let Some(action_executed) = self
                                .execute_extension_command_for_agent(
                                    agent,
                                    &target_id,
                                    &command_name,
                                    decision,
                                )
                                .await
                            {
                                actions_executed.push(action_executed);
                            }
                        }
                        "device" => {
                            // Execute device command
                            if let Some(action_executed) = self
                                .execute_single_command(agent, &target_id, &command_name, decision)
                                .await
                            {
                                actions_executed.push(action_executed);
                            }
                        }
                        _ => {
                            tracing::warn!(
                                agent_id = %agent.id,
                                cmd_type = %cmd_type,
                                "Unknown command type"
                            );
                        }
                    }
                } else {
                    // Fallback: try to find matching command in resources
                    if let Some(cmd_name) = extract_command_from_description(&decision.description)
                    {
                        // Find a matching command in resources (both device and extension)
                        for resource in &agent.resources {
                            let is_device_cmd = resource.resource_type == ResourceType::Command
                                && resource.resource_id.ends_with(&format!(":{}", cmd_name));
                            let is_ext_cmd = resource.resource_type == ResourceType::ExtensionTool
                                && resource.resource_id.ends_with(&format!(":{}", cmd_name));

                            if is_device_cmd || is_ext_cmd {
                                let parts: Vec<&str> = resource.resource_id.split(':').collect();

                                match resource.resource_type {
                                    ResourceType::Command => {
                                        if parts.len() == 2 {
                                            if let Some(action_executed) = self
                                                .execute_single_command(
                                                    agent, parts[0], parts[1], decision,
                                                )
                                                .await
                                            {
                                                actions_executed.push(action_executed);
                                            }
                                            break;
                                        }
                                    }
                                    ResourceType::ExtensionTool => {
                                        if parts.len() >= 3 && parts[0] == "extension" {
                                            if let Some(action_executed) = self
                                                .execute_extension_command_for_agent(
                                                    agent, parts[1], parts[2], decision,
                                                )
                                                .await
                                            {
                                                actions_executed.push(action_executed);
                                            }
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    } else {
                        tracing::warn!(
                            agent_id = %agent.id,
                            decision_action = %decision.action,
                            decision_description = %decision.description,
                            "Could not parse command from decision"
                        );
                    }
                }
                // Continue to next decision after handling command type
                continue;
            }

            // === Handle alert-type decisions ===
            let is_alert_decision = decision.decision_type.to_lowercase().contains("alert")
                || decision.action.to_lowercase().contains("alert")
                || decision.action.to_lowercase().contains("报警")
                || decision.action.to_lowercase().contains("notify")
                || decision.action.to_lowercase().contains("通知");

            if is_alert_decision {
                tracing::info!(
                    agent_id = %agent.id,
                    decision_type = %decision.decision_type,
                    decision_action = %decision.action,
                    "Alert-type decision detected, sending notification"
                );
                self.send_alert_for_decision(agent, decision, &mut notifications_sent)
                    .await;
            }

            // === LEGACY: Handle condition_met decisions ===
            // This executes ALL commands (old behavior, kept for backward compatibility)
            if decision.decision_type == "condition_met" {
                // Execute device commands
                if let Some(ref device_service) = self.device_service {
                    for resource in &agent.resources {
                        if resource.resource_type == ResourceType::Command {
                            // Parse device_id and command from resource_id
                            // Format: "device_id:command_name"
                            let parts: Vec<&str> = resource.resource_id.split(':').collect();
                            if parts.len() == 2 {
                                let device_id = parts[0];
                                let command_name = parts[1];

                                // Get parameters from resource config
                                let parameters = resource
                                    .config
                                    .get("parameters")
                                    .and_then(|v| v.as_object())
                                    .cloned()
                                    .unwrap_or_default();

                                // Convert parameters to HashMap for DeviceService
                                let params_map: std::collections::HashMap<
                                    String,
                                    serde_json::Value,
                                > = parameters.into_iter().collect();

                                // Actually execute the command via DeviceService
                                let execution_result = device_service
                                    .send_command(device_id, command_name, params_map)
                                    .await;

                                let (success, result) = match execution_result {
                                    Ok(_) => (true, Some("Command sent successfully".to_string())),
                                    Err(e) => {
                                        tracing::warn!(
                                            agent_id = %agent.id,
                                            device_id = %device_id,
                                            command = %command_name,
                                            error = %e,
                                            "Failed to send command"
                                        );
                                        (false, Some(format!("Failed: {}", e)))
                                    }
                                };

                                // Re-create parameters for ActionExecuted record
                                let parameters_for_record = resource
                                    .config
                                    .get("parameters")
                                    .and_then(|v| v.as_object())
                                    .cloned()
                                    .unwrap_or_default();

                                actions_executed.push(neomind_storage::ActionExecuted {
                                    action_type: "device_command".to_string(),
                                    description: format!(
                                        "Execute {} on {}",
                                        command_name, device_id
                                    ),
                                    target: device_id.to_string(),
                                    parameters: serde_json::to_value(parameters_for_record)
                                        .unwrap_or_default(),
                                    success,
                                    result,
                                });
                            }
                        }
                    }
                }

                // Send notifications for alert actions
                let should_send_alert = agent
                    .parsed_intent
                    .as_ref()
                    .map(|i| {
                        i.actions.iter().any(|a| {
                            a.contains("alert")
                                || a.contains("notification")
                                || a.contains("报警")
                                || a.contains("通知")
                        })
                    })
                    .unwrap_or(false);

                // Debug log for notification trigger
                tracing::debug!(
                    agent_id = %agent.id,
                    should_send_alert,
                    has_parsed_intent = agent.parsed_intent.is_some(),
                    actions = ?agent.parsed_intent.as_ref().map(|i| &i.actions),
                    has_message_manager = self.message_manager.is_some(),
                    "Checking if alert should be sent"
                );

                if should_send_alert {
                    self.send_alert_for_decision(agent, decision, &mut notifications_sent)
                        .await;
                }
            }

            // Execute specific actions based on decision.action
            if decision.action.to_lowercase().contains("execute_command")
                || decision.action.to_lowercase().contains("command")
                || decision.action.to_lowercase().contains("执行指令")
                || decision.action.to_lowercase().contains("控制")
            {
                // Execute commands defined in agent resources
                if let Some(ref device_service) = self.device_service {
                    // Check if decision.description specifies which commands to execute
                    // Format: "execute command: turn_on_light" or "执行指令: open_valve"
                    let mentioned_command = extract_command_from_description(&decision.description);
                    let mentioned_device = extract_device_from_description(&decision.description);

                    let commands_to_execute: Vec<_> = agent
                        .resources
                        .iter()
                        .filter(|r| r.resource_type == ResourceType::Command)
                        .filter(|r| {
                            // Filter by mentioned command if specified
                            if let Some(ref cmd_name) = mentioned_command {
                                r.resource_id.ends_with(&format!(":{}", cmd_name))
                                    || r.resource_id.contains(cmd_name)
                            } else if let Some(ref dev_id) = mentioned_device {
                                r.resource_id.starts_with(&format!("{}:", dev_id))
                            } else {
                                true // No filter, include all commands (safe default)
                            }
                        })
                        .collect();

                    if commands_to_execute.is_empty() {
                        tracing::warn!(
                            agent_id = %agent.id,
                            decision_description = %decision.description,
                            "No matching commands found for execution"
                        );
                    } else {
                        tracing::info!(
                            agent_id = %agent.id,
                            command_count = commands_to_execute.len(),
                            "Executing {} command(s) from decision",
                            commands_to_execute.len()
                        );
                    }

                    for resource in commands_to_execute {
                        // Parse device_id and command from resource_id
                        // Format: "device_id:command_name"
                        let parts: Vec<&str> = resource.resource_id.split(':').collect();
                        if parts.len() == 2 {
                            let device_id = parts[0];
                            let command_name = parts[1];

                            // Get parameters from resource config
                            let parameters = resource
                                .config
                                .get("parameters")
                                .and_then(|v| v.as_object())
                                .cloned()
                                .unwrap_or_default();

                            // Convert parameters to HashMap for DeviceService
                            let params_map: std::collections::HashMap<String, serde_json::Value> =
                                parameters.into_iter().collect();

                            tracing::info!(
                                agent_id = %agent.id,
                                device_id = %device_id,
                                command = %command_name,
                                "Executing command from decision action"
                            );

                            // Actually execute the command via DeviceService
                            let execution_result = device_service
                                .send_command(device_id, command_name, params_map)
                                .await;

                            let (success, result) = match execution_result {
                                Ok(_) => (true, Some("Command sent successfully".to_string())),
                                Err(e) => {
                                    tracing::warn!(
                                        agent_id = %agent.id,
                                        device_id = %device_id,
                                        command = %command_name,
                                        error = %e,
                                        "Failed to send command"
                                    );
                                    (false, Some(format!("Failed: {}", e)))
                                }
                            };

                            // Re-create parameters for ActionExecuted record
                            let parameters_for_record = resource
                                .config
                                .get("parameters")
                                .and_then(|v| v.as_object())
                                .cloned()
                                .unwrap_or_default();

                            actions_executed.push(neomind_storage::ActionExecuted {
                                action_type: "device_command".to_string(),
                                description: format!(
                                    "Execute {} on {} (triggered by decision: {})",
                                    command_name, device_id, decision.action
                                ),
                                target: device_id.to_string(),
                                parameters: serde_json::to_value(parameters_for_record)
                                    .unwrap_or_default(),
                                success,
                                result,
                            });
                        }
                    }
                }
            }
        }

        Ok((actions_executed, notifications_sent))
    }


    async fn send_alert_for_decision(
        &self,
        agent: &AiAgent,
        decision: &neomind_storage::Decision,
        notifications_sent: &mut Vec<neomind_storage::NotificationSent>,
    ) {
        let alert_message = format!(
            "Agent '{}' - {}: {}",
            agent.name, decision.decision_type, decision.description
        );

        // Send via MessageManager if available
        if let Some(ref message_manager) = self.message_manager {
            use neomind_messages::{Message, MessageSeverity};

            // Determine if this is an alert or notification based on decision type
            let is_alert = decision.decision_type.to_lowercase().contains("alert")
                || decision.decision_type.to_lowercase().contains("报警")
                || decision.decision_type.to_lowercase().contains("critical")
                || decision.decision_type.to_lowercase().contains("emergency")
                || decision.decision_type.to_lowercase().contains("紧急")
                || decision.decision_type.to_lowercase().contains("warning")
                || decision.decision_type.to_lowercase().contains("警告")
                || decision.decision_type.to_lowercase().contains("error")
                || decision.decision_type.to_lowercase().contains("异常")
                || decision.decision_type.to_lowercase().contains("故障");

            // Determine severity based on decision type
            let severity = if decision.decision_type.to_lowercase().contains("critical")
                || decision.decision_type.to_lowercase().contains("emergency")
                || decision.decision_type.to_lowercase().contains("紧急")
            {
                MessageSeverity::Critical
            } else if decision.decision_type.to_lowercase().contains("warning")
                || decision.decision_type.to_lowercase().contains("警告")
                || decision.decision_type.to_lowercase().contains("error")
            {
                MessageSeverity::Warning
            } else {
                MessageSeverity::Info
            };

            // Create message with appropriate category
            let (category, title_prefix) = if is_alert {
                ("alert", "Agent Alert")
            } else {
                ("notification", "Agent Notification")
            };

            let mut msg = Message::new(
                category,
                severity,
                format!("{}: {}", title_prefix, agent.name),
                alert_message.clone(),
                agent.id.clone(),
            );
            msg.source_type = "agent".to_string();

            tracing::info!(
                agent_id = %agent.id,
                category = %category,
                alert_message = %alert_message,
                severity = ?severity,
                "Sending message via MessageManager"
            );

            match message_manager.create_message(msg).await {
                Ok(msg) => {
                    notifications_sent.push(neomind_storage::NotificationSent {
                        channel: "message_manager".to_string(),
                        recipient: "configured_channels".to_string(),
                        message: alert_message,
                        sent_at: chrono::Utc::now().timestamp(),
                        success: true,
                    });
                    tracing::info!(
                        agent_id = %agent.id,
                        message_id = %msg.id.to_string(),
                        "Message sent via MessageManager successfully"
                    );
                }
                Err(e) => {
                    notifications_sent.push(neomind_storage::NotificationSent {
                        channel: "message_manager".to_string(),
                        recipient: "configured_channels".to_string(),
                        message: alert_message.clone(),
                        sent_at: chrono::Utc::now().timestamp(),
                        success: false,
                    });
                    tracing::warn!(
                        agent_id = %agent.id,
                        error = %e,
                        "Failed to send message via MessageManager"
                    );
                }
            }
        } else {
            // Fallback: Publish event to EventBus if MessageManager not available
            tracing::warn!(
                agent_id = %agent.id,
                "MessageManager not available, using EventBus fallback"
            );
            if let Some(ref bus) = self.event_bus {
                let _ = bus
                    .publish(NeoMindEvent::MessageCreated {
                        message_id: uuid::Uuid::new_v4().to_string(),
                        title: format!("Agent Alert: {}", agent.name),
                        severity: "info".to_string(),
                        message: decision.description.clone(),
                        timestamp: chrono::Utc::now().timestamp(),
                    })
                    .await;

                notifications_sent.push(neomind_storage::NotificationSent {
                    channel: "event_bus".to_string(),
                    recipient: "event_subscribers".to_string(),
                    message: alert_message,
                    sent_at: chrono::Utc::now().timestamp(),
                    success: true,
                });
            }
        }
    }


    async fn maybe_generate_report(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
    ) -> AgentResult<Option<GeneratedReport>> {
        // Only generate reports for report generation agents
        if let Some(ref intent) = agent.parsed_intent {
            if matches!(
                intent.intent_type,
                neomind_storage::IntentType::ReportGeneration
            ) {
                let content = self.generate_report_content(agent, data).await?;

                return Ok(Some(GeneratedReport {
                    report_type: "summary".to_string(),
                    content,
                    data_summary: data
                        .iter()
                        .map(|d| neomind_storage::DataSummary {
                            source: d.source.clone(),
                            metric: d.data_type.clone(),
                            count: 1,
                            statistics: d.values.clone(),
                        })
                        .collect(),
                    generated_at: chrono::Utc::now().timestamp(),
                }));
            }
        }

        Ok(None)
    }


    async fn generate_report_content(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
    ) -> AgentResult<String> {
        let mut report = format!("# {} - Report\n\n", agent.name);
        report.push_str(&format!(
            "Generated: {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));

        report.push_str("## Data Summary\n\n");
        for data_item in data {
            report.push_str(&format!(
                "- **{}**: {}\n",
                data_item.source, data_item.values
            ));
        }

        report.push_str("\n## Analysis Results\n\n");
        if let Some(ref intent) = agent.parsed_intent {
            report.push_str(&format!("Intent Type: {:?}\n", intent.intent_type));
            report.push_str(&format!("Target Metrics: {:?}\n", intent.target_metrics));
        }

        report.push_str("\n## Conclusion\n\n");
        report.push_str(&agent.user_prompt);

        Ok(report)
    }

    /// Update agent memory with learnings from this execution.
    /// Uses hierarchical memory architecture (MemGPT/Letta style):
    /// - Working Memory: Current execution (cleared after each execution)
    /// - Short-Term Memory: Recent summaries (auto-archived when full)

}
