import { useState, useEffect, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog'
import { Code, Loader2, Play, Database, Save, FlaskConical } from 'lucide-react'
import { api } from '@/lib/api'
import type { TransformAutomation, TransformScope } from '@/types'
import { cn } from '@/lib/utils'

interface TransformBuilderProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  transform?: TransformAutomation | null
  devices: Array<{ id: string; name: string; device_type?: string }>
  onSave: (data: Partial<TransformAutomation>) => void
}

type ScopeType = 'global' | 'device_type' | 'device'

// Code templates
const CODE_TEMPLATES = [
  {
    name: { zh: '16进制转换', en: 'Hex Convert' },
    code: '// 16进制字符串转 JSON\nconst hex = input.hex || input.data || "";\nconst str = hex.match(/.{1,2}/g)?.map(b => String.fromCharCode(parseInt(b, 16))).join("") || "";\ntry {\n  return JSON.parse(str);\n} catch {\n  return { ...input, decoded: str };\n}',
  },
  {
    name: { zh: '结构转化', en: 'Transform Structure' },
    code: '// 数据结构转化 - 保留 input 并添加处理后的字段\nreturn {\n  ...input,\n  timestamp: input.ts || input.timestamp || Date.now(),\n  temperature: parseFloat(input.temp || input.temperature || 0),\n  humidity: parseFloat(input.hum || input.humidity || 0)\n};',
  },
  {
    name: { zh: '添加新参数', en: 'Add New Fields' },
    code: '// 保留 input 并添加新参数\nreturn {\n  ...input,\n  processed_at: Date.now(),\n  quality: (input.confidence || input.conf || 1) > 0.8 ? "high" : "low",\n  unit: input.unit || "unknown"\n};',
  },
  {
    name: { zh: '直接透传', en: 'Pass Through' },
    code: '// 直接返回 input，不做任何处理\nreturn input;',
  },
]

function MetricsPreviewPanel({
  scopeType,
  scopeValue,
  inputData,
  inputDataLoading,
  deviceTypeMetrics,
}: {
  scopeType: ScopeType
  scopeValue: string
  inputData?: Record<string, unknown> | null
  inputDataLoading?: boolean
  deviceTypeMetrics?: Array<{ name: string; display_name: string; data_type: string; unit?: string }>
}) {
  const getScopeLabel = () => {
    switch (scopeType) {
      case 'global': return '全局'
      case 'device_type': return `设备类型: ${scopeValue || '-'}`
      case 'device': return `设备: ${scopeValue || '-'}`
    }
  }

  const getTypeColor = (type: string) => {
    switch (type) {
      case 'number': case 'integer': case 'float': return 'text-blue-500'
      case 'string': return 'text-green-500'
      case 'boolean': return 'text-purple-500'
      case 'object': return 'text-orange-500'
      case 'array': return 'text-cyan-500'
      case 'binary': return 'text-yellow-500'
      default: return 'text-gray-500'
    }
  }

  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'number': case 'integer': case 'float': return '123'
      case 'string': return '"abc"'
      case 'boolean': return 'T/F'
      case 'object': return '{}'
      case 'array': return '[]'
      case 'binary': return ' BIN'
      default: return '?'
    }
  }

  // Convert device type metrics to MetricPreview format
  const deviceMetricsPreview = useMemo(() => {
    if (!deviceTypeMetrics) return []
    return deviceTypeMetrics.map(m => ({
      name: m.name,
      displayName: m.display_name,
      type: m.data_type === 'integer' || m.data_type === 'float' ? 'number' : m.data_type as any,
      unit: m.unit
    }))
  }, [deviceTypeMetrics])

  return (
    <div className="h-full flex flex-col p-3">
      <div className="flex items-center gap-2 mb-3">
        <Database className="h-4 w-4 text-blue-500" />
        <span className="font-semibold text-sm">输入数据预览</span>
      </div>

      {/* Scope info */}
      <div className="mb-2 p-2 bg-muted/50 rounded text-xs">
        <span className="text-muted-foreground">作用域: </span>
        <span className="font-medium">{getScopeLabel()}</span>
      </div>

      {/* Device Type Metrics (when device_type or device is selected) */}
      {(scopeType === 'device' || scopeType === 'device_type') && deviceMetricsPreview.length > 0 && (
        <div className="mb-2 flex-1 min-h-0">
          <div className="text-xs text-muted-foreground mb-1.5 flex items-center gap-1">
            <Database className="h-3 w-3" />
            可用输入指标 (input.xxx)
          </div>
          <ScrollArea className="h-full">
            <div className="space-y-1 pr-2">
              {deviceMetricsPreview.map((metric, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between p-1.5 bg-background border rounded"
                >
                  <div className="flex items-center gap-2">
                    <code className="text-xs font-mono text-blue-600 dark:text-blue-400">
                      {metric.name}
                    </code>
                    {metric.displayName && metric.displayName !== metric.name && (
                      <span className="text-xs text-muted-foreground">{metric.displayName}</span>
                    )}
                  </div>
                  <div className="flex items-center gap-1">
                    {metric.unit && (
                      <span className="text-xs text-muted-foreground">{metric.unit}</span>
                    )}
                    <Badge variant="outline" className={cn('text-xs h-5 px-1.5', getTypeColor(metric.type))}>
                      {getTypeIcon(metric.type)}
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          </ScrollArea>
        </div>
      )}

      {/* Input Data Sample */}
      {(scopeType === 'device' || scopeType === 'device_type') && (
        <div className="mb-2">
          <div className="text-xs text-muted-foreground mb-1">当前数据示例</div>
          <div className="p-2 bg-background border rounded">
            {inputDataLoading ? (
              <div className="text-xs text-muted-foreground flex items-center gap-2">
                <Loader2 className="h-3 w-3 animate-spin" /> 加载中...
              </div>
            ) : inputData ? (
              <pre className="text-xs font-mono overflow-auto max-h-32 text-muted-foreground">
                {JSON.stringify(inputData, null, 2)}
              </pre>
            ) : (
              <div className="text-xs text-muted-foreground">暂无数据</div>
            )}
          </div>
        </div>
      )}

      {/* Hint for global scope */}
      {scopeType === 'global' && (
        <div className="flex-1 flex items-center justify-center text-center p-4">
          <div className="text-sm text-muted-foreground">
            <Database className="h-8 w-8 mx-auto mb-2 opacity-50" />
            全局作用域<br />
            代码中可通过 <code className="text-xs bg-muted px-1 rounded">input</code> 访问原始数据
          </div>
        </div>
      )}
    </div>
  )
}

export function TransformBuilder({
  open,
  onOpenChange,
  transform,
  devices,
  onSave,
}: TransformBuilderProps) {
  const { t } = useTranslation(['automation', 'common'])

  // Form state
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [enabled, setEnabled] = useState(true)
  const [scopeType, setScopeType] = useState<ScopeType>('global')
  const [scopeValue, setScopeValue] = useState('')
  const [outputPrefix, setOutputPrefix] = useState('') // Empty by default = no prefix

  // Code state
  const [jsCode, setJsCode] = useState('')

  // Test state
  const [testInput, setTestInput] = useState('')
  const [testOutput, setTestOutput] = useState('')
  const [testError, setTestError] = useState('')
  const [testRunning, setTestRunning] = useState(false)

  // Input data preview state
  const [inputData, setInputData] = useState<Record<string, unknown> | null>(null)
  const [inputDataLoading, setInputDataLoading] = useState(false)
  const [deviceTypeMetrics, setDeviceTypeMetrics] = useState<Array<{ name: string; display_name: string; data_type: string; unit?: string }> | null>(null)

  // Get all device types
  const deviceTypes = useMemo(() => {
    return Array.from(new Set(devices.map((d) => d.device_type).filter(Boolean)))
  }, [devices])

  // Fetch input data preview and device type metrics
  useEffect(() => {
    const fetchInputData = async () => {
      if (scopeType === 'device' && scopeValue) {
        setInputDataLoading(true)
        try {
          const device = await api.getDevice(scopeValue)
          setInputData(device.current_values || null)

          // Also fetch device type metrics for the device's type
          if (device.device_type) {
            try {
              const deviceTypeData = await api.getDeviceType(device.device_type)
              setDeviceTypeMetrics(deviceTypeData.metrics || null)
            } catch {
              setDeviceTypeMetrics(null)
            }
          } else {
            setDeviceTypeMetrics(null)
          }
        } catch {
          setInputData(null)
          setDeviceTypeMetrics(null)
        } finally {
          setInputDataLoading(false)
        }
      } else if (scopeType === 'device_type' && scopeValue) {
        setInputDataLoading(true)
        try {
          const deviceTypeData = await api.getDeviceType(scopeValue)
          const samples = deviceTypeData.uplink_samples
          setInputData(samples && samples.length > 0 ? (samples[0] as Record<string, unknown>) : null)
          setDeviceTypeMetrics(deviceTypeData.metrics || null)
        } catch {
          setInputData(null)
          setDeviceTypeMetrics(null)
        } finally {
          setInputDataLoading(false)
        }
      } else {
        setInputData(null)
        setDeviceTypeMetrics(null)
      }
    }

    const timeoutId = setTimeout(fetchInputData, 300)
    return () => clearTimeout(timeoutId)
  }, [scopeType, scopeValue])

  // Reset form when transform changes
  useEffect(() => {
    if (open && transform) {
      setName(transform.name)
      setDescription(transform.description || '')
      setEnabled(transform.enabled)
      setOutputPrefix(transform.output_prefix || 'transform')
      setJsCode(transform.js_code || '')

      // Handle new scope format: 'global' | { device_type: string } | { device: string }
      if (transform.scope === 'global') {
        setScopeType('global')
        setScopeValue('')
      } else if (typeof transform.scope === 'object') {
        if ('device_type' in transform.scope) {
          setScopeType('device_type')
          setScopeValue(transform.scope.device_type || '')
        } else if ('device' in transform.scope) {
          setScopeType('device')
          setScopeValue(transform.scope.device || '')
        }
      }
    } else if (open) {
      resetForm()
    }
  }, [transform, open])

  const resetForm = useCallback(() => {
    setName('')
    setDescription('')
    setEnabled(true)
    setScopeType('global')
    setScopeValue('')
    setOutputPrefix('')
    setJsCode('')
    setTestInput('')
    setTestOutput('')
    setTestError('')
    setInputData(null)
    setDeviceTypeMetrics(null)
  }, [])

  // Apply template
  const handleApplyTemplate = useCallback((templateCode: string) => {
    setJsCode(templateCode)
  }, [])

  // Test code - shows the actual result as returned by the code
  const handleTestCode = useCallback(async () => {
    if (!jsCode.trim()) return

    setTestRunning(true)
    setTestOutput('')
    setTestError('')

    try {
      const inputData = testInput.trim()
        ? JSON.parse(testInput)
        : { temp: 25, humidity: 60 }

      const fn = new Function('input', jsCode)
      const result = fn(inputData)

      if (typeof result === 'object' && result !== null) {
        setTestOutput(JSON.stringify(result, null, 2))
      } else {
        setTestOutput(String(result))
      }
    } catch (err) {
      setTestError(err instanceof Error ? err.message : String(err))
    } finally {
      setTestRunning(false)
    }
  }, [jsCode, testInput])

  // Save
  const handleSave = useCallback(() => {
    if (!name.trim()) return

    const scope: TransformScope = (() => {
      switch (scopeType) {
        case 'global':
          return 'global' as const
        case 'device_type':
          return { device_type: scopeValue }
        case 'device':
          return { device: scopeValue }
      }
    })()

    onSave({
      name,
      description,
      enabled,
      scope,
      js_code: jsCode,
      output_prefix: outputPrefix,
      complexity: jsCode.split('\n').length > 10 ? 3 : 2,
    })
  }, [name, description, enabled, scopeType, scopeValue, jsCode, outputPrefix, onSave])

  // Validation
  const isValid = Boolean(name.trim() && jsCode.trim())

  // Get selected scope display name (for preview panel)
  const getScopeDisplayName = () => {
    if (scopeType === 'device_type') return `设备类型: ${scopeValue || '-'}`
    if (scopeType === 'device') {
      const device = devices.find(d => d.id === scopeValue)
      return `设备: ${device?.name || scopeValue || '-'}`
    }
    return '全局'
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-5xl h-[85vh] p-0 flex flex-col">
        <DialogHeader className="px-6 py-4 border-b">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Code className="h-5 w-5 text-blue-500" />
              <div>
                <DialogTitle className="text-lg font-semibold">
                  {transform ? '编辑数据转换' : '创建数据转换'}
                </DialogTitle>
                <DialogDescription className="text-sm">
                  定义如何处理设备数据，提取有用信息或转换数据格式
                </DialogDescription>
              </div>
            </div>
          </div>
        </DialogHeader>

        {/* Form Section */}
        <div className="border-b px-6 py-4 bg-muted/20 flex-shrink-0">
          {/* Row 1: Name, Description */}
          <div className="grid grid-cols-2 gap-4 mb-4">
            <div>
              <Label htmlFor="transform-name" className="text-xs">名称 *</Label>
              <Input
                id="transform-name"
                value={name}
                onChange={e => setName(e.target.value)}
                placeholder="数据转换"
                className="mt-1 h-9"
              />
            </div>
            <div>
              <Label htmlFor="transform-desc" className="text-xs">描述</Label>
              <Input
                id="transform-desc"
                value={description}
                onChange={e => setDescription(e.target.value)}
                placeholder="描述功能"
                className="mt-1 h-9"
              />
            </div>
          </div>

          {/* Row 2: Scope Type, Scope Value, Enabled */}
          <div className="flex items-center gap-4">
            <div className="w-36">
              <Label className="text-xs">作用域</Label>
              <Select value={scopeType} onValueChange={(v: ScopeType) => setScopeType(v)}>
                <SelectTrigger className="mt-1 h-9">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="global">全局</SelectItem>
                  <SelectItem value="device_type">设备类型</SelectItem>
                  <SelectItem value="device">设备</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {scopeType === 'device_type' && (
              <div className="w-48">
                <Label className="text-xs">设备类型</Label>
                <Select value={scopeValue} onValueChange={setScopeValue}>
                  <SelectTrigger className="mt-1 h-9">
                    <SelectValue placeholder="选择类型" />
                  </SelectTrigger>
                  <SelectContent>
                    {deviceTypes.map(dt => (
                      <SelectItem key={dt} value={dt || ''}>{dt}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}
            {scopeType === 'device' && (
              <div className="w-48">
                <Label className="text-xs">设备</Label>
                <Select value={scopeValue} onValueChange={setScopeValue}>
                  <SelectTrigger className="mt-1 h-9">
                    <SelectValue placeholder="选择设备" />
                  </SelectTrigger>
                  <SelectContent>
                    {devices.map(d => (
                      <SelectItem key={d.id} value={d.id}>{d.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}

            <div className="flex items-center gap-2 ml-auto">
              <Switch checked={enabled} onCheckedChange={setEnabled} id="transform-enabled" />
              <Label htmlFor="transform-enabled" className="text-sm cursor-pointer">
                启用
              </Label>
            </div>
          </div>
        </div>

        {/* Main Content - Code Editor + Input Preview */}
        <div className="flex-1 min-h-0 flex">
          {/* Left - Code Editor */}
          <div className="flex-1 flex flex-col min-w-0">
            {/* Templates */}
            <div className="px-6 py-3 border-b bg-muted/20 flex-shrink-0">
              <Label className="text-xs text-muted-foreground mb-2 block">快速模板</Label>
              <div className="flex flex-wrap gap-2">
                {CODE_TEMPLATES.map((tpl, i) => (
                  <Button
                    key={i}
                    variant="outline"
                    size="sm"
                    type="button"
                    onClick={() => handleApplyTemplate(tpl.code)}
                    className="h-8 text-xs"
                  >
                    {t('common:lang') === 'zh' ? tpl.name.zh : tpl.name.en}
                  </Button>
                ))}
              </div>
            </div>

            {/* Code Editor */}
            <div className="flex-1 flex flex-col min-h-0">
              <div className="px-6 py-3 border-b bg-muted/30 flex items-center justify-between flex-shrink-0">
                <Label className="text-sm">转换代码 (JavaScript)</Label>
                <span className="text-xs text-muted-foreground">
                  使用 <code className="text-xs bg-muted px-1 rounded">input</code> 访问输入数据
                </span>
              </div>
              <Textarea
                value={jsCode}
                onChange={e => setJsCode(e.target.value)}
                placeholder={`// 编写转换代码\nreturn {\n  battery: input.values.battery,\n  percent: (input.values.battery / 100).toFixed(2)\n};`}
                className="flex-1 resize-none font-mono text-sm rounded-none border-r focus-visible:ring-0 p-4"
                spellCheck={false}
              />
            </div>
          </div>

          {/* Right - Input Data Preview */}
          <div className="w-80 border-l flex flex-col bg-muted/10">
            <div className="px-4 py-3 border-b bg-muted/20 flex-shrink-0">
              <Label className="text-xs flex items-center gap-2">
                <Database className="h-3 w-3" />
                输入数据预览
              </Label>
            </div>
            <div className="flex-1 overflow-auto">
              <MetricsPreviewPanel
                scopeType={scopeType}
                scopeValue={getScopeDisplayName()}
                inputData={inputData}
                inputDataLoading={inputDataLoading}
                deviceTypeMetrics={deviceTypeMetrics || undefined}
              />
            </div>
          </div>
        </div>

        {/* Bottom Row: Test (Left) + Output (Right) */}
        <div className="h-48 border-t flex">
          {/* Test Section */}
          <div className="w-1/2 border-r p-4 flex flex-col">
            <Label className="text-xs mb-2 flex items-center gap-2">
              <Play className="h-3 w-3" />
              测试
            </Label>
            <div className="flex-1 flex flex-col min-h-0">
              <Textarea
                value={testInput}
                onChange={e => setTestInput(e.target.value)}
                placeholder={inputData
                  ? `输入: ${JSON.stringify(inputData).slice(0, 60)}...`
                  : '{"temp": 25, "humidity": 60}'
                }
                className="flex-1 font-mono text-xs resize-none mb-2"
              />
              <div className="flex gap-2 flex-shrink-0">
                <Button
                  size="sm"
                  onClick={handleTestCode}
                  disabled={!jsCode || testRunning}
                  className="h-8"
                >
                  {testRunning ? <Loader2 className="h-3 w-3 animate-spin" /> : <Play className="h-3 w-3 mr-1" />}
                  运行
                </Button>
                {(scopeType === 'device_type' || scopeType === 'device') && deviceTypeMetrics && deviceTypeMetrics.length > 0 && (
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      const mockData: Record<string, unknown> = {}
                      for (const metric of deviceTypeMetrics) {
                        switch (metric.data_type) {
                          case 'integer':
                            mockData[metric.name] = Math.floor(Math.random() * 100)
                            break
                          case 'float':
                            mockData[metric.name] = parseFloat((Math.random() * 100).toFixed(2))
                            break
                          case 'string':
                            mockData[metric.name] = `sample_${metric.name}`
                            break
                          case 'boolean':
                            mockData[metric.name] = Math.random() > 0.5
                            break
                          case 'array':
                            mockData[metric.name] = [
                            Math.floor(Math.random() * 100),
                            parseFloat((Math.random() * 100).toFixed(2)),
                            `sample_${metric.name}`
                          ]
                            break
                          default:
                            mockData[metric.name] = null
                        }
                      }
                      setTestInput(JSON.stringify(mockData, null, 2))
                    }}
                    className="h-8"
                  >
                    <FlaskConical className="h-3 w-3 mr-1" />
                    模拟数据
                  </Button>
                )}
                {(testOutput || testError) && (
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => { setTestOutput(''); setTestError('') }}
                    className="h-8"
                  >
                    清除
                  </Button>
                )}
              </div>
            </div>
          </div>

          {/* Test Output */}
          <div className="w-1/2 p-4 flex flex-col">
            <Label className="text-xs mb-2 flex items-center gap-2">
              <Database className="h-3 w-3" />
              输出结果
            </Label>
            <div className="flex-1 min-h-0 overflow-auto">
              {testError && (
                <div className="p-2 bg-destructive/10 border border-destructive/20 rounded text-xs text-destructive font-mono">
                  {testError}
                </div>
              )}
              {testOutput && !testError && (
                <pre className="text-xs font-mono text-muted-foreground">
                  {testOutput}
                </pre>
              )}
              {!testOutput && !testError && (
                <div className="text-xs text-muted-foreground text-center py-8">
                  点击"运行"查看输出结果
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end px-6 py-3 border-t bg-muted/20 flex-shrink-0">
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              取消
            </Button>
            <Button onClick={handleSave} disabled={!isValid}>
              <Save className="h-4 w-4 mr-1" />
              保存
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
