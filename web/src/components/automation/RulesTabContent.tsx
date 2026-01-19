import React, { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Zap, Plus, FileDown, FileUp, Edit, Trash2, ChevronDown, ChevronUp, Clock } from 'lucide-react'
import { api } from '@/lib/api'
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
import { ActionBar, EmptyState } from '@/components/shared'
import { SimpleRuleBuilder } from './SimpleRuleBuilder'
import { AutomationCreator } from './AutomationCreator'
import type { Rule, DeviceType } from '@/types'

interface RulesTabContentProps {
  onRefresh?: () => void
}

export function RulesTabContent({ onRefresh }: RulesTabContentProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [rules, setRules] = useState<Rule[]>([])
  const [loading, setLoading] = useState(true)
  const [creatorOpen, setCreatorOpen] = useState(false)
  const [builderOpen, setBuilderOpen] = useState(false)
  const [editingRule, setEditingRule] = useState<Rule | null>(null)
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set())
  const [resources, setResources] = useState<{
    devices: Array<{ id: string; name: string; type: string; device_type?: string }>
    deviceTypes: DeviceType[]
    alertChannels: Array<{ id: string; name: string }>
  }>({ devices: [], deviceTypes: [], alertChannels: [] })

  const fetchRules = async () => {
    setLoading(true)
    try {
      const result = await api.listRules()
      setRules(result.rules || [])
    } catch {
      // ignore
    } finally {
      setLoading(false)
    }
  }

  const fetchResources = async () => {
    try {
      const [devicesResult, channelsResult, deviceTypesResult] = await Promise.all([
        api.getDevices().catch(() => ({ devices: [] })),
        api.listAlertChannels().catch(() => ({ channels: [] })),
        api.getDeviceTypes().catch(() => ({ device_types: [] })),
      ])
      setResources({
        devices: (devicesResult.devices || []).map((d: any) => ({
          id: d.id,
          name: d.name,
          type: d.type || 'unknown',
          device_type: d.device_type,
        })),
        deviceTypes: deviceTypesResult.device_types || [],
        alertChannels: (channelsResult.channels || []).map((c: any) => ({ id: c.id, name: c.name })),
      })
    } catch {
      // ignore
    }
  }

  useEffect(() => {
    fetchRules()
    fetchResources()
  }, [])

  const handleToggleRule = async (rule: Rule) => {
    try {
      if (rule.enabled) {
        await api.disableRule(rule.id)
      } else {
        await api.enableRule(rule.id)
      }
      await fetchRules()
      onRefresh?.()
    } catch {
      // ignore
    }
  }

  const handleDeleteRule = async (id: string) => {
    if (!confirm('确定要删除这个规则吗？')) return
    try {
      await api.deleteRule(id)
      await fetchRules()
      onRefresh?.()
    } catch {
      // ignore
    }
  }

  const handleCreateFromCreator = async (automation: {
    name: string
    description: string
    type: 'rule' | 'workflow'
    config: any
  }) => {
    try {
      const dsl = parseDescriptionToDSL(automation.description)
      await api.createRule({
        name: automation.name,
        dsl,
        enabled: true,
        trigger_count: 0,
      })
      await fetchRules()
      onRefresh?.()
    } catch {
      // ignore
    }
  }

  const handleSaveRule = async (data: Partial<Rule>) => {
    try {
      if (editingRule) {
        await api.updateRule(editingRule.id, {
          name: data.name || '',
          dsl: data.dsl || '',
        })
      } else {
        await api.createRule({
          name: data.name || '',
          dsl: data.dsl || '',
          enabled: data.enabled ?? true,
          trigger_count: 0,
        })
      }
      await fetchRules()
      setBuilderOpen(false)
      setEditingRule(null)
      onRefresh?.()
    } catch {
      // ignore
    }
  }

  const handleExportRules = async () => {
    try {
      const result = await api.exportRules()
      const dataStr = JSON.stringify(result, null, 2)
      const dataBlob = new Blob([dataStr], { type: 'application/json' })
      const url = URL.createObjectURL(dataBlob)
      const link = document.createElement('a')
      link.href = url
      link.download = `neotalk-rules-${new Date().toISOString().split('T')[0]}.json`
      link.click()
      URL.revokeObjectURL(url)
    } catch {
      // ignore
    }
  }

  const handleImportRules = async () => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = 'application/json'
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0]
      if (!file) return
      try {
        const text = await file.text()
        const data = JSON.parse(text)
        const result = await api.importRules(data.rules || data)
        alert(`导入: ${result.imported}, 跳过: ${result.skipped}`)
        await fetchRules()
        onRefresh?.()
      } catch {
        alert('导入失败')
      }
    }
    input.click()
  }

  const parseDescriptionToDSL = (description: string): string => {
    let dsl = ''
    const conditionMatch = description.match(/(?:当|如果)?(.{0,20})(?:温度|湿度|亮度)(?:超过|大于|>|低于|<)(\\d+)/)
    if (conditionMatch) {
      const [, value] = conditionMatch
      const metric = description.includes('温度') ? 'temperature' : description.includes('湿度') ? 'humidity' : 'brightness'
      const operator = description.includes('超过') || description.includes('大于') || description.includes('>') ? '>' : '<'
      dsl = `WHEN device.sensor_01.${metric} ${operator} ${value}\n`
    }
    if (description.includes('打开') || description.includes('开启')) {
      const device = description.includes('空调') ? 'ac_01' : description.includes('灯') ? 'light_01' : 'device_01'
      dsl += `THEN device.${device}.turn_on()`
    } else if (description.includes('关闭')) {
      const device = description.includes('空调') ? 'ac_01' : description.includes('灯') ? 'light_01' : 'device_01'
      dsl += `THEN device.${device}.turn_off()`
    } else {
      dsl += 'THEN notify("触发自动化")'
    }
    return dsl || `WHEN device.sensor_01.temperature > 30\nTHEN device.ac_01.turn_on()`
  }

  const formatTimestamp = (timestamp: string | number | undefined) => {
    if (!timestamp) return '未执行'
    if (typeof timestamp === 'string') {
      return new Date(timestamp).toLocaleString('zh-CN', {
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
      })
    }
    const date = new Date(timestamp * 1000)
    const now = new Date()
    const diff = now.getTime() - date.getTime()

    if (diff < 60000) return '刚刚'
    if (diff < 3600000) return `${Math.floor(diff / 60000)}分钟前`
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}小时前`

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

  return (
    <>
      {/* Header with actions */}
      <ActionBar
        title={t('automation:rules')}
        titleIcon={<Zap className="h-5 w-5" />}
        description={t('automation:rulesDescription')}
        actions={[
          { label: t('automation:export'), icon: <FileDown className="h-4 w-4" />, onClick: handleExportRules, variant: 'outline' },
          { label: t('automation:import'), icon: <FileUp className="h-4 w-4" />, onClick: handleImportRules, variant: 'outline' },
          { label: t('automation:createVisual'), icon: <Plus className="h-4 w-4" />, onClick: () => { setEditingRule(null); setBuilderOpen(true) }, variant: 'outline' },
          { label: t('automation:createAI'), icon: <Plus className="h-4 w-4" />, onClick: () => setCreatorOpen(true) },
        ]}
      />

      {/* Table */}
      <Card>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead style={{ width: '25%' }}>规则名称</TableHead>
              <TableHead style={{ width: '30%' }}>触发条件</TableHead>
              <TableHead style={{ width: '10%' }}>状态</TableHead>
              <TableHead style={{ width: '10%' }}>触发次数</TableHead>
              <TableHead style={{ width: '15%' }}>最后执行</TableHead>
              <TableHead style={{ width: '10%' }} className="text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center text-muted-foreground py-8">
                  加载中...
                </TableCell>
              </TableRow>
            ) : rules.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center py-12">
                  <EmptyState
                    icon={<Zap className="h-12 w-12 text-muted-foreground" />}
                    title="还没有规则自动化"
                    description="创建您的第一个自动化规则"
                  />
                </TableCell>
              </TableRow>
            ) : (
              rules.map((rule) => {
                const isExpanded = expandedRows.has(rule.id)
                return (
                  <React.Fragment key={rule.id}>
                    <TableRow className={rule.enabled ? '' : 'opacity-60'}>
                      <TableCell>
                        <div className="font-medium">{rule.name}</div>
                        {rule.description && (
                          <div className="text-xs text-muted-foreground truncate">{rule.description}</div>
                        )}
                      </TableCell>
                      <TableCell>
                        <code className="text-xs bg-muted px-2 py-1 rounded">
                          {rule.dsl?.split('\n')[0]?.replace('WHEN ', '') || '-'}
                        </code>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Switch
                            checked={rule.enabled}
                            onCheckedChange={() => handleToggleRule(rule)}
                          />
                          <span className="text-xs">
                            {rule.enabled ? '启用' : '禁用'}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1 text-muted-foreground">
                          <Zap className="h-3.5 w-3.5" />
                          {rule.trigger_count || 0}
                        </div>
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {rule.last_triggered ? (
                          <div className="flex items-center gap-1">
                            <Clock className="h-3.5 w-3.5" />
                            {formatTimestamp(rule.last_triggered)}
                          </div>
                        ) : (
                          <span className="text-xs">未执行</span>
                        )}
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex items-center justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => toggleRow(rule.id)}
                          >
                            {isExpanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => {
                              setEditingRule(rule)
                              setBuilderOpen(true)
                            }}
                          >
                            <Edit className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-destructive"
                            onClick={() => handleDeleteRule(rule.id)}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>

                    {/* Expandable details */}
                    {isExpanded && (
                      <TableRow>
                        <TableCell colSpan={6} className="bg-muted/30">
                          <div className="p-3 space-y-3">
                            <div>
                              <div className="text-xs text-muted-foreground mb-2">完整规则 (DSL)</div>
                              <pre className="text-xs bg-background p-2 rounded overflow-x-auto">
                                {rule.dsl || '-'}
                              </pre>
                            </div>
                            {rule.actions && rule.actions.length > 0 && (
                              <div>
                                <div className="text-xs text-muted-foreground mb-2">执行动作</div>
                                <div className="flex flex-wrap gap-2">
                                  {rule.actions.map((action: any, i: number) => (
                                    <Badge key={i} variant="outline" className="text-xs">
                                      {action.type === 'Execute' && (
                                        <>⚡ {action.device_id}.{action.command}()</>
                                      )}
                                      {action.type === 'Notify' && (
                                        <>🔔 {action.message}</>
                                      )}
                                      {action.type === 'Log' && (
                                        <>📝 {action.message}</>
                                      )}
                                    </Badge>
                                  ))}
                                </div>
                              </div>
                            )}
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

      {/* Automation Creator Dialog */}
      <AutomationCreator
        open={creatorOpen}
        onOpenChange={setCreatorOpen}
        onCreate={handleCreateFromCreator}
      />

      {/* Simple Rule Builder Dialog */}
      <SimpleRuleBuilder
        open={builderOpen}
        onOpenChange={(open) => {
          setBuilderOpen(open)
          if (!open) setEditingRule(null)
        }}
        rule={editingRule || undefined}
        onSave={handleSaveRule}
        resources={resources}
      />
    </>
  )
}
