/**
 * Center Picker Dialog
 *
 * A dialog for visually selecting the map center point by clicking on the map.
 * Provides an intuitive way to set map center coordinates.
 */

import { useState, useCallback, useEffect } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Check, MapPin, Crosshair, X } from 'lucide-react'
import { MapDisplay } from './MapDisplay'
import { cn } from '@/lib/utils'
import { useIsMobile, useSafeAreaInsets } from '@/hooks/useMobile'
import { useMobileBodyScrollLock } from '@/hooks/useBodyScrollLock'

interface CenterPickerDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  center: { lat: number; lng: number }
  zoom: number
  tileLayer: string
  onSave: (center: { lat: number; lng: number }) => void
}

export function CenterPickerDialog({
  open,
  onOpenChange,
  center: initialCenter,
  zoom,
  tileLayer,
  onSave,
}: CenterPickerDialogProps) {
  const { t } = useTranslation('dashboardComponents')
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()
  const [selectedCenter, setSelectedCenter] = useState(initialCenter)
  const [hasSelected, setHasSelected] = useState(false)

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setSelectedCenter(initialCenter)
      setHasSelected(false)
    }
  }, [open, initialCenter])

  // Handle map click to set new center
  const handleMapClick = useCallback((lat: number, lng: number) => {
    setSelectedCenter({ lat, lng })
    setHasSelected(true)
  }, [])

  // Handle save
  const handleSave = useCallback(() => {
    onSave(selectedCenter)
    onOpenChange(false)
  }, [selectedCenter, onSave, onOpenChange])

  // Create a marker to show the selected position
  const previewMarker = {
    id: 'center-preview',
    latitude: selectedCenter.lat,
    longitude: selectedCenter.lng,
    label: t('mapDisplay.selectedCenter', '选中的中心点'),
    markerType: 'marker' as const,
  }

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      open ? (
        <div className="fixed inset-0 z-[60] bg-background animate-in fade-in duration-200">
          <div className="flex h-full w-full flex-col">
            {/* Header */}
            <div
              className="flex items-center justify-between px-4 py-4 border-b shrink-0 bg-background"
              style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <Crosshair className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">
                    {t('mapDisplay.selectCenter', '选择地图中心点')}
                  </h1>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={() => onOpenChange(false)} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Instructions */}
            <div className="px-4 py-3 bg-muted-30 border-b text-sm text-muted-foreground shrink-0">
              <div className="flex items-center gap-2">
                <MapPin className="h-4 w-4 text-primary" />
                <span>{t('mapDisplay.clickToSelectCenter', '点击地图选择中心点位置')}</span>
              </div>
            </div>

            {/* Map Preview */}
            <div className="flex-1 relative bg-muted-30">
              <div className="absolute inset-0">
                <MapDisplay
                  center={selectedCenter}
                  zoom={zoom}
                  tileLayer={tileLayer}
                  markers={[previewMarker]}
                  showControls={true}
                  showFullscreen={false}
                  interactive={true}
                  onMapClick={handleMapClick}
                  className="w-full h-full"
                />
              </div>

              {/* Crosshair overlay */}
              <div className="absolute inset-0 pointer-events-none flex items-center justify-center">
                <div className={cn(
                  "w-8 h-8 relative transition-all duration-200",
                  hasSelected && "scale-110"
                )}>
                  <div className="absolute top-1/2 left-0 right-0 h-0.5 bg-muted0 -translate-y-1/2" />
                  <div className="absolute left-1/2 top-0 bottom-0 w-0.5 bg-muted0 -translate-x-1/2" />
                </div>
              </div>
            </div>

            {/* Selected coordinates display */}
            <div className="px-4 py-3 border-t bg-muted-20 shrink-0">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4 text-sm">
                  <div className="flex items-center gap-1">
                    <span className="text-muted-foreground">{t('visualDashboard.latitude')}:</span>
                    <span className="font-mono">{selectedCenter.lat.toFixed(4)}</span>
                  </div>
                  <div className="flex items-center gap-1">
                    <span className="text-muted-foreground">{t('visualDashboard.longitude')}:</span>
                    <span className="font-mono">{selectedCenter.lng.toFixed(4)}</span>
                  </div>
                </div>
                {hasSelected && (
                  <div className="text-sm text-success dark:text-success">
                    ✓ {t('mapDisplay.centerSelected', '已选择')}
                  </div>
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
              <Button onClick={handleSave} disabled={!hasSelected}>
                <Check className="h-4 w-4 mr-1" />
                {t('common.confirm')}
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
          className="fixed inset-0 z-[55] bg-black/80 backdrop-blur-sm animate-in fade-in duration-200"
          onClick={() => onOpenChange(false)}
        />
      )}

      {/* Dialog */}
      {open && (
        <div
          className={cn(
            'fixed left-1/2 top-1/2 z-[60]',
            'grid w-full gap-0',
            'bg-background shadow-lg',
            'duration-200',
            'animate-in fade-in zoom-in-95 slide-in-from-left-1/2 slide-in-from-top-[48%]',
            'rounded-lg sm:rounded-xl',
            'max-h-[96vh] h-[85vh]',
            'flex flex-col',
            'max-w-4xl w-[90vw]',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex items-center gap-2 flex-1 min-w-0">
              <Crosshair className="h-5 w-5 text-primary" />
              <h2 className="text-lg font-semibold leading-none truncate">
                {t('mapDisplay.selectCenter', '选择地图中心点')}
              </h2>
            </div>
            <button
              onClick={() => onOpenChange(false)}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Instructions */}
          <div className="px-6 py-2 bg-muted-30 border-b text-sm text-muted-foreground shrink-0">
            <div className="flex items-center gap-2">
              <MapPin className="h-4 w-4 text-primary" />
              <span>{t('mapDisplay.clickToSelectCenter', '点击地图选择中心点位置')}</span>
            </div>
          </div>

          {/* Map Preview */}
          <div className="flex-1 relative bg-muted-30 min-h-0">
            <div className="absolute inset-0">
              <MapDisplay
                center={selectedCenter}
                zoom={zoom}
                tileLayer={tileLayer}
                markers={[previewMarker]}
                showControls={true}
                showFullscreen={false}
                interactive={true}
                onMapClick={handleMapClick}
                className="w-full h-full"
              />
            </div>

            {/* Crosshair overlay at center */}
            <div className="absolute inset-0 pointer-events-none flex items-center justify-center">
              <div className={cn(
                "w-8 h-8 relative transition-all duration-200",
                hasSelected && "scale-110"
              )}>
                {/* Horizontal line */}
                <div className="absolute top-1/2 left-0 right-0 h-0.5 bg-muted0 -translate-y-1/2" />
                {/* Vertical line */}
                <div className="absolute left-1/2 top-0 bottom-0 w-0.5 bg-muted0 -translate-x-1/2" />
              </div>
            </div>
          </div>

          {/* Selected coordinates display */}
          <div className="px-6 py-2 border-t bg-muted-20 shrink-0">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">{t('visualDashboard.latitude')}:</span>
                  <span className="font-mono text-xs">{selectedCenter.lat.toFixed(6)}</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">{t('visualDashboard.longitude')}:</span>
                  <span className="font-mono text-xs">{selectedCenter.lng.toFixed(6)}</span>
                </div>
              </div>
              {hasSelected && (
                <div className="text-xs text-success dark:text-success">
                  ✓ {t('mapDisplay.centerSelected', '已选择新位置')}
                </div>
              )}
            </div>
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-6 py-4 border-t shrink-0 bg-muted-30">
            <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
              {t('common.cancel')}
            </Button>
            <Button size="sm" onClick={handleSave} disabled={!hasSelected}>
              <Check className="h-4 w-4 mr-1" />
              {t('common.confirm')}
            </Button>
          </div>
        </div>
      )}
    </>
  )
}
