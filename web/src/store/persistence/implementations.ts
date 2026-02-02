/**
 * Persistence Layer - Storage Implementations
 *
 * Concrete implementations of DashboardStorage for different backends.
 */

import type {
  DashboardStorage,
  StorageResult,
  DashboardDTO,
  CreateDashboardDTO,
  UpdateDashboardDTO,
} from './types'
import type { Dashboard } from '@/types/dashboard'
import {
  toDashboardDTO,
  fromDashboardDTO,
  toCreateDashboardDTO,
  toUpdateDashboardDTO,
} from './types'

// ============================================================================
// LocalStorage Storage
// ============================================================================

const LOCAL_STORAGE_KEY = 'neomind_dashboards'
const CURRENT_DASHBOARD_KEY = 'neomind_current_dashboard_id'

export class LocalStorageDashboardStorage implements DashboardStorage {
  private storageKey: string

  constructor(storageKey: string = LOCAL_STORAGE_KEY) {
    this.storageKey = storageKey
  }

  async load(): Promise<StorageResult<Dashboard[]>> {
    try {
      const stored = localStorage.getItem(this.storageKey)
      if (!stored) {
        return { data: [], error: null, source: 'local' }
      }

      const dashboards = JSON.parse(stored) as Dashboard[]
      return { data: dashboards, error: null, source: 'local' }
    } catch (error) {
      return {
        data: null,
        error: error instanceof Error ? error : new Error('Failed to load from localStorage'),
        source: 'local',
      }
    }
  }

  async save(dashboards: Dashboard[]): Promise<StorageResult<void>> {
    try {
      localStorage.setItem(this.storageKey, JSON.stringify(dashboards))
      return { data: undefined, error: null, source: 'local' }
    } catch (error) {
      return {
        data: null,
        error: error instanceof Error ? error : new Error('Failed to save to localStorage'),
        source: 'local',
      }
    }
  }

  async sync(dashboard: Dashboard): Promise<StorageResult<Dashboard>> {
    try {
      // Load existing, update, and save back
      const result = await this.load()
      const dashboards = result.data || []

      // If dashboard doesn't have an ID, generate one for new dashboards
      const dashboardToSave = dashboard.id
        ? dashboard
        : { ...dashboard, id: crypto.randomUUID(), createdAt: Date.now(), updatedAt: Date.now() }

      const index = dashboards.findIndex(d => d.id === dashboardToSave.id)
      if (index >= 0) {
        dashboards[index] = dashboardToSave
      } else {
        dashboards.push(dashboardToSave)
      }

      await this.save(dashboards)
      return { data: dashboardToSave, error: null, source: 'local' }
    } catch (error) {
      return {
        data: null,
        error: error instanceof Error ? error : new Error('Failed to sync to localStorage'),
        source: 'local',
      }
    }
  }

  async delete(id: string): Promise<StorageResult<void>> {
    try {
      const result = await this.load()
      const dashboards = (result.data || []).filter(d => d.id !== id)
      await this.save(dashboards)
      return { data: undefined, error: null, source: 'local' }
    } catch (error) {
      return {
        data: null,
        error: error instanceof Error ? error : new Error('Failed to delete from localStorage'),
        source: 'local',
      }
    }
  }

  isAvailable(): boolean {
    try {
      localStorage.setItem('test', 'test')
      localStorage.removeItem('test')
      return true
    } catch {
      return false
    }
  }

  getType(): string {
    return 'local'
  }

  // Current dashboard helpers
  getCurrentDashboardId(): string | null {
    return localStorage.getItem(CURRENT_DASHBOARD_KEY)
  }

  setCurrentDashboardId(id: string | null): void {
    if (id) {
      localStorage.setItem(CURRENT_DASHBOARD_KEY, id)
    } else {
      localStorage.removeItem(CURRENT_DASHBOARD_KEY)
    }
  }

  clear(): void {
    localStorage.removeItem(this.storageKey)
    localStorage.removeItem(CURRENT_DASHBOARD_KEY)
  }
}

// ============================================================================
// API Storage
// ============================================================================

export class ApiDashboardStorage implements DashboardStorage {
  private api: any
  private currentDashboardId: string | null = null

  constructor() {
    // Import api module dynamically to avoid circular deps
    this.api = null
  }

  private async getApi() {
    if (!this.api) {
      const module = await import('@/lib/api')
      this.api = module.api
    }
    return this.api
  }

  async load(): Promise<StorageResult<Dashboard[]>> {
    try {
      const api = await this.getApi()
      const response = await api.getDashboards()

      // Backend returns { dashboards: Dashboard[], count: number }
      const dashboards = 'dashboards' in response
        ? (response as { dashboards: typeof response.dashboards; count: number }).dashboards.map(fromDashboardDTO)
        : Array.isArray(response)
          ? response.map(fromDashboardDTO)
          : []

      return { data: dashboards, error: null, source: 'api' }
    } catch (error) {
      return {
        data: null,
        error: error instanceof Error ? error : new Error('Failed to load from API'),
        source: 'api',
      }
    }
  }

  async save(dashboards: Dashboard[]): Promise<StorageResult<void>> {
    // API doesn't support bulk save - sync individual dashboards instead
    // Cache to localStorage for instant access
    try {
      localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(dashboards))
    } catch {
      // Ignore cache errors
    }
    return { data: undefined, error: null, source: 'api' }
  }

  async sync(dashboard: Dashboard): Promise<StorageResult<Dashboard>> {
    try {
      const api = await this.getApi()

      // Check if this is a newly created dashboard (recently created, no server sync yet)
      const isNewDashboard = !dashboard.createdAt || (Date.now() - dashboard.createdAt) < 5000

      // For new dashboards, directly create without checking existence first
      if (isNewDashboard) {
        try {
          // Don't include the local ID - let server generate it
          const { id, createdAt, updatedAt, ...dashboardForCreate } = dashboard
          const createDto = toCreateDashboardDTO(dashboardForCreate as any)
          const result = await api.createDashboard(createDto)
          // Backend returns full Dashboard
          return { data: fromDashboardDTO(result), error: null, source: 'api' }
        } catch (createError) {
          // If create fails, this might be a dashboard that was just created locally
          // Don't fail the entire sync - the local version is the source of truth
          console.warn('[ApiStorage] Dashboard creation failed, using local version:', createError)
          return { data: dashboard, error: null, source: 'api' }
        }
      }

      // For existing dashboards, check if they exist on server first
      const existing = await api.getDashboard(dashboard.id).catch(() => null)

      if (existing) {
        // Update existing - use UpdateDashboardRequest format
        const updateDto = toUpdateDashboardDTO(dashboard)
        const result = await api.updateDashboard(dashboard.id, updateDto)
        // Backend returns full Dashboard
        return { data: fromDashboardDTO(result), error: null, source: 'api' }
      } else {
        // Dashboard doesn't exist on server - try to create it
        try {
          const createDto = toCreateDashboardDTO(dashboard)
          const result = await api.createDashboard(createDto)
          return { data: fromDashboardDTO(result), error: null, source: 'api' }
        } catch (createError) {
          // Create failed - keep local version
          console.warn('[ApiStorage] Dashboard sync failed, using local version:', createError)
          return { data: dashboard, error: null, source: 'api' }
        }
      }
    } catch (error) {
      return {
        data: null,
        error: error instanceof Error ? error : new Error('Failed to sync to API'),
        source: 'api',
      }
    }
  }

  async delete(id: string): Promise<StorageResult<void>> {
    try {
      const api = await this.getApi()
      await api.deleteDashboard(id)

      // Also remove from local cache
      try {
        const stored = localStorage.getItem(LOCAL_STORAGE_KEY)
        if (stored) {
          const dashboards = JSON.parse(stored) as Dashboard[]
          const filtered = dashboards.filter(d => d.id !== id)
          localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(filtered))
        }
      } catch {
        // Ignore cache errors
      }

      return { data: undefined, error: null, source: 'api' }
    } catch (error) {
      return {
        data: null,
        error: error instanceof Error ? error : new Error('Failed to delete from API'),
        source: 'api',
      }
    }
  }

  isAvailable(): boolean {
    // API is always considered available if we have network
    // Errors will be caught during operations
    return typeof window !== 'undefined' && navigator.onLine
  }

  getType(): string {
    return 'api'
  }

  getCurrentDashboardId(): string | null {
    return this.currentDashboardId
  }

  setCurrentDashboardId(id: string | null): void {
    this.currentDashboardId = id
    // Also sync to localStorage
    if (id) {
      localStorage.setItem(CURRENT_DASHBOARD_KEY, id)
    } else {
      localStorage.removeItem(CURRENT_DASHBOARD_KEY)
    }
  }

  clear(): void {
    // Clear local cache only - server data remains
    localStorage.removeItem(LOCAL_STORAGE_KEY)
    localStorage.removeItem(CURRENT_DASHBOARD_KEY)
    this.currentDashboardId = null
  }
}

// ============================================================================
// Hybrid Storage (API with localStorage fallback)
// ============================================================================

export class HybridDashboardStorage implements DashboardStorage {
  private apiStorage: ApiDashboardStorage
  private localStorage: LocalStorageDashboardStorage
  private cacheEnabled: boolean

  constructor(options: { cacheEnabled?: boolean } = {}) {
    this.apiStorage = new ApiDashboardStorage()
    this.localStorage = new LocalStorageDashboardStorage()
    this.cacheEnabled = options.cacheEnabled ?? true
  }

  async load(): Promise<StorageResult<Dashboard[]>> {
    // Try API first
    const apiResult = await this.apiStorage.load()

    if (apiResult.error || !apiResult.data) {
      console.warn('[HybridStorage] API load failed, checking error type:', apiResult.error?.message)

      // Check if the error is because the dashboards table doesn't exist
      // In this case, fall back to localStorage instead of clearing it
      // This allows users to work locally when backend is unavailable
      const errorMessage = apiResult.error?.message || ''
      const isTableNotExist = errorMessage.includes("Table 'dashboards' does not exist") ||
                             errorMessage.includes('does not exist')

      if (isTableNotExist) {
        console.log('[HybridStorage] Dashboards table does not exist on backend, using localStorage')
        // Don't clear localStorage - let users work with local data
        // When backend becomes available, data can be synced
        return this.localStorage.load()
      }

      // For other errors, also fall back to localStorage
      console.warn('[HybridStorage] API load failed, falling back to localStorage')
      return this.localStorage.load()
    }

    // Cache to localStorage if enabled
    if (this.cacheEnabled && apiResult.data) {
      this.localStorage.save(apiResult.data).catch(() => {
        // Ignore cache save errors
      })
    }

    return apiResult
  }

  async save(dashboards: Dashboard[]): Promise<StorageResult<void>> {
    // Always save to localStorage immediately for responsiveness
    const localResult = await this.localStorage.save(dashboards)

    // Try to sync to API in background
    this.syncToApi(dashboards).catch(() => {
      // API sync failed, but local save succeeded
      console.warn('[HybridStorage] Background API sync failed')
    })

    return localResult
  }

  async sync(dashboard: Dashboard): Promise<StorageResult<Dashboard>> {
    // Sync to localStorage first (fast, reliable)
    const localResult = await this.localStorage.sync(dashboard)

    // Try to sync to API in background with the dashboard that now has an ID
    if (localResult.data) {
      this.apiStorage.sync(localResult.data).catch(() => {
        // API sync failed
        console.warn('[HybridStorage] API sync failed for dashboard:', localResult.data?.id)
      })
    }

    return localResult
  }

  async delete(id: string): Promise<StorageResult<void>> {
    // Delete from localStorage first
    const localResult = await this.localStorage.delete(id)

    // Try to delete from API in background
    this.apiStorage.delete(id).catch(() => {
      console.warn('[HybridStorage] API delete failed for dashboard:', id)
    })

    return localResult
  }

  isAvailable(): boolean {
    return this.localStorage.isAvailable() || this.apiStorage.isAvailable()
  }

  getType(): string {
    return 'hybrid'
  }

  // Helper to sync all dashboards to API
  private async syncToApi(dashboards: Dashboard[]): Promise<void> {
    for (const dashboard of dashboards) {
      await this.apiStorage.sync(dashboard)
    }
  }

  // Expose current dashboard helpers from localStorage
  getCurrentDashboardId(): string | null {
    return this.localStorage.getCurrentDashboardId()
  }

  setCurrentDashboardId(id: string | null): void {
    this.localStorage.setCurrentDashboardId(id)
  }

  clear(): void {
    this.localStorage.clear()
  }
}

// ============================================================================
// Factory
// ============================================================================

export interface CreateStorageOptions {
  type?: 'local' | 'api' | 'hybrid'
  cacheEnabled?: boolean
}

export function createDashboardStorage(options: CreateStorageOptions = {}): DashboardStorage {
  const { type = 'hybrid', cacheEnabled = true } = options

  switch (type) {
    case 'local':
      return new LocalStorageDashboardStorage()
    case 'api':
      return new ApiDashboardStorage()
    case 'hybrid':
      return new HybridDashboardStorage({ cacheEnabled })
    default:
      return new HybridDashboardStorage({ cacheEnabled })
  }
}
