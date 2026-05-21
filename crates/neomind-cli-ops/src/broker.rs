use anyhow::Result;
use serde_json::json;
use crate::types::CliResponse;
use crate::ApiClient;

/// List all external MQTT brokers
pub async fn list_brokers(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/brokers").await?;
    Ok(CliResponse::success(data, "Brokers listed"))
}

/// Get broker by ID
pub async fn get_broker(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/brokers/{}", id)).await?;
    Ok(CliResponse::success(data, "Broker retrieved"))
}

/// Create a new external MQTT broker
pub async fn create_broker(
    client: &ApiClient,
    name: &str,
    host: &str,
    port: u16,
    tls: bool,
    username: Option<&str>,
    password: Option<&str>,
    subscribe_topics: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({
        "name": name,
        "broker": host,
        "port": port,
        "tls": tls,
    });
    if let Some(u) = username {
        body["username"] = json!(u);
    }
    if let Some(p) = password {
        body["password"] = json!(p);
    }
    if let Some(topics) = subscribe_topics {
        // Accept comma-separated topics
        let topic_list: Vec<&str> = topics.split(',').map(|t| t.trim()).filter(|t| !t.is_empty()).collect();
        body["subscribe_topics"] = json!(topic_list);
    }

    let data = client.post("/brokers", &body).await?;
    let connected = data.get("broker")
        .and_then(|b| b.get("connected"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let msg = if connected {
        "Broker created and connected"
    } else {
        "Broker created (connection pending or failed)"
    };

    Ok(CliResponse::success(data, msg))
}

/// Update an existing broker
pub async fn update_broker(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    host: Option<&str>,
    port: Option<u16>,
    tls: Option<bool>,
    username: Option<&str>,
    password: Option<&str>,
    subscribe_topics: Option<&str>,
    enabled: Option<bool>,
) -> Result<CliResponse> {
    // First get existing broker to preserve required fields
    let existing = client.get(&format!("/brokers/{}", id)).await?;
    let broker_data = existing.get("broker").cloned().unwrap_or(json!({}));

    let mut body = json!({
        "name": name.unwrap_or_else(|| broker_data.get("name").and_then(|v| v.as_str()).unwrap_or("")),
        "broker": host.unwrap_or_else(|| broker_data.get("broker").and_then(|v| v.as_str()).unwrap_or("")),
        "port": port.unwrap_or_else(|| broker_data.get("port").and_then(|v| v.as_u64()).unwrap_or(1883) as u16),
        "tls": tls.unwrap_or_else(|| broker_data.get("tls").and_then(|v| v.as_bool()).unwrap_or(false)),
        "enabled": enabled.unwrap_or_else(|| broker_data.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true)),
    });

    if let Some(u) = username {
        body["username"] = json!(u);
    }
    if let Some(p) = password {
        body["password"] = json!(p);
    }
    if let Some(topics) = subscribe_topics {
        let topic_list: Vec<&str> = topics.split(',').map(|t| t.trim()).filter(|t| !t.is_empty()).collect();
        body["subscribe_topics"] = json!(topic_list);
    }

    let data = client.put(&format!("/brokers/{}", id), &body).await?;
    Ok(CliResponse::success(data, "Broker updated"))
}

/// Delete a broker
pub async fn delete_broker(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/brokers/{}", id)).await?;
    Ok(CliResponse::success(json!({ "id": id }), "Broker deleted"))
}

/// Test broker connection
pub async fn test_broker(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.post(&format!("/brokers/{}/test", id), &json!({})).await?;
    let success = data.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let msg = if success { "Broker connection successful" } else { "Broker connection failed" };
    Ok(CliResponse::success(data, msg))
}

/// List MQTT subscriptions
pub async fn list_subscriptions(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/mqtt/subscriptions").await?;
    Ok(CliResponse::success(data, "Subscriptions listed"))
}

/// Subscribe to an MQTT topic
pub async fn subscribe_topic(client: &ApiClient, topic: &str, qos: Option<u8>) -> Result<CliResponse> {
    let body = json!({
        "topic": topic,
        "qos": qos.unwrap_or(1),
    });
    let data = client.post("/mqtt/subscribe", &body).await?;
    Ok(CliResponse::success(data, "Subscribed"))
}

/// Unsubscribe from an MQTT topic
pub async fn unsubscribe_topic(client: &ApiClient, topic: &str) -> Result<CliResponse> {
    let body = json!({
        "topic": topic,
    });
    let data = client.post("/mqtt/unsubscribe", &body).await?;
    Ok(CliResponse::success(data, "Unsubscribed"))
}
