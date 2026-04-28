import { useState, useEffect, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { toast } from "@/components/ui/use-toast"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { RefreshCw, Plus } from "lucide-react"
import type { DeviceType, AddDeviceRequest, ConnectionConfig } from "@/types"
import { FormSection, FormSectionGroup } from "@/components/ui/form-section"
import { FormField } from "@/components/ui/field"
import { getServerOrigin } from "@/lib/api"
import { validateUrl } from "@/lib/form-validation"
import { UnifiedFormDialog } from "@/components/dialog/UnifiedFormDialog"

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

  const [selectedDeviceType, setSelectedDeviceType] = useState("")
  const [deviceId, setDeviceId] = useState("")
  const [deviceName, setDeviceName] = useState("")
  const [adapterType, setAdapterType] = useState<"mqtt" | "http" | "webhook">("mqtt")
  const [connectionConfig, setConnectionConfig] = useState<ConnectionConfig>({})
  const [errors, setErrors] = useState<Record<string, string>>({})

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
    // Validate required fields
    const newErrors: Record<string, string> = {}
    if (!selectedDeviceType) {
      newErrors.deviceType = t('devices:deviceType') + ' is required'
    }
    if (adapterType === 'http' && connectionConfig.url) {
      const urlError = validateUrl(connectionConfig.url, 'URL')
      if (urlError) newErrors.httpUrl = urlError
    }
    setErrors(newErrors)
    if (Object.keys(newErrors).length > 0) return

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
      setErrors({})
      onOpenChange(false)
    }
  }, [adding, onOpenChange])

  const selectedTemplate = deviceTypes.find(t => t.device_type === selectedDeviceType)
  const hasCommands = (selectedTemplate?.commands?.length || 0) > 0

  return (
    <UnifiedFormDialog
      open={open}
      onOpenChange={(newOpen) => { if (!newOpen && !adding) handleClose() }}
      title={t('devices:add.title')}
      icon={<Plus className="h-5 w-5 text-primary" />}
      width="sm"
      onSubmit={handleAdd}
      isSubmitting={adding}
      submitLabel={t('common:add')}
      submitDisabled={!selectedDeviceType}
    >
      <FormSectionGroup>
        {/* Device Type */}
        <FormField
          label={t('devices:deviceType')}
          required
          error={errors.deviceType}
        >
          <Select value={selectedDeviceType} onValueChange={(v) => { setSelectedDeviceType(v); setErrors(prev => { const next = { ...prev }; delete next.deviceType; return next }) }}>
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
              <FormField label={t('devices:add.httpUrl')} error={errors.httpUrl}>
                <Input
                  value={connectionConfig.url || ''}
                  onChange={(e) => { setConnectionConfig({ ...connectionConfig, url: e.target.value }); setErrors(prev => { const next = { ...prev }; delete next.httpUrl; return next }) }}
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
    </UnifiedFormDialog>
  )
}
