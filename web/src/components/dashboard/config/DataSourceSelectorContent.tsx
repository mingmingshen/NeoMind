/**
 * DataSourceSelectorContent Component
 *
 * Content-only version of the data source selector for use in dialogs/sheets.
 * Two-panel layout: device list on left, details on right.
 * Shows selected items with individual remove buttons.
 */

import { useState, useMemo, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Search, Server, Check, Zap, Info, ChevronRight, X, ChevronDown, Activity } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { useStore } from '@/store'
import type { DataSource, DataSourceOrList } from '@/types/dashboard'
import { normalizeDataSource } from '@/types/dashboard'
import type { MetricDefinition, CommandDefinition } from '@/types'

export interface DataSourceSelectorContentProps {
  onSelect: (dataSource: DataSourceOrList | DataSource | undefined) => void
  currentDataSource?: DataSourceOrList
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command' | 'system'>
  multiple?: boolean
  maxSources?: number
}

type CategoryType = 'device-metric' | 'device-command' | 'device-info' | 'system'
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
  ]
}

// Convert old allowedTypes format to new format
function normalizeAllowedTypes(
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command' | 'system'>
): CategoryType[] {
  if (!allowedTypes) return ['device-metric', 'device-command', 'device-info', 'system']

  const result: CategoryType[] = []

  // New format types
  if (allowedTypes.includes('device-metric')) result.push('device-metric')
  if (allowedTypes.includes('device-command')) result.push('device-command')
  if (allowedTypes.includes('device-info')) result.push('device-info')
  if (allowedTypes.includes('system')) result.push('system')

  // Old format types - map to new format
  if (allowedTypes.includes('device') || allowedTypes.includes('metric')) {
    if (!result.includes('device-metric')) result.push('device-metric')
  }
  if (allowedTypes.includes('command')) {
    if (!result.includes('device-command')) result.push('device-command')
  }

  return result.length > 0 ? result : ['device-metric', 'device-command', 'device-info', 'system']
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
    }
  }

  return items
}

/**
 * Get a readable label for a selected item
 */
function getSelectedItemLabel(item: SelectedItem, devices: any[], t: (key: string) => string): string {
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

  // Build device commands map
  const deviceCommandsMap = useMemo(() => {
    const map = new Map<string, CommandDefinition[]>()
    for (const device of devices) {
      const deviceType = deviceTypes.find(dt => dt.device_type === device.device_type)

      if (deviceType?.commands && deviceType.commands.length > 0) {
        map.set(device.id, deviceType.commands)
      } else {
        // Fallback commands
        const fallbackCommands: CommandDefinition[] = [
          { name: 'toggle', display_name: t('dataSource.commandToggle'), parameters: [] },
          { name: 'setValue', display_name: t('dataSource.commandSetValue'), parameters: [{ name: 'value', data_type: 'float' }] },
        ]
        map.set(device.id, fallbackCommands)
      }
    }
    return map
  }, [devices, deviceTypes, t])

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

  // System metrics don't require device selection
  const isSystemCategory = selectedCategory === 'system'

  // Auto-select first device when switching to device-based category
  useMemo(() => {
    if (isDeviceBasedCategory && !selectedDeviceId && filteredDevices.length > 0) {
      setSelectedDeviceId(filteredDevices[0]!.id)
    }
  }, [isDeviceBasedCategory, selectedDeviceId, filteredDevices])

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
                const [type, deviceId, ...rest] = item.split(':')
                const label = rest.join(':')
                const device = devices.find(d => d.id === deviceId)
                const deviceName = device?.name || deviceId

                // Icon based on type
                const TypeIcon = type === 'device-metric' ? Server : type === 'device-command' ? Zap : Info
                const iconColor = type === 'device-metric' ? 'text-blue-500' : type === 'device-command' ? 'text-amber-500' : 'text-emerald-500'

                return (
                  <div
                    key={item}
                    className="inline-flex items-center gap-1 px-2 py-1 rounded-md bg-background border border-border/50 text-xs group hover:border-primary/40 transition-all"
                  >
                    <TypeIcon className={cn('h-3 w-3 shrink-0', iconColor)} />
                    <span className="max-w-[60px] truncate text-foreground/70" title={deviceName}>{deviceName}</span>
                    <span className="text-muted-foreground/40">·</span>
                    <span className="max-w-[80px] truncate text-foreground" title={label}>{label}</span>
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
          /* Non-device categories: single panel */
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
        )}
      </div>
    </div>
  )
}
