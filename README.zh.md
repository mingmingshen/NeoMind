<p align="center">
  <img src="web/public/logo-square.png" alt="NeoMind Logo" width="120" height="120">
</p>

# NeoMind

> **边缘部署的 LLM 智能体物联网自动化平台**

NeoMind 是一个基于 Rust 的边缘 AI 平台，通过大语言模型（LLM）实现自主设备管理和自动化决策。

[![构建状态](https://github.com/camthink-ai/NeoMind/actions/workflows/build.yml/badge.svg)](https://github.com/camthink-ai/NeoMind/actions/workflows/build.yml)
[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![版本: 0.7.2](https://img.shields.io/badge/v-0.7.2-information.svg)](https://github.com/camthink-ai/NeoMind/releases)
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![平台支持](https://img.shields.io/badge/平台-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/camthink-ai/NeoMind/releases)


<div align="center">
  <table>
    <tr>
      <td width="65%" align="center">
        <table width="100%">
          <tr>
            <td align="center">
              <img src="docs/img/dashboard_light.png" alt="Desktop Light Theme" width="500" style="border-radius: 3px; box-shadow: 0 4px 12px rgba(0,0,0,0.15);" />
              <br/>
              <sub>明亮主题</sub>
            </td>
          </tr>
          <tr>
            <td align="center" style="padding-top: 10px;">
              <img src="docs/img/dashboard_dark.png" alt="Desktop Dark Theme" width="500" style="border-radius: 3px; box-shadow: 0 4px 12px rgba(0,0,0,0.15);" />
              <br/>
              <sub>深色主题</sub>
            </td>
          </tr>
        </table>
        <sub><b>💻 桌面应用</b></sub>
      </td>
      <td width="35%" align="center" valign="top">
        <img src="docs/img/mobile_web.png" alt="Mobile Web" width="220" style="border-radius: 3px; box-shadow: 0 4px 12px rgba(0,0,0,0.15);" />
        <br/>
        <sub>📱 移动网页</sub>
      </td>
    </tr>
  </table>
</div>


## 核心特性

### 🧠 LLM 作为系统大脑
- **交互式对话**: 自然语言界面查询和控制设备
- **AI 智能体**: 具有工具调用能力的自主智能体用于自动化
- **AI 分析师**: 仪表板组件，支持绑定数据源的实时数据分析和可配置触发器
- **聚焦/自由模式**: 两种执行模式 — 聚焦模式用于限定范围的单次监控任务；自由模式用于多轮开放式推理
- **AI 指标**: 智能体可通过 `ai_metric` 工具创建和查询自定义时序指标（异常评分、预测、派生指标）
- **Shell 工具**: 智能体可执行系统命令，支持登录 Shell 环境
- **技能系统**: 用户定义的 YAML+Markdown 技能，用于场景驱动的智能体操作指南
- **聚合工具**: 高效的工具定义，减少 60%+ 的 Token 消耗
- **多后端支持**: Ollama、OpenAI、Anthropic、Google、xAI、Qwen、DeepSeek、GLM、MiniMax（含思维模型兼容）

### 🔌 模块化设备接入
- **BLE 配网**: 通过低功耗蓝牙零接触设备设置（Tauri 原生 + Web Bluetooth），支持 WiFi、HaLow 和 Cat.1 设备
- **MQTT 协议**: 主要设备集成方式，支持嵌入式 Broker、mTLS 和 CA 证书
- **设备发现**: 自动检测设备并注册类型
- **HTTP/Webhook**: 灵活的设备适配器选项
- **自动入板**: AI 辅助从数据样本注册设备

### ⚡ 事件驱动架构
- **实时响应**: 设备变化自动触发规则和自动化
- **解耦设计**: 所有组件通过事件总线通信
- **多传输方式**: REST API、WebSocket、SSE

### 🔗 多实例管理
- **远程后端切换** — 从单一 Web 界面连接并切换多个 NeoMind 后端
- **API 密钥认证** — 通过 API 密钥无密码访问远程实例（JWT 登录的替代方案）
- **预切换验证** — 切换前验证 API 密钥连通性，避免断连状态
- **实例管理器界面** — 全屏对话框用于切换和管理远程实例

### 📦 完整的存储系统
- **时序数据**: 设备指标历史存储和查询（redb）
- **状态存储**: 设备状态、自动化执行记录
- **LLM 记忆**: 分类记忆系统（用户画像、知识库、任务记录、系统演化），支持 LLM 自动提取和压缩。智能体执行过程中自动发现阈值、基线和优化洞察，存储为 `system_evolution` 记忆
- **向量检索**: 跨设备和规则的语义搜索
- **数据浏览器**: 统一时序数据浏览和探索界面

### 🔭 VLM 视觉仪表板组件
- **实时视觉分析**: 将摄像头/视频帧流式传输到 VLM 模型，实现实时场景理解
- **WebSocket 流式传输**: 低延迟逐帧分析，采用丢弃中间帧队列
- **可配置模型**: 选择任意 LLM 后端作为视觉模型，支持自定义系统提示和上下文窗口
- **仪表板集成**: 基于注册表的组件，支持数据源绑定（设备指标、扩展、AI 指标）

### 🧩 统一扩展系统（V2）
- **动态加载**: 运行时扩展加载/卸载
- **Native 和 WASM**: 支持 .so/.dylib/.dll 和 .wasm 扩展
- **设备标准**: 扩展使用与设备相同的类型系统
- **进程隔离**: 安全执行环境，崩溃时自动恢复

### 🛡️ 可靠性与性能
- **无锁并发**: 基于 `DashMap` 的设备注册表，零竞争读写
- **智能资源加载**: 延迟遥测加载、防抖事件刷新、条件扩展轮询
- **健壮的错误处理**: 所有 `unwrap()` 调用已替换为安全的错误传播，覆盖 8 个 crate
- **输入验证**: 所有 POST/PUT 端点的 API 级参数验证
- **错误边界**: React Error Boundaries 实现页面故障的优雅处理
- **速率限制**: 基于用户 JWT 的速率限制（5000 次/分钟），前端 429 重试
- **全面测试**: 跨存储、智能体、规则、消息、扩展运行器和 API 层的 480+ 单元测试

### 🖥️ 桌面应用
- **跨平台**: macOS、Windows、Linux 原生应用
- **现代 UI**: React 18 + TypeScript + Tailwind CSS 统一视觉设计
- **系统托盘**: 后台运行，快速访问
- **自动更新**: 内置更新通知


## 📸 更多截图

<details>
<summary>点击查看更多界面截图</summary>

<br/>

**登录**
![Login](docs/img/login.png)

**对话**
![Chat Interface](docs/img/chat.png)

**AI 智能体**
![AI Agent](docs/img/ai_agent.png)
![AI Agent](docs/img/create_ai_agent.png)
![AI Agent](docs/img/ai_agent_details.png)

**仪表板**
![Dashboard Light](docs/img/dashboard_light.png)
![Dashboard Dark](docs/img/dashboard_dark.png)

**设备管理**
![Devices](docs/img/device.png)
![Device Info](docs/img/device_info.png)
![Device Types](docs/img/device_type.png)
![BLE Provisioning](docs/img/ble_provision.png)
![Pending Devices](docs/img/pending_devices.png)

**自动化**
![Rules](docs/img/rules.png)
![Rules](docs/img/create_rules.png)
![Transforms](docs/img/transformsdata.png)
![Transforms](docs/img/create_transformsdata.png)

**消息**
![Messages](docs/img/messages.png)

**扩展**
![Extensions](docs/img/extensions.png)

**系统**
![Settings](docs/img/settings.png)

</details>

## 快速开始

选择您的部署方式：

### 📱 桌面应用（推荐给终端用户）

从[发布页面](https://github.com/camthink-ai/NeoMind/releases/latest)下载适合您平台的最新版本。

**支持平台：**
- macOS (Apple Silicon + Intel) - `.dmg`
- Windows - `.msi` / `.exe`
- Linux - `.AppImage` / `.deb`

首次启动时，设置向导将引导您完成：
1. 创建管理员账户
2. 配置 LLM 后端（推荐使用 Ollama 进行边缘部署）
3. 连接到您的 MQTT 代理或发现设备

### 🖥️ 服务器二进制部署

> **开箱即用**: 服务器同时提供 API 和 Web UI，无需 nginx。

**包含内容：**
- **neomind** - 内置静态文件服务的 API 服务器二进制（约 50 MB）
- **neomind-extension-runner** - 沙箱扩展的扩展进程（约 8 MB）
- **neomind-web-{version}.tar.gz** - 前端静态文件

**一键安装（Linux & macOS）：**

```bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | sh
```

安装后，在浏览器中访问 `http://your-server:9375`。

**安装指定版本：**

```bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | VERSION=0.7.2 sh
```

**自定义安装：**

```bash
# 自定义目录
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | INSTALL_DIR=~/.local/bin DATA_DIR=~/.neomind sh

# 跳过服务安装
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | NO_SERVICE=true sh

# 使用 nginx 进行前后端分离（端口 80）
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | USE_NGINX=true sh
```

**支持平台：**
- Linux (x86_64/amd64, aarch64/arm64)
- macOS (Intel, Apple Silicon)

**安装脚本功能：**
1. 自动检测操作系统和架构
2. 下载服务器二进制和前端静态文件
3. 安装到 `/usr/local/bin`（或自定义目录）
4. 创建 systemd 服务（Linux）或 launchd 服务（macOS）
5. 启动服务 — 在 `http://your-server:9375` 访问 Web UI

**手动安装：**

```bash
# 1. 下载服务器二进制和前端（替换 VERSION 和 PLATFORM）
# PLATFORM: linux-amd64, linux-arm64, darwin-arm64
VERSION=0.7.2

wget https://github.com/camthink-ai/NeoMind/releases/download/v${VERSION}/neomind-server-linux-amd64.tar.gz
wget https://github.com/camthink-ai/NeoMind/releases/download/v${VERSION}/neomind-web-${VERSION}.tar.gz

# 2. 解压并安装服务器
tar xzf neomind-server-linux-amd64.tar.gz
sudo install -m 755 neomind /usr/local/bin/
sudo install -m 755 neomind-extension-runner /usr/local/bin/

# 3. 部署前端到 nginx
sudo mkdir -p /var/www/neomind
sudo tar xzf neomind-web-${VERSION}.tar.gz -C /var/www/neomind

# 4. 启动 API 服务器
./neomind serve
```

**Nginx 配置**（Web UI 需要）：

```nginx
server {
    listen 80;
    server_name _;

    root /var/www/neomind;
    index index.html;

    # SPA 路由
    location / {
        try_files $uri $uri/ /index.html;
    }

    # API 反向代理
    location /api/ {
        proxy_pass http://127.0.0.1:9375/api/;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_read_timeout 86400;
    }
}
```

**访问应用：**
- Web UI: http://your-server（通过 nginx 端口 80）
- API: http://localhost:9375/api

### 💻 开发模式

#### 环境要求

- Rust 1.85+
- Node.js 20+
- Ollama（本地 LLM）或 OpenAI API 密钥

#### 1. 安装 Ollama

```bash
# Linux/macOS
curl -fsSL https://ollama.com/install.sh | sh

# 拉取轻量级模型
ollama pull qwen2.5:3b
```

#### 2. 启动后端

```bash
# 克隆仓库
git clone https://github.com/camthink-ai/NeoMind.git
cd NeoMind

# 构建并运行 API 服务器
cargo run -p neomind-cli -- serve
```

服务器默认在 `http://localhost:9375` 上启动。

#### 3. 启动前端

```bash
cd web
npm install
npm run dev
```

在浏览器中打开 `http://localhost:5173`。

#### 4. 构建桌面应用

```bash
cd web
npm install
npm run tauri:build
```

安装程序将在 `web/src-tauri/target/release/bundle/` 目录中。

---

## 部署选项

| 方式 | 适用场景 | 平台支持 |
|--------|----------|------|
| **桌面应用** | 终端用户桌面应用（一体化） | macOS, Windows, Linux |
| **服务器二进制** | 服务器/无头部署（一体化，无需 nginx） | Linux (amd64/arm64), macOS |
| **一键安装** | 快速服务器部署 — 服务器自带 Web UI | `curl -fsSL ... \| sh` |

---

## 配置文件

| 文件 | 说明 |
|------|-------------|
| `config.minimal.toml` | 最小配置，快速开始 |
| `config.toml` | 完整配置（从 minimal 复制） |

### LLM 后端支持

| 后端 | 特性标志 | 默认端点 |
|---------|--------------|------------------|
| Ollama | `ollama` | `http://localhost:11434` |
| OpenAI | `openai` | `https://api.openai.com/v1` |
| Anthropic | `anthropic` | `https://api.anthropic.com/v1` |
| Google | `google` | `https://generativelanguage.googleapis.com/v1beta` |
| xAI | `xai` | `https://api.x.ai/v1` |
| Qwen (阿里云) | `cloud` | `https://dashscope.aliyuncs.com/compatible-mode/v1` |
| DeepSeek | `cloud` | `https://api.deepseek.com/v1` |
| GLM (智谱) | `cloud` | `https://open.bigmodel.cn/api/paas/v4` |
| MiniMax | `cloud` | `https://api.minimax.chat/v1` |

> **注意**: Qwen、DeepSeek、GLM 和 MiniMax 使用 OpenAI 兼容 API，通过 `cloud` 特性启用。

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RUST_LOG` | `info` | 日志级别（trace、debug、info、warn、error） |
| `NEOMIND_DATA_DIR` | `/var/lib/neomind` | 数据目录 |
| `NEOMIND_BIND_ADDR` | `0.0.0.0:9375` | 绑定地址 |
| `SERVER_PORT` | `9375` | API 服务器端口 |

---

## 系统架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      桌面应用 / Web 界面                                  │
│                       React + TypeScript                                 │
└─────────────────────────────────┬───────────────────────────────────────┘
                                  │ REST API / WebSocket / SSE
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           API 网关                                       │
│                        Axum Web 服务器                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐     │
│  │  认证    │ │ 设备管理 │ │ 自动化   │ │ 消息通知 │ │ 扩展系统 │     │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘     │
└─────────────────────────────────┬───────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          事件总线                                        │
│              发布/订阅模式实现组件间解耦通信                              │
└────────┬───────────────┬───────────────┬───────────────┬────────────────┘
         │               │               │               │
         ▼               ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐
│  设备管理   │  │  自动化     │  │  LLM 智能体 │  │      扩展系统       │
│  (MQTT)     │  │   引擎      │  │             │  │                     │
│             │  │             │  │ ┌─────────┐ │  │ ┌─────────────────┐ │
│ ┌─────────┐ │  │ ┌─────────┐ │  │ │  对话   │ │  │ │ExtensionContext │ │
│ │ 指标    │ │  │ │  规则   │ │  │ │         │ │  │ │  (能力访问)     │ │
│ │         │ │  │ │         │ │  │ ├─────────┤ │  │ └────────┬────────┘ │
│ ├─────────┤ │  │ ├─────────┤ │  │ │  工具   │ │  │          │          │
│ │ 命令    │ │  │ │ 数据转换│ │  │ │         │ │  │          ▼          │
│ └─────────┘ │  │ └─────────┘ │  │ ├─────────┤ │  │ ┌─────────────────┐ │
│ ┌─────────┐ │  │             │  │ │  记忆   │ │  │ │    能力模块     │ │
│ │BLE 配网 │ │  │             │  │ └─────────┘ │  │ │设备/存储/事件/  │ │
│ └─────────┘ │  │             │  │             │  │ │规则/智能体/...  │ │
└─────────────┘  └─────────────┘  └─────────────┘  │ └─────────────────┘ │
                                                  │                     │
                                                  │  进程隔离运行        │
                                                  │  (.so/.dylib/.wasm) │
                                                  └─────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          存储层                                          │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐   │
│  │   时序数据   │ │    状态      │ │   LLM 记忆   │ │   向量检索   │   │
│  │   (redb)     │ │   (redb)     │ │              │ │              │   │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 项目结构

```
neomind/
├── crates/
│   ├── neomind-core/          # 核心 traits 和类型定义
│   ├── neomind-api/           # Web API 服务器（Axum）
│   ├── neomind-agent/         # AI 智能体与工具调用、LLM 后端（包含已合并的 LLM 模块）
│   ├── neomind-devices/       # 设备管理（MQTT）
│   ├── neomind-storage/       # 存储系统（redb）
│   ├── neomind-messages/      # 统一消息和通知
│   ├── neomind-rules/         # 自动化规则引擎
│   ├── neomind-extension-sdk/ # 扩展开发 SDK
│   ├── neomind-extension-runner/ # 扩展进程隔离运行器
│   └── neomind-cli/           # 命令行接口
├── web/               # React 前端 + Tauri 桌面应用
│   ├── src/           # TypeScript 源码
│   └── src-tauri/     # 桌面应用 Rust 后端
├── scripts/           # 部署脚本
│   ├── install.sh     # 服务器安装脚本
│   └── neomind.service # systemd 服务文件
├── docs/              # 文档
└── config.*.toml      # 配置文件
```

---

## 技术栈

### 后端
- **语言**: Rust 1.85+
- **异步运行时**: Tokio
- **Web 框架**: Axum
- **存储**: redb（嵌入式键值数据库）
- **序列化**: serde / serde_json
- **日志**: tracing

### 前端
- **框架**: React 18 + TypeScript
- **构建**: Vite
- **UI**: Tailwind CSS + Radix UI
- **桌面**: Tauri 2.x
- **状态管理**: Zustand

---

## API 端点

| 分类 | 端点 |
|------|-----------|
| **健康检查** | `/api/health`、`/api/health/status`、`/api/health/live`、`/api/health/ready` |
| **认证** | `/api/auth/login`、`/api/auth/register`、`/api/auth/status`、`/api/auth/verify` |
| **初始化** | `/api/setup/status`、`/api/setup/initialize`、`/api/setup/llm-config` |
| **设备** | `/api/devices`、`/api/devices/:id`、`/api/devices/discover` |
| **设备类型** | `/api/device-types`、`/api/device-types/:id` |
| **自动化** | `/api/automations`、`/api/automations/:id`、`/api/automations/templates` |
| **规则** | `/api/rules`、`/api/rules/:id`、`/api/rules/:id/test` |
| **数据转换** | `/api/automations/transforms`、`/api/automations/transforms/:id` |
| **会话** | `/api/sessions`、`/api/sessions/:id`、`/api/sessions/:id/chat` |
| **对话** | `/api/chat`（WebSocket） |
| **LLM 后端** | `/api/llm-backends`、`/api/llm-backends/:id`、`/api/llm-backends/types` |
| **Ollama 模型** | `/api/llm-backends/ollama/models` |
| **记忆** | `/api/memory/*`（记忆操作） |
| **工具** | `/api/tools`、`/api/tools/:name/execute` |
| **消息** | `/api/messages`、`/api/messages/:id`、`/api/messages/channels` |
| **扩展** | `/api/extensions`（动态扩展） |
| **实例** | `/api/instances`、`/api/instances/:id`、`/api/instances/:id/test` |
| **数据源** | `/api/data/sources`（跨设备、扩展、数据转换、AI 指标的统一查询） |
| **遥测** | `/api/telemetry`、`/api/devices/:id/telemetry`（时序查询） |
| **能力** | `/api/capabilities`、`/api/capabilities/:name` |
| **事件** | `/api/events/stream`（SSE）、`/api/events/ws`（WebSocket） |
| **统计** | `/api/stats/system`、`/api/stats/devices`、`/api/stats/rules` |
| **仪表板** | `/api/dashboards`、`/api/dashboards/:id`、`/api/dashboards/templates` |
| **搜索** | `/api/search` |

---


## CLI 工具

NeoMind 提供命令行界面用于服务管理和操作。

### 安装

CLI 已包含在 NeoMind 中。使用方式：

```bash
cargo run -p neomind-cli -- <命令>
```

或构建并安装到系统：

```bash
cargo build --release -p neomind-cli
cargo install --path crates/neomind-cli
```

### 可用命令

#### 健康检查

检查系统健康状态：

```bash
neomind health
```

检查内容包括：
- 服务器状态
- 数据库连接
- LLM 后端可用性
- 扩展目录

#### 日志查看

查看和过滤系统日志：

```bash
# 查看所有日志
neomind logs

# 按日志级别过滤（ERROR、WARN、INFO、DEBUG）
neomind logs --level ERROR

# 实时跟踪日志
neomind logs --follow

# 显示最后 N 行
neomind logs --tail 100

# 查看最近一段时间的日志
neomind logs --since 1h
```

#### 扩展管理

管理 NeoMind 扩展：

```bash
# 列出已安装扩展
neomind extension list

# 创建新的扩展脚手架
neomind extension create my-extension

# 安装 .nep 包
neomind extension install my-extension-1.0.0.nep

# 卸载扩展
neomind extension uninstall my-extension

# 验证包格式
neomind extension validate my-extension-1.0.0.nep

# 获取扩展信息
neomind extension info my-extension
```

#### API 密钥管理

```bash
neomind api-key create              # 创建新 API 密钥
neomind api-key list                # 列出所有 API 密钥
neomind api-key delete <name>       # 删除 API 密钥
```

#### 服务器管理

启动和管理 NeoMind 服务器：

```bash
# 启动服务器
neomind serve

# 在特定主机/端口启动
neomind serve --host 0.0.0.0 --port 9375
```

### 环境变量

CLI 遵守以下环境变量：

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `NEOMIND_SERVER_URL` | 服务器 API URL | `http://localhost:9375` |
| `NEOMIND_LOG_LEVEL` | 日志级别 | `info` |
| `NEOMIND_DATA_DIR` | 数据目录 | `~/.neomind` |

---

## 扩展开发

使用扩展 SDK V2 为 NeoMind 创建动态扩展：

### 基于能力的访问控制

扩展使用基于能力的访问控制系统来访问系统资源：

| 能力 | 描述 |
|------|------|
| `device_metrics_read` | 读取设备指标和状态 |
| `device_metrics_write` | 写入设备指标（虚拟传感器） |
| `device_control` | 向设备发送命令 |
| `storage_query` | 查询遥测存储 |
| `event_publish` | 向 EventBus 发布事件 |
| `rule_engine` | 访问自动化规则 |
| `agent_invoke` | 调用 AI 智能体能力 |
| `extension_manage` | 管理其他扩展 |

### 基础扩展示例

```rust
use neomind_extension_sdk::prelude::*;
use std::sync::atomic::{AtomicI64, Ordering};

pub struct MyExtension {
    counter: AtomicI64,
}

impl MyExtension {
    pub fn new() -> Self {
        Self { counter: AtomicI64::new(0) }
    }
}

#[async_trait]
impl Extension for MyExtension {
    fn metadata(&self) -> &ExtensionMetadata {
        static META: std::sync::OnceLock<ExtensionMetadata> = std::sync::OnceLock::new();
        META.get_or_init(|| ExtensionMetadata {
            id: "my-extension".to_string(),
            name: "我的扩展".to_string(),
            version: Version::parse("1.0.0").unwrap(),
            description: Some("我的第一个扩展".to_string()),
            author: Some("你的名字".to_string()),
            ..Default::default()
        })
    }

    async fn execute_command(&self, cmd: &str, args: &Value) -> Result<Value> {
        match cmd {
            "increment" => {
                let amount = args.get("amount").and_then(|v| v.as_i64()).unwrap_or(1);
                let new_value = self.counter.fetch_add(amount, Ordering::SeqCst) + amount;
                Ok(json!({ "counter": new_value }))
            }
            _ => Err(ExtensionError::CommandNotFound(cmd.to_string())),
        }
    }

    fn produce_metrics(&self) -> Result<Vec<ExtensionMetricValue>> {
        Ok(vec![ExtensionMetricValue {
            name: "counter".to_string(),
            value: ParamMetricValue::Integer(self.counter.load(Ordering::SeqCst)),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }])
    }
}

// 导出 FFI - 只需要这一行！
neomind_extension_sdk::neomind_export!(MyExtension);
```

详情请参阅 [扩展开发指南](docs/guides/zh/extension-system.md)。

---

## 使用示例

### 查询设备状态

```
用户: 今天家里温度怎么样？
LLM: 客厅当前温度 26°C，卧室 24°C。
     全天平均温度 25.3°C，最高 28°C（下午 3 点）。
```

### 创建自动化规则

```
用户: 当温度超过 30 度时帮我开空调
LLM: 好的，我创建了一条规则：
     "当客厅温度 > 30°C 持续 5 分钟时，
     打开空调并设置为 26°C"
     确认创建吗？
```

### 自然语言转自动化

```
用户: 当客厅温度超过 30 度时打开空调
     ↓
[意图识别 → 设备匹配 → 动作生成 → 规则创建]
     ↓
可执行的自动化规则
```

---

## 开发命令

```bash
# 构建工作区
cargo build

# 构建优化版本
cargo build --release

# 运行测试
cargo test

# 运行特定 crate 的测试
cargo test -p neomind-agent
cargo test -p neomind-core
cargo test -p neomind-api
cargo test -p neomind-storage

# 检查编译（不构建）
cargo check

# 格式化代码
cargo fmt

# 代码检查
cargo clippy

# 运行 API 服务器（默认端口：9375）
cargo run -p neomind-cli -- serve

# 使用自定义主机和端口运行
cargo run -p neomind-cli -- serve --host 0.0.0.0 --port 9375
```

---

## 数据目录

桌面应用数据存储在各平台的标准位置：

| 平台 | 数据目录 |
|----------|---------------|
| macOS | `~/Library/Application Support/NeoMind/data/` |
| Windows | `%APPDATA%/NeoMind/data/` |
| Linux | `~/.config/NeoMind/data/` |

主要数据库文件：
- `telemetry.redb` - 统一时序存储（设备 + 扩展指标）
- `sessions.redb` - 聊天历史和会话
- `devices.redb` - 设备注册表
- `extensions.redb` - 扩展注册表（V2）
- `automations.redb` - 自动化定义
- `agents.redb` - 智能体执行记录

---

## 监控

**健康检查：**
```bash
curl http://localhost:9375/api/health
```

**状态：**
```bash
curl http://localhost:9375/api/health/status
```

---

## 文档

- **[开发指南](CLAUDE.md)** - 开发和架构文档
- **[扩展开发](docs/guides/zh/extension-system.md)** - 构建你的第一个扩展
- **[模块指南](docs/guides/)** - 详细的模块文档

---

## 核心概念

### 设备类型定义

设备类型定义可用的指标和命令：

```json
{
  "type_id": "temperature_sensor",
  "name": "温度传感器",
  "uplink": [
    { "name": "temperature", "type": "float", "unit": "°C" }
  ],
  "downlink": []
}
```

### DSL（领域特定语言）

人类可读的自动化规则语言：

```
RULE "高温自动开空调"
WHEN device("living_room").temperature > 30
FOR 5m
DO
  device("ac").power_on()
  device("ac").set_temperature(26)
END
```

---

## 贡献

欢迎贡献！请随时提交 Pull Request。

---

## 许可证

Apache-2.0, 详见 [LICENSE](LICENSE) 全文。
