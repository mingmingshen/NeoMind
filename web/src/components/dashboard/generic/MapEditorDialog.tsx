/**
 * Map Editor Dialog
 *
 * Map marker position editor with:
 * - Left panel: List of bound items (devices/metrics/commands)
 * - Right panel: Large interactive map preview
 * - Click to set marker position
 * - Drag to reposition markers
 */

import { useState, useCallback, useEffect } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
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
  X,
} from 'lucide-react'
import { MapDisplay, type MapMarker } from './MapDisplay'
import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import { useStore } from '@/store'
import { useIsMobile, useSafeAreaInsets } from '@/hooks/useMobile'
import { useMobileBodyScrollLock } from '@/hooks/useBodyScrollLock'

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

// Type config factory
function getTypeConfig(t: (key: string) => string) {
  return {
    device: {
      label: t('mapDisplay.device'),
      icon: MapPin,
      color: 'text-green-500',
      bgColor: 'bg-green-500',
    },
    metric: {
      label: t('mapDisplay.metric'),
      icon: Activity,
      color: 'text-purple-500',
      bgColor: 'bg-purple-500',
    },
    command: {
      label: t('mapDisplay.command'),
      icon: Zap,
      color: 'text-blue-500',
      bgColor: 'bg-blue-500',
    },
    marker: {
      label: t('mapDisplay.marker'),
      icon: Monitor,
      color: 'text-orange-500',
      bgColor: 'bg-orange-500',
    },
  } as const
}

// Helper function to find metric value with fuzzy matching (handles nested paths like 'values.image')
function findMetricValue(currentValues: Record<string, unknown> | undefined, metricId: string): unknown {
  if (!currentValues) return undefined

  // 1. Try exact match
  if (metricId in currentValues) {
    return currentValues[metricId]
  }

  // 2. Try case-insensitive match
  const lowerMetricId = metricId.toLowerCase()
  for (const key of Object.keys(currentValues)) {
    if (key.toLowerCase() === lowerMetricId) {
      return currentValues[key]
    }
  }

  // 3. Try nested path like "values.image"
  const parts = metricId.split('.')
  let nested: any = currentValues
  for (const part of parts) {
    if (nested && typeof nested === 'object' && part in nested) {
      nested = nested[part]
    } else {
      // Try case-insensitive nested access
      let found = false
      for (const key of Object.keys(nested || {})) {
        if (key.toLowerCase() === part.toLowerCase()) {
          nested = nested[key]
          found = true
          break
        }
      }
      if (!found) {
        return undefined
      }
    }
  }
  return nested
}

export function MapEditorDialog({
  open,
  onOpenChange,
  bindings: initialBindings,
  center,
  zoom,
  tileLayer,
  onSave,
}: MapEditorDialogProps) {
  const { t } = useTranslation('dashboardComponents')
  const typeConfig = getTypeConfig(t)
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [bindings, setBindings] = useState<MapBinding[]>(initialBindings)
  const [selectedBinding, setSelectedBinding] = useState<string | null>(null)

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  // Get devices from store for reactive updates
  const devices = useStore(state => state.devices)

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setBindings(initialBindings)
      setSelectedBinding(null)
    }
  }, [open, initialBindings])

  // Convert bindings to map markers for preview
  const convertToMarkers = useCallback((): MapMarker[] => {
    const storeDevices = devices
    const getDeviceName = (deviceId: string) => {
      const device = storeDevices.find(d => d.id === deviceId || d.device_id === deviceId)
      return device?.name || device?.device_id || deviceId
    }

    const getDeviceStatus = (deviceId: string): 'online' | 'offline' | 'error' | 'warning' | undefined => {
      const device = storeDevices.find(d => d.id === deviceId || d.device_id === deviceId)
      if (!device) return undefined
      return device.online ? 'online' : 'offline'
    }

    const markers = bindings.map((binding): MapMarker => {
      const itemConfig = typeConfig[binding.icon || binding.type]
      const ds = binding.dataSource
      const sourceId = getSourceId(ds)
      const lat = binding.position === 'auto' || !binding.position
        ? center.lat
        : binding.position.lat
      const lng = binding.position === 'auto' || !binding.position
        ? center.lng
        : binding.position.lng

      const device = sourceId ? storeDevices.find(d => d.id === sourceId || d.device_id === sourceId) : undefined

      let metricValue: string | undefined = undefined
      if (binding.type === 'metric' && sourceId) {
        const metricKey = ds.metricId || ds.property
        if (device?.current_values && metricKey) {
          const rawValue = findMetricValue(device.current_values, metricKey)
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
        status: binding.type === 'device' ? getDeviceStatus(sourceId || '') : undefined,
        metricValue: binding.type === 'metric' ? (metricValue || '-') : undefined,
        deviceId: sourceId,
        sourceId,
        deviceName: sourceId ? getDeviceName(sourceId) : undefined,
        metricName: ds.metricId || ds.property,
        command: binding.type === 'command' ? ds.command : undefined,
        commandName: binding.type === 'command' ? ds.command : undefined,
      }
    })
    return markers
  }, [bindings, center, devices])

  const mapMarkers = convertToMarkers()

  // Handle removing a binding
  const handleRemoveBinding = useCallback((id: string) => {
    setBindings(bindings.filter(b => b.id !== id))
    if (selectedBinding === id) {
      setSelectedBinding(null)
    }
  }, [bindings, selectedBinding])

  // Handle updating binding position from map click
  const handleMapClick = useCallback((lat: number, lng: number) => {
    if (selectedBinding) {
      setBindings(prev => prev.map(b =>
        b.id === selectedBinding
          ? { ...b, position: { lat, lng } }
          : b
      ))
    }
  }, [selectedBinding])

  // Handle selecting a binding
  const handleSelectBinding = useCallback((id: string) => {
    setSelectedBinding(id)
  }, [])

  // Handle save
  const handleSave = useCallback(() => {
    onSave(bindings)
    onOpenChange(false)
  }, [bindings, onSave, onOpenChange])

  // Render binding item
  const renderBindingItem = (binding: MapBinding) => {
    const config = typeConfig[binding.icon || binding.type]
    const Icon = config.icon
    const isSelected = selectedBinding === binding.id

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
          config.bgColor === 'bg-blue-500' && 'bg-blue-500 text-white',
          config.bgColor === 'bg-green-500' && 'bg-green-500 text-white',
          config.bgColor === 'bg-orange-500' && 'bg-orange-500 text-white',
          config.bgColor === 'bg-purple-500' && 'bg-purple-500 text-white',
        )}>
          <Icon className="h-4 w-4 text-white" />
        </div>

        <div className="flex-1 min-w-0">
          <div className="text-sm font-medium truncate">{binding.name}</div>
          <div className="text-xs text-muted-foreground truncate">
            {config.label}
            {binding.position && binding.position !== 'auto' && (
              <span> • ({binding.position.lat.toFixed(2)}, {binding.position.lng.toFixed(2)})</span>
            )}
            {binding.position === 'auto' && <span> • {t('mapDisplay.autoPosition')}</span>}
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
            title={t('common.delete')}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </div>
    )
  }

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      open ? (
        <div className="fixed inset-0 z-[110] bg-background animate-in fade-in duration-200">
          <div className="flex h-full w-full flex-col">
            {/* Header */}
            <div
              className="flex items-center justify-between px-4 py-4 border-b shrink-0 bg-background"
              style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <MapIcon className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">
                    {t('mapDisplay.editorTitle')}
                  </h1>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={() => onOpenChange(false)} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Map Preview */}
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

            {/* Bindings List */}
            <div className="border-t bg-background shrink-0 max-h-[40vh] overflow-y-auto">
              <div className="px-4 py-2 border-b bg-muted/30 sticky top-0">
                <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                  {t('mapDisplay.boundItems')} ({bindings.length})
                </div>
              </div>
              <div className="p-2 space-y-1">
                {bindings.length === 0 ? (
                  <div className="text-center py-4 text-muted-foreground">
                    <MapIcon className="h-6 w-6 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">{t('mapDisplay.noMarkers')}</p>
                  </div>
                ) : (
                  bindings.map(renderBindingItem)
                )}
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t bg-background shrink-0"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                {t('common.cancel')}
              </Button>
              <Button onClick={handleSave}>
                <Check className="h-4 w-4 mr-1" />
                {t('common.saveChanges')}
              </Button>
            </div>
          </div>
        </div>
      ) : null,
      document.body
    )
  }

  // Desktop: Traditional dialog (following AddDeviceDialog pattern)
  return (
    <>
      {/* Backdrop */}
      {open && (
        <div
          className="fixed inset-0 z-[109] bg-black/80 backdrop-blur-sm animate-in fade-in duration-200"
          onClick={() => onOpenChange(false)}
        />
      )}

      {/* Dialog */}
      {open && (
        <div
          className={cn(
            'fixed left-1/2 top-1/2 z-[110]',
            'grid w-full gap-0',
            'bg-background shadow-lg',
            'duration-200',
            'animate-in fade-in zoom-in-95 slide-in-from-left-1/2 slide-in-from-top-[48%]',
            'rounded-lg sm:rounded-xl',
            'max-h-[96vh] h-[90vh]',
            'flex flex-col',
            'max-w-5xl w-[90vw]',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex items-center gap-2 flex-1 min-w-0">
              <MapIcon className="h-5 w-5 text-primary" />
              <h2 className="text-lg font-semibold leading-none truncate">
                {t('mapDisplay.editorTitle')}
              </h2>
            </div>
            <button
              onClick={() => onOpenChange(false)}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 flex overflow-hidden min-h-0">
            {/* Left Panel - Bindings List */}
            <div className="w-72 border-r bg-muted/20 flex flex-col shrink-0">
              <div className="px-4 py-2 border-b bg-muted/30 shrink-0">
                <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                  {t('mapDisplay.boundItems')} ({bindings.length})
                </div>
              </div>

              <div className="flex-1 overflow-y-auto p-2 space-y-1">
                {bindings.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">
                    <MapIcon className="h-8 w-8 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">{t('mapDisplay.noMarkers')}</p>
                    <p className="text-xs mt-1">{t('mapDisplay.addDataSourceHint')}</p>
                  </div>
                ) : (
                  bindings.map(renderBindingItem)
                )}
              </div>
            </div>

            {/* Right Panel - Map Preview */}
            <div className="flex-1 relative bg-muted/30 min-w-0">
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

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-6 py-4 border-t shrink-0 bg-muted/30">
            <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
              {t('common.cancel')}
            </Button>
            <Button size="sm" onClick={handleSave}>
              <Check className="h-4 w-4 mr-1" />
              {t('common.saveChanges')}
            </Button>
          </div>
        </div>
      )}
    </>
  )
}
