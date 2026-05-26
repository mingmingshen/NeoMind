# NeoMind 模块文档

本目录包含 NeoMind 项目各个模块的详细文档。

## 目录结构

```
docs/guides/zh/
├── README.md                     # 本文件
├── 01-core.md                    # 核心 traits、EventBus、DataSourceId
├── 02-llm.md                     # 多后端 LLM 支持（9+ 提供商）
├── 03-agent.md                   # AI 智能体、工具调用、记忆、技能
├── 04-devices.md                 # 设备管理（MQTT、BLE、Webhook）
├── 05-automation.md              # 自动化工作流
├── 06-rules.md                   # DSL 规则引擎
├── 07-tools.md                   # 函数调用工具
├── 08-memory.md                  # 多层级记忆系统
├── 09-messages.md                # 通知系统（7 种渠道）
├── 10-storage.md                 # 存储层（redb）
├── 11-data-push.md               # 数据推送到外部系统
├── 12-commands.md                # 设备命令队列
├── 14-api.md                     # REST/WebSocket API 参考
├── 15-web.md                     # React 前端架构
├── extension-system.md           # 扩展开发指南
├── ble-provisioning.md           # BLE 设备配网
├── custom-dashboard-components.md # 仪表板组件开发
├── examples-guide.md             # 使用示例
└── migration-0.6-to-0.7.md       # 迁移指南（v0.6 → v0.7）
```

## 快速导航

### 核心模块

| 模块 | 状态 | 说明 |
|------|------|------|
| [Core](01-core.md) | 完成 | 核心 trait 定义、事件总线、DataSourceId 格式 |
| [Storage](10-storage.md) | 完成 | 持久化存储、统一时序数据库（redb） |
| [API](14-api.md) | 完成 | REST/WebSocket API、扩展指标 |

### AI 与智能体

| 模块 | 状态 | 说明 |
|------|------|------|
| [LLM 后端](02-llm.md) | 完成 | 多后端 LLM 支持（Ollama、OpenAI、Anthropic 等） |
| [智能体](03-agent.md) | 完成 | AI 对话智能体、工具调用、会话管理 |
| [工具](07-tools.md) | 完成 | 智能体函数调用工具 |
| [记忆](08-memory.md) | 完成 | 多层级记忆（用户画像、知识库、任务记录、系统演化） |

### 物联网与自动化

| 模块 | 状态 | 说明 |
|------|------|------|
| [设备](04-devices.md) | 完成 | 设备管理、MQTT/BLE/Webhook 适配器 |
| [自动化](05-automation.md) | 已更新 | 数据转换、定时智能体 |
| [规则](06-rules.md) | 已更新 | DSL 规则引擎，事件驱动自动化 |
| [命令](12-commands.md) | 已更新 | 设备命令队列和执行跟踪 |

### 通知与集成

| 模块 | 状态 | 说明 |
|------|------|------|
| [消息](09-messages.md) | 已更新 | 7 种通知渠道（Webhook、邮件、Telegram、企业微信、钉钉、Slack、飞书） |
| [数据推送](11-data-push.md) | 新增 | 推送遥测数据到外部系统（Webhook、MQTT） |

### 扩展与前端

| 模块 | 状态 | 说明 |
|------|------|------|
| [扩展系统](extension-system.md) | 推荐 | 完整指南：架构、SDK、能力系统、进程隔离 |
| [前端](15-web.md) | 已更新 | React 18、Zustand、设计系统 |
| [自定义仪表板组件](custom-dashboard-components.md) | 完成 | 构建和发布仪表板组件 |
| [BLE 配网](ble-provisioning.md) | 完成 | 蓝牙零接触设备设置 |

## 模块依赖关系

```
Core ← Storage ← Agent ← API
                  ↑
Core ← LLM ←───┘
Core ← Devices ← Automation ← Rules
Core ← Tools ← Agent
Storage ← Memory ← Agent
Storage ← Messages ← API
```

## 生态仓库

| 仓库 | 说明 |
|------|------|
| [NeoMind-Extensions](https://github.com/camthink-ai/NeoMind-Extensions) | 官方扩展市场 |
| [NeoMind-DeviceTypes](https://github.com/camthink-ai/NeoMind-DeviceTypes) | 设备类型定义 |
| [NeoMind-Dashboard-Components](https://github.com/camthink-ai/NeoMind-Dashboard-Components) | 仪表板组件市场 |

## 技术栈

### 后端
- **语言**: Rust 2024 Edition
- **运行时**: Tokio（异步）
- **Web 框架**: Axum 0.7
- **存储**: redb 2.1
- **序列化**: serde + serde_json

### 前端
- **语言**: TypeScript
- **框架**: React 18
- **构建**: Vite
- **状态管理**: Zustand
- **UI**: Radix UI + Tailwind CSS
- **桌面**: Tauri 2.x
