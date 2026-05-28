/**
 * Device lookup utilities.
 *
 * Extracted from useDataSource/helpers.ts for use across the entire codebase.
 * Eliminates 11+ occurrences of inline `devices.find(d => d.id === id || d.device_id === id)`.
 */

import type { Device } from '@/types'

/** Find a device by id or device_id. O(n). */
export function findDevice(devices: Device[], id: string | undefined): Device | undefined {
  if (!id) return undefined
  return devices.find(d => d.id === id || d.device_id === id)
}

/** Build a Map for O(1) device lookups by both id and device_id. */
export function buildDeviceMap(devices: Device[]): Map<string, Device> {
  const map = new Map<string, Device>()
  for (const d of devices) {
    if (d.id) map.set(d.id, d)
    if (d.device_id && d.device_id !== d.id) map.set(d.device_id, d)
  }
  return map
}
