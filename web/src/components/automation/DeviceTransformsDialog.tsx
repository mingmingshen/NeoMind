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
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Database, Play, Edit, Trash2, Loader2, Sparkles } from 'lucide-react'
import { Switch } from '@/components/ui/switch'
import { api } from '@/lib/api'
import { TransformBuilder } from './TransformBuilder'
import { TransformTestDialog } from './TransformTestDialog'
import type { TransformAutomation } from '@/types'

interface DeviceTransformsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  deviceId?: string
  deviceTypeId?: string
  deviceName?: string
  onTransformCreated?: () => void
}

export function DeviceTransformsDialog({
  open,
  onOpenChange,
  deviceId,
  deviceTypeId,
  deviceName,
  onTransformCreated,
}: DeviceTransformsDialogProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [transforms, setTransforms] = useState<TransformAutomation[]>([])
  const [loading, setLoading] = useState(true)
  const [builderOpen, setBuilderOpen] = useState(false)
  const [testDialogOpen, setTestDialogOpen] = useState(false)
  const [editingTransform, setEditingTransform] = useState<TransformAutomation | null>(null)
  const [testingTransformId, setTestingTransformId] = useState<string | null>(null)
  const [devices, setDevices] = useState<Array<{ id: string; name: string; device_type?: string }>>([])

  const fetchDevices = async () => {
    try {
      const result = await api.getDevices()
      setDevices((result.devices || []).map((d: any) => ({
        id: d.id,
        name: d.name,
        device_type: d.device_type,
      })))
    } catch {
      // ignore
    }
  }

  useEffect(() => {
    fetchDevices()
  }, [])

  const fetchTransforms = async () => {
    setLoading(true)
    try {
      const result = await api.listTransforms()
      let filtered = result.transforms || []

      // Filter transforms by scope
      if (deviceId) {
        filtered = filtered.filter((tr) =>
          tr.scope.type === 'device' && tr.scope.device_id === deviceId
        )
      } else if (deviceTypeId) {
        filtered = filtered.filter((tr) =>
          tr.scope.type === 'device_type' && tr.scope.device_type === deviceTypeId
        )
      }

      setTransforms(filtered)
    } catch (error) {
      console.error('Failed to fetch transforms:', error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    if (open) {
      fetchTransforms()
    }
  }, [open, deviceId, deviceTypeId])

  const handleToggleTransform = async (transform: TransformAutomation) => {
    try {
      await api.setAutomationStatus(transform.id, !transform.enabled)
      await fetchTransforms()
    } catch {
      // ignore
    }
  }

  const handleDeleteTransform = async (id: string) => {
    if (!confirm(t('automation:confirmDeleteTransform', { defaultValue: 'Are you sure?' }))) return
    try {
      await api.deleteAutomation(id)
      await fetchTransforms()
    } catch {
      // ignore
    }
  }

  const handleSaveTransform = async (data: Partial<TransformAutomation>) => {
    try {
      const definition = {
        scope: data.scope,
        device_type_filter: data.device_type_filter,
        operations: data.operations || [],
      }

      if (editingTransform) {
        await api.updateAutomation(editingTransform.id, {
          name: data.name || '',
          description: data.description,
          definition,
        })
      } else {
        await api.createAutomation({
          name: data.name || '',
          description: data.description,
          type: 'transform',
          enabled: data.enabled ?? true,
          definition,
        })
      }
      await fetchTransforms()
      setBuilderOpen(false)
      setEditingTransform(null)
      onTransformCreated?.()
    } catch (error) {
      console.error('Failed to save transform:', error)
    }
  }

  const handleCreateNew = () => {
    // Pre-fill scope based on dialog context
    const preFilledScope = deviceId
      ? { type: 'device' as const, device_id: deviceId }
      : deviceTypeId
        ? { type: 'device_type' as const, device_type: deviceTypeId }
        : { type: 'global' as const }

    setEditingTransform({
      id: '',
      name: '',
      description: '',
      enabled: true,
      type: 'transform',
      scope: preFilledScope,
      complexity: 1,
      execution_count: 0,
      created_at: 0,
      updated_at: 0,
    } as TransformAutomation)
    setBuilderOpen(true)
  }

  const getScopeBadgeVariant = (scope: TransformAutomation['scope']) => {
    switch (scope.type) {
      case 'global':
        return 'default'
      case 'device_type':
        return 'secondary'
      case 'device':
        return 'outline'
      case 'user':
        return 'destructive' as const
      default:
        return 'default'
    }
  }

  const getScopeLabel = (scope: TransformAutomation['scope']) => {
    switch (scope.type) {
      case 'global':
        return t('automation:scopeGlobal', { defaultValue: 'Global' })
      case 'device_type':
        return t('automation:scopeDeviceType', { device_type: scope.device_type })
      case 'device':
        return t('automation:scopeDevice', { device_id: scope.device_id })
      case 'user':
        return t('automation:scopeUser', { user_id: scope.user_id })
    }
  }

  const title = deviceId
    ? t('automation:deviceTransforms', { defaultValue: 'Transforms for Device' })
    : deviceTypeId
      ? t('automation:deviceTypeTransforms', { defaultValue: 'Transforms for Device Type' })
      : t('automation:allTransforms', { defaultValue: 'All Transforms' })

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Database className="h-5 w-5 text-purple-500" />
              {title}
            </DialogTitle>
            <DialogDescription>
              {deviceName
                ? t('automation:transformsForDesc', {
                    defaultValue: 'Data transforms for {{name}}',
                    name: deviceName,
                  })
                : t('automation:manageTransformsDesc', {
                    defaultValue: 'Manage data transforms for this device.',
                  })}
            </DialogDescription>
          </DialogHeader>

          <div className="flex-1 overflow-y-auto space-y-4 py-4">
            {/* Create Button */}
            <div className="flex justify-between items-center">
              <div className="text-sm text-muted-foreground">
                {transforms.length} {t('automation:transforms', { defaultValue: 'transforms' })}
              </div>
              <Button onClick={handleCreateNew} size="sm" className="bg-purple-600 hover:bg-purple-700">
                <Sparkles className="mr-2 h-4 w-4" />
                {t('automation:createTransform', { defaultValue: 'Create Transform' })}
              </Button>
            </div>

            {/* Transforms Table */}
            {loading ? (
              <div className="flex justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : transforms.length === 0 ? (
              <Card className="p-8 text-center">
                <Database className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
                <p className="text-muted-foreground mb-4">
                  {t('automation:noTransformsForDevice', {
                    defaultValue: 'No transforms configured for this device yet.',
                  })}
                </p>
                <Button onClick={handleCreateNew} variant="outline">
                  <Sparkles className="mr-2 h-4 w-4" />
                  {t('automation:createFirstTransform', { defaultValue: 'Create First Transform' })}
                </Button>
              </Card>
            ) : (
              <Card>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t('automation:name', { defaultValue: 'Name' })}</TableHead>
                      <TableHead>{t('automation:scope', { defaultValue: 'Scope' })}</TableHead>
                      <TableHead>{t('automation:operations', { defaultValue: 'Operations' })}</TableHead>
                      <TableHead>{t('common:status', { defaultValue: 'Status' })}</TableHead>
                      <TableHead className="text-right">{t('common:actions', { defaultValue: 'Actions' })}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {transforms.map((transform) => (
                      <TableRow key={transform.id}>
                        <TableCell>
                          <div className="font-medium">{transform.name}</div>
                          <div className="text-sm text-muted-foreground">{transform.description}</div>
                        </TableCell>
                        <TableCell>
                          <Badge variant={getScopeBadgeVariant(transform.scope)}>
                            {getScopeLabel(transform.scope)}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          {transform.js_code ? (
                            <Badge variant="outline" className="text-xs bg-purple-100 dark:bg-purple-900">
                              {t('automation:jsCode', { defaultValue: 'JavaScript' })}
                            </Badge>
                          ) : (
                            <Badge variant="outline" className="text-xs text-muted-foreground">
                              {t('automation:noCode', { defaultValue: 'No code' })}
                            </Badge>
                          )}
                        </TableCell>
                        <TableCell>
                          <Switch
                            checked={transform.enabled}
                            onCheckedChange={() => handleToggleTransform(transform)}
                          />
                        </TableCell>
                        <TableCell className="text-right">
                          <div className="flex justify-end gap-1">
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() => {
                                setTestingTransformId(transform.id)
                                setTestDialogOpen(true)
                              }}
                              title={t('automation:testTransform', { defaultValue: 'Test' })}
                            >
                              <Play className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() => {
                                setEditingTransform(transform)
                                setBuilderOpen(true)
                              }}
                              title={t('common:edit', { defaultValue: 'Edit' })}
                            >
                              <Edit className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() => handleDeleteTransform(transform.id)}
                              title={t('common:delete', { defaultValue: 'Delete' })}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </Card>
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              {t('common:close', { defaultValue: 'Close' })}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Transform Builder Dialog */}
      {builderOpen && (
        <TransformBuilder
          open={builderOpen}
          onOpenChange={setBuilderOpen}
          transform={editingTransform}
          devices={devices}
          onSave={handleSaveTransform}
        />
      )}

      {/* Transform Test Dialog */}
      {testDialogOpen && testingTransformId && (
        <TransformTestDialog
          open={testDialogOpen}
          onOpenChange={setTestDialogOpen}
          transformId={testingTransformId}
          devices={devices}
        />
      )}
    </>
  )
}
