import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Clock, Play, AlertTriangle, Zap } from 'lucide-react'
import type { WorkflowTrigger } from '@/types'

interface TriggerConfigDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  trigger: WorkflowTrigger | null
  onSave: (trigger: WorkflowTrigger) => void
  resources?: {
    devices: Array<{ id: string; name: string }>
    metrics: string[]
  }
}

export function TriggerConfigDialog({ open, onOpenChange, trigger, onSave, resources }: TriggerConfigDialogProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [localTrigger, setLocalTrigger] = useState<WorkflowTrigger | null>(null)

  useEffect(() => {
    if (trigger) {
      setLocalTrigger(JSON.parse(JSON.stringify(trigger)))
    } else {
      setLocalTrigger(null)
    }
  }, [trigger, open])

  if (!localTrigger) return null

  const updateLocalTrigger = (updates: Partial<WorkflowTrigger>) => {
    setLocalTrigger({ ...localTrigger, ...updates } as WorkflowTrigger)
  }

  const handleSave = () => {
    if (localTrigger) {
      onSave(localTrigger)
    }
  }

  const renderConfig = () => {
    switch (localTrigger!.type) {
      case 'manual':
        return <ManualTriggerConfig />
      case 'cron':
        return <CronTriggerConfig trigger={localTrigger} onChange={updateLocalTrigger} />
      case 'event':
        return <EventTriggerConfig trigger={localTrigger} onChange={updateLocalTrigger} />
      case 'device':
        return <DeviceTriggerConfig trigger={localTrigger} onChange={updateLocalTrigger} resources={resources} />
      default:
        return <div className="text-muted-foreground">{t('automation:unknownTriggerType')}</div>
    }
  }

  const getTriggerIcon = () => {
    switch (localTrigger!.type) {
      case 'manual':
        return <Play className="h-5 w-5" />
      case 'cron':
        return <Clock className="h-5 w-5" />
      case 'event':
        return <AlertTriangle className="h-5 w-5" />
      case 'device':
        return <Zap className="h-5 w-5" />
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {getTriggerIcon()}
            {t('automation:configureTrigger', {
              trigger: t(`automation:triggers.${localTrigger!.type}`),
            })}
          </DialogTitle>
          <DialogDescription>
            {t(`automation:triggers.${localTrigger!.type}Desc`)}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {renderConfig()}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('common:cancel')}
          </Button>
          <Button onClick={handleSave}>{t('common:save')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// Manual Trigger Configuration (no additional config needed)
function ManualTriggerConfig() {
  const { t } = useTranslation('automation')
  return (
    <div className="text-center py-4 text-muted-foreground">
      <p>{t('automation:manualTriggerNoConfig')}</p>
    </div>
  )
}

// Cron Trigger Configuration
function CronTriggerConfig({ trigger, onChange }: { trigger: WorkflowTrigger; onChange: (updates: Partial<WorkflowTrigger>) => void }) {
  const { t } = useTranslation('automation')
  const cronTrigger = trigger as any

  const presets = [
    { label: t('automation:cronPresets.everyMinute'), expression: '* * * * *' },
    { label: t('automation:cronPresets.every5Minutes'), expression: '*/5 * * * *' },
    { label: t('automation:cronPresets.every15Minutes'), expression: '*/15 * * * *' },
    { label: t('automation:cronPresets.everyHour'), expression: '0 * * * *' },
    { label: t('automation:cronPresets.everyDay'), expression: '0 0 * * *' },
    { label: t('automation:cronPresets.everyWeek'), expression: '0 0 * * 0' },
  ]

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="cron-expression">{t('automation:cronExpression')}</Label>
        <Input
          id="cron-expression"
          value={cronTrigger.expression || ''}
          onChange={(e) => onChange({ expression: e.target.value })}
          placeholder="0 * * * *"
          className="font-mono"
        />
        <p className="text-xs text-muted-foreground mt-1">
          {t('automation:cronExpressionHint')}
        </p>
      </div>

      <div>
        <Label>{t('automation:presets')}</Label>
        <div className="grid grid-cols-2 gap-2 mt-2">
          {presets.map((preset) => (
            <Button
              key={preset.expression}
              variant="outline"
              size="sm"
              onClick={() => onChange({ expression: preset.expression })}
              className="justify-start text-xs"
            >
              {preset.label}
            </Button>
          ))}
        </div>
      </div>

      <div>
        <Label htmlFor="cron-timezone">{t('automation:timezone')}</Label>
        <Input
          id="cron-timezone"
          value={cronTrigger.timezone || 'UTC'}
          onChange={(e) => onChange({ timezone: e.target.value })}
          placeholder="UTC"
        />
      </div>
    </div>
  )
}

// Event Trigger Configuration
function EventTriggerConfig({ trigger, onChange }: { trigger: WorkflowTrigger; onChange: (updates: Partial<WorkflowTrigger>) => void }) {
  const { t } = useTranslation('automation')
  const eventTrigger = trigger as any

  const commonEvents = [
    'device.online',
    'device.offline',
    'device.metric',
    'alert.created',
    'workflow.completed',
    'workflow.failed',
  ]

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="event-type">{t('automation:eventType')}</Label>
        <Select
          value={eventTrigger.event_type || ''}
          onValueChange={(value) => onChange({ event_type: value })}
        >
          <SelectTrigger id="event-type">
            <SelectValue placeholder={t('automation:selectEventType')} />
          </SelectTrigger>
          <SelectContent>
            {commonEvents.map((event) => (
              <SelectItem key={event} value={event}>
                {event}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground mt-1">
          {t('automation:eventTypeHint')}
        </p>
      </div>

      <div>
        <Label htmlFor="event-filters">{t('automation:eventFilters')}</Label>
        <Textarea
          id="event-filters"
          value={JSON.stringify(eventTrigger.filters || {}, null, 2)}
          onChange={(e) => {
            try {
              onChange({ filters: JSON.parse(e.target.value) })
            } catch {
              // Invalid JSON, ignore
            }
          }}
          placeholder='{"device_id": "xxx"}'
          className="min-h-[100px] font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground mt-1">
          {t('automation:eventFiltersHint')}
        </p>
      </div>
    </div>
  )
}

// Device Trigger Configuration
function DeviceTriggerConfig({
  trigger,
  onChange,
  resources,
}: {
  trigger: WorkflowTrigger
  onChange: (updates: Partial<WorkflowTrigger>) => void
  resources?: { devices: Array<{ id: string; name: string }> }
}) {
  const { t } = useTranslation('automation')
  const deviceTrigger = trigger as any
  const devices = resources?.devices || []

  const conditions = [
    { value: '>', label: '>' },
    { value: '<', label: '<' },
    { value: '>=', label: '>=' },
    { value: '<=', label: '<=' },
    { value: '==', label: '==' },
    { value: '!=', label: '!=' },
  ]

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="device-trigger-device">{t('automation:device')}</Label>
        <Select
          value={deviceTrigger.device_id || ''}
          onValueChange={(value) => onChange({ device_id: value })}
        >
          <SelectTrigger id="device-trigger-device">
            <SelectValue placeholder={t('automation:selectDevice')} />
          </SelectTrigger>
          <SelectContent>
            {devices.map((device) => (
              <SelectItem key={device.id} value={device.id}>
                {device.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div>
        <Label htmlFor="device-trigger-metric">{t('automation:metric')}</Label>
        <Input
          id="device-trigger-metric"
          value={deviceTrigger.metric || ''}
          onChange={(e) => onChange({ metric: e.target.value })}
          placeholder="temperature"
        />
      </div>

      <div>
        <Label htmlFor="device-trigger-condition">{t('automation:condition')}</Label>
        <Select
          value={deviceTrigger.condition || '>'}
          onValueChange={(value) => onChange({ condition: value })}
        >
          <SelectTrigger id="device-trigger-condition">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {conditions.map((cond) => (
              <SelectItem key={cond.value} value={cond.value}>
                {cond.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="p-3 bg-muted rounded-md">
        <p className="text-sm font-medium">{t('automation:example')}</p>
        <code className="text-xs">
          {t('automation:deviceTriggerExample', {
            condition: deviceTrigger.condition || '>',
            metric: deviceTrigger.metric || 'temperature',
          })}
        </code>
      </div>
    </div>
  )
}
