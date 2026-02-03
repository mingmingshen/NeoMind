/**
 * Decision Slice
 *
 * Handles AI decision state and operations.
 */

import type { StateCreator } from 'zustand'
import type { DecisionState } from '../types'
import { api } from '@/lib/api'
import { logError } from '@/lib/errors'

export interface DecisionSlice extends DecisionState {
  // Actions
  fetchDecisions: () => Promise<void>
}

export const createDecisionSlice: StateCreator<
  DecisionSlice,
  [],
  [],
  DecisionSlice
> = (set) => ({
  // Initial state
  decisions: [],
  decisionsLoading: false,

  // Actions
  fetchDecisions: async () => {
    set({ decisionsLoading: true })
    try {
      const data = await api.listDecisions()
      set({ decisions: data.decisions || [] })
    } catch (error) {
      logError(error, { operation: 'Fetch decisions' })
      set({ decisions: [] })
    } finally {
      set({ decisionsLoading: false })
    }
  },
})
