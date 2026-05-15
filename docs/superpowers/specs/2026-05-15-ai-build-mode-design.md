# AI Build Mode - Design Specification

> Date: 2026-05-15
> Status: Draft

## 1. Overview

AI Build 是 NeoMind 平台级的智能构建模式，通过全局 AI 助手面板，让用户以对话方式完成设备接入、模板定义、Dashboard 构建、规则配置、数据转换、扩展开发/安装等全链路工作。

### 核心定位

- **专业构建场景**：不是通用聊天助手，而是加速用户快速构建方案的专业工具
- **原生本机操作**：AI 可直接操作网关/AI Box 本机——文件读写、进程管理、网络操作、硬件访问
- **代码编写能力**：AI 能写代码（Rust/JS/DSL）、编辑现有文件、编译、测试、迭代，直到可运行
- **自主执行 + 确认**：AI 直接调用平台 API 和本地命令执行操作，用户确认关键步骤
- **持久上下文 + 记忆**：Build 过程有完整的上下文感知和跨会话记忆，不会每次从零开始

### 目标用户场景

客户安装 NeoMind 系统后，通过 AI Build 完成：

1. 接入设备/外部数据源（定义模板 → 配置协议 → 注册设备）
2. 构建 Dashboard（快速生成 / 对话设计 / 自定义组件）
3. 配置规则（告警、联动、自动化）
4. 数据转换（格式转换、统计计算、AI 推理结果处理）
5. 开发/安装扩展（协议适配器、数据处理器、外部集成）

## 2. UI Design

### 2.1 Floating Action Button (AIBuildFab)

- 固定在视口右下角，`z-[90]`（低于 AlertDialog `z-[200]` 和 FullScreenDialog `z-[100]`）
- 默认显示 AI 图标（`Sparkles` from lucide），带微妙呼吸动画
- 点击展开面板，再次点击或点击面板外区域收起
- 面板展开时 FAB 变为关闭按钮

### 2.2 AI Panel (AIBuildPanel)

从右侧滑入，固定定位，桌面端宽度 `420px`，移动端全屏。独立滚动，不影响主页面。

```
┌─────────────────────────┐
│ Header: "AI Build" + 关闭 │
├─────────────────────────┤
│ 消息区域（对话 + 操作卡片） │  ← 主内容区，可滚动
│                         │
│  [用户消息]              │
│  [AI 回复 + 推荐操作卡片] │
│  [执行进度卡片]          │
├─────────────────────────┤
│ 快捷操作条（基于平台状态） │  ← 根据平台状态动态变化
│ [生成Dashboard] [创建规则] │
├─────────────────────────┤
│ 输入框 + 发送按钮         │
└─────────────────────────┘
```

### 2.3 消息类型

| 类型 | 用途 |
|---|---|
| `text` | 普通文本回复 |
| `action_card` | 可执行操作卡片，带确认/取消按钮 |
| `preview_card` | 预览卡片（Dashboard 布局预览、代码预览），接受/拒绝 |
| `progress_card` | 执行进度（步骤列表，带完成状态） |
| `suggestion_chips` | 快速建议按钮组 |

### 2.4 快捷操作推荐逻辑

基于平台全局状态推荐，与当前页面无关：

| 平台状态 | 推荐操作 |
|---|---|
| 新系统，无设备 | "扫描局域网设备" / "接入第一台设备" |
| 有设备无模板 | "为 XX 设备定义模板" |
| 有设备无 Dashboard | "生成 Dashboard" |
| 有设备无规则 | "创建告警规则" |
| 扩展数量少 | "浏览扩展市场" / "安装扩展" |

## 3. Context Injection

前端每次发送消息时，自动附带平台状态元数据。不在用户消息里拼接，作为独立 `context` 字段发送。

```typescript
interface BuildContext {
  platformStats: {
    totalDevices: number;
    onlineDevices: number;
    unconfiguredDevices: number;
    totalDashboards: number;
    totalRules: number;
    activeExtensions: string[];
    systemInfo: {
      hostname: string;
      os: string;                  // "linux" | "darwin"
      arch: string;                // "aarch64" | "x86_64"
      uptime: string;
      networkInterfaces: string[];
    };
  };
}
```

后端 Agent 在 system prompt 中注入上下文描述。

## 4. Native Machine Access

AI Build 部署在网关/AI Box 上，必须能原生操作本机，不仅仅是 shell 命令。通过 `system` 聚合工具提供分层能力：

### 4.1 文件系统操作

```
用户: "看看 /etc/mosquitto/mosquitto.conf 的配置"
AI:  → fs_read_file("/etc/mosquitto/mosquitto.conf")
     → 展示内容

用户: "把 MQTT 端口改成 1884"
AI:  → fs_write_file("/etc/mosquitto/mosquitto.conf", modified_content)
     → "需要重启 mosquitto 服务，确认？"
     → system.exec("systemctl restart mosquitto")
```

| Action | 用途 | 需确认 |
|---|---|---|
| `fs_read_file` | 读取任意文件内容 | 否 |
| `fs_write_file` | 写入/创建文件 | 是 |
| `fs_list_dir` | 列出目录内容 | 否 |
| `fs_delete` | 删除文件/目录 | 是 |
| `fs_exists` | 检查文件是否存在 | 否 |

### 4.2 系统操作

| Action | 用途 | 需确认 |
|---|---|---|
| `exec` | 执行 shell 命令 | 视情况 |
| `exec_readonly` | 只读命令（ls, cat, ip, ifconfig 等） | 否 |
| `service_manage` | 启停系统服务 | 是 |
| `process_list` | 查看进程列表 | 否 |
| `process_kill` | 终止进程 | 是 |
| `install_package` | apt/opkg/pip 安装包 | 是 |

### 4.3 网络操作

| Action | 用途 | 需确认 |
|---|---|---|
| `scan_network` | 扫描局域网设备（arp/nmap） | 否 |
| `check_port` | 检查端口连通性 | 否 |
| `http_request` | 发起 HTTP 请求（测试 API） | 否 |
| `network_interfaces` | 列出网络接口和 IP | 否 |
| `serial_list` | 列出串口设备 | 否 |
| `serial_read` | 读取串口数据（探测设备） | 否 |

### 4.4 硬件操作

| Action | 用途 | 需确认 |
|---|---|---|
| `list_usb` | 列出 USB 设备 | 否 |
| `gpio_read` | 读取 GPIO 状态（树莓派等） | 否 |
| `gpio_write` | 写入 GPIO | 是 |
| `disk_usage` | 磁盘使用情况 | 否 |
| `system_info` | CPU/内存/系统信息 | 否 |

### 4.5 安全校验层

所有本机操作经过三层校验：

1. **分类判定**：操作自动归为 readonly / write / dangerous
2. **黑名单硬拦截**：`rm -rf /`、`mkfs`、`dd`、`reboot`、`halt` 等直接拒绝
3. **确认机制**：write/dangerous 操作通过 `ConfirmActionTool` 暂停等待用户确认

## 5. Code Writing Capability

AI Build 的核心能力之一是**编写代码**——不是生成配置，而是真正地写、编辑、编译、测试、部署代码。

### 5.1 代码工作流

```
用户描述需求
    ↓
AI 分析需要什么代码（扩展/组件/脚本）
    ↓
AI 编写代码 → fs_write_file 写入本地文件
    ↓
编译/执行（扩展→cargo build, 组件→直接安装, 脚本→直接运行）
    ↓
错误？ → AI 读取错误 → 编辑修复 → fs_write_file 覆写 → 重试
    ↓
成功 → 部署注册 → 用户验证
```

### 5.2 代码编辑能力

不只是生成新代码，还要能**编辑已有代码**：

```
用户: "扩展里 Modbus 轮询间隔改成 5 秒"
AI:  → fs_read_file("data/extensions/modbus-adapter/src/lib.rs")
     → 定位到 poll_interval 相关代码
     → 生成修改 diff
     → 展示 diff 给用户确认
     → fs_write_file 写入修改
     → build_extension.compile 重新编译
     → build_extension.deploy 重新部署
```

| Action | 用途 | 需确认 |
|---|---|---|
| `code_write` | 写入代码文件（新文件或覆盖） | 是（展示内容） |
| `code_edit` | 编辑已有文件（基于 diff） | 是（展示 diff） |
| `code_read` | 读取代码文件 | 否 |
| `code_search` | 在代码中搜索模式 | 否 |

### 5.3 多语言代码生成

| 语言 | 场景 | 编译/执行方式 |
|---|---|---|
| **Rust** | 扩展开发（协议适配器、数据处理器等） | `cargo build --release` |
| **JavaScript** | 自定义 Dashboard 组件（IIFE bundle） | 直接安装，无需编译 |
| **JavaScript** | Transform 转换逻辑 | 直接保存到 TransformAutomation.js_code |
| **DSL** | 规则定义 | 通过现有 DSL 解析器 |
| **Shell** | 快速脚本（安装、配置、诊断） | 直接执行 |
| **JSON** | Dashboard 配置、设备模板、组件 manifest | 直接保存 |

### 5.4 代码生成上下文

AI 生成代码时需要丰富的上下文，不只是用户描述：

- **Extension SDK 文档**：`neomind-extension-sdk` 的 trait 签名、Builder API、能力系统
- **现有扩展示例**：weather-forecast-v2、yolo-video-v2 等的代码模式
- **项目约定**：Cargo.toml 模板、目录结构、命名规范
- **目标平台信息**：OS、arch、可用库（从 system_info 获取）
- **已有代码**：编辑时读取当前文件内容

这些上下文通过 Build Memory（见 Section 13）和 Tool 内置的 schema 模板提供。

## 6. Device Onboarding

### 6.1 流程：模板先行

```
阶段1: 定义设备类型        阶段2: 配置协议适配         阶段3: 注册设备实例
┌──────────────────┐    ┌──────────────────┐     ┌──────────────────┐
│ 用户描述设备      │ →  │ 选择通信协议      │ →   │ 填连接参数        │
│ AI 生成 MDL 模板  │    │ AI 推断映射规则    │     │ AI 验证连通性      │
│ 确认 metrics/cmds │    │ 配置 Topic/URL    │     │ 上线              │
└──────────────────┘    └──────────────────┘     └──────────────────┘
```

### 6.2 协议适配分层

```
层1: 内置适配器 (MQTT/HTTP/Webhook)     ← 配置即用，现有能力
层2: 预制协议扩展 (Modbus/OPC-UA/BACnet) ← 打包好，AI 帮安装+配置
层3: AI 生成扩展适配器                   ← 现场编码，编译部署
```

### 6.3 典型场景

**场景 A：用户知道设备型号**
```
用户: "我要接入一个 DHT22 温湿度传感器，通过 MQTT 上报"
AI:  → 已知 DHT22 数据格式，生成模板
     → 建议 MQTT Topic 和 JSON Path 映射
     → 用户确认 → 创建模板 + 映射 + 设备实例
```

**场景 B：用户只知道设备地址**
```
用户: "我有个设备在 192.168.1.100:502"
AI:  → system_check_port → 检测到 Modbus TCP
     → 询问设备型号或请求样本数据
     → 调用 device_type_generator 生成 MDL
     → 检查是否需要安装 Modbus 适配器扩展
     → 配置映射 → 注册设备
```

**场景 C：用户提供样本数据**
```
用户: [粘贴] {"t": 25.3, "h": 60.1, "p": 1013, "bat": 3.7}
AI:  → 调用 device_type_generator
     → 推断字段含义，生成 MDL 模板
     → 询问通信协议
```

## 7. Extension Development

### 7.1 AI 现场编码扩展

扩展系统为现场适配设计，AI Build 直接利用此能力。

**流程：**
```
用户描述需求 → AI 生成 Rust 扩展代码 → 本地编译 → 部署注册
```

生成的代码遵循 SDK 模式：
- 实现 `Extension` trait
- 使用 `MetricBuilder` / `CommandBuilder` 定义能力
- `neomind_export!()` 宏导出
- `cdylib` 编译目标

### 7.2 编译环境

**网关有 Rust 工具链**（主路径）：
- AI 直接在本地 `cargo build --release`
- 首次通过 `build_list_toolchain` 检查环境

**网关无工具链**：
- 使用 WASM target（`wasm32-wasip1`，更轻量）
- 或交叉编译（需配置）

### 7.3 扩展类型覆盖

| 扩展类型 | AI 能做什么 |
|---|---|
| 协议适配器 | Modbus、OPC-UA、BACnet、SNMP、自定义串口协议 |
| 数据处理器 | 复杂数据转换、协议解析、格式转换 |
| 外部集成 | 对接第三方云平台、数据库、消息队列 |
| AI 推理 | 本地模型推理（YOLO 等）、边缘 AI |
| 业务逻辑 | 自定义告警、报表生成、自动化任务 |

### 7.4 迭代流程

```
AI 生成代码 → 编译 → 失败/成功
  ↓ 失败：AI 读取编译错误 → 自动修复 → 重新编译
  ↓ 成功：部署 → 注册 → 上线
  ↓ 用户反馈问题 → AI 修改代码 → 重新编译 → 热重载
```

## 8. Dashboard Generation

### 8.1 三层生成方式

**层 1：快速生成** — 基于设备/扩展 metrics 自动组装

AI 根据数据源自动选择组件：

| 数据特征 | 推荐组件 |
|---|---|
| 单个数值 + 单位 | `value-card` |
| 单个数值 + 范围 | `progress-bar` |
| 状态（开/关/告警） | `led-indicator` |
| 时序趋势 | `line-chart` / `area-chart` |
| 多指标对比 | `bar-chart` |
| 占比分布 | `pie-chart` |
| 开关控制 | `toggle-switch` |
| 图片/视频流 | `image-display` / `video-display` |

**层 2：对话式设计** — 用户从零描述，AI 逐步构建

AI 的布局策略：
1. 关键指标置顶，趋势图居中，控制项靠下
2. 遵循组件的 minW/minH/maxW/maxH 约束
3. 大屏 12 列全展开，移动端自动折列
4. 同类组件等宽，视觉平衡

**层 3：对话微调** — 对已有 Dashboard 自然语言修改

```
用户: "把温度图表换成进度条"
用户: "视频缩小到右上角"
用户: "温度超过 35 度变红"
```

### 8.2 生成流程

```
AI 生成 Dashboard JSON → 前端渲染预览卡片 → 用户选择：
  → 确认 → 直接创建
  → 提修改 → AI 调整后重新预览
  → 拒绝 → 放弃
```

## 9. Custom Dashboard Components

### 9.1 通过社区组件系统实现

利用现有 `CommunityRegistry` 和 `FrontendComponentMeta` 体系。AI 直接生成两个文件，无需编译：

**manifest.json** — 组件元数据（尺寸、数据源、配置项、设备类型过滤等）

**bundle.js** — IIFE 格式 React 组件，通过 `<script>` 注入加载：

```js
(function(global) {
  const React = window.React;
  function ThreePhasePanel({ dataSources, config, display }) {
    // ...
  }
  global.NeoMindThreePhasePanel = { default: ThreePhasePanel };
})(window);
```

### 9.2 安装流程

```
AI 生成 manifest.json + bundle.js
  → 调用 /api/frontend-components 安装
  → CommunityRegistry 加载
  → Dashboard 中立即可用
```

无需 Node.js、无需编译、本地直接生效。随时让 AI 改代码重新安装，秒级迭代。

### 9.3 组件与 Dashboard 衔接

安装自定义组件后，AI 在 Dashboard 生成中自动匹配：
- 通过 `device_type_filter` 匹配设备类型
- 匹配成功时优先使用自定义组件替代通用组件

## 10. Rules

### 10.1 AI 生成规则

基于现有 DSL 系统，AI 从自然语言生成规则：

```
用户: "温度超过 35 度发告警，持续 5 分钟自动开风扇"
AI:  → 生成 DSL:
     RULE "高温自动降温"
     WHEN sensor.temperature > 35
     FOR 5 minutes
     DO
         NOTIFY "温度过高: {temperature}°C，已开启风扇"
         EXECUTE device.fan(speed=100)
     END
```

### 10.2 AI 职责

1. 理解用户意图 → 识别条件、阈值、持续时间、动作
2. 查询可用设备/扩展 → 确定数据源和可执行动作
3. 生成 DSL 文本
4. 用户确认 → 调用规则 API 创建

### 10.3 规则动作类型

- `NOTIFY` — 发送通知/消息
- `EXECUTE device.xxx` — 执行设备命令
- `EXECUTE extension.xxx` — 执行扩展命令
- `LOG` — 记录日志
- `TRIGGER agent` — 触发 Agent 执行

## 11. Data Transforms

### 11.1 AI 生成转换

现有 Transform 系统支持两种模式，AI 按场景选择：

**JS 代码模式（复杂逻辑）：**

```
用户: "统计 detections 数组里每种物体的数量"
AI:  → intent: "统计 detections 数组中每个 cls 的数量"
     → js_code:
       const counts = {};
       for (const item of input.detections || []) {
         counts[item.cls || 'unknown'] = (counts[item.cls] || 0) + 1;
       }
       return counts;
     → scope: DeviceType("yolo-camera")
     → output_prefix: "detection_count"
```

**声明式操作（简单映射）：**

```
用户: "把 t 映射成 temperature，h 映射成 humidity"
AI:  → Fork + Extract 操作
```

AI 自动判断：简单字段映射用声明式，复杂逻辑用 JS。

### 11.2 转换输出自动衔接

转换输出通过 `TransformOutputRegistry` 自动注册为数据源，格式为 `transform:{id}:{field}`，可立即用于 Dashboard 和规则。

## 12. Agent Tools (New)

扩展现有 Agent tool 系统，遵循 **Aggregated Tools 模式**（与现有 device/agent/rule 等 5 个聚合工具一致），新增 3 个聚合 tool：

### 12.1 `build` — 构建操作聚合工具

| Action | 用途 | 需确认 |
|---|---|---|
| `generate_template` | 从描述/样本生成 MDL 模板（包装现有 device_type_generator） | 是 |
| `suggest_protocol` | 根据设备信息推荐协议 + 映射配置 | 否 |
| `probe_device` | 扫描目标地址，识别协议和设备类型 | 否 |
| `register_device` | 一键完成：模板 + 适配配置 + 设备实例 | 是 |
| `generate_dashboard` | 根据数据源 + 需求生成完整 Dashboard JSON | 是 |
| `modify_dashboard` | 对已有 Dashboard 执行自然语言修改 | 是 |
| `suggest_layout` | 分析数据源，推荐布局方案 | 否 |
| `query_resources` | 查询平台所有可用数据源 | 否 |
| `generate_widget` | 生成自定义 Dashboard 组件（manifest + IIFE bundle） | 是 |
| `install_widget` | 安装组件到 CommunityRegistry | 是 |
| `list_widgets` | 查询已安装的社区组件 | 否 |
| `create_rule` | 从自然语言生成 DSL + 创建规则 | 是 |
| `create_transform` | 从自然语言生成 JS/声明式操作 + 创建转换 | 是 |
| `test_transform` | 用样本数据测试转换是否正确 | 否 |

### 12.2 `build_extension` — 扩展开发聚合工具

| Action | 用途 | 需确认 |
|---|---|---|
| `generate` | 根据描述生成扩展 Rust 代码 | 是（展示代码） |
| `compile` | 本地编译扩展 | 否 |
| `deploy` | 部署 + 注册扩展 | 是 |
| `iterate` | 修改已有扩展代码并重新编译 | 是（展示 diff） |
| `list_toolchain` | 检查本地编译环境 | 否 |

### 12.3 `system` — 系统能力聚合工具

扩展现有 `ShellTool`（`crates/neomind-agent/src/toolkit/shell.rs`），在 ShellTool 基础上增加白名单/黑名单机制：

| Action | 用途 | 需确认 |
|---|---|---|
| `exec` | 执行 shell 命令（走 ShellTool + 安全校验层） | 是 |
| `scan_network` | 扫描局域网发现设备 | 否 |
| `check_port` | 检查目标连通性 | 否 |
| `resource_usage` | CPU/内存/磁盘使用率 | 否 |
| `list_usb` | 列出 USB 设备 | 否 |
| `install_package` | 安装系统包/扩展 | 是 |

## 13. Security

### 13.1 执行权限

- **只读命令**（扫描、查询、检查连通性）：自动执行，无需确认
- **写入/创建/安装命令**：先展示操作内容，用户确认后执行
- **删除/卸载命令**：必须确认

### 13.2 命令安全

在现有 `ShellTool` 之上增加校验层（不修改 ShellTool 本身）：

- 维护**命令白名单**，不在白名单内的 `system.exec` 直接拒绝
- **危险命令黑名单**（`rm -rf /`、`reboot`、`mkfs`、`dd` 等）硬拦截
- 代码生成遵循安全准则，不引入注入风险

### 13.3 扩展隔离

- 生成的扩展代码在进程隔离沙箱中运行（现有 extension-runner 保证）
- 扩展能力需要明确授权（capability system）

### 13.4 自定义组件

- IIFE bundle 通过 CommunityRegistry 的 `<script>` 注入加载
- **已知风险**：`<script>` 注入的代码拥有完整的 DOM 和网络访问权限，并非真正的沙箱
- **缓解措施**：AI 生成的组件代码在安装前通过 `preview_card` 展示给用户审阅；生产环境建议配合 CSP 限制
- 未来可考虑 `iframe sandbox` 方案进一步隔离

## 14. Technical Architecture

### 14.1 前端新增模块

```
web/src/
├── components/ai-build/
│   ├── AIBuildFab.tsx           # 浮动按钮
│   ├── AIBuildPanel.tsx         # 面板主体
│   ├── MessageList.tsx          # 消息列表
│   ├── messages/
│   │   ├── TextMessage.tsx      # 文本消息
│   │   ├── ActionCard.tsx       # 操作卡片
│   │   ├── PreviewCard.tsx      # 预览卡片
│   │   ├── ProgressCard.tsx     # 进度卡片
│   │   └── SuggestionChips.tsx  # 建议按钮
│   ├── QuickActions.tsx         # 快捷操作条
│   └── ChatInput.tsx            # 输入框
├── hooks/
│   └── useAIBuild.ts            # AI Build 对话 hook
└── store/slices/
    └── aiBuildSlice.ts          # 对话状态管理
```

### 14.2 后端新增模块

```
crates/neomind-agent/src/
└── tools/build/                 # Build 类别 tools
    ├── mod.rs                   # 注册所有 build tools
    ├── system_tools.rs          # 系统能力（exec, scan, check）
    ├── device_tools.rs          # 设备接入
    ├── extension_tools.rs       # 扩展开发
    ├── dashboard_tools.rs       # Dashboard 生成
    ├── widget_tools.rs          # 自定义组件
    ├── rule_tools.rs            # 规则
    └── transform_tools.rs       # 数据转换
```

### 14.3 数据流

AI Build 复用现有 Agent 对话的 **WebSocket** 通道（`/api/sessions/{id}/ws`），不新建独立的通信机制。

```
用户输入 → AIBuildPanel → Agent WebSocket (/api/sessions/{id}/ws)
                                    ↓
                               Agent 理解意图
                                    ↓
                          选择合适的聚合 tool (build/build_extension/system)
                                    ↓
                         ┌──────────┼──────────┐
                         ↓          ↓          ↓
                    In-process   ShellTool   LLM 代码
                    Service 调用  (本地命令)   生成
                    (非 HTTP)     ↓           ↓
                         ↓          ↓          ↓
                    结果 → WebSocket 消息流 → 前端渲染消息卡片
```

Build tools 通过 in-process service 引用直接调用平台能力（与现有 aggregated tools 一致），不走 HTTP roundtrip。

### 14.4 复用现有架构

- **Agent 系统**：复用现有 neomind-agent 的 tool 框架、会话管理、LLM 调用
- **WebSocket 通信**：复用现有 Agent 对话的 WebSocket 通道
- **Aggregated Tools 模式**：新 tools 遵循 `AggregatedToolsBuilder` 模式，通过 `action` 参数路由
- **ShellTool**：`system` 工具基于现有 ShellTool 扩展，增加白名单/黑名单校验层
- **Extension SDK**：代码生成模板基于现有扩展示例
- **CommunityRegistry**：自定义组件直接使用现有安装加载流程
- **Transform 系统**：直接复用 JS 执行引擎（Boa）和输出注册
- **Rule DSL**：直接复用现有 DSL 解析器
- **Device Type Generator**：包装现有 AI 生成能力

### 14.5 确认流程

AI Build 的操作确认通过现有 `ConfirmActionTool`（`crates/neomind-agent/src/tools/interaction.rs`）实现：

1. Agent 判断操作需要确认 → 调用 `ConfirmActionTool`，展示操作描述
2. 前端渲染 `action_card` 消息，显示确认/取消按钮
3. 用户点击确认 → Agent 收到确认信号 → 执行实际操作
4. 用户拒绝 → Agent 收到拒绝信号 → 调整方案或终止

### 14.6 Dashboard JSON Schema

AI 生成 Dashboard 时，必须产出与现有 `fromDashboardDTO()`（`web/src/store/persistence/types.ts`）兼容的 snake_case JSON。Tool 内部包含 schema 校验逻辑，确保生成的 JSON 结构正确。

### 14.7 扩展编译依赖管理

在网关上编译扩展需要 `neomind-extension-sdk` 依赖。方案：

- **标准部署**：NeoMind 安装时在 `data/sdk/` 目录包含 extension-sdk 源码
- **编译时**：生成的 `Cargo.toml` 中 `neomind-extension-sdk` 指向本地路径 `{data_dir}/sdk`
- **WASM 回退**：协议适配器等需要网络/硬件访问的扩展不适用 WASM，仅限数据处理类
- **环境检查**：`build_extension.list_toolchain` 检查 `rustc`、`cargo`、SDK 路径是否就绪

### 14.8 BuildContext 数据来源

前端通过一个轻量的 `/api/build/context` 聚合接口获取平台状态，后端一次查询返回所有字段，避免前端多次 API 调用。该接口在 Agent 服务端内部实现，直接读取各 service 的状态。

### 14.9 错误处理

多步骤操作（如设备接入三阶段）支持部分失败回滚：

- 每步操作记录状态到对话上下文
- 失败时 AI 自动提示已完成的步骤，建议回退或重试
- 扩展编译失败：AI 读取编译错误自动修复，最多重试 3 次
- 网络中断：WebSocket 断线自动重连，对话历史持久化在服务端

### 14.10 i18n

所有 AI Build 面板的 UI 文本使用 `t()` 国际化。AI 的回复语言跟随用户输入语言，Tool 描述对 LLM 保持英文（与现有 tools 一致）。

### 14.11 Z-Index 层级

AI Build 面板 `z-[90]`（低于 AlertDialog `z-[200]` 和 FullScreenDialog `z-[100]`），确保确认对话框始终在面板之上。面板内部的嵌套对话框使用 `z-[110]`。

## 15. Build Memory & Context

AI Build 是专业场景，需要持久化的记忆和丰富的上下文，不能每次对话从零开始。

### 15.1 Build 专属记忆

复用现有三层记忆系统（`crates/neomind-agent/src/memory/`），为 Build 场景增加专属知识库：

**长期记忆（KnowledgeBase）中新增 Build 类别：**

| 知识类型 | 示例 | 来源 |
|---|---|---|
| 设备接入记录 | "客户现场有 3 台 Modbus 电表，型号 XX" | 自动从 Build 操作中提取 |
| 扩展开发记录 | "modbus-meter-adapter 扩展已部署，配置参数..." | 自动提取 |
| Dashboard 模板 | "客户偏好大屏 3 列布局，顶部放视频" | 从用户确认的 Dashboard 中学习 |
| 协议经验 | "客户网络 Modbus TCP 设备常用端口 502、从站地址 1" | 从成功接入中积累 |
| 代码模式 | "客户设备的寄存器映射通常是..." | 从生成的代码中总结 |

**中期记忆（会话历史）：**
- Build 会话自动保存完整对话历史
- 用户下次打开 AI Build 时，AI 知道上次的构建进度
- "上次你正在配置 XX 设备，继续吗？"

**短期记忆（当前会话）：**
- 当前会话的操作链路（已创建的模板、正在编辑的文件等）
- 多步骤操作的中间状态

### 15.2 Build Context 聚合

不只是平台统计数据，Build Context 包含丰富的开发上下文：

```typescript
interface BuildContext {
  // 平台状态（现有）
  platformStats: {
    totalDevices: number;
    onlineDevices: number;
    unconfiguredDevices: number;
    totalDashboards: number;
    totalRules: number;
    activeExtensions: string[];
  };

  // 系统环境（用于代码生成）
  systemInfo: {
    hostname: string;
    os: string;                     // "linux" | "darwin"
    arch: string;                   // "aarch64" | "x86_64"
    rustcVersion: string | null;    // 编译环境
    cargoAvailable: boolean;
    sdkPath: string | null;         // extension-sdk 路径
    diskFreeGB: number;             // 剩余空间
  };

  // 开发上下文（Build 专属）
  buildContext: {
    recentFiles: string[];          // 最近操作的文件
    activeProject: string | null;   // 正在开发的扩展/组件名
    compileErrors: string[];        // 最近编译错误（如果有）
    lastBuildAction: string;        // "extension.compile" / "widget.install" / null
  };

  // 资源索引（复用现有 ResourceIndex）
  resources: {
    devices: DeviceSummary[];       // 设备列表 + metrics + 状态
    extensions: ExtensionSummary[]; // 扩展列表 + commands + metrics
    transforms: TransformSummary[]; // 转换列表 + 输出
    rules: RuleSummary[];           // 规则列表 + 状态
    dashboards: DashboardSummary[]; // Dashboard 列表
  };
}
```

### 15.3 上下文注入策略

不是每次消息都注入全部上下文（太消耗 token），而是**按需注入**：

1. **始终注入**：platformStats + systemInfo（轻量，约 200 token）
2. **按意图注入**：Agent 分析用户意图，只注入相关资源
   - 提到设备 → 注入设备列表和 metrics
   - 提到扩展 → 注入扩展列表和 SDK 信息
   - 提到 Dashboard → 注入数据源和组件列表
3. **记忆检索**：通过 BM25 + embedding 检索相关历史记忆

### 15.4 知识积累

Build 过程中自动积累知识到长期记忆：

```
AI 成功帮用户创建了 modbus-meter-adapter 扩展
  → 提取知识：Modbus TCP 设备接入模式、寄存器映射方法、常用配置
  → 存入 KnowledgeBase（category: BuildExperience）

下次用户说 "再接一个电表"
  → AI 检索到之前的经验
  → "根据之前的配置，建议使用相同的 Modbus 适配器，只需要改从站地址。确认？"
```

### 15.5 与现有记忆系统集成

- **ShortTermMemory**：存储当前 Build 会话的操作链
- **MidTermMemory**：存储 Build 会话历史（`session_id` 前缀 `build_`）
- **LongTermMemory**：新增 `KnowledgeCategory::BuildExperience` 和 `KnowledgeCategory::DeviceProfile`
- **MemoryExtractor**：Build 会话使用独立的提取规则，关注技术决策和配置参数
- **BusinessContext**：Build 上下文通过 `ContextManager` 的 `build_context` 方法注入
