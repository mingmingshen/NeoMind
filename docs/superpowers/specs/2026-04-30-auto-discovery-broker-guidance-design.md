# Auto Discovery Broker Guidance

## Problem

When a user opens the **Drafts** (Auto Discovery) tab in the Devices page, they see an empty table with no explanation. Auto discovery depends on MQTT brokers being configured, but there's no guidance to help users set one up.

## Solution

Show a contextual empty state in `PendingDevicesList.tsx` when no draft devices exist. The empty state varies based on MQTT broker configuration status:

- **No external broker configured**: Empty state with CTA button linking to Settings → Device Connections
- **Broker configured, no drafts yet**: Simpler empty state explaining that devices will appear when detected

## Design Details

### Files to Modify

1. **`web/src/pages/devices/PendingDevicesList.tsx`** — Add broker status check and conditional empty states
2. **`web/src/i18n/locales/en/devices.json`** — English i18n strings
3. **`web/src/i18n/locales/zh/devices.json`** — Chinese i18n strings

### Logic Flow

```
activeDrafts.length === 0 && !loading
  → fetch api.getMqttStatus()
  → if no external_brokers (or empty array)
    → show EmptyState with "No Broker" messaging + CTA to /settings?tab=connections
  → else
    → show EmptyState with "No Discovered Devices" messaging
```

### Empty State Variants

**No broker configured:**
- Icon: `settings` (gear icon)
- Title: "No MQTT Broker Configured"
- Description: "Auto discovery requires an MQTT broker to detect new devices. Add a broker in Settings to get started."
- CTA: "Go to Settings" → navigates to `/settings?tab=connections`

**Broker configured, no drafts:**
- Icon: `inbox`
- Title: "No Discovered Devices"
- Description: "New devices will appear here automatically when they connect to your MQTT broker."

### i18n Keys

```json
{
  "pending": {
    "noBrokerTitle": "No MQTT Broker Configured",
    "noBrokerDesc": "Auto discovery requires an MQTT broker to detect new devices. Add a broker in Settings to get started.",
    "goToSettings": "Go to Settings",
    "noDraftsTitle": "No Discovered Devices",
    "noDraftsDesc": "New devices will appear here automatically when they connect to your MQTT broker."
  }
}
```

## Constraints

- Use existing `EmptyState` component — no new components
- Broker status fetched only when drafts list is empty (not on every render)
- Navigation uses existing `useNavigate()` from react-router-dom
- Follow project design token conventions (no hardcoded colors)
