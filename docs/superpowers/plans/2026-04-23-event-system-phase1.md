# Event System Phase 1: Restore ExtensionOutput + Agent Extension Event Trigger

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-enable ExtensionOutput event publishing, add feedback loop prevention, and extend Agent event triggers to support extension metric events.

**Architecture:** Three-part fix: (1) Restore the disabled `publish_extension_metrics_safe` call, (2) Add `dispatch_event_excluding` to `EventDispatcher` + filter ExtensionOutput in subscription service, (3) Extend Agent executor with `check_and_trigger_extension_event` and hook it into the server's EventBus listener. Frontend gains `extension.metric` event type in Agent editor.

**Tech Stack:** Rust (Axum, tokio, parking_lot), React + TypeScript

---

## Context: Problem Chain

From `docs/superpowers/specs/2026-04-23-event-driven-architecture-analysis.md`:

```
Problem 1: ExtensionOutput event publishing disabled
    ↓ blocks
Problem 2: Agent cannot trigger on extension events
    ↓ blocks
Problem 3: Dashboard cannot filter extension metrics by dimension
    ↓ blocks
Problem 4: VLM Vision cannot distinguish image sources by ROI
```

This plan fixes Problems 1 and 2.

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/neomind-api/src/handlers/extensions.rs` | Modify | Re-enable ExtensionOutput publishing |
| `crates/neomind-core/src/extension/event_dispatcher.rs` | Modify | Add `dispatch_event_excluding` method |
| `crates/neomind-core/src/extension/extension_event_subscription.rs` | Modify | Filter ExtensionOutput to exclude source extension |
| `crates/neomind-agent/src/ai_agent/executor/context.rs` | Modify | Extend `EventTriggerData` with extension source |
| `crates/neomind-agent/src/ai_agent/executor/mod.rs` | Modify | Add `check_and_trigger_extension_event` + `matches_extension_event_filter` |
| `crates/neomind-api/src/server/types.rs` | Modify | Add ExtensionOutput handler in EventBus listener loop |
| `web/src/pages/agents-components/AgentEditorFullScreen.tsx` | Modify | Add `extension.metric` event type |
| `web/src/pages/agents-components/AgentLogicPreview.tsx` | Modify | Add extension event preview |

---

### Task 1: Re-enable ExtensionOutput Event Publishing

**Files:**
- Modify: `crates/neomind-api/src/handlers/extensions.rs`

**Context:**
`publish_extension_metrics_safe` (the timeout-protected wrapper) already exists. The call is commented out with "DISABLED: causes no reactor running crashes". The call site is inside an `async fn` handler, so the `_safe` wrapper will work correctly. The original crash was from a different code path.

- [ ] **Step 1: Un-comment the publishing call**

Find the commented-out block:
```rust
// DISABLED: Publish ExtensionOutput events - causes "no reactor running" crashes
// Event publishing will be re-enabled after fixing the Tokio runtime issue
// publish_extension_metrics_safe(&state, &id, &result).await;
```

Replace with:
```rust
// Re-enabled: ExtensionOutput event publishing for agent triggers and dashboard subscriptions
publish_extension_metrics_safe(&state, &id, &result).await;
```

- [ ] **Step 2: Build**

```bash
cargo build -p neomind-api
```

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-api/src/handlers/extensions.rs
git commit -m "feat(events): re-enable ExtensionOutput event publishing"
```

---

### Task 2: Add Feedback Loop Prevention for ExtensionOutput

**Files:**
- Modify: `crates/neomind-core/src/extension/event_dispatcher.rs`
- Modify: `crates/neomind-core/src/extension/extension_event_subscription.rs`

**Context:**
The `EventDispatcher` uses `parking_lot::RwLock` (NOT tokio) with fields: `subscriptions`, `in_process_extensions`, `isolated_event_senders`. The existing `dispatch_event` method checks subscription matching and dispatches to both isolated and in-process extensions.

The `ExtensionEventSubscriptionService::handle_event` currently only filters virtual DeviceMetric events. When ExtensionOutput events are re-enabled, they will be dispatched to ALL subscribed extensions. Extensions subscribing to `"all"` or `"Extension"` prefix will receive their own output, creating a feedback loop.

- [ ] **Step 2.1: Add `dispatch_event_excluding` to `EventDispatcher`**

In `event_dispatcher.rs`, after the existing `dispatch_event` method, add:

```rust
/// Dispatch an event to all subscribed extensions EXCEPT the specified one.
/// Used to prevent feedback loops when re-dispatching ExtensionOutput events.
pub async fn dispatch_event_excluding(
    &self,
    event_type: &str,
    payload: Value,
    exclude_extension_id: &str,
) {
    // Clone necessary data to avoid holding locks across await points
    let subscriptions = self.subscriptions.read().clone();
    let isolated_event_senders = self.isolated_event_senders.read().clone();

    for (extension_id, event_types) in subscriptions.iter() {
        // Skip the excluded extension
        if extension_id == exclude_extension_id {
            continue;
        }

        // Check subscription matching (same logic as dispatch_event)
        let should_receive = event_types.iter().any(|et| {
            if et == "all" { return true; }
            if et == event_type { return true; }
            if event_type.starts_with(&format!("{}::", et)) { return true; }
            if event_type.len() > et.len() && event_type.starts_with(et) { return true; }
            false
        });

        if should_receive {
            if let Some(sender) = isolated_event_senders.get(extension_id) {
                match sender.send((event_type.to_string(), payload.clone())).await {
                    Ok(_) => { trace!(extension_id = %extension_id, "Event sent to isolated extension (excluded source)"); }
                    Err(e) => { error!(extension_id = %extension_id, error = %e, "Failed to send event"); }
                }
                continue;
            }

            let extension_opt = {
                let in_process_extensions = self.in_process_extensions.read();
                in_process_extensions.get(extension_id).cloned()
            };

            if let Some(extension) = extension_opt {
                let ext_guard = extension.read().await;
                if let Err(e) = ext_guard.handle_event(event_type, &payload) {
                    error!(extension_id = %extension_id, error = %e, "Failed to handle event");
                }
            }
        }
    }
}
```

- [ ] **Step 2.2: Add ExtensionOutput filtering in `handle_event`**

In `extension_event_subscription.rs`, in `handle_event`, after the virtual metric check:

```rust
// Prevent feedback loops: skip dispatching ExtensionOutput events back to
// the extension that produced them.
if let NeoMindEvent::ExtensionOutput { ref extension_id, .. } = event {
    trace!(
        extension_id = %extension_id,
        "Dispatching ExtensionOutput event, excluding source extension"
    );
    let (event_type, payload) = Self::convert_to_extension_format(&event);
    event_dispatcher
        .dispatch_event_excluding(&event_type, payload, extension_id)
        .await;
    return;
}
```

- [ ] **Step 2.3: Build**

```bash
cargo build -p neomind-core
```

- [ ] **Step 2.4: Commit**

```bash
git add crates/neomind-core/src/extension/event_dispatcher.rs crates/neomind-core/src/extension/extension_event_subscription.rs
git commit -m "feat(events): prevent ExtensionOutput feedback loop with source exclusion dispatch"
```

---

### Task 3: Extend Agent Event Trigger for Extension Metrics

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor/context.rs` — Extend `EventTriggerData`
- Modify: `crates/neomind-agent/src/ai_agent/executor/mod.rs` — Add extension event methods

**Context:**
`EventTriggerData` currently has only `device_id`, `metric`, `value`, `timestamp` — no extension source info. `execute_agent` takes `(agent: AiAgent, event_data: Option<EventTriggerData>)`. We need to extend the struct to support extension events, then add matching logic.

- [ ] **Step 3.1: Extend `EventTriggerData`**

In `executor/context.rs`, add an `EventSource` enum and extend the struct:

```rust
/// Source of an event that triggers agent execution.
#[derive(Clone, Debug)]
pub enum EventSource {
    Device,
    Extension,
}

/// Event data for triggering agent execution.
#[derive(Clone, Debug)]
pub struct EventTriggerData {
    pub source: EventSource,
    /// Device ID (for Device source) or Extension ID (for Extension source)
    pub source_id: String,
    /// Metric name (for Device source) or output name (for Extension source)
    pub metric: String,
    pub value: MetricValue,
    pub timestamp: i64,
}
```

**IMPORTANT**: This is a breaking change. Search for all usages of `EventTriggerData` and update them:
- Constructor calls need `source: EventSource::Device`
- Pattern matches on fields need `source_id` instead of `device_id`

- [ ] **Step 3.2: Update `check_and_trigger_event` to use new struct**

Find all places that construct `EventTriggerData` and update field names:
- `device_id` → `source_id`
- Add `source: EventSource::Device`

- [ ] **Step 3.3: Add `check_and_trigger_extension_event` method**

In `executor/mod.rs`, add alongside `check_and_trigger_event`:

```rust
/// Check if an extension metric event should trigger any agents.
pub async fn check_and_trigger_extension_event(
    &self,
    extension_id: String,
    output_name: String,
    value: &MetricValue,
) -> AgentResult<()> {
    let agents = self.agent_store.list_agents().await?;

    for agent in agents {
        if agent.schedule.schedule_type != ScheduleType::Event {
            continue;
        }

        if self.matches_extension_event_filter(&agent, &extension_id, &output_name) {
            info!(
                agent_id = %agent.id,
                extension_id = %extension_id,
                output_name = %output_name,
                "Extension event matched agent filter, triggering execution"
            );

            // Deduplication check (reuse existing pattern)
            let key = (agent.id.clone(), extension_id.clone());
            {
                let recent = self.recent_executions.read();
                if let Some(&ts) = recent.get(&key) {
                    let elapsed = chrono::Utc::now().timestamp() - ts;
                    if elapsed < self.dedup_window_seconds as i64 {
                        trace!(agent_id = %agent.id, "Skipping duplicate extension event trigger");
                        continue;
                    }
                }
            }

            let event_data = EventTriggerData {
                source: EventSource::Extension,
                source_id: extension_id.clone(),
                metric: output_name.clone(),
                value: value.clone(),
                timestamp: chrono::Utc::now().timestamp(),
            };

            self.execute_agent(agent, Some(event_data)).await?;
        }
    }

    Ok(())
}
```

- [ ] **Step 3.4: Add `matches_extension_event_filter` method**

```rust
/// Check if an extension metric event matches an agent's resource filter.
fn matches_extension_event_filter(
    &self,
    agent: &AiAgent,
    extension_id: &str,
    output_name: &str,
) -> bool {
    let resources = match &agent.resources {
        Some(r) if !r.is_empty() => r,
        _ => return true, // No resources = match all (backward compat)
    };

    let ext_metric_id = format!("{}:{}", extension_id, output_name);

    for r in resources {
        if r.resource_type == ResourceType::ExtensionMetric {
            // Match: "*" (all), "extension_id" (all outputs), "extension_id:output" (specific)
            if r.resource_id == "*" || r.resource_id == extension_id || r.resource_id == ext_metric_id {
                return true;
            }
        }
    }

    false
}
```

- [ ] **Step 3.5: Build**

```bash
cargo build -p neomind-agent
```

- [ ] **Step 3.6: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor/context.rs crates/neomind-agent/src/ai_agent/executor/mod.rs
git commit -m "feat(agent): add extension metric event trigger with EventSource enum"
```

---

### Task 4: Hook ExtensionOutput Events into Server EventBus Listener

**Files:**
- Modify: `crates/neomind-api/src/server/types.rs`

**Context:**
The actual EventBus subscription for agent triggers is in `types.rs` inside `AppState::start_agent_event_listener()` (or similar method). It's a `tokio::spawn` block that subscribes to EventBus and dispatches `DeviceMetric` events to `executor.check_and_trigger_event()`. The agent module (`ai_agent/mod.rs`) does NOT have its own EventBus subscription.

- [ ] **Step 4.1: Find the EventBus listener loop**

In `types.rs`, find the `tokio::spawn` block that contains:
```rust
if let neomind_core::NeoMindEvent::DeviceMetric { device_id, metric, value, .. } = event {
    executor.check_and_trigger_event(device_id, &metric, &value).await
}
```

This is the block at approximately lines 1941-1965.

- [ ] **Step 4.2: Add ExtensionOutput handler in the same loop**

The `while let Some((event, _metadata)) = rx.recv().await` loop currently only has a single `if let` for `DeviceMetric`. Change it to handle both event types:

```rust
while let Some((event, _metadata)) = rx.recv().await {
    match event {
        neomind_core::NeoMindEvent::DeviceMetric {
            device_id, metric, value, ..
        } => {
            if let Err(e) = executor
                .check_and_trigger_event(device_id, &metric, &value)
                .await
            {
                tracing::debug!("No agent triggered for device event: {}", e);
            }
        }
        neomind_core::NeoMindEvent::ExtensionOutput {
            extension_id, output_name, value, ..
        } => {
            if let Err(e) = executor
                .check_and_trigger_extension_event(extension_id, output_name, &value)
                .await
            {
                tracing::debug!("No agent triggered for extension event: {}", e);
            }
        }
        _ => {} // Ignore other events
    }
}
```

- [ ] **Step 4.3: Build**

```bash
cargo build -p neomind-api
```

- [ ] **Step 4.4: Commit**

```bash
git add crates/neomind-api/src/server/types.rs
git commit -m "feat(agent): hook ExtensionOutput events into agent trigger EventBus listener"
```

---

### Task 5: Frontend - Add Extension Event Trigger in Agent Editor

**Files:**
- Modify: `web/src/pages/agents-components/AgentEditorFullScreen.tsx`
- Modify: `web/src/pages/agents-components/AgentLogicPreview.tsx`
- Modify: `web/src/i18n/locales/en/agents.json` (or equivalent agent locale file)
- Modify: `web/src/i18n/locales/zh/agents.json` (or equivalent agent locale file)

**Context:**
The Agent editor has `eventConfig` with type `'device.metric' | 'manual'`. We add `'extension.metric'`. The save logic constructs an `event_filter` JSON — we need to use `event_type` key (not `source`) for consistency with existing parsing.

**IMPORTANT**: The frontend must also handle loading back an existing agent's config. When editing an agent that was saved with extension event trigger, the `eventConfig` state must be restored correctly.

- [ ] **Step 5.1: Update event config type in both files**

`AgentEditorFullScreen.tsx`:
```typescript
const [eventConfig, setEventConfig] = useState<{
  type: 'device.metric' | 'extension.metric' | 'manual'
  deviceId?: string
  extensionId?: string
}>({ type: 'device.metric', deviceId: 'all' })
```

`AgentLogicPreview.tsx`:
```typescript
eventConfig?: {
  type: 'device.metric' | 'extension.metric' | 'manual'
  deviceId?: string
  extensionId?: string
}
```

- [ ] **Step 5.2: Add extension.metric button + extension selector UI**

In the event type selector section (where the `device.metric` and `manual` buttons are), add a third button:

```tsx
<Button
  type="button"
  variant="ghost"
  onClick={() => setEventConfig(prev => ({ ...prev, type: 'extension.metric' }))}
  className={cn(
    isMobile ? "px-4 py-3 text-base flex-1 justify-center" : "px-3 py-1.5 text-sm",
    eventConfig.type === 'extension.metric' ? "bg-primary text-primary-foreground" : "bg-background hover:bg-muted"
  )}
>
  <Puzzle className={cn(isMobile ? "h-4 w-4" : "h-3.5 w-3.5")} />
  {!isMobile && <span className="ml-1">{tAgent('creator.eventTrigger.extensionMetric')}</span>}
</Button>
```

After the `device.metric` device selector section, add the extension selector:

```tsx
{eventConfig.type === 'extension.metric' && (
  <div className={cn("flex items-center gap-3", isMobile ? "flex-col items-start gap-3" : "")}>
    <Select
      value={eventConfig.extensionId || 'all'}
      onValueChange={(val) => setEventConfig(prev => ({ ...prev, extensionId: val }))}
    >
      <SelectTrigger className={cn("w-[200px]", isMobile && "w-full")}>
        <SelectValue placeholder={tAgent('creator.eventTrigger.selectExtension')} />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">{tAgent('creator.eventTrigger.allExtensions')}</SelectItem>
        {extensions.map(ext => (
          <SelectItem key={ext.id} value={ext.id}>{ext.name || ext.id}</SelectItem>
        ))}
      </SelectContent>
    </Select>
  </div>
)}
```

Note: `extensions` state should already exist in the editor component (loaded from API).

- [ ] **Step 5.3: Update save logic**

Find the event filter construction for `scheduleType === 'event'` and add:

```typescript
if (eventConfig.type === 'extension.metric') {
  finalScheduleType = 'event'
  eventFilter = JSON.stringify({
    event_type: 'extension.metric',
    extension_id: eventConfig.extensionId || 'all',
  })
  // Add ExtensionMetric resource
  const hasExtMetric = finalResources.some(r => r.type === 'extension_metric')
  if (!hasExtMetric && eventConfig.extensionId && eventConfig.extensionId !== 'all') {
    finalResources.push({
      type: 'extension_metric',
      id: eventConfig.extensionId,
      name: eventConfig.extensionId,
      config: {},
    })
  }
}
```

- [ ] **Step 5.4: Update load-back logic**

When loading an existing agent for editing, the editor parses `event_filter` JSON to restore `eventConfig` state. Find this code and add:

```typescript
// In the agent load-back logic where eventConfig is restored from saved data:
if (filterObj.event_type === 'extension.metric') {
  setEventConfig({
    type: 'extension.metric',
    extensionId: filterObj.extension_id || 'all',
  })
}
```

- [ ] **Step 5.5: Update AgentLogicPreview**

```typescript
case 'event':
  if (props.eventConfig?.type === 'manual') {
    return t('preview.trigger.manual')
  }
  if (props.eventConfig?.type === 'extension.metric') {
    return t('preview.trigger.extensionEvent', {
      extension: props.eventConfig?.extensionId || 'all',
    })
  }
  return t('preview.trigger.event', { device: props.eventConfig?.deviceId || 'all' })
```

Also update the summary line:
```typescript
if (scheduleType === 'event' && eventConfig.type === 'extension.metric')
  parts.push(`triggers on ${eventConfig.extensionId === 'all' ? 'any' : eventConfig.extensionId} extension metric updates`)
```

- [ ] **Step 5.6: Add i18n keys**

Find the agent locale files (search for existing `creator.eventTrigger` or `creator.basicInfo` keys) and add:

**English:**
```json
"creator": {
  "eventTrigger": {
    "extensionMetric": "Extension Metric",
    "allExtensions": "All Extensions",
    "selectExtension": "Select Extension"
  }
},
"preview": {
  "trigger": {
    "extensionEvent": "Triggers on {{extension}} extension metrics"
  }
}
```

**Chinese:**
```json
"creator": {
  "eventTrigger": {
    "extensionMetric": "扩展指标",
    "allExtensions": "所有扩展",
    "selectExtension": "选择扩展"
  }
},
"preview": {
  "trigger": {
    "extensionEvent": "当 {{extension}} 扩展指标更新时触发"
  }
}
```

- [ ] **Step 5.7: Build and verify**

```bash
cd web && npm run build
```

- [ ] **Step 5.8: Commit**

```bash
git add web/src/pages/agents-components/ web/src/i18n/
git commit -m "feat(agent-ui): add extension metric event trigger in agent editor"
```

---

### Task 6: Full Build + Smoke Test

- [ ] **Step 6.1: Full workspace build**

```bash
cargo build
```

- [ ] **Step 6.2: Run existing tests**

```bash
cargo test --workspace --exclude neomind-cli
```

- [ ] **Step 6.3: Start server and verify**

```bash
cargo run -p neomind-cli -- serve
```

Check logs for:
1. "Extension event listener started" — confirms the new handler is active
2. No panic/error when extensions produce metrics
3. ExtensionOutput events appear in EventBus trace logs

- [ ] **Step 6.4: Manual verification flow**

1. Ensure an extension is running that produces metrics
2. Verify ExtensionOutput events are published (check logs)
3. Verify no feedback loop (extension should NOT receive its own ExtensionOutput)
4. Create Agent with Event schedule → Extension Metric trigger
5. Trigger extension to produce metrics
6. Verify Agent is triggered and executes

---

## Summary of Changes

| Component | Change | Impact |
|-----------|--------|--------|
| `extensions.rs` | Un-comment `publish_extension_metrics_safe` | Extension metrics flow through EventBus |
| `event_dispatcher.rs` | Add `dispatch_event_excluding` | Supports filtering out source extension |
| `extension_event_subscription.rs` | Filter ExtensionOutput via exclusion | No feedback loops |
| `executor/context.rs` | Add `EventSource` enum, extend `EventTriggerData` | Unified event data for both device and extension |
| `executor/mod.rs` | Add `check_and_trigger_extension_event` | Agents trigger on extension data |
| `server/types.rs` | Add ExtensionOutput handler in EventBus loop | Server routes extension events to executor |
| Agent Editor UI | Add `extension.metric` event type | Users can configure extension-triggered agents |

## What This Unblocks

- Agent responds to extension metric updates (e.g., VLM triggered when YOLO detects objects)
- Dashboard subscriptions can receive extension events (Phase 2 will add UI)
- Cross-component communication foundation (Phase 3 will add dimensional filtering)
