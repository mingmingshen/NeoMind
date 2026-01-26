/**
 * Unified DataSource Configuration
 *
 * Simplified data source selector with direct list-based selection.
 * For device-metric and device-command categories, shows device list on left.
 */

import { useState, useMemo, useEffect, useRef } from 'react'
import { Search, Check, Server, Zap, Info, X, ChevronRight, Circle, Loader2, Database } from 'lucide-react'
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
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command'>
  multiple?: boolean
  maxSources?: number
  className?: string
  disabled?: boolean
}

type CategoryType = 'device-metric' | 'device-command'
type SelectedItem = string // Format: "device-metric:deviceId:property" or "device-command:deviceId:command" or "device-info:deviceId:property"

// ============================================================================
// Constants
// ============================================================================

// Device info property definitions
const DEVICE_INFO_PROPERTIES = [
  { id: 'name', name: '设备名称' },
  { id: 'status', name: '状态' },
  { id: 'online', name: '在线状态' },
  { id: 'last_seen', name: '最后上线' },
  { id: 'device_type', name: '设备类型' },
  { id: 'plugin_name', name: '适配器' },
  { id: 'adapter_id', name: '适配器ID' },
]

// Category configuration
const CATEGORIES = [
  { id: 'device-metric' as const, name: '指标', icon: Server, description: '设备的实时数据点' },
  { id: 'device-command' as const, name: '指令', icon: Zap, description: '控制设备的操作' },
]

// ============================================================================
// Helper Functions
// ============================================================================

// Convert old allowedTypes format to new format
function normalizeAllowedTypes(
  allowedTypes?: Array<'device-metric' | 'device-command' | 'device-info' | 'device' | 'metric' | 'command'>
): CategoryType[] {
  if (!allowedTypes) return ['device-metric', 'device-command']

  const result: CategoryType[] = []

  // New format types
  if (allowedTypes.includes('device-metric')) result.push('device-metric')
  if (allowedTypes.includes('device-command')) result.push('device-command')

  // Old format types - map to new format
  if (allowedTypes.includes('device') || allowedTypes.includes('metric')) {
    if (!result.includes('device-metric')) result.push('device-metric')
  }
  if (allowedTypes.includes('command')) {
    if (!result.includes('device-command')) result.push('device-command')
  }

  return result.length > 0 ? result : ['device-metric', 'device-command']
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
    const [type, deviceId, ...rest] = item.split(':')

    switch (type) {
      case 'device-metric':
        return {
          type: 'telemetry',
          deviceId,
          metricId: rest.join(':'),
          timeRange: 24,  // 24 hours to match availability check
          limit: 100,
          aggregate: 'raw',
          params: { includeRawPoints: true },
          transform: 'raw',
        }
      case 'device-command':
        return {
          type: 'command',
          deviceId,
          command: rest.join(':'),
        }
      case 'device-info':
        return {
          type: 'device-info',
          deviceId,
          infoProperty: rest.join(':') as any,
        }
      default:
        return undefined
    }
  }

  // Multiple selection - return array
  const result: DataSource[] = []
  for (const item of selectedItems) {
    const [type, deviceId, ...rest] = item.split(':')

    switch (type) {
      case 'device-metric':
        result.push({
          type: 'telemetry',
          deviceId,
          metricId: rest.join(':'),
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
          deviceId,
          command: rest.join(':'),
        })
        break
      case 'device-info':
        result.push({
          type: 'device-info',
          deviceId,
          infoProperty: rest.join(':') as any,
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
        items.add(`device-metric:${dataSource.deviceId}:${dataSource.property}` as SelectedItem)
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
    }
  }

  return items
}

/**
 * Get a readable label for a selected item
 */
function getSelectedItemLabel(item: SelectedItem, devices: any[]): string {
  const [type, deviceId, ...rest] = item.split(':')

  switch (type) {
    case 'device-metric':
      const device1 = devices.find(d => d.id === deviceId)
      const deviceName1 = device1?.name || deviceId
      return `${deviceName1} · ${rest.join(':')}`
    case 'device-command':
      const device2 = devices.find(d => d.id === deviceId)
      const deviceName2 = device2?.name || deviceId
      return `${deviceName2} · ${rest.join(':')}`
    case 'device-info':
      const device3 = devices.find(d => d.id === deviceId)
      const deviceName3 = device3?.name || deviceId
      const prop = DEVICE_INFO_PROPERTIES.find(p => p.id === rest.join(':'))
      return `${deviceName3} · ${prop?.name || rest.join(':')}`
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
  const prevValueRef = useRef<string>()
  // Extract core identifying fields for comparison (ignores transform settings)
  const getCoreFields = (ds: DataSourceOrList | undefined): string => {
    if (!ds) return ''
    const sources = Array.isArray(ds) ? ds : [ds]
    return sources.map(s => {
      // Only include fields that identify the selection, not transform settings
      return `${s.type}:${s.deviceId || ''}:${s.metricId || s.property || s.infoProperty || ''}:${s.command || ''}`
    }).sort().join('|')
  }

  // Sync selected items when value prop changes
  // Only update if the core identifying fields change, not when just transform settings change
  const coreFields = useMemo(() => getCoreFields(value), [value])

  useEffect(() => {
    if (prevValueRef.current !== coreFields) {
      prevValueRef.current = coreFields
      setSelectedItems(dataSourceToSelectedItems(value))
    }
  }, [coreFields])

  // Available categories based on allowedTypes
  const availableCategories = useMemo(
    () => CATEGORIES.filter(c => normalizeAllowedTypes(allowedTypes).includes(c.id)),
    [allowedTypes]
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
          map.set(device.id, [{ name: 'value', display_name: '数值', data_type: 'float' as const, unit: '' }])
        }
      }
    }
    return map
  }, [devices, deviceTypes])

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
        map.set(device.id, [{ name: 'toggle', display_name: '切换', parameters: [] }])
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
  const categoryConfig = CATEGORIES.find(c => c.id === selectedCategory)

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

  // Check if category uses device split layout
  const usesDeviceSplitLayout = selectedCategory === 'device-metric' || selectedCategory === 'device-command' || selectedCategory === 'device-info'

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
              placeholder="搜索设备..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="h-8 pl-8 text-xs"
            />
          </div>
        </div>

        {/* Device list header with count */}
        <div className="px-3 py-1.5 border-b text-xs font-medium text-muted-foreground bg-muted/30 flex items-center justify-between">
          <span>设备列表</span>
          <span className="text-[10px] text-muted-foreground/60 bg-muted/50 px-1.5 py-0.5 rounded">
            {filteredDevices.length} 个
          </span>
        </div>

        {/* Device list */}
        {filteredDevices.length === 0 ? (
          <div className="p-4 text-center text-muted-foreground text-xs">暂无可用设备</div>
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
                            {selectedCategory === 'device-metric' ? `${availableCount} 个指标` : `${availableCount} 个指令`}
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
          请选择一个设备
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
      for (const infoProp of DEVICE_INFO_PROPERTIES) {
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
        return <div className="p-4 text-center text-muted-foreground text-sm">该设备暂无可用指标</div>
      }

      // Helper to format current value
      const formatValue = (val: unknown): string => {
        if (val === null || val === undefined) return '-'
        if (typeof val === 'number') return val.toLocaleString('zh-CN', { maximumFractionDigits: 2 })
        if (typeof val === 'boolean') return val ? '是' : '否'
        return String(val)
      }

      // Badge component for item type
      const ItemBadge = ({ itemType }: { itemType: 'template' | 'virtual' | 'info' }) => {
        const config = {
          template: { label: '原生', className: 'bg-blue-500/10 text-blue-600 border-blue-500/20' },
          virtual: { label: '虚拟', className: 'bg-purple-500/10 text-purple-600 border-purple-500/20' },
          info: { label: '信息', className: 'bg-amber-500/10 text-amber-600 border-amber-500/20' },
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
            <div className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-green-500/10 border border-green-500/20" title={`有历史数据 (${count ?? 0} 个数据点)`}>
              <Circle className="h-1.5 w-1.5 fill-green-500 text-green-500" />
              <span className="text-[10px] text-green-600 font-medium">{count ?? 0}</span>
            </div>
          )
        }
        if (hasData === false) {
          return (
            <div className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-muted/30 border border-muted/30" title="暂无历史数据">
              <Circle className="h-1.5 w-1.5 fill-muted-foreground/40 text-muted-foreground/40" />
              <span className="text-[10px] text-muted-foreground">无数据</span>
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
              {selectedDevice.name || selectedDevice.id} 的指标
            </span>
            <span className="text-[10px] text-muted-foreground/60 bg-muted/50 px-1.5 py-0.5 rounded">
              {items.length} 个
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
                          <span className="text-foreground/60">当前: {formatValue(item.currentValue)}</span>
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
        return <div className="p-4 text-center text-muted-foreground text-sm">该设备暂无可用指令</div>
      }

      return (
        <div className="flex flex-col h-full">
          <div className="px-3 py-2 border-b text-xs font-medium text-muted-foreground bg-muted/30">
            {selectedDevice.name || selectedDevice.id} 的指令
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

      default:
        return null
    }
  }

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Selected items bar - moved to top */}
      {selectedItems.size > 0 && (
        <div className="px-3 py-2 border-b bg-primary/5 flex flex-wrap gap-2 items-center">
          <div className="flex items-center gap-1.5 text-xs font-medium text-primary">
            <Check className="h-3.5 w-3.5" />
            已选 {selectedItems.size} 项
          </div>
          <div className="flex-1" />
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClearSelection}
            className="h-7 px-2 text-xs hover:bg-destructive/10 hover:text-destructive"
          >
            <X className="h-3.5 w-3.5 mr-1" />
            清除
          </Button>
        </div>
      )}

      {/* Selected items chips - below the bar */}
      {selectedItems.size > 0 && (
        <div className="px-3 py-2 border-b bg-muted/20 flex flex-wrap gap-1.5 max-h-24 overflow-y-auto">
          {selectedItemsArray.map(itemKey => {
            const label = getSelectedItemLabel(itemKey, devices)
            return (
              <div
                key={itemKey}
                className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-background border text-xs group hover:border-primary/50 transition-colors"
              >
                <span className="max-w-[120px] truncate">{label}</span>
                <button
                  onClick={() => handleRemoveItem(itemKey)}
                  className="opacity-40 group-hover:opacity-100 hover:text-destructive transition-all"
                >
                  <X className="h-3 w-3" />
                </button>
              </div>
            )
          })}
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
