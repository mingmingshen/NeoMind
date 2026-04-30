# Auto Discovery Broker Guidance — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show contextual empty states in the Auto Discovery (Drafts) tab that guide users to add an MQTT broker when none is configured.

**Architecture:** Check MQTT broker status via `api.getMqttStatus()` when the drafts list is empty, then render either a "no broker" empty state with a CTA to Settings, or a "no drafts yet" empty state.

**Tech Stack:** React, react-i18next, react-router-dom, existing EmptyState component

---

### Task 1: Add i18n strings

**Files:**
- Modify: `web/src/i18n/locales/en/devices.json` (after line 580, inside `"pending"` section)
- Modify: `web/src/i18n/locales/zh/devices.json` (after the corresponding line, inside `"pending"` section)

- [ ] **Step 1: Add English strings**

In `web/src/i18n/locales/en/devices.json`, after the `"hours"` key (line 580), add these new keys inside the `"pending"` object:

```json
    "noBrokerTitle": "No MQTT Broker Configured",
    "noBrokerDesc": "Auto discovery requires an MQTT broker to detect new devices. Add a broker in Settings to get started.",
    "goToSettings": "Go to Settings",
    "noDraftsTitle": "No Discovered Devices",
    "noDraftsDesc": "New devices will appear here automatically when they connect to your MQTT broker.",
```

Insert after line 580 (`"hours": "hours",`) and before line 581 (`"noDeviceTypes"`).

- [ ] **Step 2: Add Chinese strings**

In `web/src/i18n/locales/zh/devices.json`, after the `"hours"` key (line 580), add these new keys inside the `"pending"` object:

```json
    "noBrokerTitle": "未配置 MQTT Broker",
    "noBrokerDesc": "自动发现需要 MQTT Broker 来检测新设备。请在设置中添加 Broker 以开始使用。",
    "goToSettings": "前往设置",
    "noDraftsTitle": "暂无发现的设备",
    "noDraftsDesc": "当新设备连接到您的 MQTT Broker 时，将自动显示在此处。",
```

- [ ] **Step 3: Verify build**

Run: `cd web && npx tsc --noEmit`
Expected: No type errors

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/locales/en/devices.json web/src/i18n/locales/zh/devices.json
git commit -m "feat(i18n): add auto discovery broker guidance strings"
```

---

### Task 2: Add broker status check and contextual empty state

**Files:**
- Modify: `web/src/pages/devices/PendingDevicesList.tsx`

- [ ] **Step 1: Add imports**

Add these imports at the top of the file:

- `useNavigate` from `react-router-dom` — add to a new import statement (there's no existing react-router-dom import in this file)
- `EmptyState` from `@/components/shared` — add to the existing import on line 5

After editing, line 1 changes to:
```tsx
import { useState, useEffect, useCallback, useMemo } from "react"
```

Add new import after line 2 (`useTranslation`):
```tsx
import { useNavigate } from "react-router-dom"
```

Modify line 5 to include `EmptyState`:
```tsx
import { ResponsiveTable, EmptyState } from "@/components/shared"
```

- [ ] **Step 2: Add broker state and navigate hook**

Inside the component function body, after the existing state declarations (after line 54 `const [loading, setLoading] = useState(true)`), add:

```tsx
  const navigate = useNavigate()
  const [hasBroker, setHasBroker] = useState<boolean | null>(null)
```

- [ ] **Step 3: Add broker status fetch effect**

After the existing `fetchDrafts` callback (after the `finally { setLoading(false) }` block, around line 150), add:

```tsx
  // Check broker status when drafts are empty
  useEffect(() => {
    if (loading || activeDrafts.length > 0) return

    let cancelled = false
    const checkBroker = async () => {
      try {
        const status = await api.getMqttStatus()
        if (!cancelled) {
          setHasBroker(!!status.external_brokers && status.external_brokers.length > 0)
        }
      } catch {
        if (!cancelled) setHasBroker(false)
      }
    }
    checkBroker()
    return () => { cancelled = true }
  }, [loading, activeDrafts.length])
```

- [ ] **Step 4: Replace the emptyState prop**

Replace lines 470-474 (the current `emptyState` prop of `ResponsiveTable`):

Old:
```tsx
        emptyState={
          <div className="flex items-center justify-center py-12">
            <p className="text-muted-foreground">{t('devices:pending.noPending')}</p>
          </div>
        }
```

New:
```tsx
        emptyState={
          hasBroker === false ? (
            <EmptyState
              icon="settings"
              title={t('devices:pending.noBrokerTitle')}
              description={t('devices:pending.noBrokerDesc')}
              action={{
                label: t('devices:pending.goToSettings'),
                onClick: () => navigate('/settings?tab=connections'),
              }}
            />
          ) : (
            <EmptyState
              icon="inbox"
              title={t('devices:pending.noDraftsTitle')}
              description={t('devices:pending.noDraftsDesc')}
            />
          )
        }
```

Note: `hasBroker === false` means we confirmed no broker exists (distinct from `null` which means we haven't checked yet). When `hasBroker` is `null` (still loading broker status) or `true`, we show the simpler "no drafts" state.

- [ ] **Step 5: Verify build**

Run: `cd web && npx tsc --noEmit`
Expected: No type errors

- [ ] **Step 6: Manual verification**

Run: `cd web && npm run dev`

1. Navigate to Devices → Pending Devices tab
2. Verify: when no external broker is configured, see "No MQTT Broker Configured" empty state with "Go to Settings" button
3. Click "Go to Settings" → should navigate to `/settings?tab=connections`
4. If a broker IS configured and no drafts exist, verify the "No Discovered Devices" empty state appears

- [ ] **Step 7: Commit**

```bash
git add web/src/pages/devices/PendingDevicesList.tsx
git commit -m "feat: guide users to add MQTT broker from auto discovery empty state"
```
