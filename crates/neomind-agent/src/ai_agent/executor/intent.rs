use super::*;

impl AgentExecutor {
    pub async fn parse_intent(
        &self,
        user_prompt: &str,
    ) -> AgentResult<neomind_storage::ParsedIntent> {
        // Try LLM-based parsing if available
        if let Some(ref llm) = self.llm_runtime {
            if let Ok(intent) = self.parse_intent_with_llm(llm, user_prompt).await {
                return Ok(intent);
            }
        }

        // Fall back to keyword-based parsing
        self.parse_intent_keywords(user_prompt).await
    }


    async fn parse_intent_with_llm(
        &self,
        llm: &Arc<dyn neomind_core::llm::backend::LlmRuntime + Send + Sync>,
        user_prompt: &str,
    ) -> AgentResult<neomind_storage::ParsedIntent> {
        use neomind_core::llm::backend::{GenerationParams, LlmInput};

        // Get current time context for temporal understanding
        let time_context = get_time_context();

        let system_prompt = format!(
            r#"You are an intent parser for IoT automation. Analyze the user's request and extract:
1. Intent type: Monitoring, ReportGeneration, AnomalyDetection, Control, or Automation
2. Target metrics: temperature, humidity, power, etc.
3. Conditions: any thresholds or comparison operators
4. Actions: what actions to take when conditions are met

{}

Respond in JSON format:
{{
  "intent_type": "Monitoring|ReportGeneration|AnomalyDetection|Control|Automation",
  "target_metrics": ["metric1", "metric2"],
  "conditions": ["condition1", "condition2"],
  "actions": ["action1", "action2"],
  "confidence": 0.9
}}"#,
            time_context
        );

        let messages = vec![
            Message::new(MessageRole::System, Content::text(system_prompt)),
            Message::new(MessageRole::User, Content::text(user_prompt)),
        ];

        let input = LlmInput {
            messages,
            params: GenerationParams {
                temperature: Some(0.3),
                max_tokens: Some(500),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        // Add timeout for LLM generation (5 minutes max)
        const LLM_TIMEOUT_SECS: u64 = 300;
        let llm_result = match tokio::time::timeout(
            std::time::Duration::from_secs(LLM_TIMEOUT_SECS),
            llm.generate(input),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!("LLM intent parsing timed out after {}s", LLM_TIMEOUT_SECS);
                return Err(NeoMindError::Llm(format!(
                    "LLM timeout after {}s",
                    LLM_TIMEOUT_SECS
                )));
            }
        };

        match llm_result {
            Ok(output) => {
                // Try to parse JSON from LLM output
                let json_str = output.text.trim();
                // Extract JSON if it's wrapped in markdown code blocks
                let json_str = if json_str.contains("```json") {
                    json_str
                        .split("```json")
                        .nth(1)
                        .and_then(|s| s.split("```").next())
                        .unwrap_or(json_str)
                        .trim()
                } else if json_str.contains("```") {
                    json_str.split("```").nth(1).unwrap_or(json_str).trim()
                } else {
                    json_str
                };

                serde_json::from_str(json_str).map_err(|_| {
                    NeoMindError::Llm("Failed to parse LLM intent response".to_string())
                })
            }
            Err(_) => Err(NeoMindError::Llm("LLM call failed".to_string())),
        }
    }


    async fn parse_intent_keywords(
        &self,
        user_prompt: &str,
    ) -> AgentResult<neomind_storage::ParsedIntent> {
        let prompt_lower = user_prompt.to_lowercase();

        let (intent_type, confidence) = if prompt_lower.contains("报告")
            || prompt_lower.contains("汇总")
            || prompt_lower.contains("每天")
        {
            (neomind_storage::IntentType::ReportGeneration, 0.8)
        } else if prompt_lower.contains("异常") || prompt_lower.contains("检测") {
            (neomind_storage::IntentType::AnomalyDetection, 0.8)
        } else if prompt_lower.contains("控制") || prompt_lower.contains("开关") {
            (neomind_storage::IntentType::Control, 0.7)
        } else {
            (neomind_storage::IntentType::Monitoring, 0.7)
        };

        let target_metrics = extract_metrics(&prompt_lower);
        let conditions = extract_conditions(&prompt_lower);
        let actions = extract_actions(&prompt_lower);

        Ok(neomind_storage::ParsedIntent {
            intent_type,
            target_metrics,
            conditions,
            actions,
            confidence,
        })
    }


}
