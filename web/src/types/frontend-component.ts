/**
 * Frontend Component Types
 *
 * Types for community marketplace dashboard components.
 * These components are installed from the marketplace and loaded dynamically.
 */

export interface SizeConstraints {
  min_w: number
  min_h: number
  default_w: number
  default_h: number
  max_w: number
  max_h: number
}

/**
 * Metadata for an installed frontend component
 * This is returned by the API and stored in the registry
 */
export interface FrontendComponentMeta {
  id: string
  name: string | Record<string, string>
  description: string | Record<string, string>
  icon: string
  category: string
  version: string
  author?: string
  screenshot?: string
  size_constraints: SizeConstraints
  has_data_source: boolean
  max_data_sources?: number
  has_display_config: boolean
  has_actions: boolean
  config_schema?: Record<string, unknown>
  default_config?: Record<string, unknown>
  variants?: string[]
  global_name: string
  export_name?: string
  installed_at: number
}

/**
 * Entry in the component marketplace
 * This represents a component available for installation
 */
export interface MarketComponentEntry {
  id: string
  name: string | Record<string, string>
  description: string | Record<string, string>
  icon: string
  category: string
  version: string
  author?: string
  screenshot_url?: string
  size_constraints: SizeConstraints
  has_data_source: boolean
  max_data_sources?: number
  has_display_config: boolean
  has_actions: boolean
  manifest_url: string
  bundle_url: string
}

/**
 * Component manifest for manual installation
 * This is uploaded by users when installing components manually
 */
export interface ComponentManifest {
  id: string
  name: string | Record<string, string>
  description: string | Record<string, string>
  icon?: string
  category?: string
  version?: string
  author?: string
  screenshot?: string
  size_constraints: SizeConstraints
  has_data_source?: boolean
  max_data_sources?: number
  has_display_config?: boolean
  has_actions?: boolean
  config_schema?: Record<string, unknown>
  default_config?: Record<string, unknown>
  variants?: string[]
  global_name: string
  export_name?: string
}
