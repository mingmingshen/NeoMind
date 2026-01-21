import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Badge } from '@/components/ui/badge'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import {
  Loader2,
  Code,
  Play,
  Info,
  TestTube,
} from 'lucide-react'
import type { TransformAutomation, TransformScope } from '@/types'
import {
  FullScreenBuilder,
  BuilderSection,
  FormGrid,
  TipCard,
} from './FullScreenBuilder'

interface TransformBuilderProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  transform?: TransformAutomation | null
  devices: Array<{ id: string; name: string; device_type?: string }>
  onSave: (data: Partial<TransformAutomation>) => void
}

type ScopeType = 'global' | 'device_type' | 'device'
type Tab = 'code' | 'test'

// Example code templates
const CODE_TEMPLATES = [
  {
    name: { zh: '直接使用指标', en: 'Use Extracted Metrics' },
    code: '// 设备已定义指标时，直接使用 input 中的指标\nreturn {\n  battery: input.battery ?? 0,\n  temp: input.temp ?? input.temperature ?? 0,\n  timestamp: input.ts || Date.now()\n};',
  },
  {
    name: { zh: '使用原始数据', en: 'Use Raw Data' },
    code: '// 设备未定义指标时，使用 _raw 访问原始数据\nconst raw = input._raw || input;\nconst values = raw.values || raw;\nreturn {\n  battery: values.battery || 0,\n  temp: values.temp || values.temperature || 0,\n  timestamp: raw.ts || Date.now()\n};',
  },
  {
    name: { zh: '兼容写法', en: 'Compatible Approach' },
    code: '// 同时兼容已定义指标和原始数据\nconst raw = input._raw || input;\nconst values = input.values || raw?.values || raw || {};\nreturn {\n  battery: input.battery || values.battery || 0,\n  temp: input.temp || values.temp || values.temperature || 0,\n  timestamp: input.ts || raw?.ts || Date.now()\n};',
  },
  {
    name: { zh: '计算统计值', en: 'Calculate Statistics' },
    code: '// 计算多个指标的平均值\nconst metrics = [\'battery\', \'temp\', \'humidity\'];\nconst values = metrics.map(m => input[m] ?? 0).filter(v => v > 0);\nconst avg = values.length > 0 ? values.reduce((a, b) => a + b, 0) / values.length : 0;\nreturn {\n  avg: avg,\n  count: values.length,\n  sum: values.reduce((a, b) => a + b, 0)\n};',
  },
]

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
  const [outputPrefix, setOutputPrefix] = useState('transform')

  // Code state
  const [jsCode, setJsCode] = useState('')

  // Test state
  const [testInput, setTestInput] = useState('')
  const [testOutput, setTestOutput] = useState('')
  const [testRunning, setTestRunning] = useState(false)

  // Tab state
  const [activeTab, setActiveTab] = useState<Tab>('code')

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
    setOutputPrefix('transform')
    setJsCode('')
    setTestInput('')
    setTestOutput('')
    setActiveTab('code')
  }, [])

  // Test code
  const handleTestCode = useCallback(async () => {
    setTestRunning(true)
    setTestOutput('')

    try {
      const inputData = testInput.trim()
        ? JSON.parse(testInput)
        : // Default test data: simulates a device with defined metrics + _raw
          {
            battery: 85,
            temp: 23.5,
            humidity: 60,
            ts: 1737552000,
            _raw: {
              values: { battery: 85, temp: 23.5, humidity: 60 },
              ts: 1737552000
            }
          }

      // Create a function from the JS code
      const fn = new Function('input', jsCode)
      const result = fn(inputData)
      setTestOutput(JSON.stringify(result, null, 2))
    } catch (err) {
      setTestOutput(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      setTestRunning(false)
    }
  }, [jsCode, testInput])

  // Apply template
  const handleApplyTemplate = useCallback((templateCode: string) => {
    setJsCode(templateCode)
  }, [])

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
        default:
          return 'global' as const
      }
    })()

    // Ensure output_prefix is never empty - use "transform" as default
    const finalOutputPrefix = outputPrefix.trim() || 'transform'

    onSave({
      name,
      description,
      enabled,
      scope,
      js_code: jsCode,
      output_prefix: finalOutputPrefix,
      complexity: jsCode.split('\n').length > 10 ? 3 : 2,
    })
  }, [name, description, enabled, scopeType, scopeValue, jsCode, outputPrefix, onSave])

  // Validation
  const isValid = Boolean(name.trim() && jsCode.trim())
  const getValidationMessage = () => {
    if (!name.trim()) return t('automation:validation.nameRequired', { defaultValue: '请输入名称' })
    if (!jsCode.trim()) return t('automation:validation.codeRequired', { defaultValue: '请输入代码' })
    if (scopeType !== 'global' && !scopeValue.trim()) {
      return t('automation:validation.scopeValueRequired', { defaultValue: '请输入作用域值' })
    }
    return ''
  }

  // Get unique device types
  const deviceTypes = Array.from(new Set(devices.map((d) => d.device_type).filter(Boolean)))

  // Side panel content
  const sidePanelContent = (
    <div className="space-y-4">
      <TipCard
        title={t('automation:tips.transformTitle', { defaultValue: '关于数据转换' })}
        variant="info"
      >
        {t('automation:tips.transformDesc', {
          defaultValue: 'Transform 用于处理设备原始数据，提取有用指标或转换数据格式。',
        })}
      </TipCard>

      <TipCard
        title={t('automation:tips.scopeTitle', { defaultValue: '作用域说明' })}
        variant="info"
      >
        {t('automation:tips.scopeDesc', {
          defaultValue: '选择作用域可以限定 Transform 只处理特定设备或设备类型的数据。',
        })}
      </TipCard>

      {activeTab === 'code' && (
        <TipCard
          title={t('automation:tips.codeTitle', { defaultValue: '代码说明' })}
          variant="info"
        >
          <div className="space-y-2 text-sm">
            <p>{t('automation:tips.codeDesc', {
              defaultValue: 'JavaScript 代码，接收 input 参数，返回处理后的数据。',
            })}</p>
            <ul className="list-disc pl-4 space-y-1 text-muted-foreground">
              <li><code>input.battery</code> - 直接访问已定义的设备指标</li>
              <li><code>input._raw</code> - 访问完整原始数据（未定义指标时使用）</li>
              <li>返回对象生成多个虚拟指标（如 <code>transform.avg</code>）</li>
              <li>单值返回生成 <code>transform.value</code> 指标</li>
            </ul>
            <pre className="bg-muted p-2 rounded text-xs overflow-x-auto">
{`// 已定义指标的设备 - 直接使用
return {
  battery: input.battery ?? 0,
  temp: input.temp ?? 0
};

// 未定义指标的设备 - 使用 _raw
const raw = input._raw || input;
return {
  battery: raw.values?.battery || 0,
  temp: raw.values?.temp || 0
};

// 兼容两种情况
return {
  battery: input.battery || input._raw?.values?.battery || 0
};`}
            </pre>
          </div>
        </TipCard>
      )}

      {activeTab === 'test' && (
        <TipCard
          title={t('automation:tips.testTitle', { defaultValue: '测试说明' })}
          variant="warning"
        >
          {t('automation:tips.testDesc', {
            defaultValue: '使用示例数据测试你的 Transform 代码，确保输出格式正确。',
          })}
        </TipCard>
      )}
    </div>
  )

  return (
    <FullScreenBuilder
      open={open}
      onClose={() => onOpenChange(false)}
      title={transform
        ? t('automation:editTransform', { defaultValue: '编辑数据转换' })
        : t('automation:createTransform', { defaultValue: '创建数据转换' })
      }
      description={t('automation:transformBuilderDesc', {
        defaultValue: '定义如何处理设备数据，提取有用信息或转换数据格式',
      })}
      icon={<Code className="h-5 w-5 text-blue-500" />}
      headerActions={
        <Badge variant={enabled ? 'default' : 'secondary'} className="text-xs">
          {enabled ? t('common:enabled', { defaultValue: '启用' }) : t('common:disabled', { defaultValue: '禁用' })}
        </Badge>
      }
      sidePanel={{ content: sidePanelContent, title: t('automation:tips', { defaultValue: '提示' }) }}
      isValid={isValid}
      isDirty={true}
      isSaving={false}
      saveLabel={t('common:save', { defaultValue: '保存' })}
      onSave={handleSave}
      validationMessage={getValidationMessage()}
    >
      <div className="space-y-6">
        {/* Basic Info Section */}
        <BuilderSection
          title={t('automation:basicInfo', { defaultValue: '基本信息' })}
          icon={<Info className="h-4 w-4 text-muted-foreground" />}
        >
          <FormGrid columns={2}>
            <div className="space-y-2">
              <Label htmlFor="transform-name">{t('automation:name', { defaultValue: '名称' })} *</Label>
              <Input
                id="transform-name"
                value={name}
                onChange={e => setName(e.target.value)}
                placeholder={t('automation:transformNamePlaceholder', { defaultValue: '例如：统计检测数量' })}
              />
            </div>
            <div className="flex items-center gap-2">
              <Switch checked={enabled} onCheckedChange={setEnabled} id="transform-enabled" />
              <Label htmlFor="transform-enabled" className="text-sm cursor-pointer">
                {t('automation:enableTransform', { defaultValue: '启用转换' })}
              </Label>
            </div>
          </FormGrid>

          <div className="space-y-2">
            <Label htmlFor="transform-description">{t('common:description', { defaultValue: '描述' })}</Label>
            <Input
              id="transform-description"
              value={description}
              onChange={e => setDescription(e.target.value)}
              placeholder={t('automation:transformDescPlaceholder', { defaultValue: '描述这个转换的功能' })}
            />
          </div>

          {/* Scope Configuration */}
          <div className="space-y-3 pt-2">
            <Label>{t('automation:scope', { defaultValue: '作用域' })}</Label>
            <div className="flex gap-2">
              <div className="w-40">
                <Select value={scopeType} onValueChange={(v: ScopeType) => setScopeType(v)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="global">{t('automation:scopes.global', { defaultValue: '全局' })}</SelectItem>
                    <SelectItem value="device_type">{t('automation:scopes.deviceType', { defaultValue: '设备类型' })}</SelectItem>
                    <SelectItem value="device">{t('automation:scopes.device', { defaultValue: '设备' })}</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {scopeType === 'device_type' && (
                <div className="flex-1">
                  <Select value={scopeValue} onValueChange={setScopeValue}>
                    <SelectTrigger>
                      <SelectValue placeholder={t('automation:selectDeviceType', { defaultValue: '选择设备类型' })} />
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
                <div className="flex-1">
                  <Select value={scopeValue} onValueChange={setScopeValue}>
                    <SelectTrigger>
                      <SelectValue placeholder={t('automation:selectDevice', { defaultValue: '选择设备' })} />
                    </SelectTrigger>
                    <SelectContent>
                      {devices.map(d => (
                        <SelectItem key={d.id} value={d.id}>{d.name}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              )}

              {scopeType === 'global' && (
                <div className="flex-1 text-sm text-muted-foreground">
                  {t('automation:scopes.globalDesc', { defaultValue: '应用于所有设备数据' })}
                </div>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="output-prefix">{t('automation:outputPrefix', { defaultValue: '输出前缀' })}</Label>
              <Input
                id="output-prefix"
                value={outputPrefix}
                onChange={e => setOutputPrefix(e.target.value)}
                placeholder="transform"
                className="font-mono text-sm"
              />
            </div>
          </div>
        </BuilderSection>

        {/* Main Tabs */}
        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as Tab)} className="w-full">
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="code" className="gap-2">
              <Code className="h-4 w-4" />
              <span>{t('automation:code', { defaultValue: '代码' })}</span>
            </TabsTrigger>
            <TabsTrigger value="test" className="gap-2">
              <TestTube className="h-4 w-4" />
              <span>{t('automation:test', { defaultValue: '测试' })}</span>
            </TabsTrigger>
          </TabsList>

          {/* Code Tab */}
          <TabsContent value="code" className="mt-6">
            <BuilderSection
              title={t('automation:transformCode', { defaultValue: '转换代码' })}
              description={t('automation:transformCodeDesc', {
                defaultValue: 'JavaScript 函数，接收 input 参数并返回处理后的数据',
              })}
              icon={<Code className="h-4 w-4 text-muted-foreground" />}
            >
              <div className="space-y-4">
                {/* Quick Templates */}
                <div className="space-y-2">
                  <Label className="text-sm text-muted-foreground">
                    {t('automation:quickSelect', { defaultValue: '快速选择模板' })}
                  </Label>
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

                <Textarea
                  value={jsCode}
                  onChange={e => setJsCode(e.target.value)}
                  placeholder={`// 示例：统计检测数量
return (input.detections || []).length;`}
                  rows={16}
                  className="font-mono text-sm"
                  spellCheck={false}
                />

                <div className="flex items-center justify-between text-xs text-muted-foreground">
                  <span>{t('automation:availableVars', { defaultValue: '可用变量: input' })}</span>
                  <span>{jsCode.split('\n').length} {t('automation:lines', { defaultValue: '行' })}</span>
                </div>
              </div>
            </BuilderSection>
          </TabsContent>

          {/* Test Tab */}
          <TabsContent value="test" className="mt-6">
            <BuilderSection
              title={t('automation:testTransform', { defaultValue: '测试转换' })}
              description={t('automation:testTransformDesc', {
                defaultValue: '使用示例数据测试你的转换代码',
              })}
              icon={<TestTube className="h-4 w-4 text-muted-foreground" />}
            >
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label>{t('automation:inputData', { defaultValue: '输入数据 (JSON)' })}</Label>
                    <Textarea
                      value={testInput}
                      onChange={e => setTestInput(e.target.value)}
                      placeholder='{"battery": 85, "temp": 23.5, "ts": 1737552000, "_raw": {...}}'
                      rows={8}
                      className="font-mono text-sm"
                    />
                  </div>

                  <div className="space-y-2">
                    <div className="flex items-center justify-between">
                      <Label>{t('automation:outputData', { defaultValue: '输出结果' })}</Label>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={handleTestCode}
                        disabled={!jsCode || testRunning}
                      >
                        {testRunning ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <Play className="h-4 w-4" />
                        )}
                      </Button>
                    </div>
                    <Textarea
                      readOnly
                      value={testOutput || t('automation:testOutputPlaceholder', {
                        defaultValue: '点击运行按钮查看结果...',
                      })}
                      rows={8}
                      className="font-mono text-sm bg-muted/30"
                    />
                  </div>
                </div>
              </div>
            </BuilderSection>
          </TabsContent>
        </Tabs>
      </div>
    </FullScreenBuilder>
  )
}
