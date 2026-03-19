import { useState, useEffect, useMemo, useCallback } from "react"
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
import { Edit2, X } from "lucide-react"
import type { Device, DeviceType, ConnectionConfig } from "@/types"
import { useIsMobile, useSafeAreaInsets } from "@/hooks/useMobile"
import { useMobileBodyScrollLock } from "@/hooks/useBodyScrollLock"
import { cn } from "@/lib/utils"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"
import { FormField } from "@/components/ui/field"

interface EditDeviceDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  device: Device | null
  deviceTypes: DeviceType[]
  onEdit: (id: string, data: Partial<{ name: string; adapter_type: string; connection_config: ConnectionConfig }>) => Promise<boolean>
  editing: boolean
}

export function EditDeviceDialog({
  open,
  onOpenChange,
  device,
  deviceTypes,
  onEdit,
  editing,
}: EditDeviceDialogProps) {
  const { t } = useTranslation(['common', 'devices'])
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [deviceName, setDeviceName] = useState("")
  const [adapterType, setAdapterType] = useState<"mqtt" | "http" | "webhook">("mqtt")
  const [connectionConfig, setConnectionConfig] = useState<ConnectionConfig>({})

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

  // Memoize device type info to prevent unnecessary re-renders
  const deviceTypeInfo = useMemo(() => {
    if (!device?.device_type) return null
    return deviceTypes.find(t => t.device_type === device.device_type) || null
  }, [device?.device_type, deviceTypes])

  const hasCommands = useMemo(() => {
    return (deviceTypeInfo?.commands?.length || 0) > 0
  }, [deviceTypeInfo?.commands?.length])

  // Initialize form with device data when dialog opens
  useEffect(() => {
    if (open && device) {
      setDeviceName(device.name || "")
      setAdapterType((device.adapter_type as "mqtt" | "http" | "webhook") || "mqtt")

      const config = device.connection_config || {}

      if (hasCommands && !config.command_topic && device.device_type && device.id) {
        config.command_topic = `device/${device.device_type}/${device.id}/downlink`
      }

      setConnectionConfig(config)
    }
  }, [open, device, hasCommands])

  const handleEdit = async () => {
    if (!device) return

    const success = await onEdit(device.id, {
      name: deviceName,
      adapter_type: adapterType,
      connection_config: connectionConfig,
    })

    if (success) {
      onOpenChange(false)
      toast({
        title: t('common:success'),
        description: t('devices:edit.success'),
      })
    } else {
      toast({
        title: t('common:failed'),
        description: t('devices:edit.error'),
        variant: "destructive",
      })
    }
  }

  const handleClose = useCallback(() => {
    if (!editing) {
      setDeviceName("")
      setAdapterType("mqtt")
      setConnectionConfig({})
      onOpenChange(false)
    }
  }, [editing, onOpenChange])

  const EditDeviceContent = () => (
    <FormSectionGroup>
      {/* Device ID (read-only) */}
      <FormField label={t('devices:deviceId')}>
        <Input
          value={device?.id || ''}
          readOnly
          disabled
          className="font-mono bg-muted"
        />
      </FormField>

      {/* Device Type (read-only) */}
      <FormField label={t('devices:deviceType')}>
        <Input
          value={deviceTypeInfo?.name || device?.device_type || ''}
          readOnly
          disabled
          className="bg-muted"
        />
      </FormField>

      {/* Device Name */}
      <FormField label={t('devices:deviceName')}>
        <Input
          value={deviceName}
          onChange={(e) => setDeviceName(e.target.value)}
          placeholder={t('common:optional')}
        />
      </FormField>

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
              {(window as any).__TAURI__ ? 'http://localhost:9375' : window.location.origin}/api/devices/webhook/{device?.id}
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
                <Edit2 className="h-5 w-5 text-primary shrink-0" />
                <div className="min-w-0 flex-1">
                  <h1 className="text-base font-semibold truncate">{t('devices:edit.title')}</h1>
                  <p className="text-xs text-muted-foreground truncate">
                    {device?.name || device?.id}
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="icon" onClick={handleClose} disabled={editing} className="shrink-0">
                <X className="h-5 w-5" />
              </Button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="p-4">
                <EditDeviceContent />
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
              style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
            >
              <Button variant="outline" onClick={handleClose} disabled={editing} className="min-w-[80px]">
                {t('common:cancel')}
              </Button>
              <Button onClick={handleEdit} disabled={editing} className="min-w-[80px]">
                {editing ? t('common:saving') : t('common:save')}
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
              <Edit2 className="h-5 w-5 text-primary" />
              <h2 className="text-lg font-semibold leading-none truncate">
                {t('devices:edit.title')}
              </h2>
            </div>
            <button
              onClick={handleClose}
              disabled={editing}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <EditDeviceContent />
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-6 py-4 border-t shrink-0 bg-muted/30">
            <Button variant="outline" size="sm" onClick={handleClose} disabled={editing}>
              {t('common:cancel')}
            </Button>
            <Button size="sm" onClick={handleEdit} disabled={editing}>
              {editing ? t('common:saving') : t('common:save')}
            </Button>
          </div>
        </div>
      )}
    </>
  )
}
