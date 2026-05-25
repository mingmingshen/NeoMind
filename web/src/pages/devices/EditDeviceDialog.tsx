import { useState, useEffect, useMemo, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { getServerOrigin } from "@/lib/api"
import { Input } from "@/components/ui/input"
import { toast } from "@/components/ui/use-toast"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Edit2 } from "lucide-react"
import type { Device, DeviceType, ConnectionConfig } from "@/types"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"
import { FormField } from "@/components/ui/field"
import { UnifiedFormDialog } from "@/components/dialog/UnifiedFormDialog"

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
  const [adapterType, setAdapterType] = useState<"mqtt" | "webhook">("mqtt")
  const [connectionConfig, setConnectionConfig] = useState<ConnectionConfig>({})

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
      setAdapterType((device.adapter_type as "mqtt" | "webhook") || "mqtt")

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

  return (
    <UnifiedFormDialog
      open={open}
      onOpenChange={(newOpen) => { if (!newOpen && !editing) handleClose() }}
      title={t('devices:edit.title')}
      icon={<Edit2 className="h-5 w-5 text-primary" />}
      width="sm"
      onSubmit={handleEdit}
      isSubmitting={editing}
      submitLabel={t('common:save')}
    >
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
            onValueChange={(v) => setAdapterType(v as "mqtt" | "webhook")}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="mqtt">MQTT</SelectItem>
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

        {adapterType === 'webhook' && (
          <FormSection
            title={t('devices:add.webhookConfig', { defaultValue: 'Webhook Configuration' })}
          >
            <div className="rounded-lg border bg-muted p-4">
              <p className="text-sm text-muted-foreground mb-2">
                {t('devices:add.webhookUrlDescription')}
              </p>
              <code className="text-xs break-all block">
                {getServerOrigin()}/api/devices/{device?.id}/webhook
              </code>
            </div>
          </FormSection>
        )}
      </FormSectionGroup>
    </UnifiedFormDialog>
  )
}
