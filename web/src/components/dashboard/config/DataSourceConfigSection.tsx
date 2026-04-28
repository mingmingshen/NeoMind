/**
 * DataSourceConfigSection Component
 *
 * Standardized data source configuration section.
 * All components with data binding use this consistent UI.
 */

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ConfigSection } from './ConfigSection'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Database, Zap, Settings, Search, Loader2 } from 'lucide-react'
import type { DataSource, DataSourceOrList } from '@/types/dashboard'
import { normalizeDataSource as normalizeDs, getSourceId } from '@/types/dashboard'
import { useStore } from '@/store'
import { toast } from '@/components/ui/use-toast'

export interface DataSourceConfigSectionProps {
  dataSource?: DataSourceOrList
  onChange: (dataSource: DataSourceOrList | DataSource | undefined) => void
  collapsible?: boolean
  multiple?: boolean
  maxSources?: number
  // Allowed data source types - support both old and new formats
  allowedTypes?: Array<'device' | 'metric' | 'command' | 'device-metric' | 'device-command' | 'device-info' | 'system'>
}

export function DataSourceConfigSection({
  dataSource,
  onChange,
  collapsible = true,
  // Reserved for future multi-select functionality
  multiple: _multiple = false,
  // Reserved for future multi-select functionality
  maxSources: _maxSources,
  allowedTypes = ['device', 'metric', 'command', 'system'],
}: DataSourceConfigSectionProps) {
  const { t } = useTranslation('dashboardComponents')
  const dataSources = normalizeDs(dataSource)
  const isBound = dataSources.length > 0

  // Get devices and device types from store
  const devices = useStore(state => state.devices)
  const deviceTypes = useStore(state => state.deviceTypes)

  // Dialog state
  const [deviceDialogOpen, setDeviceDialogOpen] = useState(false)
  const [metricDialogOpen, setMetricDialogOpen] = useState(false)
  const [commandDialogOpen, setCommandDialogOpen] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')

  const getDisplayText = () => {
    if (dataSources.length === 0) return t('configRenderer.dataSource')
    if (dataSources.length === 1) {
      const ds = dataSources[0]
      const sourceId = getSourceId(ds)
      if (ds.type === 'device') return `Device: ${sourceId}${ds.property ? `.${ds.property}` : ''}`
      if (ds.type === 'metric') return `Metric: ${ds.metricId}`
      if (ds.type === 'command') return `Command: ${sourceId} → ${ds.command}`
      return 'Data Source'
    }
    return `${dataSources.length} Data Sources`
  }

  // Get available properties/metrics for a device
  const getDeviceProperties = (deviceId: string) => {
    const device = devices.find(d => d.id === deviceId)
    const deviceType = deviceTypes.find(dt => dt.device_type === device?.device_type)
    return deviceType?.metrics || []
  }

  // Get available commands for a device
  const getDeviceCommands = (deviceId: string) => {
    const device = devices.find(d => d.id === deviceId)
    const deviceType = deviceTypes.find(dt => dt.device_type === device?.device_type)
    return deviceType?.commands || []
  }

  const handleBindDevice = () => {
    setDeviceDialogOpen(true)
    setSearchQuery('')
  }

  const handleSelectDevice = (deviceId: string) => {
    const properties = getDeviceProperties(deviceId)
    const firstProperty = properties[0]?.name

    const newDataSource = {
      type: 'device',
      deviceId,
      sourceId: deviceId,
      property: firstProperty || 'status',
    } as unknown as DataSource
    onChange(newDataSource)
    setDeviceDialogOpen(false)
  }

  const handleBindMetric = () => {
    setMetricDialogOpen(true)
    setSearchQuery('')
  }

  const handleSelectMetric = (metricId: string) => {
    const newDataSource = {
      type: 'metric',
      metricId,
    } as unknown as DataSource
    onChange(newDataSource)
    setMetricDialogOpen(false)
  }

  const handleBindCommand = () => {
    setCommandDialogOpen(true)
    setSearchQuery('')
  }

  const handleSelectCommand = (deviceId: string, command: string) => {
    const newDataSource = {
      type: 'command',
      deviceId,
      sourceId: deviceId,
      command,
    } as unknown as DataSource
    onChange(newDataSource)
    setCommandDialogOpen(false)
  }

  const handleClearBinding = () => {
    onChange(undefined)
  }

  // Filter devices by search query
  const filteredDevices = devices.filter(device =>
    device.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    device.id.toLowerCase().includes(searchQuery.toLowerCase())
  )

  // Filter metrics by search query (collect all unique metrics from all devices)
  const allMetrics = Array.from(
    new Set(
      deviceTypes.flatMap(dt =>
        (dt.metrics || []).map(m => m.name)
      )
    )
  ).filter(name =>
    name.toLowerCase().includes(searchQuery.toLowerCase())
  )

  // Filter commands by search query
  const commandOptions = filteredDevices.flatMap(device => {
    const commands = getDeviceCommands(device.id)
    return commands.map(cmd => ({
      deviceId: device.id,
      deviceName: device.name,
      commandName: cmd.name,
      displayName: cmd.display_name || cmd.name,
    }))
  }).filter(option =>
    option.deviceName.toLowerCase().includes(searchQuery.toLowerCase()) ||
    option.commandName.toLowerCase().includes(searchQuery.toLowerCase())
  )

  return (
    <>
      <ConfigSection
        title="Data Source"
        bordered
        collapsible={collapsible}
        defaultCollapsed={!isBound}
      >
        {isBound ? (
          <div className="space-y-2">
            <div className="flex items-center justify-between p-2 bg-muted-50 rounded-md">
              <div className="flex items-center gap-2 text-sm">
                <Database className="h-4 w-4 text-primary" />
                <span className="text-foreground">{getDisplayText()}</span>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleClearBinding}
                className="h-7 text-xs"
              >
                {t('configRenderer.clear') || 'Clear'}
              </Button>
            </div>
          </div>
        ) : (
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground">
              Bind this component to a data source for real-time updates
            </p>
            <div className="grid grid-cols-3 gap-2">
              {allowedTypes.includes('device') && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleBindDevice}
                  className="flex-col gap-1 h-auto py-3"
                >
                  <Database className="h-4 w-4" />
                  <span className="text-xs">{t('configRenderer.device') || 'Device'}</span>
                </Button>
              )}
              {allowedTypes.includes('metric') && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleBindMetric}
                  className="flex-col gap-1 h-auto py-3"
                >
                  <Zap className="h-4 w-4" />
                  <span className="text-xs">{t('configRenderer.metric') || 'Metric'}</span>
                </Button>
              )}
              {allowedTypes.includes('command') && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleBindCommand}
                  className="flex-col gap-1 h-auto py-3"
                >
                  <Settings className="h-4 w-4" />
                  <span className="text-xs">{t('configRenderer.command') || 'Command'}</span>
                </Button>
              )}
            </div>
          </div>
        )}
      </ConfigSection>

      {/* Device Selector Dialog */}
      <Dialog open={deviceDialogOpen} onOpenChange={setDeviceDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Select Device</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search devices..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-9"
              />
            </div>
            <div className="max-h-[300px] overflow-y-auto space-y-1">
              {filteredDevices.length === 0 ? (
                <div className="text-center text-sm text-muted-foreground py-8">
                  {devices.length === 0 ? 'No devices available' : 'No devices match your search'}
                </div>
              ) : (
                filteredDevices.map((device) => (
                  <button
                    key={device.id}
                    onClick={() => handleSelectDevice(device.id)}
                    className="w-full text-left p-3 hover:bg-muted rounded-md transition-colors"
                  >
                    <div className="font-medium">{device.name}</div>
                    <div className="text-xs text-muted-foreground">{device.id}</div>
                  </button>
                ))
              )}
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Metric Selector Dialog */}
      <Dialog open={metricDialogOpen} onOpenChange={setMetricDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Select Metric</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search metrics..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-9"
              />
            </div>
            <div className="max-h-[300px] overflow-y-auto space-y-1">
              {allMetrics.length === 0 ? (
                <div className="text-center text-sm text-muted-foreground py-8">
                  No metrics available
                </div>
              ) : (
                allMetrics.map((metric) => (
                  <button
                    key={metric}
                    onClick={() => handleSelectMetric(metric)}
                    className="w-full text-left p-3 hover:bg-muted rounded-md transition-colors"
                  >
                    <div className="font-medium">{metric}</div>
                  </button>
                ))
              )}
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Command Selector Dialog */}
      <Dialog open={commandDialogOpen} onOpenChange={setCommandDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Select Command</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search commands..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-9"
              />
            </div>
            <div className="max-h-[300px] overflow-y-auto space-y-1">
              {commandOptions.length === 0 ? (
                <div className="text-center text-sm text-muted-foreground py-8">
                  {devices.length === 0 ? 'No devices available' : 'No commands match your search'}
                </div>
              ) : (
                commandOptions.map((option) => (
                  <button
                    key={`${option.deviceId}-${option.commandName}`}
                    onClick={() => handleSelectCommand(option.deviceId, option.commandName)}
                    className="w-full text-left p-3 hover:bg-muted rounded-md transition-colors"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex-1">
                        <div className="font-medium">{option.displayName}</div>
                        <div className="text-xs text-muted-foreground">{option.deviceName}</div>
                      </div>
                      <Settings className="h-4 w-4 text-muted-foreground" />
                    </div>
                  </button>
                ))
              )}
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </>
  )
}
