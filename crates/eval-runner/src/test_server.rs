//! TestServer: spawn `neomind serve` with temp data dir + random port.
//! Per-case isolation (spec §4 隔离边界).
use anyhow::{Context, Result};
use std::process::Stdio;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::Duration;

pub struct TestServer {
    pub child: tokio::process::Child,
    pub _data_dir: TempDir, // dropped on shutdown → rm -rf
    pub api_base: String,
    pub api_key: String,
}

impl TestServer {
    /// Spawn neomind serve with --port 0 + temp data dir.
    ///
    /// CRITICAL (verified 2026-06-29):
    /// 1. CLI's `serve` subcommand uses clap `--port`/`--host` args directly.
    ///    Use CLI args (not env vars) to set the port reliably.
    /// 2. `AuthState::new()` hardcodes `db_path = "data/api_keys.redb"` and does
    ///    NOT respect `NEOMIND_DATA_DIR`. We set CWD = tempdir so the hardcoded
    ///    relative `data/` lands inside the tempdir.
    /// 3. `StartupLogger::ready_info` uses `println!` → stdout. Pipe BOTH streams.
    pub async fn spawn() -> Result<Self> {
        let data_dir = tempfile::tempdir().context("mktemp data dir")?;
        let data_dir_path = data_dir.path().to_string_lossy().to_string();

        let mut cmd = Command::new("neomind");
        cmd.arg("serve")
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg("0")
            .env("NEOMIND_DATA_DIR", &data_dir_path)
            .current_dir(&data_dir_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().context("spawn neomind serve")?;

        let stdout = child.stdout.take().context("no stdout")?;
        let stderr = child.stderr.take().context("no stderr")?;
        let mut out_lines = BufReader::new(stdout).lines();
        let mut err_lines = BufReader::new(stderr).lines();

        let mut bound_port: Option<u16> = None;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        while bound_port.is_none() {
            if tokio::time::Instant::now() >= deadline {
                anyhow::bail!("never saw port line within 30s");
            }
            tokio::select! {
                biased;
                l = tokio::time::timeout_at(deadline, out_lines.next_line()) => {
                    if let Ok(Ok(Some(line))) = l {
                        eprintln!("[test_server:out] {}", line);
                        if let Some(p) = parse_port_from_line(&line) { bound_port = Some(p); }
                    } else { break; }
                }
                l = tokio::time::timeout_at(deadline, err_lines.next_line()) => {
                    if let Ok(Ok(Some(line))) = l {
                        eprintln!("[test_server:err] {}", line);
                        if let Some(p) = parse_port_from_line(&line) { bound_port = Some(p); }
                    } else { break; }
                }
            }
        }
        let port = bound_port.context("never saw port line within 30s")?;

        let api_base = format!("http://127.0.0.1:{}/api", port);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let health_deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            if tokio::time::Instant::now() >= health_deadline {
                anyhow::bail!("server never became healthy within 15s");
            }
            match client.get(format!("{}/health", api_base)).send().await {
                Ok(r) if r.status().is_success() => break,
                _ => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }

        let api_key = read_default_api_key(&format!("{}/data", data_dir_path))
            .context("read default api key from api_keys.redb")?;

        Ok(Self {
            child,
            _data_dir: data_dir,
            api_base,
            api_key,
        })
    }

    pub fn api_base(&self) -> &str {
        &self.api_base
    }
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub async fn shutdown(mut self) -> Result<()> {
        let _ = self.child.kill().await;
        // Guard against a stuck child hanging the runner (e.g. redb lock).
        // After 5s we leak the PID — TempDir still drops and cleans the FS.
        match tokio::time::timeout(Duration::from_secs(5), self.child.wait()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => tracing::warn!(error = %e, "test_server child wait error"),
            Err(_) => tracing::warn!("test_server child did not exit within 5s after kill — leaking PID"),
        }
        Ok(())
    }
}

fn parse_port_from_line(line: &str) -> Option<u16> {
    // Match "127.0.0.1:PORT" or "http://127.0.0.1:PORT". Last :digits wins.
    let bytes = line.as_bytes();
    let mut colon_idx = None;
    for (i, b) in bytes.iter().enumerate().rev() {
        if *b == b':' {
            colon_idx = Some(i);
            break;
        }
        if !b.is_ascii_digit() {
            continue;
        }
    }
    let colon = colon_idx?;
    let rest: String = line[colon + 1..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    rest.parse::<u16>().ok().filter(|&p| p > 0)
}

fn read_default_api_key(data_dir: &str) -> Result<String> {
    neomind_cli_ops::auto_auth::read_default_api_key_from(data_dir)
        .ok_or_else(|| anyhow::anyhow!("no default api key in fresh data dir"))
}
