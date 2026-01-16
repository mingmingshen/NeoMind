/**
 * Settings Slice
 *
 * Handles general system settings (config import/export, etc.).
 * LLM/MQTT/Device/HASS settings are now managed via the Plugin system.
 */

import type { StateCreator } from 'zustand'
import type { SettingsState } from '../types'
import { api } from '@/lib/api'

export interface SettingsSlice extends SettingsState {
  // Dialog actions
  setSettingsDialogOpen: (open: boolean) => void

  // System Config actions
  exportConfig: () => Promise<{ config: Record<string, unknown> }>
  importConfig: (config: Record<string, unknown>, merge?: boolean) => Promise<{ imported: number; skipped?: number; errors?: Array<{ error: string }> }>
  validateConfig: (config: Record<string, unknown>) => Promise<{ valid: boolean; errors?: string[] }>
}

export const createSettingsSlice: StateCreator<
  SettingsSlice,
  [],
  [],
  SettingsSlice
> = (set) => ({
  // Initial state
  settingsDialogOpen: false,

  // Dialog actions
  setSettingsDialogOpen: (open) => set({ settingsDialogOpen: open }),

  // System Config - Export
  exportConfig: async () => {
    try {
      const result = await api.exportConfig()
      return result
    } catch (error) {
      console.error('Failed to export config:', error)
      throw error
    }
  },

  // System Config - Import
  importConfig: async (config, merge = false) => {
    try {
      const result = await api.importConfig(config, merge)
      return result
    } catch (error) {
      console.error('Failed to import config:', error)
      throw error
    }
  },

  // System Config - Validate
  validateConfig: async (config) => {
    try {
      const result = await api.validateConfig(config)
      return result
    } catch (error) {
      console.error('Failed to validate config:', error)
      return { valid: false, errors: ['验证失败'] }
    }
  },
})
