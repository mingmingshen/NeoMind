/**
 * Device Slice
 *
 * Handles device state, device types, discovery, and telemetry.
 */

import type { StateCreator } from 'zustand'
import type {
  DeviceState,
  TelemetryState,
} from '../types'
import type {
  Device,
  DeviceType,
  DiscoveredDevice,
  AdapterPluginDto,
  AddDeviceRequest,
  HassDiscoveryStatus,
  HassDiscoveredDevice,
  HassDiscoveryRequest,
  HassDiscoveryResponse,
} from '@/types'
import { api } from '@/lib/api'

export interface DeviceSlice extends DeviceState, TelemetryState {
  // Device Adapter State
  deviceAdapters: AdapterPluginDto[]
  deviceAdaptersLoading: boolean

  // HASS Discovery State
  hassDiscoveryStatus: HassDiscoveryStatus | null
  hassDiscoveredDevices: HassDiscoveredDevice[]
  hassDiscovering: boolean

  // Actions
  setSelectedDevice: (device: Device | null) => void
  setSelectedDeviceId: (id: string | null) => void
  setAddDeviceDialogOpen: (open: boolean) => void
  setAddDeviceTypeDialogOpen: (open: boolean) => void
  setDeviceDetailsDialogOpen: (open: boolean) => void

  fetchDevices: () => Promise<void>
  fetchDeviceTypes: () => Promise<void>
  addDevice: (request: AddDeviceRequest) => Promise<boolean>
  deleteDevice: (id: string) => Promise<boolean>
  sendCommand: (deviceId: string, command: string, params?: Record<string, unknown>) => Promise<boolean>

  addDeviceType: (definition: DeviceType) => Promise<boolean>
  deleteDeviceType: (id: string) => Promise<boolean>
  validateDeviceType: (definition: DeviceType) => Promise<{ valid: boolean; errors?: string[]; warnings?: string[]; message: string }>
  generateMDL: (req: { device_name: string; description?: string; uplink_example: string; downlink_example?: string }) => Promise<DeviceType>

  fetchDeviceDetails: (id: string) => Promise<Device | null>
  fetchDeviceTypeDetails: (deviceType: string) => Promise<DeviceType | null>

  discoverDevices: (host: string, ports?: number[], timeoutMs?: number) => Promise<void>
  setDiscoveredDevices: (devices: DiscoveredDevice[]) => void

  fetchTelemetryData: (deviceId: string, metric?: string, start?: number, end?: number, limit?: number) => Promise<void>
  fetchTelemetrySummary: (deviceId: string, hours?: number) => Promise<void>
  fetchCommandHistory: (deviceId: string, limit?: number) => Promise<void>

  // Device Adapter Actions
  fetchDeviceAdapters: () => Promise<void>

  // HASS Discovery Actions
  fetchHassDiscoveryStatus: () => Promise<void>
  fetchHassDiscoveredDevices: () => Promise<void>
  startHassDiscovery: (req: HassDiscoveryRequest) => Promise<HassDiscoveryResponse>
  stopHassDiscovery: () => Promise<void>
  registerHassDevice: (deviceId: string) => Promise<boolean>
  unregisterHassDevice: (deviceId: string) => Promise<boolean>
  clearHassDiscoveredDevices: () => void
}

export const createDeviceSlice: StateCreator<
  DeviceSlice,
  [],
  [],
  DeviceSlice
> = (set, get) => ({
  // Initial state
  devices: [],
  deviceTypes: [],
  selectedDevice: null,
  selectedDeviceId: null,
  deviceDetails: null,
  deviceTypeDetails: null,
  discovering: false,
  discoveredDevices: [],
  devicesLoading: false,
  deviceTypesLoading: false,
  addDeviceDialogOpen: false,
  addDeviceTypeDialogOpen: false,
  deviceDetailsDialogOpen: false,

  // Device Adapters state
  deviceAdapters: [],
  deviceAdaptersLoading: false,

  // HASS Discovery state
  hassDiscoveryStatus: null,
  hassDiscoveredDevices: [],
  hassDiscovering: false,

  // Telemetry state
  telemetryData: null,
  telemetrySummary: null,
  commandHistory: null,
  telemetryLoading: false,

  // Dialog actions
  setSelectedDevice: (device) => set({ selectedDevice: device }),
  setSelectedDeviceId: (id) => set({ selectedDeviceId: id }),
  setAddDeviceDialogOpen: (open) => set({ addDeviceDialogOpen: open }),
  setAddDeviceTypeDialogOpen: (open) => set({ addDeviceTypeDialogOpen: open }),
  setDeviceDetailsDialogOpen: (open) => set({ deviceDetailsDialogOpen: open }),

  // Device CRUD
  fetchDevices: async () => {
    set({ devicesLoading: true })
    try {
      const data = await api.getDevices()
      set({ devices: data.devices || [] })
    } catch (error) {
      if ((error as Error).message === 'UNAUTHORIZED') {
        // Will be handled by auth slice
      }
      console.error('Failed to fetch devices:', error)
      set({ devices: [] })
    } finally {
      set({ devicesLoading: false })
    }
  },

  fetchDeviceTypes: async () => {
    set({ deviceTypesLoading: true })
    try {
      const data = await api.getDeviceTypes()
      set({ deviceTypes: data.device_types || [] })
    } catch (error) {
      if ((error as Error).message === 'UNAUTHORIZED') {
        // Will be handled by auth slice
      }
      console.error('Failed to fetch device types:', error)
    } finally {
      set({ deviceTypesLoading: false })
    }
  },

  addDevice: async (request: AddDeviceRequest) => {
    try {
      const result = await api.addDevice(request)
      // Backend returns { device_id, added: true } after unwrap
      if (result.added || result.device_id) {
        await get().fetchDevices()
        return true
      }
      return false
    } catch (error) {
      if ((error as Error).message === 'UNAUTHORIZED') {
        // Will be handled by auth slice
      }
      console.error('Failed to add device:', error)
      return false
    }
  },

  deleteDevice: async (id) => {
    try {
      const result = await api.deleteDevice(id)
      // Backend returns { device_id, deleted: true } after unwrap
      if (result.deleted) {
        await get().fetchDevices()
        return true
      }
      return false
    } catch (error) {
      console.error('Failed to delete device:', error)
      return false
    }
  },

  sendCommand: async (deviceId, command, params) => {
    try {
      const result = await api.sendCommand(deviceId, command, params)
      // Backend returns { device_id, command, sent: true } after unwrap
      if (result.sent) {
        await get().fetchDevices()
        return true
      }
      return false
    } catch (error) {
      console.error('Failed to send command:', error)
      return false
    }
  },

  // Device Type CRUD
  addDeviceType: async (definition) => {
    try {
      await api.addDeviceType(definition)
      await get().fetchDeviceTypes()
      return true
    } catch (error) {
      console.error('Failed to add device type:', error)
      return false
    }
  },

  deleteDeviceType: async (id) => {
    try {
      await api.deleteDeviceType(id)
      await get().fetchDeviceTypes()
      return true
    } catch (error) {
      console.error('Failed to delete device type:', error)
      return false
    }
  },

  validateDeviceType: async (definition) => {
    try {
      return await api.validateDeviceType(definition)
    } catch (error) {
      console.error('Failed to validate device type:', error)
      return {
        valid: false,
        errors: [`验证失败: ${(error as Error).message}`],
        message: '验证请求失败'
      }
    }
  },

  generateMDL: async (req) => {
    try {
      return await api.generateMDL(req)
    } catch (error) {
      console.error('Failed to generate MDL:', error)
      throw error
    }
  },

  // Device Details
  fetchDeviceDetails: async (id) => {
    try {
      const details = await api.getDevice(id)
      set({ deviceDetails: details })
      return details
    } catch (error) {
      console.error('Failed to fetch device details:', error)
      return null
    }
  },

  fetchDeviceTypeDetails: async (deviceType) => {
    try {
      const details = await api.getDeviceType(deviceType)
      set({ deviceTypeDetails: details })
      return details
    } catch (error) {
      console.error('Failed to fetch device type details:', error)
      return null
    }
  },

  // Device Discovery
  discoverDevices: async (host, ports, timeoutMs) => {
    set({ discovering: true })
    try {
      const result = await api.discoverDevices(host, ports, timeoutMs)
      set({ discoveredDevices: result.devices || [] })
    } catch (error) {
      console.error('Failed to discover devices:', error)
      set({ discoveredDevices: [] })
    } finally {
      set({ discovering: false })
    }
  },

  setDiscoveredDevices: (devices) => set({ discoveredDevices: devices }),

  // Telemetry
  fetchTelemetryData: async (deviceId, metric, start, end, limit) => {
    set({ telemetryLoading: true })
    try {
      const data = await api.getDeviceTelemetry(deviceId, metric, start, end, limit)
      set({ telemetryData: data })
    } catch (error) {
      console.error('Failed to fetch telemetry data:', error)
      set({ telemetryData: null })
    } finally {
      set({ telemetryLoading: false })
    }
  },

  fetchTelemetrySummary: async (deviceId, hours) => {
    set({ telemetryLoading: true })
    try {
      const data = await api.getDeviceTelemetrySummary(deviceId, hours)
      set({ telemetrySummary: data })
    } catch (error) {
      console.error('Failed to fetch telemetry summary:', error)
      set({ telemetrySummary: null })
    } finally {
      set({ telemetryLoading: false })
    }
  },

  fetchCommandHistory: async (deviceId, limit) => {
    set({ telemetryLoading: true })
    try {
      const data = await api.getDeviceCommandHistory(deviceId, limit)
      set({ commandHistory: data })
    } catch (error) {
      console.error('Failed to fetch command history:', error)
      set({ commandHistory: null })
    } finally {
      set({ telemetryLoading: false })
    }
  },

  // Device Adapters
  fetchDeviceAdapters: async () => {
    set({ deviceAdaptersLoading: true })
    try {
      const data = await api.listDeviceAdapters()
      set({ deviceAdapters: data.adapters || [] })
    } catch (error) {
      if ((error as Error).message === 'UNAUTHORIZED') {
        // Will be handled by auth slice
      }
      console.error('Failed to fetch device adapters:', error)
      set({ deviceAdapters: [] })
    } finally {
      set({ deviceAdaptersLoading: false })
    }
  },

  // HASS Discovery
  fetchHassDiscoveryStatus: async () => {
    try {
      const status = await api.getHassDiscoveryStatus()
      set({ hassDiscoveryStatus: status })
    } catch (error) {
      console.error('Failed to fetch HASS discovery status:', error)
      set({ hassDiscoveryStatus: null })
    }
  },

  fetchHassDiscoveredDevices: async () => {
    try {
      const data = await api.getHassDiscoveredDevices()
      set({ hassDiscoveredDevices: data.devices || [] })
    } catch (error) {
      console.error('Failed to fetch HASS discovered devices:', error)
      set({ hassDiscoveredDevices: [] })
    }
  },

  startHassDiscovery: async (req) => {
    set({ hassDiscovering: true })
    try {
      const result = await api.startHassDiscovery(req)
      return result
    } catch (error) {
      console.error('Failed to start HASS discovery:', error)
      throw error
    } finally {
      set({ hassDiscovering: false })
    }
  },

  stopHassDiscovery: async () => {
    try {
      await api.stopHassDiscovery()
      set({ hassDiscovering: false })
    } catch (error) {
      console.error('Failed to stop HASS discovery:', error)
    }
  },

  registerHassDevice: async (deviceId) => {
    try {
      await api.registerAggregatedHassDevice(deviceId)
      // Refresh discovered devices after registration
      await get().fetchHassDiscoveredDevices()
      return true
    } catch (error) {
      console.error('Failed to register HASS device:', error)
      return false
    }
  },

  unregisterHassDevice: async (deviceId) => {
    try {
      await api.unregisterHassDevice(deviceId)
      // Refresh discovered devices after unregistration
      await get().fetchHassDiscoveredDevices()
      return true
    } catch (error) {
      console.error('Failed to unregister HASS device:', error)
      return false
    }
  },

  clearHassDiscoveredDevices: () => {
    set({ hassDiscoveredDevices: [] })
  },
})
