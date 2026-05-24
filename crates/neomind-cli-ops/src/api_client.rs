use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "http://localhost:9375/api";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub struct ApiClient {
    base_url: String,
    client: Client,
    api_key: Option<String>,
}

impl ApiClient {
    pub fn new() -> Self {
        let base_url = std::env::var("NEOMIND_API_BASE")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Self::with_base_url(&base_url)
    }

    pub fn with_base_url(base_url: &str) -> Self {
        // 1. Try env var first
        let api_key = std::env::var("NEOMIND_API_KEY").ok()
            // 2. Fall back to auto-reading from local redb
            .or_else(crate::auto_auth::read_default_api_key);
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.to_string(),
            client,
            api_key,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn add_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.api_key {
            req.header("Authorization", format!("Bearer {}", key))
        } else {
            req
        }
    }

    pub async fn get(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.client.get(&url);
        let resp = self.add_auth(req).send().await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = extract_error_message(&body);
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(body)
    }

    pub async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.client.post(&url).json(body);
        let resp = self.add_auth(req).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = extract_error_message(&resp_body);
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn post_raw(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.client.post(&url);
        let resp = self.add_auth(req).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = extract_error_message(&resp_body);
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn put(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.client.put(&url).json(body);
        let resp = self.add_auth(req).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = extract_error_message(&resp_body);
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.client.delete(&url);
        let resp = self.add_auth(req).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = extract_error_message(&resp_body);
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn delete_with_body(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.client.delete(&url).json(body);
        let resp = self.add_auth(req).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = extract_error_message(&resp_body);
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn post_file(&self, path: &str, file_path: &str) -> Result<serde_json::Value> {
        use std::fs::File;
        use std::io::Read;
        use reqwest::multipart;

        let url = format!("{}{}", self.base_url, path);
        let mut file = File::open(file_path)?;
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        // Read file content
        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content)?;

        let part = multipart::Part::bytes(file_content)
            .file_name(file_name.to_string());
        let form = multipart::Form::new().part("file", part);

        let req = self.client.post(&url).multipart(form);
        let resp = self.add_auth(req).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = extract_error_message(&resp_body);
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }
}

/// Extract error message from API response body.
/// Handles multiple response formats:
/// - {"error":{"message":"..."}} (standard ErrorResponse wrapped)
/// - {"error":{"code":"...","message":"..."}} (standard ErrorResponse)
/// - {"message":"..."} (flat message)
/// - {"error":"..."} (legacy string error)
fn extract_error_message(body: &serde_json::Value) -> String {
    // Nested: body.error.message
    body.get("error")
        .and_then(|e| e.get("message").and_then(|v| v.as_str()))
        .or_else(|| body.get("message").and_then(|v| v.as_str()))
        .or_else(|| body.get("error").and_then(|v| v.as_str()))
        .unwrap_or("Unknown error")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_new() {
        let client = ApiClient::new();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_api_client_with_custom_base_url() {
        let custom_url = "http://example.com:8080/api";
        let client = ApiClient::with_base_url(custom_url);
        assert_eq!(client.base_url(), custom_url);
    }

    #[test]
    fn test_api_client_base_url_formatting() {
        let client = ApiClient::with_base_url("http://localhost:9000/v1");
        assert_eq!(client.base_url(), "http://localhost:9000/v1");
    }

    #[test]
    fn test_api_client_default_url_const() {
        assert_eq!(DEFAULT_BASE_URL, "http://localhost:9375/api");
    }

    #[test]
    fn test_api_client_timeout_const() {
        assert_eq!(DEFAULT_TIMEOUT_SECS, 30);
    }
}
