import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Switch } from '@/components/ui/switch'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Plus, Trash2, Edit, Play, ArrowRight, ChevronDown, ChevronUp } from 'lucide-react'
import { ActionBar, EmptyStateInline } from '@/components/shared'
import { api } from '@/lib/api'
import type { Workflow, WorkflowStep } from '@/types'
import { cn } from '@/lib/utils'

interface WorkflowsTabProps {
  onRefresh?: () => void
}

export function WorkflowsTab({ onRefresh }: WorkflowsTabProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [workflows, setWorkflows] = useState<Workflow[]>([])
  const [loading, setLoading] = useState(true)
  const [createDialogOpen, setCreateDialogOpen] = useState(false)
  const [editWorkflow, setEditWorkflow] = useState<Workflow | null>(null)
  const [newWorkflowName, setNewWorkflowName] = useState('')
  const [newWorkflowDescription, setNewWorkflowDescription] = useState('')
  const [executingId, setExecutingId] = useState<string | null>(null)
  const [expandedDetails, setExpandedDetails] = useState<Set<string>>(new Set())

  const fetchWorkflows = async () => {
    setLoading(true)
    try {
      const result = await api.listWorkflows()
      setWorkflows(result.workflows || [])
    } catch (error) {
      console.error('Failed to fetch workflows:', error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchWorkflows()
  }, [])

  const handleToggleWorkflow = async (workflow: Workflow) => {
    try {
      await api.updateWorkflow(workflow.id, {
        enabled: !workflow.enabled,
      })
      await fetchWorkflows()
      onRefresh?.()
    } catch (error) {
      console.error('Failed to toggle workflow:', error)
    }
  }

  const handleDeleteWorkflow = async (id: string) => {
    if (!confirm(t('automation:deleteConfirm'))) return
    try {
      await api.deleteWorkflow(id)
      await fetchWorkflows()
      onRefresh?.()
    } catch (error) {
      console.error('Failed to delete workflow:', error)
    }
  }

  const handleExecuteWorkflow = async (id: string) => {
    setExecutingId(id)
    try {
      const result = await api.executeWorkflow(id)
      alert(`${t('automation:workflowCompleted')}: ${result.execution_id}`)
    } catch (error) {
      console.error('Failed to execute workflow:', error)
    } finally {
      setExecutingId(null)
    }
  }

  const toggleDetails = (id: string) => {
    setExpandedDetails((prev) => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }

  const handleCreateWorkflow = async () => {
    if (!newWorkflowName.trim()) return
    try {
      await api.createWorkflow({
        name: newWorkflowName,
        description: newWorkflowDescription,
        triggers: [{ type: 'manual', config: {} }],
        steps: [],
        enabled: true,
        trigger_count: 0,
        step_count: 0,
        status: 'active',
      })
      setCreateDialogOpen(false)
      setNewWorkflowName('')
      setNewWorkflowDescription('')
      await fetchWorkflows()
      onRefresh?.()
    } catch (error) {
      console.error('Failed to create workflow:', error)
    }
  }

  const handleEditWorkflow = async () => {
    if (!editWorkflow) return
    try {
      await api.updateWorkflow(editWorkflow.id, {
        name: editWorkflow.name,
        description: editWorkflow.description,
      })
      setEditWorkflow(null)
      await fetchWorkflows()
      onRefresh?.()
    } catch (error) {
      console.error('Failed to update workflow:', error)
    }
  }

  const getStepIcon = (type: WorkflowStep['type']) => {
    switch (type) {
      case 'command': return '⚡'
      case 'condition': return '❓'
      case 'delay': return '⏱️'
      case 'notification': return '🔔'
      case 'llm': return '🧠'
      default: return '📄'
    }
  }

  const formatTimestamp = (timestamp: string | number) => {
    if (typeof timestamp === 'string') {
      return new Date(timestamp).toLocaleString('zh-CN', {
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
      })
    }
    const date = new Date(timestamp * 1000)
    return date.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  return (
    <>
      {/* Header with actions */}
      <ActionBar
        title={t('automation:workflowsTitle')}
        titleIcon={<ArrowRight className="h-5 w-5" />}
        description={t('automation:workflowsDesc')}
        actions={[
          {
            label: t('automation:workflowsAdd'),
            icon: <Plus className="h-4 w-4" />,
            onClick: () => setCreateDialogOpen(true),
          },
        ]}
        onRefresh={onRefresh}
      />

      {/* Table */}
      <Card>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t('automation:workflowName')}</TableHead>
              <TableHead>{t('automation:triggers')}</TableHead>
              <TableHead align="center">{t('automation:enabled')}</TableHead>
              <TableHead align="center">{t('automation:executionCount')}</TableHead>
              <TableHead>{t('automation:updatedAt')}</TableHead>
              <TableHead align="right">{t('automation:actions')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <EmptyStateInline title={t('automation:loading')} colSpan={6} />
            ) : workflows.length === 0 ? (
              <EmptyStateInline title={`${t('automation:noWorkflows')} - ${t('automation:workflowsDesc')}`} colSpan={6} />
            ) : (
              workflows.map((workflow) => {
                const isExpanded = expandedDetails.has(workflow.id)
                const hasSteps = workflow.steps && workflow.steps.length > 0

                return (
                  <>
                    <TableRow
                      key={workflow.id}
                      className={cn(!workflow.enabled && 'opacity-60')}
                    >
                      <TableCell>
                        <div>
                          <div className="font-medium">{workflow.name}</div>
                          {workflow.description && (
                            <div className="text-xs text-muted-foreground truncate max-w-md">
                              {workflow.description}
                            </div>
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-wrap gap-1">
                          {workflow.triggers && workflow.triggers.length > 0 ? (
                            workflow.triggers.map((trigger, i) => (
                              <Badge key={i} variant="outline" className="text-xs">
                                {trigger.type === 'manual' && t('automation:manual')}
                                {trigger.type === 'event' && t('automation:event')}
                                {trigger.type === 'schedule' && t('automation:scheduleLabel')}
                                {trigger.type === 'device_state' && t('automation:deviceState')}
                              </Badge>
                            ))
                          ) : (
                            <Badge variant="secondary" className="text-xs">
                              {t('automation:manual')}
                            </Badge>
                          )}
                        </div>
                      </TableCell>
                      <TableCell align="center">
                        <Switch
                          checked={workflow.enabled}
                          onCheckedChange={() => handleToggleWorkflow(workflow)}
                        />
                      </TableCell>
                      <TableCell align="center">
                        <span className="text-sm">{workflow.execution_count || 0}</span>
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {formatTimestamp(workflow.updated_at)}
                      </TableCell>
                      <TableCell align="right">
                        <div className="flex items-center justify-end gap-1">
                          {hasSteps && (
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() => toggleDetails(workflow.id)}
                            >
                              {isExpanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                            </Button>
                          )}
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-8"
                            onClick={() => handleExecuteWorkflow(workflow.id)}
                            disabled={!workflow.enabled || executingId === workflow.id}
                          >
                            <Play className="h-3 w-3 mr-1" />
                            {executingId === workflow.id ? t('automation:executing') : t('automation:execute')}
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => setEditWorkflow(workflow)}
                          >
                            <Edit className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => handleDeleteWorkflow(workflow.id)}
                          >
                            <Trash2 className="h-4 w-4 text-destructive" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>

                    {/* Expandable details row */}
                    {isExpanded && hasSteps && (
                      <TableRow key={`${workflow.id}-details`}>
                        <TableCell colSpan={6} className="bg-muted/30">
                          <div className="space-y-3 py-2">
                            <div>
                              <div className="text-sm font-medium mb-2">{t('automation:stepsLabel')}</div>
                              <div className="flex items-center gap-1 overflow-x-auto pb-1">
                                {workflow.steps?.map((step, i) => (
                                  <div key={step.id} className="flex items-center">
                                    <div className="flex items-center gap-1 px-2 py-1 bg-background rounded-md border">
                                      <span>{getStepIcon(step.type)}</span>
                                      <span className="text-xs truncate max-w-[100px]">{step.name}</span>
                                    </div>
                                    {i < (workflow.steps?.length ?? 0) - 1 && (
                                      <ArrowRight className="h-3 w-3 text-muted-foreground shrink-0" />
                                    )}
                                  </div>
                                ))}
                              </div>
                            </div>
                          </div>
                        </TableCell>
                      </TableRow>
                    )}
                  </>
                )
              })
            )}
          </TableBody>
        </Table>
      </Card>

      {/* Create Workflow Dialog */}
      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('automation:createWorkflow')}</DialogTitle>
            <DialogDescription>
              {t('automation:workflowsDesc')}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label htmlFor="workflow-name">{t('automation:workflowName')}</Label>
              <Input
                id="workflow-name"
                value={newWorkflowName}
                onChange={(e) => setNewWorkflowName(e.target.value)}
                placeholder={t('automation:workflowNamePlaceholder')}
              />
            </div>
            <div>
              <Label htmlFor="workflow-description">{t('automation:description')}</Label>
              <Textarea
                id="workflow-description"
                value={newWorkflowDescription}
                onChange={(e) => setNewWorkflowDescription(e.target.value)}
                placeholder={t('automation:workflowDescriptionPlaceholder')}
                className="min-h-[80px]"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateDialogOpen(false)}>
              {t('automation:cancel')}
            </Button>
            <Button onClick={handleCreateWorkflow} disabled={!newWorkflowName.trim()}>
              {t('automation:createWorkflow')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Workflow Dialog */}
      <Dialog open={!!editWorkflow} onOpenChange={(open) => !open && setEditWorkflow(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('automation:edit')}</DialogTitle>
            <DialogDescription>
              {t('automation:workflowsDesc')}
            </DialogDescription>
          </DialogHeader>
          {editWorkflow && (
            <div className="space-y-4">
              <div>
                <Label htmlFor="edit-workflow-name">{t('automation:workflowName')}</Label>
                <Input
                  id="edit-workflow-name"
                  value={editWorkflow.name}
                  onChange={(e) => setEditWorkflow({ ...editWorkflow, name: e.target.value })}
                />
              </div>
              <div>
                <Label htmlFor="edit-workflow-description">{t('automation:description')}</Label>
                <Textarea
                  id="edit-workflow-description"
                  value={editWorkflow.description || ''}
                  onChange={(e) => setEditWorkflow({ ...editWorkflow, description: e.target.value })}
                  className="min-h-[80px]"
                />
              </div>
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditWorkflow(null)}>
              {t('automation:cancel')}
            </Button>
            <Button onClick={handleEditWorkflow}>
              {t('automation:saveChanges')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
