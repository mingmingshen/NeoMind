import { useState, useEffect, useCallback } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { toast } from "@/components/ui/use-toast"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { RefreshCw, Plus, X } from "lucide-react"
import type { DeviceType, AddDeviceRequest, ConnectionConfig } from "@/types"
import { useIsMobile, useSafeAreaInsets } from "@/hooks/useMobile"
import { useMobileBodyScrollLock } from "@/hooks/useBodyScrollLock"
import { cn } from "@/lib/utils"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"
import { FormField } from "@/components/ui/field"
import { getServerOrigin } from "@/lib/api"
function generateRandomId(): string {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
  let result = ''
  for (let i = 0; i < 10; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length))
  }
  return result
}

interface AddDeviceDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  deviceTypes: DeviceType[]
  onAdd: (request: AddDeviceRequest) => Promise<boolean>
  adding: boolean
}

export function AddDeviceDialog({
  open,
  onOpenChange,
  deviceTypes,
  onAdd,
  adding,
}: AddDeviceDialogProps) {
  const { t } = useTranslation(['common', 'devices'])
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [selectedDeviceType, setSelectedDeviceType] = useState("")
  const [deviceId, setDeviceId] = useState("")
  const [deviceName, setDeviceName] = useState("")
  const [adapterType, setAdapterType] = useState<"mqtt" | "http" | "webhook">("mqtt")
  const [connectionConfig, setConnectionConfig] = useState<ConnectionConfig>({})

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  useEffect(() => {
    if (open && !deviceId) {
      setDeviceId(generateRandomId())
    }
  }, [open])

  useEffect(() => {
    // Set defaults based on adapter type
    if (adapterType === 'mqtt' && selectedDeviceType && deviceId) {
      const defaultTopic = `device/${selectedDeviceType}/${deviceId}/uplink`
      const defaultCommandTopic = `device/${selectedDeviceType}/${deviceId}/downlink`
      setConnectionConfig({
        telemetry_topic: defaultTopic,
        command_topic: defaultCommandTopic,
      })
    } else if (adapterType === 'http') {
      setConnectionConfig({
        url: `http://192.168.1.100/api/telemetry`,
        method: 'GET',
        poll_interval: 30,
      })
    } else {
      setConnectionConfig({})
    }
  }, [adapterType, selectedDeviceType, deviceId])

  const handleAdd = async () => {
    if (!selectedDeviceType) return

    const request: AddDeviceRequest = {
      device_id: deviceId || undefined,
      name: deviceName || deviceId || selectedDeviceType,
      device_type: selectedDeviceType,
      adapter_type: adapterType,
      connection_config: connectionConfig,
    }

    const success = await onAdd(request)
    if (success) {
      setSelectedDeviceType("")
      setDeviceId(generateRandomId())
      setDeviceName("")
      setAdapterType("mqtt")
      setConnectionConfig({})
      onOpenChange(false)
      toast({
        title: t('devices:add.success'),
        description: deviceId ? t('devices:add.successWithId', { deviceId }) : t('devices:add.successGeneric'),
      })
    } else {
      toast({
        title: t('devices:add.error'),
        description: t('devices:add.retryMessage'),
        variant: "destructive",
      })
    }
  }

  const handleClose = useCallback(() => {
    if (!adding) {
      setSelectedDeviceType("")
      setDeviceId("")
      setDeviceName("")
      setAdapterType("mqtt")
      setConnectionConfig({})
      onOpenChange(false)
    }
  }, [adding, onOpenChange])

  const selectedTemplate = deviceTypes.find(t => t.device_type === selectedDeviceType)
  const hasCommands = (selectedTemplate?.commands?.length || 0) > 0

  const AddDeviceContent = () => (
    <FormSectionGroup>
      {/* Device Type */}
      <FormField
        label={t('devices:deviceType')}
        required
      >
        <Select value={selectedDeviceType} onValueChange={setSelectedDeviceType}>
          <SelectTrigger>
            <SelectValue placeholder={t('devices:add.typePlaceholder')} />
          </SelectTrigger>
          <SelectContent>
            {deviceTypes.map((type) => (
              <SelectItem key={type.device_type} value={type.device_type}>
                {type.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </FormField>

      {/* Device ID & Name */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <FormField label={t('devices:deviceId')}>
          <div className="flex gap-2">
            <Input
              value={deviceId}
              onChange={(e) => setDeviceId(e.target.value)}
              placeholder={t('devices:id.autoGenerate')}
              className="font-mono"
            />
            <Button
              type="button"
              variant="outline"
              size="icon"
              onClick={() => setDeviceId(generateRandomId())}
              title={t('devices:id.regenerate')}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </FormField>
        <FormField label={t('devices:deviceName')}>
          <Input
            value={deviceName}
            onChange={(e) => setDeviceName(e.target.value)}
            placeholder={t('common:optional')}
          />
        </FormField>
      </div>

      {/* Adapter Type */}
      <FormField label={t('devices:add.adapterType')}>
        <Select
          value={adapterType}
          onValueChange={(v) => setAdapterType(v as "mqtt" | "http" | "webhook")}
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="mqtt">MQTT</SelectItem>
            <SelectItem value="http">HTTP</SelectItem>
            <SelectItem value="webhook">Webhook</SelectItem>
          </SelectContent>
        </Select>
      </FormField>

      {/* Adapter Config */}
      {adapterType === 'mqtt' && (
        <FormSection
          title={t('devices:add.mqttConfig', { defaultValue: 'MQTT Configuration' })}
          collapsible
          defaultExpanded
        >
          <div className="space-y-3">
            <FormField label={t('devices:add.telemetryTopic')}>
              <Input
                value={connectionConfig.telemetry_topic || ''}
                onChange={(e) => setConnectionConfig({ ...connectionConfig, telemetry_topic: e.target.value })}
                placeholder="device/{type}/{id}/uplink"
                className="font-mono text-sm"
              />
            </FormField>
            {hasCommands && (
              <FormField label={t('devices:add.commandTopic')}>
                <Input
                  value={connectionConfig.command_topic || ''}
                  onChange={(e) => setConnectionConfig({ ...connectionConfig, command_topic: e.target.value })}
                  placeholder="device/{type}/{id}/downlink"
                  className="font-mono text-sm"
                />
              </FormField>
            )}
          </div>
        </FormSection>
      )}

      {adapterType === 'http' && (
        <FormSection
          title={t('devices:add.httpConfig', { defaultValue: 'HTTP Configuration' })}
          collapsible
          defaultExpanded
        >
          <div className="space-y-3">
            <FormField label={t('devices:add.httpUrl')}>
              <Input
                value={connectionConfig.url || ''}
                onChange={(e) => setConnectionConfig({ ...connectionConfig, url: e.target.value })}
                placeholder="http://192.168.1.100/api/telemetry"
                className="font-mono text-sm"
              />
            </FormField>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <FormField label={t('devices:add.requestMethod')}>
                <Select
                  value={connectionConfig.method || 'GET'}
                  onValueChange={(v) => setConnectionConfig({ ...connectionConfig, method: v })}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="GET">GET</SelectItem>
                    <SelectItem value="POST">POST</SelectItem>
                  </SelectContent>
                </Select>
              </FormField>
              <FormField label={t('devices:add.pollInterval')}>
                <Input
                  type="number"
                  min="1"
                  value={connectionConfig.poll_interval || 30}
                  onChange={(e) => setConnectionConfig({ ...connectionConfig, poll_interval: parseInt(e.target.value) || 30 })}
                />
              </FormField>
            </div>
          </div>
        </FormSection>
      )}

      {adapterType === 'webhook' && (
        <FormSection
          title={t('devices:add.webhookConfig', { defaultValue: 'Webhook Configuration' })}
        >
          <div className="rounded-lg border bg-muted p-4">
            <p className="text-sm text-muted-foreground mb-2">
              {t('devices:add.webhookUrlDescription')}
            </p>
            <code className="text-xs break-all block">
              {getServerOrigin()}/api/devices/webhook/{deviceId}
            </code>
          </div>
        </FormSection>
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
                <Plus className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">{t('devices:add.title')}</h1>
                  <p className="text-xs text-muted-foreground truncate">
                    {t('devices:add.subtitle', { defaultValue: 'Add a new device' })}
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} disabled={adding} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <AddDeviceContent />
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={handleClose} disabled={adding} className="min-w-[80px]">
                {t('common:cancel')}
              </Button>
              <Button onClick={handleAdd} disabled={!selectedDeviceType || adding} className="min-w-[80px]">
                {adding ? t('devices:adding') : t('common:add')}
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
            'max-w-md',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex items-center gap-2 flex-1 min-w-0">
              <Plus className="h-5 w-5 text-primary" />
              <h2 className="text-lg font-semibold leading-none truncate">
                {t('devices:add.title')}
              </h2>
            </div>
            <button
              onClick={handleClose}
              disabled={adding}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <AddDeviceContent />
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-6 py-4 border-t shrink-0 bg-muted/30">
            <Button variant="outline" size="sm" onClick={handleClose} disabled={adding}>
              {t('common:cancel')}
            </Button>
            <Button size="sm" onClick={handleAdd} disabled={!selectedDeviceType || adding}>
              {adding ? t('devices:adding') : t('common:add')}
            </Button>
          </div>
        </div>
      )}
    </>
  )
}
