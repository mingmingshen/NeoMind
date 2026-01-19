import React, { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { ArrowRight, Play, Plus, Edit, Trash2, Clock, ChevronDown, ChevronUp } from 'lucide-react'
import { api } from '@/lib/api'
import type { Workflow } from '@/types'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
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
import { WorkflowVisualEditor } from './WorkflowVisualEditor'

interface WorkflowsTabProps {
  onRefresh?: () => void
}

const STEP_ICONS: Record<string, React.ReactNode> = {
  send_command: <Play className="h-4 w-4" />,
  condition: <span className="text-sm">❓</span>,
  delay: <Clock className="h-4 w-4" />,
  send_alert: <span className="text-sm">🔔</span>,
  log: <span className="text-sm">📝</span>,
  http_request: <span className="text-sm">🌐</span>,
  device_query: <span className="text-sm">📊</span>,
  wait_for_device_state: <Clock className="h-4 w-4" />,
  parallel: <span className="text-sm">⚡⚡</span>,
}

export function WorkflowsTab({ onRefresh }: WorkflowsTabProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [workflows, setWorkflows] = useState<Workflow[]>([])
  const [loading, setLoading] = useState(true)
  const [builderOpen, setBuilderOpen] = useState(false)
  const [editingWorkflow, setEditingWorkflow] = useState<Workflow | null>(null)
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set())
  const [resources, setResources] = useState<{
    devices: Array<{ id: string; name: string; type: string }>
    metrics: string[]
    alertChannels: Array<{ id: string; name: string }>
  }>({ devices: [], metrics: [], alertChannels: [] })

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

  const fetchResources = async () => {
    try {
      const [devicesResult, channelsResult] = await Promise.all([
        api.getDevices().catch(() => ({ devices: [] })),
        api.listAlertChannels().catch(() => ({ channels: [] })),
      ])
      setResources({
        devices: (devicesResult.devices || []).map((d: any) => ({ id: d.id, name: d.name, type: d.type || 'unknown' })),
        metrics: [],
        alertChannels: (channelsResult.channels || []).map((c: any) => ({ id: c.id, name: c.name })),
      })
    } catch (error) {
      console.error('Failed to fetch resources:', error)
    }
  }

  useEffect(() => {
    fetchWorkflows()
    fetchResources()
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
    if (!confirm('确定要删除这个工作流吗？')) return
    try {
      await api.deleteWorkflow(id)
      await fetchWorkflows()
      onRefresh?.()
    } catch (error) {
      console.error('Failed to delete workflow:', error)
    }
  }

  const handleExecuteWorkflow = async (id: string) => {
    try {
      const result = await api.executeWorkflow(id)
      alert(`工作流已执行: ${result.execution_id}`)
    } catch (error) {
      console.error('Failed to execute workflow:', error)
    }
  }

  const handleSaveWorkflow = async (data: Partial<Workflow>) => {
    try {
      if (editingWorkflow) {
        await api.updateWorkflow(editingWorkflow.id, data)
      } else {
        if (!data.name) {
          throw new Error('工作流名称不能为空')
        }
        await api.createWorkflow(data as Omit<Workflow, 'id' | 'created_at' | 'updated_at'>)
      }
      await fetchWorkflows()
      setBuilderOpen(false)
      setEditingWorkflow(null)
      onRefresh?.()
    } catch (error) {
      console.error('Failed to save workflow:', error)
      throw error
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

  const toggleRow = (id: string) => {
    setExpandedRows((prev) => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }

  const getTriggerLabel = (trigger: any) => {
    switch (trigger.type) {
      case 'manual': return '手动执行'
      case 'cron': return '定时执行'
      case 'event': return '事件触发'
      case 'device': return '设备状态变化'
      default: return trigger.type || '手动'
    }
  }

  return (
    <>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-xl font-semibold">工作流自动化</h2>
          <p className="text-sm text-muted-foreground">多步骤复杂自动化流程</p>
        </div>
        <Button onClick={() => {
          setEditingWorkflow(null)
          setBuilderOpen(true)
        }}>
          <Plus className="h-4 w-4 mr-1" />
          新建
        </Button>
      </div>

      {/* Table */}
      <Card>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead style={{ width: '25%' }}>工作流名称</TableHead>
              <TableHead style={{ width: '20%' }}>触发方式</TableHead>
              <TableHead style={{ width: '10%' }}>状态</TableHead>
              <TableHead style={{ width: '10%' }}>执行次数</TableHead>
              <TableHead style={{ width: '15%' }}>更新时间</TableHead>
              <TableHead style={{ width: '20%' }} className="text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center text-muted-foreground py-8">
                  加载中...
                </TableCell>
              </TableRow>
            ) : workflows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center py-12">
                  <div className="flex flex-col items-center text-muted-foreground">
                    <Play className="h-12 w-12 mb-3 opacity-50" />
                    <p className="mb-4">还没有工作流自动化</p>
                    <Button variant="outline" onClick={() => {
                      setEditingWorkflow(null)
                      setBuilderOpen(true)
                    }}>
                      <Plus className="h-4 w-4 mr-1" />
                      创建第一个工作流
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ) : (
              workflows.map((workflow) => {
                const isExpanded = expandedRows.has(workflow.id)
                const hasSteps = workflow.steps && workflow.steps.length > 0
                return (
                  <React.Fragment key={workflow.id}>
                    <TableRow className={workflow.enabled ? '' : 'opacity-60'}>
                      <TableCell>
                        <div className="font-medium">{workflow.name}</div>
                        {workflow.description && (
                          <div className="text-xs text-muted-foreground truncate">{workflow.description}</div>
                        )}
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-wrap gap-1">
                          {workflow.triggers && workflow.triggers.length > 0 ? (
                            workflow.triggers.map((trigger, i) => (
                              <Badge key={i} variant="outline" className="text-xs">
                                {getTriggerLabel(trigger)}
                              </Badge>
                            ))
                          ) : (
                            <Badge variant="secondary" className="text-xs">手动执行</Badge>
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Switch
                            checked={workflow.enabled}
                            onCheckedChange={() => handleToggleWorkflow(workflow)}
                          />
                          <span className="text-xs">
                            {workflow.enabled ? '启用' : '禁用'}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="text-muted-foreground">{workflow.execution_count || 0}</div>
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {formatTimestamp(workflow.updated_at)}
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex items-center justify-end gap-1">
                          {hasSteps && (
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() => toggleRow(workflow.id)}
                            >
                              {isExpanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                            </Button>
                          )}
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-8"
                            onClick={() => handleExecuteWorkflow(workflow.id)}
                            disabled={!workflow.enabled}
                          >
                            <Play className="h-3 w-3 mr-1" />
                            执行
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => {
                              setEditingWorkflow(workflow)
                              setBuilderOpen(true)
                            }}
                          >
                            <Edit className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-destructive"
                            onClick={() => handleDeleteWorkflow(workflow.id)}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>

                    {/* Expandable details */}
                    {isExpanded && hasSteps && (
                      <TableRow>
                        <TableCell colSpan={6} className="bg-muted/30">
                          <div className="p-3">
                            <div className="text-xs text-muted-foreground mb-2">执行步骤</div>
                            <div className="flex items-center gap-1 overflow-x-auto pb-1">
                              {workflow.steps?.map((step, i) => (
                                <React.Fragment key={step.id || i}>
                                  <div className="flex items-center gap-1 px-2 py-1 bg-background rounded-md border shrink-0">
                                    <span>{STEP_ICONS[step.type] || <span className="text-xs">📄</span>}</span>
                                    <span className="text-xs truncate max-w-[100px]">
                                      {(step as any).name || t(`automation:steps.${step.type}`)}
                                    </span>
                                  </div>
                                  {i < (workflow.steps?.length ?? 0) - 1 && (
                                    <ArrowRight className="h-3 w-3 text-muted-foreground shrink-0" />
                                  )}
                                </React.Fragment>
                              ))}
                            </div>
                          </div>
                        </TableCell>
                      </TableRow>
                    )}
                  </React.Fragment>
                )
              })
            )}
          </TableBody>
        </Table>
      </Card>

      {/* Workflow Builder Dialog - Fullscreen */}
      <Dialog open={builderOpen} onOpenChange={setBuilderOpen}>
        <DialogContent className="max-w-[95vw] h-[95vh] max-h-[95vh] p-0 gap-0 flex flex-col">
          <DialogHeader className="px-6 py-4 border-b">
            <DialogTitle className="flex items-center gap-2">
              <Play className="h-5 w-5" />
              {editingWorkflow ? '编辑工作流' : '创建工作流'}
            </DialogTitle>
          </DialogHeader>
          <div className="flex-1 overflow-y-auto">
            <WorkflowVisualEditor
              workflow={editingWorkflow || undefined}
              onSave={handleSaveWorkflow}
              onCancel={() => {
                setBuilderOpen(false)
                setEditingWorkflow(null)
              }}
              resources={resources}
            />
          </div>
        </DialogContent>
      </Dialog>
    </>
  )
}
