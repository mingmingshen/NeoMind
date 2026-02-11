/**
 * DataSourceSelectorContent Component
 *
 * Content-only version of the data source selector for use in dialogs/sheets.
 * Two-panel layout: device list on left, details on right.
 * Shows selected items with individual remove buttons.
 */

import { useState, useMemo, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Search, Server, Check, Zap, Info, ChevronRight, X, ChevronDown, Activity, Puzzle } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { useStore } from '@/store'
import type { DataSource, DataSourceOrList } from '@/types/dashboard'
import { normalizeDataSource } from '@/types/dashboard'
import type { MetricDefinition, CommandDefinition } from '@/types'
import { api } from '@/lib/api'
import type { Extension, ExtensionDataSourceInfo, TransformDataSourceInfo } from '@/types'
import { Loader2 } from 'lucide-react'

export interface DataSourceSelectorContentProps {
  onSelect: (dataSource: DataSourceOrList | DataSource | undefined) => void
  currentDataSource?: DataSourceOrList
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command' | 'system' | 'extension'>
  multiple?: boolean
  maxSources?: number
}

type CategoryType = 'device-metric' | 'device-command' | 'device-info' | 'system' | 'extension'
type SelectedItem = string // Format: "device-metric:deviceId:property" or "device-command:deviceId:command" etc.

// Device info property definitions factory (uses translations)
function getDeviceInfoProperties(t: (key: string) => string) {
  return [
    { id: 'name', name: t('dataSource.deviceName'), description: t('dataSource.deviceNameDesc') },
    { id: 'status', name: t('dataSource.status'), description: t('dataSource.statusDesc') },
    { id: 'online', name: t('dataSource.onlineStatus'), description: t('dataSource.onlineStatusDesc') },
    { id: 'last_seen', name: t('dataSource.lastSeen'), description: t('dataSource.lastSeenDesc') },
    { id: 'device_type', name: t('dataSource.deviceType'), description: t('dataSource.deviceTypeDesc') },
    { id: 'plugin_name', name: t('dataSource.adapter'), description: t('dataSource.adapterDesc') },
    { id: 'adapter_id', name: t('dataSource.adapterId'), description: t('dataSource.adapterIdDesc') },
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
    { id: 'device-metric' as const, name: t('dataSource.metrics'), icon: Server, description: t('dataSource.metricsDesc') },
    { id: 'device-command' as const, name: t('dataSource.commands'), icon: Zap, description: t('dataSource.commandsDesc') },
    { id: 'device-info' as const, name: t('dataSource.basicInfo'), icon: Info, description: t('dataSource.basicInfoDesc') },
    { id: 'system' as const, name: t('systemDataSource.title'), icon: Activity, description: t('systemDataSource.description') },
    { id: 'extension' as const, name: t('extensions:dataSource.extensionSource'), icon: Puzzle, description: t('extensions:dataSource.selectExtension') },
  ]
}

// Convert old allowedTypes format to new format
function normalizeAllowedTypes(
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command' | 'system' | 'extension'>
): CategoryType[] {
  if (!allowedTypes) return ['device-metric', 'device-command', 'device-info', 'system', 'extension']

  const result: CategoryType[] = []

  // New format types
  if (allowedTypes.includes('device-metric')) result.push('device-metric')
  if (allowedTypes.includes('device-command')) result.push('device-command')
  if (allowedTypes.includes('device-info')) result.push('device-info')
  if (allowedTypes.includes('system')) result.push('system')
  if (allowedTypes.includes('extension')) result.push('extension')

  // Old format types - map to new format
  if (allowedTypes.includes('device') || allowedTypes.includes('metric')) {
    if (!result.includes('device-metric')) result.push('device-metric')
  }
  if (allowedTypes.includes('command')) {
    if (!result.includes('device-command')) result.push('device-command')
  }

  return result.length > 0 ? result : ['device-metric', 'device-command', 'device-info', 'system', 'extension']
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
    const type = parts[0] as CategoryType

    switch (type) {
      case 'device-metric':
        // Format: device-metric:deviceId:metricId
        return {
          type: 'telemetry',
          deviceId: parts[1],
          metricId: parts.slice(2).join(':'),
          timeRange: 1, // Default 1 hour
          limit: 50,    // Default 50 points
          aggregate: 'raw',
          params: {
            includeRawPoints: true,
          },
          transform: 'raw',
        }
      case 'device-command':
        // Format: device-command:deviceId:command
        return {
          type: 'command',
          deviceId: parts[1],
          command: parts.slice(2).join(':'),
        }
      case 'device-info':
        // Format: device-info:deviceId:property
        return {
          type: 'device-info',
          deviceId: parts[1],
          property: parts.slice(2).join(':'),
        }
      case 'system':
        // Format: system:metricId (not system:deviceId:metricId)
        return {
          type: 'system',
          systemMetric: parts.slice(1).join(':') as any,
          refresh: 10,
        }
      case 'extension':
        // Format: extension:extensionId:metricName
        return {
          type: 'extension',
          extensionId: parts[1],
          extensionMetric: parts.slice(2).join(':'),
          refresh: 10,
        }
      default:
        return undefined
    }
  }

  // Multiple selection - return array
  const result: DataSource[] = []
  for (const item of selectedItems) {
    const parts = item.split(':')
    const type = parts[0] as CategoryType

    switch (type) {
      case 'device-metric':
        // Format: device-metric:deviceId:metricId
        result.push({
          type: 'telemetry',
          deviceId: parts[1],
          metricId: parts.slice(2).join(':'),
          timeRange: 1,
          limit: 50,
          aggregate: 'raw',
          params: {
            includeRawPoints: true,
          },
          transform: 'raw',
        })
        break
      case 'device-command':
        // Format: device-command:deviceId:command
        result.push({
          type: 'command',
          deviceId: parts[1],
          command: parts.slice(2).join(':'),
        })
        break
      case 'device-info':
        // Format: device-info:deviceId:property
        result.push({
          type: 'device-info',
          deviceId: parts[1],
          property: parts.slice(2).join(':'),
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
        // Format: extension:extensionId:metricName
        result.push({
          type: 'extension',
          extensionId: parts[1],
          extensionMetric: parts.slice(2).join(':'),
          refresh: 10,
        })
        break
    }
  }

  return result
}

/**
 * Parse current data source to selected items
 */
function dataSourceToSelectedItems(ds: DataSourceOrList | undefined): Set<SelectedItem> {
  const items = new Set<SelectedItem>()
  if (!ds) return items

  const dataSources = normalizeDataSource(ds)

  for (const dataSource of dataSources) {
    switch (dataSource.type) {
      case 'device':
        items.add(`device-metric:${dataSource.deviceId}:${dataSource.property}` as SelectedItem)
        break
      case 'telemetry':
        // Telemetry type is used for historical device metric data
        items.add(`device-metric:${dataSource.deviceId}:${dataSource.metricId}` as SelectedItem)
        break
      case 'command':
        items.add(`device-command:${dataSource.deviceId}:${dataSource.command}` as SelectedItem)
        break
      case 'device-info':
        items.add(`device-info:${dataSource.deviceId}:${dataSource.property}` as SelectedItem)
        break
      case 'system':
        items.add(`system:${dataSource.systemMetric}` as SelectedItem)
        break
      case 'extension':
        items.add(`extension:${dataSource.extensionId}:${dataSource.extensionMetric}` as SelectedItem)
        break
    }
  }

  return items
}

/**
 * Get a readable label for a selected item
 */
function getSelectedItemLabel(item: SelectedItem, devices: any[], t: (key: string) => string, extensions?: Extension[]): string {
  const parts = item.split(':')
  const type = parts[0] as CategoryType

  switch (type) {
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
      const prop = getDeviceInfoProperties(t).find(p => p.id === parts.slice(2).join(':'))
      return `${deviceName} · ${prop?.name || parts.slice(2).join(':')}`
    }
    case 'system': {
      // Format: system:metricId (not system:deviceId:metricId)
      const metricId = parts.slice(1).join(':')
      const systemMetric = getSystemMetrics(t).find(m => m.id === metricId)
      return `${t('systemDataSource.title')} · ${systemMetric?.name || metricId}`
    }
    case 'extension': {
      // Format: extension:extensionId:metricName
      const extensionId = parts[1]
      const metricName = parts.slice(2).join(':')
      const extension = extensions?.find(e => e.id === extensionId)
      const extensionName = extension?.name || extensionId
      return `${t('extensions:dataSource.extensionSource')} · ${extensionName} · ${metricName}`
    }
    default:
      return item
  }
}

export function DataSourceSelectorContent({
  onSelect,
  currentDataSource,
  allowedTypes,
  multiple = false,
  maxSources = 10,
}: DataSourceSelectorContentProps) {
  const { t } = useTranslation('dashboardComponents')
  const { devices, deviceTypes } = useStore()

  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategory, setSelectedCategory] = useState<CategoryType>('device-metric')
  const [selectedDeviceId, setSelectedDeviceId] = useState<string | null>(null)
  const [showSelectedItems, setShowSelectedItems] = useState(true)

  // Extension capabilities state (V2: using Extension and ExtensionDataSourceInfo)
  const [extensions, setExtensions] = useState<Extension[]>([])
  const [extensionDataSources, setExtensionDataSources] = useState<ExtensionDataSourceInfo[]>([])
  const [extensionsLoading, setExtensionsLoading] = useState(false)
  const [extensionError, setExtensionError] = useState<string | null>(null)

  // Initialize selected items from current data source
  const [selectedItems, setSelectedItems] = useState<Set<SelectedItem>>(() =>
    dataSourceToSelectedItems(currentDataSource)
  )

  // Track previous value to detect actual selection changes
  const prevCoreFieldsRef = useRef<string>()
  const prevValueRef = useRef<DataSourceOrList>()

  // Extract core identifying fields for comparison (ignores transform settings)
  const getCoreFields = (ds: DataSourceOrList | undefined): string => {
    if (!ds) return ''
    const sources = Array.isArray(ds) ? ds : [ds]
    return sources.map(s => {
      // Only include fields that identify the selection, not transform settings
      // Exclude: timeRange, limit, aggregate, aggregateExt, transform, params, timeWindow
      if (s.type === 'extension') {
        return `${s.type}:${s.extensionId || ''}:${s.extensionMetric || ''}`
      }
      return `${s.type}:${s.deviceId || ''}:${s.metricId || s.property || s.infoProperty || ''}:${s.command || ''}`
    }).sort().join('|')
  }

  // Calculate current core fields
  const currentCoreFields = getCoreFields(currentDataSource)

  // Sync selected items when currentDataSource prop changes
  // Only reset if core fields changed, not when just transform settings changed
  useEffect(() => {
    // Initialize on first render
    if (prevValueRef.current === undefined) {
      prevValueRef.current = currentDataSource
      prevCoreFieldsRef.current = currentCoreFields
      return
    }

    // Only update if core fields actually changed (selection changed)
    // Transform settings like timeRange, aggregate, etc. should NOT trigger reset
    if (prevCoreFieldsRef.current !== currentCoreFields) {
      prevValueRef.current = currentDataSource
      prevCoreFieldsRef.current = currentCoreFields
      setSelectedItems(dataSourceToSelectedItems(currentDataSource))
    }
    // If only transform settings changed, update the value ref but don't reset selection
    else if (prevCoreFieldsRef.current === currentCoreFields && currentDataSource !== prevValueRef.current) {
      prevValueRef.current = currentDataSource
    }
  }, [currentDataSource, currentCoreFields])

  // Available categories based on allowedTypes
  const availableCategories = useMemo(
    () => getCategories(t).filter(c => (normalizeAllowedTypes(allowedTypes) as CategoryType[]).includes(c.id)),
    [allowedTypes, t]
  )

  // Set initial category to first available
  useMemo(() => {
    if (availableCategories.length > 0 && !availableCategories.find(c => c.id === selectedCategory)) {
      setSelectedCategory(availableCategories[0]!.id)
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
        // Fallback metrics for devices without type definition
        const fallbackMetrics: MetricDefinition[] = [
          { name: 'temperature', display_name: t('chart.temperature'), data_type: 'float', unit: '°C' },
          { name: 'humidity', display_name: t('chart.humidity'), data_type: 'float', unit: '%' },
          { name: 'value', display_name: t('chart.value'), data_type: 'float', unit: '' },
        ]
        map.set(device.id, fallbackMetrics)
      }
    }
    return map
  }, [devices, deviceTypes, t])

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

  // Fetch extension capabilities when extension category is selected or available (V2 API)
  // Use a ref to track if we've already fetched to prevent infinite loops
  const hasFetchedExtensions = useRef(false)

  useEffect(() => {
    const hasExtensionCategory = availableCategories.some(c => c.id === 'extension')
    if (!hasExtensionCategory) {
      hasFetchedExtensions.current = false
      return
    }

    // Skip if already fetched (only re-fetch if explicitly needed)
    if (hasFetchedExtensions.current && extensions.length > 0) {
      return
    }

    const fetchExtensionData = async () => {
      setExtensionsLoading(true)
      setExtensionError(null)
      try {
        // V2 API: fetch extensions and their data sources
        const [extData, dsData] = await Promise.all([
          api.listExtensions().catch((): Extension[] => {
            return []
          }),
          api.listAllDataSources().catch((): (ExtensionDataSourceInfo | TransformDataSourceInfo)[] => {
            return []
          }),
        ])
        setExtensions(extData)
        // Filter only extension data sources (exclude transform data sources)
        setExtensionDataSources(dsData.filter((source): source is ExtensionDataSourceInfo => 'extension_id' in source))
        hasFetchedExtensions.current = true
      } catch (err) {
        console.error('[DataSourceSelectorContent] Error fetching extension data:', err)
        setExtensionError((err as Error).message)
      } finally {
        setExtensionsLoading(false)
      }
    }

    fetchExtensionData()
  }, [availableCategories, extensions.length])

  // Get extension metrics grouped by extension (V2: using ExtensionDataSourceInfo)
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
          // Clicking the same item deselects it (clears selection)
          next.clear()
        } else {
          // Select new item, clearing others
          next.clear()
          next.add(itemKey)
        }
      }

      // Auto-apply selection
      const dataSource = selectedItemsToDataSource(next, multiple)
      onSelect(dataSource)

      return next
    })
  }

  // Remove a specific selected item
  const handleRemoveItem = (itemKey: SelectedItem) => {
    setSelectedItems(prev => {
      const next = new Set(prev)
      next.delete(itemKey)

      const dataSource = selectedItemsToDataSource(next, multiple)
      onSelect(dataSource)

      return next
    })
  }

  // Clear all selections
  const handleClearSelection = () => {
    setSelectedItems(new Set())
    onSelect(undefined)
  }

  // Get current category config
  const categoryConfig = getCategories(t).find(c => c.id === selectedCategory)
  const CategoryIcon = categoryConfig?.icon || Server

  // Filter devices by search query
  const filteredDevices = useMemo(() => {
    if (!searchQuery) return devices

    const query = searchQuery.toLowerCase()
    return devices.filter(d =>
      d.id.toLowerCase().includes(query) ||
      (d.name && d.name.toLowerCase().includes(query))
    )
  }, [devices, searchQuery])

  // Check if current category is device-based (has device list)
  const isDeviceBasedCategory = selectedCategory === 'device-metric' ||
                                selectedCategory === 'device-command' ||
                                selectedCategory === 'device-info'

  // Non-device categories (single panel layout)
  const isSystemCategory = selectedCategory === 'system'
  const isExtensionCategory = selectedCategory === 'extension'

  // Auto-select first device/extension when switching to a category
  useMemo(() => {
    if (isDeviceBasedCategory && !selectedDeviceId && filteredDevices.length > 0) {
      setSelectedDeviceId(filteredDevices[0]!.id)
    } else if (isExtensionCategory && !selectedDeviceId && extensions.length > 0) {
      // Use selectedDeviceId to track the selected extension for consistency
      setSelectedDeviceId(extensions[0]!.id)
    }
  }, [isDeviceBasedCategory, isExtensionCategory, selectedDeviceId, filteredDevices, extensions])

  // Convert selected items to array for display
  const selectedItemsArray = useMemo(() => Array.from(selectedItems), [selectedItems])

  return (
    <div className="flex flex-col h-full">
      {/* Search & Actions */}
      <div className="p-3 border-b shrink-0 flex gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={t('dataSource.searchPlaceholder')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-8 pl-9 text-xs"
          />
        </div>
        {selectedItems.size > 0 && (
          <button
            onClick={handleClearSelection}
            className="h-8 px-2 text-xs flex items-center gap-1 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded-md transition-colors shrink-0"
            title={t('dataSource.clearAllSelections')}
          >
            <X className="h-3.5 w-3.5" />
            <span className="hidden sm:inline">{t('dataSource.clearAll')}</span>
          </button>
        )}
      </div>

      {/* Selected Items Panel (shown when there are selections in multiple mode) */}
      {multiple && selectedItems.size > 0 && (
        <div className="px-3 py-2 border-b bg-gradient-to-r from-primary/5 via-primary/5 to-muted/20 shrink-0">
          <button
            onClick={() => setShowSelectedItems(!showSelectedItems)}
            className="flex items-center justify-between w-full text-xs text-muted-foreground hover:text-foreground transition-colors"
          >
            <span className="flex items-center gap-1.5">
              <Check className="h-3.5 w-3.5 text-primary" />
              <span className="font-medium">{t('dataSource.selectedItems', { count: selectedItems.size })}</span>
            </span>
            <ChevronDown className={cn(
              'h-3.5 w-3.5 transition-transform',
              showSelectedItems ? 'rotate-180' : ''
            )} />
          </button>
          {showSelectedItems && (
            <div className="mt-2 flex flex-wrap gap-1.5">
              {selectedItemsArray.map(item => {
                const parts = item.split(':')
                const type = parts[0] as CategoryType

                // Icon based on type
                let TypeIcon = Server
                let iconColor = 'text-blue-500'
                let nameDisplay = ''
                let labelDisplay = ''

                if (type === 'extension') {
                  TypeIcon = Puzzle
                  iconColor = 'text-purple-500'
                  const extensionId = parts[1]
                  const metricName = parts.slice(2).join(':')
                  const extension = extensions.find(e => e.id === extensionId)
                  nameDisplay = extension?.name || extensionId
                  labelDisplay = metricName
                } else {
                  const deviceId = parts[1]
                  const label = parts.slice(2).join(':')
                  const device = devices.find(d => d.id === deviceId)
                  const deviceName = device?.name || deviceId

                  if (type === 'device-metric') {
                    TypeIcon = Server
                    iconColor = 'text-blue-500'
                  } else if (type === 'device-command') {
                    TypeIcon = Zap
                    iconColor = 'text-amber-500'
                  } else if (type === 'system') {
                    TypeIcon = Activity
                    iconColor = 'text-cyan-500'
                  } else {
                    TypeIcon = Info
                    iconColor = 'text-emerald-500'
                  }

                  nameDisplay = deviceName
                  labelDisplay = label
                }

                return (
                  <div
                    key={item}
                    className="inline-flex items-center gap-1 px-2 py-1 rounded-md bg-background border border-border/50 text-xs group hover:border-primary/40 transition-all"
                  >
                    <TypeIcon className={cn('h-3 w-3 shrink-0', iconColor)} />
                    <span className="max-w-[60px] truncate text-foreground/70" title={nameDisplay}>{nameDisplay}</span>
                    <span className="text-muted-foreground/40">·</span>
                    <span className="max-w-[80px] truncate text-foreground" title={labelDisplay}>{labelDisplay}</span>
                    <button
                      onClick={() => handleRemoveItem(item)}
                      className="ml-0.5 opacity-40 group-hover:opacity-100 hover:text-destructive transition-all"
                      title={t('dataSource.removeItem')}
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </div>
                )
              })}
            </div>
          )}
        </div>
      )}

      {/* Category Tabs */}
      <div className="flex border-b shrink-0 overflow-x-auto">
        {availableCategories.map(cat => {
          const Icon = cat.icon
          return (
            <button
              key={cat.id}
              onClick={() => {
                setSelectedCategory(cat.id)
                setSelectedDeviceId(null)
              }}
              className={cn(
                'flex items-center gap-1 px-2 py-2 text-xs whitespace-nowrap transition-colors border-b-2 shrink-0',
                selectedCategory === cat.id
                  ? 'border-primary text-primary bg-primary/5'
                  : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/50'
              )}
            >
              <Icon className="h-3.5 w-3.5" />
              <span>{cat.name}</span>
            </button>
          )
        })}
      </div>

      {/* Two-panel Content Area */}
      <div className="flex-1 flex min-h-0">
        {/* Device-based categories: two-panel layout */}
        {isDeviceBasedCategory ? (
          <>
            {/* Left: Device List */}
            <div className="w-[180px] border-r shrink-0 overflow-y-auto">
              <div className="p-1 space-y-0.5">
                {filteredDevices.length === 0 ? (
                  <div className="text-center py-4 text-muted-foreground">
                    <p className="text-xs">{t('dataSource.noDevices')}</p>
                  </div>
                ) : (
                  filteredDevices.map(device => {
                    // Count how many items from this device are selected
                    const deviceSelectedCount = selectedItemsArray.filter(item =>
                      item.startsWith(`device-metric:${device.id}:`) ||
                      item.startsWith(`device-command:${device.id}:`) ||
                      item.startsWith(`device-info:${device.id}:`)
                    ).length

                    const isSelected = selectedDeviceId === device.id

                    return (
                      <button
                        key={device.id}
                        onClick={() => setSelectedDeviceId(device.id)}
                        className={cn(
                          'w-full flex items-center gap-2 px-2 py-1.5 text-xs text-left rounded-md transition-colors',
                          isSelected ? 'bg-primary text-primary-foreground' : 'hover:bg-muted/50'
                        )}
                      >
                        <div className="flex-1 min-w-0 truncate">
                          {device.name || device.id}
                        </div>
                        {deviceSelectedCount > 0 && !isSelected && (
                          <span className="text-[10px] bg-primary/20 text-primary px-1 rounded-full shrink-0">
                            {deviceSelectedCount}
                          </span>
                        )}
                        {isSelected && <ChevronRight className="h-3 w-3 shrink-0" />}
                      </button>
                    )
                  })
                )}
              </div>
            </div>

            {/* Right: Details Panel */}
            <div className="flex-1 overflow-y-auto">
              {!selectedDeviceId ? (
                <div className="flex items-center justify-center h-full text-muted-foreground">
                  <p className="text-xs">{t('dataSource.selectDevice')}</p>
                </div>
              ) : (
                <div className="p-2">
                  {selectedCategory === 'device-metric' && (
                    <div className="space-y-0.5">
                      {(deviceMetricsMap.get(selectedDeviceId) || []).map(metric => {
                        const itemKey = `device-metric:${selectedDeviceId}:${metric.name}` as SelectedItem
                        const isSelected = selectedItems.has(itemKey)

                        return (
                          <button
                            key={metric.name}
                            onClick={() => handleSelectItem(itemKey)}
                            className={cn(
                              'w-full flex items-center justify-between px-3 py-1.5 text-xs hover:bg-muted/50 rounded-md transition-colors',
                              isSelected && 'bg-primary/10 text-primary'
                            )}
                          >
                            <span>{metric.display_name || metric.name}</span>
                            {isSelected && <Check className="h-3.5 w-3.5" />}
                          </button>
                        )
                      })}
                    </div>
                  )}

                  {selectedCategory === 'device-command' && (
                    <div className="space-y-0.5">
                      {(deviceCommandsMap.get(selectedDeviceId) || []).map(command => {
                        const itemKey = `device-command:${selectedDeviceId}:${command.name}` as SelectedItem
                        const isSelected = selectedItems.has(itemKey)

                        return (
                          <button
                            key={command.name}
                            onClick={() => handleSelectItem(itemKey)}
                            className={cn(
                              'w-full flex items-center justify-between px-3 py-1.5 text-xs hover:bg-muted/50 rounded-md transition-colors',
                              isSelected && 'bg-primary/10 text-primary'
                            )}
                          >
                            <span>{command.display_name || command.name}</span>
                            {isSelected && <Check className="h-3.5 w-3.5" />}
                          </button>
                        )
                      })}
                    </div>
                  )}

                  {selectedCategory === 'device-info' && (
                    <div className="space-y-0.5">
                      {getDeviceInfoProperties(t).map(prop => {
                        const itemKey = `device-info:${selectedDeviceId}:${prop.id}` as SelectedItem
                        const isSelected = selectedItems.has(itemKey)

                        return (
                          <button
                            key={prop.id}
                            onClick={() => handleSelectItem(itemKey)}
                            className={cn(
                              'w-full flex items-center justify-between px-3 py-1.5 text-xs hover:bg-muted/50 rounded-md transition-colors',
                              isSelected && 'bg-primary/10 text-primary'
                            )}
                          >
                            <div className="text-left">
                              <div className="font-medium">{prop.name}</div>
                              <div className="text-[10px] text-muted-foreground">{prop.description}</div>
                            </div>
                            {isSelected && <Check className="h-3.5 w-3.5 shrink-0" />}
                          </button>
                        )
                      })}
                    </div>
                  )}
                </div>
              )}
            </div>
          </>
        ) : (
          /* Extension category: two-panel layout (unified with device selection) */
          isExtensionCategory ? (
            <>
              {/* Left: Extension List */}
              <div className="w-[180px] border-r shrink-0 overflow-y-auto">
                <div className="p-1 space-y-0.5">
                  {extensionsLoading ? (
                    <div className="flex items-center justify-center py-4">
                      <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                    </div>
                  ) : extensionError ? (
                    <div className="text-center py-4 text-destructive">
                      <p className="text-[10px]">{t('extensions:errorLoadingCapabilities', { error: extensionError })}</p>
                    </div>
                  ) : extensions.length === 0 ? (
                    <div className="text-center py-4 text-muted-foreground">
                      <p className="text-xs">{t('extensions:dataSource.noProviderExtensions')}</p>
                    </div>
                  ) : (
                    extensions.map(ext => {
                      // Count how many metrics from this extension are selected
                      const extensionSelectedCount = selectedItemsArray.filter(item =>
                        item.startsWith(`extension:${ext.id}:`)
                      ).length

                      const isSelected = selectedDeviceId === ext.id
                      const metrics = extensionMetricsMap.get(ext.id) || []
                      // Filter by search
                      const filteredMetrics = searchQuery
                        ? metrics.filter(metric => {
                            const query = searchQuery.toLowerCase()
                            return (
                              metric.name.toLowerCase().includes(query) ||
                              (metric.display_name && metric.display_name.toLowerCase().includes(query))
                            )
                          })
                        : metrics

                      // Skip if no metrics after search
                      if (searchQuery && filteredMetrics.length === 0) return null

                      return (
                        <button
                          key={ext.id}
                          onClick={() => setSelectedDeviceId(ext.id)}
                          className={cn(
                            'w-full flex items-center gap-2 px-2 py-1.5 text-xs text-left rounded-md transition-colors',
                            isSelected ? 'bg-primary text-primary-foreground' : 'hover:bg-muted/50'
                          )}
                        >
                          <div className="flex-1 min-w-0 truncate">
                            {ext.name}
                          </div>
                          <span className={cn(
                            "text-[10px] px-1 rounded-full shrink-0",
                            extensionSelectedCount > 0 && !isSelected ? "bg-primary/20 text-primary" : "text-muted-foreground"
                          )}>
                            {searchQuery ? filteredMetrics.length : metrics.length}
                          </span>
                          {isSelected && <ChevronRight className="h-3 w-3 shrink-0" />}
                        </button>
                      )
                    })
                  )}
                </div>
              </div>

              {/* Right: Metrics Panel */}
              <div className="flex-1 overflow-y-auto">
                {!selectedDeviceId ? (
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    <p className="text-xs">{t('extensions:dataSource.selectExtension', { defaultValue: 'Select an extension' })}</p>
                  </div>
                ) : (
                  <div className="p-2">
                    {(() => {
                      const metrics = extensionMetricsMap.get(selectedDeviceId) || []

                      // Filter by search
                      const filteredMetrics = searchQuery
                        ? metrics.filter(metric => {
                            const query = searchQuery.toLowerCase()
                            return (
                              metric.name.toLowerCase().includes(query) ||
                              (metric.display_name && metric.display_name.toLowerCase().includes(query))
                            )
                          })
                        : metrics

                      if (filteredMetrics.length === 0) {
                        return (
                          <div className="text-center py-4 text-muted-foreground">
                            <p className="text-xs">{t('dataSource.noMetrics', { defaultValue: 'No metrics available' })}</p>
                          </div>
                        )
                      }

                      return (
                        <div className="space-y-0.5">
                          {filteredMetrics.map(metric => {
                            const itemKey = `extension:${selectedDeviceId}:${metric.name}` as SelectedItem
                            const isSelected = selectedItems.has(itemKey)

                            return (
                              <button
                                key={metric.name}
                                onClick={() => handleSelectItem(itemKey)}
                                className={cn(
                                  'w-full flex items-center justify-between px-3 py-1.5 text-xs hover:bg-muted/50 rounded-md transition-colors',
                                  isSelected && 'bg-primary/10 text-primary'
                                )}
                              >
                                <div className="text-left">
                                  <div className="font-medium">{metric.display_name}</div>
                                  {metric.unit && (
                                    <div className="text-[10px] text-muted-foreground">
                                      Unit: {metric.unit} · Type: {metric.data_type}
                                    </div>
                                  )}
                                </div>
                                {isSelected && <Check className="h-3.5 w-3.5 shrink-0" />}
                              </button>
                            )
                          })}
                        </div>
                      )
                    })()}
                  </div>
                )}
              </div>
            </>
          ) : (
            /* System category: single panel */
            <div className="flex-1 overflow-y-auto p-2">
              {selectedCategory === 'system' && (
                <div className="space-y-0.5">
                  {getSystemMetrics(t).map(metric => {
                    const itemKey = `system:${metric.id}` as SelectedItem
                    const isSelected = selectedItems.has(itemKey)

                    return (
                      <button
                        key={metric.id}
                        onClick={() => handleSelectItem(itemKey)}
                        className={cn(
                          'w-full flex items-center justify-between px-3 py-2 text-xs hover:bg-muted/50 rounded-md transition-colors',
                          isSelected && 'bg-primary/10 text-primary'
                        )}
                      >
                        <div className="text-left">
                          <div className="font-medium">{metric.name}</div>
                          <div className="text-[10px] text-muted-foreground">{metric.description}</div>
                        </div>
                        {isSelected && <Check className="h-3.5 w-3.5 shrink-0" />}
                      </button>
                    )
                  })}
                </div>
              )}
            </div>
          )
        )}
      </div>
    </div>
  )
}
