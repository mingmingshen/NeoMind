/**
 * SimpleRuleBuilderSplit Component
 *
 * Full-screen dialog for creating/editing automation rules.
 * Using unified FullScreenDialog components with glassmorphism style.
 *
 * @module automation
 */

import React, { useState, useEffect, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { generateId } from '@/lib/id'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Plus,
  X,
  Zap,
  Bell,
  FileText,
  Lightbulb,
  Clock,
  AlertTriangle,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  Check,
  Settings,
  Eye,
  Globe,
  Timer,
  Code,
  Puzzle,
  Calendar,
  Play,
  Loader2,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { cardPadded } from '@/design-system/tokens/size'
import { textNano } from "@/design-system/tokens/typography"
import { useIsMobile } from '@/hooks/useMobile'
import type { Rule, RuleTrigger, RuleCondition, RuleAction, DeviceType, Extension, ExtensionDataSourceInfo, ExtensionCommandDescriptor } from '@/types'
// Unified dialog components
import { BuilderShell } from './dialog/BuilderShell'
import { WorkspaceSegmentedControl } from './dialog/WorkspaceSegmentedControl'
import { Field, FieldLabel, FieldMessage } from '@/components/ui/field'
import { Textarea } from '@/components/ui/textarea'
import { Switch } from '@/components/ui/switch'

// ============================================================================
// Utility Functions
// ============================================================================

// Check if an ID is an extension ID (format: extension:extension_id)
// DEPRECATED: Use source_type field instead
function isExtensionId(id: string): boolean {
  return id.startsWith('extension:')
}

// Get extension ID from the formatted ID
// DEPRECATED: Use extension_id field instead
function getExtensionId(formattedId: string): string {
  return formattedId.replace('extension:', '')
}

// Get commands for a resource (device or extension)
function getCommandsForResource(
  id: string,
  devices: Array<{ id: string; name: string; device_type: string; commands?: Array<{ name: string; description: string }> }>,
  deviceTypes?: DeviceType[],
  extensions?: Extension[]
): Array<{ name: string; description: string; display_name?: string }> {
  if (isExtensionId(id)) {
    const extId = getExtensionId(id)
    const ext = extensions?.find((e: Extension) => e.id === extId)
    return ext?.commands?.map((c: ExtensionCommandDescriptor) => ({
      name: c.id,
      description: c.description,
      display_name: c.display_name
    })) || []
  }

  const device = devices?.find((d: any) => d.id === id)
  if (device?.commands) return device.commands

  // Check device type for commands
  if (device && deviceTypes) {
    const dt = deviceTypes.find((t: any) => t.name === device.device_type)
    return dt?.commands?.map((c: any) => ({ name: c.name, description: c.description || '', display_name: c.display_name })) || []
  }

  return []
}

// Get metrics for a resource (device or extension)
function getMetricsForResource(
  id: string,
  devices: Array<{ id: string; name: string; device_type: string; metrics?: Array<{ name: string; data_type: string; unit?: string | null }> }>,
  deviceTypes?: DeviceType[],
  extensions?: Extension[],
  extensionDataSources?: ExtensionDataSourceInfo[]
): Array<{ name: string; data_type: string; unit?: string | null; display_name?: string }> {
  if (isExtensionId(id)) {
    const extId = getExtensionId(id)
    // Return extension data sources as metrics
    return extensionDataSources
      ?.filter((ds: ExtensionDataSourceInfo) => ds.extension_id === extId)
      .map((ds: ExtensionDataSourceInfo) => ({
        name: `${ds.command}.${ds.field}`,
        data_type: ds.data_type,
        unit: ds.unit,
        display_name: ds.display_name,
        extension_id: ds.extension_id,
        command: ds.command,
        field: ds.field,
      })) || []
  }

  const device = devices?.find((d: any) => d.id === id)
  if (device?.metrics) return device.metrics

  // Check device type for metrics
  if (device && deviceTypes) {
    const dt = deviceTypes.find((t: any) => t.name === device.device_type)
    return dt?.metrics?.map((m: any) => ({
      name: m.name,
      data_type: m.data_type || 'string',
      unit: m.unit,
      display_name: m.display_name || m.name
    })) || []
  }

  return []
}

// Get metric data type for a resource
function getMetricDataTypeForResource(
  resourceId: string,
  metricName: string,
  devices: Array<{ id: string; name: string; device_type: string; metrics?: Array<{ name: string; data_type: string }> }>,
  deviceTypes?: DeviceType[],
  extensions?: Extension[],
  extensionDataSources?: ExtensionDataSourceInfo[]
): string {
  if (isExtensionId(resourceId)) {
    const extId = getExtensionId(resourceId)
    const ds = extensionDataSources?.find((d: ExtensionDataSourceInfo) =>
      d.extension_id === extId && `${d.command}.${d.field}` === metricName
    )
    if (ds) return ds.data_type
    return 'string'
  }

  const device = devices?.find((d: any) => d.id === resourceId)
  if (device?.metrics) {
    const metric = device.metrics.find((m: any) => m.name === metricName)
    if (metric) return metric.data_type
  }

  // Check device type
  if (device && deviceTypes) {
    const dt = deviceTypes.find((t: any) => t.name === device.device_type)
    const metric = dt?.metrics?.find((m: any) => m.name === metricName)
    if (metric) return metric.data_type || 'string'
  }

  return 'string'
}

// ============================================================================
// Types
// ============================================================================

interface RuleBuilderProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  rule?: Rule
  onSave: (rule: Partial<Rule>) => Promise<void>
  resources?: {
    devices: Array<{
      id: string
      name: string
      device_type: string
      metrics?: Array<{ name: string; data_type: string; unit?: string | null }>
      commands?: Array<{ name: string; description: string }>
      online?: boolean
    }>
    deviceTypes?: DeviceType[]
    extensions?: Extension[]
    extensionDataSources?: ExtensionDataSourceInfo[]
    messageChannels?: Array<{ name: string; type: string; enabled: boolean }>
  }
}

// ============================================================================
// UI Condition Types
// ============================================================================

type ConditionType = 'simple' | 'range' | 'and' | 'or' | 'not'
type DataSourceType = 'device' | 'extension'

interface UICondition {
  id: string
  type: ConditionType
  source_type?: DataSourceType  // 'device' or 'extension'
  device_id?: string  // Device ID only
  extension_id?: string  // Extension ID only
  metric?: string  // Metric name (for both devices and extensions)
  operator?: string
  threshold?: number
  threshold_value?: string
  range_min?: number
  range_max?: number
  conditions?: UICondition[]
}

interface FormErrors {
  name?: string
  condition?: string[]
  actions?: Record<number, string>
  cron?: string
}

// ============================================================================
// Trigger Types
// ============================================================================

type TriggerType = 'device_state' | 'schedule' | 'manual'

interface CronTemplate {
  id: string
  label: string
  expression: string
  description: string
  icon: React.ReactNode
}

const CRON_TEMPLATES: CronTemplate[] = [
  { id: 'every_minute', label: '每分钟', expression: '* * * * *', description: '每分钟执行', icon: <Timer className="h-4 w-4" /> },
  { id: 'every_5min', label: '每5分钟', expression: '*/5 * * * *', description: '每5分钟执行', icon: <Timer className="h-4 w-4" /> },
  { id: 'every_15min', label: '每15分钟', expression: '*/15 * * * *', description: '每15分钟执行', icon: <Timer className="h-4 w-4" /> },
  { id: 'every_30min', label: '每30分钟', expression: '*/30 * * * *', description: '每30分钟执行', icon: <Timer className="h-4 w-4" /> },
  { id: 'hourly', label: '每小时', expression: '0 * * * *', description: '每小时的第0分钟', icon: <Clock className="h-4 w-4" /> },
  { id: 'daily_midnight', label: '每天午夜', expression: '0 0 * * *', description: '每天00:00', icon: <Calendar className="h-4 w-4" /> },
  { id: 'daily_morning', label: '每天早上', expression: '0 8 * * *', description: '每天08:00', icon: <Calendar className="h-4 w-4" /> },
  { id: 'daily_evening', label: '每天晚上', expression: '0 20 * * *', description: '每天20:00', icon: <Calendar className="h-4 w-4" /> },
  { id: 'weekly_monday', label: '每周一', expression: '0 0 * * 1', description: '每周一00:00', icon: <Calendar className="h-4 w-4" /> },
  { id: 'monthly', label: '每月1号', expression: '0 0 1 * *', description: '每月1号00:00', icon: <Calendar className="h-4 w-4" /> },
  { id: 'workdays_morning', label: '工作日早上', expression: '0 8 * * 1-5', description: '周一至周五08:00', icon: <Calendar className="h-4 w-4" /> },
]

// Get trigger info for display
function getTriggerInfo(type: TriggerType) {
  switch (type) {
    case 'device_state':
      return { label: '设备触发', icon: <Lightbulb className="h-4 w-4" />, color: 'text-accent-purple' }
    case 'schedule':
      return { label: '定时触发', icon: <Clock className="h-4 w-4" />, color: 'text-info' }
    case 'manual':
      return { label: '手动触发', icon: <Play className="h-4 w-4" />, color: 'text-success' }
  }
}

// Calculate next execution time from cron expression
function getNextExecutionTime(cronExpression: string): Date | null {
  try {
    const parts = cronExpression.trim().split(/\s+/)
    if (parts.length !== 5) return null

    const [minute, hour, dayOfMonth, month, dayOfWeek] = parts

    const now = new Date()
    const next = new Date(now)

    // Simple calculation for common patterns
    // For a robust implementation, consider using a cron parser library
    if (minute === '*' && hour === '*' && dayOfMonth === '*' && month === '*' && dayOfWeek === '*') {
      // Every minute
      next.setSeconds(next.getSeconds() + 60)
      return next
    }

    if (minute.startsWith('*/')) {
      const interval = parseInt(minute.slice(2))
      next.setMinutes(next.getMinutes() + (interval - (next.getMinutes() % interval)))
    } else if (minute !== '*') {
      const targetMin = parseInt(minute)
      if (next.getMinutes() >= targetMin) {
        next.setHours(next.getHours() + 1)
      }
      next.setMinutes(targetMin)
    }

    if (hour !== '*' && !minute.startsWith('*/')) {
      const targetHour = parseInt(hour)
      if (next.getHours() >= targetHour) {
        next.setDate(next.getDate() + 1)
      }
      next.setHours(targetHour)
    }

    return next
  } catch {
    return null
  }
}

// ============================================================================
// Helper Functions
// ============================================================================

const getNumericOperators = (t: (key: string) => string) => [
  { value: '>', label: t('dashboardComponents:ruleBuilder.operators.greaterThan'), symbol: '>' },
  { value: '<', label: t('dashboardComponents:ruleBuilder.operators.lessThan'), symbol: '<' },
  { value: '>=', label: t('dashboardComponents:ruleBuilder.operators.greaterOrEqual'), symbol: '≥' },
  { value: '<=', label: t('dashboardComponents:ruleBuilder.operators.lessOrEqual'), symbol: '≤' },
]

const getStringOperators = (t: (key: string) => string) => [
  { value: '==', label: t('dashboardComponents:ruleBuilder.operators.equals'), symbol: '=' },
  { value: '!=', label: t('dashboardComponents:ruleBuilder.operators.notEquals'), symbol: '≠' },
  { value: 'contains', label: t('dashboardComponents:ruleBuilder.operators.contains'), symbol: '∋' },
  { value: 'starts_with', label: t('dashboardComponents:ruleBuilder.operators.startsWith'), symbol: 'a*' },
  { value: 'ends_with', label: t('dashboardComponents:ruleBuilder.operators.endsWith'), symbol: '*z' },
  { value: 'regex', label: t('dashboardComponents:ruleBuilder.operators.regex'), symbol: '.*' },
]

const getBooleanOperators = (t: (key: string) => string) => [
  { value: '==', label: t('dashboardComponents:ruleBuilder.operators.equals'), symbol: '=' },
  { value: '!=', label: t('dashboardComponents:ruleBuilder.operators.notEquals'), symbol: '≠' },
]

const getComparisonOperators = (t: (key: string) => string, dataType?: string) => {
  if (dataType === 'string') return [...getNumericOperators(t), ...getStringOperators(t)]
  if (dataType === 'boolean') return getBooleanOperators(t)
  return [...getNumericOperators(t),
    { value: '==', label: t('dashboardComponents:ruleBuilder.operators.equals'), symbol: '=' },
    { value: '!=', label: t('dashboardComponents:ruleBuilder.operators.notEquals'), symbol: '≠' }
  ]
}

function getDeviceType(
  deviceId: string,
  devices: Array<{ id: string; name: string; device_type?: string }>,
  deviceTypes?: DeviceType[]
): string {
  const device = devices.find(d => d.id === deviceId)
  return device?.device_type || deviceTypes?.[0]?.device_type || ''
}

function getDeviceMetrics(
  deviceId: string,
  devices: Array<{ id: string; name: string; device_type?: string; metrics?: unknown }>,
  deviceTypes?: DeviceType[]
): Array<{ name: string; display_name?: string; data_type?: string }> {
  const device = devices.find(d => d.id === deviceId)
  if (!device) return []

  // If device has metrics directly (from rule resources), use them
  if (device.metrics && Array.isArray(device.metrics)) {
    return device.metrics as Array<{ name: string; display_name?: string; data_type?: string }>
  }

  // Otherwise, look up by device_type
  const deviceTypeName = device.device_type || deviceTypes?.[0]?.device_type || ''
  const deviceType = deviceTypes?.find(t => t.device_type === deviceTypeName)

  // Return metrics or default fallback
  return deviceType?.metrics || [{ name: 'value', display_name: 'Value', data_type: 'float' }]
}

function getMetricDataType(
  metricName: string,
  deviceId: string,
  devices: Array<{ id: string; name: string; device_type?: string; metrics?: unknown }>,
  deviceTypes?: DeviceType[]
): string {
  const metrics = getDeviceMetrics(deviceId, devices, deviceTypes)
  const metric = metrics.find(m => m.name === metricName)
  return metric?.data_type || 'float'
}

function getDeviceCommands(
  deviceId: string,
  devices: Array<{ id: string; name: string; device_type?: string; commands?: unknown }>,
  deviceTypes?: DeviceType[]
): Array<{ name: string; display_name?: string }> {
  // If device has commands directly (from rule resources), use them
  const device = devices.find(d => d.id === deviceId)
  if (device?.commands && Array.isArray(device.commands)) {
    return device.commands as Array<{ name: string; display_name?: string }>
  }

  const deviceTypeName = getDeviceType(deviceId, devices, deviceTypes)
  const deviceType = deviceTypes?.find(t => t.device_type === deviceTypeName)
  return deviceType?.commands || []
}

// ============================================================================
// Extension Helper Functions (V2 - decoupled from devices)
// ============================================================================

// Get extension metrics by extension ID
function getExtensionMetrics(
  extensionId: string,
  extensions: Extension[],
  extensionDataSources: ExtensionDataSourceInfo[]
): Array<{ name: string; display_name?: string; data_type?: string; unit?: string }> {
  const ext = extensions.find(e => e.id === extensionId)
  if (!ext) return []

  // Get data sources for this extension
  const dataSources = extensionDataSources.filter(ds => ds.extension_id === extensionId)

  // Also include metrics from the extension's metadata if available
  const metricsFromDataSources = dataSources.map(ds => ({
    name: ds.field,
    display_name: ds.display_name,
    data_type: ds.data_type,
    unit: ds.unit,
  }))

  // If extension has metrics metadata, include those too
  if (ext.metrics && ext.metrics.length > 0) {
    const extMetrics = ext.metrics.map(m => ({
      name: m.name,
      display_name: m.display_name || m.name,
      data_type: m.data_type,
      unit: m.unit,
    }))
    // Merge, removing duplicates by name
    const merged = new Map()
    metricsFromDataSources.forEach(m => merged.set(m.name, m))
    extMetrics.forEach(m => merged.set(m.name, m))
    return Array.from(merged.values())
  }

  return metricsFromDataSources
}

function getExtensionDataType(
  metricName: string,
  extensionId: string,
  extensions: Extension[],
  extensionDataSources: ExtensionDataSourceInfo[]
): string {
  const metrics = getExtensionMetrics(extensionId, extensions, extensionDataSources)
  const metric = metrics.find(m => m.name === metricName)
  return metric?.data_type || 'float'
}

function getExtensionCommands(
  extensionId: string,
  extensions: Extension[]
): ExtensionCommandDescriptor[] {
  const ext = extensions.find(e => e.id === extensionId)
  return ext?.commands || []
}

function uiConditionToRuleCondition(cond: UICondition): RuleCondition {
  switch (cond.type) {
    case 'simple': {
      let thresholdValue: number | string
      if (cond.threshold_value !== undefined) {
        const parsed = Number(cond.threshold_value)
        if (!isNaN(parsed) && cond.operator !== 'contains' && cond.operator !== 'starts_with' && cond.operator !== 'ends_with' && cond.operator !== 'regex') {
          thresholdValue = parsed
        } else {
          thresholdValue = cond.threshold_value
        }
      } else {
        thresholdValue = cond.threshold ?? 0
      }

      // Build condition based on source_type
      if (cond.source_type === 'extension') {
        return {
          condition_type: 'extension',
          extension_id: cond.extension_id || '',
          extension_metric: cond.metric || 'value',
          operator: cond.operator || '>',
          threshold: thresholdValue,
        }
      }

      return {
        condition_type: 'device',
        device_id: cond.device_id || '',
        metric: cond.metric || 'value',
        operator: cond.operator || '>',
        threshold: thresholdValue,
      }
    }
    case 'range': {
      if (cond.source_type === 'extension') {
        return {
          condition_type: 'extension',
          extension_id: cond.extension_id || '',
          extension_metric: cond.metric || 'value',
          operator: 'between',
          threshold: cond.range_max || 0,
          range_min: cond.range_min,
          range_max: cond.range_max,
        } as RuleCondition
      }

      return {
        condition_type: 'device',
        device_id: cond.device_id || '',
        metric: cond.metric || 'value',
        operator: 'between',
        threshold: cond.range_max || 0,
        range_min: cond.range_min,
        range_max: cond.range_max,
      } as RuleCondition
    }
    case 'and':
      return {
        condition_type: 'logical',
        logical_operator: 'and',
        conditions: cond.conditions?.map(uiConditionToRuleCondition) || [],
      } as RuleCondition
    case 'or':
      return {
        condition_type: 'logical',
        logical_operator: 'or',
        conditions: cond.conditions?.map(uiConditionToRuleCondition) || [],
      } as RuleCondition
    case 'not':
      return {
        condition_type: 'logical',
        logical_operator: 'not',
        conditions: cond.conditions?.map(uiConditionToRuleCondition) || [],
      } as RuleCondition
    default:
      return {
        condition_type: 'device',
        device_id: '',
        metric: 'value',
        operator: '>',
        threshold: 0,
      }
  }
}

function ruleConditionToUiCondition(
  ruleCond?: RuleCondition,
  devices?: Array<{ id: string; name: string; device_type?: string }>,
  dsl?: string
): UICondition {
  if (!ruleCond) {
    return {
      id: generateId(),
      type: 'simple',
      source_type: 'device',
      device_id: '',
      metric: 'value',
      operator: '>',
      threshold: 0,
    }
  }

  // Check for logical conditions first (they have 'conditions' array)
  if ('conditions' in ruleCond && Array.isArray((ruleCond as any).conditions)) {
    const op = (ruleCond as any).logical_operator || (ruleCond as any).operator
    if (op === 'and' || op === 'or') {
      return {
        id: generateId(),
        type: op,
        source_type: undefined,
        conditions: ((ruleCond as any).conditions || []).map((c: RuleCondition) => ruleConditionToUiCondition(c, devices, dsl)),
      }
    }
    if (op === 'not') {
      return {
        id: generateId(),
        type: 'not',
        source_type: undefined,
        conditions: [(ruleCond as any).conditions?.[0]].map((c: RuleCondition) => ruleConditionToUiCondition(c, devices, dsl)).filter(Boolean),
      }
    }
  }

  // Determine source_type from condition_type or extension_id presence
  const isExtension = (ruleCond as any).condition_type === 'extension' || !!(ruleCond as any).extension_id
  const sourceType: DataSourceType = isExtension ? 'extension' : 'device'

  // Check for range condition (has range_min)
  if ('range_min' in ruleCond && (ruleCond as any).range_min !== undefined) {
    const thresholdVal = ruleCond.threshold
    const rangeMax = typeof thresholdVal === 'number' ? thresholdVal :
                     typeof thresholdVal === 'string' ? parseFloat(thresholdVal) || 0 : 0

    if (isExtension) {
      return {
        id: generateId(),
        type: 'range',
        source_type: sourceType,
        extension_id: (ruleCond as any).extension_id || '',
        metric: (ruleCond as any).extension_metric || 'value',
        range_min: (ruleCond as any).range_min,
        range_max: (ruleCond as any).range_max || rangeMax,
      }
    }

    let deviceId = ruleCond.device_id || ''
    let metric = ruleCond.metric || 'value'

    // Try to reconstruct device_id from DSL if missing
    if (!deviceId && dsl && devices) {
      const reconstructed = reconstructDeviceIdFromCondition(ruleCond, dsl, devices)
      deviceId = reconstructed.device_id
      metric = reconstructed.metric || metric
    }

    return {
      id: generateId(),
      type: 'range',
      source_type: sourceType,
      device_id: deviceId,
      metric: metric,
      range_min: (ruleCond as any).range_min,
      range_max: rangeMax,
    }
  }

  // Simple condition
  const thresholdValue = ruleCond.threshold
  const isStringThreshold = typeof thresholdValue === 'string'

  if (isExtension) {
    return {
      id: generateId(),
      type: 'simple',
      source_type: sourceType,
      extension_id: (ruleCond as any).extension_id || '',
      metric: (ruleCond as any).extension_metric || 'value',
      operator: ruleCond.operator || '>',
      threshold: isStringThreshold ? undefined : typeof thresholdValue === 'number' ? thresholdValue : 0,
      threshold_value: isStringThreshold ? thresholdValue : undefined,
    }
  }

  let deviceId = ruleCond.device_id || ''
  let metric = ruleCond.metric || 'value'

  // Try to reconstruct device_id from DSL if missing
  if (!deviceId && dsl && devices) {
    const reconstructed = reconstructDeviceIdFromCondition(ruleCond, dsl, devices)
    deviceId = reconstructed.device_id
    metric = reconstructed.metric || metric
  }

  return {
    id: generateId(),
    type: 'simple',
    source_type: sourceType,
    device_id: deviceId,
    metric: metric,
    operator: ruleCond.operator || '>',
    threshold: isStringThreshold ? undefined : typeof thresholdValue === 'number' ? thresholdValue : 0,
    threshold_value: isStringThreshold ? thresholdValue : undefined,
  }
}

// Helper to reconstruct device_id and metric from DSL
// The backend parses DSL but may lose device_id, so we need to match device names back to IDs
function reconstructDeviceIdFromCondition(
  ruleCond: RuleCondition,
  dsl: string,
  devices: Array<{ id: string; name: string; device_type?: string }>
): { device_id: string; metric?: string } {
  // If we already have device_id, return it
  if (ruleCond.device_id) {
    return { device_id: ruleCond.device_id, metric: ruleCond.metric }
  }

  // The DSL format is: "DeviceName.metric operator threshold"
  // or for range: "DeviceName.metric BETWEEN min AND max"
  const dslLines = dsl.split('\n')
  const whenLine = dslLines.find(line => line.trim().startsWith('WHEN'))
  if (!whenLine) return { device_id: '' }

  // Extract the condition part after "WHEN"
  const conditionPart = whenLine.replace(/^WHEN\s+/i, '').trim()

  // Try to parse the condition to extract device name and metric
  // Format: "DeviceName.metric operator threshold" or "(conditions) operator (conditions)"
  // For range: "DeviceName.metric BETWEEN min AND max"

  // Try range format first
  const rangeMatch = conditionPart.match(/(\S+)\s+BETWEEN\s+(\d+)\s+AND\s+(\d+)/i)
  if (rangeMatch) {
    const [_, path, _min, _max] = rangeMatch
    const result = parseDeviceMetricPath(path, devices)
    return { device_id: result.device_id, metric: result.metric }
  }

  // Try simple format: DeviceName.metric operator threshold
  const simpleMatch = conditionPart.match(/(\S+\.\S+)\s*([<>=!]+)\s*(.+)/)
  if (simpleMatch) {
    const [_, path, _operator, _threshold] = simpleMatch
    const result = parseDeviceMetricPath(path, devices)
    return { device_id: result.device_id, metric: result.metric }
  }

  return { device_id: '' }
}

// Parse device.metric path and match to device_id
function parseDeviceMetricPath(
  path: string,
  devices: Array<{ id: string; name: string; device_type?: string }>
): { device_id: string; metric: string } {
  const parts = path.split('.')
  if (parts.length < 2) return { device_id: '', metric: 'value' }

  const deviceName = parts[0]
  const metric = parts.slice(1).join('.')

  // Try exact name match first
  const exactMatch = devices.find(d => d.name === deviceName)
  if (exactMatch) {
    return { device_id: exactMatch.id, metric }
  }

  // Try case-insensitive match
  const caseMatch = devices.find(d => d.name.toLowerCase() === deviceName.toLowerCase())
  if (caseMatch) {
    return { device_id: caseMatch.id, metric }
  }

  // Try matching device_type if device_name is a type name
  const typeMatch = devices.find(d => d.device_type?.toLowerCase() === deviceName.toLowerCase())
  if (typeMatch) {
    return { device_id: typeMatch.id, metric }
  }

  // Try partial match (device name contains the DSL name)
  const partialMatch = devices.find(d => d.name.toLowerCase().includes(deviceName.toLowerCase()))
  if (partialMatch) {
    return { device_id: partialMatch.id, metric }
  }

  // Try reverse: DSL name contains device name
  const reverseMatch = devices.find(d => deviceName.toLowerCase().includes(d.name.toLowerCase()))
  if (reverseMatch) {
    return { device_id: reverseMatch.id, metric }
  }

  return { device_id: '', metric }
}

// Helper to get device name from ID
function getDeviceNameById(
  deviceId: string,
  devices: Array<{ id: string; name: string; device_type?: string }>
): string {
  const device = devices.find(d => d.id === deviceId)
  return device?.name || deviceId
}

// Helper to get extension name from ID
function getExtensionNameById(
  extensionId: string,
  extensions: Extension[]
): string {
  const ext = extensions.find(e => e.id === extensionId)
  return ext?.name || extensionId
}

// Helper to get the base metric name without duplicate prefix
// Transform-generated metrics have format "prefix.name" (e.g., "ai_result.poses")
// If the device type prefix matches the metric prefix, strip it to avoid duplication
function getMetricPath(
  metric: string,
  deviceId: string,
  devices: Array<{ id: string; name: string; device_type?: string }>
): string {
  if (!metric) return 'value'

  // If metric contains a dot, it might have a prefix from transform output
  const parts = metric.split('.')
  if (parts.length > 1) {
    const prefix = parts[0]
    const device = devices.find(d => d.id === deviceId)

    // Common prefixes that should be stripped (MQTT/device standard prefixes)
    const commonPrefixes = ['values', 'value', 'data', 'telemetry', 'metrics', 'state']
    if (commonPrefixes.includes(prefix)) {
      return parts.slice(1).join('.')
    }

    // Check if the metric prefix matches the device_type
    // If so, strip the prefix to avoid: device_type.device_type.metric_name
    if (device?.device_type && prefix === device.device_type) {
      // Strip the prefix, return just the suffix
      return parts.slice(1).join('.')
    }

    // Also check against device name (in case name is used as prefix)
    const deviceName = device?.name?.toLowerCase().replace(/\s+/g, '_')
    if (deviceName && prefix === deviceName) {
      return parts.slice(1).join('.')
    }
  }

  // Return metric as-is
  return metric
}

function generateRuleDSL(
  name: string,
  condition: RuleCondition | null,
  actions: RuleAction[],
  devices: Array<{ id: string; name: string; device_type?: string }>,
  extensions: Extension[] = [],
  forDuration?: number,
  forUnit?: 'seconds' | 'minutes' | 'hours',
  tags?: string[],
  triggerType?: TriggerType,
  cronExpression?: string
): string {
  const lines: string[] = []
  lines.push(`RULE "${name}"`)
  if (tags && tags.length > 0) {
    lines.push(`TAGS ${tags.join(', ')}`)
  }

  // Add trigger clause based on trigger type
  if (triggerType === 'schedule' && cronExpression) {
    lines.push(`SCHEDULE ${cronExpression}`)
  } else if (triggerType === 'manual') {
    lines.push(`TRIGGER MANUAL`)
  } else if (condition) {
    // Default device_state trigger
    lines.push(`WHEN ${conditionToDSL(condition, devices, extensions)}`)
    if (forDuration && forDuration > 0) {
      const unit = forUnit === 'seconds' ? 'seconds' : forUnit === 'hours' ? 'hours' : 'minutes'
      lines.push(`FOR ${forDuration} ${unit}`)
    }
  }

  lines.push('DO')
  for (const action of actions) {
    lines.push(`    ${actionToDSL(action, devices)}`)
  }
  lines.push('END')
  return lines.join('\n')
}

function parseForClauseFromDSL(dsl?: string): { duration: number; unit: 'seconds' | 'minutes' | 'hours' } | null {
  if (!dsl) return null
  const forMatch = dsl.match(/^FOR\s+(\d+)\s+(seconds|minutes|hours)$/m)
  if (forMatch) {
    const duration = parseInt(forMatch[1], 10)
    const unit = forMatch[2] as 'seconds' | 'minutes' | 'hours'
    return { duration, unit }
  }
  return null
}

function parseScheduleFromDSL(dsl?: string): string | null {
  if (!dsl) return null
  // Match SCHEDULE cron_expression
  const scheduleMatch = dsl.match(/^SCHEDULE\s+([^\n]+)/im)
  if (scheduleMatch) {
    return scheduleMatch[1].trim()
  }
  return null
}

function parseManualTriggerFromDSL(dsl?: string): boolean {
  if (!dsl) return false
  // Match TRIGGER MANUAL
  return /^TRIGGER\s+MANUAL$/im.test(dsl)
}

function conditionToDSL(
  cond: RuleCondition,
  devices: Array<{ id: string; name: string; device_type?: string }>,
  extensions: Extension[] = []
): string {
  const op = (cond as any).logical_operator || (cond as any).operator
  if (op === 'and' || op === 'or') {
    const subConds = ((cond as any).conditions || []) as RuleCondition[]
    if (subConds.length === 0) return 'true'
    const parts = subConds.map(c => conditionToDSL(c, devices, extensions))
    return `(${parts.join(`) ${op.toUpperCase()} (`)})`
  }
  if (op === 'not') {
    const subConds = ((cond as any).conditions || []) as RuleCondition[]
    if (subConds.length === 0) return 'false'
    return `NOT (${conditionToDSL(subConds[0], devices, extensions)})`
  }

  // Check if this is an extension condition
  const isExtension = (cond as any).condition_type === 'extension' || !!(cond as any).extension_id

  if ('range_min' in cond && (cond as any).range_min !== undefined) {
    if (isExtension) {
      const extName = getExtensionNameById((cond as any).extension_id || '', extensions)
      const metric = (cond as any).extension_metric || 'value'
      const min = (cond as any).range_min ?? 0
      const max = (cond as any).range_max ?? 100
      return `EXTENSION ${extName}.${metric} BETWEEN ${min} AND ${max}`
    }
    const deviceName = getDeviceNameById(cond.device_id || '', devices)
    const metric = getMetricPath(cond.metric || 'value', cond.device_id || '', devices)
    const min = (cond as any).range_min ?? 0
    const max = 'range_max' in cond ? ((cond as any).range_max ?? 100) :
                typeof cond.threshold === 'number' ? cond.threshold : 100
    return `${deviceName}.${metric} BETWEEN ${min} AND ${max}`
  }

  if (isExtension) {
    const extName = getExtensionNameById((cond as any).extension_id || '', extensions)
    const metric = (cond as any).extension_metric || 'value'
    const operator = cond.operator || '>'
    let threshold = cond.threshold ?? 0
    if (typeof threshold === 'string') {
      threshold = `"${threshold}"`
    }
    return `EXTENSION ${extName}.${metric} ${operator} ${threshold}`
  }

  const deviceName = getDeviceNameById(cond.device_id || '', devices)
  const metric = getMetricPath(cond.metric || 'value', cond.device_id || '', devices)
  const operator = cond.operator || '>'
  let threshold = cond.threshold ?? 0
  if (typeof threshold === 'string') {
    threshold = `"${threshold}"`
  }
  return `${deviceName}.${metric} ${operator} ${threshold}`
}

function actionToDSL(
  action: RuleAction,
  devices: Array<{ id: string; name: string; device_type?: string }>
): string {
  switch (action.type) {
    case 'Notify': {
      const channels = (action as any).channels && Array.isArray((action as any).channels) && (action as any).channels.length > 0
        ? ` [${(action as any).channels.join(', ')}]`
        : ''
      return `NOTIFY "${action.message}"${channels}`
    }
    case 'Execute':
      const deviceName = getDeviceNameById(action.device_id || '', devices)
      const params = action.params && Object.keys(action.params).length > 0
        ? Object.entries(action.params).map(([k, v]) => `${k}=${v}`).join(', ')
        : ''
      return params ? `EXECUTE ${deviceName}.${action.command}(${params})` : `EXECUTE ${deviceName}.${action.command}`
    case 'Log': return `LOG ${action.level || 'info'}, "${action.message}"`
    case 'Set':
      const setDeviceName = getDeviceNameById(action.device_id || '', devices)
      const value = typeof action.value === 'string' ? `"${action.value}"` : String(action.value)
      return `SET ${setDeviceName}.${action.property} = ${value}`
    case 'Delay': return `DELAY ${action.duration}ms`
    case 'CreateAlert': return `ALERT "${action.title}" "${action.message}" ${(action as any).severity || 'info'}`
    case 'HttpRequest': {
      const method = (action as any).method || 'GET'
      const url = (action as any).url || ''
      const headers = (action as any).headers as Record<string, string> | undefined
      const body = (action as any).body as string | undefined
      let result = `HTTP ${method} ${url}`
      if (headers && Object.keys(headers).length > 0) {
        const headerStr = Object.entries(headers).map(([k, v]) => `${k}:${v}`).join(', ')
        result += ` headers[${headerStr}]`
      }
      if (body) {
        result += ` body="${body}"`
      }
      return result
    }
    default: return '// Unknown action'
  }
}

// ============================================================================
// Local Canvas Components for RuleWorkspace
// ============================================================================

interface ConditionCanvasProps {
  triggerType: TriggerType
  cronExpression: string
  onCronExpressionChange: (v: string) => void
  selectedCronTemplate: string
  onSelectedCronTemplateChange: (v: string) => void
  condition: UICondition | null
  onConditionChange: (c: UICondition) => void
  onAddCondition: () => UICondition
  devices: Array<{
    id: string
    name: string
    device_type: string
    metrics?: Array<{ name: string; data_type: string; unit?: string | null }>
  }>
  deviceTypes?: DeviceType[]
  extensions?: Extension[]
  extensionDataSources?: ExtensionDataSourceInfo[]
  forDuration: number
  onForDurationChange: (v: number) => void
  forUnit: 'seconds' | 'minutes' | 'hours'
  onForUnitChange: (v: 'seconds' | 'minutes' | 'hours') => void
  errors: FormErrors
  t: (key: string) => string
  tBuilder: (key: string) => string
}

function ConditionCanvas({
  triggerType,
  cronExpression,
  onCronExpressionChange,
  selectedCronTemplate,
  onSelectedCronTemplateChange,
  condition,
  onConditionChange,
  onAddCondition,
  devices,
  deviceTypes,
  extensions,
  extensionDataSources,
  forDuration,
  onForDurationChange,
  forUnit,
  onForUnitChange,
  errors,
  t,
  tBuilder,
}: ConditionCanvasProps) {
  const [showCustomCron, setShowCustomCron] = useState(false)

  const handleCronTemplateSelect = (templateId: string) => {
    if (templateId === 'custom') {
      setShowCustomCron(true)
      onSelectedCronTemplateChange('custom')
    } else {
      setShowCustomCron(false)
      const template = CRON_TEMPLATES.find(t => t.id === templateId)
      if (template) {
        onCronExpressionChange(template.expression)
        onSelectedCronTemplateChange(templateId)
      }
    }
  }

  const nextExecution = useMemo(() => {
    if (triggerType === 'schedule') {
      return getNextExecutionTime(cronExpression)
    }
    return null
  }, [triggerType, cronExpression])

  return (
    <div className="space-y-4 p-4 rounded-lg border border-border bg-background">
      {triggerType === 'device_state' && (
        <>
          <div className="flex items-center gap-2 pb-4 border-b">
            <div className="p-2 rounded-full bg-accent-indigo-light">
              <Lightbulb className="h-5 w-5 text-accent-indigo" />
            </div>
            <div>
              <h4 className="text-sm font-medium">{tBuilder('triggerDevice') || 'Device Trigger'}</h4>
              <p className="text-xs text-muted-foreground">{tBuilder('deviceTriggerDesc') || 'Trigger when device state meets conditions'}</p>
            </div>
          </div>

          {!condition && (
            <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
              <ConditionTypeButton
                label={tBuilder('simpleCondition')}
                icon={<Lightbulb className="h-5 w-5" />}
                onClick={() => onConditionChange(onAddCondition())}
              />
              <ConditionTypeButton
                label={tBuilder('rangeCondition')}
                icon={<Globe className="h-5 w-5" />}
                onClick={() => {
                  const c = onAddCondition()
                  c.type = 'range'
                  c.range_min = 0
                  c.range_max = 100
                  onConditionChange(c)
                }}
              />
              <ConditionTypeButton
                label={tBuilder('andCombination')}
                icon={<Check className="h-5 w-5" />}
                onClick={() => {
                  const c = onAddCondition()
                  c.type = 'and'
                  c.conditions = [onAddCondition(), onAddCondition()]
                  onConditionChange(c)
                }}
              />
              <ConditionTypeButton
                label={tBuilder('orCombination')}
                icon={<AlertTriangle className="h-5 w-5" />}
                onClick={() => {
                  const c = onAddCondition()
                  c.type = 'or'
                  c.conditions = [onAddCondition(), onAddCondition()]
                  onConditionChange(c)
                }}
              />
              <ConditionTypeButton
                label={tBuilder('notCondition') || 'NOT'}
                icon={<X className="h-5 w-5" />}
                onClick={() => {
                  const c = onAddCondition()
                  c.type = 'not'
                  c.conditions = [onAddCondition()]
                  onConditionChange(c)
                }}
              />
            </div>
          )}

          {condition && (
            <div className="space-y-4">
              <ConditionEditor
                condition={condition}
                onChange={onConditionChange}
                devices={devices}
                deviceTypes={deviceTypes}
                extensions={extensions}
                extensionDataSources={extensionDataSources}
                t={t}
                tBuilder={tBuilder}
              />

              <div className="flex items-center gap-3 p-4 bg-muted-30 rounded-lg border">
                <Clock className="h-4 w-4 text-muted-foreground shrink-0" />
                <Label className="text-sm font-medium">{tBuilder('duration')}</Label>
                <Input
                  type="number"
                  min={0}
                  value={forDuration}
                  onChange={e => onForDurationChange(parseInt(e.target.value) || 0)}
                  className="w-24 h-9"
                />
                <Select value={forUnit} onValueChange={(v: any) => onForUnitChange(v)}>
                  <SelectTrigger className="w-28 h-9">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="seconds">{tBuilder('seconds')}</SelectItem>
                    <SelectItem value="minutes">{tBuilder('minutes')}</SelectItem>
                    <SelectItem value="hours">{tBuilder('hours')}</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {errors.condition && errors.condition.length > 0 && (
                <div className="p-3 bg-error-light border border-destructive rounded-lg">
                  {errors.condition.map((err, i) => (
                    <p key={i} className="text-sm text-destructive">• {err}</p>
                  ))}
                </div>
              )}
            </div>
          )}
        </>
      )}

      {triggerType === 'schedule' && (
        <>
          <div className="flex items-center gap-2 pb-4 border-b">
            <div className="p-2 rounded-full bg-accent-indigo-light">
              <Clock className="h-5 w-5 text-accent-indigo" />
            </div>
            <div>
              <h4 className="text-sm font-medium">{tBuilder('triggerSchedule') || 'Schedule Trigger'}</h4>
              <p className="text-xs text-muted-foreground">{tBuilder('scheduleTriggerDesc') || 'Execute on a schedule'}</p>
            </div>
          </div>

          <div className="space-y-4">
            <div>
              <Label className="text-xs text-muted-foreground">{tBuilder('cronTemplate') || 'Cron Template'}</Label>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 mt-2">
                {CRON_TEMPLATES.slice(0, 8).map(template => (
                  <button
                    key={template.id}
                    type="button"
                    onClick={() => handleCronTemplateSelect(template.id)}
                    className={cn(
                      "flex flex-col items-center gap-1 p-2 rounded-md border text-xs transition-all",
                      selectedCronTemplate === template.id
                        ? "border-accent-indigo bg-accent-indigo-light text-accent-indigo"
                        : "border-border hover:border-accent-indigo"
                    )}
                  >
                    {template.icon}
                    <span>{template.label}</span>
                  </button>
                ))}
              </div>
            </div>

            <div>
              <div className="flex items-center justify-between mb-2">
                <Label className="text-xs text-muted-foreground">
                  {tBuilder('cronExpression') || 'Cron Expression'}
                </Label>
                <button
                  type="button"
                  onClick={() => {
                    setShowCustomCron(!showCustomCron)
                    if (showCustomCron) {
                      const template = CRON_TEMPLATES.find(t => t.expression === cronExpression)
                      if (template) {
                        onSelectedCronTemplateChange(template.id)
                      } else {
                        onSelectedCronTemplateChange('custom')
                      }
                    } else {
                      onSelectedCronTemplateChange('custom')
                    }
                  }}
                  className="text-xs text-accent-indigo hover:text-accent-indigo"
                >
                  {showCustomCron ? (tBuilder('useTemplate') || 'Use Template') : (tBuilder('customCron') || 'Custom')}
                </button>
              </div>
              {showCustomCron ? (
                <Input
                  type="text"
                  value={cronExpression}
                  onChange={e => onCronExpressionChange(e.target.value)}
                  placeholder="* * * * *"
                  className={cn(
                    "font-mono text-sm h-9",
                    errors.cron && "border-destructive"
                  )}
                />
              ) : (
                <div className="p-3 bg-muted-30 rounded-lg border">
                  <code className="text-sm font-mono">{cronExpression}</code>
                </div>
              )}
              <p className="text-xs text-muted-foreground mt-1">
                {tBuilder('cronFormat') || 'Format: minute hour day month weekday'}
              </p>
            </div>

            {nextExecution && (
              <div className="flex items-center gap-2 p-3 bg-muted-30 rounded-lg border">
                <Calendar className="h-4 w-4 text-success" />
                <span className="text-xs text-muted-foreground">
                  {tBuilder('nextExecution') || 'Next execution'}: {nextExecution.toLocaleString('zh-CN', {
                    month: 'short',
                    day: 'numeric',
                    hour: '2-digit',
                    minute: '2-digit'
                  })}
                </span>
              </div>
            )}
          </div>
        </>
      )}

      {triggerType === 'manual' && (
        <>
          <div className="flex items-center gap-2 pb-4 border-b">
            <div className="p-2 rounded-full bg-accent-indigo-light">
              <Play className="h-5 w-5 text-accent-indigo" />
            </div>
            <div>
              <h4 className="text-sm font-medium">{tBuilder('triggerManual') || 'Manual Trigger'}</h4>
              <p className="text-xs text-muted-foreground">{tBuilder('manualTriggerDesc') || 'This rule must be triggered manually'}</p>
            </div>
          </div>

          <div className="space-y-3">
            <div className="flex items-center gap-3 p-3 bg-muted-30 rounded-lg border">
              <div className="w-6 h-6 shrink-0 rounded-full bg-success-light flex items-center justify-center">
                <span className="text-xs font-medium text-success">1</span>
              </div>
              <p className="text-sm text-muted-foreground">{tBuilder('manualStep1') || 'Click execute button in rule list'}</p>
            </div>
            <div className="flex items-center gap-3 p-3 bg-muted-30 rounded-lg border">
              <div className="w-6 h-6 shrink-0 rounded-full bg-success-light flex items-center justify-center">
                <span className="text-xs font-medium text-success">2</span>
              </div>
              <p className="text-sm text-muted-foreground">{tBuilder('manualStep2') || 'Or call execution API'}</p>
            </div>
          </div>
        </>
      )}
    </div>
  )
}

interface ActionCanvasProps {
  actions: RuleAction[]
  onActionsChange: (actions: RuleAction[]) => void
  devices: Array<{
    id: string
    name: string
    device_type: string
    commands?: Array<{ name: string; description: string }>
    metrics?: Array<{ name: string; data_type: string; unit?: string | null }>
  }>
  deviceTypes?: DeviceType[]
  extensions?: Extension[]
  messageChannels?: Array<{ name: string; type: string; enabled: boolean }>
  t: (key: string) => string
  tBuilder: (key: string) => string
}

function ActionCanvas({ actions, onActionsChange, devices, deviceTypes, extensions, messageChannels, t, tBuilder }: ActionCanvasProps) {
  return (
    <div className="space-y-4">
      {/* Action type buttons */}
      <div className="flex flex-wrap gap-2">
        <Button size="sm" variant="outline" onClick={() => {
          const firstDevice = devices[0]
          const commands = firstDevice ? getCommandsForResource(firstDevice.id, devices, deviceTypes, extensions) : []
          onActionsChange([...actions, { type: 'Execute', device_id: firstDevice?.id || '', command: commands[0]?.name || 'turn_on', params: {} }])
        }}>
          <Zap className="h-4 w-4 mr-1" />
          {tBuilder('executeCommand') || 'Execute'}
        </Button>
        <Button size="sm" variant="outline" onClick={() => {
          const firstDevice = devices[0]
          onActionsChange([...actions, { type: 'Set', device_id: firstDevice?.id || '', property: 'state', value: true }])
        }}>
          <Settings className="h-4 w-4 mr-1" />
          {tBuilder('writeValue') || 'Set'}
        </Button>
        <Button size="sm" variant="outline" onClick={() => onActionsChange([...actions, { type: 'Notify', message: '', channels: [] }])}>
          <Bell className="h-4 w-4 mr-1" />
          {tBuilder('sendNotification') || 'Notify'}
        </Button>
        <Button size="sm" variant="outline" onClick={() => onActionsChange([...actions, { type: 'CreateAlert', title: '', message: '', severity: 'info' }])}>
          <AlertTriangle className="h-4 w-4 mr-1" />
          {tBuilder('createAlert') || 'Alert'}
        </Button>
        <Button size="sm" variant="outline" onClick={() => onActionsChange([...actions, { type: 'HttpRequest', method: 'POST', url: '', headers: {}, body: '' }])}>
          <Globe className="h-4 w-4 mr-1" />
          {tBuilder('httpRequest') || 'HTTP'}
        </Button>
        <Button size="sm" variant="outline" onClick={() => onActionsChange([...actions, { type: 'Log', level: 'info', message: '' }])}>
          <FileText className="h-4 w-4 mr-1" />
          {tBuilder('logRecord') || 'Log'}
        </Button>
        <Button size="sm" variant="outline" onClick={() => onActionsChange([...actions, { type: 'Delay', duration: 5000 }])}>
          <Clock className="h-4 w-4 mr-1" />
          {tBuilder('delay') || 'Delay'}
        </Button>
      </div>

      {/* Actions list — rendered via ActionEditorCompact (handles all 7 types) */}
      <div className="space-y-2">
        {actions.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            <Zap className="h-8 w-8 mx-auto mb-2 opacity-50" />
            <p className="text-sm">{tBuilder('noActionsHint') || 'No actions yet, click buttons above to add'}</p>
          </div>
        ) : (
          actions.map((action, index) => (
            <ActionEditorCompact
              key={index}
              index={index}
              action={action}
              devices={devices}
              deviceTypes={deviceTypes}
              extensions={extensions}
              messageChannels={messageChannels}
              t={t}
              tBuilder={tBuilder}
              onUpdate={(updates) => onActionsChange(actions.map((a, i) => i === index ? ({ ...a, ...updates } as RuleAction) : a))}
              onRemove={() => onActionsChange(actions.filter((_, i) => i !== index))}
            />
          ))
        )}
      </div>
    </div>
  )
}

function ConditionTypeButton({ label, icon, onClick }: { label: string; icon: React.ReactNode; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="p-4 rounded-lg border-2 border-border hover:border-accent-indigo hover:bg-muted-30 transition-all text-left"
    >
      <div className="flex items-center gap-3">
        <div className="p-2 rounded-lg bg-muted-30">{icon}</div>
        <span className="font-medium">{label}</span>
      </div>
    </button>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function SimpleRuleBuilderSplit({
  open,
  onOpenChange,
  rule,
  onSave,
  resources = { devices: [], deviceTypes: [] },
}: RuleBuilderProps) {
  const { t } = useTranslation(['automation', 'common', 'dashboardComponents'])
  const tBuilder = (key: string) => t(`automation:ruleBuilder.${key}`)
  const isEditMode = !!rule
  const isMobile = useIsMobile()

  // Workspace state
  const [workspaceTab, setWorkspaceTab] = useState<'form' | 'dsl'>('form')

  // Form data
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [tags, setTags] = useState<string[]>([])
  const [tagInput, setTagInput] = useState('')
  const [enabled, setEnabled] = useState(true)
  const [triggerType, setTriggerType] = useState<TriggerType>('device_state')
  const [cronExpression, setCronExpression] = useState('0 0 * * *') // Default: daily at midnight
  const [selectedCronTemplate, setSelectedCronTemplate] = useState('daily_midnight')
  const [condition, setCondition] = useState<UICondition | null>(null)
  const [forDuration, setForDuration] = useState<number>(0)
  const [forUnit, setForUnit] = useState<'seconds' | 'minutes' | 'hours'>('minutes')
  const [actions, setActions] = useState<RuleAction[]>([])
  const [saving, setSaving] = useState(false)
  const [formErrors, setFormErrors] = useState<FormErrors>({})

  // Reset when dialog opens or rule changes
  useEffect(() => {
    if (open) {
      if (rule) {
        setName(rule.name || '')
        setDescription(rule.description || '')
        setEnabled(rule.enabled ?? true)
        setTags(rule.tags || (rule as any).source?.tags || [])
        setFormErrors({})

        // Restore trigger type - check trigger field or parse from DSL
        const savedTriggerType = (rule as any).source?.triggerType as TriggerType
        const savedCronExpression = (rule as any).source?.cronExpression as string

        if (rule.trigger?.type === 'schedule' || savedTriggerType === 'schedule') {
          setTriggerType('schedule')
          setCronExpression(savedCronExpression || parseScheduleFromDSL(rule.dsl) || '0 0 * * *')
          // Find matching template
          const matchingTemplate = CRON_TEMPLATES.find(t => t.expression === (savedCronExpression || parseScheduleFromDSL(rule.dsl)))
          setSelectedCronTemplate(matchingTemplate?.id || 'custom')
        } else if (rule.trigger?.type === 'manual' || savedTriggerType === 'manual') {
          setTriggerType('manual')
        } else {
          setTriggerType('device_state')
        }

        // Try to restore from source.uiCondition first (exact restoration)
        const sourceUiCond = (rule as any).source?.uiCondition
        if (sourceUiCond) {
          setCondition(sourceUiCond)
        } else if (rule.condition && triggerType === 'device_state') {
          // Fall back to converting the condition
          // Pass devices and dsl to help reconstruct device_id if missing
          const uiCond = ruleConditionToUiCondition(rule.condition, resources.devices, rule.dsl)
          setCondition(uiCond)
        } else {
          setCondition(null)
        }

        // Restore actions - prefer source.uiActions for exact restoration
        const sourceUiActions = (rule as any).source?.uiActions
        if (sourceUiActions && sourceUiActions.length > 0) {
          setActions(sourceUiActions)
        } else if (rule.actions && rule.actions.length > 0) {
          // Validate and clean up actions to ensure correct structure
          const cleanedActions: RuleAction[] = rule.actions.map(action => {
            // Ensure action has correct structure based on type
            switch (action.type) {
              case 'Log':
                return { type: 'Log', level: (action as any).level || 'info', message: (action as any).message || 'Rule triggered' } as RuleAction
              case 'Notify':
                return { type: 'Notify', message: (action as any).message || '', channels: (action as any).channels || [] } as RuleAction
              case 'Execute':
                return { type: 'Execute', device_id: (action as any).device_id || '', command: (action as any).command || '', params: (action as any).params || {} } as RuleAction
              case 'CreateAlert':
                return { type: 'CreateAlert', title: (action as any).title || '', message: (action as any).message || '', severity: (action as any).severity || 'info' } as RuleAction
              case 'Set':
                return { type: 'Set', device_id: (action as any).device_id || '', property: (action as any).property || 'state', value: (action as any).value ?? true } as RuleAction
              case 'Delay':
                return { type: 'Delay', duration: (action as any).duration || 1000 } as RuleAction
              case 'HttpRequest':
                return {
                  type: 'HttpRequest',
                  method: (action as any).method || 'GET',
                  url: (action as any).url || '',
                  headers: (action as any).headers || {},
                  body: (action as any).body || ''
                } as RuleAction
              default:
                // Unknown action type, default to Log
                return { type: 'Log', level: 'info', message: 'Rule triggered' } as RuleAction
            }
          })
          setActions(cleanedActions)
        } else {
          setActions([])
        }

        // Restore forDuration and forUnit - prefer source values
        const sourceForDuration = (rule as any).source?.forDuration
        const sourceForUnit = (rule as any).source?.forUnit
        if (sourceForDuration !== undefined && sourceForUnit !== undefined) {
          setForDuration(sourceForDuration)
          setForUnit(sourceForUnit)
        } else {
          const forClause = parseForClauseFromDSL(rule.dsl)
          if (forClause) {
            setForDuration(forClause.duration)
            setForUnit(forClause.unit)
          } else {
            setForDuration(0)
            setForUnit('minutes')
          }
        }
      } else {
        resetForm()
      }
    }
  }, [open, rule, resources.devices, resources.deviceTypes])

  const resetForm = useCallback(() => {
    setName('')
    setDescription('')
    setTags([])
    setTagInput('')
    setEnabled(true)
    setTriggerType('device_state')
    setCronExpression('0 0 * * *')
    setSelectedCronTemplate('daily_midnight')
    setCondition(null)
    setForDuration(0)
    setForUnit('minutes')
    // Use a fixed default message instead of translation to avoid issues
    setActions([{ type: 'Log', level: 'info', message: 'Rule triggered' }])
    setFormErrors({})
  }, [])

  const createDefaultCondition = useCallback((): UICondition => {
    // Try devices first, then extensions
    const firstDevice = resources.devices[0]
    const firstExtension = resources.extensions?.[0]

    if (!firstDevice && !firstExtension) {
      return {
        id: generateId(),
        type: 'simple',
        source_type: 'device',
        metric: 'value',
        operator: '>',
        threshold: 0,
      }
    }

    // Use first available resource (prefer device over extension)
    if (firstDevice) {
      const metrics = getDeviceMetrics(firstDevice.id, resources.devices, resources.deviceTypes)
      return {
        id: generateId(),
        type: 'simple',
        source_type: 'device',
        device_id: firstDevice.id,
        metric: metrics[0]?.name || 'value',
        operator: '>',
        threshold: 0,
      }
    } else {
      const metrics = getExtensionMetrics(firstExtension!.id, resources.extensions || [], resources.extensionDataSources || [])
      return {
        id: generateId(),
        type: 'simple',
        source_type: 'extension',
        extension_id: firstExtension!.id,
        metric: metrics[0]?.name || 'value',
        operator: '>',
        threshold: 0,
      }
    }
  }, [resources.devices, resources.deviceTypes, resources.extensions, resources.extensionDataSources])

  // Validate form
  const validate = (): boolean => {
    const errors: FormErrors = {}

    if (!name.trim()) {
      errors.name = tBuilder('ruleNameRequired')
    }

    // Only validate condition for device_state trigger type
    if (triggerType === 'device_state') {
      if (!condition) {
        errors.condition = [tBuilder('addTriggerCondition')]
      } else {
        const validateCondition = (cond: UICondition): string[] => {
          const errs: string[] = []
          if (cond.type === 'simple' || cond.type === 'range') {
            const hasSourceId = cond.source_type === 'extension' ? !!cond.extension_id : !!cond.device_id
            if (!hasSourceId) errs.push(cond.source_type === 'extension' ? (tBuilder('selectExtension') || 'Select extension') : tBuilder('selectDevice'))
            if (!cond.metric) errs.push(tBuilder('selectMetric'))
            if (cond.type === 'simple') {
              const hasValue = cond.threshold !== undefined || cond.threshold_value !== undefined
              if (!hasValue) errs.push(tBuilder('enterThreshold'))
            }
          } else if (cond.type === 'and' || cond.type === 'or' || cond.type === 'not') {
            if (!cond.conditions || cond.conditions.length === 0) {
              errs.push(tBuilder('addSubConditions'))
            } else {
              cond.conditions.forEach((sub) => {
                errs.push(...validateCondition(sub))
              })
            }
          }
          return errs
        }
        const conditionErrors = validateCondition(condition)
        if (conditionErrors.length > 0) {
          errors.condition = conditionErrors
        }
      }
    }

    setFormErrors(errors)
    return Object.keys(errors).length === 0
  }

  // Save
  const handleSave = async () => {
    if (!validate()) return

    setSaving(true)
    try {
      let finalCondition: RuleCondition | undefined = undefined
      if (triggerType === 'device_state' && condition) {
        finalCondition = uiConditionToRuleCondition(condition)
      }

      // Build trigger based on type
      let trigger: RuleTrigger
      if (triggerType === 'schedule') {
        trigger = { type: 'schedule', cron: cronExpression }
      } else if (triggerType === 'manual') {
        trigger = { type: 'manual' }
      } else {
        trigger = { type: 'device_state', device_id: finalCondition?.extension_id ? undefined : (finalCondition?.device_id || ''), extension_id: finalCondition?.extension_id || undefined, state: 'changed' }
      }

      const dsl = generateRuleDSL(name, finalCondition || null, actions, resources.devices, resources.extensions || [], forDuration, forUnit, tags, triggerType, cronExpression)
      const ruleData: Partial<Rule> = {
        name,
        description,
        enabled,
        tags: tags.length > 0 ? tags : undefined,
        trigger,
        condition: finalCondition,
        actions: actions.length > 0 ? actions : undefined,
        dsl,
        // Store original UI state in source field for proper restoration on edit
        source: {
          condition: finalCondition,
          uiCondition: condition, // Store the UI condition for exact restoration
          uiActions: actions, // Store the UI actions for exact restoration
          forDuration,
          forUnit,
          tags,
          triggerType,
          cronExpression,
        } as any,
      }
      if (rule?.id) ruleData.id = rule.id
      await onSave(ruleData)
    } finally {
      setSaving(false)
    }
  }

  // Generate preview DSL
  const previewDSL = useMemo(() => {
  const finalCondition = triggerType === 'device_state' && condition ? uiConditionToRuleCondition(condition) : null
  return generateRuleDSL(
    name || tBuilder('name'),
    finalCondition,
    actions,
    resources.devices,
    resources.extensions || [],
    forDuration,
    forUnit,
    tags,
    triggerType,
    cronExpression
  )
}, [name, condition, actions, resources.devices, resources.extensions, forDuration, forUnit, tags, triggerType, cronExpression, tBuilder])

  // Local RuleWorkspace component
  function RuleWorkspace() {
    return (
      <div className="space-y-4">
        <WorkspaceSegmentedControl
          accent="indigo"
          segments={[
            { value: 'form', label: tBuilder('form') || 'Form' },
            { value: 'dsl', label: tBuilder('dsl') || 'DSL' },
          ]}
          value={workspaceTab}
          onChange={(v) => setWorkspaceTab(v as 'form' | 'dsl')}
        />

        {workspaceTab === 'form' && (
          <div className="space-y-6">
            <ConditionCanvas
              triggerType={triggerType}
              cronExpression={cronExpression}
              onCronExpressionChange={setCronExpression}
              selectedCronTemplate={selectedCronTemplate}
              onSelectedCronTemplateChange={setSelectedCronTemplate}
              condition={condition}
              onConditionChange={setCondition}
              onAddCondition={createDefaultCondition}
              devices={resources.devices}
              deviceTypes={resources.deviceTypes}
              extensions={resources.extensions}
              extensionDataSources={resources.extensionDataSources}
              forDuration={forDuration}
              onForDurationChange={setForDuration}
              forUnit={forUnit}
              onForUnitChange={setForUnit}
              errors={formErrors}
              t={t}
              tBuilder={tBuilder}
            />
            <div className="border-t border-border" />
            <ActionCanvas
              actions={actions}
              onActionsChange={setActions}
              devices={resources.devices}
              deviceTypes={resources.deviceTypes}
              extensions={resources.extensions}
              messageChannels={resources.messageChannels}
              t={t}
              tBuilder={tBuilder}
            />
          </div>
        )}

        {workspaceTab === 'dsl' && (
          <div className="rounded-lg border border-border bg-muted-30 p-4">
            <pre className={cn(textNano, "font-mono overflow-x-auto whitespace-pre-wrap break-all")}>
              {previewDSL || '// No DSL generated'}
            </pre>
          </div>
        )}
      </div>
    )
  }

  return (
    <BuilderShell
      open={open}
      onOpenChange={onOpenChange}
      accent="indigo"
      mobileConfigLabel={t('automation:ruleBuilder.config')}
      title={rule ? t('automation:ruleBuilder.editRule') : t('automation:newRule')}
      subtitle={t('automation:ruleBuilder.subtitle')}
      icon={<Zap className="h-5 w-5" />}
      statusIndicator={
        <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <span className={cn('h-1.5 w-1.5 rounded-full', enabled ? 'bg-success' : 'bg-muted-foreground/40')} />
          {enabled ? t('automation:ruleBuilder.enabled') : t('automation:ruleBuilder.disabled')}
        </span>
      }
      config={
        <div className="space-y-3.5">
          {/* Name */}
          <Field>
            <FieldLabel htmlFor="rule-name">{t('automation:ruleBuilder.ruleName')}</FieldLabel>
            <Input id="rule-name" value={name} onChange={(e) => setName(e.target.value)} />
            {formErrors.name && <FieldMessage>{formErrors.name}</FieldMessage>}
          </Field>

          {/* Description */}
          <Field>
            <FieldLabel htmlFor="rule-desc">{t('automation:ruleBuilder.description')}</FieldLabel>
            <Textarea id="rule-desc" value={description} onChange={(e) => setDescription(e.target.value)} rows={3} />
          </Field>

          {/* Trigger type select */}
          <Field>
            <FieldLabel>{t('automation:ruleBuilder.triggerType')}</FieldLabel>
            <Select value={triggerType} onValueChange={(v) => setTriggerType(v as typeof triggerType)}>
              <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="device_state">{t('automation:ruleBuilder.triggerDevice')}</SelectItem>
                <SelectItem value="schedule">{t('automation:ruleBuilder.triggerSchedule')}</SelectItem>
                <SelectItem value="manual">{t('automation:ruleBuilder.triggerManual')}</SelectItem>
              </SelectContent>
            </Select>
          </Field>

          {/* Tags editor - lifted from BasicInfoStep */}
          <Field>
            <FieldLabel>{t('automation:ruleBuilder.tags') || 'Tags'}</FieldLabel>
            <div className="flex flex-wrap gap-2 p-2 border rounded-md bg-background min-h-[42px]">
              {tags.map(tag => (
                <Badge key={tag} variant="secondary" className="gap-1 pl-2">
                  {tag}
                  <button
                    type="button"
                    onClick={() => setTags(tags.filter(t => t !== tag))}
                    className="rounded-full p-0 hover:bg-muted"
                  >
                    <X className="h-3 w-3" />
                  </button>
                </Badge>
              ))}
              <input
                type="text"
                value={tagInput}
                onChange={e => setTagInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault()
                    const trimmed = tagInput.trim()
                    if (trimmed && !tags.includes(trimmed)) {
                      setTags([...tags, trimmed])
                      setTagInput('')
                    }
                  } else if (e.key === 'Backspace' && !tagInput && tags.length > 0) {
                    setTags(tags.slice(0, -1))
                  }
                }}
                placeholder={tags.length === 0 ? (tBuilder('addTag') || 'Add tag...') : ''}
                className="flex-1 min-w-[80px] outline-none bg-transparent text-sm"
              />
            </div>
          </Field>

          {/* Enabled switch */}
          <div className="flex items-center gap-3">
            <Switch
              id="rule-enabled"
              checked={enabled}
              onCheckedChange={(checked) => setEnabled(!!checked)}
            />
            <Label htmlFor="rule-enabled" className="text-sm font-medium cursor-pointer">
              {tBuilder('enabled')}
            </Label>
          </div>
        </div>
      }
      workspace={<RuleWorkspace />}
      footer={
        <>
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>{t('automation:ruleBuilder.cancel')}</Button>
          </div>
          <div className="flex gap-2">
            <Button onClick={handleSave} disabled={saving}>
              {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {t('automation:ruleBuilder.save')}
            </Button>
          </div>
        </>
      }
    />
  )
}

// ============================================================================
// Condition Editor Component
// ============================================================================

interface ConditionEditorProps {
  condition: UICondition
  onChange: (c: UICondition) => void
  devices: Array<{
    id: string
    name: string
    device_type: string
    metrics?: Array<{ name: string; data_type: string; unit?: string | null }>
    commands?: Array<{ name: string; description: string }>
    online?: boolean
  }>
  deviceTypes?: DeviceType[]
  extensions?: Extension[]
  extensionDataSources?: ExtensionDataSourceInfo[]
  t: (key: string) => string
  tBuilder: (key: string) => string
}

function ConditionEditor({ condition, onChange, devices, deviceTypes, extensions, extensionDataSources, t, tBuilder }: ConditionEditorProps) {
  const updateField = <K extends keyof UICondition>(field: K, value: UICondition[K]) => {
    onChange({ ...condition, [field]: value })
  }

  const updateNestedCondition = (index: number, updates: Partial<UICondition>) => {
    if (!condition.conditions) return
    const newConditions = [...condition.conditions]
    newConditions[index] = { ...newConditions[index], ...updates }
    onChange({ ...condition, conditions: newConditions })
  }

  const removeNestedCondition = (index: number) => {
    if (!condition.conditions) return
    onChange({ ...condition, conditions: condition.conditions.filter((_, i) => i !== index) })
  }

  // Build device options - only devices
  const deviceOptions = devices.map(d => ({ value: d.id, label: d.name, type: 'device' as const }))

  // Build extension options - only extensions
  const extensionOptions = (extensions || []).map(ext => ({
    value: ext.id,
    label: `${ext.name} (${tBuilder('extension') || 'Extension'})`,
    type: 'extension' as const
  }))

  // Get current source type (default to device if not set)
  const currentSourceType = condition.source_type || 'device'

  // Render simple condition
  const renderSimpleCondition = (cond: UICondition) => {
    // Get metrics based on source type
    const metrics = cond.source_type === 'extension' && cond.extension_id
      ? getExtensionMetrics(cond.extension_id, extensions || [], extensionDataSources || [])
      : cond.source_type === 'device' && cond.device_id
      ? getDeviceMetrics(cond.device_id, devices, deviceTypes)
      : []

    // Get metric data type
    const metricDataType = cond.metric && ((cond.source_type === 'extension' && cond.extension_id) || (cond.source_type === 'device' && cond.device_id))
      ? (cond.source_type === 'extension'
          ? (extensionDataSources?.find((ds: ExtensionDataSourceInfo) =>
              ds.extension_id === cond.extension_id && ds.field === cond.metric
            )?.data_type || 'float')
          : getMetricDataTypeForResource(cond.device_id!, cond.metric, devices, deviceTypes, extensions, extensionDataSources))
      : 'float'

    const isStringType = metricDataType === 'string'
    const isBooleanType = metricDataType === 'boolean'
    const isNumericType = ['integer', 'float'].includes(metricDataType)

    const renderValueInput = () => {
      if (isBooleanType) {
        return (
          <Select
            value={cond.threshold_value ?? String(cond.threshold ?? '')}
            onValueChange={(v) => {
              const boolVal = v === 'true'
              updateField('threshold', boolVal ? 1 : 0)
              updateField('threshold_value', v)
            }}
          >
            <SelectTrigger className="w-20 h-9"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="true">true</SelectItem>
              <SelectItem value="false">false</SelectItem>
            </SelectContent>
          </Select>
        )
      }

      if (isStringType || !isNumericType) {
        return (
          <Input
            type="text"
            value={cond.threshold_value ?? String(cond.threshold ?? '')}
            onChange={e => {
              updateField('threshold_value', e.target.value)
            }}
            className="w-28 h-9"
            disabled={!(cond.source_type === 'extension' ? cond.extension_id : cond.device_id)}
          />
        )
      }

      return (
        <Input
          type="number"
          value={cond.threshold ?? ''}
          onChange={e => updateField('threshold', parseFloat(e.target.value) || 0)}
          className="w-24 h-9"
          disabled={!(cond.source_type === 'extension' ? cond.extension_id : cond.device_id)}
        />
      )
    }

    return (
      <div className="p-3 bg-accent-purple-light rounded-lg border border-accent-purple">
        <div className="flex flex-wrap items-center gap-2">
          {/* Source Type Selector */}
          <Select
            value={cond.source_type || 'device'}
            onValueChange={(v: 'device' | 'extension') => {
              // Clear both device_id and extension_id when changing source type
              onChange({ ...condition, source_type: v, device_id: undefined, extension_id: undefined, metric: undefined })
            }}
          >
            <SelectTrigger className="w-28 h-9 text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="device">{tBuilder('device') || 'Device'}</SelectItem>
              <SelectItem value="extension">{tBuilder('extension') || 'Extension'}</SelectItem>
            </SelectContent>
          </Select>

          {/* Device/Extension Selector */}
          <Select
            value={cond.source_type === 'extension' ? cond.extension_id : cond.device_id}
            onValueChange={(v) => {
              const newSourceType = cond.source_type || 'device'
              const newMetrics = newSourceType === 'extension'
                ? getExtensionMetrics(v, extensions || [], extensionDataSources || [])
                : getDeviceMetrics(v, devices, deviceTypes)

              // Update the appropriate ID field and metric
              if (newSourceType === 'extension') {
                onChange({ ...condition, extension_id: v, device_id: undefined, metric: newMetrics[0]?.name || 'value' })
              } else {
                onChange({ ...condition, device_id: v, extension_id: undefined, metric: newMetrics[0]?.name || 'value' })
              }
            }}
          >
            <SelectTrigger className="w-36 h-9 text-sm">
              <SelectValue placeholder={cond.source_type === 'extension' ? tBuilder('selectExtension') : tBuilder('selectDevice')} />
            </SelectTrigger>
            <SelectContent>
              {(cond.source_type === 'extension' ? extensionOptions : deviceOptions).map(d => (
                <SelectItem key={d.value} value={d.value}>{d.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* Metric Selector */}
          {((cond.source_type === 'extension' && cond.extension_id) || (cond.source_type === 'device' && cond.device_id)) && metrics.length > 0 ? (
            <Select value={cond.metric} onValueChange={(v) => updateField('metric', v)}>
              <SelectTrigger className="w-32 h-9 text-sm"><SelectValue /></SelectTrigger>
              <SelectContent>
                {metrics.map(m => (
                  <SelectItem key={m.name} value={m.name}>
                    {m.display_name || m.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ) : (
            <span className="text-xs text-muted-foreground italic">
              {cond.source_type === 'extension' ? tBuilder('selectExtensionFirst') : tBuilder('selectDeviceFirst')}
            </span>
          )}

          {/* Operator Selector */}
          <Select
            value={cond.operator}
            onValueChange={(v) => updateField('operator', v)}
            disabled={!(cond.source_type === 'extension' ? cond.extension_id : cond.device_id)}
          >
            <SelectTrigger className="w-20 h-9 text-sm"><SelectValue /></SelectTrigger>
            <SelectContent>
              {getComparisonOperators((k) => k, metricDataType).map(o => <SelectItem key={o.value} value={o.value}>{o.symbol}</SelectItem>)}
            </SelectContent>
          </Select>

          {renderValueInput()}
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => onChange(null as any)}>
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>
    )
  }

  // Render range condition
  const renderRangeCondition = (cond: UICondition) => {
    // Get metrics based on source type
    const metrics = cond.source_type === 'extension' && cond.extension_id
      ? getExtensionMetrics(cond.extension_id, extensions || [], extensionDataSources || [])
      : cond.source_type === 'device' && cond.device_id
      ? getDeviceMetrics(cond.device_id, devices, deviceTypes)
      : []

    const hasValidId = (cond.source_type === 'extension' && cond.extension_id) || (cond.source_type === 'device' && cond.device_id)

    return (
      <div className="p-3 bg-info-light rounded-lg border border-info">
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="outline" className="text-xs bg-info-light text-info border-info">BETWEEN</Badge>

          {/* Source Type Selector */}
          <Select
            value={cond.source_type || 'device'}
            onValueChange={(v: 'device' | 'extension') => {
              onChange({ ...condition, source_type: v, device_id: undefined, extension_id: undefined, metric: undefined })
            }}
          >
            <SelectTrigger className="w-28 h-9 text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="device">{tBuilder('device') || 'Device'}</SelectItem>
              <SelectItem value="extension">{tBuilder('extension') || 'Extension'}</SelectItem>
            </SelectContent>
          </Select>

          {/* Device/Extension Selector */}
          <Select
            value={cond.source_type === 'extension' ? cond.extension_id : cond.device_id}
            onValueChange={(v) => {
              const newSourceType = cond.source_type || 'device'
              const newMetrics = newSourceType === 'extension'
                ? getExtensionMetrics(v, extensions || [], extensionDataSources || [])
                : getDeviceMetrics(v, devices, deviceTypes)

              if (newSourceType === 'extension') {
                onChange({ ...condition, extension_id: v, device_id: undefined, metric: newMetrics[0]?.name || 'value' })
              } else {
                onChange({ ...condition, device_id: v, extension_id: undefined, metric: newMetrics[0]?.name || 'value' })
              }
            }}
          >
            <SelectTrigger className="w-36 h-9 text-sm">
              <SelectValue placeholder={cond.source_type === 'extension' ? tBuilder('selectExtension') : tBuilder('selectDevice')} />
            </SelectTrigger>
            <SelectContent>
              {(cond.source_type === 'extension' ? extensionOptions : deviceOptions).map(d => (
                <SelectItem key={d.value} value={d.value}>{d.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* Metric Selector */}
          {hasValidId && metrics.length > 0 ? (
            <Select value={cond.metric} onValueChange={(v) => updateField('metric', v)}>
              <SelectTrigger className="w-32 h-9 text-sm"><SelectValue /></SelectTrigger>
              <SelectContent>
                {metrics.map(m => <SelectItem key={m.name} value={m.name}>{m.display_name || m.name}</SelectItem>)}
              </SelectContent>
            </Select>
          ) : (
            <span className="text-xs text-muted-foreground italic">
              {cond.source_type === 'extension' ? tBuilder('selectExtensionFirst') : tBuilder('selectDeviceFirst')}
            </span>
          )}

          <span className="text-xs font-medium text-muted-foreground px-1">BETWEEN</span>
          <Input
            type="number"
            value={cond.range_min}
            onChange={e => updateField('range_min', parseFloat(e.target.value) || 0)}
            className="w-20 h-9"
            placeholder="Min"
            disabled={!hasValidId}
          />
          <span className="text-xs text-muted-foreground">AND</span>
          <Input
            type="number"
            value={cond.range_max}
            onChange={e => updateField('range_max', parseFloat(e.target.value) || 0)}
            className="w-20 h-9"
            placeholder="Max"
            disabled={!hasValidId}
          />
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => onChange(null as any)}>
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>
    )
  }

  // Render logical condition (AND/OR/NOT)
  const renderLogicalCondition = () => {
    const label = condition.type.toUpperCase()
    const badgeClass = condition.type === 'and'
      ? 'bg-success-light text-success border-success-light'
      : condition.type === 'or'
      ? 'bg-warning-light text-warning border-warning'
      : 'bg-error-light text-error border-error'

    return (
      <div className="space-y-3">
        <div className="flex items-center gap-2 p-2.5 bg-muted rounded-t-lg border">
          <Badge variant="outline" className={cn('text-xs px-2.5 py-1', badgeClass)}>{label}</Badge>
          <span className="text-xs text-muted-foreground flex-1">
            {condition.type === 'and' ? tBuilder('allConditionsMustMeet') : condition.type === 'or' ? tBuilder('anyConditionMustMeet') : tBuilder('conditionNotMet')}
          </span>
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => onChange(null as any)}>
            <X className="h-4 w-4" />
          </Button>
        </div>

        <div className="p-3 bg-background border border-t-0 rounded-b-lg space-y-3">
          {condition.conditions?.map((subCond, i) => (
            <div key={subCond.id} className="relative group">
              {i > 0 && (
                <div className="flex items-center justify-start -mb-2 mt-1">
                  <span className={cn(
                    'text-xs font-semibold px-2.5 py-1 rounded-full',
                    condition.type === 'and' ? 'bg-success-light text-success dark:bg-success-light dark:text-success' : 'bg-warning-light text-warning'
                  )}>
                    {condition.type.toUpperCase()}
                  </span>
                </div>
              )}
              <div className="relative pr-8">
                <div className="rounded-lg bg-muted-30">
                  <ConditionEditor
                    condition={subCond}
                    onChange={(c) => updateNestedCondition(i, c)}
                    devices={devices}
                    deviceTypes={deviceTypes}
                    extensions={extensions}
                    extensionDataSources={extensionDataSources}
                    t={t}
                    tBuilder={tBuilder}
                  />
                </div>
                {condition.conditions && condition.conditions.length > 1 && (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 absolute right-0 top-2 opacity-0 group-hover:opacity-100"
                    onClick={() => removeNestedCondition(i)}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                )}
              </div>
            </div>
          ))}

          <div className="pt-2 border-t border-border">
            <Button
              variant="outline"
              size="sm"
              className="w-full border-dashed h-9"
              onClick={() => {
                // Try devices first, then extensions
                const firstDevice = devices[0]
                const firstExtension = extensions?.[0]

                let newCond: UICondition

                if (firstDevice) {
                  const metrics = getDeviceMetrics(firstDevice.id, devices, deviceTypes)
                  newCond = {
                    id: generateId(),
                    type: 'simple',
                    source_type: 'device',
                    device_id: firstDevice.id,
                    metric: metrics[0]?.name || 'value',
                    operator: '>',
                    threshold: 0,
                  }
                } else if (firstExtension) {
                  const metrics = getExtensionMetrics(firstExtension.id, extensions || [], extensionDataSources || [])
                  newCond = {
                    id: generateId(),
                    type: 'simple',
                    source_type: 'extension',
                    extension_id: firstExtension.id,
                    metric: metrics[0]?.name || 'value',
                    operator: '>',
                    threshold: 0,
                  }
                } else {
                  // Fallback - create empty condition
                  newCond = {
                    id: generateId(),
                    type: 'simple',
                    source_type: 'device',
                    operator: '>',
                    threshold: 0,
                  }
                }

                onChange({
                  ...condition,
                  conditions: [...(condition.conditions || []), newCond]
                })
              }}
            >
              <Plus className="h-4 w-4 mr-1" />{tBuilder('addCondition')}
            </Button>
          </div>
        </div>
      </div>
    )
  }

  switch (condition.type) {
    case 'simple': return renderSimpleCondition(condition)
    case 'range': return renderRangeCondition(condition)
    case 'and':
    case 'or':
    case 'not': return renderLogicalCondition()
    default: return null
  }
}

// ============================================================================
// Helper Functions for Set Action
// ============================================================================

function getAvailableMetricsForDevice(
  deviceId: string,
  devices: Array<{ id: string; name: string; metrics?: Array<{ name: string; data_type: string; unit?: string | null }> }>,
  extensions?: Extension[]
): Array<{ name: string; data_type: string; unit?: string | null }> {
  // Find device metrics
  const device = devices.find(d => d.id === deviceId)
  if (device?.metrics) {
    return device.metrics
  }

  // Check if it's an extension
  if (extensions && deviceId.startsWith('extension:')) {
    const extId = deviceId.replace('extension:', '').split(':')[0]
    const extension = extensions.find(e => e.id === extId)
    if (extension?.metrics) {
      return extension.metrics.map(m => ({
        name: m.name,
        data_type: m.data_type,
        unit: m.unit
      }))
    }
  }

  return []
}

function getMetricDataTypeForSet(
  deviceId: string,
  propertyName: string,
  devices: Array<{ id: string; name: string; metrics?: Array<{ name: string; data_type: string; unit?: string | null }> }>,
  extensions?: Extension[]
): string {
  const metrics = getAvailableMetricsForDevice(deviceId, devices, extensions)
  const metric = metrics.find(m => m.name === propertyName)
  return metric?.data_type || 'string'
}

interface MetricValueInputProps {
  deviceId: string
  propertyName: string
  value: unknown
  devices: Array<{ id: string; name: string; metrics?: Array<{ name: string; data_type: string; unit?: string | null }> }>
  extensions?: Extension[]
  onChange: (value: unknown) => void
  tBuilder: (key: string) => string
}

function MetricValueInput({ deviceId, propertyName, value, devices, extensions, onChange, tBuilder }: MetricValueInputProps) {
  const dataType = propertyName
    ? getMetricDataTypeForSet(deviceId, propertyName, devices, extensions)
    : 'string'

  if (dataType === 'boolean' || dataType === 'bool') {
    return (
      <Select
        value={value === undefined ? '' : String(value)}
        onValueChange={(v) => onChange(v === 'true')}
      >
        <SelectTrigger className="w-20 h-9 text-sm flex-shrink-0">
          <SelectValue placeholder={tBuilder('valuePlaceholder')} />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="true">true</SelectItem>
          <SelectItem value="false">false</SelectItem>
        </SelectContent>
      </Select>
    )
  }

  if (dataType === 'number' || dataType === 'integer' || dataType === 'float') {
    return (
      <Input
        type="number"
        value={String(value ?? '')}
        onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
        placeholder={tBuilder('valuePlaceholder')}
        className="w-24 h-9 text-sm flex-shrink-0"
      />
    )
  }

  // Default: string input
  return (
    <Input
      type="text"
      value={String(value ?? '')}
      onChange={(e) => onChange(e.target.value)}
      placeholder={tBuilder('valuePlaceholder')}
      className="w-24 h-9 text-sm flex-shrink-0"
    />
  )
}

// ============================================================================
// Action Editor Component
// ============================================================================

interface ActionEditorCompactProps {
  action: RuleAction
  index: number
  devices: Array<{
    id: string
    name: string
    device_type: string
    commands?: Array<{ name: string; description: string }>
    metrics?: Array<{ name: string; data_type: string; unit?: string | null }>
  }>
  deviceTypes?: DeviceType[]
  extensions?: Extension[]
  messageChannels?: Array<{ name: string; type: string; enabled: boolean }>
  t: (key: string) => string
  tBuilder: (key: string) => string
  onUpdate: (data: Partial<RuleAction>) => void
  onRemove: () => void
}

function ActionEditorCompact({ action, devices, deviceTypes, extensions, messageChannels, t, tBuilder, onUpdate, onRemove }: ActionEditorCompactProps) {
  // Build device/extension options for Execute action
  const deviceOptions = [
    ...devices.map(d => ({ value: d.id, label: d.name, type: 'device' as const })),
    ...(extensions || []).map(ext => ({
      value: `extension:${ext.id}`,
      label: `${ext.name} (Extension)`,
      type: 'extension' as const
    }))
  ]

  const renderActionContent = () => {
    switch (action.type) {
      case 'Execute': {
        const commands = getCommandsForResource(action.device_id || '', devices, deviceTypes, extensions)
        const isExtension = action.device_id && isExtensionId(action.device_id)
        return (
          <div className="space-y-2 w-full">
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground w-20">
                {isExtension ? t('extensions:extension') : t('automation:device')}:
              </span>
              <Select
                value={action.device_id}
                onValueChange={(v) => {
                  const cmds = getCommandsForResource(v, devices, deviceTypes, extensions)
                  onUpdate({ device_id: v, command: cmds[0]?.name || 'turn_on' })
                }}
              >
                <SelectTrigger className="h-8 text-sm flex-1 max-w-xs">
                  <SelectValue placeholder={isExtension ? tBuilder('selectExtension') : t('automation:selectDevice')} />
                </SelectTrigger>
                <SelectContent>
                  {deviceOptions.map(d => <SelectItem key={d.value} value={d.value}>{d.label}</SelectItem>)}
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground w-20">{t('dashboardComponents:ruleBuilder.command')}:</span>
              <Select value={action.command} onValueChange={(v) => onUpdate({ command: v })}>
                <SelectTrigger className="h-8 text-sm flex-1 max-w-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {commands.map(c => (
                    <SelectItem key={c.name} value={c.name}>{c.display_name || c.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        )
      }

      case 'Notify': {
        const currentChannels = ((action as any).channels || []) as string[]
        return (
          <div className="space-y-2 w-full">
            <Input
              value={action.message}
              onChange={(e) => onUpdate({ message: e.target.value })}
              placeholder={tBuilder('notificationContentPlaceholder')}
              className="h-8 text-sm"
            />
            {messageChannels && messageChannels.length > 0 && (
              <Popover>
                <PopoverTrigger asChild>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-8 w-full justify-start text-sm font-normal"
                  >
                    {currentChannels.length === 0
                      ? t('automation:channels')
                      : currentChannels.length === 1
                        ? currentChannels[0]
                        : `${currentChannels.length} ${t('automation:channels')}`}
                    <ChevronDown className="ml-auto h-4 w-4 opacity-50" />
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-56 p-2" align="start">
                  <div className="space-y-1">
                    {messageChannels.filter(ch => ch.enabled).map((ch) => (
                      <div
                        key={ch.name}
                        className="flex items-center gap-2 px-2 py-1.5 rounded-sm hover:bg-accent cursor-pointer"
                        onClick={() => {
                          const newChannels = currentChannels.includes(ch.name)
                            ? currentChannels.filter(c => c !== ch.name)
                            : [...currentChannels, ch.name]
                          onUpdate({ channels: newChannels })
                        }}
                      >
                        <Checkbox
                          checked={currentChannels.includes(ch.name)}
                          className="pointer-events-none"
                        />
                        <span className="text-sm flex-1">{ch.name}</span>
                      </div>
                    ))}
                  </div>
                </PopoverContent>
              </Popover>
            )}
          </div>
        )
      }

      case 'Log': {
        return (
          <div className="flex items-center gap-2 w-full">
            <Select value={action.level} onValueChange={(v: any) => onUpdate({ level: v })}>
              <SelectTrigger className="w-20 h-8 text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="debug">{t('dashboardComponents:ruleBuilder.logLevels.debug')}</SelectItem>
                <SelectItem value="info">{t('dashboardComponents:ruleBuilder.logLevels.info')}</SelectItem>
                <SelectItem value="warn">{t('dashboardComponents:ruleBuilder.logLevels.warn')}</SelectItem>
                <SelectItem value="error">{t('dashboardComponents:ruleBuilder.logLevels.error')}</SelectItem>
              </SelectContent>
            </Select>
            <Input
              value={action.message}
              onChange={(e) => onUpdate({ message: e.target.value })}
              placeholder={tBuilder('logContentPlaceholder')}
              className="flex-1 h-8 text-sm"
            />
          </div>
        )
      }

      case 'Set': {
        return (
          <div className="space-y-2 w-full">
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground w-12">{t('automation:device')}:</span>
              <Select value={action.device_id} onValueChange={(v) => onUpdate({ device_id: v, property: '' })}>
                <SelectTrigger className="h-8 text-sm flex-1 max-w-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {deviceOptions.map(d => <SelectItem key={d.value} value={d.value}>{d.label}</SelectItem>)}
                </SelectContent>
              </Select>
            </div>
            {action.device_id && (
              <>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground w-12">{t('automation:property')}:</span>
                  <Select value={action.property} onValueChange={(v) => onUpdate({ property: v })}>
                    <SelectTrigger className="h-8 text-sm flex-1">
                      <SelectValue placeholder={tBuilder('propertyNamePlaceholder')} />
                    </SelectTrigger>
                    <SelectContent>
                      {getAvailableMetricsForDevice(action.device_id, devices, extensions).map(m => (
                        <SelectItem key={m.name} value={m.name}>
                          {m.name}
                          {m.unit && <span className="text-muted-foreground ml-1">({m.unit})</span>}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground w-12">{t('dashboardComponents:ruleBuilder.value')}:</span>
                  <MetricValueInput
                    deviceId={action.device_id}
                    propertyName={action.property}
                    value={action.value}
                    devices={devices}
                    extensions={extensions}
                    onChange={(value) => onUpdate({ value })}
                    tBuilder={tBuilder}
                  />
                </div>
              </>
            )}
          </div>
        )
      }

      case 'Delay': {
        return (
          <div className="flex items-center gap-2">
            <Input
              type="number"
              value={(action.duration || 0) / 1000}
              onChange={(e) => onUpdate({ duration: (parseInt(e.target.value) || 0) * 1000 })}
              className="w-20 h-8 text-sm"
            />
            <span className="text-xs text-muted-foreground">{tBuilder('seconds')}</span>
          </div>
        )
      }

      case 'CreateAlert': {
        return (
          <div className="space-y-2 w-full">
            <Input
              value={action.title}
              onChange={(e) => onUpdate({ title: e.target.value })}
              placeholder={tBuilder('alertTitlePlaceholder')}
              className="h-8 text-sm"
            />
            <Input
              value={action.message}
              onChange={(e) => onUpdate({ message: e.target.value })}
              placeholder={tBuilder('alertMessagePlaceholder')}
              className="h-8 text-sm"
            />
            <Select value={action.severity} onValueChange={(v: any) => onUpdate({ severity: v })}>
              <SelectTrigger className="w-24 h-8 text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="info">{t('dashboardComponents:ruleBuilder.severity.info')}</SelectItem>
                <SelectItem value="warning">{t('dashboardComponents:ruleBuilder.severity.warning')}</SelectItem>
                <SelectItem value="error">{t('dashboardComponents:ruleBuilder.severity.error')}</SelectItem>
                <SelectItem value="critical">{t('dashboardComponents:ruleBuilder.severity.critical')}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        )
      }

      case 'HttpRequest': {
        const headers = (action as any).headers as Record<string, string> || {}
        const body = (action as any).body as string || ''
        return (
          <div className="space-y-2 w-full">
            {/* Method + URL */}
            <div className="flex items-center gap-2">
              <Select value={action.method} onValueChange={(v: any) => onUpdate({ method: v })}>
                <SelectTrigger className="w-20 h-8 text-sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="GET">GET</SelectItem>
                  <SelectItem value="POST">POST</SelectItem>
                  <SelectItem value="PUT">PUT</SelectItem>
                  <SelectItem value="DELETE">DELETE</SelectItem>
                  <SelectItem value="PATCH">PATCH</SelectItem>
                </SelectContent>
              </Select>
              <Input
                value={action.url}
                onChange={(e) => onUpdate({ url: e.target.value })}
                placeholder={tBuilder('urlPlaceholder') || 'https://example.com/api'}
                className="flex-1 h-8 text-sm font-mono"
              />
            </div>

            {/* Headers */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs text-muted-foreground">{t('automation:headers')}</span>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-5 px-1 text-xs text-muted-foreground hover:text-foreground"
                  onClick={() => onUpdate({ headers: { ...headers, '': '' } })}
                >
                  <Plus className="h-4 w-4 mr-0.5" />
                  {t('automation:add') || 'Add'}
                </Button>
              </div>
              {Object.keys(headers).length === 0 ? (
                <div className="text-xs text-muted-foreground py-1 italic">
                  {t('automation:noHeaders') || 'No headers'}
                </div>
              ) : (
                <div className="space-y-1">
                  {Object.entries(headers).map(([key, value], idx) => (
                    <div key={idx} className="flex items-center gap-1">
                      <Input
                        type="text"
                        value={key}
                        onChange={(e) => {
                          const newHeaders = { ...headers }
                          if (key !== e.target.value) {
                            delete newHeaders[key]
                          }
                          newHeaders[e.target.value] = headers[key]
                          onUpdate({ headers: newHeaders })
                        }}
                        placeholder={t('automation:headerKey') || 'Key'}
                        className="h-7 text-xs flex-1"
                      />
                      <span className="text-muted-foreground text-xs">:</span>
                      <Input
                        type="text"
                        value={value}
                        onChange={(e) => onUpdate({ headers: { ...headers, [key]: e.target.value } })}
                        placeholder={t('automation:headerValue') || 'Value'}
                        className="h-7 text-xs flex-1"
                      />
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 flex-shrink-0"
                        onClick={() => {
                          const newHeaders = { ...headers }
                          delete newHeaders[key]
                          onUpdate({ headers: newHeaders })
                        }}
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Body */}
            <div>
              <span className="text-xs text-muted-foreground block mb-1">{t('automation:body')}</span>
              <Input
                type="text"
                value={body}
                onChange={(e) => onUpdate({ body: e.target.value })}
                placeholder={t('automation:bodyPlaceholder') || 'Request body (JSON, text, etc.)'}
                className="h-8 text-sm"
              />
            </div>
          </div>
        )
      }

      default:
        return null
    }
  }

  const getActionIcon = () => {
    switch (action.type) {
      case 'Execute': return <Zap className="h-4 w-4" />
      case 'Notify': return <Bell className="h-4 w-4" />
      case 'Log': return <FileText className="h-4 w-4" />
      case 'Set': return <Globe className="h-4 w-4" />
      case 'Delay': return <Timer className="h-4 w-4" />
      case 'CreateAlert': return <AlertTriangle className="h-4 w-4" />
      case 'HttpRequest': return <Globe className="h-4 w-4" />
      default: return <Zap className="h-4 w-4" />
    }
  }

  const getActionLabel = (): string => {
    switch (action.type) {
      case 'Execute': return tBuilder('executeCommand')
      case 'Notify': return tBuilder('sendNotification')
      case 'Log': return tBuilder('logRecord')
      case 'Set': return tBuilder('writeValue')
      case 'Delay': return tBuilder('delay')
      case 'CreateAlert': return tBuilder('createAlert')
      case 'HttpRequest': return tBuilder('httpRequest')
    }
  }

  const getActionColor = (): string => {
    switch (action.type) {
      case 'Execute': return 'text-warning bg-warning-light border-warning'
      case 'Notify': return 'text-info bg-info-light border-info'
      case 'Log': return 'text-muted-foreground bg-muted border-border'
      case 'Set': return 'text-accent-purple bg-accent-purple-light border-accent-purple-light'
      case 'Delay': return 'text-accent-orange bg-accent-orange-light border-accent-orange-light'
      case 'CreateAlert': return 'text-error bg-error-light border-error'
      case 'HttpRequest': return 'text-success dark:text-success bg-success-light dark:bg-success-light border-success-light dark:border-success-light'
      default: return 'text-muted-foreground bg-muted border-border'
    }
  }

  return (
    <div className={cn(
      "group rounded-lg border bg-card p-3 shadow-sm transition-all hover:shadow-md",
      getActionColor()
    )}>
      <div className="flex items-start gap-3">
        {/* Action Icon */}
        <div className={cn(
          "flex items-center justify-center w-8 h-8 rounded-full flex-shrink-0",
          getActionColor().replace('text-', 'bg-opacity-10 ')
        )}>
          {getActionIcon()}
        </div>

        {/* Action Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between mb-1">
            <span className="text-sm font-medium">{getActionLabel()}</span>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={onRemove}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
          {renderActionContent()}
        </div>
      </div>
    </div>
  )
}

