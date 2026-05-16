use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "http://localhost:9375/api";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub struct ApiClient {
    base_url: String,
    client: Client,
}

impl ApiClient {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    pub fn with_base_url(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.to_string(),
            client,
        }
    }

    pub async fn get(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(body)
    }

    pub async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.post(&url).json(body).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = resp_body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn post_raw(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.post(&url).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = resp_body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn put(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.put(&url).json(body).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = resp_body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }

    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.delete(&url).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = resp_body["error"].as_str().unwrap_or("Unknown error");
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

        let resp = self.client.post(&url).multipart(form).send().await?;
        let status = resp.status();
        let resp_body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = resp_body["error"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        Ok(resp_body)
    }
}
