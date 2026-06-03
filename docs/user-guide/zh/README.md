# NeoMind 用户指南

> **版本**：v0.8.5 | **许可证**：Apache 2.0 | **平台**：macOS、Windows、Linux

NeoMind 是一款面向物联网的边缘 AI 平台。连接设备、运行 AI Agent 实现监控与自动化，并通过实时仪表板可视化一切——后端基于 Rust 构建，在边缘硬件上发挥极致性能。

---

## 目录

| # | 指南 | 说明 |
|---|------|------|
| 1 | [安装部署](01-installation.md) | 系统要求、桌面端与服务端安装、网络配置 |
| 2 | [系统设置](02-settings.md) | LLM 后端、通用偏好、数据保留策略 |
| 3 | [AI 对话](03-chat.md) | 会话管理、图片上传、记忆与技能、工具调用 |
| 4 | [设备管理](04-devices.md) | MQTT 与 Webhook 设备、命令、自动发现、设备类型 |
| 4a | [设备接入](04a-device-connection.md) | **深入专题**：MQTT 主题、Webhook 认证、TLS/mTLS、BLE 配网、代码示例（Python/ESP32/Node.js） |
| 5 | [自动化](05-automation.md) | 规则引擎、数据转换、数据浏览、数据推送到外部系统 |
| 6 | [AI Agent](06-agents.md) | Agent 构建器、执行模式、调度、记忆、提示词工程 |
| 7 | [仪表板](07-dashboard.md) | 组件、布局编辑、数据源绑定、公开分享 |
| 8 | [通知推送](08-notifications.md) | 7 种通知渠道、配置指南、消息生命周期、重试机制 |
| 9 | [扩展](09-extensions.md) | 扩展市场、数据源、社区扩展 |

---

## 快速开始

只需四步，即可从零开始完成首次 AI 对话。

### 第一步——安装与启动

**桌面端（推荐）：** 从 [GitHub Releases](https://github.com/camthink-ai/NeoMind/releases/latest) 下载最新版本（.dmg / .msi / .AppImage）。启动应用后，内置服务端将自动在 9375 端口运行。

**服务端：** 在 Linux 或 macOS 上执行一行命令安装：

```bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | sh
```

然后在浏览器中打开 `http://localhost:9375`。更多安装方式请参阅 [安装指南](01-installation.md)。

### 第二步——创建管理员账户

首次启动（无管理员账户时），NeoMind 会显示初始化设置页面：

![初始化设置页面](../../img/login.png)

① **用户名**——管理员登录名（至少 3 个字符）。② **密码**——至少 8 个字符，须同时包含字母和数字。③ **确认密码**——须与上方密码一致。④ **时区**——浏览器自动检测，可按需修改。⑤ 点击 **创建账户** 完成设置。

> 如果未出现设置页面，说明服务端已完成初始化。请直接访问 `http://localhost:9375` 并使用已有账户登录。

### 第三步——配置 LLM 后端

AI 功能至少需要一个 LLM（大语言模型）后端。

1. 从顶部导航栏打开 **设置**，选择 **LLM 后端** 标签页。
2. 点击 **添加后端**。
3. 选择提供商并填写必要字段。
4. 点击 **测试连接** 验证，然后点击 **保存**。

![LLM 后端配置](../../img/settings-llm-page.png)

**新手推荐——Ollama（免费、本地、隐私安全）：**

```bash
# 从 https://ollama.com 安装 Ollama，然后执行：
ollama pull qwen3:8b
```

在设置的 LLM 后端中添加一个 Ollama 后端，地址填写 `http://localhost:11434`，模型名称填写 `qwen3:8b`，无需 API 密钥。

**支持的提供商：**

| 提供商 | 需要 API 密钥 | 默认端点 | 说明 |
|--------|---------------|----------|------|
| **Ollama** | 否 | `http://localhost:11434` | 免费、本地，上手首选 |
| **OpenAI** | 是 | `https://api.openai.com/v1` | GPT-4o、GPT-4o-mini |
| **Anthropic** | 是 | `https://api.anthropic.com` | Claude 系列模型 |
| **Google** | 是 | -- | Gemini 系列模型 |
| **xAI** | 是 | `https://api.x.ai` | Grok 系列模型 |
| **Qwen** | 是 | `https://dashscope.aliyuncs.com/compatible-mode/v1` | 阿里通义千问系列 |
| **DeepSeek** | 是 | `https://api.deepseek.com` | DeepSeek V3/R1 |
| **GLM** | 是 | `https://open.bigmodel.cn/api/paas/v4` | 智谱 AI 模型 |
| **MiniMax** | 是 | -- | MiniMax 模型 |
| **LlamaCpp** | 否 | `http://localhost:8080` | 自托管，适合高级用户 |

### 第四步——开始对话

点击导航栏中的 **对话**，输入消息并按 **Enter** 键发送。

![聊天界面](../../img/chat-main.png)

① **新建对话**——开始一个新的聊天会话。② **历史会话**——在过往对话之间切换。③ **消息输入**——输入文字消息或粘贴图片进行视觉分析。④ **AI 回复**——来自已配置 LLM 的流式响应。

> AI 可以通过自然语言控制设备、查询数据和管理你的 IoT 环境。试试说："显示我的所有设备" 或 "创建一条规则：当温度超过 30 度时通知我"。

---

## 接下来做什么

完成快速开始后，可以按顺序阅读以下指南：

1. **连接设备**——通过 MQTT、Webhook 或 BLE 接入设备，详见 [设备管理](04-devices.md) 和 [设备接入](04a-device-connection.md)。
2. **搭建仪表板**——拖拽组件构建可视化面板，详见 [仪表板](07-dashboard.md)。
3. **设置自动化**——通过规则引擎和数据转换实现自动控制，详见 [自动化](05-automation.md)。
4. **部署 AI Agent**——按计划自主运行的智能代理，详见 [AI Agent](06-agents.md)。
5. **安装扩展**——获取天气、YOLO 检测、OCR 等功能，详见 [扩展](09-extensions.md)。

---

## 移动端访问

NeoMind 的 Web 界面完全支持响应式布局。在手机或平板浏览器中打开 `http://your-server:9375` 即可使用。

![移动端界面](../../img/mobile_web.png)

---

## 附录

### A. 键盘快捷键

| 快捷键 | 功能 |
|--------|------|
| `Enter` | 发送聊天消息 |
| `Shift + Enter` | 换行（不发送） |
| `Escape` | 关闭当前对话框或浮层 |

### B. API 速查

**基础地址**：`http://localhost:9375/api`

**认证方式**：在 `Authorization` 请求头中携带 Bearer 令牌。通过登录接口获取令牌。

| 端点 | 方法 | 说明 |
|------|------|------|
| `/chat/sessions` | `GET` | 获取所有聊天会话 |
| `/devices` | `GET` / `POST` | 列出或创建设备 |
| `/devices/{id}` | `GET` / `PATCH` / `DELETE` | 获取、更新或删除设备 |
| `/rules` | `GET` / `POST` | 列出或创建规则 |
| `/automations` | `GET` / `POST` | 列出或创建数据转换 |
| `/push/targets` | `GET` / `POST` | 列出或创建数据推送目标 |
| `/agents` | `GET` / `POST` | 列出或创建 AI Agent |
| `/agents/{id}/run` | `POST` | 手动触发 Agent 执行 |
| `/messages/channels` | `GET` / `POST` | 列出或创建通知渠道 |
| `/extensions` | `GET` | 列出已安装的扩展 |
| `/settings` | `GET` / `PATCH` | 获取或更新设置 |
| `/setup/status` | `GET` | 检查初始化设置是否完成 |

**完整 API 文档**：访问 `http://localhost:9375/api/docs` 查看交互式 Swagger UI。

### C. 常见问题排查

#### AI 没有响应

| 检查项 | 操作 |
|--------|------|
| 是否已配置 LLM 后端？ | 前往 **设置 > LLM 后端** 确认至少存在一个后端 |
| API 密钥是否有效？ | 点击 LLM 后端的 **测试连接** 进行验证 |
| Ollama 是否在运行？ | 执行 `ollama serve`，检查 `http://localhost:11434` 是否可访问 |
| 模型是否可用？ | 执行 `ollama list` 查看已下载的模型；使用 `ollama pull <模型名>` 拉取 |

#### 设备收不到数据

| 检查项 | 操作 |
|--------|------|
| MQTT Broker 是否可访问？ | 确认 1883 端口已开放且内置 Broker 正在运行 |
| Topic 格式是否正确？ | 确保设备发布到 `device/{type}/{id}/uplink` |
| 适配器是否已配置？ | 检查设备的适配器设置（MQTT Topic 或 Webhook URL） |
| 是否已被自动发现？ | 在 **草稿** 标签页查看是否有未识别的设备数据 |

#### Agent 不执行

| 检查项 | 操作 |
|--------|------|
| Agent 是否已启用？ | 将 Agent 开关切换为 **开启** |
| LLM 后端是否正常？ | 在 **设置** 中测试已分配的 LLM 后端 |
| 调度是否已配置？ | 确认调度类型和参数设置正确 |
| 日志中是否有错误？ | 查看执行历史中的错误信息 |

#### 仪表板组件无数据

| 检查项 | 操作 |
|--------|------|
| 数据源绑定是否正确？ | 确认格式为 `{type}:{id}:{field}`（如 `device:sensor-01:temperature`） |
| 设备/扩展是否在线？ | 确保引用的实体在线且正在发送数据 |
| 数据是否在保留期内？ | 检查保留策略是否过滤掉了过期数据 |

#### 通知未送达

| 检查项 | 操作 |
|--------|------|
| 渠道是否已配置？ | 使用 **测试** 按钮发送一条示例消息 |
| 网络是否可达？ | 确认服务端能访问通知服务的 URL |
| 凭据是否有效？ | 仔细检查 API 密钥、令牌和密码 |
| 投递日志如何？ | 在 **消息** 页面查看错误和重试状态 |

---

## 相关资源

| 资源 | 地址 |
|------|------|
| GitHub 仓库 | [https://github.com/camthink-ai/NeoMind](https://github.com/camthink-ai/NeoMind) |
| 下载（Releases） | [https://github.com/camthink-ai/NeoMind/releases](https://github.com/camthink-ai/NeoMind/releases) |
| 扩展市场 | [https://github.com/camthink-ai/NeoMind-Extensions](https://github.com/camthink-ai/NeoMind-Extensions) |
| 设备类型 | [https://github.com/camthink-ai/NeoMind-DeviceTypes](https://github.com/camthink-ai/NeoMind-DeviceTypes) |
| 仪表板组件 | [https://github.com/camthink-ai/NeoMind-Dashboard-Components](https://github.com/camthink-ai/NeoMind-Dashboard-Components) |
| 问题反馈 | [https://github.com/camthink-ai/NeoMind/issues](https://github.com/camthink-ai/NeoMind/issues) |

---

&copy; 2026 CamThink. 基于 [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) 许可证发布。
