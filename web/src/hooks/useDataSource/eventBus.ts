// ============================================================================
// Shared Event Bus — Single useEvents connection for ALL useDataSource hooks
// ============================================================================
// Previously each useDataSource hook created its own useEvents() → 20 hooks = 40
// setEvents() calls per incoming event. Now: ONE connection, fans out via callbacks.

export type DeviceEventCallback = (event: any) => void
export type ExtensionEventCallback = (event: any) => void

// Store both callback and the device IDs this listener cares about
// so the bus can filter before dispatching
interface DeviceListenerEntry {
  callback: DeviceEventCallback
  deviceIds: Set<string>  // empty = accept all
}
interface ExtensionListenerEntry {
  callback: ExtensionEventCallback
  extensionIds: Set<string>  // empty = accept all
}

const deviceEventListeners = new Map<string, DeviceListenerEntry>()
const extensionEventListeners = new Map<string, ExtensionListenerEntry>()

let deviceEventId = 0
let extensionEventId = 0

// Singleton connection state
let deviceConnectionCleanup: (() => void) | null = null
let extensionConnectionCleanup: (() => void) | null = null
let deviceRefCount = 0
let extensionRefCount = 0

// Pending events batched via RAF — single pass through all listeners per frame
let pendingDeviceEvents: any[] = []
let pendingExtensionEvents: any[] = []
let deviceRafId: number | null = null
let extensionRafId: number | null = null

function flushDeviceEvents() {
  deviceRafId = null
  if (pendingDeviceEvents.length === 0) return
  const events = pendingDeviceEvents
  pendingDeviceEvents = []
  for (const [, entry] of deviceEventListeners) {
    try {
      const { callback, deviceIds } = entry
      if (deviceIds.size === 0) {
        // Accept all
        for (const e of events) callback(e)
      } else {
        // Only forward events for relevant device IDs
        for (const e of events) {
          const d = (e as any).data || e
          const eid = d?.device_id
          if (!eid || deviceIds.has(eid)) callback(e)
        }
      }
    } catch { /* skip failing listeners */ }
  }
}

function flushExtensionEvents() {
  extensionRafId = null
  if (pendingExtensionEvents.length === 0) return
  const events = pendingExtensionEvents
  pendingExtensionEvents = []
  for (const [, entry] of extensionEventListeners) {
    try {
      const { callback, extensionIds } = entry
      if (extensionIds.size === 0) {
        for (const e of events) callback(e)
      } else {
        for (const e of events) {
          const d = (e as any).data || e
          const eid = d?.extension_id
          if (!eid || extensionIds.has(eid)) callback(e)
        }
      }
    } catch { /* skip failing listeners */ }
  }
}

/** Internal: shared device event connection — started by first registerDeviceListener */
function initSharedDeviceConnection() {
  // Dynamic import to avoid circular deps; called lazily from a React component
  import('@/lib/events').then((eventsLib) => {
    const getEventsConnection = eventsLib.getEventsConnection || (eventsLib as any).default?.getEventsConnection
    if (!getEventsConnection) return

    const connection = getEventsConnection('events-device-shared', { category: 'device' })

    const unsubEvent = connection.onEvent((event: any) => {
      // Batch events and flush once per frame
      pendingDeviceEvents.push(event)
      if (!deviceRafId) {
        deviceRafId = requestAnimationFrame(flushDeviceEvents)
      }
    })

    deviceConnectionCleanup = () => {
      unsubEvent()
    }
  }).catch(() => {})
}

function initSharedExtensionConnection() {
  import('@/lib/events').then((eventsLib) => {
    const getEventsConnection = eventsLib.getEventsConnection || (eventsLib as any).default?.getEventsConnection
    if (!getEventsConnection) return

    const connection = getEventsConnection('events-extension-shared', { category: 'extension' })

    const unsubEvent = connection.onEvent((event: any) => {
      pendingExtensionEvents.push(event)
      if (!extensionRafId) {
        extensionRafId = requestAnimationFrame(flushExtensionEvents)
      }
    })

    extensionConnectionCleanup = () => {
      unsubEvent()
    }
  }).catch(() => {})
}

export function registerDeviceListener(cb: DeviceEventCallback, relevantDeviceIds?: Set<string>): { unregister: () => void } {
  const id = `ds_${++deviceEventId}`
  deviceEventListeners.set(id, { callback: cb, deviceIds: relevantDeviceIds ?? new Set() })
  deviceRefCount++
  if (deviceRefCount === 1 && !deviceConnectionCleanup) {
    initSharedDeviceConnection()
  }
  return {
    unregister: () => {
      deviceEventListeners.delete(id)
      deviceRefCount--
      if (deviceRefCount === 0 && deviceConnectionCleanup) {
        deviceConnectionCleanup()
        deviceConnectionCleanup = null
      }
    }
  }
}

export function registerExtensionListener(cb: ExtensionEventCallback, relevantExtensionIds?: Set<string>): { unregister: () => void } {
  const id = `es_${++extensionEventId}`
  extensionEventListeners.set(id, { callback: cb, extensionIds: relevantExtensionIds ?? new Set() })
  extensionRefCount++
  if (extensionRefCount === 1 && !extensionConnectionCleanup) {
    initSharedExtensionConnection()
  }
  return {
    unregister: () => {
      extensionEventListeners.delete(id)
      extensionRefCount--
      if (extensionRefCount === 0 && extensionConnectionCleanup) {
        extensionConnectionCleanup()
        extensionConnectionCleanup = null
      }
    }
  }
}
