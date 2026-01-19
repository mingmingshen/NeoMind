import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  ArrowRight,
  Plus,
  Zap,
  Bell,
  Play,
  Save,
  FileText,
  Settings,
  Check,
  X,
  Loader2,
} from 'lucide-react'
import type {
  Rule,
  RuleCondition,
  RuleAction,
  RuleTrigger,
} from '@/types'
import { ConditionBuilder } from './rule/ConditionBuilder'
import { ActionBuilder } from './rule/ActionBuilder'
import { RuleTester } from './rule/RuleTester'
import { api } from '@/lib/api'

interface RuleBuilderProps {
  rule?: Rule
  onSave: (rule: Partial<Rule>) => Promise<void>
  onCancel: () => void
  resources?: {
    devices: Array<{ id: string; name: string; type: string }>
    metrics: string[]
    alertChannels: Array<{ id: string; name: string }>
  }
}

interface RuleFormData {
  id?: string
  name: string
  description: string
  enabled: boolean
  trigger: RuleTrigger
  conditions: RuleCondition[]
  conditionOperator: 'AND' | 'OR'
  actions: RuleAction[]
}

const DEFAULT_TRIGGER: RuleTrigger = { type: 'device_state', device_id: '', state: 'active' }

const DEFAULT_CONDITION: RuleCondition = {
  device_id: '',
  metric: '',
  operator: '>',
  threshold: 0,
}

const DEFAULT_ACTION: RuleAction = {
  type: 'Execute',
  device_id: '',
  command: '',
  params: {},
}

function generateId() {
  return `_${Math.random().toString(36).substr(2, 9)}`
}

export function RuleBuilder({ rule, onSave, onCancel, resources }: RuleBuilderProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)
  const [activeTab, setActiveTab] = useState<'builder' | 'dsl' | 'test'>('builder')
  const [testResult, setTestResult] = useState<{
    triggered: boolean
    message: string
    details?: Record<string, unknown>
  } | null>(null)

  const [formData, setFormData] = useState<RuleFormData>({
    name: rule?.name || '',
    description: rule?.description || '',
    enabled: rule?.enabled ?? true,
    trigger: rule?.trigger || DEFAULT_TRIGGER,
    conditions: rule?.condition ? [rule.condition] : [],
    conditionOperator: 'AND',
    actions: rule?.actions || [],
  })

  const [dslPreview, setDslPreview] = useState('')

  // Generate DSL preview
  useEffect(() => {
    const dsl = generateDSL(formData)
    setDslPreview(dsl)
  }, [formData])

  // Update form data when rule changes
  useEffect(() => {
    if (rule) {
      setFormData({
        name: rule.name || '',
        description: rule.description || '',
        enabled: rule.enabled ?? true,
        trigger: rule.trigger || DEFAULT_TRIGGER,
        conditions: rule.condition ? [rule.condition] : [],
        conditionOperator: 'AND',
        actions: rule.actions || [],
      })
    }
  }, [rule])

  const generateDSL = (data: RuleFormData): string => {
    const { conditions, conditionOperator, actions } = data

    if (conditions.length === 0 && actions.length === 0) {
      return '// Configure conditions and actions to generate DSL'
    }

    let dsl = ''

    // Generate conditions (WHEN clause)
    if (conditions.length > 0) {
      const conditionParts = conditions.map((cond, index) => {
        const prefix = index > 0 ? ` ${conditionOperator} ` : ''
        return `${prefix}device.${cond.device_id}.${cond.metric} ${cond.operator} ${cond.threshold}`
      })
      dsl += `WHEN ${conditionParts.join('')}\n`
    }

    // Generate actions (THEN clause)
    if (actions.length > 0) {
      const actionParts = actions.map((action) => {
        switch (action.type) {
          case 'Execute':
            const params = Object.keys(action.params || {}).length > 0
              ? `{${Object.entries(action.params || {}).map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(', ')}}`
              : ''
            return `device.${action.device_id}.${action.command}(${params})`
          case 'Notify':
            return `notify(${JSON.stringify(action.message)})`
          case 'Log':
            return `log(${JSON.stringify(action.message)})`
          default:
            return ''
        }
      })
      dsl += `THEN ${actionParts.join(' + ')}`
    }

    return dsl
  }

  const updateFormData = (updates: Partial<RuleFormData>) => {
    setFormData((prev) => ({ ...prev, ...updates }))
  }

  const handleAddCondition = () => {
    setFormData((prev) => ({
      ...prev,
      conditions: [...prev.conditions, { ...DEFAULT_CONDITION, id: generateId() }],
    }))
  }

  const handleUpdateCondition = (index: number, condition: RuleCondition) => {
    setFormData((prev) => {
      const newConditions = [...prev.conditions]
      newConditions[index] = condition
      return { ...prev, conditions: newConditions }
    })
  }

  const handleRemoveCondition = (index: number) => {
    setFormData((prev) => {
      const newConditions = prev.conditions.filter((_, i) => i !== index)
      return { ...prev, conditions: newConditions }
    })
  }

  const handleAddAction = () => {
    setFormData((prev) => ({
      ...prev,
      actions: [...prev.actions, { ...DEFAULT_ACTION } as RuleAction],
    }))
  }

  const handleUpdateAction = (index: number, action: RuleAction) => {
    setFormData((prev) => {
      const newActions = [...prev.actions]
      newActions[index] = action
      return { ...prev, actions: newActions }
    })
  }

  const handleRemoveAction = (index: number) => {
    setFormData((prev) => {
      const newActions = prev.actions.filter((_, i) => i !== index)
      return { ...prev, actions: newActions }
    })
  }

  const handleSave = async () => {
    if (!formData.name.trim()) {
      return
    }

    setSaving(true)
    try {
      const ruleData: Partial<Rule> = {
        id: formData.id,
        name: formData.name,
        description: formData.description,
        enabled: formData.enabled,
        trigger: formData.trigger,
        condition: formData.conditions[0] || undefined,
        actions: formData.actions,
        dsl: dslPreview,
      }
      await onSave(ruleData)
    } finally {
      setSaving(false)
    }
  }

  const handleTestRule = async () => {
    setTesting(true)
    setTestResult(null)
    try {
      // Validate DSL syntax
      const result = await api.validateRuleDSL(dslPreview)
      setTestResult({
        triggered: result.valid || false,
        message: result.valid
          ? t('automation:dslValid')
          : (result.errors?.join(', ') || t('automation:dslInvalid')),
        details: result.parsed as Record<string, unknown>,
      })
    } catch (error) {
      setTestResult({
        triggered: false,
        message: (error as Error).message || 'Test failed',
      })
    } finally {
      setTesting(false)
    }
  }

  const isValid = formData.name.trim() &&
    formData.conditions.length > 0 &&
    formData.conditions.every((c) => c.device_id && c.metric) &&
    formData.actions.length > 0 &&
    formData.actions.every((a) => {
      if (a.type === 'Execute') return a.device_id && a.command
      if (a.type === 'Notify' || a.type === 'Log') return a.message
      return false
    })

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">
            {rule ? t('automation:editRule') : t('automation:createRule')}
          </h2>
          <p className="text-sm text-muted-foreground">
            {t('automation:createRuleDesc')}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Switch
            checked={formData.enabled}
            onCheckedChange={(checked) => updateFormData({ enabled: checked })}
          />
          <span className="text-sm">{formData.enabled ? t('automation:enabled') : t('automation:disabled')}</span>
        </div>
      </div>

      {/* Basic Info */}
      <Card className="p-4 space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <Label htmlFor="rule-name">{t('automation:ruleName')}</Label>
            <Input
              id="rule-name"
              value={formData.name}
              onChange={(e) => updateFormData({ name: e.target.value })}
              placeholder={t('automation:ruleNamePlaceholder')}
            />
          </div>
        </div>
        <div>
          <Label htmlFor="rule-description">{t('automation:description')}</Label>
          <Input
            id="rule-description"
            value={formData.description}
            onChange={(e) => updateFormData({ description: e.target.value })}
            placeholder={t('automation:ruleDescription')}
          />
        </div>
      </Card>

      {/* Main Tabs */}
      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as any)} className="space-y-4">
        <TabsList className="grid w-full grid-cols-3">
          <TabsTrigger value="builder" className="flex items-center gap-2">
            <Settings className="h-4 w-4" />
            {t('automation:visualBuilder')}
          </TabsTrigger>
          <TabsTrigger value="dsl" className="flex items-center gap-2">
            <FileText className="h-4 w-4" />
            DSL
          </TabsTrigger>
          <TabsTrigger value="test" className="flex items-center gap-2">
            <Play className="h-4 w-4" />
            {t('automation:test')}
          </TabsTrigger>
        </TabsList>

        {/* Visual Builder Tab */}
        <TabsContent value="builder" className="space-y-4">
          {/* Conditions Section */}
          <Card className="p-4">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <Zap className="h-5 w-5 text-warning" />
                <h3 className="font-semibold">{t('automation:triggerConditions')}</h3>
                <Badge variant="outline">{formData.conditions.length}</Badge>
              </div>
              <Button size="sm" variant="outline" onClick={handleAddCondition}>
                <Plus className="h-4 w-4 mr-1" />
                {t('automation:addCondition')}
              </Button>
            </div>

            {formData.conditions.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Zap className="h-12 w-12 mx-auto mb-2 opacity-50" />
                <p>{t('automation:noConditions')}</p>
                <Button variant="outline" size="sm" className="mt-2" onClick={handleAddCondition}>
                  {t('automation:addFirstCondition')}
                </Button>
              </div>
            ) : (
              <div className="space-y-3">
                {formData.conditions.map((condition, index) => (
                  <ConditionBuilder
                    key={index}
                    condition={condition}
                    onChange={(cond) => handleUpdateCondition(index, cond)}
                    onRemove={() => handleRemoveCondition(index)}
                    resources={resources}
                    showOperator={index > 0}
                    operator={formData.conditionOperator}
                    onOperatorChange={(op) => updateFormData({ conditionOperator: op })}
                    canRemove={formData.conditions.length > 1}
                  />
                ))}
              </div>
            )}
          </Card>

          {/* Arrow */}
          <div className="flex justify-center">
            <ArrowRight className="h-6 w-6 text-muted-foreground" />
          </div>

          {/* Actions Section */}
          <Card className="p-4">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <Bell className="h-5 w-5 text-primary" />
                <h3 className="font-semibold">{t('automation:executeActions')}</h3>
                <Badge variant="outline">{formData.actions.length}</Badge>
              </div>
              <Button size="sm" variant="outline" onClick={handleAddAction}>
                <Plus className="h-4 w-4 mr-1" />
                {t('automation:addAction')}
              </Button>
            </div>

            {formData.actions.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Bell className="h-12 w-12 mx-auto mb-2 opacity-50" />
                <p>{t('automation:noActions')}</p>
                <Button variant="outline" size="sm" className="mt-2" onClick={handleAddAction}>
                  {t('automation:addFirstAction')}
                </Button>
              </div>
            ) : (
              <div className="space-y-3">
                {formData.actions.map((action, index) => (
                  <ActionBuilder
                    key={index}
                    action={action}
                    onChange={(act) => handleUpdateAction(index, act)}
                    onRemove={() => handleRemoveAction(index)}
                    resources={resources}
                    canRemove={formData.actions.length > 1}
                  />
                ))}
              </div>
            )}
          </Card>
        </TabsContent>

        {/* DSL Tab */}
        <TabsContent value="dsl">
          <Card className="p-4">
            <div className="flex items-center justify-between mb-4">
              <Label>{t('automation:generatedDSL')}</Label>
              <Button
                size="sm"
                variant="outline"
                onClick={() => navigator.clipboard.writeText(dslPreview)}
              >
                {t('common:copy')}
              </Button>
            </div>
            <pre className="bg-muted p-4 rounded-md text-sm font-mono overflow-x-auto min-h-[120px]">
              {dslPreview || '// Configure conditions and actions to generate DSL'}
            </pre>
            <div className="mt-4 flex items-center gap-2 text-sm">
              {isValid ? (
                <span className="flex items-center gap-1 text-green-600">
                  <Check className="h-4 w-4" />
                  {t('automation:dslValid')}
                </span>
              ) : (
                <span className="flex items-center gap-1 text-yellow-600">
                  <X className="h-4 w-4" />
                  {t('automation:incomplete')}
                </span>
              )}
            </div>
          </Card>
        </TabsContent>

        {/* Test Tab */}
        <TabsContent value="test">
          <RuleTester
            formData={formData}
            dsl={dslPreview}
            onTest={handleTestRule}
            testing={testing}
            testResult={testResult}
            resources={resources}
          />
        </TabsContent>
      </Tabs>

      {/* Footer Actions */}
      <div className="flex items-center justify-between">
        <Button variant="outline" onClick={onCancel}>
          {t('common:cancel')}
        </Button>
        <Button onClick={handleSave} disabled={!isValid || saving}>
          {saving ? (
            <>
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              {t('automation:saving')}
            </>
          ) : (
            <>
              <Save className="h-4 w-4 mr-2" />
              {t('automation:saveRule')}
            </>
          )}
        </Button>
      </div>
    </div>
  )
}
