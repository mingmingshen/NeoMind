/**
 * Map Editor Dialog
 *
 * Map marker position editor with:
 * - Left panel: List of bound items (devices/metrics/commands)
 * - Right panel: Large interactive map preview
 * - Click to set marker position
 * - Drag to reposition markers
 */

import { useState, useCallback, useRef, useEffect } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { cn } from '@/lib/utils'
import {
  Trash2,
  GripVertical,
  Check,
  Activity,
  Zap,
  Map as MapIcon,
  MapPin,
  Monitor,
} from 'lucide-react'
import { MapDisplay, type MapMarker } from './MapDisplay'
import type { DataSource } from '@/types/dashboard'
import { useStore } from '@/store'

export type MapBindingType = 'device' | 'metric' | 'command' | 'marker'

export interface MapBinding {
  id: string
  type: MapBindingType
  name: string
  dataSource: DataSource
  position?: { lat: number; lng: number } | 'auto'
  icon?: MapBindingType
}

interface MapEditorDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  bindings: MapBinding[]
  center: { lat: number; lng: number }
  zoom: number
  tileLayer: string
  onSave: (bindings: MapBinding[]) => void
}

const TYPE_CONFIG = {
  device: {
    label: '设备',
    icon: MapPin,
    color: 'text-green-500',
    bgColor: 'bg-green-500',
  },
  metric: {
    label: '指标',
    icon: Activity,
    color: 'text-purple-500',
    bgColor: 'bg-purple-500',
  },
  command: {
    label: '指令',
    icon: Zap,
    color: 'text-blue-500',
    bgColor: 'bg-blue-500',
  },
  marker: {
    label: '位置标记',
    icon: Monitor,
    color: 'text-orange-500',
    bgColor: 'bg-orange-500',
  },
} as const

export function MapEditorDialog({
  open,
  onOpenChange,
  bindings: initialBindings,
  center,
  zoom,
  tileLayer,
  onSave,
}: MapEditorDialogProps) {
  const [bindings, setBindings] = useState<MapBinding[]>(initialBindings)
  const [selectedBinding, setSelectedBinding] = useState<string | null>(null)
  const [addingMode, setAddingMode] = useState(false)

  // Get devices from store for reactive updates
  const devices = useStore(state => state.devices)

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setBindings(initialBindings)
      setSelectedBinding(null)
      setAddingMode(false)
    }
  }, [open, initialBindings])

  // Convert bindings to map markers for preview
  const convertToMarkers = useCallback((): MapMarker[] => {
    console.log('convertToMarkers called, bindings:', bindings)

    // Get devices from store for metric values and names
    const storeDevices = devices
    const getDeviceName = (deviceId: string) => {
      const device = storeDevices.find(d => d.id === deviceId || d.device_id === deviceId)
      return device?.name || device?.device_id || deviceId
    }

    // Helper to get device status
    const getDeviceStatus = (deviceId: string): 'online' | 'offline' | 'error' | 'warning' | undefined => {
      const device = storeDevices.find(d => d.id === deviceId || d.device_id === deviceId)
      if (!device) return undefined
      return device.online ? 'online' : 'offline'
    }

    const markers = bindings.map((binding): MapMarker => {
      const typeConfig = TYPE_CONFIG[binding.icon || binding.type]
      const ds = binding.dataSource as any
      const lat = binding.position === 'auto' || !binding.position
        ? center.lat
        : binding.position.lat
      const lng = binding.position === 'auto' || !binding.position
        ? center.lng
        : binding.position.lng
      console.log(`Binding ${binding.name}: lat=${lat}, lng=${lng}, position=`, binding.position)

      // Get the device for this binding
      const device = ds?.deviceId ? storeDevices.find(d => d.id === ds.deviceId || d.device_id === ds.deviceId) : undefined

      // Get metric value for metric bindings
      let metricValue: string | undefined = undefined
      if (binding.type === 'metric' && ds?.deviceId) {
        const metricKey = ds.metricId || ds.property
        if (device?.current_values && metricKey) {
          const rawValue = device.current_values[metricKey]
          if (rawValue !== undefined && rawValue !== null) {
            metricValue = typeof rawValue === 'number'
              ? rawValue.toFixed(1)
              : String(rawValue)
          }
        }
      }

      return {
        id: binding.id,
        latitude: lat,
        longitude: lng,
        label: binding.name,
        markerType: binding.icon || binding.type,
        // Use actual device status
        status: binding.type === 'device' ? getDeviceStatus(ds.deviceId) : undefined,
        metricValue: binding.type === 'metric' ? (metricValue || '--') : undefined,
        deviceId: ds?.deviceId,
        deviceName: ds?.deviceId ? getDeviceName(ds.deviceId) : undefined,
        metricName: ds?.metricId || ds?.property,
        command: binding.type === 'command' ? ds?.command : undefined,
        commandName: binding.type === 'command' ? ds?.command : undefined,
      }
    })
    console.log('Generated markers:', markers)
    return markers
  }, [bindings, center, devices])

  const mapMarkers = convertToMarkers()
  console.log('mapMarkers prop:', mapMarkers)

  // Handle removing a binding
  const handleRemoveBinding = useCallback((id: string) => {
    setBindings(bindings.filter(b => b.id !== id))
    if (selectedBinding === id) {
      setSelectedBinding(null)
    }
  }, [bindings, selectedBinding])

  // Handle updating binding position from map click
  const handleMapClick = useCallback((lat: number, lng: number) => {
    console.log('MapEditor handleMapClick:', { lat, lng, selectedBinding })
    if (selectedBinding) {
      setBindings(prev => prev.map(b =>
        b.id === selectedBinding
          ? { ...b, position: { lat, lng } }
          : b
      ))
    }
  }, [selectedBinding])

  // Handle selecting a binding (auto-enter positioning mode)
  const handleSelectBinding = useCallback((id: string) => {
    setSelectedBinding(id)
    // Auto-enter adding mode when selecting
    setAddingMode(true)
  }, [])

  // Handle save
  const handleSave = useCallback(() => {
    onSave(bindings)
    onOpenChange(false)
  }, [bindings, onSave, onOpenChange])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-6xl h-[80vh] p-0 gap-0 flex flex-col">
        <DialogHeader className="px-6 py-4 border-b">
          <DialogTitle className="text-lg">编辑地图标记</DialogTitle>
        </DialogHeader>

        <div className="flex-1 flex overflow-hidden">
          {/* Left Panel - Bindings List */}
          <div className="w-80 border-r bg-muted/20 flex flex-col">
            <div className="p-3 border-b bg-muted/30">
              <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                已绑定项目 ({bindings.length})
              </div>
            </div>

            <div className="flex-1 overflow-y-auto p-2 space-y-1">
              {bindings.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <MapIcon className="h-8 w-8 mx-auto mb-2 opacity-50" />
                  <p className="text-sm">暂无标记</p>
                  <p className="text-xs mt-1">请在配置面板中添加数据源</p>
                </div>
              ) : (
                bindings.map((binding) => {
                  const config = TYPE_CONFIG[binding.icon || binding.type]
                  const Icon = config.icon
                  const isSelected = selectedBinding === binding.id
                  const isAdding = addingMode && isSelected

                  return (
                    <div
                      key={binding.id}
                      className={cn(
                        'group flex items-center gap-2 p-2 rounded-lg border transition-all cursor-pointer',
                        isSelected
                          ? 'border-primary bg-primary/5'
                          : 'border-border hover:border-primary/50 hover:bg-muted/50'
                      )}
                      onClick={() => handleSelectBinding(binding.id)}
                    >
                      <GripVertical className="h-4 w-4 text-muted-foreground cursor-grab" />

                      <div className={cn(
                        'w-8 h-8 rounded-full flex items-center justify-center',
                        config.bgColor === 'bg-blue-500' && 'bg-blue-500/20 text-blue-600',
                        config.bgColor === 'bg-green-500' && 'bg-green-500/20 text-green-600',
                        config.bgColor === 'bg-orange-500' && 'bg-orange-500/20 text-orange-600',
                        config.bgColor === 'bg-purple-500' && 'bg-purple-500/20 text-purple-600',
                      )}>
                        <Icon className={cn('h-4 w-4', config.color)} />
                      </div>

                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium truncate">{binding.name}</div>
                        <div className="text-xs text-muted-foreground truncate">
                          {config.label}
                          {binding.position && binding.position !== 'auto' && (
                            <span> • ({binding.position.lat.toFixed(2)}, {binding.position.lng.toFixed(2)})</span>
                          )}
                          {binding.position === 'auto' && <span> • 自动定位</span>}
                        </div>
                      </div>

                      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-6 w-6 text-destructive"
                          onClick={(e) => {
                            e.stopPropagation()
                            handleRemoveBinding(binding.id)
                          }}
                          title="删除"
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    </div>
                  )
                })
              )}
            </div>
          </div>

          {/* Right Panel - Map Preview */}
          <div className="flex-1 relative bg-muted/30">
            <div className="absolute inset-0">
              <MapDisplay
                center={center}
                zoom={zoom}
                tileLayer={tileLayer}
                markers={mapMarkers}
                showControls={true}
                showFullscreen={false}
                interactive={true}
                onMapClick={handleMapClick}
                className="w-full h-full"
              />
            </div>

          </div>
        </div>

        <DialogFooter className="px-6 py-4 border-t bg-muted/20">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={handleSave}>
            <Check className="h-4 w-4 mr-1" />
            保存更改
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
