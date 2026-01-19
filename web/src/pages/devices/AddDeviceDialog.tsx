import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
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
import { RefreshCw } from "lucide-react"
import type { DeviceType, AddDeviceRequest, ConnectionConfig } from "@/types"
import { TemplatePreview } from "@/components/devices/TemplatePreview"
// Generate 10-character random string (lowercase alphanumeric)
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

  // Generate random ID when dialog opens
  useEffect(() => {
    if (open && !deviceId) {
      setDeviceId(generateRandomId())
    }
  }, [open])

  // Generate default telemetry topic for MQTT adapter
  useEffect(() => {
    if (adapterType === 'mqtt' && selectedDeviceType && deviceId) {
      // Generate topic: device/{device_type}/{device_id}/uplink
      // This matches the MQTT adapter's subscription pattern: device/+/+/uplink
      // Only set if not already set by user
      if (!connectionConfig.telemetry_topic) {
        const defaultTopic = `device/${selectedDeviceType}/${deviceId}/uplink`
        setConnectionConfig(prev => ({
          ...prev,
          telemetry_topic: defaultTopic
        }))
      }
      // Generate default command topic if template has commands
      const template = deviceTypes.find(t => t.device_type === selectedDeviceType)
      const hasCommands = template?.commands && template.commands.length > 0
      if (hasCommands && !connectionConfig.command_topic) {
        setConnectionConfig(prev => ({
          ...prev,
          command_topic: `device/${selectedDeviceType}/${deviceId}/downlink`
        }))
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [adapterType, selectedDeviceType, deviceId, deviceTypes])

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
      setDeviceId("")
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

  // Reset form when dialog opens
  const handleOpenChange = (open: boolean) => {
    if (open) {
      // Generate new random ID when opening
      setDeviceId(generateRandomId())
      setConnectionConfig({})
    } else {
      setSelectedDeviceType("")
      setDeviceId("")
      setDeviceName("")
      setAdapterType("mqtt")
      setConnectionConfig({})
    }
    onOpenChange(open)
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-md max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t('devices:add.title')}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="device-type" dangerouslySetInnerHTML={{ __html: t('devices:add.typeRequired') }} />
            <Select value={selectedDeviceType} onValueChange={setSelectedDeviceType}>
              <SelectTrigger id="device-type">
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
            {!selectedDeviceType && (
              <p className="text-xs text-destructive">{t('devices:add.typeValidation')}</p>
            )}
          </div>
          
          {/* Template Preview */}
          {selectedDeviceType && (() => {
            const template = deviceTypes.find(t => t.device_type === selectedDeviceType)
            return template ? (
              <div className="space-y-2">
                <TemplatePreview template={template} />
              </div>
            ) : null
          })()}
          
          <div className="space-y-2">
            <Label htmlFor="device-id">{t('devices:add.id')}</Label>
            <div className="flex gap-2">
              <Input
                id="device-id"
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
            <p className="text-xs text-muted-foreground">{t('devices:id.topicHint', { type: selectedDeviceType || '{type}', id: deviceId || '{id}' })}</p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="device-name">{t('devices:deviceName')}</Label>
            <Input
              id="device-name"
              value={deviceName}
              onChange={(e) => setDeviceName(e.target.value)}
              placeholder={t('common:optional')}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="adapter-type">{t('devices:adapterType') || 'Adapter Type'}</Label>
            <Select value={adapterType} onValueChange={(v) => {
              setAdapterType(v as "mqtt" | "http" | "webhook")
              setConnectionConfig({}) // Reset config when adapter type changes
            }}>
              <SelectTrigger id="adapter-type">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="mqtt">MQTT</SelectItem>
                <SelectItem value="http">HTTP (Polling)</SelectItem>
                <SelectItem value="webhook">Webhook (Push)</SelectItem>
              </SelectContent>
            </Select>
          </div>
          {adapterType === 'mqtt' && (
            <div className="space-y-2">
              <Label htmlFor="telemetry-topic">Telemetry Topic</Label>
              <Input
                id="telemetry-topic"
                value={connectionConfig.telemetry_topic || ''}
                onChange={(e) => setConnectionConfig({ ...connectionConfig, telemetry_topic: e.target.value })}
                placeholder="device/{device_type}/{device_id}/uplink"
              />
              {selectedDeviceType && (() => {
                const template = deviceTypes.find(t => t.device_type === selectedDeviceType)
                const commands = template?.commands || []
                return commands.length > 0
              })() && (
                <>
                  <Label htmlFor="command-topic">Command Topic</Label>
                  <Input
                    id="command-topic"
                    value={connectionConfig.command_topic || ''}
                    onChange={(e) => setConnectionConfig({ ...connectionConfig, command_topic: e.target.value })}
                    placeholder="device/{device_type}/{device_id}/downlink"
                  />
                </>
              )}
            </div>
          )}
          {adapterType === 'http' && (
            <div className="space-y-3">
              <div className="space-y-2">
                <Label htmlFor="http-url">URL</Label>
                <Input
                  id="http-url"
                  value={connectionConfig.url || ''}
                  onChange={(e) => setConnectionConfig({ ...connectionConfig, url: e.target.value })}
                  placeholder="http://192.168.1.100/api/telemetry"
                />
              </div>
              <div className="grid grid-cols-2 gap-2">
                <div className="space-y-2">
                  <Label htmlFor="http-method">Method</Label>
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
                  <Label htmlFor="poll-interval">Poll Interval (sec)</Label>
                  <Input
                    id="poll-interval"
                    type="number"
                    min="1"
                    value={connectionConfig.poll_interval || 30}
                    onChange={(e) => setConnectionConfig({ ...connectionConfig, poll_interval: parseInt(e.target.value) || 30 })}
                  />
                </div>
              </div>
              <div className="space-y-2">
                <Label htmlFor="data-path">Data Path (JSONPath, optional)</Label>
                <Input
                  id="data-path"
                  value={connectionConfig.data_path || ''}
                  onChange={(e) => setConnectionConfig({ ...connectionConfig, data_path: e.target.value })}
                  placeholder="$.data.sensors[0]"
                />
                <p className="text-xs text-muted-foreground">Extract nested JSON values using dot notation</p>
              </div>
            </div>
          )}
          {adapterType === 'webhook' && (
            <div className="space-y-4 rounded-lg border p-4 bg-muted/30">
              <div className="flex items-center justify-between">
                <h4 className="text-sm font-medium">Webhook Configuration</h4>
                <span className="text-xs text-muted-foreground">Devices push data to you</span>
              </div>
              <div className="space-y-2">
                <p className="text-sm text-muted-foreground">
                  After adding the device, use the Webhook URL below to configure your device:
                </p>
                <div className="flex items-center gap-2 rounded-md bg-background p-3 font-mono text-sm">
                  <code className="flex-1">
                    {window.location.origin}/api/devices/webhook/{deviceId}
                  </code>
                </div>
                <p className="text-xs text-muted-foreground">
                  Device should POST JSON data: <code className="bg-muted px-1 rounded">{"{\"data\": {\"temperature\": 23.5}}"}</code>
                </p>
              </div>
            </div>
          )}
        </div>
        <DialogFooter>
          <Button onClick={handleAdd} disabled={!selectedDeviceType || adding} size="sm">
            {adding ? t('devices:adding') : t('common:add')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
