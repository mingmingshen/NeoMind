# AI Build Mode - Design Specification

> Date: 2026-05-15
> Status: Draft

## 1. Overview

AI Build 是 NeoMind 平台级的智能构建模式，通过全局 AI 助手面板，让用户以对话方式完成设备接入、模板定义、Dashboard 构建、规则配置、数据转换、扩展开发/安装等全链路工作。

### 核心定位

- **全局对话助手**：不绑定特定页面，用户随时描述需求，AI 理解并执行
- **自主执行 + 确认**：AI 直接调用平台 API 和本地命令执行操作，用户确认关键步骤
- **本地优先**：部署在网关/AI Box 上，AI 可直接操作本机（编译、部署、网络扫描等）
- **代码生成能力**：AI 能生成扩展 Rust 代码、自定义组件 JS 代码、转换 JS 代码、规则 DSL

### 目标用户场景

客户安装 NeoMind 系统后，通过 AI Build 完成：

1. 接入设备/外部数据源（定义模板 → 配置协议 → 注册设备）
2. 构建 Dashboard（快速生成 / 对话设计 / 自定义组件）
3. 配置规则（告警、联动、自动化）
4. 数据转换（格式转换、统计计算、AI 推理结果处理）
5. 开发/安装扩展（协议适配器、数据处理器、外部集成）

## 2. UI Design

### 2.1 Floating Action Button (AIBuildFab)

- 固定在视口右下角，`z-[300]`
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

## 4. Device Onboarding

### 4.1 流程：模板先行

```
阶段1: 定义设备类型        阶段2: 配置协议适配         阶段3: 注册设备实例
┌──────────────────┐    ┌──────────────────┐     ┌──────────────────┐
│ 用户描述设备      │ →  │ 选择通信协议      │ →   │ 填连接参数        │
│ AI 生成 MDL 模板  │    │ AI 推断映射规则    │     │ AI 验证连通性      │
│ 确认 metrics/cmds │    │ 配置 Topic/URL    │     │ 上线              │
└──────────────────┘    └──────────────────┘     └──────────────────┘
```

### 4.2 协议适配分层

```
层1: 内置适配器 (MQTT/HTTP/Webhook)     ← 配置即用，现有能力
层2: 预制协议扩展 (Modbus/OPC-UA/BACnet) ← 打包好，AI 帮安装+配置
层3: AI 生成扩展适配器                   ← 现场编码，编译部署
```

### 4.3 典型场景

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

## 5. Extension Development

### 5.1 AI 现场编码扩展

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

### 5.2 编译环境

**网关有 Rust 工具链**（主路径）：
- AI 直接在本地 `cargo build --release`
- 首次通过 `build_list_toolchain` 检查环境

**网关无工具链**：
- 使用 WASM target（`wasm32-wasip1`，更轻量）
- 或交叉编译（需配置）

### 5.3 扩展类型覆盖

| 扩展类型 | AI 能做什么 |
|---|---|
| 协议适配器 | Modbus、OPC-UA、BACnet、SNMP、自定义串口协议 |
| 数据处理器 | 复杂数据转换、协议解析、格式转换 |
| 外部集成 | 对接第三方云平台、数据库、消息队列 |
| AI 推理 | 本地模型推理（YOLO 等）、边缘 AI |
| 业务逻辑 | 自定义告警、报表生成、自动化任务 |

### 5.4 迭代流程

```
AI 生成代码 → 编译 → 失败/成功
  ↓ 失败：AI 读取编译错误 → 自动修复 → 重新编译
  ↓ 成功：部署 → 注册 → 上线
  ↓ 用户反馈问题 → AI 修改代码 → 重新编译 → 热重载
```

## 6. Dashboard Generation

### 6.1 三层生成方式

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

### 6.2 生成流程

```
AI 生成 Dashboard JSON → 前端渲染预览卡片 → 用户选择：
  → 确认 → 直接创建
  → 提修改 → AI 调整后重新预览
  → 拒绝 → 放弃
```

## 7. Custom Dashboard Components

### 7.1 通过社区组件系统实现

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

### 7.2 安装流程

```
AI 生成 manifest.json + bundle.js
  → 调用 /api/frontend-components 安装
  → CommunityRegistry 加载
  → Dashboard 中立即可用
```

无需 Node.js、无需编译、本地直接生效。随时让 AI 改代码重新安装，秒级迭代。

### 7.3 组件与 Dashboard 衔接

安装自定义组件后，AI 在 Dashboard 生成中自动匹配：
- 通过 `device_type_filter` 匹配设备类型
- 匹配成功时优先使用自定义组件替代通用组件

## 8. Rules

### 8.1 AI 生成规则

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

### 8.2 AI 职责

1. 理解用户意图 → 识别条件、阈值、持续时间、动作
2. 查询可用设备/扩展 → 确定数据源和可执行动作
3. 生成 DSL 文本
4. 用户确认 → 调用规则 API 创建

### 8.3 规则动作类型

- `NOTIFY` — 发送通知/消息
- `EXECUTE device.xxx` — 执行设备命令
- `EXECUTE extension.xxx` — 执行扩展命令
- `LOG` — 记录日志
- `TRIGGER agent` — 触发 Agent 执行

## 9. Data Transforms

### 9.1 AI 生成转换

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

### 9.2 转换输出自动衔接

转换输出通过 `TransformOutputRegistry` 自动注册为数据源，格式为 `transform:{id}:{field}`，可立即用于 Dashboard 和规则。

## 10. Agent Tools (New)

扩展现有 Agent tool 系统，新增 `build` 类别：

### 10.1 系统能力 Tools

| Tool | 用途 | 需确认 |
|---|---|---|
| `system_exec` | 执行 shell 命令 | 是 — 展示命令让用户确认 |
| `system_scan_network` | 扫描局域网发现设备 | 否 — 只读 |
| `system_check_port` | 检查目标连通性 | 否 |
| `system_resource_usage` | CPU/内存/磁盘使用率 | 否 |
| `system_list_usb` | 列出 USB 设备 | 否 |
| `system_install_package` | 安装系统包/扩展 | 是 |

### 10.2 设备接入 Tools

| Tool | 用途 |
|---|---|
| `build_generate_template` | 从描述/样本生成 MDL 模板（包装现有 device_type_generator） |
| `build_suggest_protocol` | 根据设备信息推荐协议 + 映射配置 |
| `build_probe_device` | 扫描目标地址，识别协议和设备类型 |
| `build_register_device` | 一键完成：模板 + 适配配置 + 设备实例 |

### 10.3 扩展开发 Tools

| Tool | 用途 |
|---|---|
| `build_generate_extension` | 根据描述生成扩展 Rust 代码 |
| `build_compile_extension` | 本地编译扩展 |
| `build_deploy_extension` | 部署 + 注册扩展 |
| `build_iterate_extension` | 修改已有扩展代码并重新编译 |
| `build_list_toolchain` | 检查本地编译环境 |

### 10.4 Dashboard Tools

| Tool | 用途 |
|---|---|
| `build_generate_dashboard` | 根据数据源 + 需求生成完整 Dashboard JSON |
| `build_modify_dashboard` | 对已有 Dashboard 执行自然语言修改 |
| `build_suggest_layout` | 分析数据源，推荐布局方案 |
| `build_query_platform_resources` | 查询平台所有可用数据源 |

### 10.5 自定义组件 Tools

| Tool | 用途 |
|---|---|
| `build_generate_widget` | 生成自定义 Dashboard 组件（manifest + IIFE bundle） |
| `build_install_widget` | 安装组件到 CommunityRegistry |
| `build_list_widgets` | 查询已安装的社区组件 |

### 10.6 规则 Tools

| Tool | 用途 |
|---|---|
| `build_create_rule` | 从自然语言生成 DSL + 创建规则 |

### 10.7 数据转换 Tools

| Tool | 用途 |
|---|---|
| `build_create_transform` | 从自然语言生成 JS/声明式操作 + 创建转换 |
| `build_test_transform` | 用样本数据测试转换是否正确 |

## 11. Security

### 11.1 执行权限

- **只读命令**（扫描、查询、检查连通性）：自动执行，无需确认
- **写入/创建/安装命令**：先展示操作内容，用户确认后执行
- **删除/卸载命令**：必须确认

### 11.2 命令安全

- 维护**命令白名单**，不在白名单内的 `system_exec` 直接拒绝
- **危险命令黑名单**（`rm -rf /`、`reboot`、`mkfs`、`dd` 等）硬拦截
- 代码生成遵循安全准则，不引入注入风险

### 11.3 扩展隔离

- 生成的扩展代码在进程隔离沙箱中运行（现有 extension-runner 保证）
- 扩展能力需要明确授权（capability system）

### 11.4 自定义组件

- IIFE bundle 通过 CommunityRegistry 加载，运行在浏览器沙箱中
- 组件代码遵循 CSP 策略

## 12. Technical Architecture

### 12.1 前端新增模块

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

### 12.2 后端新增模块

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

### 12.3 数据流

```
用户输入 → AIBuildPanel → Agent API (SSE)
                              ↓
                         Agent 理解意图
                              ↓
                    选择合适的 build tool
                              ↓
               ┌──────────────┼──────────────┐
               ↓              ↓              ↓
          Platform API    System Exec    Code Gen
          (CRUD 操作)     (本地命令)    (代码生成)
               ↓              ↓              ↓
          结果 → SSE 事件流 → 前端渲染消息卡片
```

### 12.4 复用现有架构

- **Agent 系统**：复用现有 neomind-agent 的 tool 框架、会话管理、LLM 调用
- **SSE 事件流**：复用现有 Agent 对话的 SSE 推送机制
- **Extension SDK**：代码生成模板基于现有扩展示例
- **CommunityRegistry**：自定义组件直接使用现有安装加载流程
- **Transform 系统**：直接复用 JS 执行引擎（Boa）和输出注册
- **Rule DSL**：直接复用现有 DSL 解析器
- **Device Type Generator**：包装现有 AI 生成能力
