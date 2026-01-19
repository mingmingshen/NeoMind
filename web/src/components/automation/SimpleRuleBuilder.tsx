import { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import { Switch } from '@/components/ui/switch'
import {
  ArrowDown,
  Plus,
  X,
  Wand2,
  Eye,
  Code,
} from 'lucide-react'
import type { DeviceType } from '@/types'

interface Rule {
  id?: string
  name: string
  dsl?: string
  enabled?: boolean
}

interface Condition {
  id: string
  deviceId: string
  metric: string
  operator: string
  value: string
}

interface Action {
  id: string
  type: 'device' | 'notification'
  deviceId?: string
  command?: string
  message?: string
}

interface SimpleRuleBuilderProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  rule?: Rule
  onSave: (rule: Partial<Rule>) => Promise<void>
  resources?: {
    devices: Array<{ id: string; name: string; device_type?: string }>
    deviceTypes?: DeviceType[]
  }
}

type Mode = 'visual' | 'code' | 'ai'

const OPERATORS = [
  { value: '>', label: '大于', symbol: '>' },
  { value: '<', label: '小于', symbol: '<' },
  { value: '>=', label: '大于等于', symbol: '≥' },
  { value: '<=', label: '小于等于', symbol: '≤' },
  { value: '==', label: '等于', symbol: '=' },
]

const COMMANDS = [
  { value: 'turn_on', label: '打开' },
  { value: 'turn_off', label: '关闭' },
]

const AI_EXAMPLES = [
  '温度超过30度打开空调',
  '天黑时自动开灯',
  '湿度低于40%开启加湿器',
]

function parseDSL(dsl: string): { conditions: Condition[]; actions: Action[] } {
  const conditions: Condition[] = []
  const actions: Action[] = []

  const whenMatch = dsl.match(/WHEN\s+(.+?)\s+THEN/s)
  if (whenMatch) {
    const conds = whenMatch[1].split(/\s+AND\s+/)
    conds.forEach(cond => {
      const m = cond.match(/device\.(\w+)\.(\w+)\s*([><=!]+)\s*(\d+)/)
      if (m) {
        conditions.push({ id: `c-${Date.now()}-${Math.random()}`, deviceId: m[1], metric: m[2], operator: m[3], value: m[4] })
      }
    })
  }

  const thenMatch = dsl.match(/THEN\s+(.+)/s)
  if (thenMatch) {
    const acts = thenMatch[1].split(/\s+\+\s+/)
    acts.forEach(act => {
      const dm = act.match(/device\.(\w+)\.(\w+)\(\)/)
      if (dm) {
        actions.push({ id: `a-${Date.now()}-${Math.random()}`, type: 'device', deviceId: dm[1], command: dm[2] })
      }
      const nm = act.match(/notify\("(.+?)"\)/)
      if (nm) {
        actions.push({ id: `a-${Date.now()}-${Math.random()}`, type: 'notification', message: nm[1] })
      }
    })
  }

  return { conditions, actions }
}

function generateDSL(conditions: Condition[], actions: Action[]): string {
  if (conditions.length === 0) return ''
  const condStr = conditions.map(c => `device.${c.deviceId}.${c.metric} ${c.operator} ${c.value}`).join(' AND ')
  const actStr = actions.map(a => {
    if (a.type === 'device') return `device.${a.deviceId}.${a.command}()`
    return `notify("${a.message}")`
  }).join(' + ')
  return `WHEN ${condStr} THEN ${actStr}`
}

function parseAI(text: string): { name: string; conditions: Condition[]; actions: Action[] } {
  const conditions: Condition[] = []
  const actions: Action[] = []

  const tempMatch = text.match(/温度.*?(\d+)/)
  const humMatch = text.match(/湿度.*?(\d+)/)
  const luxMatch = text.match(/亮度.*?(\d+)/)

  if (tempMatch) conditions.push({ id: `c-${Date.now()}`, deviceId: 'sensor', metric: 'temperature', operator: '>', value: tempMatch[1] })
  if (humMatch) conditions.push({ id: `c-${Date.now()}`, deviceId: 'sensor', metric: 'humidity', operator: '<', value: humMatch[1] })
  if (luxMatch) conditions.push({ id: `c-${Date.now()}`, deviceId: 'sensor', metric: 'brightness', operator: '<', value: luxMatch[1] })

  if (text.includes('开空调') || text.includes('打开空调')) actions.push({ id: `a-${Date.now()}`, type: 'device', deviceId: 'ac', command: 'turn_on' })
  if (text.includes('关空调') || text.includes('关闭空调')) actions.push({ id: `a-${Date.now()}`, type: 'device', deviceId: 'ac', command: 'turn_off' })
  if (text.includes('开灯') || text.includes('打开灯')) actions.push({ id: `a-${Date.now()}`, type: 'device', deviceId: 'light', command: 'turn_on' })
  if (text.includes('关灯') || text.includes('关闭灯')) actions.push({ id: `a-${Date.now()}`, type: 'device', deviceId: 'light', command: 'turn_off' })
  if (text.includes('加湿器')) actions.push({ id: `a-${Date.now()}`, type: 'device', deviceId: 'humidifier', command: 'turn_on' })
  if (text.includes('通知')) actions.push({ id: `a-${Date.now()}`, type: 'notification', message: '规则已触发' })

  if (conditions.length === 0) conditions.push({ id: `c-${Date.now()}`, deviceId: 'sensor', metric: 'temperature', operator: '>', value: '30' })
  if (actions.length === 0) actions.push({ id: `a-${Date.now()}`, type: 'device', deviceId: 'light', command: 'turn_on' })

  return { name: text.slice(0, 20), conditions, actions }
}

export function SimpleRuleBuilder({
  open,
  onOpenChange,
  rule,
  onSave,
  resources = { devices: [], deviceTypes: [] },
}: SimpleRuleBuilderProps) {
  const [mode, setMode] = useState<Mode>('visual')
  const [name, setName] = useState(rule?.name || '')
  const [enabled, setEnabled] = useState(rule?.enabled ?? true)
  const [conditions, setConditions] = useState<Condition[]>([])
  const [actions, setActions] = useState<Action[]>([])
  const [code, setCode] = useState(rule?.dsl || '')
  const [aiText, setAiText] = useState('')
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)

  // Helper functions - defined before useEffect
  const getDeviceType = (deviceId: string) => {
    const device = resources.devices.find(d => d.id === deviceId)
    return device?.device_type || resources.deviceTypes?.[0]?.device_type || ''
  }

  const getDeviceMetrics = (deviceId: string) => {
    const deviceTypeName = getDeviceType(deviceId)
    const deviceType = resources.deviceTypes?.find(t => t.device_type === deviceTypeName)
    return deviceType?.metrics || []
  }

  const getDeviceCommands = (deviceId: string) => {
    const deviceTypeName = getDeviceType(deviceId)
    const deviceType = resources.deviceTypes?.find(t => t.device_type === deviceTypeName)
    return deviceType?.commands || []
  }

  const createDefaultCondition = (): Condition => {
    const firstDevice = resources.devices[0]
    if (!firstDevice) return { id: `c-${Date.now()}`, deviceId: '', metric: 'temperature', operator: '>', value: '30' }
    const metrics = getDeviceMetrics(firstDevice.id)
    return {
      id: `c-${Date.now()}`,
      deviceId: firstDevice.id,
      metric: metrics[0]?.name || 'temperature',
      operator: '>',
      value: '30',
    }
  }

  const createDefaultAction = (): Action => {
    const firstDevice = resources.devices[0]
    if (!firstDevice) return { id: `a-${Date.now()}`, type: 'device', deviceId: '', command: 'turn_on' }
    const commands = getDeviceCommands(firstDevice.id)
    return {
      id: `a-${Date.now()}`,
      type: 'device',
      deviceId: firstDevice.id,
      command: commands[0]?.name || 'turn_on',
    }
  }

  useEffect(() => {
    if (open && rule?.dsl) {
      setCode(rule.dsl)
      const parsed = parseDSL(rule.dsl)
      setConditions(parsed.conditions.length > 0 ? parsed.conditions : [createDefaultCondition()])
      setActions(parsed.actions.length > 0 ? parsed.actions : [createDefaultAction()])
      setName(rule.name || '')
    } else if (open) {
      // Start with one condition and one action by default
      setConditions([createDefaultCondition()])
      setActions([createDefaultAction()])
      setCode('')
      setName('')
      setAiText('')
      setMode('visual')
    }
    setEnabled(rule?.enabled ?? true)
  }, [open, rule])

  const deviceOptions = resources.devices.map(d => ({ value: d.id, label: d.name }))

  const addCondition = () => {
    setConditions([...conditions, createDefaultCondition()])
  }

  const updateCondition = (id: string, data: Partial<Condition>) => {
    setConditions(conditions.map(c => c.id === id ? { ...c, ...data } : c))
  }

  const removeCondition = (id: string) => {
    setConditions(conditions.filter(c => c.id !== id))
  }

  const addAction = () => {
    setActions([...actions, createDefaultAction()])
  }

  const updateAction = (id: string, data: Partial<Action>) => {
    setActions(actions.map(a => a.id === id ? { ...a, ...data } : a))
  }

  const removeAction = (id: string) => {
    setActions(actions.filter(a => a.id !== id))
  }

  const handleAIGenerate = async () => {
    if (!aiText.trim()) return
    setLoading(true)
    await new Promise(r => setTimeout(r, 600))
    const result = parseAI(aiText)
    setName(result.name)
    setConditions(result.conditions)
    setActions(result.actions)
    setMode('visual')
    setLoading(false)
  }

  const handleSave = async () => {
    setSaving(true)
    try {
      const dsl = mode === 'code' ? code : generateDSL(conditions, actions)
      await onSave({ name, dsl, enabled })
    } finally {
      setSaving(false)
    }
  }

  const isValid = name.trim() && conditions.length > 0 && actions.length > 0

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-hidden flex flex-col p-0 gap-0">
        <DialogHeader className="px-6 pb-4 pt-6 border-t-0 border-x-0 border-b shrink-0 pr-12">
          <DialogTitle>{rule ? '编辑规则' : '创建自动化规则'}</DialogTitle>
          <DialogDescription>
            当条件满足时，自动执行设定的动作
          </DialogDescription>
        </DialogHeader>

        <Tabs value={mode} onValueChange={(v) => setMode(v as Mode)} className="flex-1 min-h-0 overflow-hidden flex flex-col">
          <div className="px-6 pt-4 pb-2 shrink-0">
            <TabsList className="grid w-full grid-cols-3">
              <TabsTrigger value="visual" className="gap-1.5">
                <Eye className="h-3.5 w-3.5" />
                <span>可视化</span>
              </TabsTrigger>
              <TabsTrigger value="code" className="gap-1.5">
                <Code className="h-3.5 w-3.5" />
                <span>代码</span>
              </TabsTrigger>
              <TabsTrigger value="ai" className="gap-1.5">
                <Wand2 className="h-3.5 w-3.5" />
                <span>AI 生成</span>
              </TabsTrigger>
            </TabsList>
          </div>

          <div className="flex-1 min-h-0 overflow-y-auto px-6 pb-4">
            {/* Name Input - Always visible */}
            <div className="space-y-3 mb-4">
              <Label htmlFor="rule-name">规则名称</Label>
              <Input
                id="rule-name"
                value={name}
                onChange={e => setName(e.target.value)}
                placeholder="例如：温度过高自动开空调"
              />
            </div>

            {/* Visual Mode */}
            <TabsContent value="visual" className="mt-0 space-y-4">
              {/* Conditions */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label className="text-sm font-medium">当满足以下条件</Label>
                  {conditions.length < 5 && (
                    <Button onClick={addCondition} variant="ghost" size="sm" className="h-7 text-xs">
                      <Plus className="h-3 w-3 mr-1" />
                      添加
                    </Button>
                  )}
                </div>
                <div className="space-y-1.5">
                  {conditions.map((c, i) => {
                    const metrics = getDeviceMetrics(c.deviceId)
                    return (
                      <div key={c.id} className="flex items-center gap-1.5 p-2 bg-muted/40 rounded-md text-sm">
                        <span className="text-xs text-muted-foreground w-4">{i + 1}</span>
                        <Select value={c.deviceId} onValueChange={v => updateCondition(c.id, { deviceId: v, metric: getDeviceMetrics(v)[0]?.name || 'temperature' })}>
                          <SelectTrigger className="h-7 w-28 text-xs">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            {deviceOptions.map(d => (
                              <SelectItem key={d.value} value={d.value} className="text-xs">{d.label}</SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                        <span className="text-xs text-muted-foreground">的</span>
                        {metrics.length > 0 ? (
                          <Select value={c.metric} onValueChange={v => updateCondition(c.id, { metric: v })}>
                            <SelectTrigger className="h-7 w-20 text-xs">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {metrics.map((m: any) => (
                                <SelectItem key={m.name} value={m.name} className="text-xs">{m.display_name || m.name}</SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        ) : (
                          <Input
                            value={c.metric}
                            onChange={e => updateCondition(c.id, { metric: e.target.value })}
                            placeholder="指标"
                            className="h-7 w-20 text-xs"
                          />
                        )}
                        <Select value={c.operator} onValueChange={v => updateCondition(c.id, { operator: v })}>
                          <SelectTrigger className="h-7 w-14 text-xs">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            {OPERATORS.map(o => (
                              <SelectItem key={o.value} value={o.value} className="text-xs">{o.symbol}</SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                        <Input
                          value={c.value}
                          onChange={e => updateCondition(c.id, { value: e.target.value })}
                          placeholder="值"
                          className="h-7 w-14 text-xs"
                        />
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-6 w-6 ml-auto"
                          onClick={() => removeCondition(c.id)}
                        >
                          <X className="h-3 w-3" />
                        </Button>
                      </div>
                    )
                  })}
                </div>
              </div>

              {/* Arrow */}
              {conditions.length > 0 && actions.length > 0 && (
                <div className="flex justify-center py-1">
                  <ArrowDown className="h-4 w-4 text-muted-foreground" />
                </div>
              )}

              {/* Actions */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label className="text-sm font-medium">执行以下动作</Label>
                  {actions.length < 5 && (
                    <Button onClick={addAction} variant="ghost" size="sm" className="h-7 text-xs">
                      <Plus className="h-3 w-3 mr-1" />
                      添加
                    </Button>
                  )}
                </div>
                <div className="space-y-1.5">
                  {actions.map((a, i) => {
                    const commands = a.type === 'device' ? getDeviceCommands(a.deviceId || '') : []
                    return (
                      <div key={a.id} className="flex items-center gap-1.5 p-2 bg-muted/40 rounded-md text-sm">
                        <span className="text-xs text-muted-foreground w-4">{i + 1}</span>
                        <Select
                          value={a.type}
                          onValueChange={(v: 'device' | 'notification') => updateAction(a.id, { type: v })}
                        >
                          <SelectTrigger className="h-7 w-20 text-xs">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="device" className="text-xs">控制设备</SelectItem>
                            <SelectItem value="notification" className="text-xs">发送通知</SelectItem>
                          </SelectContent>
                        </Select>
                        {a.type === 'device' ? (
                          <>
                            <Select value={a.deviceId} onValueChange={v => updateAction(a.id, { deviceId: v, command: getDeviceCommands(v)[0]?.name || 'turn_on' })}>
                              <SelectTrigger className="h-7 w-28 text-xs">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                {deviceOptions.map(d => (
                                  <SelectItem key={d.value} value={d.value} className="text-xs">{d.label}</SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                            {commands.length > 0 ? (
                              <Select value={a.command} onValueChange={v => updateAction(a.id, { command: v })}>
                                <SelectTrigger className="h-7 w-20 text-xs">
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  {commands.map((c: any) => (
                                    <SelectItem key={c.name} value={c.name} className="text-xs">{c.display_name || c.name}</SelectItem>
                                  ))}
                                </SelectContent>
                              </Select>
                            ) : (
                              <Select value={a.command} onValueChange={v => updateAction(a.id, { command: v })}>
                                <SelectTrigger className="h-7 w-20 text-xs">
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  {COMMANDS.map(c => (
                                    <SelectItem key={c.value} value={c.value} className="text-xs">{c.label}</SelectItem>
                                  ))}
                                </SelectContent>
                              </Select>
                            )}
                          </>
                        ) : (
                          <Input
                            value={a.message}
                            onChange={e => updateAction(a.id, { message: e.target.value })}
                            placeholder="通知内容"
                            className="h-7 flex-1 text-xs"
                          />
                        )}
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-6 w-6 ml-auto"
                          onClick={() => removeAction(a.id)}
                        >
                          <X className="h-3 w-3" />
                        </Button>
                      </div>
                    )
                  })}
                </div>
              </div>
            </TabsContent>

            {/* Code Mode */}
            <TabsContent value="code" className="mt-0 space-y-3">
              <div className="space-y-2">
                <Label>DSL 规则代码</Label>
                <textarea
                  value={code}
                  onChange={e => setCode(e.target.value)}
                  placeholder={`WHEN device.sensor.temperature > 30\nTHEN device.ac.turn_on()`}
                  className="w-full min-h-[160px] px-3 py-2 text-sm font-mono rounded-md border bg-background resize-none focus:outline-none focus:ring-2 focus:ring-ring"
                />
              </div>
              <div className="p-3 bg-muted/50 rounded-md text-xs text-muted-foreground space-y-1">
                <div><code>WHEN</code> - 触发条件</div>
                <div><code>THEN</code> - 执行动作</div>
                <div className="text-muted-foreground/70 mt-2">示例: WHEN device.sensor.temperature {'>'} 30 THEN device.ac.turn_on()</div>
              </div>
            </TabsContent>

            {/* AI Mode */}
            <TabsContent value="ai" className="mt-0 space-y-4">
              <div className="space-y-2">
                <Label>描述你想要的自动化规则</Label>
                <textarea
                  value={aiText}
                  onChange={e => setAiText(e.target.value)}
                  placeholder="例如：温度超过30度时打开空调"
                  className="w-full min-h-[100px] px-3 py-2 text-sm rounded-md border bg-background resize-none focus:outline-none focus:ring-2 focus:ring-ring"
                />
              </div>
              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">快速选择</Label>
                <div className="flex flex-wrap gap-2">
                  {AI_EXAMPLES.map(ex => (
                    <Button
                      key={ex}
                      variant="outline"
                      size="sm"
                      onClick={() => setAiText(ex)}
                      className="h-7 text-xs"
                    >
                      {ex}
                    </Button>
                  ))}
                </div>
              </div>
              <Button
                className="w-full"
                onClick={handleAIGenerate}
                disabled={!aiText.trim() || loading}
              >
                {loading ? '生成中...' : '生成规则'}
              </Button>
            </TabsContent>
          </div>
        </Tabs>

        <DialogFooter className="px-6 py-4 border-t shrink-0">
          <div className="flex items-center justify-between w-full">
            <div className="flex items-center gap-2">
              <Switch checked={enabled} onCheckedChange={setEnabled} />
              <Label className="text-sm cursor-pointer">启用规则</Label>
            </div>
            <div className="flex gap-2">
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                取消
              </Button>
              <Button onClick={handleSave} disabled={!isValid || saving}>
                {saving ? '保存中...' : '保存'}
              </Button>
            </div>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
