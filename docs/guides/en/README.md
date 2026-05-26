# NeoMind Module Documentation

This directory contains detailed documentation for each module of the NeoMind project.

## Contents

```
docs/guides/en/
├── README.md                     # This file
├── 01-core.md                    # Core traits, EventBus, DataSourceId
├── 02-llm.md                     # Multi-backend LLM support (9+ providers)
├── 03-agent.md                   # AI agent, tool calling, memory, skills
├── 04-devices.md                 # Device management (MQTT, BLE, Webhook)
├── 05-automation.md              # Automation workflows
├── 06-rules.md                   # DSL rule engine
├── 07-tools.md                   # Function calling tools
├── 08-memory.md                  # Multi-tier memory system
├── 09-messages.md                # Notification system (7 channels)
├── 10-storage.md                 # Storage layer (redb)
├── 11-data-push.md               # Data push to external systems
├── 12-commands.md                # Device command queue
├── 14-api.md                     # REST/WebSocket API reference
├── 15-web.md                     # React frontend architecture
├── extension-system.md           # Extension development guide
├── ble-provisioning.md           # BLE device provisioning
├── custom-dashboard-components.md # Dashboard component development
├── examples-guide.md             # Usage examples
└── migration-0.6-to-0.7.md       # Migration guide (v0.6 → v0.7)
```

## Quick Navigation

### Core Modules

| Module | Status | Description |
|--------|--------|-------------|
| [Core](01-core.md) | Complete | Core trait definitions, event bus, DataSourceId format |
| [Storage](10-storage.md) | Complete | Persistent storage, unified time-series database (redb) |
| [API](14-api.md) | Complete | REST/WebSocket API, extension metrics |

### AI & Agent

| Module | Status | Description |
|--------|--------|-------------|
| [LLM Backends](02-llm.md) | Complete | Multi-backend LLM support (Ollama, OpenAI, Anthropic, etc.) |
| [Agent](03-agent.md) | Complete | AI chat agent, tool calling, session management |
| [Tools](07-tools.md) | Complete | Function calling tools for agents |
| [Memory](08-memory.md) | Complete | Multi-tier memory (Profile, Knowledge, Tasks, Evolution) |

### IoT & Automation

| Module | Status | Description |
|--------|--------|-------------|
| [Devices](04-devices.md) | Complete | Device management, MQTT/BLE/Webhook adapters |
| [Automation](05-automation.md) | Updated | Data transformation, scheduled agents |
| [Rules](06-rules.md) | Updated | DSL rule engine for event-driven automation |
| [Commands](12-commands.md) | Updated | Device command queue and execution tracking |

### Notification & Integration

| Module | Status | Description |
|--------|--------|-------------|
| [Messages](09-messages.md) | Updated | 7 notification channels (Webhook, Email, Telegram, WeCom, DingTalk, Slack, Feishu) |
| [Data Push](11-data-push.md) | New | Push telemetry data to external systems (Webhook, MQTT) |

### Extension & Frontend

| Module | Status | Description |
|--------|--------|-------------|
| [Extension System](extension-system.md) | Recommended | Complete guide: architecture, SDK, capabilities, process isolation |
| [Frontend](15-web.md) | Updated | React 18, Zustand, design system |
| [Custom Dashboard Components](custom-dashboard-components.md) | Complete | Build and publish dashboard widgets |
| [BLE Provisioning](ble-provisioning.md) | Complete | Zero-touch device setup via Bluetooth |

## Module Dependencies

```
Core ← Storage ← Agent ← API
                  ↑
Core ← LLM ←───┘
Core ← Devices ← Automation ← Rules
Core ← Tools ← Agent
Storage ← Memory ← Agent
Storage ← Messages ← API
```

## Ecosystem Repositories

| Repository | Description |
|------------|-------------|
| [NeoMind-Extensions](https://github.com/camthink-ai/NeoMind-Extensions) | Official extension marketplace |
| [NeoMind-DeviceTypes](https://github.com/camthink-ai/NeoMind-DeviceTypes) | Device type definitions |
| [NeoMind-Dashboard-Components](https://github.com/camthink-ai/NeoMind-Dashboard-Components) | Dashboard widget marketplace |

## Tech Stack

### Backend
- **Language**: Rust 2024 Edition
- **Runtime**: Tokio (async)
- **Web Framework**: Axum 0.7
- **Storage**: redb 2.1
- **Serialization**: serde + serde_json

### Frontend
- **Language**: TypeScript
- **Framework**: React 18
- **Build**: Vite
- **State**: Zustand
- **UI**: Radix UI + Tailwind CSS
- **Desktop**: Tauri 2.x
