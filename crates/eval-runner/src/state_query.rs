//! 12 state-query types (spec §4a). All execute via HTTP GET on the test server.
use crate::test_server::TestServer;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct StateQueryInput {
    pub r#type: String,
    pub params: Value,
    pub expected: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateQueryResult {
    pub r#type: String,
    pub params: Value,
    pub expected: Value,
    pub actual: Value,
    pub passed: bool,
}

pub async fn run_query(q: &StateQueryInput, server: &TestServer) -> anyhow::Result<StateQueryResult> {
    let actual = match q.r#type.as_str() {
        "device_exists" => exists_at(server, &format!("/devices/{}", sid(&q.params, "id")?)).await?,
        "rule_exists" => exists_at(server, &format!("/rules/{}", sid(&q.params, "id")?)).await?,
        "agent_exists" => exists_at(server, &format!("/agents/{}", sid(&q.params, "id")?)).await?,
        "transform_exists" => {
            exists_at(server, &format!("/automations/{}", sid(&q.params, "id")?)).await?
        }
        "dashboard_exists" => {
            exists_at(server, &format!("/dashboards/{}", sid(&q.params, "id")?)).await?
        }
        "channel_exists" => {
            exists_at(server, &format!("/messages/channels/{}", sid(&q.params, "name")?)).await?
        }
        "rule_enabled" => {
            field_at(server, &format!("/rules/{}", sid(&q.params, "id")?), "enabled").await?
        }
        "agent_status" => {
            field_at(server, &format!("/agents/{}", sid(&q.params, "id")?), "status").await?
        }
        "push_enabled" => {
            field_at(server, &format!("/data-push/{}", sid(&q.params, "id")?), "enabled").await?
        }
        "device_count" => count_at(server, "/devices").await?,
        "dashboard_component_count" => {
            let v = get_json(server, &format!("/dashboards/{}", sid(&q.params, "id")?)).await?;
            Value::from(
                v.get("components")
                    .and_then(|c| c.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0),
            )
        }
        "message_count" => count_at(server, "/messages").await?,
        other => anyhow::bail!("unknown state_query type: {}", other),
    };
    let passed = actual == q.expected;
    Ok(StateQueryResult {
        r#type: q.r#type.clone(),
        params: q.params.clone(),
        expected: q.expected.clone(),
        actual,
        passed,
    })
}

pub fn sid(params: &Value, key: &str) -> anyhow::Result<String> {
    Ok(params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing param {}", key))?
        .to_string())
}

async fn exists_at(server: &TestServer, path: &str) -> anyhow::Result<Value> {
    let resp = server.http_get(path).await?;
    Ok(Value::Bool(resp.status().as_u16() == 200))
}

pub async fn get_json(server: &TestServer, path: &str) -> anyhow::Result<Value> {
    let resp = server.http_get(path).await?;
    let body: Value = resp.json().await?;
    // Many NeoMind routes return {success, data} envelope.
    Ok(body.get("data").cloned().unwrap_or(body))
}

async fn field_at(server: &TestServer, path: &str, field: &str) -> anyhow::Result<Value> {
    let v = get_json(server, path).await?;
    Ok(v.get(field).cloned().unwrap_or(Value::Null))
}

async fn count_at(server: &TestServer, path: &str) -> anyhow::Result<Value> {
    let v = get_json(server, path).await?;
    let n = match v {
        Value::Array(a) => a.len(),
        _ => 0,
    };
    Ok(Value::from(n))
}
