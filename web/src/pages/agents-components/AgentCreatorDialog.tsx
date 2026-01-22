import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Checkbox } from "@/components/ui/checkbox"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Bot, Clock, Zap } from "lucide-react"
import type {
  AiAgentDetail,
  CreateAgentRequest,
  Device,
  DeviceType,
  AgentScheduleType,
} from "@/types"

interface AgentCreatorDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  agent: AiAgentDetail | undefined
  devices: Device[]
  deviceTypes: DeviceType[]
  onSave: (data: CreateAgentRequest | Partial<AiAgentDetail>) => Promise<void>
}

export function AgentCreatorDialog({
  open,
  onOpenChange,
  agent,
  devices,
  deviceTypes,
  onSave,
}: AgentCreatorDialogProps) {
  const { t } = useTranslation(['common', 'agents'])
  const isEditing = !!agent

  // Form state
  const [name, setName] = useState("")
  const [userPrompt, setUserPrompt] = useState("")
  const [scheduleType, setScheduleType] = useState<AgentScheduleType>('interval')
  const [intervalSeconds, setIntervalSeconds] = useState(60)
  const [cronExpression, setCronExpression] = useState("")
  const [timezone, setTimezone] = useState(Intl.DateTimeFormat().resolvedOptions().timeZone)

  // Selection state
  const [selectedDeviceIds, setSelectedDeviceIds] = useState<Set<string>>(new Set())
  const [selectedMetrics, setSelectedMetrics] = useState<Array<{ device_id: string; metric_name: string; display_name: string }>>([])
  const [selectedCommands, setSelectedCommands] = useState<Array<{ device_id: string; command_name: string; display_name: string; parameters: Record<string, unknown> }>>([])

  // Loading state
  const [saving, setSaving] = useState(false)

  // Initialize form from agent
  useEffect(() => {
    if (agent) {
      setName(agent.name)
      setUserPrompt(agent.user_prompt)
      // TODO: Parse schedule from agent when available
    } else {
      setName("")
      setUserPrompt("")
      setScheduleType('interval')
      setIntervalSeconds(60)
      setCronExpression("")
      setSelectedDeviceIds(new Set())
      setSelectedMetrics([])
      setSelectedCommands([])
    }
  }, [agent, open])

  const handleSave = async () => {
    if (!name.trim() || !userPrompt.trim()) {
      return
    }

    setSaving(true)
    try {
      if (isEditing && agent) {
        await onSave({
          name: name.trim(),
          user_prompt: userPrompt.trim(),
        })
      } else {
        const data: CreateAgentRequest = {
          name: name.trim(),
          user_prompt: userPrompt.trim(),
          device_ids: Array.from(selectedDeviceIds),
          metrics: selectedMetrics,
          commands: selectedCommands,
          schedule: {
            schedule_type: scheduleType,
            interval_seconds: scheduleType === 'interval' ? intervalSeconds : undefined,
            cron_expression: scheduleType === 'cron' ? cronExpression : undefined,
            timezone,
          },
        }
        await onSave(data)
      }
      onOpenChange(false)
    } catch (error) {
      console.error('Failed to save agent:', error)
    } finally {
      setSaving(false)
    }
  }

  const toggleDevice = (deviceId: string) => {
    const newSelected = new Set(selectedDeviceIds)
    if (newSelected.has(deviceId)) {
      newSelected.delete(deviceId)
    } else {
      newSelected.add(deviceId)
    }
    setSelectedDeviceIds(newSelected)
  }

  const getDeviceMetrics = (deviceId: string) => {
    const device = devices.find(d => d.device_id === deviceId)
    if (!device) return []

    const deviceType = deviceTypes.find(dt => dt.device_type === device.device_type)
    return deviceType?.metrics || []
  }

  const getDeviceCommands = (deviceId: string) => {
    const device = devices.find(d => d.device_id === deviceId)
    if (!device) return []

    const deviceType = deviceTypes.find(dt => dt.device_type === device.device_type)
    return deviceType?.commands || []
  }

  const toggleMetric = (deviceId: string, metricName: string, displayName: string) => {
    const existing = selectedMetrics.findIndex(
      m => m.device_id === deviceId && m.metric_name === metricName
    )
    if (existing >= 0) {
      setSelectedMetrics(selectedMetrics.filter((_, i) => i !== existing))
    } else {
      setSelectedMetrics([...selectedMetrics, { device_id: deviceId, metric_name: metricName, display_name: displayName }])
    }
  }

  const toggleCommand = (deviceId: string, commandName: string, displayName: string) => {
    const existing = selectedCommands.findIndex(
      c => c.device_id === deviceId && c.command_name === commandName
    )
    if (existing >= 0) {
      setSelectedCommands(selectedCommands.filter((_, i) => i !== existing))
    } else {
      setSelectedCommands([...selectedCommands, { device_id: deviceId, command_name: commandName, display_name: displayName, parameters: {} }])
    }
  }

  const isMetricSelected = (deviceId: string, metricName: string) => {
    return selectedMetrics.some(m => m.device_id === deviceId && m.metric_name === metricName)
  }

  const isCommandSelected = (deviceId: string, commandName: string) => {
    return selectedCommands.some(c => c.device_id === deviceId && c.command_name === commandName)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Bot className="h-5 w-5" />
            {isEditing ? t('agents:editAgent') : t('agents:createAgent')}
          </DialogTitle>
          <DialogDescription>
            {isEditing ? t('agents:editAgentDesc') : t('agents:createAgentDesc')}
          </DialogDescription>
        </DialogHeader>

        <ScrollArea className="max-h-[60vh] pr-4">
          <div className="space-y-6">
            {/* Basic Info */}
            <div className="space-y-4">
              <div>
                <Label htmlFor="name">{t('agents:agentName')} *</Label>
                <Input
                  id="name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder={t('agents:agentNamePlaceholder')}
                />
              </div>

              <div>
                <Label htmlFor="prompt">{t('agents:userPrompt')} *</Label>
                <Textarea
                  id="prompt"
                  value={userPrompt}
                  onChange={(e) => setUserPrompt(e.target.value)}
                  placeholder={t('agents:userPromptPlaceholder')}
                  rows={4}
                  className="resize-none"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  {t('agents:userPromptHint')}
                </p>
              </div>
            </div>

            {/* Schedule - only for new agents */}
            {!isEditing && (
              <div className="space-y-4">
                <Label className="flex items-center gap-2">
                  <Clock className="h-4 w-4" />
                  {t('agents:schedule')}
                </Label>

                <Select
                  value={scheduleType}
                  onValueChange={(v) => setScheduleType(v as AgentScheduleType)}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="interval">{t('agents:scheduleTypes.interval')}</SelectItem>
                    <SelectItem value="cron">{t('agents:scheduleTypes.cron')}</SelectItem>
                    <SelectItem value="event">{t('agents:scheduleTypes.event')}</SelectItem>
                    <SelectItem value="once">{t('agents:scheduleTypes.once')}</SelectItem>
                  </SelectContent>
                </Select>

                {scheduleType === 'interval' && (
                  <div>
                    <Label htmlFor="interval">{t('agents:intervalSeconds')}</Label>
                    <Input
                      id="interval"
                      type="number"
                      value={intervalSeconds}
                      onChange={(e) => setIntervalSeconds(Number(e.target.value))}
                      min={10}
                      step={10}
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      {t('agents:intervalHint', { seconds: intervalSeconds })}
                    </p>
                  </div>
                )}

                {scheduleType === 'cron' && (
                  <div>
                    <Label htmlFor="cron">{t('agents:cronExpression')}</Label>
                    <Input
                      id="cron"
                      value={cronExpression}
                      onChange={(e) => setCronExpression(e.target.value)}
                      placeholder="0 * * * *"
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      {t('agents:cronHint')}
                    </p>
                  </div>
                )}

                <div>
                  <Label htmlFor="timezone">{t('agents:timezone')}</Label>
                  <Input
                    id="timezone"
                    value={timezone}
                    onChange={(e) => setTimezone(e.target.value)}
                    placeholder="UTC"
                  />
                </div>
              </div>
            )}

            {/* Devices - only for new agents */}
            {!isEditing && (
              <div className="space-y-4">
                <Label className="flex items-center gap-2">
                  <Zap className="h-4 w-4" />
                  {t('agents:resources')}
                </Label>

                {devices.length === 0 ? (
                  <p className="text-sm text-muted-foreground">{t('agents:noDevices')}</p>
                ) : (
                  <div className="space-y-4">
                    {devices.map((device) => {
                      const isSelected = selectedDeviceIds.has(device.device_id)
                      const metrics = getDeviceMetrics(device.device_id)
                      const commands = getDeviceCommands(device.device_id)

                      return (
                        <div key={device.device_id} className="border rounded-lg p-4 space-y-3">
                          {/* Device selection */}
                          <div className="flex items-center gap-3">
                            <Checkbox
                              id={`device-${device.device_id}`}
                              checked={isSelected}
                              onCheckedChange={() => toggleDevice(device.device_id)}
                            />
                            <Label htmlFor={`device-${device.device_id}`} className="font-medium cursor-pointer">
                              {device.name}
                            </Label>
                            <span className="text-xs text-muted-foreground">
                              ({device.device_type})
                            </span>
                          </div>

                          {/* Metrics and Commands - only show if device is selected */}
                          {isSelected && (metrics.length > 0 || commands.length > 0) && (
                            <div className="ml-7 space-y-3">
                              {metrics.length > 0 && (
                                <div>
                                  <p className="text-xs font-medium text-muted-foreground mb-2">
                                    {t('agents:metrics')}
                                  </p>
                                  <div className="flex flex-wrap gap-2">
                                    {metrics.map((metric) => (
                                      <button
                                        key={metric.name}
                                        onClick={() => toggleMetric(device.device_id, metric.name, metric.display_name)}
                                        className={`text-xs px-2 py-1 rounded border transition-colors ${
                                          isMetricSelected(device.device_id, metric.name)
                                            ? 'bg-primary text-primary-foreground border-primary'
                                            : 'bg-background hover:bg-muted'
                                        }`}
                                      >
                                        {metric.display_name}
                                      </button>
                                    ))}
                                  </div>
                                </div>
                              )}

                              {commands.length > 0 && (
                                <div>
                                  <p className="text-xs font-medium text-muted-foreground mb-2">
                                    {t('agents:commands')}
                                  </p>
                                  <div className="flex flex-wrap gap-2">
                                    {commands.map((command) => (
                                      <button
                                        key={command.name}
                                        onClick={() => toggleCommand(device.device_id, command.name, command.display_name)}
                                        className={`text-xs px-2 py-1 rounded border transition-colors ${
                                          isCommandSelected(device.device_id, command.name)
                                            ? 'bg-primary text-primary-foreground border-primary'
                                            : 'bg-background hover:bg-muted'
                                        }`}
                                      >
                                        {command.display_name}
                                      </button>
                                    ))}
                                  </div>
                                </div>
                              )}
                            </div>
                          )}
                        </div>
                      )
                    })}
                  </div>
                )}
              </div>
            )}
          </div>
        </ScrollArea>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('common:cancel')}
          </Button>
          <Button onClick={handleSave} disabled={saving || !name.trim() || !userPrompt.trim()}>
            {saving ? t('common:saving') : isEditing ? t('common:save') : t('common:create')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
