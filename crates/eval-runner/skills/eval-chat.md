---
id: eval-chat
name: Chat Agent Eval Runner
category: eval
origin: builtin
priority: 50
token_budget: 12000
triggers:
  keywords: [/eval chat-eval, eval, 评估, benchmark, chat eval, 评测]
  tool_target: []
anti_triggers:
  keywords: [device create, dashboard create, rule create, agent create]
---

# Chat Agent Eval Runner

Run the NeoMind chat agent eval system. You (Claude) are the **judge**.

> This skill is a dev tool, NOT a NeoMind agent capability. It is intentionally
> NOT registered in `neomind-agent/src/skills/registry.rs`. Run it from a Claude
> Code session at the NeoMind repo root.

## Invocation

`/eval chat-eval [--workflow device,rule] [--lang zh|en|both] [--case-id <id>] [--no-judge] [--report-name <name>]`

## Prerequisites

The eval-runner binary must be built. If missing:
```bash
cargo build -p eval-runner --release
```

Required env (the agent LLM — NOT the test server's auth):
- `AGENT_LLM_API_KEY` — API key for the chat LLM
- `AGENT_LLM_ENDPOINT` — OpenAI-compatible base URL (no `/chat/completions`)
- `AGENT_LLM_MODEL` — model name (e.g. `qwen3-32b`)
- Optional: `AGENT_LLM_TIMEOUT_SECS` (default 180)

The test server (one subprocess per case) auto-derives its own API key from the
temp data dir — you do NOT set `NEOMIND_API_KEY` for it.

## Your job

For each case file under `eval/cases/<lang>/<workflow>/`:

1. **Run** the case via Bash:
   ```
   cargo run -p eval-runner --release -- run-case eval/cases/zh/device/create-001.json
   ```
   Output is one JSON line: a `CaseRecord` with `turn_records`, `state_queries`,
   `suspected_fallback`. If `status` is non-null, the agent failed (no scores
   possible — emit a score line with `status` filled and no `scores`).

2. **Read** the case's `expectations.per_turn[]` and `expectations.overall` from
   the case file.

3. **Score** each dimension in the case's `applies[]` using the rubric below. Use
   your reasoning — do NOT call any external API.

4. **Emit** one JSONL score line into the run dir's `scores.jsonl`. Use a fresh
   run dir: `eval/runs/<timestamp>/` where timestamp = `date +%Y%m%d-%H%M%S`.

## Dimensions & Weights

| Dimension | Weight | Anchor 10 | Anchor 8 | Anchor 5 | Anchor 2 | Anchor 0 |
|-----------|--------|-----------|----------|----------|----------|----------|
| tool_accuracy | 25% | exact right tool + args | right tool, minor arg slip | wrong tool but recovered | wrong tool, no recovery | no tool call |
| task_completion | 25% | all expectations + state queries pass | main goal met, minor miss | partial | barely any | nothing done |
| response_quality | 20% | clear, structured, on-brand | clear but verbose | unclear in places | confusing | incoherent |
| context_retention | 15% | perfect recall across turns | minor lapse | one missed reference | multiple lapses | lost the thread (multi_turn only) |
| error_recovery | 15% | no errors, OR recovered gracefully | recovered with effort | recovery attempted | recovery failed | gave up |
| language_adherence | 5% | native-quality zh/en | fluent | minor cross-language leak | frequent leaks | wrong language |

If `applies[]` is a subset, weights renormalize over the present dimensions (see
`report.rs::aggregate`).

## Score Line Format (write into scores.jsonl)

```json
{
  "case_id": "device-create-001",
  "lang": "zh",
  "scores": { "tool_accuracy": 9, "task_completion": 10, "response_quality": 8 },
  "overall_reasoning": "Agent correctly called device create with the right id.",
  "judge": "claude-opus-4-6",
  "duration_ms": 1234,
  "suspected_fallback": false
}
```

If the agent failed (CaseRecord.status is non-null):
```json
{
  "case_id": "x", "lang": "zh", "scores": {},
  "overall_reasoning": "agent runtime_error: <message>",
  "judge": "claude-opus-4-6", "duration_ms": 0, "suspected_fallback": false,
  "status": "runtime_error", "error_type": "runtime_error", "message": "<orig>"
}
```

## After All Cases

Generate the grade card:
```
cargo run -p eval-runner --release -- report eval/runs/<ts>/scores.jsonl --out eval/runs/<ts>/grade-card.md
```

Print the grade to the user.

## Sanity checks

- **Malformed >5%**: Investigate JSONL corruption — your score lines must parse.
- **`suspected_fallback` rate > 10%**: The agent likely has a broken LLM config —
  double-check `AGENT_LLM_*`.
- **agent_errors > 20%**: Tests are catching real failures; investigate root cause
  before reporting scores.

## Flags

- `--no-judge`: Only run cases, dump `cases.jsonl`, don't score. Useful for
  debugging the runner without spending judge tokens.
- `--case-id X`: Run only one case (for triage).
