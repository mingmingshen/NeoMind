/**
 * Dashboard core types — new feature module
 *
 * Re-exports existing types from @/types/dashboard for compatibility.
 */

export type {
  GenericComponentType,
  BusinessComponentType,
  ImplementedComponentType,
  ComponentType,
  ColorScaleType,
  ColorScale,
  ColorStop,
  Threshold,
  Size,
  Density,
  DisplayConfig,
  ComponentPosition,
  ComponentSizeConstraints,
  DashboardLayout,
  ActionType,
  ActionConfig,
} from '@/types/dashboard'

export type {
  BaseComponent,
  GenericComponent,
  BusinessComponent,
  DashboardComponent,
  Dashboard,
  DashboardTemplate,
} from '@/types/dashboard'

export {
  isGenericComponent,
  isBusinessComponent,
  COMPONENT_SIZE_CONSTRAINTS,
} from '@/types/dashboard'
