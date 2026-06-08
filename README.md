<p align="center">
  <img src="web/public/logo-light.png" alt="NeoMind" width="400">
</p>

<h3 align="center">Edge AI Platform for IoT Automation</h3>

<p align="center">
  Rust-powered edge intelligence — connect devices, run AI agents, automate everything.
</p>

<p align="center">
  <a href="https://github.com/camthink-ai/NeoMind/actions/workflows/build.yml">
    <img src="https://github.com/camthink-ai/NeoMind/actions/workflows/build.yml/badge.svg" alt="Build Status">
  </a>
  <img src="https://img.shields.io/badge/License-Apache--2.0-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/v-0.8.8-information.svg" alt="Version">
  <img src="https://img.shields.io/badge/Rust-1.85+-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg" alt="Platform">
</p>

<br/>

<div align="center">
  <table>
    <tr>
      <td align="center" width="35%">
        <img src="docs/img/dashboard_light.png" alt="Dashboard" width="480" style="border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.12);" />
        <br/><sub><b>Dashboard</b></sub>
      </td>
      <td align="center" width="35%">
        <img src="docs/img/dashboard_dark.png" alt="Dark Mode" width="480" style="border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.12);" />
        <br/><sub><b>Dark Mode</b></sub>
      </td>
      <td align="center" width="30%" rowspan="2">
        <img src="docs/img/mobile_web.png" alt="Mobile" width="200" style="border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.12);" />
        <br/><sub><b>Mobile Web</b></sub>
      </td>
    </tr>
    <tr>
      <td align="center">
        <img src="docs/img/chat.png" alt="AI Chat" width="480" style="border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.12);" />
        <br/><sub><b>AI Chat</b></sub>
      </td>
      <td align="center">
        <img src="docs/img/devices.png" alt="Devices" width="480" style="border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.12);" />
        <br/><sub><b>Device Management</b></sub>
      </td>
    </tr>
  </table>
</div>

<br/>

## What is NeoMind?

NeoMind is an **edge-deployed AI platform** that brings intelligence to IoT. It runs LLM-powered agents directly on your hardware, connecting to devices via MQTT/BLE/Webhook, automating responses through a rule engine, and visualizing everything on real-time dashboards — all without relying on cloud services.

**Key idea**: Talk to your devices in natural language. The AI understands your intent, queries device states, creates automation rules, and takes action autonomously.

## Features

### AI-Powered Intelligence
- **Natural Language Chat** — Conversational interface to query and control all connected devices
- **Autonomous Agents** — Scheduled AI agents that monitor, analyze, and act on device data independently
- **10+ LLM Backends** — Ollama, OpenAI, Anthropic, Google, xAI, Qwen, DeepSeek, GLM, MiniMax, and any OpenAI-compatible endpoint
- **Memory System** — Multi-tier memory (Profile, Knowledge, Tasks, Evolution) with automatic extraction and compression
- **Skill System** — YAML+Markdown skills that guide agent behavior for specific scenarios
- **Multimodal** — Image upload and visual analysis support

### Device Management
- **MQTT Protocol** — Primary device integration with embedded broker, mTLS, and CA certificate support
- **BLE Provisioning** — Zero-touch device setup via Bluetooth (Tauri native + Web Bluetooth)
- **HTTP/Webhook** — Flexible device adapter for REST-based devices
- **Auto-Discovery** — Automatic device detection, type registration, and AI-assisted onboarding
- **Command Queue** — Send control commands to devices with parameter validation and tracking
- **Custom Device Types** — Define device metrics and commands via JSON type definitions

### Automation
- **DSL Rule Engine** — Human-readable rule language: `WHEN device("sensor").temperature > 30 DO device("ac").power_on()`
- **Data Transforms** — JavaScript-based data transformation for creating virtual metrics
- **Scheduled Agents** — Time-based and event-driven AI agent execution
- **Event Bus** — Pub/sub architecture for decoupled component communication

### Dashboards & Visualization
- **Drag-and-Drop Builder** — Visual dashboard editor with responsive grid layout
- **Rich Widgets** — Value cards, charts, gauges, tables, VLM vision components
- **Real-time Updates** — WebSocket/SSE for live data streaming to dashboards
- **Dashboard Sharing** — Public links with expiration for sharing dashboards
- **Custom Components** — Build and publish your own dashboard widgets

### Notification & Data Push
- **7 Notification Channels** — Webhook, Email, Telegram, WeCom, DingTalk, Slack, Feishu
- **Data Push** — Forward telemetry data to external systems via Webhook or MQTT
- **Delivery Tracking** — Retry logic with exponential backoff, delivery history, and log management
- **Message Deduplication** — Prevent notification storms from high-frequency triggers

### Platform
- **Multi-Instance** — Connect to and manage multiple NeoMind backends from a single interface
- **Extension System** — Native & WASM extensions with process isolation and capability-based permissions
- **Cross-Platform Desktop** — macOS, Windows, Linux native apps via Tauri
- **Mobile-Friendly Web** — Responsive web UI optimized for phone and tablet
- **i18n** — English and Chinese language support
- **Dark Mode** — System-aware dark/light theme
- **API Key Auth** — Alternative to JWT for programmatic access
- **CLI Tools** — Full-featured command-line interface for all operations

## Ecosystem

NeoMind is a modular ecosystem with specialized repositories for each concern:

| Repository | Purpose |
|------------|---------|
| **[NeoMind](https://github.com/camthink-ai/NeoMind)** | Core platform (this repo) — backend, frontend, desktop app |
| **[NeoMind-Extensions](https://github.com/camthink-ai/NeoMind-Extensions)** | Official extension marketplace — weather, YOLO detection, OCR, face recognition, streaming |
| **[NeoMind-DeviceTypes](https://github.com/camthink-ai/NeoMind-DeviceTypes)** | Device type definitions — standardized metrics and commands for IoT hardware |
| **[NeoMind-Dashboard-Components](https://github.com/camthink-ai/NeoMind-Dashboard-Components)** | Dashboard widget marketplace — community-contributed React components |

### Available Extensions

| Extension | Description |
|-----------|-------------|
| **Weather Forecast** | Real-time weather data via Open-Meteo API. Provides temperature, humidity, wind speed, and precipitation metrics as data sources for dashboards and automation rules. Supports configurable location and polling interval. |
| **Image Analyzer** | YOLOv11-based object detection on uploaded images. Detects people, vehicles, animals, and 80+ COCO categories. Returns bounding boxes, confidence scores, and class labels as structured data. |
| **YOLO Video** | Real-time object detection on live video streams (RTSP/RTMP/HLS). Processes frames at configurable FPS with drop-intermediate-frame queue for low latency. Supports overlay rendering and detection count metrics. |
| **YOLO Device Inference** | Automatically runs YOLO detection on device camera feeds. Binds to NE301/NE101 camera streams, publishes detection results as device metrics. Enables AI-powered alerts when specific objects are detected. |
| **Face Recognition** | ArcFace-based face recognition with enrollment and matching. Supports face database management, real-time detection from camera feeds, and confidence-threshold matching for access control scenarios. |
| **OCR Device Inference** | PP-OCRv4 text recognition on device camera feeds. Extracts text from images and video frames with support for multi-language recognition. Useful for meter reading, license plate recognition, and document processing. |
| **Stream Player** | Video player dashboard component supporting RTSP, RTMP, and HLS protocols. Provides low-latency playback with snapshot capture, fullscreen mode, and device metric overlay. |
| **Home Assistant Bridge** | Bidirectional integration with Home Assistant via REST and WebSocket APIs. Automatic entity discovery, state synchronization, and service call support for device control. |
| **LoRaWAN Bridge** | Connect to LoRaWAN Network Servers (ChirpStack v3/v4, TTN) for IoT device data collection, payload decoding (Cayenne LPP, custom binary), and downlink command injection. |
| **Modbus Bridge** | Modbus TCP/RTU device integration with flexible register map decoding (uint16, int16, uint32, float32, etc.). Multi-device management with independent polling loops. |
| **Uink-RMS Bridge** | Bridge for Uink-RMS e-paper displays. JWT authentication, device template registration, batch device sync, and telemetry collection (battery, temperature, signal strength). |

### Supported Devices

NE301 (Edge AI Camera) and NE101 (Sensing Camera). See [NeoMind-DeviceTypes](https://github.com/camthink-ai/NeoMind-DeviceTypes) for full device type definitions.

### Contribute to the Ecosystem

We welcome community contributions to grow the NeoMind ecosystem:

- **[Build an Extension](https://github.com/camthink-ai/NeoMind-Extensions)** — Create extensions for new data sources, AI models, or integrations. Follow the [Extension Guide](docs/guides/en/extension-system.md) to get started, then submit a PR to the marketplace.
- **[Add a Device Type](https://github.com/camthink-ai/NeoMind-DeviceTypes)** — Define metrics and commands for your IoT hardware so others can use it out of the box. Just add a JSON file.
- **[Create a Dashboard Widget](https://github.com/camthink-ai/NeoMind-Dashboard-Components)** — Build reusable React dashboard components (charts, gauges, maps, etc.) and share them with the community.

## Quick Start

### Desktop App (Recommended)

Download the latest release from [GitHub Releases](https://github.com/camthink-ai/NeoMind/releases/latest).

| Platform | Format |
|----------|--------|
| macOS (Apple Silicon + Intel) | `.dmg` |
| Windows | `.msi` / `.exe` |
| Linux | `.AppImage` / `.deb` |

On first launch, a setup wizard guides you through creating an admin account, configuring your LLM backend, and connecting devices.

### Server Deployment

One-line install (Linux & macOS):

```bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | sh
```

Access the web UI at `http://your-server:9375`.

<details>
<summary>More installation options</summary>

**Docker:**

```bash
git clone https://github.com/camthink-ai/NeoMind.git
cd NeoMind
docker compose up -d
```

**Specific version:**
```bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | VERSION=0.8.7 sh
```

**Custom directories:**
```bash
curl -fsSL ... | INSTALL_DIR=~/.local/bin DATA_DIR=~/.neomind sh
```

**Backend only (no web UI):**
```bash
curl -fsSL ... | NO_WEB=true sh
```

**With nginx reverse proxy (port 80):**
```bash
curl -fsSL ... | USE_NGINX=true sh
```

**Manual installation:**
```bash
VERSION=0.8.7
wget https://github.com/camthink-ai/NeoMind/releases/download/v${VERSION}/neomind-server-linux-amd64.tar.gz
wget https://github.com/camthink-ai/NeoMind/releases/download/v${VERSION}/neomind-web-${VERSION}.tar.gz
tar xzf neomind-server-linux-amd64.tar.gz
sudo install -m 755 neomind /usr/local/bin/
sudo install -m 755 neomind-extension-runner /usr/local/bin/
sudo mkdir -p /var/www/neomind
sudo tar xzf neomind-web-${VERSION}.tar.gz -C /var/www/neomind
./neomind serve
```

**Nginx config:**
```nginx
server {
    listen 80;
    root /var/www/neomind;
    index index.html;
    location / { try_files $uri $uri/ /index.html; }
    location /api/ {
        proxy_pass http://127.0.0.1:9375/api/;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

</details>

### Development

**Prerequisites:** Rust 1.85+, Node.js 20+, Ollama (or other LLM backend)

```bash
# Clone
git clone https://github.com/camthink-ai/NeoMind.git
cd NeoMind

# Start backend (port 9375)
cargo run -p neomind-cli -- serve

# Start frontend dev server (port 5173)
cd web && npm install && npm run dev

# Build desktop app
cd web && npm run tauri:build
```

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                  Desktop App / Web UI                         │
│                   React 18 + TypeScript                       │
├──────────────────────────────────────────────────────────────┤
│                   Tauri 2.x / Browser                         │
└────────────────────────┬─────────────────────────────────────┘
                         │ REST / WebSocket / SSE
                         ▼
┌──────────────────────────────────────────────────────────────┐
│                        API Gateway                            │
│                     Axum Web Server                           │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐    │
│  │ Auth   │ │Devices │ │Automate│ │Messages│ │Extension│   │
│  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘    │
└────────────────────────┬─────────────────────────────────────┘
                         │ Event Bus
          ┌──────────────┼──────────────┬────────────────┐
          ▼              ▼              ▼                ▼
   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐
   │ Devices  │  │Automation│  │ AI Agent │  │   Extensions     │
   │          │  │          │  │          │  │                  │
   │ MQTT     │  │ Rules    │  │ Chat     │  │ Process Isolated │
   │ BLE      │  │ Transform│  │ Tools    │  │ Native + WASM    │
   │ Webhook  │  │ Agents   │  │ Memory   │  │ Capabilities     │
   └──────────┘  └──────────┘  └──────────┘  └──────────────────┘
          │              │              │                │
          └──────────────┴──────────────┴────────────────┘
                         │
                         ▼
   ┌─────────────────────────────────────────────────────────┐
   │                    Storage Layer                          │
   │  ┌────────────┐ ┌────────────┐ ┌──────────┐ ┌────────┐ │
   │  │ Time-Series│ │   State    │ │   LLM    │ │  Push  │ │
   │  │  (redb)    │ │  (redb)    │ │  Memory  │ │  Logs  │ │
   │  └────────────┘ └────────────┘ └──────────┘ └────────┘ │
   └─────────────────────────────────────────────────────────┘
```

## Project Structure

```
NeoMind/
├── crates/
│   ├── neomind-core/            # Core traits and type system
│   ├── neomind-api/             # Web API server (Axum)
│   ├── neomind-agent/           # AI Agent, tool calling, LLM backends
│   ├── neomind-devices/         # Device management (MQTT, BLE, Webhook)
│   ├── neomind-storage/         # Storage layer (redb)
│   ├── neomind-messages/        # Notifications (7 channels)
│   ├── neomind-rules/           # DSL rule engine
│   ├── neomind-data-push/       # Data push to external systems
│   ├── neomind-extension-sdk/   # Extension development SDK
│   ├── neomind-extension-runner/# Extension process isolation
│   └── neomind-cli/             # Command-line interface
├── web/
│   ├── src/                     # React frontend (TypeScript)
│   └── src-tauri/               # Tauri desktop backend (Rust)
├── scripts/                     # Deployment scripts
├── docs/                        # Documentation
├── deploy/                      # Deployment configs (nginx, systemd)
├── Dockerfile                   # Multi-stage Docker build
├── docker-compose.yml           # Docker Compose configuration
└── .env.example                 # Environment variable template
```

## More Screenshots

<details>
<summary>Click to expand</summary>

<br/>

<table>
  <tr>
    <td><b>Login</b></td>
    <td><b>AI Chat</b></td>
  </tr>
  <tr>
    <td><img src="docs/img/login.png" width="480" /></td>
    <td><img src="docs/img/chat.png" width="480" /></td>
  </tr>
  <tr>
    <td><b>AI Agents</b></td>
    <td><b>Rules Engine</b></td>
  </tr>
  <tr>
    <td><img src="docs/img/agents.png" width="480" /></td>
    <td><img src="docs/img/rules.png" width="480" /></td>
  </tr>
  <tr>
    <td><b>Data Transforms</b></td>
    <td><b>Messages</b></td>
  </tr>
  <tr>
    <td><img src="docs/img/transforms.png" width="480" /></td>
    <td><img src="docs/img/messages.png" width="480" /></td>
  </tr>
  <tr>
    <td><b>Extensions</b></td>
    <td><b>Data Push</b></td>
  </tr>
  <tr>
    <td><img src="docs/img/extensions.png" width="480" /></td>
    <td><img src="docs/img/data-push.png" width="480" /></td>
  </tr>
  <tr>
    <td><b>LLM Backends</b></td>
    <td><b>Mobile</b></td>
  </tr>
  <tr>
    <td><img src="docs/img/llm-backends.png" width="480" /></td>
    <td><img src="docs/img/mobile_web.png" width="200" /></td>
  </tr>
</table>

</details>

## Configuration

### LLM Backends

| Backend | Endpoint |
|---------|----------|
| Ollama | `http://localhost:11434` |
| OpenAI | `https://api.openai.com/v1` |
| Anthropic | `https://api.anthropic.com/v1` |
| Google | `https://generativelanguage.googleapis.com/v1beta` |
| xAI | `https://api.x.ai/v1` |
| Qwen | `https://dashscope.aliyuncs.com/compatible-mode/v1` |
| DeepSeek | `https://api.deepseek.com/v1` |
| GLM | `https://open.bigmodel.cn/api/paas/v4` |
| MiniMax | `https://api.minimax.chat/v1` |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |
| `NEOMIND_DATA_DIR` | `/var/lib/neomind` | Data directory |
| `NEOMIND_BIND_ADDR` | `0.0.0.0:9375` | Server bind address |
| `SERVER_PORT` | `9375` | API server port |

## CLI Reference

```bash
neomind serve                          # Start API server
neomind health                        # System health check
neomind device list                   # List devices
neomind device create --name "..."    # Create device
neomind rule list                     # List automation rules
neomind extension list                # List extensions
neomind extension install file.nep    # Install extension
neomind agent list                    # List AI agents
neomind message list                  # List messages
neomind system info                   # System status & network info
neomind api-key create                # Create API key
```

## Extension Development

Build extensions using the Rust SDK with process isolation:

```rust
use neomind_extension_sdk::prelude::*;

pub struct MyExtension;

#[async_trait]
impl Extension for MyExtension {
    fn metadata(&self) -> &ExtensionMetadata {
        static META: OnceLock<ExtensionMetadata> = OnceLock::new();
        META.get_or_init(|| {
            ExtensionMetadata::new("my-extension", "My Extension", "1.0.0")
                .with_description("My custom extension")
                .with_author("Your Name")
        })
    }

    async fn execute_command(&self, cmd: &str, args: &Value) -> Result<Value> {
        match cmd {
            "do_something" => Ok(json!({ "result": "done" })),
            _ => Err(ExtensionError::CommandNotFound(cmd.to_string())),
        }
    }

    fn produce_metrics(&self) -> Result<Vec<ExtensionMetricValue>> {
        Ok(vec![])
    }
}

neomind_export!(MyExtension);
```

See the [Extension Development Guide](docs/guides/en/extension-system.md) and [NeoMind-Extensions](https://github.com/camthink-ai/NeoMind-Extensions) for more examples.

## Documentation

| Resource | Description |
|----------|-------------|
| [CLAUDE.md](CLAUDE.md) | Development guide and code conventions |
| [CHANGELOG.md](CHANGELOG.md) | Version history and release notes |
| [Module Guides](docs/guides/en/) | Detailed module documentation |
| [Extension Guide](docs/guides/en/extension-system.md) | Build your first extension |
| [API Reference](docs/guides/en/14-api.md) | REST/WebSocket API documentation |
| [Frontend Spec](web/DESIGN_SPEC.md) | UI design system and component standards |

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Backend** | Rust, Axum, Tokio, redb |
| **Frontend** | React 18, TypeScript, Tailwind CSS, Zustand, Radix UI |
| **Desktop** | Tauri 2.x |
| **AI/LLM** | Ollama, OpenAI, Anthropic, and 6+ more backends |
| **IoT** | MQTT (embedded broker), BLE, HTTP/Webhook |
| **Extensions** | Native (.so/.dylib/.dll), WASM, process isolation |

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

[Apache-2.0](LICENSE)
