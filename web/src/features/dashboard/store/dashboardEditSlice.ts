/**
 * Dashboard Edit Slice
 *
 * Simple UI state for dashboard editing mode, component selection,
 * and panel visibility. No persistence needed.
 */

import type { StateCreator } from 'zustand'
import type { DashboardStore } from './index'

// ============================================================================
// Slice State Type
// ============================================================================

export interface DashboardEditSlice {
  // UI State
  editMode: boolean
  selectedComponent: string | null
  isReadOnly: boolean

  // Panels
  componentLibraryOpen: boolean
  configPanelOpen: boolean
  configComponentId: string | null
  templateDialogOpen: boolean

  // Actions
  setEditMode: (edit: boolean) => void
  setSelectedComponent: (id: string | null) => void
  setComponentLibraryOpen: (open: boolean) => void
  setConfigPanelOpen: (open: boolean, componentId?: string) => void
  setTemplateDialogOpen: (open: boolean) => void
}

// ============================================================================
// Create Slice
// ============================================================================

export const createDashboardEditSlice: StateCreator<
  DashboardStore,
  [],
  [],
  DashboardEditSlice
> = (set, get) => ({
  // Initial state
  editMode: false,
  selectedComponent: null,
  isReadOnly: false,

  componentLibraryOpen: false,
  configPanelOpen: false,
  configComponentId: null,
  templateDialogOpen: false,

  // ========================================================================
  // Actions
  // ========================================================================

  setEditMode: (edit) => set({ editMode: edit, selectedComponent: null }),

  setSelectedComponent: (id) => {
    const { currentDashboard } = get()
    // Validate that the component exists before selecting it
    if (id && currentDashboard?.components) {
      const componentExists = currentDashboard.components.some((c) => c.id === id)
      if (!componentExists) {
        // Component no longer exists, clear selection
        set({ selectedComponent: null })
        return
      }
    }
    set({ selectedComponent: id })
  },

  setComponentLibraryOpen: (open) => set({ componentLibraryOpen: open }),

  setConfigPanelOpen: (open, componentId) => {
    const { currentDashboard } = get()
    // Validate that the component exists before opening config
    if (open && componentId && currentDashboard?.components) {
      const componentExists = currentDashboard.components.some(
        (c) => c.id === componentId
      )
      if (!componentExists) {
        // Component no longer exists, don't open config panel
        set({ configPanelOpen: false, configComponentId: null })
        return
      }
    }
    set({
      configPanelOpen: open,
      configComponentId: componentId || null,
    })
  },

  setTemplateDialogOpen: (open) => set({ templateDialogOpen: open }),
})
