/**
 * Map Display Component
 *
 * Interactive map component for visualizing devices, metrics, and commands.
 * Supports device markers, metric overlays, and command interactions.
 */

import { useState, useCallback, useRef, useEffect } from 'react'
import { Skeleton } from '@/components/ui/skeleton'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { dashboardCardBase, dashboardComponentSize } from '@/design-system/tokens/size'
import { useDataSource } from '@/hooks/useDataSource'
import {
  MapPin,
  Navigation,
  ZoomIn,
  ZoomOut,
  Layers,
  RefreshCw,
  Map as MapIcon,
  Maximize2,
  Minimize2,
} from 'lucide-react'
import type { DataSource } from '@/types/dashboard'
import { EmptyState } from '../shared'

// ============================================================================
// Types
// ============================================================================

export interface MapMarker {
  id: string
  deviceId?: string
  latitude: number
  longitude: number
  label?: string
  value?: number
  status?: 'online' | 'offline' | 'error' | 'warning'
  onClick?: () => void
  metricValue?: string
  command?: string
}

export interface MapLayer {
  id: string
  name: string
  type: 'devices' | 'metrics' | 'commands' | 'custom'
  visible: boolean
  markers?: MapMarker[]
}

export interface MapDisplayProps {
  dataSource?: DataSource
  markers?: MapMarker[]
  layers?: MapLayer[]

  // Map options
  center?: { lat: number; lng: number }
  zoom?: number
  minZoom?: number
  maxZoom?: number

  // Display options
  size?: 'sm' | 'md' | 'lg'
  showControls?: boolean
  showLayers?: boolean
  showFullscreen?: boolean
  interactive?: boolean

  // Styling
  tileLayer?: string
  markerColor?: string
  className?: string
}

// Default tile layers
const TILE_LAYERS = {
  osm: 'https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png',
  satellite: 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
  dark: 'https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png',
  terrain: 'https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png',
}

// ============================================================================
// Marker Component
// ============================================================================

interface MapMarkerDotProps {
  marker: MapMarker
  onClick: () => void
}

function MapMarkerDot({ marker, onClick }: MapMarkerDotProps) {
  const getStatusColor = (status?: string) => {
    switch (status) {
      case 'online': return 'bg-green-500'
      case 'offline': return 'bg-gray-400'
      case 'error': return 'bg-red-500'
      case 'warning': return 'bg-yellow-500'
      default: return 'bg-blue-500'
    }
  }

  return (
    <button
      onClick={onClick}
      className="absolute transform -translate-x-1/2 -translate-y-1/2 group"
      style={{ left: '50%', top: '50%' }}
    >
      {/* Pulsing effect for online devices */}
      {marker.status === 'online' && (
        <span className="absolute inline-flex h-full w-full animate-ping opacity-75 rounded-full bg-green-400" />
      )}

      {/* Marker dot */}
      <span
        className={cn(
          'relative inline-flex rounded-full items-center justify-center',
          getStatusColor(marker.status),
          'h-4 w-4 border-2 border-background shadow-sm',
          'group-hover:scale-125 transition-transform'
        )}
      />

      {/* Label tooltip */}
      {marker.label && (
        <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1 px-2 py-1 bg-background/95 rounded shadow-lg text-xs whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
          <div className="font-medium">{marker.label}</div>
          {marker.metricValue && (
            <div className="text-muted-foreground">{marker.metricValue}</div>
          )}
        </div>
      )}
    </button>
  )
}

// ============================================================================
// Simple SVG Map (embedded solution without external deps)
// ============================================================================

interface SimpleSvgMapProps {
  center: { lat: number; lng: number }
  zoom: number
  width: number
  height: number
  markers: MapMarker[]
  onMarkerClick: (marker: MapMarker) => void
  onZoomIn: () => void
  onZoomOut: () => void
  interactive: boolean
}

function SimpleSvgMap({
  center,
  zoom,
  width,
  height,
  markers,
  onMarkerClick,
  onZoomIn,
  onZoomOut,
  interactive,
}: SimpleSvgMapProps) {
  const [dragState, setDragState] = useState<{ isDragging: boolean; startX: number; startY: number; offsetX: number; offsetY: number }>({
    isDragging: false,
    startX: 0,
    startY: 0,
    offsetX: 0,
    offsetY: 0,
  })

  // Convert lat/lng to map coordinates
  const latLngToMap = (lat: number, lng: number) => {
    // Simple equirectangular projection
    const scale = Math.pow(2, zoom) * 100
    const x = ((lng - center.lng) * scale) + width / 2 + dragState.offsetX
    const y = ((center.lat - lat) * scale) + height / 2 + dragState.offsetY
    return { x, y }
  }

  const handleMouseDown = (e: React.MouseEvent) => {
    if (!interactive) return
    setDragState(prev => ({
      ...prev,
      isDragging: true,
      startX: e.clientX - prev.offsetX,
      startY: e.clientY - prev.offsetY,
    }))
  }

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!dragState.isDragging || !interactive) return
    setDragState(prev => ({
      ...prev,
      offsetX: e.clientX - prev.startX,
      offsetY: e.clientY - prev.startY,
    }))
  }

  const handleMouseUp = () => {
    setDragState(prev => ({ ...prev, isDragging: false }))
  }

  // Background grid pattern
  const gridSize = 50
  const gridWidth = width * 3
  const gridHeight = height * 3

  return (
    <div
      className="relative w-full h-full overflow-hidden bg-muted/10 cursor-move"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
    >
      <svg
        width={width}
        height={height}
        className="absolute inset-0"
        style={{ pointerEvents: 'none' }}
      >
        <defs>
          <pattern
            id="grid"
            width={gridSize}
            height={gridSize}
            patternUnits="userSpaceOnUse"
          >
            <path
              d={`M ${gridSize} 0 L 0 0 0 ${gridSize}`}
              fill="none"
              stroke="hsl(var(--border))"
              strokeWidth="0.5"
              opacity="0.3"
            />
          </pattern>
        </defs>
        <rect
          width={gridWidth}
          height={gridHeight}
          x={-width + dragState.offsetX}
          y={-height + dragState.offsetY}
          fill="url(#grid)"
        />
      </svg>

      {/* Render markers as absolute positioned elements */}
      <div className="absolute inset-0" style={{ pointerEvents: 'none' }}>
        {markers.map((marker) => {
          const pos = latLngToMap(marker.latitude, marker.longitude)
          const isInBounds = pos.x >= 0 && pos.x <= width && pos.y >= 0 && pos.y <= height

          if (!isInBounds) return null

          return (
            <div
              key={marker.id}
              className="absolute"
              style={{
                left: pos.x,
                top: pos.y,
                pointerEvents: interactive ? 'auto' : 'none',
              }}
            >
              <MapMarkerDot
                marker={marker}
                onClick={() => onMarkerClick(marker)}
              />
            </div>
          )
        })}
      </div>

      {/* Zoom controls */}
      {interactive && (
        <div className="absolute top-2 right-2 flex flex-col gap-1" style={{ pointerEvents: 'auto' }}>
          <Button
            variant="secondary"
            size="icon"
            className="h-7 w-7 bg-background/80 backdrop-blur"
            onClick={onZoomIn}
          >
            <ZoomIn className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="secondary"
            size="icon"
            className="h-7 w-7 bg-background/80 backdrop-blur"
            onClick={onZoomOut}
          >
            <ZoomOut className="h-3.5 w-3.5" />
          </Button>
        </div>
      )}

      {/* Center point indicator */}
      <div className="absolute top-1/2 left-1/2 w-3 h-3 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-primary/30 bg-primary/10" />
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function MapDisplay({
  dataSource,
  markers: propMarkers = [],
  layers: propLayers = [],
  center = { lat: 39.9042, lng: 116.4074 }, // Default: Beijing
  zoom = 10,
  minZoom = 2,
  maxZoom = 18,
  size = 'md',
  showControls = true,
  showLayers = true,
  showFullscreen = true,
  interactive = true,
  tileLayer = 'osm',
  markerColor,
  className,
}: MapDisplayProps) {
  const { data, loading, error } = useDataSource<MapMarker[]>(dataSource, {
    fallback: propMarkers,
  })

  const markers = error ? propMarkers : (data ?? propMarkers)
  const [currentZoom, setCurrentZoom] = useState(zoom)
  const [currentCenter, setCurrentCenter] = useState(center)
  const [isFullscreen, setIsFullscreen] = useState(false)
  const [selectedMarker, setSelectedMarker] = useState<MapMarker | null>(null)
  const [showLayerPanel, setShowLayerPanel] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)
  const [containerSize, setContainerSize] = useState({ width: 400, height: 300 })

  // Sync internal state when props change (e.g., after configuration update)
  useEffect(() => {
    setCurrentZoom(zoom)
  }, [zoom])

  useEffect(() => {
    setCurrentCenter(center)
  }, [center])

  // Track container size
  useEffect(() => {
    const updateSize = () => {
      if (containerRef.current) {
        setContainerSize({
          width: containerRef.current.offsetWidth,
          height: containerRef.current.offsetHeight,
        })
      }
    }

    updateSize()
    const observer = new ResizeObserver(updateSize)
    if (containerRef.current) {
      observer.observe(containerRef.current)
    }

    return () => observer.disconnect()
  }, [])

  const sizeConfig = dashboardComponentSize[size]

  // Default layers
  const defaultLayers: MapLayer[] = [
    {
      id: 'devices',
      name: 'Devices',
      type: 'devices',
      visible: true,
      markers: markers.filter(m => m.deviceId),
    },
    {
      id: 'metrics',
      name: 'Metrics',
      type: 'metrics',
      visible: true,
      markers: markers.filter(m => m.metricValue),
    },
  ]

  const layers = propLayers.length > 0 ? propLayers : defaultLayers

  // Get visible markers
  const visibleMarkers = markers.filter(marker => {
    return layers.some(layer => layer.visible && layer.markers?.includes(marker))
  })

  const handleZoomIn = useCallback(() => {
    setCurrentZoom(prev => Math.min(prev + 1, maxZoom))
  }, [maxZoom])

  const handleZoomOut = useCallback(() => {
    setCurrentZoom(prev => Math.max(prev - 1, minZoom))
  }, [minZoom])

  const handleMarkerClick = useCallback((marker: MapMarker) => {
    if (!interactive) return
    setSelectedMarker(marker)

    // Execute marker's click handler
    if (marker.onClick) {
      marker.onClick()
    }
  }, [interactive])

  const toggleLayer = useCallback((layerId: string) => {
    // This would update layer visibility - for now just a placeholder
    // In a real implementation, you'd update state or parent component
  }, [])

  // Loading state
  if (loading) {
    return (
      <div className={cn(dashboardCardBase, 'flex items-center justify-center', sizeConfig.padding, className)}>
        <Skeleton className={cn('w-full h-full rounded-lg')} />
      </div>
    )
  }

  // Empty state
  if (markers.length === 0) {
    return (
      <EmptyState
        size={size}
        className={className}
        icon={<MapIcon />}
        message="No Markers"
        subMessage="Add markers to display on the map"
      />
    )
  }

  const mapContent = (
    <div className={cn(dashboardCardBase, 'relative overflow-hidden flex flex-col', className)}>
      {/* Header */}
      {showControls && (
        <div className="flex items-center justify-between px-3 py-2 border-b bg-muted/20">
          <div className="flex items-center gap-2">
            <MapPin className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Map</span>
            <span className="text-xs text-muted-foreground">
              ({currentCenter.lat.toFixed(2)}, {currentCenter.lng.toFixed(2)})
            </span>
          </div>
          <div className="flex items-center gap-1">
            {showLayers && (
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={() => setShowLayerPanel(!showLayerPanel)}
              >
                <Layers className="h-3.5 w-3.5" />
              </Button>
            )}
            {showFullscreen && (
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={() => setIsFullscreen(!isFullscreen)}
              >
                {isFullscreen ? <Minimize2 className="h-3.5 w-3.5" /> : <Maximize2 className="h-3.5 w-3.5" />}
              </Button>
            )}
          </div>
        </div>
      )}

      {/* Map container */}
      <div
        ref={containerRef}
        className={cn(
          'flex-1 relative',
          isFullscreen ? 'fixed inset-0 z-50' : 'min-h-[200px]'
        )}
      >
        <SimpleSvgMap
          center={currentCenter}
          zoom={currentZoom}
          width={containerSize.width}
          height={containerSize.height}
          markers={visibleMarkers}
          onMarkerClick={handleMarkerClick}
          onZoomIn={handleZoomIn}
          onZoomOut={handleZoomOut}
          interactive={interactive}
        />

        {/* Layer panel */}
        {showLayerPanel && showLayers && (
          <div className="absolute top-2 left-2 bg-background/95 backdrop-blur rounded-lg shadow-lg border p-2 min-w-[150px]">
            <div className="text-xs font-medium mb-2">Layers</div>
            <div className="space-y-1">
              {layers.map((layer) => (
                <label
                  key={layer.id}
                  className="flex items-center gap-2 text-xs cursor-pointer hover:bg-muted/50 px-2 py-1 rounded"
                >
                  <input
                    type="checkbox"
                    checked={layer.visible}
                    onChange={() => toggleLayer(layer.id)}
                    className="h-3 w-3"
                  />
                  <span>{layer.name}</span>
                  {layer.markers && (
                    <span className="text-muted-foreground">({layer.markers.length})</span>
                  )}
                </label>
              ))}
            </div>
          </div>
        )}

        {/* Selected marker info */}
        {selectedMarker && (
          <div className="absolute bottom-2 left-2 right-2 bg-background/95 backdrop-blur rounded-lg shadow-lg border p-3">
            <div className="flex items-start justify-between">
              <div>
                <div className="font-medium text-sm">{selectedMarker.label || 'Marker'}</div>
                <div className="text-xs text-muted-foreground">
                  {selectedMarker.latitude.toFixed(4)}, {selectedMarker.longitude.toFixed(4)}
                </div>
                {selectedMarker.metricValue && (
                  <div className="text-sm mt-1">{selectedMarker.metricValue}</div>
                )}
                {selectedMarker.deviceId && (
                  <div className="text-xs text-muted-foreground mt-1">Device: {selectedMarker.deviceId}</div>
                )}
              </div>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6"
                onClick={() => setSelectedMarker(null)}
              >
                ×
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  )

  return mapContent
}
