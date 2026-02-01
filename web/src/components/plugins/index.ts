/**
 * Plugin Components
 *
 * NOTE: The Plugin system has been migrated to the Extension system.
 * See /components/extensions for extension management components.
 *
 * The components in this file are kept for backward compatibility:
 * - ConfigFormBuilder: Generic form builder used by LLM/Message configuration
 * - UniversalPluginConfigDialog: Configuration dialog for LLM backends and message channels
 * - AlertChannelPluginConfigDialog: Specific configuration for message channels (legacy name)
 * - Schema-based components: For schema-driven UI generation
 * - UnifiedPluginCard: Card component for displaying plugin-like items
 *
 * DEPRECATED components (do not use for new code):
 * - PluginGrid: Use ExtensionGrid from /components/extensions instead
 * - PluginUploadDialog: Use ExtensionUploadDialog from /components/extensions instead
 * - PluginCard: Use ExtensionCard from /components/extensions instead
 */

// Generic configuration components (still used by LLM/Message systems)
export { ConfigFormBuilder } from './ConfigFormBuilder'
export { UniversalPluginConfigDialog } from './UniversalPluginConfigDialog'
export type { PluginInstance, UnifiedPluginType } from './UniversalPluginConfigDialog'
export { AlertChannelPluginConfigDialog } from './AlertChannelPluginConfigDialog'

// Schema-based components (for schema-driven UI)
export { SchemaPluginCard, SchemaPluginTypeCard, SchemaPluginConfigDialog } from './SchemaPluginCard'
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

// Marketplace (may be repurposed for extensions)
export { PluginMarketplace } from './PluginMarketplace'
export type { MarketplacePlugin } from './PluginMarketplace'

// DEPRECATED: Use ExtensionGrid from /components/extensions instead
export { PluginGrid } from './PluginGrid'
export type { PluginGridProps } from './PluginGrid'

// DEPRECATED: Use ExtensionCard from /components/extensions instead
export { PluginCard } from './PluginCard'
export type { PluginCardProps } from './PluginCard'

// DEPRECATED: Use ExtensionUploadDialog from /components/extensions instead
export { PluginUploadDialog } from './PluginUploadDialog'
export type { PluginUploadDialogProps } from './PluginUploadDialog'
