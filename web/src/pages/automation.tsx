/**
 * NeoTalk Automation Page
 *
 * Unified automation interface with rules, workflows, and data transforms.
 * Uses PageLayout + PageTabs structure consistent with other pages.
 */

import { useState, useMemo, useCallback, useEffect } from "react"
import { useTranslation } from "react-i18next"
import {
  Zap,
  Workflow,
  GitBranch,
  Edit,
  Trash2,
  Play,
  MoreVertical,
} from "lucide-react"
import { PageLayout } from "@/components/layout/PageLayout"
import { PageTabs, PageTabsContent } from "@/components/shared"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Switch } from "@/components/ui/switch"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Card } from "@/components/ui/card"
import { api } from "@/lib/api"
import { useToast } from "@/hooks/use-toast"
import type { TransformAutomation } from "@/types"

// Import dialog components for creation/editing
import { SimpleRuleBuilder } from "@/components/automation/SimpleRuleBuilder"
import { TransformBuilder } from "@/components/automation/TransformBuilder"
import { WorkflowBuilder } from "@/components/automation/WorkflowBuilder"

type AutomationTab = 'rules' | 'workflows' | 'transforms'
type AutomationStatus = 'enabled' | 'disabled' | 'error' | 'running'

interface AutomationItem {
  id: string
  name: string
  description: string
  status: AutomationStatus
  type: 'rule' | 'workflow' | 'transform'
  triggerCount: number
  lastTriggered?: string
  icon: React.ReactNode
  category?: string
  complexity?: 'simple' | 'medium' | 'complex'
}

export function AutomationPage() {
  const { t: tCommon } = useTranslation('common')
  const { t: tAuto } = useTranslation('automation')
  const { toast } = useToast()
  const [activeTab, setActiveTab] = useState<AutomationTab>('rules')

  // Dialog states
  const [showRuleDialog, setShowRuleDialog] = useState(false)
  const [showTransformDialog, setShowTransformDialog] = useState(false)
  const [showWorkflowDialog, setShowWorkflowDialog] = useState(false)

  // Editing states
  const [editingRule, setEditingRule] = useState<any>(null)
  const [editingTransform, setEditingTransform] = useState<TransformAutomation | null>(null)
  const [editingWorkflow, setEditingWorkflow] = useState<any>(null)

  // Data state
  const [rules, setRules] = useState<any[]>([])
  const [workflows, setWorkflows] = useState<any[]>([])
  const [transforms, setTransforms] = useState<any[]>([])
  const [loading, setLoading] = useState(false)

  // Resources for dialogs
  const [devices, setDevices] = useState<Array<{ id: string; name: string; device_type?: string }>>([])
  const [deviceTypes, setDeviceTypes] = useState<any[]>([])

  // Fetch data
  const loadItems = useCallback(async () => {
    setLoading(true)
    try {
      // Load devices for all tabs
      const devicesData = await api.getDevices()
      setDevices(devicesData.devices?.map((d: any) => ({
        id: d.id,
        name: d.name,
        device_type: d.device_type
      })) || [])

      // Load device types
      try {
        const typesData = await api.getDeviceTypes()
        setDeviceTypes(typesData.device_types || [])
      } catch {
        setDeviceTypes([])
      }

      // Load tab-specific data
      if (activeTab === 'rules') {
        const data = await api.listRules()
        setRules(data.rules || [])
      } else if (activeTab === 'workflows') {
        const data = await api.listWorkflows()
        setWorkflows(data.workflows || [])
      } else if (activeTab === 'transforms') {
        const data = await api.listTransforms()
        setTransforms(data.transforms || [])
      }
    } catch (error) {
      console.error(`Failed to load ${activeTab}:`, error)
    } finally {
      setLoading(false)
    }
  }, [activeTab])

  // Load items when tab changes
  useEffect(() => {
    loadItems()
  }, [loadItems])

  // Convert API data to display items
  const displayItems = useMemo(() => {
    const sourceItems = activeTab === 'rules' ? rules : activeTab === 'workflows' ? workflows : transforms

    return sourceItems.map((item: any) => {
      let complexity: 'simple' | 'medium' | 'complex' | undefined = undefined
      if (item.complexity) {
        if (item.complexity < 3) complexity = 'simple'
        else if (item.complexity < 5) complexity = 'medium'
        else complexity = 'complex'
      }

      return {
        id: item.id,
        name: item.name,
        description: item.description || '',
        status: item.enabled ? 'enabled' : 'disabled',
        type: activeTab === 'rules' ? 'rule' : activeTab === 'workflows' ? 'workflow' : 'transform',
        triggerCount: item.trigger_count || item.execution_count || 0,
        lastTriggered: item.last_triggered || item.last_execution,
        icon: activeTab === 'rules' ? <Zap className="h-4 w-4" /> :
               activeTab === 'workflows' ? <Workflow className="h-4 w-4" /> :
               <GitBranch className="h-4 w-4" />,
        category: item.category,
        complexity,
      } as AutomationItem
    })
  }, [rules, workflows, transforms, activeTab])

  // Handlers
  const handleCreate = () => {
    if (activeTab === 'rules') {
      setEditingRule(null)
      setShowRuleDialog(true)
    } else if (activeTab === 'workflows') {
      setEditingWorkflow(null)
      setShowWorkflowDialog(true)
    } else if (activeTab === 'transforms') {
      setEditingTransform(null)
      setShowTransformDialog(true)
    }
  }

  const handleEdit = (item: AutomationItem) => {
    if (activeTab === 'rules') {
      const sourceItems = rules
      const originalItem = sourceItems.find((i: any) => i.id === item.id)
      setEditingRule(originalItem)
      setShowRuleDialog(true)
    } else if (activeTab === 'workflows') {
      const sourceItems = workflows
      const originalItem = sourceItems.find((i: any) => i.id === item.id)
      setEditingWorkflow(originalItem)
      setShowWorkflowDialog(true)
    } else if (activeTab === 'transforms') {
      const sourceItems = transforms
      const originalItem = sourceItems.find((i: any) => i.id === item.id)
      setEditingTransform(originalItem)
      setShowTransformDialog(true)
    }
  }

  const handleDelete = async (item: AutomationItem) => {
    if (!confirm(tAuto('deleteConfirm'))) return

    try {
      if (activeTab === 'rules') {
        await api.deleteRule(item.id)
      } else if (activeTab === 'workflows') {
        await api.deleteWorkflow(item.id)
      } else if (activeTab === 'transforms') {
        await api.deleteAutomation(item.id)
      }
      await loadItems()
      toast({
        title: tCommon('success'),
        description: tAuto('itemDeleted'),
      })
    } catch (error) {
      console.error('Failed to delete item:', error)
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  const handleToggleStatus = async (item: AutomationItem) => {
    try {
      const newStatus = !item.status || item.status !== 'enabled'
      if (activeTab === 'rules') {
        if (newStatus) {
          await api.enableRule(item.id)
        } else {
          await api.disableRule(item.id)
        }
      } else if (activeTab === 'workflows') {
        await api.setAutomationStatus(item.id, newStatus)
      } else if (activeTab === 'transforms') {
        await api.setAutomationStatus(item.id, newStatus)
      }
      await loadItems()
    } catch (error) {
      console.error('Failed to toggle status:', error)
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  const handleExecute = async (item: AutomationItem) => {
    try {
      if (activeTab === 'rules') {
        await api.testRule(item.id)
      } else if (activeTab === 'workflows') {
        await api.executeWorkflow(item.id)
      }
      toast({
        title: tCommon('success'),
        description: tAuto('executeSuccess'),
      })
    } catch (error) {
      console.error('Failed to execute:', error)
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
    }
  }

  // Rule save handler
  const handleSaveRule = async (rule: any) => {
    try {
      if (rule.id) {
        await api.updateRule(rule.id, rule)
      } else {
        await api.createRule(rule)
      }
      setShowRuleDialog(false)
      setEditingRule(null)
      await loadItems()
      toast({
        title: tCommon('success'),
        description: tAuto('ruleSaved'),
      })
    } catch (error) {
      console.error('Failed to save rule:', error)
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
      throw error
    }
  }

  // Transform save handler
  const handleSaveTransform = async (data: Partial<TransformAutomation>) => {
    try {
      // Transform uses unified automation API
      if (editingTransform?.id) {
        await api.updateAutomation(editingTransform.id, {
          name: data.name,
          description: data.description,
          enabled: data.enabled,
          definition: {
            scope: data.scope,
            operations: data.operations,
            js_code: data.js_code,
            output_prefix: data.output_prefix,
          },
        })
      } else {
        await api.createAutomation({
          name: data.name || '',
          description: data.description,
          type: 'transform',
          enabled: data.enabled ?? true,
          definition: {
            scope: data.scope || { type: 'global' },
            operations: data.operations || [],
            js_code: data.js_code,
            output_prefix: data.output_prefix,
          },
        })
      }
      setShowTransformDialog(false)
      setEditingTransform(null)
      await loadItems()
      toast({
        title: tCommon('success'),
        description: tAuto('transformSaved'),
      })
    } catch (error) {
      console.error('Failed to save transform:', error)
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
      throw error
    }
  }

  // Workflow save handler
  const handleSaveWorkflow = async (workflow: any) => {
    try {
      if (workflow.id) {
        await api.updateWorkflow(workflow.id, workflow)
      } else {
        await api.createWorkflow(workflow)
      }
      setShowWorkflowDialog(false)
      setEditingWorkflow(null)
      await loadItems()
      toast({
        title: tCommon('success'),
        description: tAuto('workflowSaved'),
      })
    } catch (error) {
      console.error('Failed to save workflow:', error)
      toast({
        title: tCommon('failed'),
        description: (error as Error).message,
        variant: 'destructive',
      })
      throw error
    }
  }

  const getStatusBadge = (status: AutomationStatus) => {
    const variants: Record<AutomationStatus, { variant: string; label: string }> = {
      enabled: { variant: 'default', label: tAuto('statusEnabled') },
      disabled: { variant: 'secondary', label: tAuto('statusDisabled') },
      error: { variant: 'destructive', label: tAuto('statusError') },
      running: { variant: 'outline', label: tAuto('statusRunning') },
    }
    const { variant, label } = variants[status]
    return <Badge variant={variant as any}>{label}</Badge>
  }

  const getEmptyState = () => {
    const messages = {
      rules: { title: tAuto('noRules'), desc: tAuto('noRulesDesc') },
      workflows: { title: tAuto('noWorkflows'), desc: tAuto('noWorkflowsDesc') },
      transforms: { title: tAuto('noTransforms'), desc: tAuto('noTransformsDesc') },
    }
    const msg = messages[activeTab]
    return (
      <Card className="p-12 text-center">
        <div className="flex flex-col items-center gap-4">
          <div className="p-4 rounded-full bg-muted">
            {activeTab === 'rules' && <Zap className="h-8 w-8 text-muted-foreground" />}
            {activeTab === 'workflows' && <Workflow className="h-8 w-8 text-muted-foreground" />}
            {activeTab === 'transforms' && <GitBranch className="h-8 w-8 text-muted-foreground" />}
          </div>
          <h3 className="text-lg font-semibold">{msg.title}</h3>
          <p className="text-muted-foreground">{msg.desc}</p>
        </div>
      </Card>
    )
  }

  return (
    <PageLayout
      title={tAuto('title')}
      subtitle={tAuto('description')}
    >
      {/* Tabs with Actions */}
      <PageTabs
        tabs={[
          { value: 'rules', label: tAuto('tabs.rules') },
          { value: 'workflows', label: tAuto('tabs.workflows') },
          { value: 'transforms', label: tAuto('tabs.transforms') },
        ]}
        activeTab={activeTab}
        onTabChange={(v) => setActiveTab(v as AutomationTab)}
        actions={[
          {
            label: tCommon('create'),
            onClick: handleCreate,
          },
          {
            label: tCommon('refresh'),
            variant: 'outline',
            onClick: loadItems,
            disabled: loading,
          },
        ]}
      >
        {/* Content for each tab */}
        <PageTabsContent value="rules" activeTab={activeTab}>
          {displayItems.length === 0 ? getEmptyState() : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-12"></TableHead>
                  <TableHead>{tAuto('ruleName')}</TableHead>
                  <TableHead>{tCommon('description')}</TableHead>
                  <TableHead>{tAuto('status')}</TableHead>
                  <TableHead>{tAuto('triggerCount')}</TableHead>
                  <TableHead>{tAuto('lastTriggered')}</TableHead>
                  <TableHead className="text-right">{tCommon('actions')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {displayItems.map((item, index) => (
                  <TableRow key={item.id}>
                    <TableCell>{index + 1}</TableCell>
                    <TableCell className="font-medium">{item.name}</TableCell>
                    <TableCell className="text-muted-foreground max-w-md truncate">
                      {item.description || '-'}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Switch
                          checked={item.status === 'enabled'}
                          onCheckedChange={() => handleToggleStatus(item)}
                        />
                        {getStatusBadge(item.status)}
                      </div>
                    </TableCell>
                    <TableCell>{item.triggerCount}</TableCell>
                    <TableCell>{item.lastTriggered || '-'}</TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => handleEdit(item)}>
                            <Edit className="mr-2 h-4 w-4" />
                            {tCommon('edit')}
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => handleExecute(item)}>
                            <Play className="mr-2 h-4 w-4" />
                            {tAuto('execute')}
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem onClick={() => handleDelete(item)} className="text-destructive">
                            <Trash2 className="mr-2 h-4 w-4" />
                            {tCommon('delete')}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </PageTabsContent>

        <PageTabsContent value="workflows" activeTab={activeTab}>
          {displayItems.length === 0 ? getEmptyState() : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-12"></TableHead>
                  <TableHead>{tAuto('workflowName')}</TableHead>
                  <TableHead>{tCommon('description')}</TableHead>
                  <TableHead>{tAuto('status')}</TableHead>
                  <TableHead>{tAuto('executionCount')}</TableHead>
                  <TableHead>{tAuto('updatedAt')}</TableHead>
                  <TableHead className="text-right">{tCommon('actions')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {displayItems.map((item, index) => (
                  <TableRow key={item.id}>
                    <TableCell>{index + 1}</TableCell>
                    <TableCell className="font-medium">{item.name}</TableCell>
                    <TableCell className="text-muted-foreground max-w-md truncate">
                      {item.description || '-'}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Switch
                          checked={item.status === 'enabled'}
                          onCheckedChange={() => handleToggleStatus(item)}
                        />
                        {getStatusBadge(item.status)}
                      </div>
                    </TableCell>
                    <TableCell>{item.triggerCount}</TableCell>
                    <TableCell>{item.lastTriggered || '-'}</TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => handleEdit(item)}>
                            <Edit className="mr-2 h-4 w-4" />
                            {tCommon('edit')}
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => handleExecute(item)}>
                            <Play className="mr-2 h-4 w-4" />
                            {tAuto('execute')}
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem onClick={() => handleDelete(item)} className="text-destructive">
                            <Trash2 className="mr-2 h-4 w-4" />
                            {tCommon('delete')}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </PageTabsContent>

        <PageTabsContent value="transforms" activeTab={activeTab}>
          {displayItems.length === 0 ? getEmptyState() : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-12"></TableHead>
                  <TableHead>{tAuto('name')}</TableHead>
                  <TableHead>{tAuto('scope')}</TableHead>
                  <TableHead>{tCommon('description')}</TableHead>
                  <TableHead>{tAuto('status')}</TableHead>
                  <TableHead className="text-right">{tCommon('actions')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {displayItems.map((item, index) => (
                  <TableRow key={item.id}>
                    <TableCell>{index + 1}</TableCell>
                    <TableCell className="font-medium">{item.name}</TableCell>
                    <TableCell>
                      <Badge variant="outline">{item.category || 'global'}</Badge>
                    </TableCell>
                    <TableCell className="text-muted-foreground max-w-md truncate">
                      {item.description || '-'}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Switch
                          checked={item.status === 'enabled'}
                          onCheckedChange={() => handleToggleStatus(item)}
                        />
                        {getStatusBadge(item.status)}
                      </div>
                    </TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => handleEdit(item)}>
                            <Edit className="mr-2 h-4 w-4" />
                            {tCommon('edit')}
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem onClick={() => handleDelete(item)} className="text-destructive">
                            <Trash2 className="mr-2 h-4 w-4" />
                            {tCommon('delete')}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </PageTabsContent>
      </PageTabs>

      {/* Rule Builder Dialog */}
      <SimpleRuleBuilder
        open={showRuleDialog}
        onOpenChange={setShowRuleDialog}
        rule={editingRule}
        onSave={handleSaveRule}
        resources={{ devices, deviceTypes }}
      />

      {/* Transform Builder Dialog */}
      <TransformBuilder
        open={showTransformDialog}
        onOpenChange={setShowTransformDialog}
        transform={editingTransform}
        devices={devices}
        onSave={handleSaveTransform}
      />

      {/* Workflow Builder Dialog */}
      {showWorkflowDialog && (
        <Dialog open={showWorkflowDialog} onOpenChange={setShowWorkflowDialog}>
          <DialogContent className="max-w-5xl max-h-[90vh] overflow-hidden flex flex-col p-0 gap-0">
            <DialogHeader className="px-6 pb-4 pt-6 border-t-0 border-x-0 border-b shrink-0">
              <DialogTitle>
                {editingWorkflow ? tAuto('editWorkflow') : tAuto('createWorkflow')}
              </DialogTitle>
              <DialogDescription>
                {tAuto('workflowBuilderDesc')}
              </DialogDescription>
            </DialogHeader>

            <div className="flex-1 min-h-0 overflow-auto px-6 py-4">
              <WorkflowBuilder
                workflow={editingWorkflow}
                onSave={handleSaveWorkflow}
                onCancel={() => setShowWorkflowDialog(false)}
                resources={{ devices, metrics: ['temperature', 'humidity'], alertChannels: [] }}
              />
            </div>
          </DialogContent>
        </Dialog>
      )}
    </PageLayout>
  )
}

// Export as default for the route
export default AutomationPage
