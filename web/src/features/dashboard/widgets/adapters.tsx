/**
 * Widget Adapters — thin bridge layer from new WidgetProps to existing components
 *
 * Each adapter accepts the unified WidgetProps interface and delegates
 * to the corresponding existing widget component. The existing components
 * manage their own data fetching via their `dataSource` prop and internal
 * `useDataSource` hook, so we pass through the original DataSource config.
 */

import type { ComponentType } from 'react'
import type { WidgetProps, WidgetType } from '../types'

// ---------------------------------------------------------------------------
// Generic widget imports
// ---------------------------------------------------------------------------
import { ValueCard } from '@/components/dashboard/generic/ValueCard'
import { LEDIndicator } from '@/components/dashboard/generic/LEDIndicator'
import { Sparkline } from '@/components/dashboard/generic/Sparkline'
import { ProgressBar } from '@/components/dashboard/generic/ProgressBar'
import { LineChart, AreaChart } from '@/components/dashboard/generic/LineChart'
import { BarChart } from '@/components/dashboard/generic/BarChart'
import { PieChart } from '@/components/dashboard/generic/PieChart'
import { ToggleSwitch } from '@/components/dashboard/generic/ToggleSwitch'
import { ImageDisplay } from '@/components/dashboard/generic/ImageDisplay'
import { ImageHistory } from '@/components/dashboard/generic/ImageHistory'
import { WebDisplay } from '@/components/dashboard/generic/WebDisplay'
import { MarkdownDisplay } from '@/components/dashboard/generic/MarkdownDisplay'
import { MapDisplay } from '@/components/dashboard/generic/MapDisplay'
import { VideoDisplay } from '@/components/dashboard/generic/VideoDisplay'
import { CustomLayer } from '@/components/dashboard/generic/CustomLayer'

// ---------------------------------------------------------------------------
// Business widget imports
// ---------------------------------------------------------------------------
import { AgentMonitorWidget } from '@/components/dashboard/generic/AgentMonitorWidget'
import { AiAnalyst } from '@/components/dashboard/generic/AiAnalyst'

// ============================================================================
// Adapter components
// ============================================================================

function ValueCardAdapter({ dataSource, title }: WidgetProps) {
  return <ValueCard dataSource={dataSource?.source} title={title} />
}

function LEDIndicatorAdapter({ dataSource, title }: WidgetProps) {
  return <LEDIndicator dataSource={dataSource?.source} title={title} />
}

function SparklineAdapter({ dataSource, title }: WidgetProps) {
  return <Sparkline dataSource={dataSource?.source} title={title} showCard />
}

function ProgressBarAdapter({ dataSource, title }: WidgetProps) {
  return (
    <ProgressBar
      dataSource={dataSource?.source}
      title={title}
      max={dataSource?.max}
      showCard
    />
  )
}

function LineChartAdapter({ dataSource, title }: WidgetProps) {
  return <LineChart dataSource={dataSource?.source} title={title} />
}

function AreaChartAdapter({ dataSource, title }: WidgetProps) {
  return <AreaChart dataSource={dataSource?.source} title={title} />
}

function BarChartAdapter({ dataSource, title }: WidgetProps) {
  return <BarChart dataSource={dataSource?.source} title={title} />
}

function PieChartAdapter({ dataSource, title }: WidgetProps) {
  return <PieChart dataSource={dataSource?.source} title={title} />
}

function ToggleSwitchAdapter({ dataSource, isEditing, title }: WidgetProps) {
  return (
    <ToggleSwitch
      dataSource={dataSource?.source}
      title={title}
      editMode={isEditing}
    />
  )
}

function ImageDisplayAdapter({ dataSource, title }: WidgetProps) {
  return <ImageDisplay dataSource={dataSource?.source} title={title} />
}

function ImageHistoryAdapter({ dataSource, title }: WidgetProps) {
  return <ImageHistory dataSource={dataSource?.source} title={title} />
}

function WebDisplayAdapter({ dataSource, title }: WidgetProps) {
  return <WebDisplay dataSource={dataSource?.source} title={title} />
}

function MarkdownDisplayAdapter({ dataSource }: WidgetProps) {
  return <MarkdownDisplay dataSource={dataSource?.source} />
}

function MapDisplayAdapter({ dataSource }: WidgetProps) {
  return <MapDisplay dataSource={dataSource?.source} />
}

function VideoDisplayAdapter({ dataSource }: WidgetProps) {
  return <VideoDisplay dataSource={dataSource?.source} />
}

function CustomLayerAdapter({ dataSource }: WidgetProps) {
  return <CustomLayer dataSource={dataSource?.source} />
}

function AgentMonitorAdapter({ isEditing }: WidgetProps) {
  return <AgentMonitorWidget editMode={isEditing} />
}

function AiAnalystAdapter({ dataSource, isEditing, title }: WidgetProps) {
  return (
    <AiAnalyst
      dataSource={dataSource?.source}
      title={title}
      editMode={isEditing}
    />
  )
}

// ============================================================================
// Registry map & lookup
// ============================================================================

const WIDGET_ADAPTERS: Record<string, ComponentType<WidgetProps>> = {
  'value-card': ValueCardAdapter,
  'led-indicator': LEDIndicatorAdapter,
  'sparkline': SparklineAdapter,
  'progress-bar': ProgressBarAdapter,
  'line-chart': LineChartAdapter,
  'area-chart': AreaChartAdapter,
  'bar-chart': BarChartAdapter,
  'pie-chart': PieChartAdapter,
  'toggle-switch': ToggleSwitchAdapter,
  'image-display': ImageDisplayAdapter,
  'image-history': ImageHistoryAdapter,
  'web-display': WebDisplayAdapter,
  'markdown-display': MarkdownDisplayAdapter,
  'map-display': MapDisplayAdapter,
  'video-display': VideoDisplayAdapter,
  'custom-layer': CustomLayerAdapter,
  'agent-monitor-widget': AgentMonitorAdapter,
  'ai-analyst': AiAnalystAdapter,
}

/**
 * Get the adapter component for a given widget type.
 * Returns undefined if the type is not a built-in widget.
 */
export function getWidgetComponent(type: WidgetType | string): ComponentType<WidgetProps> | undefined {
  return WIDGET_ADAPTERS[type]
}

/**
 * Check whether a widget type has a registered adapter.
 */
export function hasWidgetAdapter(type: string): boolean {
  return type in WIDGET_ADAPTERS
}
