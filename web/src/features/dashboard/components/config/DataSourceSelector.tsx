/**
 * DataSourceSelector — picks data source type and entity
 */

import { useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Input } from '@/components/ui/input'
import { useDevices, useExtensions } from '@/lib/react-query-hooks'
import { useStore } from '@/store'
import type { DataSource, DataSourceType } from '../../types'

interface DataSourceSelectorProps {
  value: DataSource | undefined
  onChange: (source: DataSource) => void
}

const SOURCE_TYPES: { value: DataSourceType; label: string }[] = [
  { value: 'device', label: 'Device' },
  { value: 'telemetry', label: 'Telemetry' },
  { value: 'metric', label: 'Metric' },
  { value: 'command', label: 'Command' },
  { value: 'device-info', label: 'Device Info' },
  { value: 'extension', label: 'Extension' },
  { value: 'extension-metric', label: 'Extension Metric' },
  { value: 'extension-command', label: 'Extension Command' },
  { value: 'system', label: 'System' },
  { value: 'transform', label: 'Transform' },
  { value: 'ai-metric', label: 'AI Metric' },
  { value: 'agent', label: 'Agent' },
]

export function DataSourceSelector({ value, onChange }: DataSourceSelectorProps) {
  const { t } = useTranslation()
  const { data: devicesData } = useDevices()
  const { data: extensionsData } = useExtensions()
  const deviceTypeList = useStore((s) => s.deviceTypes) ?? []

  // API returns { devices: [...] } wrapped in { success, data }
  const deviceList = (Array.isArray(devicesData) ? devicesData : (devicesData as any)?.devices) ?? []
  const extensionList = (Array.isArray(extensionsData) ? extensionsData : (extensionsData as any)?.extensions) ?? []

  // Build available metrics for the currently selected device
  const availableMetrics = useMemo(() => {
    if (!value?.sourceId) return []
    const device = deviceList.find((d: any) => (d.id || d.device_id) === value.sourceId)
    if (!device) return []

    // Look up device type template for metric definitions
    const dt = deviceTypeList.find((t: any) => t.device_type === device.device_type)
    if (dt?.metrics && dt.metrics.length > 0) {
      return dt.metrics.map((m: any) => ({ name: m.name, label: m.display_name || m.name, unit: m.unit || '' }))
    }

    // Fallback: use current_values keys
    if (device.current_values && typeof device.current_values === 'object') {
      return Object.keys(device.current_values).map(key => ({ name: key, label: key, unit: '' }))
    }

    return []
  }, [value?.sourceId, deviceList, deviceTypeList])

  const handleTypeChange = useCallback((type: string) => {
    onChange({ ...value, type: type as DataSourceType } as DataSource)
  }, [value, onChange])

  const handleDeviceChange = useCallback((deviceId: string) => {
    onChange({ ...value, sourceId: deviceId } as DataSource)
  }, [value, onChange])

  const handleExtensionChange = useCallback((extId: string) => {
    onChange({ ...value, extensionId: extId } as DataSource)
  }, [value, onChange])

  const handlePropertyChange = useCallback((property: string) => {
    onChange({ ...value, property } as DataSource)
  }, [value, onChange])

  const isDeviceType = value?.type === 'device' || value?.type === 'telemetry' || value?.type === 'metric' || value?.type === 'command' || value?.type === 'device-info'
  const isExtensionType = value?.type === 'extension' || value?.type === 'extension-metric' || value?.type === 'extension-command'

  return (
    <div className="space-y-3">
      {/* Source Type */}
      <div className="space-y-1.5">
        <Label className="text-xs">{t('dashboard.dataSource', 'Data Source Type')}</Label>
        <Select value={value?.type ?? ''} onValueChange={handleTypeChange}>
          <SelectTrigger>
            <SelectValue placeholder="Select source type" />
          </SelectTrigger>
          <SelectContent>
            {SOURCE_TYPES.map(st => (
              <SelectItem key={st.value} value={st.value}>{st.label}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Device picker */}
      {isDeviceType && (
        <div className="space-y-1.5">
          <Label className="text-xs">{t('dashboard.device', 'Device')}</Label>
          <Select value={value?.sourceId ?? ''} onValueChange={handleDeviceChange}>
            <SelectTrigger>
              <SelectValue placeholder="Select device" />
            </SelectTrigger>
            <SelectContent>
              {deviceList.map((d: any) => (
                <SelectItem key={d.id || d.device_id} value={d.id || d.device_id}>
                  {d.name || d.id || d.device_id}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <div className="space-y-1.5">
            <Label className="text-xs">{t('dashboard.property', 'Property/Metric')}</Label>
            {availableMetrics.length > 0 ? (
              <Select
                value={value?.property ?? value?.metricId ?? ''}
                onValueChange={handlePropertyChange}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select metric" />
                </SelectTrigger>
                <SelectContent>
                  {availableMetrics.map((m: { name: string; label: string; unit: string }) => (
                    <SelectItem key={m.name} value={m.name}>
                      {m.label}{m.unit ? ` (${m.unit})` : ''}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : (
              <Input
                value={value?.property ?? value?.metricId ?? ''}
                onChange={(e) => handlePropertyChange(e.target.value)}
                placeholder="e.g., temperature, humidity"
              />
            )}
          </div>
        </div>
      )}

      {/* Extension picker */}
      {isExtensionType && (
        <div className="space-y-1.5">
          <Label className="text-xs">{t('dashboard.extension', 'Extension')}</Label>
          <Select value={value?.extensionId ?? ''} onValueChange={handleExtensionChange}>
            <SelectTrigger>
              <SelectValue placeholder="Select extension" />
            </SelectTrigger>
            <SelectContent>
              {extensionList.map((e: any) => (
                <SelectItem key={e.id || e.extension_id} value={e.id || e.extension_id}>
                  {e.name || e.id || e.extension_id}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <div className="space-y-1.5">
            <Label className="text-xs">Metric</Label>
            <Input
              value={value?.extensionMetric ?? ''}
              onChange={(e) => onChange({ ...value, extensionMetric: e.target.value } as DataSource)}
              placeholder="Metric name"
            />
          </div>
        </div>
      )}

      {/* System metric */}
      {value?.type === 'system' && (
        <div className="space-y-1.5">
          <Label className="text-xs">System Metric</Label>
          <Select
            value={value?.systemMetric ?? ''}
            onValueChange={(v) => onChange({ ...value, systemMetric: v as any } as DataSource)}
          >
            <SelectTrigger>
              <SelectValue placeholder="Select metric" />
            </SelectTrigger>
            <SelectContent>
              {['uptime', 'cpu_count', 'total_memory', 'used_memory', 'free_memory', 'memory_percent', 'platform', 'arch'].map(m => (
                <SelectItem key={m} value={m}>{m}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )}
    </div>
  )
}
