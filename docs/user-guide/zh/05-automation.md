# 自动化

NeoMind 的自动化页面提供两种自动化工具：**规则** 和 **数据转换**。

---

## 自动化页面

在顶部导航栏中点击 **自动化** 进入（`/automation`）。页面默认打开 **规则** 标签页，通过顶部标签栏 ① 切换不同标签。点击 **创建** ② 可根据当前标签新建规则或数据转换。使用 **导入/导出** ③ 下拉菜单可批量管理规则或数据转换的 JSON 文件。

![自动化 - 规则](../../img/automation-rules.png)

> **上图说明**：自动化 > 规则标签页。每行显示规则名称、条件摘要、动作类型和启用状态。行内的开关可启用或禁用规则。操作菜单（三点按钮）提供编辑、执行和删除选项。

---

### 规则标签页

规则用于监控设备或扩展的指标数据，当条件满足时自动触发动作。

#### 创建规则

1. 在 **规则** 标签页中，点击右上角的 **创建** 按钮。
2. 系统打开全屏规则构建器，左侧导航面板显示四个步骤：**基本信息**、**触发条件**、**动作配置** 和 **确认**。

**第一步 —— 基本信息**

3. 输入 **规则名称**（例如"高温报警"）。
4. （可选）添加 **描述**。

**第二步 —— 触发条件配置**

5. 选择 **触发类型**：
   - **数据变化** —— 当指标值满足条件时触发。
   - **定时任务** —— 按照 cron 表达式定时触发（例如每 5 分钟）。
6. 对于 **数据变化** 触发器，需要配置：
   - **来源类型** —— 设备或扩展。
   - **来源** —— 选择具体的设备或扩展。
   - **指标** —— 要监控的指标字段（例如 `temperature`）。
   - **运算符** —— 比较运算符（`>`、`<`、`=`、`>=`、`<=`、`!=`）。
   - **阈值** —— 用于比较的数值（例如 `35`）。
   - **持续时间** —— 条件持续满足多久才触发（例如 `300` 秒 = 5 分钟）。设为 0 则条件满足时立即触发。

**第三步 —— 动作配置**

7. 选择 **动作类型**：

| 动作类型 | 用途 | 关键参数 |
|---------|------|---------|
| **NOTIFY** | 发送通知消息 | 通道、消息模板 |
| **EXECUTE** | 向设备发送命令 | 设备、命令、参数 |
| **LOG** | 写入日志 | 严重级别、消息内容 |
| **SET** | 设置设备属性 | 设备、属性、值 |
| **ALERT** | 创建系统告警 | 标题、消息、严重级别 |
| **HTTP** | 发送 HTTP 请求 | 方法、URL、请求头、请求体 |

8. 填写动作参数。NOTIFY 类型需要选择通知通道并编写消息模板；HTTP 类型需要填写目标 URL 和可选的请求头。
9. 点击 **添加动作** 可在同一规则中串联多个动作。

**第四步 —— 确认**

10. 检查规则摘要，包括名称、条件和动作。
11. 点击 **保存** 创建规则。规则默认为启用状态。

#### 管理规则

- **启用/禁用** —— 点击规则行的开关即可切换。禁用的规则不会被评估，但配置会保留。
- **编辑** —— 打开操作菜单（三点按钮），选择 **编辑** 可重新打开规则构建器，所有字段已预填充。
- **执行** —— 在操作菜单中选择 **执行** 可手动触发规则，立即运行其动作，不受条件是否满足的限制。
- **删除** —— 在操作菜单中选择 **删除** 并确认。
- **导入/导出** —— 使用标签栏中的下拉菜单，将所有规则导出为 JSON 文件，或从之前导出的文件导入规则。

---

### 数据转换标签页

切换到 **数据转换** 标签页。数据转换通过 JavaScript 表达式将原始设备或扩展数据转换为派生指标，生成新的虚拟指标供全平台使用。

![自动化 - 数据转换](../../img/automation-transforms.png)

> **上图说明**：数据转换标签页。每行显示转换名称、输入数据源、输出前缀和启用状态。行内开关可启用或禁用转换。操作菜单提供编辑、导出和删除选项。

#### 创建数据转换

1. 在 **数据转换** 标签页中，点击 **创建**。
2. 系统打开全屏数据转换构建器。

3. **名称** —— 输入描述性名称（例如"摄氏转华氏"）。
4. **描述** —— （可选）说明转换的用途。
5. **输入范围** —— 配置输入数据源：
   - **数据源** —— 选择要转换的设备指标或扩展输出（例如 `device:sensor-01:temperature`）。
   - 输入范围可以是单个数据源或更宽泛的匹配模式。
6. **JavaScript 表达式** —— 编写转换逻辑。输入值通过 `value` 变量获取，表达式必须返回数值或字符串。

| 示例表达式 | 输入 | 输出 | 说明 |
|-----------|------|------|------|
| `value * 9/5 + 32` | 25 | 77 | 摄氏转华氏 |
| `Math.round(value * 100) / 100` | 3.14159 | 3.14 | 保留两位小数 |
| `value > 100 ? 1 : 0` | 150 | 1 | 二值化阈值判断 |
| `Math.max(0, Math.min(100, ((value - 3000) / 1200) * 100))` | 3600 | 50 | 毫伏转电量百分比 |

7. **测试** —— 点击 **测试** 按钮，输入一个样例值即可查看表达式输出结果。反复调整并测试直到正确。
8. **输出前缀** —— 设置输出指标 ID 的前缀（例如 `transform:sensor-01`）。完整的输出 ID 为 `transform:sensor-01:temperature_f`。
9. 点击 **保存**。

#### 使用转换后的数据

转换产生的指标是一等数据源，可以在全平台使用：

- **仪表盘** —— 将转换输出添加为图表或数值组件。
- **规则** —— 在规则条件中引用 `transform:{id}:{field}` 作为指标来源。
- **数据推送** —— 将转换后的数据转发到外部系统。
- **数据浏览** —— 与原始指标一起浏览和查询转换输出。

#### 管理数据转换

- **启用/禁用** —— 开关切换。禁用后转换停止处理，但保留配置。
- **编辑** —— 重新打开构建器修改表达式、范围或输出设置。
- **导出** —— 将单个转换下载为 JSON 文件。
- **删除** —— 永久删除转换。
- **导入/导出** —— 通过标签栏的下拉菜单批量导入或导出转换。

---

## 相关：数据页面

顶部导航栏中的 **数据** 页面（`/data`）提供了额外的工具，用于浏览和转发遥测数据：

- **数据浏览** —— 在统一的可筛选表格中浏览所有设备、扩展和数据转换的遥测数据源，支持历史图表和数据导出。
- **数据推送** —— 通过 Webhook（HTTP）或 MQTT 将遥测数据转发到外部系统（InfluxDB、TimescaleDB、自定义端点等）。

---

## 附录：规则 JSON 格式

高级用户可以通过 CLI（`--json`）或 REST API 直接创建规则。这适用于版本控制、批量导入和复杂多条件规则的场景。

### 基本结构

```json
{
  "name": "<规则名称>",
  "trigger": {"trigger_type": "schedule", "cron": "<cron 表达式>"},
  "condition": { "<条件类型>": "..." },
  "for_duration": <毫秒>,
  "actions": [ { "<类型>": "..." } ]
}
```

### 条件类型

| 类型 | 字段 | 示例 |
|------|------|------|
| `comparison` | `source`, `operator`, `threshold` | `{"condition_type":"comparison","source":"device:sensor:temp","operator":"greater_than","threshold":30}` |
| `range` | `source`, `min`, `max` | `{"condition_type":"range","source":"device:sensor:temp","min":18,"max":25}` |
| `logical` | `operator` (and/or/not), `conditions` | `{"condition_type":"logical","operator":"and","conditions":[...]}` |

**运算符**: `greater_than`, `less_than`, `greater_equal`, `less_equal`, `equal`, `not_equal`

### 动作类型

| 类型 | 字段 | 示例 |
|------|------|------|
| `notify` | `message`, `severity` | `{"type":"notify","message":"过热: {value}","severity":"critical"}` |
| `execute` | `target`, `target_type`, `command`, `params` | `{"type":"execute","target":"fan","target_type":"device","command":"on","params":{"speed":100}}` |
| `trigger_agent` | `agent_id`, `input` | `{"type":"trigger_agent","agent_id":"analyzer","input":"检查温度"}` |

**严重级别**: `info`, `warning`, `critical`, `emergency`

### 完整示例

```bash
neomind rule create --json '{
  "name": "Temperature Alert",
  "condition": {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 50},
  "for_duration": 120000,
  "actions": [
    {"type": "notify", "message": "严重超标: {value}C", "severity": "critical"},
    {"type": "execute", "target": "fan", "target_type": "device", "command": "set_speed", "params": {"speed": 100}}
  ]
}'
```

### API 与 CLI

```bash
# 通过 API 创建规则
curl -X POST http://localhost:9375/api/rules \
  -H "Content-Type: application/json" \
  -d '{"name":"Alert","condition":{"condition_type":"comparison","source":"device:sensor:temp","operator":"greater_than","threshold":30},"actions":[{"type":"notify","message":"温度过高","severity":"warning"}]}'

# CLI 命令（规则默认启用）
neomind rule list
neomind rule create --json '{"name":"Alert","condition":{"condition_type":"comparison","source":"device:sensor:temp","operator":"greater_than","threshold":30},"actions":[{"type":"notify","message":"温度过高","severity":"warning"}]}'
neomind rule disable <rule_id>
neomind rule enable <rule_id>
neomind rule delete <rule_id>
```

---

[< 上一章：设备管理](./04-devices.md) | [下一章：AI 代理 >](./06-agents.md)
