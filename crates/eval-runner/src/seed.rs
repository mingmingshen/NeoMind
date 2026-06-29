//! HTTP-driven seeding. All methods POST to the test server using the same
//! API surface the agent's shell tool uses. (Spec §4 — HTTP-driven section.)
use crate::test_server::TestServer;
use anyhow::{Context, Result};

impl TestServer {
    pub async fn seed_devices(&self, devices: &[serde_json::Value]) -> Result<()> {
        for d in devices {
            let url = format!("{}/devices", self.api_base());
            let resp = self.http_post(&url, d).await?;
            if !resp.status().is_success() {
                anyhow::bail!(
                    "seed device {:?} -> {}: {}",
                    d.get("id"),
                    resp.status(),
                    resp.text().await?
                );
            }
        }
        Ok(())
    }

    pub async fn seed_metrics(&self, metrics: &[serde_json::Value]) -> Result<()> {
        // WriteMetricRequest (handlers/devices/metrics.rs:175) expects field "metric".
        for m in metrics {
            let device_id = m
                .get("device_id")
                .and_then(|v| v.as_str())
                .context("metric.device_id")?;
            let url = format!("{}/devices/{}/metrics", self.api_base(), device_id);
            let body = serde_json::json!({
                "metric": m.get("metric").cloned().unwrap_or_default(),
                "value": m.get("value").cloned().unwrap_or_default(),
            });
            let resp = self.http_post(&url, &body).await?;
            if !resp.status().is_success() {
                anyhow::bail!("seed metric -> {}: {}", resp.status(), resp.text().await?);
            }
        }
        Ok(())
    }

    pub async fn seed_rules(&self, rules: &[serde_json::Value]) -> Result<()> {
        for r in rules {
            self.post_or_bail("/rules", r).await?
        }
        Ok(())
    }
    pub async fn seed_agents(&self, agents: &[serde_json::Value]) -> Result<()> {
        for a in agents {
            self.post_or_bail("/agents", a).await?
        }
        Ok(())
    }
    pub async fn seed_transforms(&self, ts: &[serde_json::Value]) -> Result<()> {
        for t in ts {
            self.post_or_bail("/automations", t).await?
        }
        Ok(())
    }
    pub async fn seed_dashboards(&self, ds: &[serde_json::Value]) -> Result<()> {
        for d in ds {
            self.post_or_bail("/dashboards", d).await?
        }
        Ok(())
    }
    pub async fn seed_channels(&self, chs: &[serde_json::Value]) -> Result<()> {
        for c in chs {
            self.post_or_bail("/messages/channels", c).await?
        }
        Ok(())
    }
    pub async fn seed_extensions_metadata(&self, exts: &[serde_json::Value]) -> Result<()> {
        // Tier 1 avoids real extension installs — case writers should not seed
        // extensions that require .nep binary upload (spec §10). The actual
        // install route is `/extensions/market/install` (verified router.rs:980).
        // This seeder is included for completeness; cases that need real
        // extensions must bundle a .nep binary (out of scope for Tier 1).
        for e in exts {
            self.post_or_bail("/extensions/market/install", e).await?
        }
        Ok(())
    }

    async fn post_or_bail(&self, path: &str, body: &serde_json::Value) -> Result<()> {
        let url = format!("{}{}", self.api_base(), path);
        let resp = self.http_post(&url, body).await?;
        if !resp.status().is_success() {
            anyhow::bail!("POST {} -> {}: {}", path, resp.status(), resp.text().await?);
        }
        Ok(())
    }

    pub async fn http_post(&self, url: &str, body: &serde_json::Value) -> Result<reqwest::Response> {
        Ok(reqwest::Client::new()
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key()))
            .json(body)
            .send()
            .await?)
    }

    /// HTTP GET helper — also used by state_query.rs.
    pub async fn http_get(&self, path: &str) -> Result<reqwest::Response> {
        Ok(reqwest::Client::new()
            .get(format!("{}{}", self.api_base(), path))
            .header("Authorization", format!("Bearer {}", self.api_key()))
            .send()
            .await?)
    }
}
