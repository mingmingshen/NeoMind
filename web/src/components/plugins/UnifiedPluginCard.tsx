import { useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  CardFooter,
} from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  MoreVertical,
  Edit,
  Trash2,
  TestTube,
  Wifi,
  Loader2,
  Server,
  Home,
  Network,
  BrainCircuit,
  Sparkles,
  Gem,
  Zap,
  FileCode,
  Settings,
  Play,
  Square,
  RefreshCw,
  AlertTriangle,
  CheckCircle,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import type {
  PluginUISchema,
  DynamicPluginMetadata,
  PluginStatus,
  PluginType,
  PluginCategory,
  PluginCapabilities,
} from '@/types/plugin-schema'
import { SchemaConfigForm } from './SchemaConfigForm'

// ============================================================================
// Icons
// ============================================================================

const ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  Server,
  Home,
  Network,
  BrainCircuit,
  Sparkles,
  Gem,
  Zap,
  Wifi,
  FileCode,
  Settings,
}

function getIcon(iconName?: string) {
  if (!iconName) return Server
  return ICONS[iconName] || Server
}

// ============================================================================
// Unified Plugin Data
// ============================================================================

export interface UnifiedPluginData {
  id: string
  schema: PluginUISchema
  metadata?: DynamicPluginMetadata
  status: PluginStatus
  config: Record<string, unknown>
  // Device adapter specific
  deviceCount?: number
  connected?: boolean
  // Additional display data
  lastError?: string
  version?: string
  author?: string
}

// ============================================================================
// Props
// ============================================================================

export interface UnifiedPluginCardProps {
  plugin: UnifiedPluginData
  onToggle?: (id: string, enabled: boolean) => Promise<boolean>
  onStart?: (id: string) => Promise<boolean>
  onStop?: (id: string) => Promise<boolean>
  onDelete?: (id: string) => Promise<boolean>
  onConfigure?: (id: string, config: Record<string, unknown>) => Promise<boolean>
  onTest?: (id: string) => Promise<boolean>
  onRefresh?: (id: string) => Promise<boolean>
  onViewDevices?: (id: string) => void
}

// ============================================================================
// Capabilities Badge Component
// ============================================================================

interface PluginCapabilitiesBadgeProps {
  capabilities: PluginCapabilities
  compact?: boolean
}

export function PluginCapabilitiesBadge({ capabilities, compact = false }: PluginCapabilitiesBadgeProps) {
  const { t } = useTranslation('plugins')

  const items: Array<{ key: keyof PluginCapabilities; label: string; icon: React.ComponentType<{ className?: string }> }> = [
    { key: 'async', label: t('capabilities.async'), icon: Zap },
    { key: 'threadSafe', label: t('capabilities.threadSafe'), icon: Sparkles },
    { key: 'streaming', label: t('capabilities.streaming'), icon: BrainCircuit },
    { key: 'stateless', label: t('capabilities.stateless'), icon: Server },
    { key: 'hotReload', label: t('capabilities.hotReload'), icon: RefreshCw },
  ]

  const activeItems = items.filter(item => capabilities[item.key])

  if (compact) {
    return (
      <div className="flex items-center gap-1">
        {activeItems.slice(0, 3).map(item => {
          const Icon = item.icon
          return <Icon key={item.key} className="h-3 w-3 text-muted-foreground" />
        })}
        {activeItems.length > 3 && (
          <span className="text-xs text-muted-foreground">+{activeItems.length - 3}</span>
        )}
      </div>
    )
  }

  return (
    <div className="flex flex-wrap gap-1">
      {activeItems.map(item => {
        const Icon = item.icon
        return (
          <Badge key={item.key} variant="outline" className="text-xs">
            <Icon className="h-3 w-3 mr-1" />
            {item.label}
          </Badge>
        )
      })}
    </div>
  )
}

// ============================================================================
// Status Badge Component
// ============================================================================

interface StatusBadgeProps {
  state: PluginStatus['state']
  health?: PluginStatus['health']
}

export function StatusBadge({ state }: StatusBadgeProps) {
  const { t } = useTranslation('plugins')

  const getStatusConfig = () => {
    switch (state) {
      case 'running':
        return {
          variant: 'default' as const,
          icon: CheckCircle,
          label: t('running'),
          className: 'bg-success text-success-foreground',
        }
      case 'loaded':
        return {
          variant: 'secondary' as const,
          icon: Server,
          label: t('loaded'),
          className: '',
        }
      case 'error':
        return {
          variant: 'destructive' as const,
          icon: AlertTriangle,
          label: t('error'),
          className: '',
        }
      case 'unloading':
        return {
          variant: 'outline' as const,
          icon: Loader2,
          label: t('unloading'),
          className: '',
        }
      default:
        return {
          variant: 'outline' as const,
          icon: Server,
          label: t('stopped'),
          className: '',
        }
    }
  }

  const config = getStatusConfig()
  const Icon = config.icon

  return (
    <Badge variant={config.variant} className={cn("text-xs", config.className)}>
      {state === 'unloading' ? (
        <Icon className="h-3 w-3 mr-1 animate-spin" />
      ) : (
        <Icon className="h-3 w-3 mr-1" />
      )}
      {config.label}
    </Badge>
  )
}

// ============================================================================
// Type Badge Component
// ============================================================================

interface TypeBadgeProps {
  type: PluginType
  category: PluginCategory
  isDynamic?: boolean
}

export function TypeBadge({ type, category, isDynamic }: TypeBadgeProps) {
  const { t } = useTranslation('plugins')

  const getTypeLabel = () => {
    if (isDynamic) return t('dynamicPlugin')
    switch (type) {
      case 'llm_backend':
        return t('llmBackend')
      case 'device_adapter':
        return t('deviceAdapter')
      case 'storage_backend':
        return t('storageBackend')
      case 'alert_channel':
        return t('alertChannel')
      case 'integration':
        return t('integration')
      case 'tool':
        return t('tool')
      case 'native':
        return t('builtin')
      default:
        return type
    }
  }

  const getCategoryColor = () => {
    switch (category) {
      case 'ai':
        return 'bg-purple-100 text-purple-700 dark:bg-purple-900/20 dark:text-purple-400'
      case 'devices':
        return 'bg-blue-100 text-blue-700 dark:bg-blue-900/20 dark:text-blue-400'
      case 'storage':
        return 'bg-amber-100 text-amber-700 dark:bg-amber-900/20 dark:text-amber-400'
      case 'notify':
        return 'bg-green-100 text-green-700 dark:bg-green-900/20 dark:text-green-400'
      default:
        return 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-400'
    }
  }

  return (
    <Badge variant="outline" className={cn("text-xs", getCategoryColor())}>
      {getTypeLabel()}
    </Badge>
  )
}

// ============================================================================
// Main Card Component
// ============================================================================

export function UnifiedPluginCard({
  plugin,
  onToggle,
  onStart,
  onStop,
  onDelete,
  onConfigure,
  onTest,
  onRefresh,
  onViewDevices,
}: UnifiedPluginCardProps) {
  const { t } = useTranslation(['plugins', 'common'])
  const [configDialogOpen, setConfigDialogOpen] = useState(false)
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [saving, setSaving] = useState(false)
  const [actionLoading, setActionLoading] = useState<string | null>(null)

  const { schema, status, config, metadata } = plugin
  const isRunning = status.state === 'running'
  const canEdit = !schema.builtin
  const canDelete = canEdit && (schema.type === 'dynamic' || schema.type === 'integration')
  const canToggle = !schema.builtin && status.state !== 'unloading'

  const handleAction = useCallback(async (
    actionName: string,
    action: () => Promise<boolean>
  ) => {
    setActionLoading(actionName)
    try {
      const result = await action()
      return result
    } finally {
      setActionLoading(null)
    }
  }, [])

  const handleToggle = async () => {
    if (!onToggle) return
    await handleAction('toggle', () => onToggle(plugin.id, !status.enabled))
  }

  const handleStart = async () => {
    if (!onStart) return
    const result = await handleAction('start', () => onStart(plugin.id))
    if (result && onRefresh) {
      await onRefresh(plugin.id)
    }
  }

  const handleStop = async () => {
    if (!onStop) return
    const result = await handleAction('stop', () => onStop(plugin.id))
    if (result && onRefresh) {
      await onRefresh(plugin.id)
    }
  }

  const handleTest = async () => {
    if (!onTest) return
    await handleAction('test', () => onTest(plugin.id))
  }

  const handleRefresh = async () => {
    if (!onRefresh) return
    await handleAction('refresh', () => onRefresh(plugin.id))
  }

  const handleDelete = async () => {
    if (!onDelete) return
    const result = await handleAction('delete', () => onDelete(plugin.id))
    if (result) {
      setDeleteDialogOpen(false)
    }
  }

  const handleSaveConfig = async (newConfig: Record<string, unknown>) => {
    if (!onConfigure) return
    setSaving(true)
    try {
      await onConfigure(plugin.id, newConfig)
    } finally {
      setSaving(false)
    }
  }

  const getIconBg = () => {
    switch (schema.category) {
      case 'ai':
        return 'bg-purple-100 text-purple-700 dark:bg-purple-900/20 dark:text-purple-400'
      case 'devices':
        return 'bg-blue-100 text-blue-700 dark:bg-blue-900/20 dark:text-blue-400'
      case 'storage':
        return 'bg-amber-100 text-amber-700 dark:bg-amber-900/20 dark:text-amber-400'
      case 'notify':
        return 'bg-green-100 text-green-700 dark:bg-green-900/20 dark:text-green-400'
      default:
        return 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-400'
    }
  }

  const Icon = getIcon(schema.icon)

  return (
    <>
      <Card
        className={cn(
          "transition-all duration-200 hover:shadow-md",
          isRunning && "border-green-500/50",
          status.state === 'error' && "border-destructive/50"
        )}
      >
        <CardHeader className="pb-3">
          <div className="flex items-start justify-between">
            <div className="flex items-start gap-3 flex-1">
              {/* Icon */}
              <div className={cn("flex items-center justify-center w-12 h-12 rounded-lg", getIconBg())}>
                <Icon className="h-6 w-6" />
              </div>

              {/* Title and badges */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1 flex-wrap">
                  <CardTitle className="text-base truncate">
                    {config.name as string || schema.name}
                  </CardTitle>
                  <StatusBadge state={status.state} health={status.health} />
                  <TypeBadge type={schema.type} category={schema.category} isDynamic={metadata !== undefined} />
                </div>
                <CardDescription className="text-xs">
                  {schema.description}
                </CardDescription>

                {/* Metadata info */}
                <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
                  {plugin.version && (
                    <span>v{plugin.version}</span>
                  )}
                  {plugin.author && (
                    <span>by {plugin.author}</span>
                  )}
                  {metadata?.capabilities && (
                    <PluginCapabilitiesBadge capabilities={metadata.capabilities} compact />
                  )}
                </div>
              </div>
            </div>

            {/* Action Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {canEdit && (
                  <DropdownMenuItem onClick={() => setConfigDialogOpen(true)}>
                    <Edit className="mr-2 h-4 w-4" />
                    {t('plugins:edit')}
                  </DropdownMenuItem>
                )}

                {onTest && (
                  <DropdownMenuItem onClick={handleTest} disabled={actionLoading === 'test'}>
                    <TestTube className="mr-2 h-4 w-4" />
                    {t('plugins:llm.testConnection')}
                  </DropdownMenuItem>
                )}

                {onRefresh && (
                  <DropdownMenuItem onClick={handleRefresh} disabled={actionLoading === 'refresh'}>
                    <RefreshCw className={cn("mr-2 h-4 w-4", actionLoading === 'refresh' && "animate-spin")} />
                    {t('common:refresh')}
                  </DropdownMenuItem>
                )}

                <DropdownMenuSeparator />

                {isRunning && onStop && (
                  <DropdownMenuItem onClick={handleStop} disabled={actionLoading === 'stop'}>
                    <Square className="mr-2 h-4 w-4" />
                    {t('plugins:stop')}
                  </DropdownMenuItem>
                )}
                {!isRunning && onStart && (
                  <DropdownMenuItem onClick={handleStart} disabled={actionLoading === 'start'}>
                    <Play className="mr-2 h-4 w-4" />
                    {t('plugins:start')}
                  </DropdownMenuItem>
                )}

                {onViewDevices && plugin.deviceCount !== undefined && (
                  <DropdownMenuItem onClick={() => onViewDevices(plugin.id)}>
                    <Wifi className="mr-2 h-4 w-4" />
                    {t('plugins:viewDevicesWithCount', { count: plugin.deviceCount })}
                  </DropdownMenuItem>
                )}

                {canDelete && (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={() => setDeleteDialogOpen(true)}
                      className="text-destructive focus:text-destructive"
                    >
                      <Trash2 className="mr-2 h-4 w-4" />
                      {t('plugins:delete')}
                    </DropdownMenuItem>
                  </>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </CardHeader>

        <CardContent className="pb-3">
          <div className="space-y-2 text-sm">
            {/* Status info */}
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">{t('plugins:status')}:</span>
              <span className={cn(
                "font-medium",
                status.health === 'healthy' && "text-success",
                status.health === 'degraded' && "text-warning",
                status.health === 'unhealthy' && "text-destructive"
              )}>
                {status.health === 'healthy' ? t('healthy') :
                 status.health === 'degraded' ? t('degraded') : t('unhealthy')}
              </span>
            </div>

            {/* Device count */}
            {plugin.deviceCount !== undefined && (
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">{t('plugins:deviceCount')}:</span>
                <span className="font-medium">{plugin.deviceCount}</span>
              </div>
            )}

            {/* Connection status */}
            {plugin.connected !== undefined && (
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">{t('plugins:connection')}:</span>
                <span className={plugin.connected ? "text-success font-medium" : "text-muted-foreground font-medium"}>
                  {plugin.connected ? t('plugins:connected') : t('plugins:disconnected')}
                </span>
              </div>
            )}

            {/* Stats */}
            {status.stats && (
              <>
                {status.stats.totalRequests !== undefined && (
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">{t('plugins:totalRequests')}:</span>
                    <span className="font-medium">{status.stats.totalRequests}</span>
                  </div>
                )}
              </>
            )}

            {/* Last error */}
            {status.lastError && (
              <div className="flex items-start justify-between">
                <span className="text-muted-foreground">{t('plugins:lastError')}:</span>
                <span className="text-destructive font-medium text-xs max-w-[200px] truncate">
                  {status.lastError}
                </span>
              </div>
            )}
          </div>
        </CardContent>

        <CardFooter className="pt-3 border-t justify-between">
          {/* Enable/Disable toggle */}
          {canToggle && onToggle && (
            <div className="flex items-center gap-2">
              <Switch
                checked={status.enabled}
                onCheckedChange={handleToggle}
                disabled={actionLoading === 'toggle' || status.state === 'unloading'}
              />
              <span className="text-xs text-muted-foreground">
                {status.enabled ? t('plugins:enabled') : t('plugins:disabled')}
              </span>
            </div>
          )}

          {/* Action buttons */}
          <div className="flex items-center gap-2">
            {onViewDevices && plugin.deviceCount !== undefined && plugin.deviceCount > 0 && (
              <Button variant="outline" size="sm" onClick={() => onViewDevices(plugin.id)}>
                <Wifi className="mr-2 h-4 w-4" />
                {t('plugins:viewDevices')}
              </Button>
            )}
            {canEdit && (
              <Button variant="outline" size="sm" onClick={() => setConfigDialogOpen(true)}>
                <Settings className="mr-2 h-4 w-4" />
                {t('plugins:configure')}
              </Button>
            )}
          </div>
        </CardFooter>
      </Card>

      {/* Config Dialog */}
      <Dialog open={configDialogOpen} onOpenChange={setConfigDialogOpen}>
        <DialogContent className="max-w-md max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <div className={cn("p-2 rounded-lg", getIconBg())}>
                <Icon className="h-5 w-5" />
              </div>
              {t('plugins:configure')} {schema.name}
            </DialogTitle>
            <DialogDescription>
              {schema.description}
            </DialogDescription>
          </DialogHeader>

          <SchemaConfigForm
            schema={schema}
            config={config}
            onChange={() => {/* Handled by dialog */}}
          />

          <DialogFooter>
            <Button variant="outline" onClick={() => setConfigDialogOpen(false)} disabled={saving}>
              {t('common:cancel')}
            </Button>
            <Button onClick={() => handleSaveConfig(config)} disabled={saving}>
              {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {t('plugins:save')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('plugins:deleteConfirmTitle')}</DialogTitle>
            <DialogDescription>
              {t('plugins:deleteConfirmDesc', { name: config.name as string || schema.name })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialogOpen(false)} disabled={actionLoading === 'delete'}>
              {t('common:cancel')}
            </Button>
            <Button
              variant="destructive"
              onClick={handleDelete}
              disabled={actionLoading === 'delete'}
            >
              {actionLoading === 'delete' && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {t('plugins:delete')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}

// ============================================================================
// Compact Card Variant (for grid views)
// ============================================================================

export interface UnifiedPluginCardCompactProps {
  plugin: UnifiedPluginData
  onClick: () => void
  isActive?: boolean
}

export function UnifiedPluginCardCompact({
  plugin,
  onClick,
  isActive = false,
}: UnifiedPluginCardCompactProps) {
  const { t } = useTranslation('plugins')

  const { schema, status } = plugin
  const isRunning = status.state === 'running'

  const getIconBg = () => {
    switch (schema.category) {
      case 'ai':
        return 'bg-purple-100 text-purple-700 dark:bg-purple-900/20 dark:text-purple-400'
      case 'devices':
        return 'bg-blue-100 text-blue-700 dark:bg-blue-900/20 dark:text-blue-400'
      case 'storage':
        return 'bg-amber-100 text-amber-700 dark:bg-amber-900/20 dark:text-amber-400'
      case 'notify':
        return 'bg-green-100 text-green-700 dark:bg-green-900/20 dark:text-green-400'
      default:
        return 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-400'
    }
  }

  const Icon = getIcon(schema.icon)

  return (
    <Card
      className={cn(
        "transition-all duration-200 cursor-pointer hover:shadow-md",
        isActive && "border-primary border-2",
        isRunning && "border-green-500/50"
      )}
      onClick={onClick}
    >
      <CardHeader className="pb-3">
        <div className={cn("flex items-center justify-center w-12 h-12 rounded-lg mx-auto", getIconBg())}>
          <Icon className="h-6 w-6" />
        </div>
        <CardTitle className="text-base mt-3 text-center">{schema.name}</CardTitle>
        <CardDescription className="mt-1 text-xs text-center">
          {schema.description}
        </CardDescription>
      </CardHeader>
      <CardContent className="text-sm">
        <div className="flex justify-between items-center">
          <span className="text-muted-foreground">{t('plugins:status')}:</span>
          <StatusBadge state={status.state} />
        </div>
      </CardContent>
    </Card>
  )
}
