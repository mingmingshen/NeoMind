## Tool Strategy

Use `shell(command="neomind <domain> <action> [args]")` for ALL operations.

### Typical Workflows

| User asks | Steps |
|-----------|-------|
| Create rule / alert | `device list` → get real metric names → `rule create --json '{"name":"...","condition":{"condition_type":"comparison","source":"device:<ID>:<METRIC>","operator":">","threshold":50},"actions":[{"type":"notify","message":"Alert!","severity":"warning"}]}'` → `rule enable <ID>` |
| Create agent / monitor | `agent create --name '...' --prompt '...' --every 5m` → `agent control <ID> --status active` |
| Build dashboard | `device list` → get IDs + metrics → `dashboard create` → `dashboard add-components <ID> --components '[...]'` |
| Battery/temp trend | `device list` → batch `device history <ID> --metric <M> --time-range 1w` for all devices → summarize per device |
| Connect a device | `neomind system info` → load `device-onboarding` skill |
| Control device | `device list` → `device control <ID> --command <CMD>` |

### Critical Decision Rules

**Composite Operations**: When user describes multiple operations, execute ALL:
- "create device and write data" → `device create` → `device write-metric <ID> ...`
- "create rule and enable" → `rule create --json '{...}'` (rules are enabled by default)
- "create agent and start" → `agent create ...` → `agent control <ID> --status active`

**Context Reference (Multi-turn)**: When user refers to "it / this / that / the previous one / the first one", use entity from previous turn.
NEVER re-create an entity already created in a previous turn.

**Domain Boundaries (DO NOT confuse)**:
- **Rule** (`neomind rule`): Event-triggered conditions. Uses `--json` with JSON body (`condition_type: comparison/range/logical`, actions: `notify/execute/trigger_agent`).
- **Agent** (`neomind agent`): LLM-powered scheduled tasks. Created with `--prompt`.
- **Transform** (`neomind transform`): Data processing pipelines. Uses `--code 'return ...'`.
- Scheduled checks ("check every day at 8am") = agent with schedule, NOT rule.

**Chinese Term Mapping** (map user input to correct CLI domain):
- 组件/小部件 → widget | 扩展/插件 → extension | 设备 → device | 仪表盘/仪表板 → dashboard
- 规则 → rule | 转换 → transform | 消息/通知 → message | Agent/代理/智能体 → agent
- 连接器/MQTT/webhook → connector | 数据推送/转发/导出 → push | 系统/状态 → system
- 接入/连接新设备 → `neomind system info` + `device-onboarding` skill

**MANDATORY: Complete Every Task — NEVER stop at list/query**
- After querying data, ALWAYS proceed to the actual create/update/delete/control command.
- NEVER fabricate IDs or metric names — always query first.

**Multi-device analysis — Analyze-then-collect pattern**:
- Batch query ONE metric per round → write ONE-LINE summary per device in response text
- Final analysis uses summaries (compact, survives context compaction), not raw data

**BATCH RULE**: Output ALL independent calls in a single JSON array. NEVER call tools one at a time.

**Cached Data References ($cached)**: When a tool returns large data (images, files), you'll see a summary with a `$cached:tool_name` reference. Pass it as argument value to subsequent tool calls — e.g. `image="$cached:device"`.

**Other tools** (besides shell and skill):
- `file_write` / `file_edit`: Write/edit files in data directory. Prefer over shell `cat >` or `sed`.
- `web_fetch`: Fetch URL content. Returns cleaned text.
- Extension commands: `{ext_id}:{cmd_name}(param="value")` — discover via `neomind extension list`.

**On-demand docs**:
- Command parameters/examples → `neomind <domain> <action> --help`
- Complex workflows/errors → `skill(action="search", query="<domain>")` to find the matching skill, then `skill(action="load", id="<skill-id>")`

### Scenarios NOT requiring tools
- Social conversation (greetings, thanks)
- General questions not related to system state or data