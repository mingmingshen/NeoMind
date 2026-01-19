import React, { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import {
  ArrowRight,
  Trash2,
  Save,
  Zap,
  Code,
  Sparkles,
  Play,
  X,
  Plus,
  GripVertical,
  Home,
  Gamepad2,
  Timer,
  GitBranch,
  Bell,
  FileText,
  Hand,
  Clock,
  Sun,
  Thermometer,
  Moon,
  Power,
  PowerOff,
  RefreshCw,
  type LucideIcon,
} from 'lucide-react'
import { cn } from '@/lib/utils'

interface WorkflowVisualEditorProps {
  workflow?: any
  onSave: (workflow: any) => Promise<void>
  onCancel: () => void
  resources?: {
    devices: Array<{ id: string; name: string; type: string }>
    metrics: string[]
    alertChannels: Array<{ id: string; name: string }>
  }
}

type EditMode = 'visual' | 'json'
type ViewMode = 'design' | 'config'

// Step/node types with Lucide icons and NeoTalk colors
const NODE_TYPES = [
  { value: 'send_command', label: '控制设备', icon: Gamepad2, color: 'bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-300 border-amber-200' },
  { value: 'delay', label: '等待延迟', icon: Timer, color: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300 border-blue-200' },
  { value: 'condition', label: '条件判断', icon: GitBranch, color: 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300 border-purple-200' },
  { value: 'send_alert', label: '发送通知', icon: Bell, color: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300 border-red-200' },
  { value: 'log', label: '记录日志', icon: FileText, color: 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-300 border-gray-200' },
]

// Trigger types
const TRIGGER_TYPES = [
  { value: 'manual', label: '手动执行', icon: Hand, color: 'bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-300 border-gray-200' },
  { value: 'cron', label: '定时执行', icon: Clock, color: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300 border-blue-200' },
  { value: 'device', label: '设备触发', icon: Zap, color: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300 border-green-200' },
]

interface Trigger {
  id: string
  type: 'manual' | 'cron' | 'device'
  deviceId?: string
  metric?: string
  operator?: string
  threshold?: string | number
  cron?: string
  cronLabel?: string
}

interface Step {
  id: string
  type: string
  name: string
  deviceId?: string
  command?: string
  value?: string | number
  message?: string
  duration?: number
  condition?: string
}

// Workflow templates with Lucide icons
const WORKFLOW_TEMPLATES: Array<{
  name: string
  icon: LucideIcon
  iconColor: string
  iconBg: string
  desc: string
  triggers: Array<{
    id: string
    type: 'manual' | 'cron' | 'device'
    cron?: string
    cronLabel?: string
    metric?: string
    operator?: string
    threshold?: number
  }>
  steps: Array<{
    id: string
    type: string
    name: string
    deviceId?: string
    command?: string
    message?: string
  }>
}> = [
  {
    name: '晨间开灯',
    icon: Sun,
    iconColor: 'text-orange-500',
    iconBg: 'bg-orange-500/10',
    desc: '每天早上7点自动打开灯光',
    triggers: [{ id: 't1', type: 'cron', cron: '0 7 * * *', cronLabel: '每天早上7点' }],
    steps: [{ id: 's1', type: 'send_command', name: '打开客厅灯', deviceId: 'light', command: 'turn_on' }],
  },
  {
    name: '温度控制',
    icon: Thermometer,
    iconColor: 'text-red-500',
    iconBg: 'bg-red-500/10',
    desc: '温度过高时打开空调并发送通知',
    triggers: [{ id: 't1', type: 'device', metric: 'temperature', operator: '>', threshold: 30 }],
    steps: [
      { id: 's1', type: 'send_command', name: '打开空调', deviceId: 'ac', command: 'turn_on' },
      { id: 's2', type: 'send_alert', name: '发送通知', message: '温度过高，已自动开启空调' },
    ],
  },
  {
    name: '夜间模式',
    icon: Moon,
    iconColor: 'text-indigo-500',
    iconBg: 'bg-indigo-500/10',
    desc: '晚上10点关闭所有灯光',
    triggers: [{ id: 't1', type: 'cron', cron: '0 22 * * *', cronLabel: '每天晚上10点' }],
    steps: [{ id: 's1', type: 'send_command', name: '关闭所有灯光', deviceId: 'light', command: 'turn_off' }],
  },
]

export function WorkflowVisualEditor({
  workflow,
  onSave,
  onCancel,
  resources,
}: WorkflowVisualEditorProps) {
  const [editMode, setEditMode] = useState<EditMode>('visual')
  const [viewMode, setViewMode] = useState<ViewMode>('design')
  const [workflowName, setWorkflowName] = useState(workflow?.name || '')
  const [workflowDesc, setWorkflowDesc] = useState(workflow?.description || '')
  const [enabled, setEnabled] = useState(workflow?.enabled ?? true)
  const [triggers, setTriggers] = useState<Trigger[]>([])
  const [steps, setSteps] = useState<Step[]>([])
  const [jsonInput, setJsonInput] = useState('')
  const [selectedStep, setSelectedStep] = useState<Step | null>(null)
  const [saving, setSaving] = useState(false)

  // Initialize from workflow
  useEffect(() => {
    if (workflow) {
      setWorkflowName(workflow.name || '')
      setWorkflowDesc(workflow.description || '')
      setEnabled(workflow.enabled ?? true)

      if (workflow.triggers?.length > 0) {
        setTriggers(workflow.triggers.map((t: any, i: number) => ({
          id: t.id || `trigger-${i}`,
          type: t.type || 'manual',
          deviceId: t.device_id,
          metric: t.metric,
          operator: t.operator,
          threshold: t.threshold,
          cron: t.cron,
          cronLabel: t.cron_label,
        })))
      }

      if (workflow.steps?.length > 0) {
        setSteps(workflow.steps.map((s: any, i: number) => ({
          id: s.id || `step-${i}`,
          type: s.type || 'send_command',
          name: s.name || '',
          deviceId: s.device_id,
          command: s.command,
          value: s.value,
          message: s.message,
          duration: s.duration,
          condition: s.condition,
        })))
      }
    }
  }, [workflow])

  const applyTemplate = (template: typeof WORKFLOW_TEMPLATES[0]) => {
    setWorkflowName(template.name)
    setWorkflowDesc(template.desc)
    setTriggers(template.triggers.map(t => ({
      ...t,
      id: `trigger-${Date.now()}`,
      type: t.type as Trigger['type'],
    })))
    setSteps(template.steps.map(s => ({ ...s, id: `step-${Date.now()}-${Math.random()}` })))
  }

  const addTrigger = (type: Trigger['type']) => {
    const newTrigger: Trigger = {
      id: `trigger-${Date.now()}`,
      type,
      ...(type === 'cron' ? { cron: '0 9 * * *', cronLabel: '每天9点' } : {}),
    }
    setTriggers([...triggers, newTrigger])
  }

  const removeTrigger = (id: string) => {
    setTriggers(triggers.filter(t => t.id !== id))
  }

  const addStep = (type: string) => {
    const nodeType = NODE_TYPES.find(n => n.value === type)
    const newStep: Step = {
      id: `step-${Date.now()}`,
      type,
      name: nodeType?.label || '新步骤',
      ...(type === 'delay' ? { duration: 5 } : {}),
      ...(type === 'send_command' ? { command: 'turn_on' } : {}),
      ...(type === 'send_alert' || type === 'log' ? { message: '' } : {}),
    }
    setSteps([...steps, newStep])
  }

  const updateStep = (id: string, updates: Partial<Step>) => {
    setSteps(steps.map(s => (s.id === id ? { ...s, ...updates } : s)))
    if (selectedStep?.id === id) {
      setSelectedStep({ ...selectedStep, ...updates })
    }
  }

  const removeStep = (id: string) => {
    setSteps(steps.filter(s => s.id !== id))
    if (selectedStep?.id === id) {
      setSelectedStep(null)
      setViewMode('design')
    }
  }

  const selectStep = (step: Step) => {
    setSelectedStep(step)
    setViewMode('config')
  }

  const handleSave = async () => {
    setSaving(true)
    try {
      const data = editMode === 'json'
        ? JSON.parse(jsonInput)
        : {
            name: workflowName,
            description: workflowDesc,
            enabled,
            triggers: triggers.map(t => ({
              id: t.id,
              type: t.type,
              ...(t.deviceId && { device_id: t.deviceId }),
              ...(t.metric && { metric: t.metric }),
              ...(t.operator && { operator: t.operator }),
              ...(t.threshold !== undefined && { threshold: t.threshold }),
              ...(t.cron && { cron: t.cron }),
              ...(t.cronLabel && { cron_label: t.cronLabel }),
            })),
            steps: steps.map(s => ({
              id: s.id,
              type: s.type,
              name: s.name,
              ...(s.deviceId && { device_id: s.deviceId }),
              ...(s.command && { command: s.command }),
              ...(s.value !== undefined && { value: s.value }),
              ...(s.message && { message: s.message }),
              ...(s.duration && { duration: s.duration }),
              ...(s.condition && { condition: s.condition }),
            })),
          }
      await onSave({ id: workflow?.id, ...data })
    } finally {
      setSaving(false)
    }
  }

  const getDeviceName = (id: string) => resources?.devices.find(d => d.id === id)?.name || id
  const getTriggerInfo = (type: string) => TRIGGER_TYPES.find(t => t.value === type)
  const getStepInfo = (type: string) => NODE_TYPES.find(n => n.value === type)

  // JSON Mode
  if (editMode === 'json') {
    return (
      <div className="h-full flex flex-col">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h3 className="text-lg font-semibold">JSON 编辑模式</h3>
            <p className="text-sm text-muted-foreground">直接编辑工作流 JSON 配置</p>
          </div>
          <Button variant="outline" onClick={() => setEditMode('visual')}>
            <Zap className="h-4 w-4 mr-2" />
            可视化模式
          </Button>
        </div>
        <Textarea
          value={jsonInput}
          onChange={e => setJsonInput(e.target.value)}
          className="flex-1 font-mono text-sm resize-none"
          spellCheck={false}
          placeholder={`{
  "name": "工作流名称",
  "enabled": true,
  "triggers": [...],
  "steps": [...]
}`}
        />
        <div className="flex justify-end gap-3 mt-4">
          <Button variant="outline" onClick={onCancel}>取消</Button>
          <Button onClick={handleSave} disabled={saving}>
            {saving ? '保存中...' : <><Save className="h-4 w-4 mr-2" />保存</>}
          </Button>
        </div>
      </div>
    )
  }

  // Visual Mode - Horizontal node-based design
  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-primary" />
            {workflow ? '编辑工作流' : '创建工作流'}
          </h3>
          <p className="text-sm text-muted-foreground">拖拽构建自动化流程</p>
        </div>
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 mr-4">
            <Switch checked={enabled} onCheckedChange={setEnabled} />
            <Label className="text-sm">启用</Label>
          </div>
          <div className="flex items-center gap-1 p-1 bg-muted rounded-lg">
            <Button
              variant={viewMode === 'design' ? 'default' : 'ghost'}
              size="sm"
              onClick={() => setViewMode('design')}
            >
              设计视图
            </Button>
            <Button
              variant={viewMode === 'config' ? 'default' : 'ghost'}
              size="sm"
              onClick={() => setViewMode('config')}
              disabled={!selectedStep}
            >
              配置节点
            </Button>
          </div>
          <Button variant="outline" size="sm" onClick={() => setEditMode('json')}>
            <Code className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Design View */}
      {viewMode === 'design' && (
        <div className="flex-1 flex flex-col min-h-0">
          {/* Workflow Name Card */}
          <Card className="mb-4">
            <CardContent className="p-4">
              <Input
                value={workflowName}
                onChange={e => setWorkflowName(e.target.value)}
                placeholder="工作流名称..."
                className="text-lg font-semibold border-none focus-visible:ring-0 px-0"
              />
              <Input
                value={workflowDesc}
                onChange={e => setWorkflowDesc(e.target.value)}
                placeholder="描述这个工作流的用途..."
                className="text-sm text-muted-foreground border-none focus-visible:ring-0 px-0 mt-1"
              />
            </CardContent>
          </Card>

          {/* Quick Templates */}
          {!workflow && triggers.length === 0 && steps.length === 0 && (
            <Card className="mb-4 bg-gradient-to-r from-primary/5 to-primary/10">
              <CardHeader className="pb-3">
                <div className="flex items-center gap-2">
                  <Sparkles className="h-4 w-4 text-primary" />
                  <CardTitle className="text-sm font-medium">快速模板</CardTitle>
                </div>
              </CardHeader>
              <CardContent className="pt-0">
                <div className="flex gap-2">
                  {WORKFLOW_TEMPLATES.map(template => {
                    const Icon = template.icon
                    return (
                      <button
                        key={template.name}
                        onClick={() => applyTemplate(template)}
                        className="flex-1 p-3 bg-background rounded-lg hover:bg-background/80 transition-all duration-200 text-left border border-transparent hover:border-border"
                      >
                        <div className={`w-8 h-8 rounded-lg ${template.iconBg} flex items-center justify-center mb-2`}>
                          <Icon className={`h-4 w-4 ${template.iconColor}`} />
                        </div>
                        <p className="font-medium text-sm">{template.name}</p>
                        <p className="text-xs text-muted-foreground truncate">{template.desc}</p>
                      </button>
                    )
                  })}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Canvas */}
          <div className="flex-1 overflow-y-auto">
            <Card className="min-h-[300px]">
              <CardContent className="p-6">
                {/* Empty State */}
              {triggers.length === 0 && steps.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full min-h-[200px]">
                  <Play className="h-12 w-12 text-muted-foreground/30 mb-4" />
                  <p className="text-sm text-muted-foreground mb-4">开始构建你的工作流</p>
                  <div className="flex gap-2">
                    {TRIGGER_TYPES.map(tt => {
                      const Icon = tt.icon
                      return (
                        <Button
                          key={tt.value}
                          variant="outline"
                          onClick={() => addTrigger(tt.value as any)}
                          className={cn(tt.color, 'border-2 transition-all duration-200')}
                        >
                          <Icon className="h-4 w-4 mr-2" />
                          {tt.label}
                        </Button>
                      )
                    })}
                  </div>
                </div>
              ) : (
                <div className="space-y-6">
                  {/* Horizontal Flow */}
                  <div className="flex items-center gap-4 overflow-x-auto pb-4">
                    {/* Triggers Section */}
                    <div className="flex-shrink-0">
                      <div className="text-xs text-muted-foreground mb-2 px-1">触发器</div>
                      <div className="flex gap-2">
                        {triggers.map((trigger) => {
                          const tt = getTriggerInfo(trigger.type)
                          const Icon = tt?.icon
                          return (
                            <div
                              key={trigger.id}
                              className={cn(
                                'relative group min-w-[140px] p-3 rounded-lg border-2 cursor-pointer transition-all duration-200',
                                tt?.color
                              )}
                            >
                              <div className="flex items-center gap-2">
                                {Icon && <Icon className="h-5 w-5" />}
                                <span className="font-medium text-sm">{tt?.label}</span>
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="h-6 w-6 ml-auto opacity-0 group-hover:opacity-100"
                                  onClick={() => removeTrigger(trigger.id)}
                                >
                                  <X className="h-3 w-3" />
                                </Button>
                              </div>
                              {trigger.type === 'cron' && (
                                <div className="text-xs mt-1 opacity-75">{trigger.cronLabel}</div>
                              )}
                              {trigger.type === 'device' && (
                                <div className="text-xs mt-1 opacity-75">
                                  {trigger.metric} {trigger.operator} {trigger.threshold}
                                </div>
                              )}
                              {/* Connection dot */}
                              <div className="absolute -right-3 top-1/2 -translate-y-1/2 w-6 h-6 bg-background rounded-full flex items-center justify-center">
                                <div className="w-2 h-2 bg-primary rounded-full" />
                              </div>
                            </div>
                          )
                        })}
                        {triggers.length < 2 && (
                          <button
                            onClick={() => addTrigger('manual')}
                            className="min-w-[80px] h-[70px] border-2 border-dashed rounded-lg flex items-center justify-center text-muted-foreground hover:border-primary/50 hover:text-primary transition-all duration-200"
                          >
                            <Plus className="h-5 w-5" />
                          </button>
                        )}
                      </div>
                    </div>

                    {/* Arrow */}
                    {triggers.length > 0 && steps.length > 0 && (
                      <div className="flex-shrink-0">
                        <div className="w-16 h-0.5 bg-gradient-to-r from-muted to-primary/50 relative">
                          <ArrowRight className="absolute right-0 top-1/2 -translate-y-1/2 h-4 w-4 text-primary/50" />
                        </div>
                      </div>
                    )}

                    {/* Steps Section */}
                    {steps.length > 0 && (
                      <div className="flex-shrink-0">
                        <div className="text-xs text-muted-foreground mb-2 px-1">执行步骤</div>
                        <div className="flex gap-2 items-center">
                          {steps.map((step, i) => {
                            const st = getStepInfo(step.type)
                            const StepIcon = st?.icon
                            return (
                              <React.Fragment key={step.id}>
                                <div
                                  onClick={() => selectStep(step)}
                                  className={cn(
                                    'relative group min-w-[140px] p-3 rounded-lg border-2 cursor-pointer transition-all duration-200 hover:shadow-md',
                                    st?.color
                                  )}
                                >
                                  <div className="flex items-center gap-2">
                                    {StepIcon && <StepIcon className="h-5 w-5" />}
                                    <span className="font-medium text-sm">{step.name}</span>
                                    <Button
                                      variant="ghost"
                                      size="icon"
                                      className="h-6 w-6 ml-auto opacity-0 group-hover:opacity-100"
                                      onClick={(e) => { e.stopPropagation(); removeStep(step.id) }}
                                    >
                                      <X className="h-3 w-3" />
                                    </Button>
                                  </div>
                                  <div className="text-xs mt-1 opacity-60">
                                    {step.type === 'send_command' && (
                                      <>{getDeviceName(step.deviceId || '')} · {step.command}</>
                                    )}
                                    {step.type === 'delay' && <>等待 {step.duration} 秒</>}
                                    {step.type === 'send_alert' && <>{step.message || '通知'}</>}
                                    {step.type === 'log' && <>{step.message || '日志'}</>}
                                  </div>
                                  {/* Step number badge */}
                                  <div className="absolute -top-2 -left-2 w-5 h-5 bg-background rounded-full flex items-center justify-center text-xs font-bold shadow">
                                    {i + 1}
                                  </div>
                                  {/* Connection dot */}
                                  {i < steps.length - 1 && (
                                    <div className="absolute -right-3 top-1/2 -translate-y-1/2 w-6 h-6 bg-background rounded-full flex items-center justify-center">
                                      <div className="w-2 h-2 bg-muted rounded-full" />
                                    </div>
                                  )}
                                </div>
                                {i < steps.length - 1 && (
                                  <div className="flex-shrink-0">
                                    <ArrowRight className="h-4 w-4 text-muted-foreground" />
                                  </div>
                                )}
                              </React.Fragment>
                            )
                          })}
                          {/* Add step button */}
                          <div className="relative">
                            <div className="flex items-center gap-1">
                              <div className="w-8 h-0.5 bg-muted" />
                            </div>
                            <div className="absolute left-1/2 -translate-x-1/2 top-1/2 -translate-y-1/2">
                              <div className="relative group">
                                <button className="w-10 h-10 bg-primary text-primary-foreground rounded-full flex items-center justify-center shadow-lg hover:scale-110 transition-transform">
                                  <Plus className="h-5 w-5" />
                                </button>
                                {/* Step type popup */}
                                <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap">
                                  <div className="flex gap-1 bg-background border rounded-lg shadow-lg p-1">
                                    {NODE_TYPES.map(nt => {
                                      const NodeIcon = nt.icon
                                      return (
                                        <button
                                          key={nt.value}
                                          onClick={() => addStep(nt.value)}
                                          className="px-2 py-1 hover:bg-muted rounded text-sm transition-colors flex items-center gap-1"
                                        >
                                          <NodeIcon className="h-3 w-3" />
                                          {nt.label}
                                        </button>
                                      )
                                    })}
                                  </div>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              )}
              </CardContent>
            </Card>
          </div>
        </div>
      )}

      {/* Config View */}
      {viewMode === 'config' && selectedStep && (
        <div className="flex-1 flex gap-6 min-h-0">
          {/* Left: Canvas Preview */}
          <div className="w-1/2 overflow-y-auto">
            <Card className="mb-4">
              <CardContent className="p-4">
                <div className="text-xs text-muted-foreground mb-2">工作流预览</div>
              <div className="flex items-center gap-2 flex-wrap">
                {triggers.map(t => {
                  const tt = getTriggerInfo(t.type)
                  const TIcon = tt?.icon
                  return (
                    <div key={t.id} className={cn('px-2 py-1 rounded-lg text-sm flex items-center gap-1', tt?.color)}>
                      {TIcon && <TIcon className="h-3 w-3" />}
                      {tt?.label}
                    </div>
                  )
                })}
                <ArrowRight className="h-4 w-4 text-muted-foreground" />
                {steps.map((s, i) => {
                  const st = getStepInfo(s.type)
                  const SIcon = st?.icon
                  const isSelected = s.id === selectedStep.id
                  return (
                    <React.Fragment key={s.id}>
                      <div
                        onClick={() => setSelectedStep(s)}
                        className={cn(
                          'px-2 py-1 rounded-lg text-sm cursor-pointer border-2 flex items-center gap-1',
                          isSelected ? 'border-primary' : 'border-transparent hover:border-muted',
                          st?.color
                        )}
                      >
                        {SIcon && <SIcon className="h-3 w-3" />}
                        {s.name}
                      </div>
                      {i < steps.length - 1 && <ArrowRight className="h-4 w-4 text-muted-foreground" />}
                    </React.Fragment>
                  )
                })}
              </div>
              </CardContent>
            </Card>

            {/* Quick step list */}
            <div className="space-y-2">
              {steps.map((step, i) => {
                const st = getStepInfo(step.type)
                const StepIcon = st?.icon
                const isSelected = step.id === selectedStep.id
                return (
                  <div
                    key={step.id}
                    onClick={() => setSelectedStep(step)}
                    className={cn(
                      'flex items-center gap-3 p-3 rounded-lg border-2 cursor-pointer transition-all duration-200',
                      isSelected ? 'border-primary bg-primary/5' : 'border-transparent hover:border-muted'
                    )}
                  >
                    <div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center text-xs font-bold">
                      {i + 1}
                    </div>
                    {StepIcon && <StepIcon className="h-5 w-5" />}
                    <span className="font-medium flex-1">{step.name}</span>
                    <GripVertical className="h-4 w-4 text-muted-foreground" />
                  </div>
                )
              })}
            </div>
          </div>

          {/* Right: Step Config Panel */}
          <div className="w-1/2 overflow-y-auto">
            <Card>
              <CardContent className="p-6">
                <div className="flex items-center justify-between mb-6">
                  <div className="flex items-center gap-3">
                    {(() => {
                      const StepInfo = getStepInfo(selectedStep.type)
                      const Icon = StepInfo?.icon
                      return Icon ? <Icon className="h-6 w-6" /> : null
                    })()}
                    <h3 className="text-lg font-semibold">配置步骤</h3>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => { setViewMode('design'); setSelectedStep(null) }}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>

              <div className="space-y-4">
                {/* Step Name */}
                <div>
                  <Label>步骤名称</Label>
                  <Input
                    value={selectedStep.name}
                    onChange={e => updateStep(selectedStep.id, { name: e.target.value })}
                    className="mt-1"
                  />
                </div>

                {/* Device Control Config */}
                {selectedStep.type === 'send_command' && (
                  <>
                    <div>
                      <Label>选择设备</Label>
                      <Select
                        value={selectedStep.deviceId}
                        onValueChange={v => updateStep(selectedStep.id, { deviceId: v })}
                      >
                        <SelectTrigger className="mt-1">
                          <SelectValue placeholder="选择设备" />
                        </SelectTrigger>
                        <SelectContent>
                          {resources?.devices.map(d => (
                            <SelectItem key={d.id} value={d.id}>
                              <Home className="h-4 w-4 mr-2" />
                              {d.name}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div>
                      <Label>操作命令</Label>
                      <div className="grid grid-cols-3 gap-2 mt-1">
                        {[
                          { value: 'turn_on', label: '打开', icon: Power, color: 'text-green-500' },
                          { value: 'turn_off', label: '关闭', icon: PowerOff, color: 'text-red-500' },
                          { value: 'toggle', label: '切换', icon: RefreshCw, color: 'text-blue-500' },
                        ].map(cmd => {
                          const CmdIcon = cmd.icon
                          return (
                            <button
                              key={cmd.value}
                              onClick={() => updateStep(selectedStep.id, { command: cmd.value })}
                              className={cn(
                                'p-2 rounded-lg border-2 text-center transition-all duration-200',
                                selectedStep.command === cmd.value
                                  ? 'border-primary bg-primary/10'
                                  : 'border-muted hover:border-muted-foreground/50'
                              )}
                            >
                              <CmdIcon className={cn('h-5 w-5 mx-auto', cmd.color)} />
                              <p className="text-sm mt-1">{cmd.label}</p>
                            </button>
                          )
                        })}
                      </div>
                    </div>
                  </>
                )}

                {/* Delay Config */}
                {selectedStep.type === 'delay' && (
                  <div>
                    <Label>等待时长</Label>
                    <div className="flex items-center gap-2 mt-1">
                      <Input
                        type="number"
                        value={selectedStep.duration || 5}
                        onChange={e => updateStep(selectedStep.id, { duration: parseInt(e.target.value) || 0 })}
                        className="flex-1"
                        min={1}
                      />
                      <span className="text-sm text-muted-foreground">秒</span>
                    </div>
                  </div>
                )}

                {/* Alert Config */}
                {selectedStep.type === 'send_alert' && (
                  <div>
                    <Label>通知内容</Label>
                    <Input
                      value={selectedStep.message || ''}
                      onChange={e => updateStep(selectedStep.id, { message: e.target.value })}
                      placeholder="输入通知内容..."
                      className="mt-1"
                    />
                  </div>
                )}

                {/* Log Config */}
                {selectedStep.type === 'log' && (
                  <div>
                    <Label>日志内容</Label>
                    <Input
                      value={selectedStep.message || ''}
                      onChange={e => updateStep(selectedStep.id, { message: e.target.value })}
                      placeholder="输入日志内容..."
                      className="mt-1"
                    />
                  </div>
                )}

                {/* Condition Config */}
                {selectedStep.type === 'condition' && (
                  <div>
                    <Label>条件表达式</Label>
                    <Input
                      value={selectedStep.condition || ''}
                      onChange={e => updateStep(selectedStep.id, { condition: e.target.value })}
                      placeholder="device.sensor.temperature > 30"
                      className="mt-1 font-mono text-sm"
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      支持变量和比较操作符
                    </p>
                  </div>
                )}

                {/* Delete Button */}
                <div className="pt-4 border-t">
                  <Button
                    variant="destructive"
                    className="w-full"
                    onClick={() => removeStep(selectedStep.id)}
                  >
                    <Trash2 className="h-4 w-4 mr-2" />
                    删除此步骤
                  </Button>
                </div>
              </div>
              </CardContent>
            </Card>
          </div>
        </div>
      )}

      {/* Bottom Actions */}
      <div className="flex items-center justify-between pt-4 border-t mt-4">
        <Button variant="outline" onClick={onCancel}>
          取消
        </Button>
        <Button onClick={handleSave} disabled={!workflowName.trim() || saving}>
          {saving ? '保存中...' : (
            <>
              <Save className="h-4 w-4 mr-2" />
              保存工作流
            </>
          )}
        </Button>
      </div>
    </div>
  )
}
