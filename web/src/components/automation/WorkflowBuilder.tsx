import { useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Switch } from '@/components/ui/switch'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  ArrowRight,
  Plus,
  Trash2,
  GripVertical,
  Play,
  Save,
  Clock,
  AlertTriangle,
  Database,
  Globe,
  Code,
  Split,
  GitBranch,
  Zap,
  Bell,
  Image as ImageIcon,
  FileText,
  Pause,
  Braces,
} from 'lucide-react'
import type {
  Workflow,
  WorkflowStep,
  WorkflowStepType,
  WorkflowTrigger,
  WorkflowTriggerType,
} from '@/types'
import { StepConfigDialog } from './workflow/StepConfigDialog'
import { TriggerConfigDialog } from './workflow/TriggerConfigDialog'

interface WorkflowBuilderProps {
  workflow?: Workflow
  onSave: (workflow: Partial<Workflow>) => Promise<void>
  onCancel: () => void
  resources?: {
    devices: Array<{ id: string; name: string }>
    metrics: string[]
    alertChannels: Array<{ id: string; name: string }>
  }
}

// Step type definitions with icons and labels
const STEP_TYPES: Array<{
  type: WorkflowStepType
  icon: React.ReactNode
  label: string
  description: string
  category: 'device' | 'logic' | 'action' | 'advanced'
}> = [
  {
    type: 'send_command',
    icon: <Zap className="h-4 w-4" />,
    label: 'automation:steps.sendCommand',
    description: 'automation:steps.sendCommandDesc',
    category: 'device',
  },
  {
    type: 'device_query',
    icon: <Database className="h-4 w-4" />,
    label: 'automation:steps.deviceQuery',
    description: 'automation:steps.deviceQueryDesc',
    category: 'device',
  },
  {
    type: 'wait_for_device_state',
    icon: <Clock className="h-4 w-4" />,
    label: 'automation:steps.waitForDeviceState',
    description: 'automation:steps.waitForDeviceStateDesc',
    category: 'device',
  },
  {
    type: 'condition',
    icon: <GitBranch className="h-4 w-4" />,
    label: 'automation:steps.condition',
    description: 'automation:steps.conditionDesc',
    category: 'logic',
  },
  {
    type: 'delay',
    icon: <Pause className="h-4 w-4" />,
    label: 'automation:steps.delay',
    description: 'automation:steps.delayDesc',
    category: 'logic',
  },
  {
    type: 'parallel',
    icon: <Split className="h-4 w-4" />,
    label: 'automation:steps.parallel',
    description: 'automation:steps.parallelDesc',
    category: 'logic',
  },
  {
    type: 'send_alert',
    icon: <Bell className="h-4 w-4" />,
    label: 'automation:steps.sendAlert',
    description: 'automation:steps.sendAlertDesc',
    category: 'action',
  },
  {
    type: 'log',
    icon: <FileText className="h-4 w-4" />,
    label: 'automation:steps.log',
    description: 'automation:steps.logDesc',
    category: 'action',
  },
  {
    type: 'http_request',
    icon: <Globe className="h-4 w-4" />,
    label: 'automation:steps.httpRequest',
    description: 'automation:steps.httpRequestDesc',
    category: 'action',
  },
  {
    type: 'data_query',
    icon: <Database className="h-4 w-4" />,
    label: 'automation:steps.dataQuery',
    description: 'automation:steps.dataQueryDesc',
    category: 'advanced',
  },
  {
    type: 'execute_wasm',
    icon: <Code className="h-4 w-4" />,
    label: 'automation:steps.executeWasm',
    description: 'automation:steps.executeWasmDesc',
    category: 'advanced',
  },
  {
    type: 'image_process',
    icon: <ImageIcon className="h-4 w-4" />,
    label: 'automation:steps.imageProcess',
    description: 'automation:steps.imageProcessDesc',
    category: 'advanced',
  },
]

// Trigger type definitions
const TRIGGER_TYPES: Array<{
  type: WorkflowTriggerType
  icon: React.ReactNode
  label: string
  description: string
}> = [
  {
    type: 'manual',
    icon: <Play className="h-4 w-4" />,
    label: 'automation:triggers.manual',
    description: 'automation:triggers.manualDesc',
  },
  {
    type: 'cron',
    icon: <Clock className="h-4 w-4" />,
    label: 'automation:triggers.cron',
    description: 'automation:triggers.cronDesc',
  },
  {
    type: 'event',
    icon: <AlertTriangle className="h-4 w-4" />,
    label: 'automation:triggers.event',
    description: 'automation:triggers.eventDesc',
  },
  {
    type: 'device',
    icon: <Zap className="h-4 w-4" />,
    label: 'automation:triggers.device',
    description: 'automation:triggers.deviceDesc',
  },
]

interface WorkflowFormData {
  id: string
  name: string
  description: string
  enabled: boolean
  steps: WorkflowStep[]
  triggers: WorkflowTrigger[]
  variables: Record<string, unknown>
  timeout_seconds: number
}

export function WorkflowBuilder({ workflow, onSave, onCancel, resources }: WorkflowBuilderProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [formData, setFormData] = useState<WorkflowFormData>({
    id: workflow?.id || `workflow-${Date.now()}`,
    name: workflow?.name || '',
    description: workflow?.description || '',
    enabled: workflow?.enabled ?? true,
    steps: (workflow?.steps as WorkflowStep[]) || [],
    triggers: (workflow?.triggers as WorkflowTrigger[]) || [{ type: 'manual', id: 'trigger-manual' }],
    variables: (workflow?.variables as Record<string, unknown>) || {},
    timeout_seconds: 300,
  })
  const [saving, setSaving] = useState(false)
  const [activeTab, setActiveTab] = useState<'steps' | 'triggers' | 'variables' | 'settings'>('steps')

  // Step configuration dialog state
  const [stepDialogOpen, setStepDialogOpen] = useState(false)
  const [editingStep, setEditingStep] = useState<WorkflowStep | null>(null)
  const [stepIndex, setStepIndex] = useState<number | null>(null)

  // Trigger configuration dialog state
  const [triggerDialogOpen, setTriggerDialogOpen] = useState(false)
  const [editingTrigger, setEditingTrigger] = useState<WorkflowTrigger | null>(null)
  const [triggerIndex, setTriggerIndex] = useState<number | null>(null)

  // New variable state
  const [newVarName, setNewVarName] = useState('')
  const [newVarValue, setNewVarValue] = useState('')

  const updateFormData = useCallback((updates: Partial<WorkflowFormData>) => {
    setFormData((prev) => ({ ...prev, ...updates }))
  }, [])

  // Generate a unique step ID
  const generateStepId = useCallback(() => {
    return `step-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
  }, [])

  // Add a new step
  const handleAddStep = useCallback(
    (type: WorkflowStepType) => {
      const newStep: WorkflowStep = {
        id: generateStepId(),
        type,
      } as WorkflowStep

      // Set default values based on step type
      switch (type) {
        case 'delay':
          (newStep as DelayStep).duration_seconds = 5
          break
        case 'send_alert':
          ;(newStep as SendAlertStep).severity = 'info'
          ;(newStep as SendAlertStep).title = ''
          ;(newStep as SendAlertStep).message = ''
          break
        case 'log':
          ;(newStep as LogStep).message = ''
          ;(newStep as LogStep).level = 'info'
          break
        case 'condition':
          ;(newStep as ConditionStep).condition = ''
          ;(newStep as ConditionStep).then_steps = []
          ;(newStep as ConditionStep).else_steps = []
          break
        case 'parallel':
          ;(newStep as ParallelStep).steps = []
          break
        case 'http_request':
          ;(newStep as HttpRequestStep).method = 'GET'
          ;(newStep as HttpRequestStep).url = ''
          break
        case 'send_command':
          ;(newStep as SendCommandStep).device_id = ''
          ;(newStep as SendCommandStep).command = ''
          ;(newStep as SendCommandStep).parameters = {}
          break
        case 'device_query':
          ;(newStep as DeviceQueryStep).device_id = ''
          ;(newStep as DeviceQueryStep).metric = ''
          break
        case 'wait_for_device_state':
          ;(newStep as WaitForDeviceStateStep).device_id = ''
          ;(newStep as WaitForDeviceStateStep).metric = ''
          ;(newStep as WaitForDeviceStateStep).expected_value = 0
          ;(newStep as WaitForDeviceStateStep).timeout_seconds = 60
          ;(newStep as WaitForDeviceStateStep).poll_interval_seconds = 5
          break
      }

      setEditingStep(newStep)
      setStepIndex(null)
      setStepDialogOpen(true)
    },
    [generateStepId]
  )

  // Edit an existing step
  const handleEditStep = useCallback((index: number) => {
    setEditingStep(formData.steps[index])
    setStepIndex(index)
    setStepDialogOpen(true)
  }, [formData.steps])

  // Delete a step
  const handleDeleteStep = useCallback((index: number) => {
    setFormData((prev) => ({
      ...prev,
      steps: prev.steps.filter((_, i) => i !== index),
    }))
  }, [])

  // Move a step up
  const handleMoveStepUp = useCallback((index: number) => {
    if (index === 0) return
    setFormData((prev) => {
      const steps = [...prev.steps]
      ;[steps[index - 1], steps[index]] = [steps[index], steps[index - 1]]
      return { ...prev, steps }
    })
  }, [])

  // Move a step down
  const handleMoveStepDown = useCallback((index: number) => {
    if (index >= formData.steps.length - 1) return
    setFormData((prev) => {
      const steps = [...prev.steps]
      ;[steps[index], steps[index + 1]] = [steps[index + 1], steps[index]]
      return { ...prev, steps }
    })
  }, [formData.steps.length])

  // Save step from dialog
  const handleSaveStep = useCallback((step: WorkflowStep) => {
    setFormData((prev) => {
      const steps = [...prev.steps]
      if (stepIndex !== null) {
        steps[stepIndex] = step
      } else {
        steps.push(step)
      }
      return { ...prev, steps }
    })
    setStepDialogOpen(false)
    setEditingStep(null)
    setStepIndex(null)
  }, [stepIndex])

  // Add a new trigger
  const handleAddTrigger = useCallback((type: WorkflowTriggerType) => {
    const newTrigger: WorkflowTrigger = {
      id: `trigger-${Date.now()}`,
      type,
    } as WorkflowTrigger

    // Set default values based on trigger type
    switch (type) {
      case 'cron':
        ;(newTrigger as CronTrigger).expression = '0 * * * *'
        break
      case 'event':
        ;(newTrigger as EventTrigger).event_type = ''
        break
      case 'device':
        ;(newTrigger as DeviceTrigger).device_id = ''
        ;(newTrigger as DeviceTrigger).metric = ''
        ;(newTrigger as DeviceTrigger).condition = '>'
        break
    }

    setEditingTrigger(newTrigger)
    setTriggerIndex(null)
    setTriggerDialogOpen(true)
  }, [])

  // Edit an existing trigger
  const handleEditTrigger = useCallback((index: number) => {
    setEditingTrigger(formData.triggers[index])
    setTriggerIndex(index)
    setTriggerDialogOpen(true)
  }, [formData.triggers])

  // Delete a trigger
  const handleDeleteTrigger = useCallback((index: number) => {
    setFormData((prev) => ({
      ...prev,
      triggers: prev.triggers.filter((_, i) => i !== index),
    }))
  }, [])

  // Save trigger from dialog
  const handleSaveTrigger = useCallback((trigger: WorkflowTrigger) => {
    setFormData((prev) => {
      const triggers = [...prev.triggers]
      if (triggerIndex !== null) {
        triggers[triggerIndex] = trigger
      } else {
        triggers.push(trigger)
      }
      return { ...prev, triggers }
    })
    setTriggerDialogOpen(false)
    setEditingTrigger(null)
    setTriggerIndex(null)
  }, [triggerIndex])

  // Add a variable
  const handleAddVariable = useCallback(() => {
    if (!newVarName.trim()) return
    setFormData((prev) => ({
      ...prev,
      variables: {
        ...prev.variables,
        [newVarName]: newVarValue,
      },
    }))
    setNewVarName('')
    setNewVarValue('')
  }, [newVarName, newVarValue])

  // Delete a variable
  const handleDeleteVariable = useCallback((key: string) => {
    setFormData((prev) => {
      const vars = { ...prev.variables }
      delete vars[key]
      return { ...prev, variables: vars }
    })
  }, [])

  // Get step icon
  const getStepIcon = (type: WorkflowStepType) => {
    return STEP_TYPES.find((s) => s.type === type)?.icon || <Braces className="h-4 w-4" />
  }

  // Get step label
  const getStepLabel = (step: WorkflowStep) => {
    const stepType = STEP_TYPES.find((s) => s.type === step.type)
    if (!stepType) return t(`automation:steps.${step.type}`)

    // Add additional info based on step type
    switch (step.type) {
      case 'send_command':
        return `${t(stepType.label)} (${ (step as SendCommandStep).command || 'N/A' })`
      case 'delay':
        return `${t(stepType.label)} (${ (step as DelayStep).duration_seconds }s)`
      case 'send_alert':
        return `${t(stepType.label)} (${ (step as SendAlertStep).title || 'N/A' })`
      case 'log':
        return `${t(stepType.label)}: ${(step as LogStep).message?.substring(0, 30) || ''}...`
      case 'condition':
        return `${t(stepType.label)} (${ (step as ConditionStep).condition || 'N/A' })`
      case 'http_request':
        return `${t(stepType.label)} (${ (step as HttpRequestStep).method })`
      case 'device_query':
        return `${t(stepType.label)} (${ (step as DeviceQueryStep).metric || 'N/A' })`
      default:
        return t(stepType.label)
    }
  }

  // Get trigger icon
  const getTriggerIcon = (type: WorkflowTriggerType) => {
    return TRIGGER_TYPES.find((t) => t.type === type)?.icon || <Clock className="h-4 w-4" />
  }

  // Get trigger label
  const getTriggerLabel = (trigger: WorkflowTrigger) => {
    switch (trigger.type) {
      case 'manual':
        return t('automation:triggers.manual')
      case 'cron':
        return `${t('automation:triggers.cron')}: ${(trigger as CronTrigger).expression}`
      case 'event':
        return `${t('automation:triggers.event')}: ${(trigger as EventTrigger).event_type || 'N/A'}`
      case 'device':
        return `${t('automation:triggers.device')}: ${(trigger as DeviceTrigger).metric || 'N/A'}`
    }
  }

  // Validate and save
  const handleSave = useCallback(async () => {
    if (!formData.name.trim()) {
      return
    }

    if (formData.steps.length === 0) {
      return
    }

    setSaving(true)
    try {
      await onSave({
        id: formData.id,
        name: formData.name,
        description: formData.description,
        enabled: formData.enabled,
        steps: formData.steps,
        triggers: formData.triggers,
        variables: formData.variables,
        timeout_seconds: formData.timeout_seconds,
      })
    } finally {
      setSaving(false)
    }
  }, [formData, onSave])

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">
            {workflow ? t('automation:editWorkflow') : t('automation:createWorkflow')}
          </h2>
          <p className="text-sm text-muted-foreground">
            {t('automation:workflowBuilderDesc')}
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={onCancel} disabled={saving}>
            {t('common:cancel')}
          </Button>
          <Button onClick={handleSave} disabled={saving || !formData.name.trim()}>
            {saving ? (
              <>
                <Pause className="mr-2 h-4 w-4 animate-spin" />
                {t('common:saving')}
              </>
            ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                {t('common:save')}
              </>
            )}
          </Button>
        </div>
      </div>

      {/* Basic Info */}
      <Card className="p-4 space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <Label htmlFor="workflow-name">{t('automation:workflowName')}</Label>
            <Input
              id="workflow-name"
              value={formData.name}
              onChange={(e) => updateFormData({ name: e.target.value })}
              placeholder={t('automation:workflowNamePlaceholder')}
            />
          </div>
          <div className="flex items-center gap-4">
            <Switch
              id="workflow-enabled"
              checked={formData.enabled}
              onCheckedChange={(enabled) => updateFormData({ enabled })}
            />
            <Label htmlFor="workflow-enabled">{t('automation:enabled')}</Label>
          </div>
        </div>
        <div>
          <Label htmlFor="workflow-description">{t('common:description')}</Label>
          <Textarea
            id="workflow-description"
            value={formData.description}
            onChange={(e) => updateFormData({ description: e.target.value })}
            placeholder={t('automation:workflowDescriptionPlaceholder')}
            className="min-h-[80px]"
          />
        </div>
      </Card>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as any)}>
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="steps">{t('automation:steps')}</TabsTrigger>
          <TabsTrigger value="triggers">{t('automation:triggers')}</TabsTrigger>
          <TabsTrigger value="variables">{t('automation:variables')}</TabsTrigger>
          <TabsTrigger value="settings">{t('common:settings')}</TabsTrigger>
        </TabsList>

        {/* Steps Tab */}
        <TabsContent value="steps" className="space-y-4">
          <Card className="p-4">
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold">{t('automation:workflowSteps')}</h3>
              <Badge variant="outline">{formData.steps.length} steps</Badge>
            </div>

            {/* Step Type Selector */}
            <div className="mb-4">
              <Label className="mb-2 block">{t('automation:addStep')}</Label>
              <div className="flex flex-wrap gap-2">
                {STEP_TYPES.map((stepType) => (
                  <Button
                    key={stepType.type}
                    variant="outline"
                    size="sm"
                    onClick={() => handleAddStep(stepType.type)}
                    className="flex items-center gap-1"
                  >
                    {stepType.icon}
                    <span className="hidden sm:inline">{t(stepType.label)}</span>
                  </Button>
                ))}
              </div>
            </div>

            {/* Steps List */}
            {formData.steps.length === 0 ? (
              <div className="text-center py-12 text-muted-foreground border-2 border-dashed rounded-lg">
                <Braces className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p>{t('automation:noSteps')}</p>
                <p className="text-sm">{t('automation:noStepsDesc')}</p>
              </div>
            ) : (
              <div className="space-y-2">
                {formData.steps.map((step, index) => (
                  <div
                    key={step.id}
                    className="flex items-center gap-2 p-3 bg-muted/30 rounded-lg border group"
                  >
                    <GripVertical className="h-5 w-5 text-muted-foreground cursor-move" />
                    <div className="flex items-center gap-2 px-2 py-1 bg-background rounded">
                      {getStepIcon(step.type)}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">{getStepLabel(step)}</div>
                      <div className="text-xs text-muted-foreground truncate">
                        {t(`automation:steps.${step.type}Desc`)}
                      </div>
                    </div>
                    <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7"
                        onClick={() => handleMoveStepUp(index)}
                        disabled={index === 0}
                      >
                        ↑
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7"
                        onClick={() => handleMoveStepDown(index)}
                        disabled={index >= formData.steps.length - 1}
                      >
                        ↓
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7"
                        onClick={() => handleEditStep(index)}
                      >
                        ✏️
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 text-destructive"
                        onClick={() => handleDeleteStep(index)}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                    {index < formData.steps.length - 1 && (
                      <ArrowRight className="h-4 w-4 text-muted-foreground" />
                    )}
                  </div>
                ))}
              </div>
            )}
          </Card>
        </TabsContent>

        {/* Triggers Tab */}
        <TabsContent value="triggers" className="space-y-4">
          <Card className="p-4">
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold">{t('automation:workflowTriggers')}</h3>
              <Badge variant="outline">{formData.triggers.length} triggers</Badge>
            </div>

            {/* Trigger Type Selector */}
            <div className="mb-4">
              <Label className="mb-2 block">{t('automation:addTrigger')}</Label>
              <div className="flex flex-wrap gap-2">
                {TRIGGER_TYPES.map((triggerType) => (
                  <Button
                    key={triggerType.type}
                    variant="outline"
                    size="sm"
                    onClick={() => handleAddTrigger(triggerType.type)}
                    className="flex items-center gap-1"
                  >
                    {triggerType.icon}
                    <span className="hidden sm:inline">{t(triggerType.label)}</span>
                  </Button>
                ))}
              </div>
            </div>

            {/* Triggers List */}
            <div className="space-y-2">
              {formData.triggers.map((trigger, index) => (
                <div
                  key={trigger.id}
                  className="flex items-center gap-2 p-3 bg-muted/30 rounded-lg border group"
                >
                  <div className="flex items-center gap-2 px-2 py-1 bg-background rounded">
                    {getTriggerIcon(trigger.type)}
                  </div>
                  <div className="flex-1">
                    <div className="font-medium">{getTriggerLabel(trigger)}</div>
                  </div>
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7"
                      onClick={() => handleEditTrigger(index)}
                    >
                      ✏️
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 text-destructive"
                      onClick={() => handleDeleteTrigger(index)}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </TabsContent>

        {/* Variables Tab */}
        <TabsContent value="variables" className="space-y-4">
          <Card className="p-4">
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold">{t('automation:variables')}</h3>
              <Badge variant="outline">{Object.keys(formData.variables).length} variables</Badge>
            </div>

            {/* Add Variable */}
            <div className="flex gap-2 mb-4">
              <Input
                placeholder={t('automation:variableName')}
                value={newVarName}
                onChange={(e) => setNewVarName(e.target.value)}
              />
              <Input
                placeholder={t('automation:variableValue')}
                value={newVarValue}
                onChange={(e) => setNewVarValue(e.target.value)}
              />
              <Button onClick={handleAddVariable} disabled={!newVarName.trim()}>
                <Plus className="h-4 w-4" />
              </Button>
            </div>

            {/* Variables List */}
            <div className="space-y-2">
              {Object.entries(formData.variables).map(([key, value]) => (
                <div
                  key={key}
                  className="flex items-center gap-2 p-3 bg-muted/30 rounded-lg border"
                >
                  <div className="flex-1 font-mono text-sm">
                    <span className="text-blue-500">{key}</span>
                    <span className="text-muted-foreground mx-2">=</span>
                    <span>{JSON.stringify(value)}</span>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 text-destructive"
                    onClick={() => handleDeleteVariable(key)}
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              ))}
              {Object.keys(formData.variables).length === 0 && (
                <div className="text-center py-8 text-muted-foreground">
                  {t('automation:noVariables')}
                </div>
              )}
            </div>
          </Card>
        </TabsContent>

        {/* Settings Tab */}
        <TabsContent value="settings" className="space-y-4">
          <Card className="p-4">
            <h3 className="font-semibold mb-4">{t('automation:workflowSettings')}</h3>
            <div className="space-y-4">
              <div>
                <Label htmlFor="timeout">{t('automation:timeoutSeconds')}</Label>
                <Input
                  id="timeout"
                  type="number"
                  value={formData.timeout_seconds}
                  onChange={(e) => updateFormData({ timeout_seconds: parseInt(e.target.value) || 300 })}
                  min={1}
                  max={3600}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  {t('automation:timeoutDesc')}
                </p>
              </div>
            </div>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Step Configuration Dialog */}
      <StepConfigDialog
        open={stepDialogOpen}
        onOpenChange={setStepDialogOpen}
        step={editingStep}
        onSave={handleSaveStep}
        resources={resources}
      />

      {/* Trigger Configuration Dialog */}
      <TriggerConfigDialog
        open={triggerDialogOpen}
        onOpenChange={setTriggerDialogOpen}
        trigger={editingTrigger}
        onSave={handleSaveTrigger}
        resources={resources}
      />
    </div>
  )
}

// Import delay step type reference
type DelayStep = import('@/types').DelayStep
type SendAlertStep = import('@/types').SendAlertStep
type LogStep = import('@/types').LogStep
type ConditionStep = import('@/types').ConditionStep
type ParallelStep = import('@/types').ParallelStep
type HttpRequestStep = import('@/types').HttpRequestStep
type SendCommandStep = import('@/types').SendCommandStep
type DeviceQueryStep = import('@/types').DeviceQueryStep
type WaitForDeviceStateStep = import('@/types').WaitForDeviceStateStep
type CronTrigger = import('@/types').CronTrigger
type EventTrigger = import('@/types').EventTrigger
type DeviceTrigger = import('@/types').DeviceTrigger
