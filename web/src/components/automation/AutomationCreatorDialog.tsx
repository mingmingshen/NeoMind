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
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Loader2, Sparkles, Wand2, CheckCircle2, AlertTriangle, Zap, Workflow as WorkflowIcon } from 'lucide-react'
import { cn } from '@/lib/utils'
import { api } from '@/lib/api'
import { useToast } from '@/hooks/use-toast'
import type {
  AutomationType,
  CreateAutomationRequest,
  IntentResult,
  AutomationTemplate,
} from '@/types'

export interface AutomationCreatorDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onAutomationCreated: () => void
  initialDescription?: string
  suggestedType?: AutomationType
}

type TabValue = 'describe' | 'templates' | 'manual'

export function AutomationCreatorDialog({
  open,
  onOpenChange,
  onAutomationCreated,
  initialDescription = '',
  suggestedType,
}: AutomationCreatorDialogProps) {
  const { t } = useTranslation(['common', 'automation', 'devices'])
  const { toast } = useToast()
  const [activeTab, setActiveTab] = useState<TabValue>('describe')

  // Describe tab state
  const [description, setDescription] = useState(initialDescription)
  const [analyzing, setAnalyzing] = useState(false)
  const [intentResult, setIntentResult] = useState<IntentResult | null>(null)
  const [selectedType, setSelectedType] = useState<AutomationType | null>(null)
  const [automationName, setAutomationName] = useState('')
  const [creating, setCreating] = useState(false)

  // Templates tab state
  const [templates, setTemplates] = useState<AutomationTemplate[]>([])
  const [selectedTemplate, setSelectedTemplate] = useState<AutomationTemplate | null>(null)
  const [templateParams, setTemplateParams] = useState<Record<string, string>>({})
  const [loadingTemplates, setLoadingTemplates] = useState(false)

  // Resources for templates
  const [devices, setDevices] = useState<Array<{ id: string; name: string }>>([])

  // Reset state when dialog opens/closes
  useEffect(() => {
    if (open) {
      setDescription(initialDescription)
      setSelectedType(suggestedType || null)
      setAutomationName('')
      setIntentResult(null)
      setSelectedTemplate(null)
      setTemplateParams({})
      // If initial description is provided, analyze it automatically
      if (initialDescription) {
        analyzeIntent()
      }
    }
  }, [open, initialDescription, suggestedType])

  // Load devices and templates when dialog opens
  useEffect(() => {
    if (open && activeTab === 'templates') {
      loadResources()
    }
  }, [open, activeTab])

  const analyzeIntent = async () => {
    if (!description.trim()) return

    setAnalyzing(true)
    try {
      const result = await api.analyzeAutomationIntent(description)
      setIntentResult(result)
      if (result.recommended_type) {
        setSelectedType(result.recommended_type)
      }
      // Generate a name from the description
      if (!automationName) {
        const words = description.split(' ').slice(0, 4)
        setAutomationName(words.join(' ').replace(/[^a-zA-Z0-9\s]/g, '').trim())
      }
    } catch (error) {
      console.error('Failed to analyze intent:', error)
      toast({
        title: t('common:failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    } finally {
      setAnalyzing(false)
    }
  }

  const loadResources = async () => {
    setLoadingTemplates(true)
    try {
      const [templatesResult, devicesResult] = await Promise.all([
        api.listAutomationTemplates(),
        api.getDevices(),
      ])
      setTemplates((templatesResult.templates || []) as AutomationTemplate[])
      setDevices(devicesResult.devices?.map((d: any) => ({ id: d.id, name: d.name })) || [])
    } catch (error) {
      console.error('Failed to load resources:', error)
      toast({
        title: t('common:failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    } finally {
      setLoadingTemplates(false)
    }
  }

  const handleCreateFromIntent = async () => {
    if (!selectedType || !automationName.trim()) {
      toast({
        title: t('automation:nameRequired', { defaultValue: 'Automation name is required' }),
        variant: 'destructive',
      })
      return
    }

    setCreating(true)
    try {
      // Create automation based on analyzed intent
      const definition = generateDefinitionFromIntent()
      const request: CreateAutomationRequest = {
        name: automationName,
        description: description,
        type: selectedType,
        enabled: true,
        definition,
      }
      await api.createAutomation(request)
      toast({
        title: t('common:success'),
        description: t('automation:automationCreated', { defaultValue: 'Automation created successfully' }),
      })
      onOpenChange(false)
      onAutomationCreated()
    } catch (error) {
      console.error('Failed to create automation:', error)
      toast({
        title: t('common:failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    } finally {
      setCreating(false)
    }
  }

  const generateDefinitionFromIntent = (): Record<string, unknown> => {
    // Generate a basic automation definition from the intent
    if (selectedType === 'transform') {
      return {
        scope: { type: 'global' },
        operations: [
          {
            type: 'single',
            json_path: '$.value',
            output_metric: 'processed_value',
          },
        ],
      }
    } else if (selectedType === 'rule') {
      return {
        trigger: { type: 'manual' },
        condition: null, // Will be filled by AI
        actions: [
          {
            type: 'Notify',
            message: description,
          },
        ],
      }
    } else {
      return {
        triggers: [{ type: 'manual', config: {} }],
        steps: [
          {
            id: 'step-1',
            name: 'Notification',
            type: 'notification',
            config: {
              message: description,
            },
          },
        ],
        variables: {},
      }
    }
  }

  const handleSelectTemplate = (template: AutomationTemplate) => {
    setSelectedTemplate(template)
    // Initialize params with defaults
    const params: Record<string, string> = {}
    for (const param of template.parameters) {
      if (param.default) {
        params[param.name] = param.default
      }
    }
    setTemplateParams(params)
    // Generate name from template
    if (!automationName) {
      setAutomationName(template.name)
    }
  }

  const groupedTemplates = templates.reduce((acc, template) => {
    const category = template.category || 'general'
    if (!acc[category]) {
      acc[category] = []
    }
    acc[category].push(template)
    return acc
  }, {} as Record<string, AutomationTemplate[]>)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5" />
            {t('automation:createAutomation')}
          </DialogTitle>
          <DialogDescription>
            {t('automation:createAutomationDesc')}
          </DialogDescription>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as TabValue)} className="flex-1 overflow-hidden flex flex-col">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="describe" className="flex items-center gap-2">
              <Wand2 className="h-4 w-4" />
              {t('automation:naturalLanguage')}
            </TabsTrigger>
            <TabsTrigger value="templates" className="flex items-center gap-2">
              <Sparkles className="h-4 w-4" />
              {t('automation:templates')}
            </TabsTrigger>
            <TabsTrigger value="manual" className="flex items-center gap-2">
              <Zap className="h-4 w-4" />
              {t('automation:manual')}
            </TabsTrigger>
          </TabsList>

          {/* Describe Tab - AI-powered creation */}
          <TabsContent value="describe" className="flex-1 overflow-y-auto mt-4 space-y-4">
            <div className="space-y-4">
              <div>
                <Label htmlFor="automation-description">{t('automation:describeAutomation')}</Label>
                <Textarea
                  id="automation-description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder={t('automation:automationDescriptionPlaceholder')}
                  className="min-h-[120px] mt-2"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  {t('automation:automationDescriptionHint')}
                </p>
              </div>

              <Button
                onClick={analyzeIntent}
                disabled={!description.trim() || analyzing}
                variant="outline"
                className="w-full"
              >
                {analyzing ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    {t('automation:analyzing')}
                  </>
                ) : (
                  <>
                    <Wand2 className="h-4 w-4 mr-2" />
                    {t('automation:analyzeIntent')}
                  </>
                )}
              </Button>

              {intentResult && (
                <Card className="p-4 space-y-3">
                  <div className="flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4 text-green-500" />
                    <span className="font-medium">{t('automation:analysisComplete')}</span>
                    <Badge variant="outline">
                      {intentResult.confidence}% {t('automation:confidence')}
                    </Badge>
                  </div>

                  <div className="text-sm">
                    <p className="text-muted-foreground">{intentResult.reasoning}</p>
                  </div>

                  <div className="flex items-center gap-2">
                    <Label>{t('automation:recommendedType')}:</Label>
                    <Badge
                      variant={intentResult.recommended_type === 'rule' ? 'default' : 'secondary'}
                      className={cn(
                        intentResult.recommended_type === 'rule' ? 'bg-blue-500/10 text-blue-500' : 'bg-purple-500/10 text-purple-500'
                      )}
                    >
                      {intentResult.recommended_type === 'rule' ? (
                        <>
                          <Zap className="h-3 w-3 mr-1 inline" />
                          {t('automation:rule')}
                        </>
                      ) : (
                        <>
                          <WorkflowIcon className="h-3 w-3 mr-1 inline" />
                          {t('automation:workflow')}
                        </>
                      )}
                    </Badge>
                  </div>

                  {intentResult.warnings.length > 0 && (
                    <div className="space-y-1">
                      {intentResult.warnings.map((warning, i) => (
                        <div key={i} className="flex items-center gap-2 text-sm text-amber-600">
                          <AlertTriangle className="h-3 w-3" />
                          {warning}
                        </div>
                      ))}
                    </div>
                  )}

                  <div className="space-y-2">
                    <Label htmlFor="automation-name">{t('automation:automationName')}</Label>
                    <Input
                      id="automation-name"
                      value={automationName}
                      onChange={(e) => setAutomationName(e.target.value)}
                      placeholder={t('automation:automationNamePlaceholder')}
                    />
                  </div>

                  <div className="flex items-center gap-2">
                    <Label htmlFor="auto-enable">{t('automation:enableAfterCreate')}</Label>
                    <Switch id="auto-enable" defaultChecked />
                  </div>

                  <Button onClick={handleCreateFromIntent} disabled={creating || !automationName.trim()} className="w-full">
                    {creating ? (
                      <>
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                        {t('automation:creating')}
                      </>
                    ) : (
                      t('automation:createAutomation')
                    )}
                  </Button>
                </Card>
              )}
            </div>
          </TabsContent>

          {/* Templates Tab */}
          <TabsContent value="templates" className="flex-1 overflow-hidden flex flex-col mt-4">
            <div className="flex gap-4 h-full overflow-hidden">
              {/* Template List */}
              <div className="w-1/2 overflow-y-auto pr-2">
                {loadingTemplates ? (
                  <div className="flex items-center justify-center h-40">
                    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                  </div>
                ) : (
                  <div className="space-y-4">
                    {Object.keys(groupedTemplates).length === 0 ? (
                      <div className="text-center text-muted-foreground py-8">
                        {t('automation:noTemplates')}
                      </div>
                    ) : (
                      Object.entries(groupedTemplates).map(([category, categoryTemplates]) => (
                        <div key={category}>
                          <h4 className="text-sm font-medium capitalize mb-2 text-muted-foreground">
                            {category}
                          </h4>
                          <div className="space-y-2">
                            {categoryTemplates.map((template) => (
                              <Card
                                key={template.id}
                                className={cn(
                                  'p-3 cursor-pointer hover:bg-accent transition-colors',
                                  selectedTemplate?.id === template.id && 'bg-accent border-primary'
                                )}
                                onClick={() => handleSelectTemplate(template)}
                              >
                                <div className="flex items-center gap-2">
                                  <Badge
                                    variant="outline"
                                    className={cn(
                                      template.automation_type === 'rule'
                                        ? 'bg-blue-500/10 text-blue-500'
                                        : 'bg-purple-500/10 text-purple-500'
                                    )}
                                  >
                                    {template.automation_type === 'rule' ? (
                                      <Zap className="h-3 w-3 mr-1" />
                                    ) : (
                                      <WorkflowIcon className="h-3 w-3 mr-1" />
                                    )}
                                    {template.automation_type}
                                  </Badge>
                                  <span className="font-medium">{template.name}</span>
                                </div>
                                <div className="text-xs text-muted-foreground mt-1">
                                  {template.description}
                                </div>
                              </Card>
                            ))}
                          </div>
                        </div>
                      ))
                    )}
                  </div>
                )}
              </div>

              {/* Template Parameters */}
              <div className="w-1/2 overflow-y-auto pl-2 border-l">
                {selectedTemplate ? (
                  <div className="space-y-4">
                    <div>
                      <h3 className="font-medium">{selectedTemplate.name}</h3>
                      <p className="text-sm text-muted-foreground mt-1">
                        {selectedTemplate.description}
                      </p>
                    </div>

                    <div className="space-y-3">
                      {selectedTemplate.parameters.map((param) => (
                        <div key={param.name}>
                          <Label htmlFor={`param-${param.name}`} className="flex items-center gap-1">
                            {param.label}
                            {param.required && <span className="text-destructive">*</span>}
                          </Label>

                          {param.param_type === 'device' ? (
                            <Select
                              value={templateParams[param.name] || ''}
                              onValueChange={(value) =>
                                setTemplateParams((prev) => ({ ...prev, [param.name]: value }))
                              }
                            >
                              <SelectTrigger id={`param-${param.name}`}>
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
                          ) : param.param_type === 'enum' && param.options?.length > 0 ? (
                            <Select
                              value={templateParams[param.name] || ''}
                              onValueChange={(value) =>
                                setTemplateParams((prev) => ({ ...prev, [param.name]: value }))
                              }
                            >
                              <SelectTrigger id={`param-${param.name}`}>
                                <SelectValue placeholder={t('automation:selectPlaceholder')} />
                              </SelectTrigger>
                              <SelectContent>
                                {param.options.map((option) => (
                                  <SelectItem key={option} value={option}>
                                    {option}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          ) : param.param_type === 'number' ? (
                            <Input
                              id={`param-${param.name}`}
                              type="number"
                              value={templateParams[param.name] || ''}
                              onChange={(e) =>
                                setTemplateParams((prev) => ({ ...prev, [param.name]: e.target.value }))
                              }
                            />
                          ) : param.param_type === 'boolean' ? (
                            <div className="flex items-center gap-2">
                              <Switch
                                id={`param-${param.name}`}
                                checked={templateParams[param.name] === 'true'}
                                onCheckedChange={(checked) =>
                                  setTemplateParams((prev) => ({ ...prev, [param.name]: checked.toString() }))
                                }
                              />
                              <Label htmlFor={`param-${param.name}`} className="text-sm text-muted-foreground">
                                {templateParams[param.name] === 'true' ? t('common:yes') : t('common:no')}
                              </Label>
                            </div>
                          ) : (
                            <Input
                              id={`param-${param.name}`}
                              value={templateParams[param.name] || ''}
                              onChange={(e) =>
                                setTemplateParams((prev) => ({ ...prev, [param.name]: e.target.value }))
                              }
                              placeholder={param.default || ''}
                            />
                          )}
                        </div>
                      ))}
                    </div>

                    <div className="space-y-2 pt-4 border-t">
                      <Label htmlFor="template-automation-name">{t('automation:automationName')}</Label>
                      <Input
                        id="template-automation-name"
                        value={automationName}
                        onChange={(e) => setAutomationName(e.target.value)}
                        placeholder={t('automation:automationNamePlaceholder')}
                      />
                    </div>
                  </div>
                ) : (
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    {t('automation:selectTemplate')}
                  </div>
                )}
              </div>
            </div>
          </TabsContent>

          {/* Manual Tab - Advanced */}
          <TabsContent value="manual" className="flex-1 overflow-y-auto mt-4 space-y-4">
            <div className="space-y-4">
              <p className="text-sm text-muted-foreground">
                {t('automation:manualCreationDesc')}
              </p>

              <div className="flex items-center gap-4">
                <div className="flex-1">
                  <Label>{t('automation:automationType')}</Label>
                  <Select
                    value={selectedType || ''}
                    onValueChange={(v) => setSelectedType(v as AutomationType)}
                  >
                    <SelectTrigger className="mt-2">
                      <SelectValue placeholder={t('automation:selectType')} />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="rule">
                        <div className="flex items-center gap-2">
                          <Zap className="h-4 w-4" />
                          {t('automation:rule')}
                        </div>
                      </SelectItem>
                      <SelectItem value="workflow">
                        <div className="flex items-center gap-2">
                          <WorkflowIcon className="h-4 w-4" />
                          {t('automation:workflow')}
                        </div>
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div>
                <Label htmlFor="manual-name">{t('automation:automationName')}</Label>
                <Input
                  id="manual-name"
                  value={automationName}
                  onChange={(e) => setAutomationName(e.target.value)}
                  placeholder={t('automation:automationNamePlaceholder')}
                  className="mt-2"
                />
              </div>

              <div>
                <Label htmlFor="manual-description">{t('common:description')}</Label>
                <Textarea
                  id="manual-description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder={t('automation:descriptionPlaceholder')}
                  className="mt-2 min-h-[80px]"
                />
              </div>

              <div className="p-4 bg-muted rounded-md">
                <p className="text-sm font-medium mb-2">{t('automation:manualCreationNote')}</p>
                <p className="text-xs text-muted-foreground">
                  {t('automation:manualCreationHint')}
                </p>
              </div>

              <Button
                onClick={() => {
                  // Switch to legacy creation based on type
                  if (selectedType === 'rule') {
                    // Navigate to rules creation
                    window.location.hash = '#automation?tab=rules&action=create'
                  } else {
                    // Navigate to workflows creation
                    window.location.hash = '#automation?tab=workflows&action=create'
                  }
                  onOpenChange(false)
                }}
                disabled={!selectedType}
                className="w-full"
              >
                {t('automation:continueToManual')}
              </Button>
            </div>
          </TabsContent>
        </Tabs>

        <DialogFooter className="border-t pt-4">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('automation:cancel')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
