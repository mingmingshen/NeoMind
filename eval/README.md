# NeoMind Chat Agent Eval System

A Claude-as-judge eval harness for the NeoMind chat AI agent. The Rust runner
spawns a real `neomind serve` subprocess per case (full isolation), runs the
agent in-process via `SessionManager::memory()`, captures full tool-call traces,
and a Claude Code session grades the output across 6 dimensions.

## Layout

```
eval/
├── README.md                   # this file
├── fixtures/                   # seed-default.json, seed-empty.json
├── smoke/                      # 5 sanity cases (good-001..003, bad-001..002)
├── cases/
│   ├── zh/<workflow>/*.json    # Chinese Tier 1 cases
│   └── en/<workflow>/*.json    # English Tier 1 cases (shared IDs)
├── runs/                       # auto-created per run (gitignored)
└── reports/                    # auto-created (gitignored)
```

Crates: `crates/eval-runner/` (Rust binary + lib).

## Quick Start

```bash
# 1. Build the runner
cargo build -p eval-runner --release

# 2. Validate every case (no server, no LLM)
cargo run -p eval-runner --release -- validate-all eval/cases

# 3. Run a single case (requires AGENT_LLM_* + neomind binary on PATH)
AGENT_LLM_API_KEY=sk-xxx \
AGENT_LLM_ENDPOINT=https://dashscope.aliyuncs.com/compatible-mode/v1 \
AGENT_LLM_MODEL=qwen-plus \
  cargo run -p eval-runner --release -- run-case eval/smoke/good-002.json

# 4. Run the full eval as Claude-judge (from Claude Code session)
/eval chat-eval
```

## Case Schema (spec §3)

```json
{
  "id": "device-create-temp-sensor",      // language-agnostic, NO zh-/en- prefix
  "lang": "zh",                            // "zh" | "en"
  "category": "device",                    // device|rule|agent|dashboard|transform|message
  "workflow": "device-onboarding",         // sub-folder name
  "scenario_type": "single_turn",          // "single_turn" | "multi_turn"
  "description": "...",                    // human-readable summary
  "setup": {
    "fixture": "seed-default",             // eval/fixtures/<name>.json
    "extras": {                            // case-specific add-ons
      "devices": [], "metrics": [], "rules": [], "agents": [],
      "transforms": [], "dashboards": [], "channels": []
    }
  },
  "turns": [{ "user": "..." }],
  "applies": ["tool_accuracy","task_completion"],   // dimensions to score
  "expectations": {
    "per_turn": ["Turn 1: ..."],           // length MUST equal turns
    "overall": "..."
  },
  "state_queries": [                       // optional — HTTP-verified post-conditions
    { "type": "device_exists", "params": { "id": "x" }, "expected": true }
  ]
}
```

## Dimensions & Weights (spec §5)

| Dimension | Weight |
|-----------|--------|
| tool_accuracy | 25% |
| task_completion | 25% |
| response_quality | 20% |
| context_retention | 15% (multi_turn only) |
| error_recovery | 15% |
| language_adherence | 5% |

Weights renormalize over `applies[]` subset.

## State Query Types (spec §4a)

`device_exists`, `device_count`, `rule_exists`, `rule_enabled`, `agent_exists`,
`agent_status`, `transform_exists`, `dashboard_exists`,
`dashboard_component_count`, `channel_exists`, `message_count`, `push_enabled`.

## Architecture

For each case:
1. Spawn `neomind serve --host 127.0.0.1 --port 0` with temp `NEOMIND_DATA_DIR`
   and CWD = tempdir (so `AuthState::new()`'s hardcoded `data/api_keys.redb`
   lands inside the tempdir).
2. Parse the bound port from merged stdout+stderr (banner uses `println!`).
3. Poll `/api/health` until ready.
4. Read the auto-created API key from `<tempdir>/data/api_keys.redb` via
   `neomind_cli_ops::auto_auth::read_default_api_key_from`.
5. Seed fixture + extras via HTTP POST.
6. Build `SessionManager::memory()` in-process, inject `CloudRuntime` via
   `set_custom_llm`, point the agent's shell tool at the test server via
   `NEOMIND_API_BASE`/`NEOMIND_API_KEY`.
7. For each turn: call `sm.process_message()` with 120s timeout, capture
   `AgentResponse.tool_calls` (full args + results).
8. Run state queries via HTTP GET.
9. Run `detect_suspected_fallback` (3 conditions: all turns empty+fast AND
   expectations mention a tool word).
10. Teardown: kill subprocess, drop tempdir.

## Two env-var groups (do not confuse!)

| Var | Purpose | Where |
|-----|---------|-------|
| `AGENT_LLM_API_KEY` / `AGENT_LLM_ENDPOINT` / `AGENT_LLM_MODEL` | Powers the chat agent | eval-runner process |
| `NEOMIND_API_BASE` / `NEOMIND_API_KEY` | Where the agent's shell tool hits | Set per-case from temp server |

The test server auto-derives its OWN API key — never set `NEOMIND_API_KEY`
yourself when running the runner.

## See Also

- Spec: `docs/superpowers/specs/2026-06-29-eval-system-design.md`
- Plan: `docs/superpowers/plans/2026-06-29-eval-system.md`
- Skill: `crates/eval-runner/skills/eval-chat.md` (the Claude-judge slash command)
