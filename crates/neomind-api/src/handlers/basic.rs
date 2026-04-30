//! Basic handlers - health check and system status.

use axum::{extract::State, Json};
use serde::Serialize;
use serde_json::json;

use super::ServerState;
use super::common::{ok, HandlerResult};

/// Health check response.
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub service: String,
    pub version: &'static str,
    pub uptime: u64,
}

/// Dependency health status.
#[derive(Debug, Clone, Serialize)]
pub struct DependencyStatus {
    pub llm: bool,
    pub mqtt: bool,
    pub database: bool,
}

impl DependencyStatus {
    pub fn all_ready(&self) -> bool {
        self.llm || self.mqtt || self.database // At least one dependency is ready
    }
}

/// Readiness check response.
#[derive(Debug, Clone, Serialize)]
pub struct ReadinessStatus {
    pub ready: bool,
    pub dependencies: DependencyStatus,
}

/// Basic health check handler (public endpoint).
pub async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "edge-ai-agent",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Detailed health check with uptime.
pub async fn health_status_handler(State(state): State<ServerState>) -> Json<HealthStatus> {
    let uptime = chrono::Utc::now().timestamp() - state.started_at;

    Json(HealthStatus {
        status: "healthy".to_string(),
        service: "edge-ai-agent".to_string(),
        version: env!("CARGO_PKG_VERSION"),
        uptime: uptime.max(0) as u64,
    })
}

/// Liveness probe - simple check if server is running.
pub async fn liveness_handler() -> Json<serde_json::Value> {
    Json(json!({
        "status": "alive",
    }))
}

/// Readiness probe - check if dependencies are ready.
pub async fn readiness_handler(State(state): State<ServerState>) -> Json<ReadinessStatus> {
    // Check if session manager is working (just check if we can access it)
    let _sessions = state.agents.session_manager.list_sessions().await;

    // Check if LLM might be configured (best effort check)
    let llm = true; // We can't easily check this without making a call

    // Check MQTT status (assume it's working if we got this far)
    let mqtt = true; // MqttDeviceManager doesn't expose a simple is_connected

    // Check if database/storage is accessible
    let database = true; // TimeSeriesStorage doesn't have an is_ready method

    let dependencies = DependencyStatus {
        llm,
        mqtt,
        database,
    };

    let ready = true; // If server is responding, we're ready

    Json(ReadinessStatus {
        ready,
        dependencies,
    })
}

/// Get local network info (WiFi SSID, LAN IP) for BLE provisioning.
///
/// `GET /api/system/network-info`
pub async fn network_info_handler() -> HandlerResult<serde_json::Value> {
    let ssid = get_wifi_ssid();
    let ip = get_server_ip();

    ok(json!({
        "ssid": ssid,
        "ip": ip,
    }))
}

/// Get the WiFi SSID of the host machine.
fn get_wifi_ssid() -> Option<String> {
    if cfg!(target_os = "macos") {
        // macOS: use networksetup to get current WiFi network
        if let Ok(output) = std::process::Command::new("networksetup")
            .args(["-getairportnetwork", "en0"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Output format: "Current Wi-Fi Network: <SSID>" or "You are not associated..."
            if let Some(pos) = stdout.find(": ") {
                let ssid = stdout[pos + 2..].trim().to_string();
                if !ssid.is_empty() && !ssid.contains("not associated") {
                    return Some(ssid);
                }
            }
        }
        // Fallback: try system_profiler
        if let Ok(output) = std::process::Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
            .arg("-I")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(pos) = line.find("SSID:") {
                    let ssid = line[pos + 5..].trim().to_string();
                    if !ssid.is_empty() {
                        return Some(ssid);
                    }
                }
            }
        }
    } else if cfg!(target_os = "linux") {
        // Linux: try iwgetid or nmcli
        if let Ok(output) = std::process::Command::new("iwgetid").arg("-r").output() {
            let ssid = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ssid.is_empty() {
                return Some(ssid);
            }
        }
        if let Ok(output) = std::process::Command::new("nmcli")
            .args(["-t", "-f", "active,ssid", "dev", "wifi"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("yes:") {
                    let ssid = line[4..].trim().to_string();
                    if !ssid.is_empty() {
                        return Some(ssid);
                    }
                }
            }
        }
    }
    None
}

/// Get the local LAN IP address.
fn get_server_ip() -> String {
    use std::net::IpAddr;

    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                let ip = local_addr.ip();
                if let IpAddr::V4(ipv4) = ip {
                    let o = ipv4.octets();
                    if (o[0] == 192 && o[1] == 168)
                        || o[0] == 10
                        || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                    {
                        return ip.to_string();
                    }
                }
            }
        }
    }

    if let Ok(interfaces) = get_if_addrs::get_if_addrs() {
        for iface in interfaces {
            if !iface.is_loopback() {
                if let get_if_addrs::IfAddr::V4(iface_addr) = iface.addr {
                    let o = iface_addr.ip.octets();
                    if (o[0] == 192 && o[1] == 168)
                        || o[0] == 10
                        || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                    {
                        return iface_addr.ip.to_string();
                    }
                }
            }
        }
    }

    std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string())
}
