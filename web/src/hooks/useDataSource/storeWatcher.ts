/**
 * Shared Store Watcher — Single subscription for ALL useDataSource hooks
 *
 * Previously each hook created its own useStore.subscribe → 20 hooks = 20
 * synchronous callbacks on every store change, each building Maps and doing
 * comparisons. Now: ONE subscription, detects which devices changed, then
 * notifies only the hooks that care about those devices.
 */

import type { Device } from '@/types'
import type { NeoMindStore } from '@/store'
import { useStore } from '@/store'

// ============================================================================
// Types
// ============================================================================

export type StoreChangeCallback = (changedDeviceIds: Set<string>, devices: Device[], deviceMap: Map<string, Device>) => void

interface StoreWatcherEntry {
  callback: StoreChangeCallback
  deviceIds: Set<string>  // device IDs this hook cares about
}

// ============================================================================
// Module State
// ============================================================================

const storeWatchers = new Map<string, StoreWatcherEntry>()
let storeWatcherId = 0
let storeSubscriptionUnsub: (() => void) | null = null
let storeWatcherRefCount = 0

// Shared deviceMap cache — avoids rebuilding Map for every readDataFromStore call
let cachedDeviceMap: Map<string, Device> | null = null
let cachedDeviceMapSource: Device[] | null = null

// Cache previous devices array reference to skip unchanged updates
let prevDevicesRef: Device[] | null = null
// Throttle: max one notification per RAF frame
let storeWatcherRafScheduled = false

// Shared devicesLoading watcher state
const devicesLoadingCallbacks = new Map<string, () => void>()
let devicesLoadingUnsub: (() => void) | null = null
let devicesLoadingRefCount = 0

// ============================================================================
// Shared Device Map Cache
// ============================================================================

export function getSharedDeviceMap(devices: Device[]): Map<string, Device> {
  if (devices === cachedDeviceMapSource && cachedDeviceMap) return cachedDeviceMap
  const map = new Map<string, Device>()
  for (const d of devices) {
    map.set(d.id, d)
    if (d.device_id) map.set(d.device_id, d)
  }
  cachedDeviceMap = map
  cachedDeviceMapSource = devices
  return map
}

// ============================================================================
// Store Watcher — Device Changes
// ============================================================================

function startStoreWatcher() {
  if (storeSubscriptionUnsub) return  // Already running

  storeSubscriptionUnsub = useStore.subscribe((state: NeoMindStore) => {
    // Skip if devices slice hasn't changed
    if (state.devices === prevDevicesRef) return
    const newDevices = state.devices

    // Batch into RAF to avoid synchronous processing storm
    if (!storeWatcherRafScheduled) {
      storeWatcherRafScheduled = true
      requestAnimationFrame(() => {
        storeWatcherRafScheduled = false
        dispatchStoreChanges(newDevices)
      })
    }
  })
}

function dispatchStoreChanges(newDevices: Device[]) {
  const changed = new Set<string>()

  // Build map of current devices for O(1) lookup
  const currentMap = new Map<string, { device: Device; index: number }>()
  for (let i = 0; i < newDevices.length; i++) {
    const d = newDevices[i]
    currentMap.set(d.id, { device: d, index: i })
    if (d.device_id) currentMap.set(d.device_id, { device: d, index: i })
  }

  // Build map of previous devices
  const prevArr = prevDevicesRef || []
  const prevMap = new Map<string, Device>()
  for (const d of prevArr) {
    prevMap.set(d.id, d)
    if (d.device_id) prevMap.set(d.device_id, d)
  }

  // Detect which devices changed their current_values
  for (const [key, entry] of currentMap) {
    const prevDevice = prevMap.get(key)
    if (!prevDevice) {
      changed.add(entry.device.id)
      if (entry.device.device_id) changed.add(entry.device.device_id)
    } else if (prevDevice.current_values !== entry.device.current_values) {
      changed.add(entry.device.id)
      if (entry.device.device_id) changed.add(entry.device.device_id)
    } else if (prevDevice.status !== entry.device.status ||
               prevDevice.online !== entry.device.online ||
               prevDevice.last_seen !== entry.device.last_seen) {
      changed.add(entry.device.id)
      if (entry.device.device_id) changed.add(entry.device.device_id)
    }
  }

  // Check for removed devices
  for (const [key] of prevMap) {
    if (!currentMap.has(key)) changed.add(key)
  }

  prevDevicesRef = newDevices

  // Build deviceMap once — shared across all watchers
  const deviceMap = new Map<string, Device>()
  for (const d of newDevices) {
    deviceMap.set(d.id, d)
    if (d.device_id) deviceMap.set(d.device_id, d)
  }

  // Dispatch to relevant watchers immediately (single diff, shared deviceMap)
  const dispatchStart = performance.now()
  let dispatchCount = 0
  for (const [, entry] of storeWatchers) {
    try {
      const { callback, deviceIds } = entry
      let relevant = deviceIds.size === 0
      if (!relevant) {
        for (const did of deviceIds) {
          if (changed.has(did)) { relevant = true; break }
        }
      }
      if (relevant) {
        callback(changed, newDevices, deviceMap)
        dispatchCount++
      }
    } catch { /* skip failing watchers */ }
  }
  const dispatchMs = performance.now() - dispatchStart
  if (dispatchMs > 16) {
    console.warn(`[StoreWatcher] dispatch took ${dispatchMs.toFixed(1)}ms for ${dispatchCount}/${storeWatchers.size} watchers, ${changed.size} devices changed`)
  }
}

export function registerStoreWatcher(
  cb: StoreChangeCallback,
  relevantDeviceIds: Set<string>
): { unregister: () => void } {
  const id = `sw_${++storeWatcherId}`
  storeWatchers.set(id, { callback: cb, deviceIds: relevantDeviceIds })
  storeWatcherRefCount++
  if (storeWatcherRefCount === 1) {
    prevDevicesRef = useStore.getState().devices
    startStoreWatcher()
  }
  return {
    unregister: () => {
      storeWatchers.delete(id)
      storeWatcherRefCount--
      if (storeWatcherRefCount === 0 && storeSubscriptionUnsub) {
        storeSubscriptionUnsub()
        storeSubscriptionUnsub = null
        prevDevicesRef = null
      }
    }
  }
}

// ============================================================================
// Shared devicesLoading Watcher — replaces per-hook subscriptions
// ============================================================================

export function registerDevicesLoadingWatcher(cb: () => void): { unregister: () => void } {
  const id = `dl_${devicesLoadingCallbacks.size}_${Date.now()}`
  devicesLoadingCallbacks.set(id, cb)
  devicesLoadingRefCount++
  if (devicesLoadingRefCount === 1) {
    let prevLoading = useStore.getState().devicesLoading
    devicesLoadingUnsub = useStore.subscribe((state: NeoMindStore) => {
      if (state.devicesLoading === prevLoading) return
      prevLoading = state.devicesLoading
      if (!state.devicesLoading) {
        for (const [, callback] of devicesLoadingCallbacks) callback()
      }
    })
  }
  return {
    unregister: () => {
      devicesLoadingCallbacks.delete(id)
      devicesLoadingRefCount--
      if (devicesLoadingRefCount === 0 && devicesLoadingUnsub) {
        devicesLoadingUnsub()
        devicesLoadingUnsub = null
      }
    }
  }
}
