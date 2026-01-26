/**
 * Alert Slice
 *
 * Handles alert state and operations.
 */

import type { StateCreator } from 'zustand'
import type { AlertState } from '../types'
import { api } from '@/lib/api'

// Get auth token
const getToken = (): string | null => {
  return localStorage.getItem('neotalk_token') || sessionStorage.getItem('neotalk_token_session')
}

export interface AlertSlice extends AlertState {
  // Actions
  fetchAlerts: () => Promise<void>
  acknowledgeAlert: (id: string) => Promise<boolean>
  createAlert: (alert: { title: string; message: string; severity?: string; source?: string }) => Promise<boolean>
}

export const createAlertSlice: StateCreator<
  AlertSlice,
  [],
  [],
  AlertSlice
> = (set, get) => ({
  // Initial state
  alerts: [],
  alertsLoading: false,

  // Actions
  fetchAlerts: async () => {
    set({ alertsLoading: true })
    try {
      const token = getToken()
      const response = await fetch('/api/alerts', {
        headers: {
          'Authorization': token ? `Bearer ${token}` : '',
        },
      })
      const rawData = await response.json()

      // Handle different response formats
      let alertsArray: unknown[] = []
      if (Array.isArray(rawData)) {
        alertsArray = rawData
      } else if (rawData?.data?.alerts && Array.isArray(rawData.data.alerts)) {
        alertsArray = rawData.data.alerts
      } else if (rawData?.alerts && Array.isArray(rawData.alerts)) {
        alertsArray = rawData.alerts
      } else if (rawData?.data && Array.isArray(rawData.data)) {
        alertsArray = rawData.data
      }

      set({ alerts: alertsArray as any })
    } catch (error) {
      console.error('Failed to fetch alerts:', error)
      set({ alerts: [] })
    } finally {
      set({ alertsLoading: false })
    }
  },

  acknowledgeAlert: async (id: string) => {
    try {
      const result = await api.acknowledgeAlert(id)
      if (result.acknowledged) {
        // Update the alert in the list
        set((state) => ({
          alerts: state.alerts.map((alert) =>
            alert.id === id
              ? { ...alert, acknowledged: true, status: 'acknowledged' as const }
              : alert
          ),
        }))
        return true
      }
      return false
    } catch (error) {
      console.error('Failed to acknowledge alert:', error)
      return false
    }
  },

  createAlert: async (alert) => {
    try {
      await api.createAlert(alert)
      // Refresh the alerts list after creating
      await get().fetchAlerts()
      return true
    } catch (error) {
      console.error('Failed to create alert:', error)
      return false
    }
  },
})
