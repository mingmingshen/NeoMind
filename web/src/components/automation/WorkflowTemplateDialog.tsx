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
import { api } from '@/lib/api'
import type { WorkflowTemplate, GeneratedWorkflow, WorkflowResources } from '@/types'
import { Loader2, Sparkles, FileText, Wand2, CheckCircle2, AlertTriangle } from 'lucide-react'
import { cn } from '@/lib/utils'

export interface WorkflowTemplateDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onWorkflowCreated: () => void
}

type TabValue = 'templates' | 'generate' | 'import'

export function WorkflowTemplateDialog({
  open,
  onOpenChange,
  onWorkflowCreated,
}: WorkflowTemplateDialogProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [activeTab, setActiveTab] = useState<TabValue>('templates')
  const [templates, setTemplates] = useState<WorkflowTemplate[]>([])
  const [selectedTemplate, setSelectedTemplate] = useState<WorkflowTemplate | null>(null)
  const [templateParams, setTemplateParams] = useState<Record<string, string>>({})
  const [loading, setLoading] = useState(false)
  const [creating, setCreating] = useState(false)

  // Generator state
  const [description, setDescription] = useState('')
  const [generating, setGenerating] = useState(false)
  const [generatedWorkflow, setGeneratedWorkflow] = useState<GeneratedWorkflow | null>(null)

  // Import/Export state
  const [importJson, setImportJson] = useState('')
  const [exportJson, setExportJson] = useState('')
  const [loadingExport, setLoadingExport] = useState(false)

  // Resources
  const [resources, setResources] = useState<WorkflowResources | null>(null)

  useEffect(() => {
    if (open) {
      loadTemplates()
      loadResources()
    }
  }, [open])

  const loadTemplates = async () => {
    setLoading(true)
    try {
      const result = await api.getWorkflowTemplates()
      setTemplates(result.templates || [])
    } catch (error) {
      console.error('Failed to load templates:', error)
    } finally {
      setLoading(false)
    }
  }

  const loadResources = async () => {
    try {
      const result = await api.getWorkflowResources()
      setResources(result)
    } catch (error) {
      console.error('Failed to load resources:', error)
    }
  }

  const loadExport = async () => {
    setLoadingExport(true)
    try {
      const result = await api.exportWorkflows()
      setExportJson(JSON.stringify(result, null, 2))
    } catch (error) {
      console.error('Failed to export workflows:', error)
    } finally {
      setLoadingExport(false)
    }
  }

  const handleSelectTemplate = (template: WorkflowTemplate) => {
    setSelectedTemplate(template)
    // Initialize params with defaults
    const params: Record<string, string> = {}
    for (const param of template.parameters) {
      if (param.default) {
        params[param.name] = param.default
      }
    }
    setTemplateParams(params)
  }

  const handleParamChange = (name: string, value: string) => {
    setTemplateParams((prev) => ({ ...prev, [name]: value }))
  }

  const handleCreateFromTemplate = async () => {
    if (!selectedTemplate) return

    // Check required params
    for (const param of selectedTemplate.parameters) {
      if (param.required && !templateParams[param.name]) {
        alert(`${t('automation:missingRequiredParam')}: ${param.label}`)
        return
      }
    }

    setCreating(true)
    try {
      const result = await api.fillWorkflowTemplate(selectedTemplate.id, templateParams)
      const workflow = JSON.parse(result.workflow_json)
      await api.createWorkflow({
        ...workflow,
        enabled: false,
        status: 'active',
        step_count: workflow.steps?.length || 0,
        trigger_count: workflow.triggers?.length || 0,
      })
      onOpenChange(false)
      onWorkflowCreated()
    } catch (error) {
      console.error('Failed to create workflow from template:', error)
    } finally {
      setCreating(false)
    }
  }

  const handleGenerate = async () => {
    if (!description.trim()) return

    setGenerating(true)
    try {
      const result = await api.generateWorkflow(description)
      setGeneratedWorkflow(result)
    } catch (error) {
      console.error('Failed to generate workflow:', error)
    } finally {
      setGenerating(false)
    }
  }

  const handleCreateFromGenerated = async () => {
    if (!generatedWorkflow) return

    setCreating(true)
    try {
      const workflow = JSON.parse(generatedWorkflow.workflow_json)
      await api.createWorkflow({
        ...workflow,
        enabled: false,
        status: 'active',
        step_count: workflow.steps?.length || 0,
        trigger_count: workflow.triggers?.length || 0,
      })
      onOpenChange(false)
      onWorkflowCreated()
    } catch (error) {
      console.error('Failed to create generated workflow:', error)
    } finally {
      setCreating(false)
    }
  }

  const handleImport = async () => {
    try {
      const data = JSON.parse(importJson)
      const workflows = data.workflows || data.rules || []
      const result = await api.importWorkflows(workflows)
      if (result.imported > 0) {
        alert(`${t('automation:imported')}: ${result.imported}`)
        onOpenChange(false)
        onWorkflowCreated()
      } else if (result.errors.length > 0) {
        alert(`${t('automation:importFailed')}: ${result.errors.map((e) => e.error).join(', ')}`)
      }
    } catch (error) {
      console.error('Failed to import workflows:', error)
      alert(t('automation:invalidJson'))
    }
  }

  const groupedTemplates = templates.reduce((acc, template) => {
    if (!acc[template.category]) {
      acc[template.category] = []
    }
    acc[template.category].push(template)
    return acc
  }, {} as Record<string, WorkflowTemplate[]>)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            {t('automation:createWorkflow')}
          </DialogTitle>
          <DialogDescription>
            {t('automation:createWorkflowDesc')}
          </DialogDescription>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as TabValue)} className="flex-1 overflow-hidden flex flex-col">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="templates" className="flex items-center gap-2">
              <FileText className="h-4 w-4" />
              {t('automation:templates')}
            </TabsTrigger>
            <TabsTrigger value="generate" className="flex items-center gap-2">
              <Wand2 className="h-4 w-4" />
              {t('automation:aiGenerate')}
            </TabsTrigger>
            <TabsTrigger value="import" className="flex items-center gap-2">
              <Sparkles className="h-4 w-4" />
              {t('automation:importExport')}
            </TabsTrigger>
          </TabsList>

          {/* Templates Tab */}
          <TabsContent value="templates" className="flex-1 overflow-hidden flex flex-col mt-4">
            <div className="flex gap-4 h-full overflow-hidden">
              {/* Template List */}
              <div className="w-1/2 overflow-y-auto pr-2">
                {loading ? (
                  <div className="flex items-center justify-center h-40">
                    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                  </div>
                ) : (
                  <div className="space-y-4">
                    {Object.entries(groupedTemplates).map(([category, categoryTemplates]) => (
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
                              <div className="font-medium">{template.name}</div>
                              <div className="text-xs text-muted-foreground mt-1">
                                {template.description}
                              </div>
                              <div className="flex flex-wrap gap-1 mt-2">
                                {template.parameters.slice(0, 3).map((param) => (
                                  <Badge key={param.name} variant="secondary" className="text-xs">
                                    {param.label}
                                  </Badge>
                                ))}
                                {template.parameters.length > 3 && (
                                  <Badge variant="secondary" className="text-xs">
                                    +{template.parameters.length - 3}
                                  </Badge>
                                )}
                              </div>
                            </Card>
                          ))}
                        </div>
                      </div>
                    ))}
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

                          {param.param_type === 'enum' && param.options.length > 0 ? (
                            <Select
                              value={templateParams[param.name] || ''}
                              onValueChange={(value) => handleParamChange(param.name, value)}
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
                          ) : param.param_type === 'device' ? (
                            <Select
                              value={templateParams[param.name] || ''}
                              onValueChange={(value) => handleParamChange(param.name, value)}
                            >
                              <SelectTrigger id={`param-${param.name}`}>
                                <SelectValue placeholder={t('automation:selectDevice')} />
                              </SelectTrigger>
                              <SelectContent>
                                {resources?.devices.map((device) => (
                                  <SelectItem key={device.id} value={device.id}>
                                    {device.name}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          ) : param.param_type === 'metric' ? (
                            <Select
                              value={templateParams[param.name] || ''}
                              onValueChange={(value) => handleParamChange(param.name, value)}
                            >
                              <SelectTrigger id={`param-${param.name}`}>
                                <SelectValue placeholder={t('automation:selectMetric')} />
                              </SelectTrigger>
                              <SelectContent>
                                {resources?.metrics.map((metric) => (
                                  <SelectItem key={metric} value={metric}>
                                    {metric}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          ) : param.param_type === 'number' ? (
                            <Input
                              id={`param-${param.name}`}
                              type="number"
                              value={templateParams[param.name] || ''}
                              onChange={(e) => handleParamChange(param.name, e.target.value)}
                              placeholder={param.default || ''}
                            />
                          ) : param.param_type === 'boolean' ? (
                            <Select
                              value={templateParams[param.name] || ''}
                              onValueChange={(value) => handleParamChange(param.name, value)}
                            >
                              <SelectTrigger id={`param-${param.name}`}>
                                <SelectValue placeholder={t('automation:selectPlaceholder')} />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="true">{t('common:yes')}</SelectItem>
                                <SelectItem value="false">{t('common:no')}</SelectItem>
                              </SelectContent>
                            </Select>
                          ) : (
                            <Input
                              id={`param-${param.name}`}
                              value={templateParams[param.name] || ''}
                              onChange={(e) => handleParamChange(param.name, e.target.value)}
                              placeholder={param.default || ''}
                            />
                          )}
                        </div>
                      ))}
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

          {/* AI Generate Tab */}
          <TabsContent value="generate" className="flex-1 overflow-y-auto mt-4 space-y-4">
            <div className="space-y-4">
              <div>
                <Label htmlFor="workflow-description">{t('automation:describeWorkflow')}</Label>
                <Textarea
                  id="workflow-description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder={t('automation:workflowDescriptionPlaceholder')}
                  className="min-h-[100px] mt-2"
                />
              </div>

              <Button
                onClick={handleGenerate}
                disabled={!description.trim() || generating}
                className="w-full"
              >
                {generating ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    {t('automation:generating')}
                  </>
                ) : (
                  <>
                    <Wand2 className="h-4 w-4 mr-2" />
                    {t('automation:generateWorkflow')}
                  </>
                )}
              </Button>

              {generatedWorkflow && (
                <Card className="p-4 space-y-3">
                  <div className="flex items-center gap-2 text-sm">
                    <CheckCircle2 className="h-4 w-4 text-green-500" />
                    <span className="font-medium">{t('automation:generated')}</span>
                    <Badge variant="outline">
                      {Math.round(generatedWorkflow.confidence * 100)}% {t('automation:confidence')}
                    </Badge>
                  </div>

                  <div className="text-sm text-muted-foreground">
                    {generatedWorkflow.explanation}
                  </div>

                  {generatedWorkflow.warnings.length > 0 && (
                    <div className="space-y-1">
                      {generatedWorkflow.warnings.map((warning, i) => (
                        <div key={i} className="flex items-center gap-2 text-sm text-amber-600">
                          <AlertTriangle className="h-3 w-3" />
                          {warning}
                        </div>
                      ))}
                    </div>
                  )}

                  <details className="text-sm">
                    <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
                      {t('automation:viewWorkflowJson')}
                    </summary>
                    <pre className="mt-2 p-2 bg-muted rounded text-xs overflow-x-auto">
                      {JSON.stringify(JSON.parse(generatedWorkflow.workflow_json), null, 2)}
                    </pre>
                  </details>

                  <Button onClick={handleCreateFromGenerated} disabled={creating} className="w-full">
                    {creating ? (
                      <>
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                        {t('automation:creating')}
                      </>
                    ) : (
                      t('automation:useThisWorkflow')
                    )}
                  </Button>
                </Card>
              )}
            </div>
          </TabsContent>

          {/* Import/Export Tab */}
          <TabsContent value="import" className="flex-1 overflow-y-auto mt-4 space-y-6">
            {/* Export */}
            <div className="space-y-3">
              <h3 className="font-medium">{t('automation:exportWorkflows')}</h3>
              <Button onClick={loadExport} disabled={loadingExport} variant="outline">
                {loadingExport ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    {t('automation:loading')}
                  </>
                ) : (
                  <>
                    <Sparkles className="h-4 w-4 mr-2" />
                    {t('automation:export')}
                  </>
                )}
              </Button>
              {exportJson && (
                <Textarea
                  value={exportJson}
                  onChange={(e) => setExportJson(e.target.value)}
                  className="min-h-[150px] font-mono text-xs"
                  readOnly
                />
              )}
            </div>

            {/* Import */}
            <div className="space-y-3">
              <h3 className="font-medium">{t('automation:importWorkflows')}</h3>
              <Textarea
                value={importJson}
                onChange={(e) => setImportJson(e.target.value)}
                placeholder={t('automation:pasteWorkflowJson')}
                className="min-h-[150px] font-mono text-xs"
              />
              <Button onClick={handleImport} disabled={!importJson.trim()}>
                {t('automation:import')}
              </Button>
            </div>
          </TabsContent>
        </Tabs>

        <DialogFooter className="border-t pt-4">
          {activeTab === 'templates' && selectedTemplate && (
            <Button onClick={handleCreateFromTemplate} disabled={creating}>
              {creating ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  {t('automation:creating')}
                </>
              ) : (
                t('automation:createWorkflow')
              )}
            </Button>
          )}
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('automation:cancel')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
