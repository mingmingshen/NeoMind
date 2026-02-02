/**
 * Unified DataSource Configuration
 *
 * Simplified data source selector with direct list-based selection.
 * For device-metric and device-command categories, shows device list on left.
 */

import { useState, useMemo, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Search, Check, Server, Zap, Info, X, ChevronRight, Circle, Loader2, Database, MapPin, Activity } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useStore } from '@/store'
import type { DataSource, DataSourceOrList } from '@/types/dashboard'
import { normalizeDataSource } from '@/types/dashboard'
import type { MetricDefinition, CommandDefinition } from '@/types'
import { useDataAvailability } from '@/hooks/useDataAvailability'

// ============================================================================
// Types
// ============================================================================

export interface UnifiedDataSourceConfigProps {
  value?: DataSourceOrList
  onChange: (dataSource: DataSourceOrList | undefined) => void
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command' | 'system'>
  multiple?: boolean
  maxSources?: number
  className?: string
  disabled?: boolean
}

type CategoryType = 'device-metric' | 'device-command' | 'device' | 'system'
type SelectedItem = string // Format: "device-metric:deviceId:property" or "device-command:deviceId:command" or "device:deviceId" or "system:metric"

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
  ]
}

// ============================================================================
// Helper Functions
// ============================================================================

// Convert old allowedTypes format to new format
function normalizeAllowedTypes(
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command' | 'system'>
): CategoryType[] {
  if (!allowedTypes) return ['device', 'device-metric', 'device-command', 'system']

  const result: CategoryType[] = []

  // Device category (for map markers, etc.)
  if (allowedTypes.includes('device')) result.push('device')

  // New format types
  if (allowedTypes.includes('device-metric')) result.push('device-metric')
  if (allowedTypes.includes('device-command')) result.push('device-command')
  if (allowedTypes.includes('system')) result.push('system')

  // Old format types - map to new format (but not 'device' since it's distinct now)
  if (allowedTypes.includes('metric')) {
    if (!result.includes('device-metric')) result.push('device-metric')
  }
  if (allowedTypes.includes('command')) {
    if (!result.includes('device-command')) result.push('device-command')
  }

  return result.length > 0 ? result : ['device', 'device-metric', 'device-command', 'system']
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
          timeRange: 24,  // 24 hours to match availability check
          limit: 100,
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
          timeRange: 24,  // 24 hours to match availability check
          limit: 100,
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

  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategory, setSelectedCategory] = useState<CategoryType>('device-metric')
  const [selectedDeviceId, setSelectedDeviceId] = useState<string | null>(null)

  // Data availability checking - now includes summaries with virtual metrics
  const { availability, summaries, loading: checkingData, checkDevice } = useDataAvailability()

  // Initialize selected items from current data source
  const [selectedItems, setSelectedItems] = useState<Set<SelectedItem>>(() =>
    dataSourceToSelectedItems(value)
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

  // Set initial category to first available
  useMemo(() => {
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

  // Build device commands map
  const deviceCommandsMap = useMemo(() => {
    const map = new Map<string, CommandDefinition[]>()
    for (const device of devices) {
      const deviceType = deviceTypes.find(dt => dt.device_type === device.device_type)

      if (deviceType?.commands && deviceType.commands.length > 0) {
        map.set(device.id, deviceType.commands)
      } else {
        map.set(device.id, [{ name: 'toggle', display_name: t('dataSource.commandToggle'), parameters: [] }])
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

  // Check if category uses device split layout (device category uses single column layout)
  const usesDeviceSplitLayout = selectedCategory === 'device-metric' || selectedCategory === 'device-command'

  // Get selected device
  const selectedDevice = devices.find(d => d.id === selectedDeviceId)

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
                  onClick={() => setSelectedDeviceId(device.id)}
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
                  {selectedCount > 0 && (
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

  // Render content based on category
  const renderCategoryContent = () => {
    switch (selectedCategory) {
      case 'device-metric':
      case 'device-command':
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
              const [type, deviceId, ...rest] = itemKey.split(':')
              const label = rest.join(':')
              const device = devices.find(d => d.id === deviceId)
              const deviceName = device?.name || deviceId

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
              }

              return (
                <div
                  key={itemKey}
                  className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-background border border-border/50 text-xs group hover:border-primary/40 transition-all max-w-[140px]"
                >
                  <TypeIcon className={cn('h-3 w-3 shrink-0', iconColor)} />
                  <span className="max-w-[80px] truncate text-foreground/70" title={deviceName}>{deviceName}</span>
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
      ) : (
        // Single column layout
        <div className="flex-1 overflow-y-auto p-3">
          {renderCategoryContent()}
        </div>
      )}
    </div>
  )
}

export default UnifiedDataSourceConfig
