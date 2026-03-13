import { useState, useEffect, useMemo } from "react"
import { useTranslation } from "react-i18next"
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogContentBody } from "@/components/ui/dialog"
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
import type { Device, DeviceType, ConnectionConfig } from "@/types"

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
  const [deviceName, setDeviceName] = useState("")
  const [adapterType, setAdapterType] = useState<"mqtt" | "http" | "webhook">("mqtt")
  const [connectionConfig, setConnectionConfig] = useState<ConnectionConfig>({})

  // Memoize device type info to prevent unnecessary re-renders
  // Only recompute when device.device_type or deviceTypes array actually changes
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

      // Initialize connection config
      const config = device.connection_config || {}

      // If device type has commands but no command_topic is set, generate default
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

  const handleOpenChange = (open: boolean) => {
    if (!open) {
      // Reset form when closing
      setDeviceName("")
      setAdapterType("mqtt")
      setConnectionConfig({})
    }
    onOpenChange(open)
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md flex flex-col">
        <DialogHeader>
          <DialogTitle>{t('devices:edit.title')}</DialogTitle>
        </DialogHeader>

        <DialogContentBody className="space-y-4 py-4">
          {/* Device ID (read-only) */}
          <div className="space-y-2">
            <Label htmlFor="device-id">{t('devices:deviceId')}</Label>
            <Input
              id="device-id"
              value={device?.id || ''}
              readOnly
              disabled
              className="font-mono bg-muted"
            />
          </div>

          {/* Device Type (read-only) */}
          <div className="space-y-2">
            <Label htmlFor="device-type">{t('devices:deviceType')}</Label>
            <Input
              id="device-type"
              value={deviceTypeInfo?.name || device?.device_type || ''}
              readOnly
              disabled
              className="bg-muted"
            />
          </div>

          {/* Device Name */}
          <div className="space-y-2">
            <Label htmlFor="device-name">{t('devices:deviceName')}</Label>
            <Input
              id="device-name"
              value={deviceName}
              onChange={(e) => setDeviceName(e.target.value)}
              placeholder={t('common:optional')}
            />
          </div>

          {/* Adapter Type */}
          <div className="space-y-2">
            <Label htmlFor="adapter-type">{t('devices:add.adapterType')}</Label>
            <Select
              value={adapterType}
              onValueChange={(v) => setAdapterType(v as "mqtt" | "http" | "webhook")}
            >
              <SelectTrigger id="adapter-type">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="mqtt">MQTT</SelectItem>
                <SelectItem value="http">HTTP</SelectItem>
                <SelectItem value="webhook">Webhook</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Adapter Config */}
          {adapterType === 'mqtt' && (
            <div className="space-y-3">
              <div className="space-y-2">
                <Label htmlFor="telemetry-topic">{t('devices:add.telemetryTopic')}</Label>
                <Input
                  id="telemetry-topic"
                  value={connectionConfig.telemetry_topic || ''}
                  onChange={(e) => setConnectionConfig({ ...connectionConfig, telemetry_topic: e.target.value })}
                  placeholder="device/{type}/{id}/uplink"
                  className="font-mono text-sm"
                />
              </div>
              {hasCommands && (
                <div className="space-y-2">
                  <Label htmlFor="command-topic">{t('devices:add.commandTopic')}</Label>
                  <Input
                    id="command-topic"
                    value={connectionConfig.command_topic || ''}
                    onChange={(e) => setConnectionConfig({ ...connectionConfig, command_topic: e.target.value })}
                    placeholder="device/{type}/{id}/downlink"
                    className="font-mono text-sm"
                  />
                </div>
              )}
            </div>
          )}

          {adapterType === 'http' && (
            <div className="space-y-3">
              <div className="space-y-2">
                <Label htmlFor="http-url">{t('devices:add.httpUrl')}</Label>
                <Input
                  id="http-url"
                  value={connectionConfig.url || ''}
                  onChange={(e) => setConnectionConfig({ ...connectionConfig, url: e.target.value })}
                  placeholder="http://192.168.1.100/api/telemetry"
                  className="font-mono text-sm"
                />
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="http-method">{t('devices:add.requestMethod')}</Label>
                  <Select
                    value={connectionConfig.method || 'GET'}
                    onValueChange={(v) => setConnectionConfig({ ...connectionConfig, method: v })}
                  >
                    <SelectTrigger id="http-method">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="GET">GET</SelectItem>
                      <SelectItem value="POST">POST</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="poll-interval">{t('devices:add.pollInterval')}</Label>
                  <Input
                    id="poll-interval"
                    type="number"
                    min="1"
                    value={connectionConfig.poll_interval || 30}
                    onChange={(e) => setConnectionConfig({ ...connectionConfig, poll_interval: parseInt(e.target.value) || 30 })}
                  />
                </div>
              </div>
            </div>
          )}

          {adapterType === 'webhook' && (
            <div className="rounded-lg border bg-muted p-4">
              <p className="text-sm text-muted-foreground mb-2">
                {t('devices:add.webhookUrlDescription')}
              </p>
              <code className="text-xs break-all block">
                {(window as any).__TAURI__ ? 'http://localhost:9375' : window.location.origin}/api/devices/webhook/{device?.id}
              </code>
            </div>
          )}
        </DialogContentBody>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('common:cancel')}
          </Button>
          <Button onClick={handleEdit} disabled={editing}>
            {editing ? t('common:saving') : t('common:save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
