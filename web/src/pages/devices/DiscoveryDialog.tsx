import { useState, useCallback } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { RefreshCw, Radar, PlusCircle, X } from "lucide-react"
import type { DiscoveredDevice, DeviceType } from "@/types"
import { useIsMobile, useSafeAreaInsets } from "@/hooks/useMobile"
import { useMobileBodyScrollLock } from "@/hooks/useBodyScrollLock"
import { cn } from "@/lib/utils"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"

interface DiscoveryDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  discovering: boolean
  discoveredDevices: DiscoveredDevice[]
  deviceTypes: DeviceType[]
  onDiscover: (host: string) => Promise<void>
  onAddDiscovered: (device: DiscoveredDevice) => void
}

export function DiscoveryDialog({
  open,
  onOpenChange,
  discovering,
  discoveredDevices,
  deviceTypes,
  onDiscover,
  onAddDiscovered,
}: DiscoveryDialogProps) {
  const { t } = useTranslation(['common', 'devices'])
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [host, setHost] = useState("localhost")
  const [scanned, setScanned] = useState(false)

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  const handleDiscover = async () => {
    if (!host.trim()) return
    setScanned(false)
    await onDiscover(host.trim())
    setScanned(true)
  }

  const handleClose = useCallback(() => {
    if (!discovering) {
      onOpenChange(false)
    }
  }, [discovering, onOpenChange])

  const getDeviceTypeName = (typeId: string | null) => {
    if (!typeId) return t('devices:discoveryDialog.unknown')
    const dt = deviceTypes.find((t) => t.device_type === typeId)
    return dt?.name || typeId
  }

  const getDeviceTypeLabel = (deviceType: string | null) => {
    if (deviceType === "mqtt_gateway") return t('devices:discoveryDialog.types.mqtt')
    if (deviceType === "http_device") return t('devices:discoveryDialog.types.http')
    if (deviceType === "coap_device") return t('devices:discoveryDialog.types.coap')
    return ""
  }

  const handleAddDevice = (device: DiscoveredDevice) => {
    onAddDiscovered(device)
  }

  const DiscoveryContent = () => (
    <FormSectionGroup>
      {/* Scan Controls */}
      <FormSection title={t('devices:discoveryDialog.scanNetwork', { defaultValue: 'Scan Network' })}>
        <div className="flex gap-2">
          <Input
            value={host}
            onChange={(e) => setHost(e.target.value)}
            placeholder={t('devices:discoveryDialog.hostPlaceholder')}
            className="flex-1"
          />
          <Button
            onClick={handleDiscover}
            disabled={!host.trim() || discovering}
            className="shrink-0"
          >
            {discovering ? (
              <>
                <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                {t('devices:discoveryDialog.scanning')}
              </>
            ) : (
              <>
                <Radar className="mr-2 h-4 w-4" />
                {t('devices:discoveryDialog.scan')}
              </>
            )}
          </Button>
        </div>
      </FormSection>

      {/* Results */}
      {scanned && discoveredDevices.length === 0 && (
        <div className="text-sm text-muted-foreground text-center py-8 border rounded-lg">
          {t('devices:discoveryDialog.noDevices')}
        </div>
      )}

      {discoveredDevices.length > 0 && (
        <FormSection
          title={`${t('devices:discoveryDialog.discoveredDevices', { defaultValue: 'Discovered Devices' })} (${discoveredDevices.length})`}
          
        >
          <div className="space-y-2">
            {discoveredDevices.map((device) => (
              <div
                key={device.id}
                className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 rounded-md border p-3"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-mono text-sm truncate">{device.host}:{device.port}</span>
                    <Badge variant="outline" className="text-xs">
                      {getDeviceTypeName(device.device_type)}
                    </Badge>
                  </div>
                  <p className="text-xs text-muted-foreground mt-1 truncate">
                    {getDeviceTypeLabel(device.device_type)}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handleAddDevice(device)}
                  className="shrink-0"
                >
                  <PlusCircle className="mr-2 h-4 w-4" />
                  {t('devices:discoveryDialog.add')}
                </Button>
              </div>
            ))}
          </div>
        </FormSection>
      )}

      {/* Initial State */}
      {!scanned && (
        <div className="py-8 text-center text-sm text-muted-foreground border rounded-lg">
          {t('devices:discoveryDialog.instruction')}
        </div>
      )}
    </FormSectionGroup>
  )

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      open ? (
        <div className="fixed inset-0 z-[100] bg-background animate-in fade-in duration-200">
          <div className="flex h-full w-full flex-col">
            {/* Header */}
            <div
              className="flex items-center justify-between px-4 py-4 border-b shrink-0 bg-background"
              style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <Radar className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">{t('devices:discoveryDialog.title')}</h1>
                  <p className="text-xs text-muted-foreground truncate">
                    {t('devices:discoveryDialog.subtitle', { defaultValue: 'Discover devices on network' })}
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} disabled={discovering} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <DiscoveryContent />
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={handleClose} disabled={discovering} className="min-w-[80px]">
                {t('common:close')}
              </Button>
            </div>
          </div>
        </div>
      ) : null,
      document.body
    )
  }

  // Desktop: Traditional dialog
  return (
    <>
      {/* Backdrop */}
      {open && (
        <div
          className="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm animate-in fade-in duration-200"
          onClick={handleClose}
        />
      )}

      {/* Dialog */}
      {open && (
        <div
          className={cn(
            'fixed left-1/2 top-1/2 z-50',
            'grid w-full gap-0',
            'bg-background shadow-lg',
            'duration-200',
            'animate-in fade-in zoom-in-95 slide-in-from-left-1/2 slide-in-from-top-[48%]',
            'rounded-lg sm:rounded-xl',
            'max-h-[calc(100vh-2rem)] sm:max-h-[85vh]',
            'flex flex-col',
            'max-w-3xl',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex flex-col gap-1.5 flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <Radar className="h-5 w-5 text-primary" />
                <h2 className="text-lg font-semibold leading-none truncate">
                  {t('devices:discoveryDialog.title')}
                </h2>
              </div>
              <p className="text-sm text-muted-foreground">
                {t('devices:discoveryDialog.subtitle', { defaultValue: 'Discover devices on your network' })}
              </p>
            </div>
            <button
              onClick={handleClose}
              disabled={discovering}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <DiscoveryContent />
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-6 py-4 border-t shrink-0 bg-muted/30">
            <Button variant="outline" size="sm" onClick={handleClose} disabled={discovering}>
              {t('common:close')}
            </Button>
          </div>
        </div>
      )}
    </>
  )
}
