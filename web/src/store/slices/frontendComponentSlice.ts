/**
 * Frontend Component Slice
 *
 * Zustand slice for managing community marketplace dashboard components.
 * Handles fetching, installing, and uninstalling frontend components.
 *
 * Matches backend API: crates/neomind-api/src/handlers/frontend_components.rs
 */

import type { StateCreator } from 'zustand'
import type {
  FrontendComponentMeta,
  MarketComponentEntry,
  ComponentManifest,
} from '@/types/frontend-component'
import { api } from '@/lib/api'
import { communityRegistry } from '@/components/dashboard/registry/CommunityRegistry'
import { logError } from '@/lib/errors'

// ============================================================================
// State
// ============================================================================

export interface FrontendComponentState {
  installed: FrontendComponentMeta[]
  marketComponents: MarketComponentEntry[]
  marketLoading: boolean
  loading: boolean
  error: string | null
  fetchCache: Record<string, { timestamp: number }>
}

// ============================================================================
// Slice Interface
// ============================================================================

export interface FrontendComponentSlice extends FrontendComponentState {
  fetchInstalled: () => Promise<void>
  fetchMarket: () => Promise<void>
  installFromMarket: (componentId: string) => Promise<void>
  installManual: (manifest: ComponentManifest, bundleFile: File) => Promise<FrontendComponentMeta>
  uninstall: (id: string) => Promise<void>
}

// ============================================================================
// Cache Helper
// ============================================================================

const CACHE_TTL = 10_000 // 10 seconds

function shouldFetch(cache: Record<string, { timestamp: number }>, key: string): boolean {
  const entry = cache[key]
  if (!entry) return true
  return Date.now() - entry.timestamp > CACHE_TTL
}

// ============================================================================
// Slice Creator
// ============================================================================

export const createFrontendComponentSlice: StateCreator<
  FrontendComponentSlice,
  [],
  [],
  FrontendComponentSlice
> = (set, get) => ({
  // Initial state
  installed: [],
  marketComponents: [],
  marketLoading: false,
  loading: false,
  error: null,
  fetchCache: {},

  // ========== Actions ==========

  /**
   * Fetch all installed frontend components
   * Backend: GET /api/frontend-components -> { components: FrontendComponentMeta[] }
   */
  fetchInstalled: async () => {
    const cache = get().fetchCache
    if (!shouldFetch(cache, 'installed')) return

    set({ loading: true, error: null })
    try {
      const res = await api.get<{ components: FrontendComponentMeta[] }>('/api/frontend-components')
      const components = res.components || []

      // Sync with community registry
      communityRegistry.syncFromApi(components)

      set({
        installed: components,
        fetchCache: { ...cache, installed: { timestamp: Date.now() } },
        error: null,
      })
    } catch (error) {
      logError(error, { operation: 'Fetch installed components' })
      set({
        installed: [],
        error: error instanceof Error ? error.message : 'Failed to fetch installed components',
      })
    } finally {
      set({ loading: false })
    }
  },

  /**
   * Fetch all available components from the marketplace
   * Backend: GET /api/frontend-components/market/list -> { components: MarketComponentEntry[] }
   */
  fetchMarket: async () => {
    set({ marketLoading: true, error: null })
    try {
      const res = await api.get<{ components: MarketComponentEntry[] }>('/api/frontend-components/market/list')
      const components = res.components || []

      set({
        marketComponents: components,
        error: null,
      })
    } catch (error) {
      logError(error, { operation: 'Fetch market components' })
      set({
        marketComponents: [],
        error: error instanceof Error ? error.message : 'Failed to fetch marketplace',
      })
    } finally {
      set({ marketLoading: false })
    }
  },

  /**
   * Install a component from the marketplace
   * Backend: POST /api/frontend-components/market/install -> { component: FrontendComponentMeta }
   */
  installFromMarket: async (componentId) => {
    set({ loading: true, error: null })
    try {
      const res = await api.post<{ component: FrontendComponentMeta }>(
        '/api/frontend-components/market/install',
        { component_id: componentId }
      )
      const component = res.component

      // Add to installed list
      set((state) => ({
        installed: [...state.installed, component],
      }))

      // Sync with community registry
      communityRegistry.syncFromApi([...get().installed, component])

      // Clear cache to force refresh
      set((state) => ({
        fetchCache: Object.fromEntries(
          Object.entries(state.fetchCache).filter(([key]) => key !== 'installed')
        ),
      }))

      // Refresh installed list
      await get().fetchInstalled()
    } catch (error) {
      logError(error, { operation: 'Install from marketplace', context: { componentId } })
      set({
        error: error instanceof Error ? error.message : 'Failed to install component',
      })
      throw error
    } finally {
      set({ loading: false })
    }
  },

  /**
   * Install a component manually with manifest and bundle file
   * Backend: POST /api/frontend-components -> { component: FrontendComponentMeta }
   */
  installManual: async (manifest, bundleFile) => {
    set({ loading: true, error: null })
    try {
      // Create FormData
      const formData = new FormData()
      formData.append('manifest', JSON.stringify(manifest))
      formData.append('bundle', bundleFile)

      const res = await api.post<{ component: FrontendComponentMeta }>(
        '/api/frontend-components',
        formData,
        {
          headers: {
            'Content-Type': 'multipart/form-data',
          },
        }
      )
      const component = res.component

      // Add to installed list
      set((state) => ({
        installed: [...state.installed, component],
      }))

      // Sync with community registry
      communityRegistry.syncFromApi([...get().installed, component])

      // Clear cache to force refresh
      set((state) => ({
        fetchCache: Object.fromEntries(
          Object.entries(state.fetchCache).filter(([key]) => key !== 'installed')
        ),
      }))

      return component
    } catch (error) {
      logError(error, { operation: 'Install manual component' })
      set({
        error: error instanceof Error ? error.message : 'Failed to install component',
      })
      throw error
    } finally {
      set({ loading: false })
    }
  },

  /**
   * Uninstall a component
   * Backend: DELETE /api/frontend-components/:id
   */
  uninstall: async (id) => {
    set({ loading: true, error: null })
    try {
      await api.delete(`/api/frontend-components/${id}`)

      // Remove from installed list
      set((state) => ({
        installed: state.installed.filter((c) => c.id !== id),
      }))

      // Unregister from community registry
      communityRegistry.unregister(id)

      // Clear cache to force refresh
      set((state) => ({
        fetchCache: Object.fromEntries(
          Object.entries(state.fetchCache).filter(([key]) => key !== 'installed')
        ),
      }))
    } catch (error) {
      logError(error, { operation: 'Uninstall component', context: { id } })
      set({
        error: error instanceof Error ? error.message : 'Failed to uninstall component',
      })
      throw error
    } finally {
      set({ loading: false })
    }
  },
})
