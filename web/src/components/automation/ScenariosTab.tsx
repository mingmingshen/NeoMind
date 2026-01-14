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
import { Plus, Trash2, Edit, Play, Home } from 'lucide-react'
import { ActionBar, EmptyStateInline } from '@/components/shared'
import { api } from '@/lib/api'
import type { Scenario, ScenarioAction } from '@/types'
import { cn } from '@/lib/utils'

interface ScenariosTabProps {
  onRefresh?: () => void
}

const SCENARIO_ICONS: Record<string, string> = {
  '回家模式': '🏠',
  '离家模式': '🚪',
  '睡眠模式': '💤',
  '起床模式': '☀️',
  '观影模式': '🎬',
  '阅读模式': '📖',
  '会客模式': '👋',
  '节能模式': '⚡',
}

export function ScenariosTab({ onRefresh }: ScenariosTabProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [scenarios, setScenarios] = useState<Scenario[]>([])
  const [loading, setLoading] = useState(true)
  const [createDialogOpen, setCreateDialogOpen] = useState(false)
  const [editScenario, setEditScenario] = useState<Scenario | null>(null)
  const [newScenarioName, setNewScenarioName] = useState('')
  const [newScenarioIcon, setNewScenarioIcon] = useState('🏠')
  const [newScenarioActions, setNewScenarioActions] = useState('')
  const [executingId, setExecutingId] = useState<string | null>(null)

  const fetchScenarios = async () => {
    setLoading(true)
    try {
      const result = await api.listScenarios()
      setScenarios(result.scenarios || [])
    } catch (error) {
      console.error('Failed to fetch scenarios:', error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchScenarios()
  }, [])

  const handleToggleScenario = async (scenario: Scenario) => {
    try {
      if (scenario.active) {
        await api.deactivateScenario(scenario.id)
      } else {
        await api.activateScenario(scenario.id)
      }
      await fetchScenarios()
      onRefresh?.()
    } catch (error) {
      console.error('Failed to toggle scenario:', error)
    }
  }

  const handleDeleteScenario = async (id: string) => {
    if (!confirm(t('automation:deleteConfirm'))) return
    try {
      await api.deleteScenario(id)
      await fetchScenarios()
      onRefresh?.()
    } catch (error) {
      console.error('Failed to delete scenario:', error)
    }
  }

  const handleActivateScenario = async (id: string) => {
    setExecutingId(id)
    try {
      await api.activateScenario(id)
      await fetchScenarios()
    } catch (error) {
      console.error('Failed to activate scenario:', error)
    } finally {
      setExecutingId(null)
    }
  }

  const handleCreateScenario = async () => {
    if (!newScenarioName.trim()) return
    try {
      // Parse actions from simple format
      const actions: ScenarioAction[] = newScenarioActions
        .split('\n')
        .filter(line => line.trim())
        .map((line, i) => ({
          id: `action-${Date.now()}-${i}`,
          type: 'device_command' as const,
          device_id: '',
          command: line,
        }))

      await api.createScenario({
        name: newScenarioName,
        icon: newScenarioIcon,
        actions,
        enabled: true,
        active: false,
      })
      setCreateDialogOpen(false)
      setNewScenarioName('')
      setNewScenarioIcon('🏠')
      setNewScenarioActions('')
      await fetchScenarios()
      onRefresh?.()
    } catch (error) {
      console.error('Failed to create scenario:', error)
    }
  }

  const handleEditScenario = async () => {
    if (!editScenario) return
    try {
      await api.updateScenario(editScenario.id, {
        name: editScenario.name,
        icon: editScenario.icon,
        actions: editScenario.actions,
      })
      setEditScenario(null)
      await fetchScenarios()
    } catch (error) {
      console.error('Failed to update scenario:', error)
    }
  }

  const getActionIcon = (action: ScenarioAction) => {
    switch (action.type) {
      case 'device_command': return '⚡'
      case 'scene': return '🎬'
      case 'delay': return '⏱️'
      case 'notification': return '🔔'
      default: return '📄'
    }
  }

  const getActionDescription = (action: ScenarioAction) => {
    switch (action.type) {
      case 'device_command':
        return action.device_id
          ? `${action.device_id}: ${action.command}`
          : action.command || t('automation:execute')
      case 'scene':
        return `${t('automation:scenario')}: ${action.device_id}`
      case 'delay':
        return `${t('automation:delay')} ${action.delay_ms ? Math.round(action.delay_ms / 1000) : 0}s`
      case 'notification':
        return t('automation:notification')
      default:
        return t('automation:actions')
    }
  }

  return (
    <>
      {/* Header with actions */}
      <ActionBar
        title={t('automation:scenariosTitle')}
        titleIcon={<Home className="h-5 w-5" />}
        description={t('automation:scenariosDesc')}
        actions={[
          {
            label: t('automation:scenariosAdd'),
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
              <TableHead>{t('automation:scenarioName')}</TableHead>
              <TableHead>{t('automation:actions')}</TableHead>
              <TableHead align="center">{t('automation:enabled')}</TableHead>
              <TableHead align="center">{t('automation:status')}</TableHead>
              <TableHead align="right">{t('automation:actions')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <EmptyStateInline title={t('automation:loading')} colSpan={5} />
            ) : scenarios.length === 0 ? (
              <EmptyStateInline title={`${t('automation:noScenarios')} - ${t('automation:scenariosEmptyHint')}`} colSpan={5} />
            ) : (
              scenarios.map((scenario) => (
                <TableRow
                  key={scenario.id}
                  className={cn(
                    scenario.active && 'bg-green-500/5',
                    !scenario.enabled && 'opacity-50'
                  )}
                >
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <span className="text-xl">{scenario.icon || SCENARIO_ICONS[scenario.name] || '🎬'}</span>
                      <span className="font-medium">{scenario.name}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="max-w-md">
                      <div className="flex flex-wrap gap-1">
                        {scenario.actions.slice(0, 3).map((action, i) => (
                          <Badge key={i} variant="outline" className="text-xs">
                            {getActionIcon(action)} {getActionDescription(action)}
                          </Badge>
                        ))}
                        {scenario.actions.length > 3 && (
                          <Badge variant="secondary" className="text-xs">
                            +{scenario.actions.length - 3}
                          </Badge>
                        )}
                      </div>
                    </div>
                  </TableCell>
                  <TableCell align="center">
                    <Switch
                      checked={scenario.enabled}
                      onCheckedChange={() => handleToggleScenario(scenario)}
                    />
                  </TableCell>
                  <TableCell align="center">
                    {scenario.active ? (
                      <Badge className="bg-green-500">{t('automation:scenariosActive')}</Badge>
                    ) : (
                      <Badge variant="outline">{t('automation:ready')}</Badge>
                    )}
                  </TableCell>
                  <TableCell align="right">
                    <div className="flex items-center justify-end gap-1">
                      <Button
                        variant={scenario.active ? 'default' : 'outline'}
                        size="sm"
                        className="h-8"
                        onClick={() => handleActivateScenario(scenario.id)}
                        disabled={!scenario.enabled || scenario.active || executingId === scenario.id}
                      >
                        <Play className="h-3 w-3 mr-1" />
                        {executingId === scenario.id ? t('automation:executing') : t('automation:scenariosExecute')}
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        onClick={() => setEditScenario(scenario)}
                      >
                        <Edit className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        onClick={() => handleDeleteScenario(scenario.id)}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Card>

      {/* Create Scenario Dialog */}
      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('automation:createScenario')}</DialogTitle>
            <DialogDescription>
              {t('automation:scenariosDesc')}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label htmlFor="scenario-name">{t('automation:scenarioName')}</Label>
              <Input
                id="scenario-name"
                value={newScenarioName}
                onChange={(e) => setNewScenarioName(e.target.value)}
                placeholder={t('automation:scenarioNamePlaceholder')}
              />
            </div>
            <div>
              <Label>{t('automation:scenarioIcon')}</Label>
              <div className="flex gap-2 mt-2">
                {['🏠', '🚪', '💤', '☀️', '🎬', '📖', '⚡', '🔔'].map((icon) => (
                  <button
                    key={icon}
                    type="button"
                    onClick={() => setNewScenarioIcon(icon)}
                    className={cn(
                      'w-10 h-10 rounded-lg border-2 flex items-center justify-center text-xl transition-all',
                      newScenarioIcon === icon
                        ? 'border-primary bg-primary/10'
                        : 'border-transparent hover:border-muted-foreground/30'
                    )}
                  >
                    {icon}
                  </button>
                ))}
              </div>
            </div>
            <div>
              <Label htmlFor="scenario-actions">{t('automation:actions')}</Label>
              <Textarea
                id="scenario-actions"
                value={newScenarioActions}
                onChange={(e) => setNewScenarioActions(e.target.value)}
                placeholder={t('automation:scenarioActionsPlaceholder')}
                className="font-mono text-sm min-h-[120px]"
              />
              <p className="text-xs text-muted-foreground mt-1">
                {t('automation:scenarioActionsHint')}
              </p>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setCreateDialogOpen(false)}>
                {t('automation:cancel')}
              </Button>
              <Button onClick={handleCreateScenario} disabled={!newScenarioName}>
                {t('automation:createScenario')}
              </Button>
            </DialogFooter>
          </div>
        </DialogContent>
      </Dialog>

      {/* Edit Scenario Dialog */}
      <Dialog open={!!editScenario} onOpenChange={() => setEditScenario(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('automation:edit')}</DialogTitle>
            <DialogDescription>
              {t('automation:editActionsHint')}
            </DialogDescription>
          </DialogHeader>
          {editScenario && (
            <div className="space-y-4">
              <div>
                <Label htmlFor="edit-scenario-name">{t('automation:scenarioName')}</Label>
                <Input
                  id="edit-scenario-name"
                  value={editScenario.name}
                  onChange={(e) => setEditScenario({ ...editScenario, name: e.target.value })}
                />
              </div>
              <div>
                <Label>{t('automation:scenarioIcon')}</Label>
                <div className="flex gap-2 mt-2">
                  {['🏠', '🚪', '💤', '☀️', '🎬', '📖', '⚡', '🔔'].map((icon) => (
                    <button
                      key={icon}
                      type="button"
                      onClick={() => setEditScenario({ ...editScenario, icon })}
                      className={cn(
                        'w-10 h-10 rounded-lg border-2 flex items-center justify-center text-xl transition-all',
                        editScenario.icon === icon
                          ? 'border-primary bg-primary/10'
                          : 'border-transparent hover:border-muted-foreground/30'
                      )}
                    >
                      {icon}
                    </button>
                  ))}
                </div>
              </div>
              <div>
                <Label>{t('automation:actions')}</Label>
                <Textarea
                  value={editScenario.actions.map(a => {
                    if (a.type === 'device_command') {
                      return a.device_id ? `${a.device_id}.${a.command}` : a.command
                    }
                    return ''
                  }).join('\n')}
                  readOnly
                  className="font-mono text-sm min-h-[100px] bg-muted"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  {t('automation:editActionsHint')}
                </p>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setEditScenario(null)}>
                  {t('automation:cancel')}
                </Button>
                <Button onClick={handleEditScenario}>
                  {t('automation:saveChanges')}
                </Button>
              </DialogFooter>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </>
  )
}
