use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List all agents
pub async fn list_agents(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/agents").await?;
    Ok(CliResponse::success(data, "Agents listed"))
}

/// Get agent by ID
pub async fn get_agent(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/agents/{}", id)).await?;
    Ok(CliResponse::success(data, "Agent retrieved"))
}

/// Create a new agent
pub async fn create_agent(
    client: &ApiClient,
    name: &str,
    user_prompt: &str,
    description: Option<&str>,
    schedule_type: Option<&str>,
    schedule_config: Option<&str>,
    llm_backend: Option<&str>,
    system_prompt: Option<&str>,
) -> Result<CliResponse> {
    let schedule_type_val = schedule_type.unwrap_or("event");
    let mut schedule = json!({
        "schedule_type": schedule_type_val,
    });
    match schedule_type_val {
        "interval" => {
            if let Some(config) = schedule_config {
                if let Ok(secs) = config.parse::<u64>() {
                    schedule["interval_seconds"] = json!(secs);
                }
            }
        }
        "cron" => {
            if let Some(config) = schedule_config {
                schedule["cron_expression"] = json!(config);
            }
        }
        _ => {}
    }

    let mut body = json!({
        "name": name,
        "user_prompt": user_prompt,
        "schedule": schedule,
        "execution_mode": "free",
    });
    if let Some(desc) = description {
        body["description"] = json!(desc);
    }
    if let Some(backend) = llm_backend {
        body["llm_backend_id"] = json!(backend);
    }
    if let Some(prompt) = system_prompt {
        body["system_prompt"] = json!(prompt);
    }

    let data = client.post("/agents", &body).await?;
    let agent_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let meta = BuildMeta {
        r#type: "agent".to_string(),
        action: "create".to_string(),
        entity_id: agent_id.clone(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind agent delete {}", agent_id),
    };

    Ok(CliResponse::success_with_meta(data, "Agent created", meta))
}

/// Update agent
pub async fn update_agent(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    llm_backend: Option<&str>,
    system_prompt: Option<&str>,
    user_prompt: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(desc) = description {
        body["description"] = json!(desc);
    }
    if let Some(backend) = llm_backend {
        body["llm_backend_id"] = json!(backend);
    }
    if let Some(prompt) = system_prompt {
        body["system_prompt"] = json!(prompt);
    }
    if let Some(prompt) = user_prompt {
        body["user_prompt"] = json!(prompt);
    }

    let data = client.put(&format!("/agents/{}", id), &body).await?;
    Ok(CliResponse::success(data, "Agent updated"))
}

/// Delete agent
pub async fn delete_agent(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/agents/{}", id)).await?;
    Ok(CliResponse::success(
        json!({ "id": id }),
        "Agent deleted",
    ))
}

/// Control agent status (active/paused)
pub async fn control_agent(
    client: &ApiClient,
    id: &str,
    status: &str,
) -> Result<CliResponse> {
    let body = json!({ "status": status });
    let data = client.post(&format!("/agents/{}/status", id), &body).await?;
    Ok(CliResponse::success(data, "Agent status updated"))
}

/// Invoke agent with input
pub async fn invoke_agent(
    client: &ApiClient,
    id: &str,
    input: &str,
) -> Result<CliResponse> {
    let body = json!({ "input": input });
    let data = client.post(&format!("/agents/{}/execute", id), &body).await?;
    Ok(CliResponse::success(data, "Agent invoked"))
}

/// Get agent memory
pub async fn get_agent_memory(
    client: &ApiClient,
    id: &str,
) -> Result<CliResponse> {
    let data = client.get(&format!("/agents/{}/memory", id)).await?;
    Ok(CliResponse::success(data, "Agent memory retrieved"))
}

/// Get agent execution history
pub async fn get_agent_executions(
    client: &ApiClient,
    id: &str,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<CliResponse> {
    let mut path = format!("/agents/{}/executions", id);
    let mut params = Vec::new();
    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        params.push(format!("offset={}", o));
    }
    if !params.is_empty() {
        path.push('?');
        path.push_str(&params.join("&"));
    }

    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Agent executions retrieved"))
}

/// Get latest agent execution
pub async fn get_latest_execution(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let path = format!("/agents/{}/executions?limit=1", id);
    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Latest execution retrieved"))
}

/// Get agent conversation (user messages)
pub async fn get_conversation(client: &ApiClient, id: &str, limit: Option<usize>) -> Result<CliResponse> {
    let mut path = format!("/agents/{}/messages", id);
    if let Some(l) = limit {
        path.push_str(&format!("?limit={}", l));
    }
    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Agent conversation retrieved"))
}

/// Send message to agent
pub async fn send_message(
    client: &ApiClient,
    id: &str,
    message: &str,
    message_type: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({
        "content": message,
    });
    if let Some(mt) = message_type {
        body["type"] = json!(mt);
    }
    let data = client.post(&format!("/agents/{}/messages", id), &body).await?;
    Ok(CliResponse::success(data, "Message sent"))
}
