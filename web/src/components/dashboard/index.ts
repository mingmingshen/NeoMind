/**
 * Dashboard Components Index
 *
 * Exports all dashboard-related components.
 */

// Registry (New - centralized component metadata and rendering)
export * from './registry'

// Wrapper
export { DashboardComponentWrapper } from './DashboardComponentWrapper'
export type { DashboardComponentWrapperProps } from './DashboardComponentWrapper'

// Layout
export { DashboardGrid } from './DashboardGrid'
export { DashboardListSidebar } from './DashboardListSidebar'
export type { DashboardListSidebarProps } from './DashboardListSidebar'

// Generic components - Indicators
export { ValueCard } from './generic/ValueCard'
export { LEDIndicator, type LEDState } from './generic/LEDIndicator'
export { Sparkline } from './generic/Sparkline'
export { ProgressBar } from './generic/ProgressBar'

// Generic components - Charts
export { LineChart, AreaChart } from './generic/LineChart'
export { BarChart } from './generic/BarChart'
export { PieChart } from './generic/PieChart'

// Generic components - Controls
export { ToggleSwitch } from './generic/ToggleSwitch'
export { ButtonGroup } from './generic/ButtonGroup'
export { Slider } from './generic/Slider'
export { Dropdown } from './generic/Dropdown'
export { InputField } from './generic/InputField'

// Generic components - Display & Content
export { ImageDisplay } from './generic/ImageDisplay'
export { ImageHistory, type ImageHistoryProps, type ImageHistoryItem } from './generic/ImageHistory'
export { WebDisplay } from './generic/WebDisplay'
export { MarkdownDisplay } from './generic/MarkdownDisplay'

// Config system
export * from './config'
