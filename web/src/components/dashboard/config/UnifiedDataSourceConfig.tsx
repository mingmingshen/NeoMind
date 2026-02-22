/**
 * Unified DataSource Configuration
 *
 * Simplified data source selector with direct list-based selection.
 * For device-metric and device-command categories, shows device list on left.
 */

import { useState, useMemo, useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { Search, Check, Server, Zap, Info, X, ChevronRight, ChevronLeft, Circle, Loader2, Database, MapPin, Activity, Puzzle } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useStore } from '@/store'
import type { DataSource, DataSourceOrList } from '@/types/dashboard'
import { normalizeDataSource } from '@/types/dashboard'
import type { MetricDefinition, CommandDefinition } from '@/types'
import { useDataAvailability } from '@/hooks/useDataAvailability'
import { useIsMobile, useSafeAreaInsets } from '@/hooks/useMobile'
import { useMobileBodyScrollLock } from '@/hooks/useBodyScrollLock'
import { api } from '@/lib/api'
import type { Extension, ExtensionDataSourceInfo, ExtensionCommandDescriptor, TransformDataSourceInfo } from '@/types'

// ============================================================================
// Types
// ============================================================================

export interface UnifiedDataSourceConfigProps {
  value?: DataSourceOrList
  onChange: (dataSource: DataSourceOrList | undefined) => void
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command' | 'system' | 'extension' | 'extension-command'>
  multiple?: boolean
  maxSources?: number
  className?: string
  disabled?: boolean
}

type CategoryType = 'device-metric' | 'device-command' | 'device' | 'system' | 'extension' | 'extension-command'
type SelectedItem = string // Format: "device-metric:deviceId:property" or "device-command:deviceId:command" or "device:deviceId" or "system:metric" or "extension:extensionId:metric" or "extension-command:extensionId:command"

// ============================================================================
// Constants
// ============================================================================

// Device info property definitions factory (uses translations)
function getDeviceInfoProperties(t: (key: string) => string) {
  return [
    { id: 'name', name: t('dataSource.deviceName') },
    { id: 'status', name: t('dataSource.status') },
    { id: 'online', name: t('dataSource.onlineStatus') },
    { id: 'last_seen', name: t('dataSource.lastSeen') },
    { id: 'device_type', name: t('dataSource.deviceType') },
    { id: 'plugin_name', name: t('dataSource.adapter') },
    { id: 'adapter_id', name: t('dataSource.adapterId') },
  ]
}

// System metrics definitions factory (uses translations)
function getSystemMetrics(t: (key: string) => string) {
  return [
    { id: 'uptime', name: t('systemDataSource.uptime'), description: t('systemDataSource.uptimeDesc'), unit: '', dataType: 'number' as const },
    { id: 'cpu_count', name: t('systemDataSource.cpuCount'), description: t('systemDataSource.cpuCountDesc'), unit: ' cores', dataType: 'number' as const },
    { id: 'total_memory', name: t('systemDataSource.totalMemory'), description: t('systemDataSource.totalMemoryDesc'), unit: ' GB', dataType: 'bytes' as const },
    { id: 'used_memory', name: t('systemDataSource.usedMemory'), description: t('systemDataSource.usedMemoryDesc'), unit: ' GB', dataType: 'bytes' as const },
    { id: 'free_memory', name: t('systemDataSource.freeMemory'), description: t('systemDataSource.freeMemoryDesc'), unit: ' GB', dataType: 'bytes' as const },
    { id: 'available_memory', name: t('systemDataSource.availableMemory'), description: t('systemDataSource.availableMemoryDesc'), unit: ' GB', dataType: 'bytes' as const },
    { id: 'memory_percent', name: t('systemDataSource.memoryPercent'), description: t('systemDataSource.memoryPercentDesc'), unit: '%', dataType: 'number' as const },
    { id: 'platform', name: t('systemDataSource.platform'), description: t('systemDataSource.platformDesc'), unit: '', dataType: 'string' as const },
    { id: 'arch', name: t('systemDataSource.arch'), description: t('systemDataSource.archDesc'), unit: '', dataType: 'string' as const },
    { id: 'version', name: t('systemDataSource.version'), description: t('systemDataSource.versionDesc'), unit: '', dataType: 'string' as const },
  ]
}

// Category configuration factory (uses translations)
function getCategories(t: (key: string) => string) {
  return [
    { id: 'device' as const, name: t('dataSource.device'), icon: MapPin, description: t('dataSource.deviceDesc') },
    { id: 'device-metric' as const, name: t('dataSource.metrics'), icon: Server, description: t('dataSource.metricsDesc') },
    { id: 'device-command' as const, name: t('dataSource.commands'), icon: Zap, description: t('dataSource.commandsDesc') },
    { id: 'system' as const, name: t('systemDataSource.title'), icon: Activity, description: t('systemDataSource.description') },
    { id: 'extension' as const, name: t('extensions:dataSource.extensionSource') || 'Extension Metrics', icon: Puzzle, description: t('extensions:dataSource.selectExtension') || 'Select extension metrics' },
    { id: 'extension-command' as const, name: t('extensions:dataSource.extensionCommand') || 'Extension Commands', icon: Zap, description: t('extensions:dataSource.selectExtensionCommand') || 'Select extension commands' },
  ]
}

// ============================================================================
// Helper Functions
// ============================================================================

// Convert old allowedTypes format to new format
function normalizeAllowedTypes(
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command' | 'system' | 'extension' | 'extension-command'>
): CategoryType[] {
  if (!allowedTypes) return ['device', 'device-metric', 'device-command', 'system', 'extension', 'extension-command']

  const result: CategoryType[] = []

  // Device category (for map markers, etc.)
  if (allowedTypes.includes('device')) result.push('device')

  // New format types
  if (allowedTypes.includes('device-metric')) result.push('device-metric')
  if (allowedTypes.includes('device-command')) result.push('device-command')
  if (allowedTypes.includes('system')) result.push('system')
  if (allowedTypes.includes('extension')) result.push('extension')
  if (allowedTypes.includes('extension-command')) result.push('extension-command')

  // Old format types - map to new format (but not 'device' since it's distinct now)
  if (allowedTypes.includes('metric')) {
    if (!result.includes('device-metric')) result.push('device-metric')
  }
  if (allowedTypes.includes('command')) {
    // When 'command' is specified, include both device-command and extension-command
    if (!result.includes('device-command')) result.push('device-command')
    if (!result.includes('extension-command')) result.push('extension-command')
  }

  return result.length > 0 ? result : ['device', 'device-metric', 'device-command', 'system', 'extension', 'extension-command']
}

/**
 * Convert selected items to DataSource format
 */
function selectedItemsToDataSource(
  selectedItems: Set<SelectedItem>,
  multiple: boolean
): DataSourceOrList | DataSource | undefined {
  if (selectedItems.size === 0) return undefined
  if (!multiple && selectedItems.size === 1) {
    const item = [...selectedItems][0]!
    const parts = item.split(':')
    const type = parts[0]

    switch (type) {
      case 'device':
        // Device location marker - just store device reference
        return {
          type: 'device',
          deviceId: parts[1],
        }
      case 'device-metric':
        return {
          type: 'telemetry',
          deviceId: parts[1],
          metricId: parts.slice(2).join(':'),
          timeRange: 1,  // 1 hour for real-time dashboards (was 24, too large)
          limit: 50,     // Reduced from 100 for better performance
          aggregate: 'raw',
          params: { includeRawPoints: true },
          transform: 'raw',
        }
      case 'device-command':
        return {
          type: 'command',
          deviceId: parts[1],
          command: parts.slice(2).join(':'),
        }
      case 'device-info':
        return {
          type: 'device-info',
          deviceId: parts[1],
          infoProperty: parts.slice(2).join(':') as any,
        }
      case 'system':
        // Format: system:metricId (not system:deviceId:metricId)
        return {
          type: 'system',
          systemMetric: parts.slice(1).join(':') as any,
          refresh: 10,
        }
      case 'extension':
        // Format: extension:extensionId:metric
        // For provider extension metrics, use "produce:metric" format
        return {
          type: 'extension',
          extensionId: parts[1],
          extensionMetric: `produce:${parts[2]}`,
          refresh: 10,  // Auto-refresh every 10 seconds
          timeRange: 1,
          limit: 50,
          aggregate: 'raw',
          params: { includeRawPoints: true },
          transform: 'raw',
        } as any
      case 'extension-command':
        // Format: extension-command:extensionId:commandName
        return {
          type: 'extension-command',
          extensionId: parts[1],
          command: parts.slice(2).join(':'),
        } as any
      default:
        return undefined
    }
  }

  // Multiple selection - return array
  const result: DataSource[] = []
  for (const item of selectedItems) {
    const parts = item.split(':')
    const type = parts[0]

    switch (type) {
      case 'device':
        result.push({
          type: 'device',
          deviceId: parts[1],
        })
        break
      case 'device-metric':
        result.push({
          type: 'telemetry',
          deviceId: parts[1],
          metricId: parts.slice(2).join(':'),
          timeRange: 1,  // 1 hour for real-time dashboards (was 24, too large)
          limit: 50,     // Reduced from 100 for better performance
          aggregate: 'raw',
          params: { includeRawPoints: true },
          transform: 'raw',
        })
        break
      case 'device-command':
        result.push({
          type: 'command',
          deviceId: parts[1],
          command: parts.slice(2).join(':'),
        })
        break
      case 'device-info':
        result.push({
          type: 'device-info',
          deviceId: parts[1],
          infoProperty: parts.slice(2).join(':') as any,
        })
        break
      case 'system':
        // Format: system:metricId (not system:deviceId:metricId)
        result.push({
          type: 'system',
          systemMetric: parts.slice(1).join(':') as any,
          refresh: 10,
        })
        break
      case 'extension':
        result.push({
          type: 'extension',
          extensionId: parts[1],
          extensionMetric: `produce:${parts[2]}`,
          refresh: 10,  // Auto-refresh every 10 seconds
          timeRange: 1,
          limit: 50,
          aggregate: 'raw',
          params: { includeRawPoints: true },
          transform: 'raw',
        } as any)
        break
      case 'extension-command':
        result.push({
          type: 'extension-command',
          extensionId: parts[1],
          command: parts.slice(2).join(':'),
        } as any)
        break
    }
  }

  return result
}

/**
 * Parse current data source to selected items
 */
function dataSourceToSelectedItems(ds: DataSourceOrList | undefined): Set<SelectedItem>  {
  const items = new Set<SelectedItem>()
  if (!ds) return items

  const dataSources = normalizeDataSource(ds)

  for (const dataSource of dataSources) {
    switch (dataSource.type) {
      case 'device':
        // Plain device reference (for map markers) - no property/metric
        items.add(`device:${dataSource.deviceId}` as SelectedItem)
        break
      case 'telemetry':
        items.add(`device-metric:${dataSource.deviceId}:${dataSource.metricId}` as SelectedItem)
        break
      case 'command':
        items.add(`device-command:${dataSource.deviceId}:${dataSource.command}` as SelectedItem)
        break
      case 'device-info':
        items.add(`device-info:${dataSource.deviceId}:${dataSource.infoProperty}` as SelectedItem)
        break
      case 'system':
        items.add(`system:${dataSource.systemMetric}` as SelectedItem)
        break
      case 'extension':
        // For extension type, check if it has extensionId and extensionMetric
        if ((dataSource as any).extensionId && (dataSource as any).extensionMetric) {
          // Strip "produce:" prefix if present to get just the metric name
          const metric = (dataSource as any).extensionMetric
          const metricName = metric.startsWith('produce:') ? metric.slice(8) : metric
          items.add(`extension:${(dataSource as any).extensionId}:${metricName}` as SelectedItem)
        }
        break
      case 'extension-command':
        // For extension-command type, check if it has extensionId and command
        if ((dataSource as any).extensionId && (dataSource as any).command) {
          items.add(`extension-command:${(dataSource as any).extensionId}:${(dataSource as any).command}` as SelectedItem)
        }
        break
    }
  }

  return items
}

/**
 * Get a readable label for a selected item
 */
function getSelectedItemLabel(item: SelectedItem, devices: any[], t: (key: string) => string): string {
  const parts = item.split(':')
  const type = parts[0]

  switch (type) {
    case 'device': {
      // Format: device:deviceId - just show device name
      const device = devices.find(d => d.id === parts[1])
      return device?.name || parts[1]
    }
    case 'device-metric': {
      // Format: device-metric:deviceId:metricId
      const device = devices.find(d => d.id === parts[1])
      const deviceName = device?.name || parts[1]
      return `${deviceName} · ${parts.slice(2).join(':')}`
    }
    case 'device-command': {
      // Format: device-command:deviceId:command
      const device = devices.find(d => d.id === parts[1])
      const deviceName = device?.name || parts[1]
      return `${deviceName} · ${parts.slice(2).join(':')}`
    }
    case 'device-info': {
      // Format: device-info:deviceId:property
      const device = devices.find(d => d.id === parts[1])
      const deviceName = device?.name || parts[1]
      const prop = getDeviceInfoProperties(t).find((p: { id: string; name: string }) => p.id === parts.slice(2).join(':'))
      return `${deviceName} · ${prop?.name || parts.slice(2).join(':')}`
    }
    case 'system': {
      // Format: system:metricId (not system:deviceId:metricId)
      const metricId = parts.slice(1).join(':')
      const systemMetric = getSystemMetrics(t).find(m => m.id === metricId)
      return `${t('systemDataSource.title')} · ${systemMetric?.name || metricId}`
    }
    case 'extension': {
      // Format: extension:extensionId:metricId - will be resolved in the component
      return `Extension · ${parts[2]}`
    }
    case 'extension-command': {
      // Format: extension-command:extensionId:commandName
      return `Extension · ${parts.slice(2).join(':')}`
    }
    default:
      return item
  }
}

// ============================================================================
// Main Component
// ============================================================================

export function UnifiedDataSourceConfig({
  value,
  onChange,
  allowedTypes,
  multiple = false,
  maxSources = 10,
  className,
}: UnifiedDataSourceConfigProps) {
  const { t } = useTranslation('dashboardComponents')
  const { devices, deviceTypes } = useStore()
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategory, setSelectedCategory] = useState<CategoryType>('device-metric')
  const [selectedDeviceId, setSelectedDeviceId] = useState<string | null>(null)
  const [selectedExtensionId, setSelectedExtensionId] = useState<string | null>(null)

  // Mobile: full-screen selector state
  const [showMobileSelector, setShowMobileSelector] = useState(false)

  // Extension state
  const [extensions, setExtensions] = useState<Extension[]>([])
  const [extensionDataSources, setExtensionDataSources] = useState<ExtensionDataSourceInfo[]>([])
  const [extensionsLoading, setExtensionsLoading] = useState(false)
  const [extensionError, setExtensionError] = useState<string | null>(null)

  // Data availability checking - now includes summaries with virtual metrics
  const { availability, summaries, loading: checkingData, checkDevice } = useDataAvailability()

  // Initialize selected items from current data source
  const [selectedItems, setSelectedItems] = useState<Set<SelectedItem>>(() =>
    dataSourceToSelectedItems(value)
  )

  // Track previous value to detect actual selection changes
  const prevCoreFieldsRef = useRef<string>()
  const prevValueRef = useRef<DataSourceOrList>()

  // Track if we've already fetched extensions to prevent infinite loops
  const hasFetchedExtensions = useRef(false)

  // Track if we've already restored extension selection from value
  const hasRestoredExtensionSelection = useRef(false)

  // Extract core identifying fields for comparison (ignores transform settings)
  const getCoreFields = (ds: DataSourceOrList | undefined): string => {
    if (!ds) return ''
    const sources = Array.isArray(ds) ? ds : [ds]
    return sources.map(s => {
      // Only include fields that identify the selection, not transform settings
      // Exclude: timeRange, limit, aggregate, aggregateExt, transform, params, timeWindow
      return `${s.type}:${s.deviceId || ''}:${s.metricId || s.property || s.infoProperty || ''}:${s.command || ''}`
    }).sort().join('|')
  }

  // Calculate current core fields
  const currentCoreFields = getCoreFields(value)

  useEffect(() => {
    // Initialize on first render
    if (prevValueRef.current === undefined) {
      prevValueRef.current = value
      prevCoreFieldsRef.current = currentCoreFields
      return
    }

    // Only update if core fields actually changed (selection changed)
    // Transform settings like timeRange, aggregate, etc. should NOT trigger reset
    if (prevCoreFieldsRef.current !== currentCoreFields) {
      prevValueRef.current = value
      prevCoreFieldsRef.current = currentCoreFields
      setSelectedItems(dataSourceToSelectedItems(value))
    }
    // If only transform settings changed, update the value ref but don't reset selection
    else if (prevCoreFieldsRef.current === currentCoreFields && value !== prevValueRef.current) {
      prevValueRef.current = value
    }
  }, [value, currentCoreFields])

  // Available categories based on allowedTypes
  const availableCategories = useMemo(
    () => getCategories(t).filter(c => (normalizeAllowedTypes(allowedTypes) as CategoryType[]).includes(c.id)),
    [allowedTypes, t]
  )

  // Set initial category to first available (useEffect instead of useMemo to avoid setState during render)
  useEffect(() => {
    if (availableCategories.length > 0 && !availableCategories.find(c => c.id === selectedCategory)) {
      setSelectedCategory(availableCategories[0]!.id)
      setSelectedDeviceId(null)
    }
  }, [availableCategories, selectedCategory])

  // Build device metrics map
  const deviceMetricsMap = useMemo(() => {
    const map = new Map<string, MetricDefinition[]>()
    for (const device of devices) {
      const deviceType = deviceTypes.find(dt => dt.device_type === device.device_type)

      if (deviceType?.metrics && deviceType.metrics.length > 0) {
        map.set(device.id, deviceType.metrics)
      } else {
        // Fallback: use current_values
        if (device.current_values && typeof device.current_values === 'object') {
          const dynamicMetrics: MetricDefinition[] = Object.keys(device.current_values).map(key => ({
            name: key,
            display_name: key,
            data_type: 'float' as const,
            unit: '',
          }))
          map.set(device.id, dynamicMetrics)
        } else {
          map.set(device.id, [{ name: 'value', display_name: t('chart.value'), data_type: 'float' as const, unit: '' }])
        }
      }
    }
    return map
  }, [devices, deviceTypes, t])

  // Fetch extension data sources when extension category is available
  useEffect(() => {
    const hasExtensionCategory = availableCategories.some(c => c.id === 'extension' || c.id === 'extension-command')
    if (!hasExtensionCategory) {
      hasFetchedExtensions.current = false
      return
    }

    // Skip if already fetched
    if (hasFetchedExtensions.current && extensions.length > 0) {
      return
    }

    const fetchExtensionData = async () => {
      setExtensionsLoading(true)
      setExtensionError(null)
      try {
        console.log('[UnifiedDataSourceConfig] Fetching extensions...')
        const [extData, dsData] = await Promise.all([
          api.listExtensions().catch((err) => {
            console.error('[UnifiedDataSourceConfig] listExtensions error:', err)
            return []
          }),
          api.listAllDataSources().catch((err) => {
            console.error('[UnifiedDataSourceConfig] listAllDataSources error:', err)
            return []
          }),
        ])
        // Debug logging
        console.log('[UnifiedDataSourceConfig] Fetched extensions:', {
          count: extData.length,
          extensions: extData.map(e => ({
            id: e.id,
            name: e.name,
            commandsCount: e.commands?.length || 0,
            metricsCount: e.metrics?.length || 0,
            commands: e.commands
          }))
        })
        setExtensions(extData)
        // Filter only extension data sources (exclude transform data sources)
        setExtensionDataSources(dsData.filter((source): source is ExtensionDataSourceInfo => 'extension_id' in source) as ExtensionDataSourceInfo[])
        hasFetchedExtensions.current = true
      } catch (err) {
        console.error('[UnifiedDataSourceConfig] Error fetching extension data:', err)
        setExtensionError((err as Error).message)
      } finally {
        setExtensionsLoading(false)
      }
    }

    fetchExtensionData()
  }, [availableCategories, extensions.length])

  // Restore extension selection from saved value (only runs once after extensions are loaded)
  useEffect(() => {
    // Skip if already restored or no extensions loaded yet
    if (hasRestoredExtensionSelection.current || extensions.length === 0) return

    if (!value) {
      hasRestoredExtensionSelection.current = true
      return
    }

    const dataSources = normalizeDataSource(value)

    // Find first extension or extension-command data source
    const extensionSource = dataSources.find(ds =>
      ds.type === 'extension' || ds.type === 'extension-command'
    ) as any

    if (extensionSource?.extensionId) {
      // Check if this extension exists in the loaded extensions
      const extensionExists = extensions.some(e => e.id === extensionSource.extensionId)

      if (extensionExists) {
        // Restore the selected extension
        setSelectedExtensionId(extensionSource.extensionId)

        // Also set the correct category based on the data source type
        if (extensionSource.type === 'extension-command') {
          setSelectedCategory('extension-command')
        } else if (extensionSource.type === 'extension') {
          setSelectedCategory('extension')
        }
      }
    }

    // Mark as restored regardless of whether we found an extension
    hasRestoredExtensionSelection.current = true
  }, [value, extensions])

  // Build extension metrics map
  const extensionMetricsMap = useMemo(() => {
    const map = new Map<string, Array<{ name: string; display_name: string; data_type: string; unit?: string }>>()
    extensionDataSources.forEach(ds => {
      if (!map.has(ds.extension_id)) {
        map.set(ds.extension_id, [])
      }
      map.get(ds.extension_id)!.push({
        name: ds.field,
        display_name: ds.display_name,
        data_type: ds.data_type,
        unit: ds.unit,
      })
    })
    return map
  }, [extensionDataSources])

  // Check data availability when device is selected (for metrics category)
  useEffect(() => {
    if (selectedCategory === 'device-metric' && selectedDeviceId) {
      const metrics = deviceMetricsMap.get(selectedDeviceId) || []
      const metricIds = metrics.map(m => m.name)
      if (metricIds.length > 0) {
        checkDevice(selectedDeviceId, metricIds)
      }
    }
  }, [selectedCategory, selectedDeviceId, deviceMetricsMap, checkDevice])

  // Build device commands map - only show commands that are actually defined in the device template
  const deviceCommandsMap = useMemo(() => {
    const map = new Map<string, CommandDefinition[]>()
    for (const device of devices) {
      const deviceType = deviceTypes.find(dt => dt.device_type === device.device_type)

      // Only include commands that are actually defined in the device type
      // No fallback commands - this prevents users from selecting commands that don't exist
      if (deviceType?.commands && deviceType.commands.length > 0) {
        map.set(device.id, deviceType.commands)
      } else {
        // No commands defined - return empty array
        map.set(device.id, [])
      }
    }
    return map
  }, [devices, deviceTypes])

  // Handle item selection
  const handleSelectItem = (itemKey: SelectedItem) => {
    setSelectedItems(prev => {
      const next = new Set(prev)

      if (multiple) {
        // Multiple selection mode - toggle
        if (next.has(itemKey)) {
          next.delete(itemKey)
        } else if (next.size < maxSources) {
          next.add(itemKey)
        }
      } else {
        // Single selection mode
        if (next.has(itemKey)) {
          next.clear()
        } else {
          next.clear()
          next.add(itemKey)
        }
      }

      // Auto-apply selection
      const dataSource = selectedItemsToDataSource(next, multiple)
      onChange(dataSource as any)

      return next
    })
  }

  // Remove a specific selected item
  const handleRemoveItem = (itemKey: SelectedItem) => {
    setSelectedItems(prev => {
      const next = new Set(prev)
      next.delete(itemKey)

      const dataSource = selectedItemsToDataSource(next, multiple)
      onChange(dataSource as any)

      return next
    })
  }

  // Clear all selections
  const handleClearSelection = () => {
    setSelectedItems(new Set())
    onChange(undefined)
  }

  // Get current category config
  const categoryConfig = getCategories(t).find(c => c.id === selectedCategory)

  // Filter devices by search query
  const filteredDevices = useMemo(() => {
    if (!searchQuery) return devices

    const query = searchQuery.toLowerCase()
    return devices.filter(d =>
      d.id.toLowerCase().includes(query) ||
      (d.name && d.name.toLowerCase().includes(query))
    )
  }, [devices, searchQuery])

  // Convert selected items to array for display
  const selectedItemsArray = useMemo(() => Array.from(selectedItems), [selectedItems])

  // Check if category uses split layout
  const usesDeviceSplitLayout = selectedCategory === 'device-metric' || selectedCategory === 'device-command'
  const usesExtensionSplitLayout = selectedCategory === 'extension' || selectedCategory === 'extension-command'

  // Get selected device/extension
  const selectedDevice = devices.find(d => d.id === selectedDeviceId)
  const selectedExtension = extensions.find(e => e.id === selectedExtensionId)

  // Filter extensions by search query
  const filteredExtensions = useMemo(() => {
    if (!searchQuery) return extensions
    return extensions.filter(e =>
      e.id.toLowerCase().includes(searchQuery) ||
      (e.name && e.name.toLowerCase().includes(searchQuery))
    )
  }, [extensions, searchQuery])

  // Render device list for split layout
  const renderDeviceList = () => {
    return (
      <div className="flex flex-col h-full">
        {/* Search input inside device list */}
        <div className="p-2 border-b bg-muted/20">
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              placeholder={t('dataSource.searchPlaceholder')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="h-8 pl-8 text-xs"
            />
          </div>
        </div>

        {/* Device list header with count */}
        <div className="px-3 py-1.5 border-b text-xs font-medium text-muted-foreground bg-muted/30 flex items-center justify-between">
          <span>{t('dataSource.deviceList')}</span>
          <span className="text-[10px] text-muted-foreground/60 bg-muted/50 px-1.5 py-0.5 rounded">
            {filteredDevices.length} {t('dataSource.count')}
          </span>
        </div>

        {/* Device list */}
        {filteredDevices.length === 0 ? (
          <div className="p-4 text-center text-muted-foreground text-xs">{t('dataSource.noDevices')}</div>
        ) : (
          <div className="flex-1 overflow-y-auto">
            {filteredDevices.map(device => {
              const isSelected = selectedDeviceId === device.id
              // Count selected items for this device
              const devicePrefix = `${selectedCategory}:${device.id}:`
              const selectedCount = Array.from(selectedItems).filter(item => item.startsWith(devicePrefix)).length

              // Get available items count (metrics or commands)
              let availableCount = 0
              if (selectedCategory === 'device-metric') {
                const metrics = deviceMetricsMap.get(device.id) || []
                availableCount = metrics.length
              } else if (selectedCategory === 'device-command') {
                const commands = deviceCommandsMap.get(device.id) || []
                availableCount = commands.length
              }

              return (
                <button
                  key={device.id}
                  type="button"
                  onClick={() => {
                    if (isMobile) {
                      setShowMobileSelector(true)
                    }
                    setSelectedDeviceId(device.id)
                  }}
                  className={cn(
                    'w-full flex items-center gap-2 px-3 py-2 text-left border-b transition-all duration-150',
                    isSelected
                      ? 'bg-primary/10 border-l-2 border-l-primary'
                      : 'bg-transparent border-l-2 border-l-transparent hover:bg-muted/40'
                  )}
                >
                  <div className="flex-1 min-w-0">
                    <div className={cn(
                      'text-sm truncate',
                      isSelected ? 'font-medium text-foreground' : 'font-normal text-foreground/80'
                    )}>{device.name || device.id}</div>
                    <div className="text-[10px] text-muted-foreground truncate flex items-center gap-1.5">
                      <span>{device.device_type}</span>
                      {availableCount > 0 && (
                        <>
                          <span className="text-muted-foreground/30">•</span>
                          <span className="text-muted-foreground/70">
                            {selectedCategory === 'device-metric' ? `${availableCount} ${t('dataSource.metricsCount')}` : `${availableCount} ${t('dataSource.commandsCount')}`}
                          </span>
                        </>
                      )}
                    </div>
                  </div>
                  {isMobile && (
                    <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
                  )}
                  {!isMobile && selectedCount > 0 && (
                    <span className="shrink-0 min-w-[18px] h-[18px] flex items-center justify-center text-[10px] font-medium bg-primary text-primary-foreground rounded-sm">
                      {selectedCount}
                    </span>
                  )}
                </button>
              )
            })}
          </div>
        )}
      </div>
    )
  }

  // Render metrics/commands for selected device
  const renderDeviceContent = () => {
    if (!selectedDevice) {
      return (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
          {t('dataSource.selectDevice')}
        </div>
      )
    }

    if (selectedCategory === 'device-metric') {
      const metrics = deviceMetricsMap.get(selectedDevice.id) || []
      const deviceSummary = summaries.get(selectedDevice.id) || {}

      // Track template metric names to identify virtual metrics
      const templateMetricNames = new Set(metrics.map(m => m.name))

      type Item = {
        key: string
        propertyName: string
        propertyDisplayName: string
        currentValue?: unknown
        isSelected: boolean
        hasData: boolean | null
        dataPointCount?: number
        itemType: 'template' | 'virtual' | 'info'
        unit?: string
        dataType?: string
      }

      const items: Item[] = []

      // Add template metrics
      for (const metric of metrics) {
        const itemKey = `device-metric:${selectedDevice.id}:${metric.name}` as SelectedItem
        const availabilityKey = `${selectedDevice.id}:${metric.name}`
        const metricAvailability = availability.get(availabilityKey)
        items.push({
          key: itemKey,
          propertyName: metric.name,
          propertyDisplayName: metric.display_name || metric.name,
          currentValue: selectedDevice.current_values?.[metric.name],
          isSelected: selectedItems.has(itemKey),
          hasData: metricAvailability?.hasData ?? null,
          dataPointCount: metricAvailability?.dataPointCount,
          itemType: 'template',
          unit: metric.unit,
          dataType: metric.data_type,
        })
      }

      // Add virtual metrics from summary (not in template)
      for (const [metricId, metricSummary] of Object.entries(deviceSummary)) {
        if (!templateMetricNames.has(metricId) && metricSummary.is_virtual) {
          const itemKey = `device-metric:${selectedDevice.id}:${metricId}` as SelectedItem
          const availabilityKey = `${selectedDevice.id}:${metricId}`
          const metricAvailability = availability.get(availabilityKey)
          items.push({
            key: itemKey,
            propertyName: metricId,
            propertyDisplayName: metricSummary.display_name || metricId,
            currentValue: metricSummary.current,
            isSelected: selectedItems.has(itemKey),
            hasData: metricAvailability?.hasData ?? null,
            dataPointCount: metricAvailability?.dataPointCount,
            itemType: 'virtual',
            unit: metricSummary.unit,
            dataType: metricSummary.data_type,
          })
        }
      }

      // Add device info properties
      for (const infoProp of getDeviceInfoProperties(t)) {
        const itemKey = `device-info:${selectedDevice.id}:${infoProp.id}` as SelectedItem
        let currentValue: unknown = undefined

        // Get current value from device
        switch (infoProp.id) {
          case 'name':
            currentValue = selectedDevice.name
            break
          case 'status':
            currentValue = selectedDevice.status
            break
          case 'online':
            currentValue = selectedDevice.online
            break
          case 'last_seen':
            currentValue = selectedDevice.last_seen
            break
          case 'device_type':
            currentValue = selectedDevice.device_type
            break
          case 'plugin_name':
            currentValue = selectedDevice.plugin_name
            break
          case 'adapter_id':
            currentValue = selectedDevice.adapter_id
            break
        }

        items.push({
          key: itemKey,
          propertyName: infoProp.id,
          propertyDisplayName: infoProp.name,
          currentValue,
          isSelected: selectedItems.has(itemKey),
          hasData: null, // Info props don't have historical data
          itemType: 'info',
        })
      }

      // Sort items: template metrics first, then device info, then virtual metrics
      items.sort((a, b) => {
        const order = { template: 0, info: 1, virtual: 2 }
        return order[a.itemType] - order[b.itemType]
      })

      if (items.length === 0) {
        return <div className="p-4 text-center text-muted-foreground text-sm">{t('dataSource.noAvailableMetrics')}</div>
      }

      // Helper to format current value
      const formatValue = (val: unknown): string => {
        if (val === null || val === undefined) return '-'
        if (typeof val === 'number') return val.toLocaleString('en-US', { maximumFractionDigits: 2 })
        if (typeof val === 'boolean') return val ? t('dataSource.yes') : t('dataSource.no')
        return String(val)
      }

      // Badge component for item type
      const ItemBadge = ({ itemType }: { itemType: 'template' | 'virtual' | 'info' }) => {
        const config = {
          template: { label: t('dataSource.badgeTemplate'), className: 'bg-blue-500/10 text-blue-600 border-blue-500/20' },
          virtual: { label: t('dataSource.badgeVirtual'), className: 'bg-purple-500/10 text-purple-600 border-purple-500/20' },
          info: { label: t('dataSource.badgeInfo'), className: 'bg-amber-500/10 text-amber-600 border-amber-500/20' },
        }[itemType]
        return (
          <span className={cn('px-1.5 py-0.5 text-[10px] font-medium rounded-[3px] border shrink-0', config.className)}>
            {config.label}
          </span>
        )
      }

      // Data indicator component
      const DataIndicator = ({ hasData, count }: { hasData: boolean | null; count?: number }) => {
        if (hasData === true) {
          return (
            <div className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-green-500/10 border border-green-500/20" title={`${t('dataSource.hasHistoricalData')} (${count ?? 0} ${t('dataSource.dataPoints')})`}>
              <Circle className="h-1.5 w-1.5 fill-green-500 text-green-500" />
              <span className="text-[10px] text-green-600 font-medium">{count ?? 0}</span>
            </div>
          )
        }
        if (hasData === false) {
          return (
            <div className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-muted/30 border border-muted/30" title={t('dataSource.noHistoricalData')}>
              <Circle className="h-1.5 w-1.5 fill-muted-foreground/40 text-muted-foreground/40" />
              <span className="text-[10px] text-muted-foreground">{t('dataSource.noData')}</span>
            </div>
          )
        }
        return null
      }

      return (
        <div className="flex flex-col h-full">
          <div className="px-3 py-2.5 border-b text-xs font-medium text-muted-foreground bg-muted/30 flex items-center justify-between">
            <span className="flex items-center gap-1.5">
              <Database className="h-3.5 w-3.5" />
              {t('dataSource.metricsOf', { device: selectedDevice.name || selectedDevice.id })}
            </span>
            <span className="text-[10px] text-muted-foreground/60 bg-muted/50 px-1.5 py-0.5 rounded">
              {items.length} {t('dataSource.count')}
            </span>
          </div>
          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {items.map(item => (
              <button
                key={item.key}
                type="button"
                onClick={() => handleSelectItem(item.key)}
                className={cn(
                  'w-full text-left transition-colors duration-150',
                  'group relative rounded-md border',
                  item.isSelected
                    ? 'bg-primary/10 border-primary/50'
                    : 'bg-card border-border hover:bg-accent/40 hover:border-primary/20'
                )}
              >
                <div className="flex items-center gap-2 p-2">
                  {/* Check icon - positioned on left */}
                  <div className={cn(
                    'shrink-0 w-4 h-4 rounded-sm flex items-center justify-center transition-colors',
                    item.isSelected
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-muted text-muted-foreground group-hover:bg-muted-foreground/10'
                  )}>
                    <Check className={cn(
                      'h-3 w-3',
                      item.isSelected ? 'opacity-100' : 'opacity-0'
                    )} />
                  </div>

                  {/* Main content */}
                  <div className="flex-1 min-w-0 space-y-0.5">
                    {/* Header row */}
                    <div className="flex items-center gap-1.5">
                      <ItemBadge itemType={item.itemType} />
                      <span className={cn(
                        'text-sm truncate',
                        item.isSelected ? 'font-medium text-foreground' : 'font-normal text-foreground/80'
                      )}>{item.propertyDisplayName}</span>
                      <div className="flex-1" />
                      {item.hasData !== null && (
                        <DataIndicator hasData={item.hasData} count={item.dataPointCount} />
                      )}
                      {item.hasData === null && checkingData && (
                        <Loader2 className="h-3 w-3 animate-spin text-muted-foreground/50" />
                      )}
                    </div>

                    {/* Subtitle row */}
                    <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground">
                      <code className="px-1 py-0.5 bg-muted/40 rounded text-[9px] font-mono">
                        {item.propertyName}
                      </code>
                      {item.currentValue !== undefined && item.currentValue !== null && (
                        <>
                          <span className="text-muted-foreground/20">·</span>
                          <span className="text-foreground/60">{t('dataSource.current')}: {formatValue(item.currentValue)}</span>
                          {item.unit && item.unit !== '-' && (
                            <>
                              <span className="text-muted-foreground/20">·</span>
                              <span className="text-muted-foreground/40">{item.unit}</span>
                            </>
                          )}
                        </>
                      )}
                    </div>
                  </div>
                </div>
              </button>
            ))}
          </div>
        </div>
      )
    }

    if (selectedCategory === 'device-command') {
      const commands = deviceCommandsMap.get(selectedDevice.id) || []
      const items: Array<{
        key: string
        commandName: string
        commandDisplayName: string
        isSelected: boolean
      }> = []

      for (const cmd of commands) {
        const itemKey = `device-command:${selectedDevice.id}:${cmd.name}` as SelectedItem
        items.push({
          key: itemKey,
          commandName: cmd.name,
          commandDisplayName: cmd.display_name || cmd.name,
          isSelected: selectedItems.has(itemKey),
        })
      }

      if (items.length === 0) {
        return <div className="p-4 text-center text-muted-foreground text-sm">{t('dataSource.noAvailableCommands')}</div>
      }

      return (
        <div className="flex flex-col h-full">
          <div className="px-3 py-2 border-b text-xs font-medium text-muted-foreground bg-muted/30">
            {t('dataSource.commandsOf', { device: selectedDevice.name || selectedDevice.id })}
          </div>
          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {items.map(item => (
              <button
                key={item.key}
                type="button"
                onClick={() => handleSelectItem(item.key)}
                className={cn(
                  'w-full flex items-center justify-between p-3 rounded-lg border text-left transition-colors',
                  'hover:bg-accent/50',
                  item.isSelected
                    ? 'border-primary bg-primary/10'
                    : 'border-muted'
                )}
              >
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-sm truncate">{item.commandDisplayName}</div>
                </div>
                {item.isSelected && (
                  <Check className="h-4 w-4 text-primary shrink-0 ml-2" />
                )}
              </button>
            ))}
          </div>
        </div>
      )
    }

    return null
  }

  // Render extension list for split layout (left sidebar)
  const renderExtensionList = () => {
    return (
      <div className="flex flex-col h-full">
        {/* Search input inside extension list */}
        <div className="p-2 border-b bg-muted/20">
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              placeholder={t('dataSource.searchPlaceholder')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="h-8 pl-8 text-xs"
            />
          </div>
        </div>

        {/* Extension list header with count */}
        <div className="px-3 py-1.5 border-b text-xs font-medium text-muted-foreground bg-muted/30 flex items-center justify-between">
          <span>{t('extensions:extensionList') || 'Extensions'}</span>
          <span className="text-[10px] text-muted-foreground/60 bg-muted/50 px-1.5 py-0.5 rounded">
            {filteredExtensions.length} {t('dataSource.count')}
          </span>
        </div>

        {/* Extension list */}
        <div className="flex-1 overflow-y-auto">
          {extensionsLoading ? (
            <div className="p-4 text-center text-muted-foreground text-sm flex items-center justify-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t('extensions:loading') || 'Loading...'}
            </div>
          ) : filteredExtensions.length === 0 ? (
            <div className="p-4 text-center text-muted-foreground text-sm">
              {t('extensions:noExtensions') || 'No extensions'}
            </div>
          ) : (
            filteredExtensions.map(ext => {
              const isSelected = selectedExtensionId === ext.id
              return (
                <button
                  key={ext.id}
                  type="button"
                  onClick={() => setSelectedExtensionId(ext.id)}
                  className={cn(
                    'w-full flex items-center gap-3 px-3 py-2.5 border-b text-left transition-colors',
                    isSelected
                      ? 'bg-primary/10 border-primary/30'
                      : 'hover:bg-muted/30 border-transparent'
                  )}
                >
                  <Puzzle className={cn(
                    'h-4 w-4 shrink-0',
                    isSelected ? 'text-primary' : 'text-muted-foreground'
                  )} />
                  <div className="flex-1 min-w-0">
                    <div className={cn(
                      'text-sm truncate',
                      isSelected ? 'font-medium text-foreground' : 'text-foreground/80'
                    )}>
                      {ext.name}
                    </div>
                    <div className="text-[10px] text-muted-foreground truncate">
                      {ext.version && `v${ext.version}`}
                      {ext.description && ` · ${ext.description}`}
                    </div>
                  </div>
                  {isSelected && (
                    <Check className="h-4 w-4 text-primary shrink-0" />
                  )}
                </button>
              )
            })
          )}
        </div>
      </div>
    )
  }

  // Render extension content for split layout (right side)
  const renderExtensionContent = () => {
    if (!selectedExtension) {
      return (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
          {t('extensions:selectExtension') || 'Select an extension'}
        </div>
      )
    }

    if (selectedCategory === 'extension') {
      // Show extension metrics
      const metrics = extensionMetricsMap.get(selectedExtension.id) || []

      if (metrics.length === 0) {
        return (
          <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
            {t('extensions:noMetrics') || 'No metrics available'}
          </div>
        )
      }

      return (
        <div className="flex flex-col h-full">
          <div className="px-3 py-2 border-b text-xs font-medium text-muted-foreground bg-muted/30">
            {selectedExtension.name} · {t('dataSource.metrics') || 'Metrics'}
          </div>
          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {metrics.map(metric => {
              const itemKey = `extension:${selectedExtension.id}:${metric.name}` as SelectedItem
              const isSelected = selectedItems.has(itemKey)

              return (
                <button
                  key={metric.name}
                  type="button"
                  onClick={() => handleSelectItem(itemKey)}
                  className={cn(
                    'w-full flex items-center justify-between p-3 rounded-lg border text-left transition-colors',
                    'hover:bg-accent/50',
                    isSelected
                      ? 'border-primary bg-primary/10'
                      : 'border-muted'
                  )}
                >
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm truncate">{metric.display_name || metric.name}</div>
                    <div className="text-xs text-muted-foreground truncate">{metric.name}</div>
                  </div>
                  {isSelected && (
                    <Check className="h-4 w-4 text-primary shrink-0 ml-2" />
                  )}
                </button>
              )
            })}
          </div>
        </div>
      )
    }

    if (selectedCategory === 'extension-command') {
      // Show extension commands
      const commands = selectedExtension.commands || []
      // Debug logging
      console.log('[UnifiedDataSourceConfig] Extension commands:', {
        extensionId: selectedExtension.id,
        extensionName: selectedExtension.name,
        commandsCount: commands.length,
        commands: commands,
        fullExtension: selectedExtension
      })

      if (commands.length === 0) {
        return (
          <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
            {t('extensions:noCommands') || 'No commands available'}
          </div>
        )
      }

      return (
        <div className="flex flex-col h-full">
          <div className="px-3 py-2 border-b text-xs font-medium text-muted-foreground bg-muted/30">
            {selectedExtension.name} · {t('dataSource.commands') || 'Commands'}
          </div>
          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {commands.map((cmd: ExtensionCommandDescriptor) => {
              const itemKey = `extension-command:${selectedExtension.id}:${cmd.id}` as SelectedItem
              const isSelected = selectedItems.has(itemKey)

              return (
                <button
                  key={cmd.id}
                  type="button"
                  onClick={() => handleSelectItem(itemKey)}
                  className={cn(
                    'w-full flex items-center justify-between p-3 rounded-lg border text-left transition-colors',
                    'hover:bg-accent/50',
                    isSelected
                      ? 'border-primary bg-primary/10'
                      : 'border-muted'
                  )}
                >
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm truncate">{cmd.display_name || cmd.id}</div>
                    <div className="text-xs text-muted-foreground truncate">{cmd.description || cmd.id}</div>
                  </div>
                  <Zap className={cn(
                    'h-4 w-4 shrink-0 ml-2',
                    isSelected ? 'text-amber-500' : 'text-muted-foreground'
                  )} />
                </button>
              )
            })}
          </div>
        </div>
      )
    }

    return null
  }

  // Render content based on category
  const renderCategoryContent = () => {
    switch (selectedCategory) {
      case 'device-metric':
      case 'device-command':
      case 'extension':
      case 'extension-command':
        // Split layout handled separately
        return null

      case 'device':
        // Device category - show device list directly for selection (for map markers)
        return (
          <div className="space-y-1">
            {filteredDevices.length === 0 ? (
              <div className="p-4 text-center text-muted-foreground text-sm">{t('dataSource.noDevices')}</div>
            ) : (
              filteredDevices.map(device => {
                const itemKey = `device:${device.id}` as SelectedItem
                const isSelected = selectedItems.has(itemKey)

                return (
                  <button
                    key={device.id}
                    type="button"
                    onClick={() => handleSelectItem(itemKey)}
                    className={cn(
                      'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg border text-left transition-all duration-150',
                      isSelected
                        ? 'bg-primary/10 border-primary/50'
                        : 'bg-card border-border hover:bg-accent/40 hover:border-primary/20'
                    )}
                  >
                    {/* Check icon */}
                    <div className={cn(
                      'shrink-0 w-5 h-5 rounded-md flex items-center justify-center transition-colors',
                      isSelected
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted text-muted-foreground'
                    )}>
                      <Check className={cn(
                        'h-3.5 w-3.5',
                        isSelected ? 'opacity-100' : 'opacity-0'
                      )} />
                    </div>

                    {/* Device info */}
                    <div className="flex-1 min-w-0">
                      <div className={cn(
                        'text-sm truncate',
                        isSelected ? 'font-medium text-foreground' : 'font-normal text-foreground/80'
                      )}>
                        {device.name || device.id}
                      </div>
                      <div className="text-[10px] text-muted-foreground truncate">
                        {device.device_type}
                      </div>
                    </div>

                    {/* Status indicator */}
                    <div className={cn(
                      'w-2 h-2 rounded-full',
                      device.online ? 'bg-green-500' : 'bg-muted-foreground/30'
                    )} />
                  </button>
                )
              })
            )}
          </div>
        )

      case 'system':
        // System metrics - show system metrics list
        return (
          <div className="space-y-1">
            {getSystemMetrics(t).map(metric => {
              const itemKey = `system:${metric.id}` as SelectedItem
              const isSelected = selectedItems.has(itemKey)

              return (
                <button
                  key={metric.id}
                  type="button"
                  onClick={() => handleSelectItem(itemKey)}
                  className={cn(
                    'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg border text-left transition-all duration-150',
                    isSelected
                      ? 'bg-primary/10 border-primary/50'
                      : 'bg-card border-border hover:bg-accent/40 hover:border-primary/20'
                  )}
                >
                  {/* Check icon */}
                  <div className={cn(
                    'shrink-0 w-5 h-5 rounded-md flex items-center justify-center transition-colors',
                    isSelected
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-muted text-muted-foreground'
                  )}>
                    <Check className={cn(
                      'h-3.5 w-3.5',
                      isSelected ? 'opacity-100' : 'opacity-0'
                    )} />
                  </div>

                  {/* Metric info */}
                  <div className="flex-1 min-w-0">
                    <div className={cn(
                      'text-sm truncate',
                      isSelected ? 'font-medium text-foreground' : 'font-normal text-foreground/80'
                    )}>
                      {metric.name}
                    </div>
                    <div className="text-[10px] text-muted-foreground truncate">
                      {metric.description}
                    </div>
                  </div>

                  {/* Data type indicator */}
                  <div className="shrink-0 text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
                    {metric.unit || '—'}
                  </div>
                </button>
              )
            })}
          </div>
        )

      default:
        return null
    }
  }

  return (
    <>
      <div className={cn('flex flex-col h-full', className)}>
        {/* Selected items bar - compact single row */}
      {selectedItems.size > 0 && (
        <div className="px-3 py-2 border-b bg-gradient-to-r from-primary/5 via-primary/5 to-muted/20 flex flex-wrap gap-2 items-center">
          <div className="flex items-center gap-1.5 text-xs font-medium text-primary">
            <Check className="h-3.5 w-3.5" />
            {t('dataSource.selectedItems', { count: selectedItems.size })}
          </div>
          <div className="h-4 w-px bg-border" />
          <div className="flex flex-wrap gap-1.5 flex-1 min-w-0">
            {selectedItemsArray.slice(0, 3).map(itemKey => {
              const [type, entityId, ...rest] = itemKey.split(':')
              const label = rest.join(':')

              // For extension types, find the extension by id
              let entityName = entityId
              if (type.startsWith('extension')) {
                const ext = extensions.find(e => e.id === entityId)
                entityName = ext?.name || entityId
              } else {
                const device = devices.find(d => d.id === entityId)
                entityName = device?.name || entityId
              }

              // Icon and color based on type
              let TypeIcon = Info
              let iconColor = 'text-emerald-500'
              let displayLabel = label
              let showSeparator = true

              if (type === 'device-metric') {
                TypeIcon = Server
                iconColor = 'text-blue-500'
              } else if (type === 'device-command') {
                TypeIcon = Zap
                iconColor = 'text-amber-500'
              } else if (type === 'device') {
                TypeIcon = MapPin
                iconColor = 'text-purple-500'
                displayLabel = ''  // No label for device type, just device name
                showSeparator = false
              } else if (type === 'extension') {
                TypeIcon = Puzzle
                iconColor = 'text-cyan-500'
              } else if (type === 'extension-command') {
                TypeIcon = Zap
                iconColor = 'text-orange-500'
              }

              return (
                <div
                  key={itemKey}
                  className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-background border border-border/50 text-xs group hover:border-primary/40 transition-all max-w-[140px]"
                >
                  <TypeIcon className={cn('h-3 w-3 shrink-0', iconColor)} />
                  <span className="max-w-[80px] truncate text-foreground/70" title={entityName}>{entityName}</span>
                  {showSeparator && <span className="text-muted-foreground/40">·</span>}
                  {displayLabel && <span className="truncate text-foreground" title={displayLabel}>{displayLabel}</span>}
                </div>
              )
            })}
            {selectedItemsArray.length > 3 && (
              <div className="inline-flex items-center px-2 py-0.5 rounded-md bg-muted/50 text-xs text-muted-foreground">
                +{selectedItemsArray.length - 3} {t('dataSource.more')}
              </div>
            )}
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClearSelection}
            className="h-7 px-2 text-xs hover:bg-destructive/10 hover:text-destructive shrink-0"
            title={t('dataSource.clearAllSelections')}
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      )}

      {/* Category tabs */}
      <div className="flex border-b overflow-x-auto shrink-0">
        {availableCategories.map(cat => {
          const Icon = cat.icon
          const isActive = selectedCategory === cat.id
          return (
            <button
              key={cat.id}
              type="button"
              onClick={() => {
                setSelectedCategory(cat.id)
                setSelectedDeviceId(null)
                setSelectedExtensionId(null)
              }}
              className={cn(
                'flex items-center gap-1.5 px-4 py-2.5 text-sm font-medium border-b-2 transition-colors shrink-0',
                isActive
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/50'
              )}
            >
              <Icon className="h-4 w-4" />
              {cat.name}
            </button>
          )
        })}
      </div>

      {/* Content area */}
      {usesDeviceSplitLayout ? (
        // Split layout: device list on left, metrics/commands on right
        // Mobile: Show device list only, content opens in full screen
        isMobile ? (
          <div className="flex-1 flex overflow-hidden">
            <div className="flex-1 overflow-hidden flex flex-col">
              {renderDeviceList()}
            </div>
          </div>
        ) : (
          <div className="flex-1 flex overflow-hidden">
            {/* Left: Device list */}
            <div className="w-56 border-r shrink-0 overflow-hidden flex flex-col">
              {renderDeviceList()}
            </div>

            {/* Right: Device content */}
            <div className="flex-1 overflow-hidden flex flex-col">
              {renderDeviceContent()}
            </div>
          </div>
        )
      ) : usesExtensionSplitLayout ? (
        // Split layout: extension list on left, metrics/commands on right
        // Mobile: Show extension list only, content opens in full screen
        isMobile ? (
          <div className="flex-1 flex overflow-hidden">
            <div className="flex-1 overflow-hidden flex flex-col">
              {renderExtensionList()}
            </div>
          </div>
        ) : (
          <div className="flex-1 flex overflow-hidden">
            {/* Left: Extension list */}
            <div className="w-56 border-r shrink-0 overflow-hidden flex flex-col">
              {renderExtensionList()}
            </div>

            {/* Right: Extension content */}
            <div className="flex-1 overflow-hidden flex flex-col">
              {renderExtensionContent()}
            </div>
          </div>
        )
      ) : (
        // Single column layout
        <div className="flex-1 overflow-y-auto p-3">
          {renderCategoryContent()}
        </div>
      )}
    </div>

    {/* Mobile: Full-screen selector for device/extension content */}
    {isMobile && showMobileSelector && (
      <MobileItemSelector
        isOpen={showMobileSelector}
        onClose={() => setShowMobileSelector(false)}
        selectedDevice={selectedDevice}
        selectedExtension={selectedExtension}
        selectedCategory={selectedCategory}
        selectedItems={selectedItems}
        onSelectItem={handleSelectItem}
        deviceMetricsMap={deviceMetricsMap}
        deviceCommandsMap={deviceCommandsMap}
        extensionMetricsMap={extensionMetricsMap}
        devices={devices}
        extensions={extensions}
        summaries={summaries}
        availability={availability}
        checkingData={checkingData}
        getDeviceInfoProperties={getDeviceInfoProperties}
        t={t}
        insets={insets}
      />
    )}
    </>
  )
}

// ============================================================================
// Mobile Item Selector Component
// ============================================================================

interface MobileItemSelectorProps {
  isOpen: boolean
  onClose: () => void
  selectedDevice: any
  selectedExtension: any
  selectedCategory: CategoryType
  selectedItems: Set<SelectedItem>
  onSelectItem: (item: SelectedItem) => void
  deviceMetricsMap: Map<string, MetricDefinition[]>
  deviceCommandsMap: Map<string, CommandDefinition[]>
  extensionMetricsMap: Map<string, Array<{ name: string; display_name: string; data_type: string; unit?: string }>>
  devices: any[]
  extensions: Extension[]
  summaries: Map<string, any>
  availability: Map<string, { hasData: boolean; dataPointCount?: number }>
  checkingData: boolean
  getDeviceInfoProperties: (t: (key: string) => string) => Array<{ id: string; name: string }>
  t: (key: string) => string
  insets: { top: number; bottom: number; left: number; right: number }
}

function MobileItemSelector({
  isOpen,
  onClose,
  selectedDevice,
  selectedExtension,
  selectedCategory,
  selectedItems,
  onSelectItem,
  deviceMetricsMap,
  deviceCommandsMap,
  extensionMetricsMap,
  devices,
  extensions,
  summaries,
  availability,
  checkingData,
  getDeviceInfoProperties,
  t,
  insets,
}: MobileItemSelectorProps) {
  // Lock body scroll when mobile selector is open
  useMobileBodyScrollLock(isOpen)

  if (!isOpen) return null

  const title = selectedCategory === 'device-metric' || selectedCategory === 'device-command'
    ? (selectedDevice?.name || selectedDevice?.id || t('dataSource.selectDevice'))
    : (selectedExtension?.name || t('extensions:selectExtension') || 'Select Extension')

  return createPortal(
    <div className="fixed inset-0 z-[200] bg-background animate-in slide-in-from-right-0 duration-200">
      <div className="flex h-full w-full flex-col">
        {/* Header */}
        <div
          className="flex items-center gap-3 px-4 py-4 border-b shrink-0 bg-background"
          style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
        >
          <Button variant="ghost" size="icon" onClick={onClose} className="shrink-0">
            <ChevronLeft className="h-5 w-5" />
          </Button>
          <div className="min-w-0 flex-1">
            <h1 className="text-base font-semibold truncate">{title}</h1>
            <p className="text-xs text-muted-foreground truncate">
              {selectedCategory === 'device-metric' && t('dataSource.selectMetrics')}
              {selectedCategory === 'device-command' && t('dataSource.selectCommands')}
              {selectedCategory === 'extension' && (t('dataSource.metrics') || 'Metrics')}
              {selectedCategory === 'extension-command' && (t('dataSource.commands') || 'Commands')}
            </p>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto">
          {selectedCategory === 'device-metric' && selectedDevice && (
            <MobileMetricsList
              device={selectedDevice}
              deviceMetricsMap={deviceMetricsMap}
              summaries={summaries}
              availability={availability}
              checkingData={checkingData}
              getDeviceInfoProperties={getDeviceInfoProperties}
              selectedItems={selectedItems}
              onSelectItem={onSelectItem}
              t={t}
            />
          )}

          {selectedCategory === 'device-command' && selectedDevice && (
            <MobileCommandsList
              device={selectedDevice}
              deviceCommandsMap={deviceCommandsMap}
              selectedItems={selectedItems}
              onSelectItem={onSelectItem}
              t={t}
            />
          )}

          {selectedCategory === 'extension' && selectedExtension && (
            <MobileExtensionMetricsList
              extension={selectedExtension}
              extensionMetricsMap={extensionMetricsMap}
              selectedItems={selectedItems}
              onSelectItem={onSelectItem}
              t={t}
            />
          )}

          {selectedCategory === 'extension-command' && selectedExtension && (
            <MobileExtensionCommandsList
              extension={selectedExtension}
              selectedItems={selectedItems}
              onSelectItem={onSelectItem}
              t={t}
            />
          )}

          {selectedCategory === 'device-metric' && !selectedDevice && (
            <div className="flex items-center justify-center h-full text-muted-foreground text-sm p-4 text-center">
              {t('dataSource.selectDevice')}
            </div>
          )}

          {selectedCategory === 'device-command' && !selectedDevice && (
            <div className="flex items-center justify-center h-full text-muted-foreground text-sm p-4 text-center">
              {t('dataSource.selectDevice')}
            </div>
          )}

          {selectedCategory === 'extension' && !selectedExtension && (
            <div className="flex items-center justify-center h-full text-muted-foreground text-sm p-4 text-center">
              {t('extensions:selectExtension') || 'Select an extension'}
            </div>
          )}

          {selectedCategory === 'extension-command' && !selectedExtension && (
            <div className="flex items-center justify-center h-full text-muted-foreground text-sm p-4 text-center">
              {t('extensions:selectExtension') || 'Select an extension'}
            </div>
          )}
        </div>
      </div>
    </div>,
    document.body
  )
}

// Mobile metrics list
interface MobileMetricsListProps {
  device: any
  deviceMetricsMap: Map<string, MetricDefinition[]>
  summaries: Map<string, any>
  availability: Map<string, { hasData: boolean; dataPointCount?: number }>
  checkingData: boolean
  getDeviceInfoProperties: (t: (key: string) => string) => Array<{ id: string; name: string }>
  selectedItems: Set<SelectedItem>
  onSelectItem: (item: SelectedItem) => void
  t: (key: string) => string
}

function MobileMetricsList({
  device,
  deviceMetricsMap,
  summaries,
  availability,
  checkingData,
  getDeviceInfoProperties,
  selectedItems,
  onSelectItem,
  t,
}: MobileMetricsListProps) {
  const metrics = deviceMetricsMap.get(device.id) || []
  const deviceSummary = summaries.get(device.id) || {}
  const templateMetricNames = new Set(metrics.map((m: MetricDefinition) => m.name))

  type Item = {
    key: string
    propertyName: string
    propertyDisplayName: string
    currentValue?: unknown
    isSelected: boolean
    hasData: boolean | null
    dataPointCount?: number
    itemType: 'template' | 'virtual' | 'info'
    unit?: string
  }

  const items: Item[] = []

  // Template metrics
  for (const metric of metrics) {
    const itemKey = `device-metric:${device.id}:${metric.name}` as SelectedItem
    const availabilityKey = `${device.id}:${metric.name}`
    const metricAvailability = availability.get(availabilityKey)
    items.push({
      key: itemKey,
      propertyName: metric.name,
      propertyDisplayName: metric.display_name || metric.name,
      currentValue: device.current_values?.[metric.name],
      isSelected: selectedItems.has(itemKey),
      hasData: metricAvailability?.hasData ?? null,
      dataPointCount: metricAvailability?.dataPointCount,
      itemType: 'template',
      unit: metric.unit,
    })
  }

  // Virtual metrics
  for (const [metricId, metricSummary] of Object.entries(deviceSummary)) {
    const summary = metricSummary as { is_virtual?: boolean; display_name?: string; current?: unknown; unit?: string }
    if (!templateMetricNames.has(metricId) && summary.is_virtual) {
      const itemKey = `device-metric:${device.id}:${metricId}` as SelectedItem
      const availabilityKey = `${device.id}:${metricId}`
      const metricAvailability = availability.get(availabilityKey)
      items.push({
        key: itemKey,
        propertyName: metricId,
        propertyDisplayName: summary.display_name || metricId,
        currentValue: summary.current,
        isSelected: selectedItems.has(itemKey),
        hasData: metricAvailability?.hasData ?? null,
        dataPointCount: metricAvailability?.dataPointCount,
        itemType: 'virtual',
        unit: summary.unit,
      })
    }
  }

  // Device info properties
  for (const infoProp of getDeviceInfoProperties(t)) {
    const itemKey = `device-info:${device.id}:${infoProp.id}` as SelectedItem
    let currentValue: unknown = undefined

    switch (infoProp.id) {
      case 'name': currentValue = device.name; break
      case 'status': currentValue = device.status; break
      case 'online': currentValue = device.online; break
      case 'last_seen': currentValue = device.last_seen; break
      case 'device_type': currentValue = device.device_type; break
      case 'plugin_name': currentValue = device.plugin_name; break
      case 'adapter_id': currentValue = device.adapter_id; break
    }

    items.push({
      key: itemKey,
      propertyName: infoProp.id,
      propertyDisplayName: infoProp.name,
      currentValue,
      isSelected: selectedItems.has(itemKey),
      hasData: null,
      itemType: 'info',
    })
  }

  // Sort: template -> info -> virtual
  items.sort((a, b) => {
    const order = { template: 0, info: 1, virtual: 2 }
    return order[a.itemType] - order[b.itemType]
  })

  const formatValue = (val: unknown): string => {
    if (val === null || val === undefined) return '-'
    if (typeof val === 'number') return val.toLocaleString('en-US', { maximumFractionDigits: 2 })
    if (typeof val === 'boolean') return val ? t('dataSource.yes') : t('dataSource.no')
    return String(val)
  }

  const ItemBadge = ({ itemType }: { itemType: 'template' | 'virtual' | 'info' }) => {
    const config = {
      template: { label: t('dataSource.badgeTemplate'), className: 'bg-blue-500/10 text-blue-600 border-blue-500/20' },
      virtual: { label: t('dataSource.badgeVirtual'), className: 'bg-purple-500/10 text-purple-600 border-purple-500/20' },
      info: { label: t('dataSource.badgeInfo'), className: 'bg-amber-500/10 text-amber-600 border-amber-500/20' },
    }[itemType]
    return (
      <span className={`px-2 py-0.5 text-xs font-medium rounded-md border shrink-0 ${config.className}`}>
        {config.label}
      </span>
    )
  }

  return (
    <div className="p-4 space-y-3">
      {items.map(item => (
        <button
          key={item.key}
          type="button"
          onClick={() => onSelectItem(item.key)}
          className={cn(
            'w-full text-left transition-colors duration-150',
            'group relative rounded-2xl border p-4',
            item.isSelected
              ? 'bg-primary/10 border-primary/50'
              : 'bg-card border-border active:bg-accent/40'
          )}
        >
          <div className="flex items-start gap-3">
            {/* Check icon */}
            <div className={cn(
              'shrink-0 w-6 h-6 rounded-full flex items-center justify-center transition-colors mt-0.5',
              item.isSelected
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted text-muted-foreground'
            )}>
              <Check className={cn(
                'h-4 w-4',
                item.isSelected ? 'opacity-100' : 'opacity-0'
              )} />
            </div>

            {/* Content */}
            <div className="flex-1 min-w-0 space-y-2">
              {/* Header */}
              <div className="flex items-center gap-2 flex-wrap">
                <ItemBadge itemType={item.itemType} />
                <span className={cn(
                  'text-base font-medium',
                  item.isSelected ? 'text-foreground' : 'text-foreground/90'
                )}>
                  {item.propertyDisplayName}
                </span>
              </div>

              {/* Subtitle */}
              <div className="space-y-1">
                <code className="text-xs text-muted-foreground px-2 py-1 bg-muted/40 rounded-md block">
                  {item.propertyName}
                </code>
                {item.currentValue !== undefined && item.currentValue !== null && (
                  <div className="text-sm text-muted-foreground">
                    {t('dataSource.current')}: <span className="text-foreground font-medium">{formatValue(item.currentValue)}</span>
                    {item.unit && item.unit !== '-' && <span className="ml-1 text-muted-foreground/60">{item.unit}</span>}
                  </div>
                )}
              </div>
            </div>

            {/* Data indicator */}
            {item.hasData !== null && (
              <div className="shrink-0">
                {item.hasData ? (
                  <div className="px-2 py-1 rounded-lg bg-green-500/10 border border-green-500/20 text-xs text-green-600 font-medium" title={`${t('dataSource.hasHistoricalData')} (${item.dataPointCount ?? 0} ${t('dataSource.dataPoints')})`}>
                    {item.dataPointCount ?? 0}
                  </div>
                ) : (
                  <div className="px-2 py-1 rounded-lg bg-muted/30 border border-muted/30 text-xs text-muted-foreground">
                    {t('dataSource.noData')}
                  </div>
                )}
              </div>
            )}
          </div>
        </button>
      ))}
    </div>
  )
}

// Mobile commands list
interface MobileCommandsListProps {
  device: any
  deviceCommandsMap: Map<string, CommandDefinition[]>
  selectedItems: Set<SelectedItem>
  onSelectItem: (item: SelectedItem) => void
  t: (key: string) => string
}

function MobileCommandsList({
  device,
  deviceCommandsMap,
  selectedItems,
  onSelectItem,
  t,
}: MobileCommandsListProps) {
  const commands = deviceCommandsMap.get(device.id) || []

  if (commands.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm p-4">
        {t('dataSource.noAvailableCommands')}
      </div>
    )
  }

  return (
    <div className="p-4 space-y-3">
      {commands.map(cmd => {
        const itemKey = `device-command:${device.id}:${cmd.name}` as SelectedItem
        const isSelected = selectedItems.has(itemKey)

        return (
          <button
            key={cmd.name}
            type="button"
            onClick={() => onSelectItem(itemKey)}
            className={cn(
              'w-full text-left transition-colors duration-150',
              'group relative rounded-2xl border p-4',
              isSelected
                ? 'bg-primary/10 border-primary/50'
                : 'bg-card border-border active:bg-accent/40'
            )}
          >
            <div className="flex items-center gap-3">
              <div className={cn(
                'shrink-0 w-6 h-6 rounded-full flex items-center justify-center transition-colors',
                isSelected
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-muted-foreground'
              )}>
                <Check className={cn(
                  'h-4 w-4',
                  isSelected ? 'opacity-100' : 'opacity-0'
                )} />
              </div>
              <div className="flex-1 min-w-0">
                <div className={cn(
                  'text-base font-medium truncate',
                  isSelected ? 'text-foreground' : 'text-foreground/90'
                )}>
                  {cmd.display_name || cmd.name}
                </div>
                <div className="text-sm text-muted-foreground truncate">
                  {cmd.name}
                </div>
              </div>
              <Zap className={cn(
                'h-5 w-5 shrink-0',
                isSelected ? 'text-amber-500' : 'text-muted-foreground/40'
              )} />
            </div>
          </button>
        )
      })}
    </div>
  )
}

// Mobile extension metrics list
interface MobileExtensionMetricsListProps {
  extension: Extension
  extensionMetricsMap: Map<string, Array<{ name: string; display_name: string; data_type: string; unit?: string }>>
  selectedItems: Set<SelectedItem>
  onSelectItem: (item: SelectedItem) => void
  t: (key: string) => string
}

function MobileExtensionMetricsList({
  extension,
  extensionMetricsMap,
  selectedItems,
  onSelectItem,
  t,
}: MobileExtensionMetricsListProps) {
  const metrics = extensionMetricsMap.get(extension.id) || []

  if (metrics.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm p-4">
        {t('extensions:noMetrics') || 'No metrics available'}
      </div>
    )
  }

  return (
    <div className="p-4 space-y-3">
      {metrics.map(metric => {
        const itemKey = `extension:${extension.id}:${metric.name}` as SelectedItem
        const isSelected = selectedItems.has(itemKey)

        return (
          <button
            key={metric.name}
            type="button"
            onClick={() => onSelectItem(itemKey)}
            className={cn(
              'w-full text-left transition-colors duration-150',
              'group relative rounded-2xl border p-4',
              isSelected
                ? 'bg-primary/10 border-primary/50'
                : 'bg-card border-border active:bg-accent/40'
            )}
          >
            <div className="flex items-center gap-3">
              <div className={cn(
                'shrink-0 w-6 h-6 rounded-full flex items-center justify-center transition-colors',
                isSelected
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-muted-foreground'
              )}>
                <Check className={cn(
                  'h-4 w-4',
                  isSelected ? 'opacity-100' : 'opacity-0'
                )} />
              </div>
              <div className="flex-1 min-w-0">
                <div className={cn(
                  'text-base font-medium truncate',
                  isSelected ? 'text-foreground' : 'text-foreground/90'
                )}>
                  {metric.display_name || metric.name}
                </div>
                <div className="text-sm text-muted-foreground truncate">
                  {metric.name}
                  {metric.unit && ` (${metric.unit})`}
                </div>
              </div>
            </div>
          </button>
        )
      })}
    </div>
  )
}

// Mobile extension commands list
interface MobileExtensionCommandsListProps {
  extension: Extension
  selectedItems: Set<SelectedItem>
  onSelectItem: (item: SelectedItem) => void
  t: (key: string) => string
}

function MobileExtensionCommandsList({
  extension,
  selectedItems,
  onSelectItem,
  t,
}: MobileExtensionCommandsListProps) {
  const commands = extension.commands || []

  if (commands.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm p-4">
        {t('extensions:noCommands') || 'No commands available'}
      </div>
    )
  }

  return (
    <div className="p-4 space-y-3">
      {commands.map((cmd: ExtensionCommandDescriptor) => {
        const itemKey = `extension-command:${extension.id}:${cmd.id}` as SelectedItem
        const isSelected = selectedItems.has(itemKey)

        return (
          <button
            key={cmd.id}
            type="button"
            onClick={() => onSelectItem(itemKey)}
            className={cn(
              'w-full text-left transition-colors duration-150',
              'group relative rounded-2xl border p-4',
              isSelected
                ? 'bg-primary/10 border-primary/50'
                : 'bg-card border-border active:bg-accent/40'
            )}
          >
            <div className="flex items-center gap-3">
              <div className={cn(
                'shrink-0 w-6 h-6 rounded-full flex items-center justify-center transition-colors',
                isSelected
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-muted-foreground'
              )}>
                <Check className={cn(
                  'h-4 w-4',
                  isSelected ? 'opacity-100' : 'opacity-0'
                )} />
              </div>
              <div className="flex-1 min-w-0">
                <div className={cn(
                  'text-base font-medium truncate',
                  isSelected ? 'text-foreground' : 'text-foreground/90'
                )}>
                  {cmd.display_name || cmd.id}
                </div>
                <div className="text-sm text-muted-foreground truncate">
                  {cmd.description || cmd.id}
                </div>
              </div>
              <Zap className={cn(
                'h-5 w-5 shrink-0',
                isSelected ? 'text-amber-500' : 'text-muted-foreground/40'
              )} />
            </div>
          </button>
        )
      })}
    </div>
  )
}

export default UnifiedDataSourceConfig
