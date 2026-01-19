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
import { Switch } from '@/components/ui/switch'
import { Card } from '@/components/ui/card'
import { Loader2, Sparkles, Code, Play, CheckCircle2 } from 'lucide-react'
import { api } from '@/lib/api'
import type { TransformAutomation, TransformScope } from '@/types'

interface TransformBuilderProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  transform?: TransformAutomation | null
  devices: Array<{ id: string; name: string; device_type?: string }>
  onSave: (data: Partial<TransformAutomation>) => void
}

type ScopeType = 'global' | 'device_type' | 'device' | 'user'

type GenerationState = 'idle' | 'generating' | 'success' | 'error'

export function TransformBuilder({ open, onOpenChange, transform, devices, onSave }: TransformBuilderProps) {
  const { t } = useTranslation(['automation', 'common'])

  // Form state
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [enabled, setEnabled] = useState(true)
  const [scopeType, setScopeType] = useState<ScopeType>('global')
  const [scopeValue, setScopeValue] = useState('')
  const [outputPrefix, setOutputPrefix] = useState('transform')

  // AI Generation state
  const [intent, setIntent] = useState('')
  const [generatedCode, setGeneratedCode] = useState('')
  const [generationState, setGenerationState] = useState<GenerationState>('idle')
  const [errorMessage, setErrorMessage] = useState('')

  // Test state
  const [testInput, setTestInput] = useState('')
  const [testOutput, setTestOutput] = useState('')
  const [testRunning, setTestRunning] = useState(false)

  // Reset form when transform changes
  useEffect(() => {
    if (transform) {
      setName(transform.name)
      setDescription(transform.description)
      setEnabled(transform.enabled)
      setScopeType(transform.scope.type as ScopeType)
      setOutputPrefix(transform.output_prefix || 'transform')
      setIntent(transform.intent || '')
      setGeneratedCode(transform.js_code || '')

      if (transform.scope.type === 'device_type') {
        setScopeValue(transform.scope.device_type)
      } else if (transform.scope.type === 'device') {
        setScopeValue(transform.scope.device_id)
      } else if (transform.scope.type === 'user') {
        setScopeValue(transform.scope.user_id)
      }
    } else {
      resetForm()
    }
  }, [transform, open])

  const resetForm = () => {
    setName('')
    setDescription('')
    setEnabled(true)
    setScopeType('global')
    setScopeValue('')
    setOutputPrefix('transform')
    setIntent('')
    setGeneratedCode('')
    setGenerationState('idle')
    setErrorMessage('')
    setTestInput('')
    setTestOutput('')
  }

  // Example intents for quick start
  const exampleIntents = [
    { zh: '统计 detections 数组中每个 cls 的数量', en: 'Count detections by class' },
    { zh: '计算数组中所有值的平均值', en: 'Calculate average of array values' },
    { zh: '过滤置信度低于 0.5 的检测', en: 'Filter detections with confidence < 0.5' },
    { zh: '16进制字符串转 JSON', en: 'Convert hex string to JSON' },
    { zh: '提取嵌套字段到根级别', en: 'Extract nested fields to root level' },
  ]

  const handleGenerateCode = async () => {
    if (!intent.trim()) return

    setGenerationState('generating')
    setErrorMessage('')

    try {
      // Call the API to generate code
      const result = await api.generateTransformCode({
        intent,
        language: t('common:lang', { defaultValue: 'en' }), // Use current language
      })

      setGeneratedCode(result.js_code)
      setGenerationState('success')

      // Auto-fill name if empty
      if (!name.trim()) {
        setName(result.suggested_name || intent.slice(0, 50))
      }

      // Auto-fill output prefix if default
      if (outputPrefix === 'transform') {
        setOutputPrefix(result.output_prefix || 'transform')
      }
    } catch (err) {
      setErrorMessage(err instanceof Error ? err.message : String(err))
      setGenerationState('error')
    }
  }

  const handleTestCode = async () => {
    setTestRunning(true)
    setTestOutput('')

    try {
      const inputData = testInput.trim() ? JSON.parse(testInput) : { detections: [{ cls: 'fish' }, { cls: 'fish' }, { cls: 'shrimp' }] }

      // Create a function from the generated code
      const fn = new Function('input', generatedCode)
      const result = fn(inputData)
      setTestOutput(JSON.stringify(result, null, 2))
    } catch (err) {
      setTestOutput(`Error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      setTestRunning(false)
    }
  }

  const handleSave = () => {
    if (!name.trim()) return

    const scope: TransformScope = (() => {
      switch (scopeType) {
        case 'global':
          return { type: 'global' }
        case 'device_type':
          return { type: 'device_type', device_type: scopeValue }
        case 'device':
          return { type: 'device', device_id: scopeValue }
        case 'user':
          return { type: 'user', user_id: scopeValue }
      }
    })()

    onSave({
      name,
      description: description || intent,
      enabled,
      scope,
      intent,
      js_code: generatedCode,
      output_prefix: outputPrefix,
      complexity: generatedCode.split('\n').length > 10 ? 3 : 2,
    })
  }

  // Get unique device types
  const deviceTypes = Array.from(new Set(devices.map((d) => d.device_type).filter(Boolean)))

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-purple-500" />
            {transform
              ? t('automation:editTransform', { defaultValue: 'Edit Transform' })
              : t('automation:createTransform', { defaultValue: 'Create Transform' })}
          </DialogTitle>
          <DialogDescription>
            {t('automation:aiTransformDesc', {
              defaultValue: 'Describe what you want to transform in plain language. AI will generate the JavaScript code for you.',
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-6 py-4">
          {/* Intent Input */}
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="transform-intent" className="flex items-center gap-2">
                <Sparkles className="h-4 w-4 text-purple-500" />
                {t('automation:whatToTransform', { defaultValue: 'What do you want to transform?' })}
              </Label>
              <Textarea
                id="transform-intent"
                value={intent}
                onChange={(e) => setIntent(e.target.value)}
                placeholder={t('automation:intentPlaceholder', {
                  defaultValue: 'e.g., 统计 detections 数组中每个 cls 的数量',
                })}
                rows={2}
                className="resize-none"
              />
            </div>

            {/* Example Intents */}
            <div className="flex flex-wrap gap-2">
              {exampleIntents.map((example, idx) => (
                <Button
                  key={idx}
                  variant="outline"
                  size="sm"
                  type="button"
                  onClick={() => setIntent(example.zh)}
                >
                  {example.zh}
                </Button>
              ))}
            </div>

            {/* Generate Button */}
            <Button
              onClick={handleGenerateCode}
              disabled={!intent.trim() || generationState === 'generating'}
              className="w-full bg-purple-600 hover:bg-purple-700"
            >
              {generationState === 'generating' ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  {t('automation:generating', { defaultValue: 'Generating...' })}
                </>
              ) : (
                <>
                  <Sparkles className="h-4 w-4 mr-2" />
                  {t('automation:generateCode', { defaultValue: 'Generate JavaScript Code' })}
                </>
              )}
            </Button>

            {/* Error Message */}
            {generationState === 'error' && (
              <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md">
                <p className="text-sm text-destructive">{errorMessage}</p>
              </div>
            )}
          </div>

          {/* Generated Code Display */}
          {generatedCode && (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label className="flex items-center gap-2">
                  <Code className="h-4 w-4" />
                  {t('automation:generatedCode', { defaultValue: 'Generated JavaScript Code' })}
                  {generationState === 'success' && (
                    <CheckCircle2 className="h-4 w-4 text-green-500" />
                  )}
                </Label>
                <Card className="p-4 bg-muted">
                  <pre className="text-sm overflow-x-auto font-mono whitespace-pre-wrap">
                    <code>{generatedCode}</code>
                  </pre>
                </Card>
              </div>

              {/* Test Section */}
              <div className="space-y-2">
                <Label>{t('automation:testTransform', { defaultValue: 'Test with sample data' })}</Label>
                <div className="flex gap-2">
                  <Textarea
                    value={testInput}
                    onChange={(e) => setTestInput(e.target.value)}
                    placeholder={t('automation:testInputPlaceholder', {
                      defaultValue: 'Enter JSON to test (leave empty for sample data)',
                    })}
                    rows={3}
                    className="flex-1 font-mono text-sm"
                  />
                  <Button
                    onClick={handleTestCode}
                    disabled={testRunning}
                    variant="outline"
                    className="self-end"
                  >
                    {testRunning ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Play className="h-4 w-4" />
                    )}
                  </Button>
                </div>
                {testOutput && (
                  <Card className="p-3 bg-muted">
                    <pre className="text-xs overflow-x-auto font-mono">{testOutput}</pre>
                  </Card>
                )}
              </div>
            </div>
          )}

          {/* Basic Configuration */}
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="transform-name">{t('automation:name', { defaultValue: 'Name' })} *</Label>
              <Input
                id="transform-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t('automation:transformNamePlaceholder', { defaultValue: 'e.g., Detection Count' })}
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="output-prefix">{t('automation:outputPrefix', { defaultValue: 'Output Prefix' })}</Label>
                <Input
                  id="output-prefix"
                  value={outputPrefix}
                  onChange={(e) => setOutputPrefix(e.target.value)}
                  placeholder="detection_count"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="scope-type">{t('automation:scopeType', { defaultValue: 'Scope' })}</Label>
                <Select value={scopeType} onValueChange={(v: ScopeType) => setScopeType(v)}>
                  <SelectTrigger id="scope-type">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="global">{t('automation:scopeGlobal', { defaultValue: 'Global' })}</SelectItem>
                    <SelectItem value="device_type">{t('automation:scopeDeviceType', { defaultValue: 'Device Type' })}</SelectItem>
                    <SelectItem value="device">{t('automation:scopeDevice', { defaultValue: 'Device' })}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>

            {(scopeType === 'device_type' || scopeType === 'device') && (
              <div className="space-y-2">
                <Label htmlFor="scope-value">
                  {scopeType === 'device_type'
                    ? t('automation:deviceType', { defaultValue: 'Device Type' })
                    : t('automation:device', { defaultValue: 'Device' })}
                </Label>
                {scopeType === 'device_type' ? (
                  <Select value={scopeValue} onValueChange={setScopeValue}>
                    <SelectTrigger id="scope-value">
                      <SelectValue placeholder={t('automation:selectDeviceType', { defaultValue: 'Select device type' })} />
                    </SelectTrigger>
                    <SelectContent>
                      {deviceTypes.map((dt) => (
                        <SelectItem key={dt} value={dt || ''}>
                          {dt || 'Unknown'}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                ) : (
                  <Select value={scopeValue} onValueChange={setScopeValue}>
                    <SelectTrigger id="scope-value">
                      <SelectValue placeholder={t('automation:selectDevice', { defaultValue: 'Select device' })} />
                    </SelectTrigger>
                    <SelectContent>
                      {devices.map((d) => (
                        <SelectItem key={d.id} value={d.id}>
                          {d.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </div>
            )}

            <div className="flex items-center gap-2">
              <Switch id="transform-enabled" checked={enabled} onCheckedChange={setEnabled} />
              <Label htmlFor="transform-enabled">{t('automation:enableTransform', { defaultValue: 'Enable transform' })}</Label>
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('common:cancel', { defaultValue: 'Cancel' })}
          </Button>
          <Button onClick={handleSave} disabled={!name.trim() || !generatedCode}>
            {t('common:save', { defaultValue: 'Save' })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
