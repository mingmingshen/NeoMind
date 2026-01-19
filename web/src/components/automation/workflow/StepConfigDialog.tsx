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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Badge } from '@/components/ui/badge'
import { Zap, Clock, Bell, Database, Code, Globe, FileText, ImageIcon } from 'lucide-react'
import type { WorkflowStep } from '@/types'

interface StepConfigDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  step: WorkflowStep | null
  onSave: (step: WorkflowStep) => void
  resources?: {
    devices: Array<{ id: string; name: string }>
    metrics: string[]
    alertChannels: Array<{ id: string; name: string }>
  }
}

export function StepConfigDialog({ open, onOpenChange, step, onSave, resources }: StepConfigDialogProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [localStep, setLocalStep] = useState<WorkflowStep | null>(null)
  const [activeTab, setActiveTab] = useState<'basic' | 'advanced'>('basic')

  useEffect(() => {
    if (step) {
      setLocalStep(JSON.parse(JSON.stringify(step)))
    } else {
      setLocalStep(null)
    }
  }, [step, open])

  if (!localStep) return null

  const updateLocalStep = (updates: Partial<WorkflowStep>) => {
    setLocalStep({ ...localStep, ...updates } as WorkflowStep)
  }

  const handleSave = () => {
    if (localStep) {
      onSave(localStep)
    }
  }

  const renderBasicConfig = () => {
    switch (localStep!.type) {
      case 'delay':
        return <DelayStepConfig step={localStep} onChange={updateLocalStep} />
      case 'send_alert':
        return <SendAlertStepConfig step={localStep} onChange={updateLocalStep} resources={resources} />
      case 'log':
        return <LogStepConfig step={localStep} onChange={updateLocalStep} />
      case 'http_request':
        return <HttpRequestStepConfig step={localStep} onChange={updateLocalStep} />
      case 'send_command':
        return <SendCommandStepConfig step={localStep} onChange={updateLocalStep} resources={resources} />
      case 'device_query':
        return <DeviceQueryStepConfig step={localStep} onChange={updateLocalStep} resources={resources} />
      case 'wait_for_device_state':
        return <WaitForDeviceStateStepConfig step={localStep} onChange={updateLocalStep} resources={resources} />
      case 'execute_wasm':
        return <ExecuteWasmStepConfig step={localStep} onChange={updateLocalStep} />
      case 'condition':
        return <ConditionStepConfig step={localStep} onChange={updateLocalStep} />
      case 'parallel':
        return <ParallelStepConfig step={localStep} onChange={updateLocalStep} />
      case 'data_query':
        return <DataQueryStepConfig step={localStep} onChange={updateLocalStep} />
      case 'image_process':
        return <ImageProcessStepConfig step={localStep} onChange={updateLocalStep} />
      default:
        return <div className="text-muted-foreground">{t('automation:unknownStepType')}</div>
    }
  }

  const getStepIcon = () => {
    switch (localStep!.type) {
      case 'send_command':
        return <Zap className="h-5 w-5" />
      case 'delay':
        return <Clock className="h-5 w-5" />
      case 'send_alert':
        return <Bell className="h-5 w-5" />
      case 'log':
        return <FileText className="h-5 w-5" />
      case 'http_request':
        return <Globe className="h-5 w-5" />
      case 'device_query':
      case 'data_query':
        return <Database className="h-5 w-5" />
      case 'execute_wasm':
        return <Code className="h-5 w-5" />
      case 'image_process':
        return <ImageIcon className="h-5 w-5" />
      default:
        return null
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {getStepIcon()}
            {t('automation:configureStep', {
              step: t(`automation:steps.${localStep!.type}`),
            })}
          </DialogTitle>
          <DialogDescription>
            {t(`automation:steps.${localStep!.type}Desc`)}
          </DialogDescription>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as any)} className="flex-1 overflow-hidden flex flex-col">
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="basic">{t('automation:basicConfig')}</TabsTrigger>
            <TabsTrigger value="advanced">{t('automation:advancedConfig')}</TabsTrigger>
          </TabsList>

          <div className="flex-1 overflow-y-auto mt-4">
            <TabsContent value="basic" className="m-0">
              {renderBasicConfig()}
            </TabsContent>

            <TabsContent value="advanced" className="m-0 space-y-4">
              <div>
                <Label htmlFor="step-id">{t('automation:stepId')}</Label>
                <Input
                  id="step-id"
                  value={localStep!.id}
                  onChange={(e) => updateLocalStep({ id: e.target.value })}
                  className="font-mono text-sm"
                />
              </div>
              <div className="p-3 bg-muted rounded-md">
                <pre className="text-xs overflow-x-auto">
                  {JSON.stringify(localStep, null, 2)}
                </pre>
              </div>
            </TabsContent>
          </div>
        </Tabs>

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

// Delay Step Configuration
function DelayStepConfig({ step, onChange }: { step: WorkflowStep; onChange: (updates: Partial<WorkflowStep>) => void }) {
  const { t } = useTranslation('automation')
  const delayStep = step as any

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="duration">{t('automation:durationSeconds')}</Label>
        <Input
          id="duration"
          type="number"
          value={delayStep.duration_seconds || 5}
          onChange={(e) => onChange({ duration_seconds: parseInt(e.target.value) || 5 })}
          min={0}
          max={86400}
        />
        <p className="text-xs text-muted-foreground mt-1">
          {t('automation:delayHint')}
        </p>
      </div>
    </div>
  )
}

// Send Alert Step Configuration
function SendAlertStepConfig({
  step,
  onChange,
  resources,
}: {
  step: WorkflowStep
  onChange: (updates: Partial<WorkflowStep>) => void
  resources?: { alertChannels: Array<{ id: string; name: string }> }
}) {
  const { t } = useTranslation('automation')
  const alertStep = step as any
  const channels = resources?.alertChannels || []

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="severity">{t('automation:severity')}</Label>
        <Select
          value={alertStep.severity || 'info'}
          onValueChange={(value) => onChange({ severity: value as 'info' | 'warning' | 'critical' | 'error' })}
        >
          <SelectTrigger id="severity">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="info">{t('automation:severities.info')}</SelectItem>
            <SelectItem value="warning">{t('automation:severities.warning')}</SelectItem>
            <SelectItem value="error">{t('automation:severities.error')}</SelectItem>
            <SelectItem value="critical">{t('automation:severities.critical')}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div>
        <Label htmlFor="title">{t('automation:alertTitle')}</Label>
        <Input
          id="title"
          value={alertStep.title || ''}
          onChange={(e) => onChange({ title: e.target.value })}
          placeholder={t('automation:alertTitlePlaceholder')}
        />
      </div>
      <div>
        <Label htmlFor="message">{t('automation:alertMessage')}</Label>
        <Textarea
          id="message"
          value={alertStep.message || ''}
          onChange={(e) => onChange({ message: e.target.value })}
          placeholder={t('automation:alertMessagePlaceholder')}
          className="min-h-[100px]"
        />
      </div>
      {channels.length > 0 && (
        <div>
          <Label>{t('automation:channels')}</Label>
          <div className="flex flex-wrap gap-2 mt-2">
            {channels.map((channel) => {
              const isSelected = (alertStep.channels || []).includes(channel.id)
              return (
                <Badge
                  key={channel.id}
                  variant={isSelected ? 'default' : 'outline'}
                  className="cursor-pointer"
                  onClick={() => {
                    const current = alertStep.channels || []
                    const updated = isSelected
                      ? current.filter((c: string) => c !== channel.id)
                      : [...current, channel.id]
                    onChange({ channels: updated })
                  }}
                >
                  {channel.name}
                </Badge>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}

// Log Step Configuration
function LogStepConfig({ step, onChange }: { step: WorkflowStep; onChange: (updates: Partial<WorkflowStep>) => void }) {
  const { t } = useTranslation('automation')
  const logStep = step as any

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="log-message">{t('automation:logMessage')}</Label>
        <Textarea
          id="log-message"
          value={logStep.message || ''}
          onChange={(e) => onChange({ message: e.target.value })}
          placeholder={t('automation:logMessagePlaceholder')}
          className="min-h-[80px]"
        />
      </div>
      <div>
        <Label htmlFor="log-level">{t('automation:logLevel')}</Label>
        <Select
          value={logStep.level || 'info'}
          onValueChange={(value) => onChange({ level: value as 'debug' | 'info' | 'warn' | 'error' })}
        >
          <SelectTrigger id="log-level">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="debug">{t('automation:logLevels.debug')}</SelectItem>
            <SelectItem value="info">{t('automation:logLevels.info')}</SelectItem>
            <SelectItem value="warn">{t('automation:logLevels.warn')}</SelectItem>
            <SelectItem value="error">{t('automation:logLevels.error')}</SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}

// HTTP Request Step Configuration
function HttpRequestStepConfig({ step, onChange }: { step: WorkflowStep; onChange: (updates: Partial<WorkflowStep>) => void }) {
  const { t } = useTranslation('automation')
  const httpStep = step as any

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="http-method">{t('automation:method')}</Label>
        <Select
          value={httpStep.method || 'GET'}
          onValueChange={(value) => onChange({ method: value as 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' })}
        >
          <SelectTrigger id="http-method">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="GET">GET</SelectItem>
            <SelectItem value="POST">POST</SelectItem>
            <SelectItem value="PUT">PUT</SelectItem>
            <SelectItem value="DELETE">DELETE</SelectItem>
            <SelectItem value="PATCH">PATCH</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div>
        <Label htmlFor="http-url">{t('automation:url')}</Label>
        <Input
          id="http-url"
          value={httpStep.url || ''}
          onChange={(e) => onChange({ url: e.target.value })}
          placeholder="https://api.example.com/endpoint"
        />
      </div>
      <div>
        <Label htmlFor="http-headers">{t('automation:headers')}</Label>
        <Textarea
          id="http-headers"
          value={Object.entries(httpStep.headers || {})
            .map(([k, v]) => `${k}: ${v}`)
            .join('\n')}
          onChange={(e) => {
            const lines = e.target.value.split('\n')
            const headers: Record<string, string> = {}
            lines.forEach((line) => {
              const [key, ...valueParts] = line.split(':')
              if (key && valueParts.length > 0) {
                headers[key.trim()] = valueParts.join(':').trim()
              }
            })
            onChange({ headers })
          }}
          placeholder="Authorization: Bearer token&#10;Content-Type: application/json"
          className="min-h-[80px] font-mono text-sm"
        />
      </div>
      <div>
        <Label htmlFor="http-body">{t('automation:body')}</Label>
        <Textarea
          id="http-body"
          value={httpStep.body || ''}
          onChange={(e) => onChange({ body: e.target.value })}
          placeholder='{"key": "value"}'
          className="min-h-[100px] font-mono text-sm"
        />
      </div>
    </div>
  )
}

// Send Command Step Configuration
function SendCommandStepConfig({
  step,
  onChange,
  resources,
}: {
  step: WorkflowStep
  onChange: (updates: Partial<WorkflowStep>) => void
  resources?: { devices: Array<{ id: string; name: string }> }
}) {
  const { t } = useTranslation('automation')
  const cmdStep = step as any
  const devices = resources?.devices || []

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="cmd-device">{t('automation:device')}</Label>
        <Select
          value={cmdStep.device_id || ''}
          onValueChange={(value) => onChange({ device_id: value })}
        >
          <SelectTrigger id="cmd-device">
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
        <Label htmlFor="cmd-name">{t('automation:command')}</Label>
        <Input
          id="cmd-name"
          value={cmdStep.command || ''}
          onChange={(e) => onChange({ command: e.target.value })}
          placeholder="turn_on"
        />
      </div>
      <div>
        <Label htmlFor="cmd-params">{t('automation:parameters')}</Label>
        <Textarea
          id="cmd-params"
          value={JSON.stringify(cmdStep.parameters || {}, null, 2)}
          onChange={(e) => {
            try {
              onChange({ parameters: JSON.parse(e.target.value) })
            } catch {
              // Invalid JSON, ignore
            }
          }}
          placeholder='{"brightness": 100}'
          className="min-h-[100px] font-mono text-sm"
        />
      </div>
    </div>
  )
}

// Device Query Step Configuration
function DeviceQueryStepConfig({
  step,
  onChange,
  resources,
}: {
  step: WorkflowStep
  onChange: (updates: Partial<WorkflowStep>) => void
  resources?: { devices: Array<{ id: string; name: string }> }
}) {
  const { t } = useTranslation('automation')
  const queryStep = step as any
  const devices = resources?.devices || []

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="query-device">{t('automation:device')}</Label>
        <Select
          value={queryStep.device_id || ''}
          onValueChange={(value) => onChange({ device_id: value })}
        >
          <SelectTrigger id="query-device">
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
        <Label htmlFor="query-metric">{t('automation:metric')}</Label>
        <Input
          id="query-metric"
          value={queryStep.metric || ''}
          onChange={(e) => onChange({ metric: e.target.value })}
          placeholder="temperature"
        />
      </div>
      <div>
        <Label htmlFor="query-aggregation">{t('automation:aggregation')}</Label>
        <Select
          value={queryStep.aggregation || 'last'}
          onValueChange={(value) => onChange({ aggregation: value })}
        >
          <SelectTrigger id="query-aggregation">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="last">{t('automation:aggregations.last')}</SelectItem>
            <SelectItem value="avg">{t('automation:aggregations.avg')}</SelectItem>
            <SelectItem value="min">{t('automation:aggregations.min')}</SelectItem>
            <SelectItem value="max">{t('automation:aggregations.max')}</SelectItem>
            <SelectItem value="sum">{t('automation:aggregations.sum')}</SelectItem>
            <SelectItem value="count">{t('automation:aggregations.count')}</SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}

// Wait For Device State Step Configuration
function WaitForDeviceStateStepConfig({
  step,
  onChange,
  resources,
}: {
  step: WorkflowStep
  onChange: (updates: Partial<WorkflowStep>) => void
  resources?: { devices: Array<{ id: string; name: string }> }
}) {
  const { t } = useTranslation('automation')
  const waitStep = step as any
  const devices = resources?.devices || []

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="wait-device">{t('automation:device')}</Label>
        <Select
          value={waitStep.device_id || ''}
          onValueChange={(value) => onChange({ device_id: value })}
        >
          <SelectTrigger id="wait-device">
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
        <Label htmlFor="wait-metric">{t('automation:metric')}</Label>
        <Input
          id="wait-metric"
          value={waitStep.metric || ''}
          onChange={(e) => onChange({ metric: e.target.value })}
          placeholder="temperature"
        />
      </div>
      <div>
        <Label htmlFor="wait-value">{t('automation:expectedValue')}</Label>
        <Input
          id="wait-value"
          type="number"
          value={waitStep.expected_value ?? 0}
          onChange={(e) => onChange({ expected_value: parseFloat(e.target.value) || 0 })}
        />
      </div>
      <div>
        <Label htmlFor="wait-tolerance">{t('automation:tolerance')}</Label>
        <Input
          id="wait-tolerance"
          type="number"
          value={waitStep.tolerance ?? 0.1}
          onChange={(e) => onChange({ tolerance: parseFloat(e.target.value) || 0 })}
          step={0.01}
        />
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <Label htmlFor="wait-timeout">{t('automation:timeoutSeconds')}</Label>
          <Input
            id="wait-timeout"
            type="number"
            value={waitStep.timeout_seconds ?? 60}
            onChange={(e) => onChange({ timeout_seconds: parseInt(e.target.value) || 60 })}
            min={1}
            max={3600}
          />
        </div>
        <div>
          <Label htmlFor="wait-poll">{t('automation:pollInterval')}</Label>
          <Input
            id="wait-poll"
            type="number"
            value={waitStep.poll_interval_seconds ?? 5}
            onChange={(e) => onChange({ poll_interval_seconds: parseInt(e.target.value) || 5 })}
            min={1}
            max={60}
          />
        </div>
      </div>
    </div>
  )
}

// Execute WASM Step Configuration
function ExecuteWasmStepConfig({ step, onChange }: { step: WorkflowStep; onChange: (updates: Partial<WorkflowStep>) => void }) {
  const { t } = useTranslation('automation')
  const wasmStep = step as any

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="wasm-module">{t('automation:wasmModule')}</Label>
        <Input
          id="wasm-module"
          value={wasmStep.module_id || ''}
          onChange={(e) => onChange({ module_id: e.target.value })}
          placeholder="my-module"
        />
      </div>
      <div>
        <Label htmlFor="wasm-function">{t('automation:functionName')}</Label>
        <Input
          id="wasm-function"
          value={wasmStep.function || ''}
          onChange={(e) => onChange({ function: e.target.value })}
          placeholder="process"
        />
      </div>
      <div>
        <Label htmlFor="wasm-args">{t('automation:arguments')}</Label>
        <Textarea
          id="wasm-args"
          value={JSON.stringify(wasmStep.arguments || {}, null, 2)}
          onChange={(e) => {
            try {
              onChange({ arguments: JSON.parse(e.target.value) })
            } catch {
              // Invalid JSON, ignore
            }
          }}
          placeholder='{"input": "value"}'
          className="min-h-[100px] font-mono text-sm"
        />
      </div>
    </div>
  )
}

// Condition Step Configuration
function ConditionStepConfig({ step, onChange }: { step: WorkflowStep; onChange: (updates: Partial<WorkflowStep>) => void }) {
  const { t } = useTranslation('automation')
  const condStep = step as any

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="condition-expression">{t('automation:condition')}</Label>
        <Input
          id="condition-expression"
          value={condStep.condition || ''}
          onChange={(e) => onChange({ condition: e.target.value })}
          placeholder="${temperature} > 25"
          className="font-mono"
        />
        <p className="text-xs text-muted-foreground mt-1">
          {t('automation:conditionHint')}
        </p>
      </div>
      <div className="p-3 bg-muted rounded-md">
        <p className="text-sm font-medium mb-2">{t('automation:note')}</p>
        <p className="text-xs text-muted-foreground">
          {t('automation:conditionNote')}
        </p>
      </div>
    </div>
  )
}

// Parallel Step Configuration
function ParallelStepConfig({ step, onChange }: { step: WorkflowStep; onChange: (updates: Partial<WorkflowStep>) => void }) {
  const { t } = useTranslation('automation')
  const parallelStep = step as any

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="parallel-max">{t('automation:maxParallel')}</Label>
        <Input
          id="parallel-max"
          type="number"
          value={parallelStep.max_parallel || ''}
          onChange={(e) => onChange({ max_parallel: parseInt(e.target.value) || undefined })}
          placeholder={t('automation:unlimited')}
          min={1}
        />
        <p className="text-xs text-muted-foreground mt-1">
          {t('automation:parallelHint')}
        </p>
      </div>
      <div className="p-3 bg-muted rounded-md">
        <p className="text-sm text-muted-foreground">
          {t('automation:parallelNote')}
        </p>
      </div>
    </div>
  )
}

// Data Query Step Configuration
function DataQueryStepConfig({ step, onChange }: { step: WorkflowStep; onChange: (updates: Partial<WorkflowStep>) => void }) {
  const { t } = useTranslation('automation')
  const queryStep = step as any

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="data-query-type">{t('automation:queryType')}</Label>
        <Select
          value={queryStep.query_type || 'telemetry'}
          onValueChange={(value) => onChange({ query_type: value as 'telemetry' | 'history' | 'aggregate' })}
        >
          <SelectTrigger id="data-query-type">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="telemetry">{t('automation:queryTypes.telemetry')}</SelectItem>
            <SelectItem value="history">{t('automation:queryTypes.history')}</SelectItem>
            <SelectItem value="aggregate">{t('automation:queryTypes.aggregate')}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div>
        <Label htmlFor="data-query-params">{t('automation:parameters')}</Label>
        <Textarea
          id="data-query-params"
          value={JSON.stringify(queryStep.parameters || {}, null, 2)}
          onChange={(e) => {
            try {
              onChange({ parameters: JSON.parse(e.target.value) })
            } catch {
              // Invalid JSON, ignore
            }
          }}
          placeholder='{"device_id": "xxx", "metric": "temperature"}'
          className="min-h-[100px] font-mono text-sm"
        />
      </div>
    </div>
  )
}

// Image Process Step Configuration
function ImageProcessStepConfig({ step, onChange }: { step: WorkflowStep; onChange: (updates: Partial<WorkflowStep>) => void }) {
  const { t } = useTranslation('automation')
  const imgStep = step as any

  return (
    <div className="space-y-4">
      <div>
        <Label htmlFor="img-source">{t('automation:imageSource')}</Label>
        <Input
          id="img-source"
          value={imgStep.image_source || ''}
          onChange={(e) => onChange({ image_source: e.target.value })}
          placeholder="${device_id}/snapshot"
        />
      </div>
      <div>
        <Label htmlFor="img-format">{t('automation:outputFormat')}</Label>
        <Select
          value={imgStep.output_format || 'jpeg'}
          onValueChange={(value) => onChange({ output_format: value })}
        >
          <SelectTrigger id="img-format">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="jpeg">JPEG</SelectItem>
            <SelectItem value="png">PNG</SelectItem>
            <SelectItem value="webp">WebP</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div>
        <Label>{t('automation:operations')}</Label>
        <div className="p-3 bg-muted rounded-md">
          <p className="text-sm text-muted-foreground">
            {t('automation:imageOpsHint')}
          </p>
        </div>
      </div>
    </div>
  )
}
