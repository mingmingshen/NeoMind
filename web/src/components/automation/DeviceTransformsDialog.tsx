import { useState, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { useErrorHandler } from '@/hooks/useErrorHandler'
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
import { Switch } from '@/components/ui/switch'
import { Database, Play, Edit, Trash2, Loader2, Sparkles, ChevronDown, ChevronUp, CheckCircle2, X } from 'lucide-react'
import { api } from '@/lib/api'
import { confirm } from '@/hooks/use-confirm'
import { useIsMobile, useSafeAreaInsets } from '@/hooks/useMobile'
import { useMobileBodyScrollLock } from '@/hooks/useBodyScrollLock'
import { TransformBuilder as TransformBuilderSplit } from './TransformBuilderSplit'
import { TransformTestDialog } from './TransformTestDialog'
import { cn } from '@/lib/utils'
import type { TransformAutomation, TransformScope } from '@/types'

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
  const { t } = useTranslation('automation')
  const { handleError } = useErrorHandler()
  const isMobile = useIsMobile()
  const insets = useSafeAreaInsets()

  const [transforms, setTransforms] = useState<TransformAutomation[]>([])
  const [loading, setLoading] = useState(true)
  const [builderOpen, setBuilderOpen] = useState(false)
  const [testDialogOpen, setTestDialogOpen] = useState(false)
  const [editingTransform, setEditingTransform] = useState<TransformAutomation | null>(null)
  const [testingTransformId, setTestingTransformId] = useState<string | null>(null)
  const [devices, setDevices] = useState<Array<{ id: string; name: string; device_type?: string }>>([])
  const [deviceTypes, setDeviceTypes] = useState<Array<{ device_type: string; name?: string }>>([])

  // Mobile collapsible sections
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['transforms']))

  // Lock body scroll on mobile
  useMobileBodyScrollLock(isMobile && open)

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

  const fetchDeviceTypes = async () => {
    try {
      const result = await api.getDeviceTypes()
      setDeviceTypes((result.device_types || []).map((dt: any) => ({
        device_type: dt.device_type,
        name: dt.name,
      })))
    } catch {
      // ignore
    }
  }

  useEffect(() => {
    fetchDevices()
    fetchDeviceTypes()
  }, [])

  const fetchTransforms = async () => {
    setLoading(true)
    try {
      const result = await api.listTransforms()
      let filtered = result.transforms || []

      if (deviceId) {
        filtered = filtered.filter((tr) =>
          typeof tr.scope === 'object' && 'device' in tr.scope && tr.scope.device === deviceId
        )
      } else if (deviceTypeId) {
        filtered = filtered.filter((tr) =>
          typeof tr.scope === 'object' && 'device_type' in tr.scope && tr.scope.device_type === deviceTypeId
        )
      }

      setTransforms(filtered)
    } catch (error) {
      handleError(error, { operation: 'Fetch transforms', showToast: false })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    if (open) {
      fetchTransforms()
      setExpandedSections(new Set(['transforms']))
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
    const confirmed = await confirm({
      title: t('common:delete'),
      description: t('automation:confirmDeleteTransform', { defaultValue: 'Are you sure?' }),
      confirmText: t('common:delete'),
      cancelText: t('common:cancel'),
      variant: "destructive"
    })
    if (!confirmed) return

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
        scope: data.scope || 'global',
        js_code: data.js_code || '',
        output_prefix: data.output_prefix || '',
        complexity: data.complexity || 2,
      }

      if (editingTransform) {
        await api.updateAutomation(editingTransform.id, {
          name: data.name || '',
          description: data.description,
          enabled: data.enabled,
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
      handleError(error, { operation: 'Save transform', showToast: true })
      throw error
    }
  }

  const handleCreateNew = () => {
    const preFilledScope: TransformScope = deviceId
      ? { device: deviceId }
      : deviceTypeId
        ? { device_type: deviceTypeId }
        : 'global'

    setEditingTransform({
      id: '',
      name: '',
      description: '',
      enabled: true,
      type: 'transform',
      scope: preFilledScope,
      output_prefix: '',
      complexity: 1,
      execution_count: 0,
      created_at: 0,
      updated_at: 0,
      last_executed: null,
    } as TransformAutomation)
    setBuilderOpen(true)
  }

  const toggleSection = useCallback((sectionKey: string) => {
    setExpandedSections(prev => {
      const newSet = new Set(prev)
      if (newSet.has(sectionKey)) {
        newSet.delete(sectionKey)
      } else {
        newSet.add(sectionKey)
      }
      return newSet
    })
  }, [])

  const handleClose = useCallback(() => {
    onOpenChange(false)
  }, [onOpenChange])

  const getScopeBadgeVariant = (scope: TransformAutomation['scope']) => {
    if (scope === 'global') return 'default'
    if (typeof scope === 'object') {
      if ('device_type' in scope) return 'secondary'
      if ('device' in scope) return 'outline'
      if ('user' in scope) return 'destructive' as const
    }
    return 'default'
  }

  const getScopeLabel = (scope: TransformAutomation['scope']) => {
    if (scope === 'global') return t('automation:scopeGlobal', { defaultValue: 'Global' })
    if (typeof scope === 'object') {
      if ('device_type' in scope) return t('automation:scopeDeviceType', { device_type: scope.device_type })
      if ('device' in scope) return t('automation:scopeDevice', { device_id: scope.device })
    }
    return String(scope)
  }

  const title = deviceId
    ? t('automation:deviceTransforms', { defaultValue: 'Transforms for Device' })
    : deviceTypeId
      ? t('automation:deviceTypeTransforms', { defaultValue: 'Transforms for Device Type' })
      : t('automation:allTransforms', { defaultValue: 'All Transforms' })

  const description = deviceName
    ? t('automation:transformsForDesc', { defaultValue: 'Data transforms for {{name}}', name: deviceName })
    : t('automation:manageTransformsDesc', { defaultValue: 'Manage data transforms for this device.' })

  // Transform card for mobile view
  const TransformCard = ({ transform }: { transform: TransformAutomation }) => (
    <Card className="p-4 space-y-3">
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <div className="font-medium truncate">{transform.name}</div>
          <div className="text-sm text-muted-foreground line-clamp-2">{transform.description}</div>
        </div>
        <Switch
          checked={transform.enabled}
          onCheckedChange={() => handleToggleTransform(transform)}
        />
      </div>

      <div className="flex flex-wrap gap-2">
        <Badge variant={getScopeBadgeVariant(transform.scope)}>
          {getScopeLabel(transform.scope)}
        </Badge>
        {transform.js_code ? (
          <Badge variant="outline" className="text-xs bg-accent-purple-light text-accent-purple">
            {t('automation:jsCode', { defaultValue: 'JavaScript' })}
          </Badge>
        ) : (
          <Badge variant="outline" className="text-xs text-muted-foreground">
            {t('automation:noCode', { defaultValue: 'No code' })}
          </Badge>
        )}
      </div>

      <div className="flex justify-end gap-2 pt-2 border-t">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setTestingTransformId(transform.id)
            setTestDialogOpen(true)
          }}
        >
          <Play className="h-4 w-4 mr-1" />
          {t('automation:testTransform', { defaultValue: 'Test' })}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setEditingTransform(transform)
            setBuilderOpen(true)
          }}
        >
          <Edit className="h-4 w-4 mr-1" />
          {t('common:edit', { defaultValue: 'Edit' })}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => handleDeleteTransform(transform.id)}
        >
          <Trash2 className="h-4 w-4 mr-1" />
          {t('common:delete', { defaultValue: 'Delete' })}
        </Button>
      </div>
    </Card>
  )

  // Mobile collapsible section
  const MobileSection = ({
    title,
    isExpanded,
    onToggle,
    children,
  }: {
    title: string
    isExpanded: boolean
    onToggle: () => void
    children: React.ReactNode
  }) => (
    <div className="overflow-hidden rounded-lg border border-border bg-card">
      <button
        onClick={onToggle}
        className="w-full flex items-center justify-between px-4 py-4 bg-muted-30 hover:bg-muted active:bg-muted-50 transition-colors touch-action-manipulation"
      >
        <div className="flex items-center gap-3">
          <Database className="h-5 w-5 text-muted-foreground" />
          <span className="font-semibold text-foreground">{title}</span>
        </div>
        <div className="h-8 w-8 rounded-full bg-background flex items-center justify-center shrink-0">
          {isExpanded ? (
            <ChevronUp className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          )}
        </div>
      </button>
      {isExpanded && (
        <div className="p-4 bg-background animate-in slide-in-from-top-2 duration-200 border-t border-border">
          {children}
        </div>
      )}
    </div>
  )

  // Mobile: Full-screen portal
  if (isMobile) {
    return createPortal(
      <>
        {open && (
          <div className="fixed inset-0 z-50 bg-background animate-in fade-in duration-200">
            <div className="flex h-full w-full flex-col">
              {/* Header */}
              <div
                className="flex items-center justify-between px-4 py-4 border-b shrink-0 bg-background"
                style={{ paddingTop: `calc(1rem + ${insets.top}px)` }}
              >
                <div className="flex items-center gap-3 min-w-0 flex-1">
                  <Database className="h-5 w-5 text-accent-purple shrink-0" />
                  <div className="min-w-0 flex-1">
                    <h1 className="text-base font-semibold truncate">{title}</h1>
                    <p className="text-xs text-muted-foreground truncate">{description}</p>
                  </div>
                </div>
                <Button variant="ghost" size="icon" onClick={handleClose} className="shrink-0">
                  <X className="h-5 w-5" />
                </Button>
              </div>

              {/* Content */}
              <div className="flex-1 overflow-y-auto overflow-x-hidden">
                <div className="p-4 space-y-4">
                  {/* Create Button */}
                  <Button onClick={handleCreateNew} className="w-full bg-accent-purple hover:bg-accent-purple/80">
                    <Sparkles className="mr-2 h-4 w-4" />
                    {t('automation:createTransform', { defaultValue: 'Create Transform' })}
                  </Button>

                  {/* Transforms List */}
                  {loading ? (
                    <div className="flex justify-center py-8">
                      <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                    </div>
                  ) : transforms.length === 0 ? (
                    <Card className="p-8 text-center">
                      <Database className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
                      <p className="text-muted-foreground mb-4">
                        {t('automation:noTransformsForDevice', { defaultValue: 'No transforms configured yet.' })}
                      </p>
                      <Button onClick={handleCreateNew} variant="outline">
                        <Sparkles className="mr-2 h-4 w-4" />
                        {t('automation:createFirstTransform', { defaultValue: 'Create First Transform' })}
                      </Button>
                    </Card>
                  ) : (
                    <div className="space-y-3">
                      <div className="text-sm text-muted-foreground">
                        {transforms.length} {t('automation:transforms', { defaultValue: 'transforms' })}
                      </div>
                      {transforms.map((transform) => (
                        <TransformCard key={transform.id} transform={transform} />
                      ))}
                    </div>
                  )}
                </div>
              </div>

              {/* Footer */}
              <div
                className="flex items-center justify-end gap-3 px-4 py-4 border-t shrink-0 bg-background"
                style={{ paddingBottom: `calc(1rem + ${insets.bottom}px)` }}
              >
                <Button variant="outline" onClick={handleClose} className="min-w-[80px]">
                  {t('common:close', { defaultValue: 'Close' })}
                </Button>
              </div>
            </div>
          </div>
        )}

        {/* Transform Builder Dialog */}
        <TransformBuilderSplit
          open={builderOpen}
          onOpenChange={setBuilderOpen}
          transform={editingTransform}
          devices={devices}
          deviceTypes={deviceTypes}
          onSave={handleSaveTransform}
        />

        {/* Transform Test Dialog */}
        {testDialogOpen && testingTransformId && (
          <TransformTestDialog
            open={testDialogOpen}
            onOpenChange={setTestDialogOpen}
            transformId={testingTransformId}
            devices={devices}
          />
        )}
      </>,
      document.body
    )
  }

  // Desktop: Traditional dialog
  return (
    <>
      {/* Backdrop */}
      {open && (
        <div
          className="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm animate-in fade-in duration-200"
          onClick={handleClose}
        />
      )}

      {/* Dialog */}
      {open && (
        <div
          className={cn(
            'fixed left-1/2 top-1/2 z-50',
            'grid w-full gap-0',
            'bg-background shadow-lg',
            'duration-200',
            'animate-in fade-in zoom-in-95 slide-in-from-left-1/2 slide-in-from-top-[48%]',
            'rounded-lg sm:rounded-xl',
            'max-h-[calc(100vh-2rem)] sm:max-h-[90vh]',
            'flex flex-col',
            'max-w-4xl',
            '-translate-x-1/2 -translate-y-1/2'
          )}
        >
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-6 py-4 border-b shrink-0">
            <div className="flex flex-col gap-1.5 flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <Database className="h-5 w-5 text-accent-purple" />
                <h2 className="text-lg font-semibold leading-none truncate">{title}</h2>
              </div>
              <p className="text-sm text-muted-foreground">{description}</p>
            </div>
            <button
              onClick={handleClose}
              className="inline-flex items-center justify-center rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
            {/* Create Button */}
            <div className="flex justify-between items-center">
              <div className="text-sm text-muted-foreground">
                {transforms.length} {t('automation:transforms', { defaultValue: 'transforms' })}
              </div>
              <Button onClick={handleCreateNew} size="sm" className="bg-accent-purple hover:bg-accent-purple/80">
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
                  {t('automation:noTransformsForDevice', { defaultValue: 'No transforms configured for this device yet.' })}
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
                            <Badge variant="outline" className="text-xs bg-accent-purple-light text-accent-purple">
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

          {/* Footer */}
          <div className="flex items-center justify-end gap-3 px-6 py-4 border-t shrink-0 bg-muted-30">
            <Button variant="outline" size="sm" onClick={handleClose}>
              {t('common:close', { defaultValue: 'Close' })}
            </Button>
          </div>
        </div>
      )}

      {/* Transform Builder Dialog */}
      <TransformBuilderSplit
        open={builderOpen}
        onOpenChange={setBuilderOpen}
        transform={editingTransform}
        devices={devices}
        deviceTypes={deviceTypes}
        onSave={handleSaveTransform}
      />

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
