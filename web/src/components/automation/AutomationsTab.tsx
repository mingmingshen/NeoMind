import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/lib/api'
import type { Automation } from '@/types'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from '@/components/ui/dropdown-menu'
import { LoadingState, EmptyState, ResponsiveTable } from '@/components/shared'
import { AutomationCreatorDialog } from '@/components/automation'
import { formatTimestamp } from '@/lib/utils/format'
import { useToast } from '@/hooks/use-toast'
import { useErrorHandler } from '@/hooks/useErrorHandler'
import { showErrorToast } from '@/lib/error-messages'
import { confirm } from '@/hooks/use-confirm'
import {
  Plus,
  Search,
  Play,
  Edit,
  Trash2,
  Sparkles,
  MoreVertical,
} from 'lucide-react'
import { Switch } from '@/components/ui/switch'
import { useIsMobile } from '@/hooks/useMobile'

export interface AutomationsTabProps {
  searchQuery?: string
  onSearchChange?: (query: string) => void
}

type FilterType = 'all' | 'transform'
type StatusFilter = 'all' | 'enabled' | 'disabled'

export function AutomationsTab({ searchQuery: externalSearchQuery, onSearchChange }: AutomationsTabProps) {
  const { t } = useTranslation(['automation', 'common'])
  const { toast } = useToast()
  const { handleError } = useErrorHandler()
  const isMobile = useIsMobile()

  // Data state
  const [automations, setAutomations] = useState<Automation[]>([])
  const [loading, setLoading] = useState(true)

  // Filter state
  const [typeFilter, setTypeFilter] = useState<FilterType>('all')
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all')
  const [searchQuery, setSearchQuery] = useState(externalSearchQuery || '')

  // Dialog state
  const [creatorOpen, setCreatorOpen] = useState(false)

  // Sync external search query
  useEffect(() => {
    if (externalSearchQuery !== undefined) {
      setSearchQuery(externalSearchQuery)
    }
  }, [externalSearchQuery])

  const loadAutomations = async () => {
    setLoading(true)
    try {
      const response = await api.listAutomations({
        type: typeFilter === 'all' ? undefined : typeFilter,
        enabled: statusFilter === 'all' ? undefined : statusFilter === 'enabled',
        search: searchQuery || undefined,
      })
      setAutomations(response.automations || [])
    } catch (error) {
      handleError(error, { operation: 'Load automations', showToast: false })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadAutomations()
  }, [typeFilter, statusFilter, searchQuery])

  const handleToggleEnabled = async (automation: Automation) => {
    const newEnabled = !automation.enabled
    try {
      await api.setAutomationStatus(automation.id, newEnabled)
      toast({
        title: t('common:success'),
        description: newEnabled ? t('automation:enabled') : t('automation:disabled'),
      })
      loadAutomations()
    } catch (error) {
      showErrorToast(toast, error, t('common:failed'))
    }
  }

  const handleExecute = async (automation: Automation) => {
    try {
      toast({
        title: t('common:success'),
        description: t('automation:executed'),
      })
    } catch (error) {
      showErrorToast(toast, error, t('common:failed'))
    }
  }

  const handleDelete = async (automation: Automation) => {
    const confirmed = await confirm({
      title: t('common:delete'),
      description: t('automation:deleteConfirm'),
      confirmText: t('common:delete'),
      cancelText: t('common:cancel'),
      variant: "destructive"
    })
    if (!confirmed) return

    try {
      await api.deleteAutomation(automation.id)
      toast({
        title: t('common:success'),
        description: t('automation:deleted'),
      })
      loadAutomations()
    } catch (error) {
      showErrorToast(toast, error, t('common:failed'))
    }
  }

  const handleSearchChange = (value: string) => {
    setSearchQuery(value)
    onSearchChange?.(value)
  }

  const getTypeLabel = (_type: string) =>
    t('automation:transforms')

  const getTypeColor = (_type: string) =>
    'bg-success-light text-success border-success-light dark:bg-success-light dark:text-success'

  const getComplexityDots = (complexity: number) => {
    return Array.from({ length: 5 }, (_, i) => (
      <span
        key={i}
        className={`w-2 h-2 rounded-full ${
          i < complexity ? 'bg-warning' : 'bg-muted-foreground/30 dark:bg-muted-foreground/50'
        }`}
      />
    ))
  }

  return (
    <>
      {/* Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 mb-6">
        <div className="flex flex-wrap items-center gap-3">
          <Select value={typeFilter} onValueChange={(v) => setTypeFilter(v as FilterType)}>
            <SelectTrigger className="w-[140px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t('automation:all')}</SelectItem>
              <SelectItem value="transform">{t('automation:transforms')}</SelectItem>
            </SelectContent>
          </Select>

          <Select value={statusFilter} onValueChange={(v) => setStatusFilter(v as StatusFilter)}>
            <SelectTrigger className="w-[140px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t('automation:all')}</SelectItem>
              <SelectItem value="enabled">{t('automation:enabled')}</SelectItem>
              <SelectItem value="disabled">{t('automation:disabled')}</SelectItem>
            </SelectContent>
          </Select>

          <div className="relative">
            <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder={t('automation:searchPlaceholder', { defaultValue: 'Search...' })}
              value={searchQuery}
              onChange={(e) => handleSearchChange(e.target.value)}
              className="pl-8 w-[200px]"
            />
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Button size="sm" onClick={() => setCreatorOpen(true)}>
            <Plus className="h-4 w-4 mr-1" />
            {t('automation:createAutomation')}
          </Button>
        </div>
      </div>

      {/* Content */}
      {loading ? (
        <LoadingState variant="page" text={t('automation:loading')} />
      ) : !automations || automations.length === 0 ? (
        <EmptyState
          icon={<Sparkles className="h-12 w-12 text-muted-foreground" />}
          title={t('automation:noAutomations', { defaultValue: 'No automations' })}
          description={t('automation:noAutomationsDesc', {
            defaultValue: 'Create your first automation using AI, templates, or manual configuration',
          })}
          action={{
            label: t('automation:createAutomation'),
            onClick: () => setCreatorOpen(true),
          }}
        />
      ) : isMobile ? (
        <div className="space-y-2">
          {automations.map((automation) => (
            <Card
              key={automation.id}
              className="overflow-hidden border-border shadow-sm active:scale-[0.99] transition-all"
            >
              <div className="px-3 py-2.5">
                {/* Row 1: icon + name + switch + actions */}
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-lg flex items-center justify-center shrink-0 bg-warning-light text-warning">
                    <Sparkles className="h-4 w-4" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm truncate">{automation.name}</div>
                  </div>
                  <Switch
                    checked={automation.enabled}
                    onCheckedChange={() => handleToggleEnabled(automation)}
                    className="scale-75"
                  />
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <button className="p-1 rounded-md hover:bg-muted">
                        <MoreVertical className="h-4 w-4 text-muted-foreground" />
                      </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => handleExecute(automation)}>
                        <Play className="h-4 w-4 mr-2" />
                        {t('automation:execute')}
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => {}}>
                        <Edit className="h-4 w-4 mr-2" />
                        {t('automation:edit')}
                      </DropdownMenuItem>
                      <DropdownMenuItem
                        className="text-error"
                        onClick={() => handleDelete(automation)}
                      >
                        <Trash2 className="h-4 w-4 mr-2" />
                        {t('automation:delete')}
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
                {/* Row 2: type + complexity + execution count + time */}
                <div className="flex items-center gap-1.5 mt-1.5 ml-[42px]">
                  <Badge variant="outline" className={getTypeColor(automation.type) + " text-[11px] h-5 px-1.5"}>
                    {getTypeLabel(automation.type)}
                  </Badge>
                  <div className="flex gap-0.5">{getComplexityDots(automation.complexity)}</div>
                  <span className="text-[11px] text-muted-foreground ml-auto">
                    {automation.execution_count}x &middot; {formatTimestamp(automation.updated_at)}
                  </span>
                </div>
              </div>
            </Card>
          ))}
        </div>
      ) : (
        <ResponsiveTable
          columns={[
            {
              key: 'enabled',
              label: '',
              width: 'w-[50px]',
              align: 'center',
            },
            {
              key: 'name',
              label: t('automation:automationName'),
            },
            {
              key: 'type',
              label: t('automation:recommendedType'),
              align: 'center',
            },
            {
              key: 'complexity',
              label: t('automation:complexity'),
              align: 'center',
            },
            {
              key: 'status',
              label: t('automation:status'),
              align: 'center',
            },
            {
              key: 'execution_count',
              label: t('automation:executionCount'),
              align: 'center',
            },
            {
              key: 'updated_at',
              label: t('automation:updatedAt'),
            },
          ]}
          data={automations as unknown as Record<string, unknown>[]}
          rowKey={(auto) => (auto as unknown as Automation).id}
          renderCell={(columnKey, rowData) => {
            const automation = rowData as unknown as Automation
            switch (columnKey) {
              case 'enabled':
                return (
                  <Switch
                    checked={automation.enabled}
                    onCheckedChange={() => handleToggleEnabled(automation)}
                  />
                )
              case 'name':
                return (
                  <div>
                    <p className="font-medium">{automation.name}</p>
                    <p className="text-sm text-muted-foreground line-clamp-1">
                      {automation.description}
                    </p>
                  </div>
                )
              case 'type':
                return (
                  <Badge variant="outline" className={getTypeColor(automation.type)}>
                    {getTypeLabel(automation.type)}
                  </Badge>
                )
              case 'complexity':
                return <div className="flex gap-0.5 justify-center">{getComplexityDots(automation.complexity)}</div>
              case 'status':
                return (
                  <Badge variant={automation.enabled ? 'default' : 'secondary'}>
                    {automation.enabled ? t('automation:enabled') : t('automation:disabled')}
                  </Badge>
                )
              case 'execution_count':
                return <span>{automation.execution_count}</span>
              case 'updated_at':
                return (
                  <span className="text-sm text-muted-foreground">
                    {formatTimestamp(automation.updated_at)}
                  </span>
                )
              default:
                return null
            }
          }}
          actions={[
            {
              label: t('automation:execute'),
              icon: <Play className="h-4 w-4" />,
              onClick: (rowData) => {
                const automation = rowData as unknown as Automation
                handleExecute(automation)
              },
            },
            {
              label: t('automation:edit'),
              icon: <Edit className="h-4 w-4" />,
              onClick: () => {},
            },
            {
              label: t('automation:delete'),
              icon: <Trash2 className="h-4 w-4" />,
              variant: 'destructive',
              onClick: (rowData) => {
                const automation = rowData as unknown as Automation
                handleDelete(automation)
              },
            },
          ]}
        />
      )}

      {/* Dialogs */}
      <AutomationCreatorDialog
        open={creatorOpen}
        onOpenChange={setCreatorOpen}
        onAutomationCreated={loadAutomations}
      />
    </>
  )
}
