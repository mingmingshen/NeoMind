/**
 * Persistence Layer - Types
 *
 * Abstract interface for dashboard storage operations.
 * Supports multiple storage backends (localStorage, API, hybrid).
 */

import type { Dashboard, DashboardComponent, ComponentPosition, GenericComponent, BusinessComponent, DataSource, ActionConfig, DisplayConfig } from '@/types/dashboard'
import { isGenericComponent } from '@/types/dashboard'

// ============================================================================
// Storage Operation Result
// ============================================================================

export interface StorageResult<T> {
  data: T | null
  error: Error | null
  source: 'local' | 'api' | 'cache'
}

// ============================================================================
// Storage Backend Interface
// ============================================================================

export interface DashboardStorage {
  /**
   * Load all dashboards from storage
   */
  load(): Promise<StorageResult<Dashboard[]>>

  /**
   * Save all dashboards to storage
   */
  save(dashboards: Dashboard[]): Promise<StorageResult<void>>

  /**
   * Sync a single dashboard (create or update)
   */
  sync(dashboard: Dashboard): Promise<StorageResult<Dashboard>>

  /**
   * Delete a dashboard
   */
  delete(id: string): Promise<StorageResult<void>>

  /**
   * Clear all dashboard data from storage
   */
  clear(): void

  /**
   * Check if storage is available
   */
  isAvailable(): boolean

  /**
   * Get storage type identifier
   */
  getType(): string
}

// ============================================================================
// Storage Configuration
// ============================================================================

export interface StorageConfig {
  // Primary storage type
  primary: 'api' | 'local'

  // Fallback to localStorage if API fails
  fallback?: boolean

  // Cache API responses in localStorage
  cache?: boolean

  // Debounce sync operations (ms)
  debounceMs?: number

  // Auto-save changes
  autoSave?: boolean
}

// ============================================================================
// DTO Conversion (between API and internal format)
// ============================================================================

/** Component in API snake_case format */
export interface ComponentDTO {
  id: string
  type: string
  position: {
    x: number
    y: number
    w: number
    h: number
    min_w?: number
    min_h?: number
    max_w?: number
    max_h?: number
  }
  title?: string
  data_source?: Record<string, unknown>
  /** @deprecated API returns data_source, but some internal paths still use dataSource */
  dataSource?: Record<string, unknown>
  display?: Record<string, unknown>
  config?: Record<string, unknown>
  actions?: Array<Record<string, unknown>>
}

export interface DashboardDTO {
  id: string
  name: string
  layout: {
    columns: number
    rows: 'auto' | number
    breakpoints: {
      lg: number
      md: number
      sm: number
      xs: number
    }
  }
  components: ComponentDTO[]
  created_at: number
  updated_at: number
  is_default?: boolean
}

export interface CreateDashboardDTO {
  name: string
  layout: DashboardDTO['layout']
  components: DashboardDTO['components']
}

export interface UpdateDashboardDTO {
  name?: string
  layout?: DashboardDTO['layout']
  components?: DashboardDTO['components']
}

// ============================================================================
// DTO Conversion Helpers
// ============================================================================

/** Convert internal ComponentPosition to API snake_case position */
function positionToDTO(p: ComponentPosition): ComponentDTO['position'] {
  return {
    x: p.x,
    y: p.y,
    w: p.w,
    h: p.h,
    min_w: p.minW,
    min_h: p.minH,
    max_w: p.maxW,
    max_h: p.maxH,
  }
}

/** Convert a single DashboardComponent to API snake_case component DTO */
function componentToDTO(c: DashboardComponent): ComponentDTO {
  const isGeneric = isGenericComponent(c)
  return {
    id: c.id,
    type: c.type,
    position: positionToDTO(c.position),
    title: c.title,
    data_source: (isGeneric ? (c as GenericComponent).dataSource : (c as BusinessComponent).dataSource) as Record<string, unknown> | undefined,
    display: isGeneric ? ((c as GenericComponent).display as Record<string, unknown> | undefined) : undefined,
    config: 'config' in c ? ((c as GenericComponent).config as Record<string, unknown> | undefined) : undefined,
    actions: isGeneric && (c as GenericComponent).actions
      ? ((c as GenericComponent).actions as unknown as Array<Record<string, unknown>>)
      : undefined,
  }
}

/** Convert API snake_case position to internal ComponentPosition */
function positionFromDTO(p: ComponentDTO['position']): ComponentPosition {
  return {
    x: p.x,
    y: p.y,
    w: p.w,
    h: p.h,
    minW: p.min_w,
    minH: p.min_h,
    maxW: p.max_w,
    maxH: p.max_h,
  }
}

/**
 * Convert internal Dashboard to API DTO format
 * Returns API format with snake_case fields (data_source)
 */
export function toDashboardDTO(dashboard: Dashboard): CreateDashboardDTO & { id: string; created_at: number; updated_at: number; is_default?: boolean } {
  return {
    id: dashboard.id,
    name: dashboard.name,
    layout: dashboard.layout,
    components: dashboard.components.map(componentToDTO),
    created_at: dashboard.createdAt,
    updated_at: dashboard.updatedAt,
    is_default: dashboard.isDefault,
  }
}

/**
 * Convert API DTO to internal Dashboard format
 * Handles both DashboardDTO (camelCase) and API DashboardResponse (snake_case)
 */
export function fromDashboardDTO(dto: DashboardDTO): Dashboard {
  const components: DashboardComponent[] = (dto.components || []).map((c) => {
    const dataSource = c.data_source ?? c.dataSource

    const base = {
      id: c.id,
      type: c.type as DashboardComponent['type'],
      position: positionFromDTO(c.position),
      title: c.title,
    }

    if (isGenericComponent({ type: c.type } as DashboardComponent)) {
      const comp: GenericComponent = {
        ...base,
        type: c.type as GenericComponent['type'],
        ...(dataSource ? { dataSource: dataSource as unknown as DataSource } : {}),
        ...(c.display ? { display: c.display as unknown as DisplayConfig } : {}),
        ...(c.config ? { config: c.config } : {}),
        ...(c.actions ? { actions: c.actions as unknown as ActionConfig[] } : {}),
      }
      return comp
    }

    // BusinessComponent: config + dataSource (for agent binding, etc.)
    const comp: BusinessComponent = {
      ...base,
      type: c.type as BusinessComponent['type'],
      ...(dataSource ? { dataSource: dataSource as unknown as DataSource } : {}),
      ...(c.config ? { config: c.config } : {}),
    }
    return comp
  })

  return {
    id: dto.id,
    name: dto.name,
    layout: dto.layout,
    components,
    createdAt: dto.created_at,
    updatedAt: dto.updated_at,
    isDefault: dto.is_default,
  }
}

/**
 * Convert to create DTO (without id and timestamps)
 * Returns API format with snake_case fields (data_source)
 */
export function toCreateDashboardDTO(dashboard: Omit<Dashboard, 'id' | 'createdAt' | 'updatedAt'>): CreateDashboardDTO {
  return {
    name: dashboard.name,
    layout: dashboard.layout,
    components: dashboard.components.map(componentToDTO),
  }
}

/**
 * Convert to update DTO (partial)
 * Returns API format with snake_case fields (data_source)
 */
export function toUpdateDashboardDTO(updates: Partial<Dashboard>): UpdateDashboardDTO {
  const dto: UpdateDashboardDTO = {}

  if (updates.name !== undefined) dto.name = updates.name
  if (updates.layout !== undefined) dto.layout = updates.layout

  if (updates.components !== undefined) {
    dto.components = updates.components.map(componentToDTO)
  }

  return dto
}
