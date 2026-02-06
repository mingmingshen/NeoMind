/**
 * Plugin Components
 *
 * NOTE: The Plugin system has been migrated to the Extension system.
 * See /components/extensions for extension management components.
 *
 * The components in this file are used for unified configuration of LLM backends,
 * message channels, and device connections.
 */

// Generic configuration components (used by LLM/Message/Device systems)
export { ConfigFormBuilder } from './ConfigFormBuilder'
export { UniversalPluginConfigDialog } from './UniversalPluginConfigDialog'
export type { PluginInstance, UnifiedPluginType } from './UniversalPluginConfigDialog'

// Schema-based components (for schema-driven UI generation)
export { SchemaConfigForm } from './SchemaConfigForm'

// Unified plugin card components (for displaying plugin-like items)
export {
  UnifiedPluginCard,
  UnifiedPluginCardCompact,
  PluginCapabilitiesBadge,
  StatusBadge,
  TypeBadge,
} from './UnifiedPluginCard'
export type {
  UnifiedPluginData,
  UnifiedPluginCardProps,
  UnifiedPluginCardCompactProps,
} from './UnifiedPluginCard'
