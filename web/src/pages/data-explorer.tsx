import { useEffect, useState, useMemo, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { PageLayout } from '@/components/layout/PageLayout'
import { ResponsiveTable, type TableColumn, Pagination } from '@/components/shared'
import { PageTabsBar, PageTabsContent, PageTabsBottomNav } from '@/components/shared/PageTabs'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog'
import { Search, Database, RefreshCw, Cpu, Puzzle, Workflow, Brain } from 'lucide-react'
import { api } from '@/lib/api'
import type { UnifiedDataSourceInfo } from '@/types'
import { useIsMobile } from '@/hooks/useMobile'
import { useEvents } from '@/hooks/useEvents'

type SourceType = 'all' | string

function SourceTypeBadge({ type }: { type: string }) {
  const colorMap: Record<string, string> = {
    device: 'bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20',
    extension: 'bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20',
    transform: 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20',
    ai: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20',
  }
  const iconMap: Record<string, React.ComponentType<{ className?: string }>> = {
    device: Cpu, extension: Puzzle, transform: Workflow, ai: Brain,
  }
  const Icon = iconMap[type] || Database
  return (
    <Badge variant="outline" className={`text-[11px] px-1.5 py-0 h-6 gap-1 ${colorMap[type] || ''}`}>
      <Icon className="h-3 w-3" />
      {type}
    </Badge>
  )
}

function formatTime(timestamp?: number): string {
  if (!timestamp) return '-'
  // Backend telemetry timestamps are in seconds, normalize to ms
  const ms = timestamp < 1e12 ? timestamp * 1000 : timestamp
  const d = new Date(ms)
  const now = new Date()
  const isToday = d.toDateString() === now.toDateString()
  const pad = (n: number) => String(n).padStart(2, '0')
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  if (isToday) return time
  return `${d.getMonth() + 1}/${d.getDate()} ${time}`
}

export function DataExplorerPage() {
  const { t } = useTranslation(['common', 'data'])
  const isMobile = useIsMobile()

  const [sources, setSources] = useState<UnifiedDataSourceInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [search, setSearch] = useState('')
  const [activeType, setActiveType] = useState<SourceType>('all')
  const [selectedSourceName, setSelectedSourceName] = useState<string>('__all__')
  const [page, setPage] = useState(1)
  const [selectedSource, setSelectedSource] = useState<UnifiedDataSourceInfo | null>(null)
  const pageSize = 15
  const fetchRef = useRef(false)

  const fetchDataSources = useCallback(async () => {
    setLoading(true)
    try {
      const data = await api.listUnifiedDataSources()
      setSources(data)
    } catch (err) {
      console.error('[DataExplorer] Failed to fetch data sources:', err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (!fetchRef.current) {
      fetchRef.current = true
      fetchDataSources()
    }
  }, [fetchDataSources])

  // Real-time updates via events
  useEvents({
    enabled: true,
    category: 'device',
    onEvent: () => {
      fetchDataSources()
    },
  })

  // Unique source names for the current type tab
  const sourceOptions = useMemo(() => {
    const filtered = activeType === 'all'
      ? sources
      : sources.filter(s => s.source_type === activeType)
    const map = new Map<string, string>()
    filtered.forEach(s => {
      if (!map.has(s.source_name)) {
        map.set(s.source_name, s.source_display_name)
      }
    })
    return Array.from(map.entries()).sort((a, b) => a[1].localeCompare(b[1]))
  }, [sources, activeType])

  // Filter and search
  const filteredSources = useMemo(() => {
    let result = sources
    if (activeType !== 'all') {
      result = result.filter(s => s.source_type === activeType)
    }
    if (selectedSourceName !== '__all__') {
      result = result.filter(s => s.source_name === selectedSourceName)
    }
    if (search.trim()) {
      const q = search.toLowerCase()
      result = result.filter(s =>
        s.id.toLowerCase().includes(q) ||
        s.source_display_name.toLowerCase().includes(q) ||
        s.field_display_name.toLowerCase().includes(q) ||
        s.source_name.toLowerCase().includes(q) ||
        (s.description && s.description.toLowerCase().includes(q))
      )
    }
    return result
  }, [sources, activeType, selectedSourceName, search])

  // Pagination
  const paginatedSources = useMemo(() => {
    if (isMobile) {
      return filteredSources.slice(0, page * pageSize)
    }
    return filteredSources.slice((page - 1) * pageSize, page * pageSize)
  }, [filteredSources, page, pageSize, isMobile])

  // Reset page when filters change; reset source filter when type changes
  useEffect(() => { setPage(1) }, [search, activeType, selectedSourceName])
  useEffect(() => { setSelectedSourceName('__all__') }, [activeType])

  // Tabs config for PageTabsBar / PageTabsBottomNav
  // Fixed tabs - always shown for discoverability, even when empty
  const tabs = useMemo(() => [
    { value: 'all', label: t('data:tabs.all', 'All'), icon: <Database className="h-4 w-4" /> },
    { value: 'device', label: t('data:tabs.device', 'Device'), icon: <Cpu className="h-4 w-4" /> },
    { value: 'extension', label: t('data:tabs.extension', 'Extension'), icon: <Puzzle className="h-4 w-4" /> },
    { value: 'transform', label: t('data:tabs.transform', 'Transform'), icon: <Workflow className="h-4 w-4" /> },
    { value: 'ai', label: t('data:tabs.ai', 'AI Metrics'), icon: <Brain className="h-4 w-4" /> },
  ], [t])

  // Table columns
  const columns: TableColumn[] = [
    { key: 'id', label: 'ID', width: '30%' },
    { key: 'source_type', label: t('data:columns.type', 'Type'), width: '10%' },
    { key: 'source_display_name', label: t('data:columns.source', 'Source'), width: '20%' },
    { key: 'field_display_name', label: t('data:columns.field', 'Field'), width: '20%' },
    { key: 'data_type', label: t('data:columns.dataType', 'Data Type'), width: '10%' },
    { key: 'last_update', label: t('data:columns.updated', 'Updated'), width: '10%' },
  ]

  const renderCell = (columnKey: string, rowData: Record<string, unknown>) => {
    const source = rowData as unknown as UnifiedDataSourceInfo
    switch (columnKey) {
      case 'id':
        return (
          <div className="font-mono text-xs truncate max-w-[300px]" title={source.id}>
            {source.id}
          </div>
        )
      case 'source_type':
        return <SourceTypeBadge type={source.source_type} />
      case 'source_display_name':
        return <span className="text-sm">{source.source_display_name}</span>
      case 'field_display_name':
        return (
          <div className="flex flex-col">
            <span className="text-sm font-medium">{source.field_display_name}</span>
            {source.unit && <span className="text-xs text-muted-foreground">{source.unit}</span>}
          </div>
        )
      case 'data_type':
        return <Badge variant="secondary" className="text-[10px]">{source.data_type}</Badge>
      case 'last_update':
        return <span className="text-xs text-muted-foreground">{formatTime(source.last_update)}</span>
      default:
        return String(rowData[columnKey] ?? '')
    }
  }

  // Shared table component for all tabs
  const dataTable = (
    <ResponsiveTable
      columns={columns}
      data={paginatedSources as unknown as Record<string, unknown>[]}
      renderCell={renderCell}
      rowKey={(row) => (row as unknown as UnifiedDataSourceInfo).id}
      onRowClick={(row) => setSelectedSource(row as unknown as UnifiedDataSourceInfo)}
      loading={loading}
      flexHeight
      emptyState={
        <div className="text-center py-12">
          <Database className="h-12 w-12 mx-auto mb-3 text-muted-foreground/50" />
          <p className="text-sm text-muted-foreground">
            {search ? t('data:noResults', 'No data sources match your search') : t('data:noSources', 'No data sources found')}
          </p>
        </div>
      }
    />
  )

  // Source filter dropdown
  const sourceFilter = sourceOptions.length > 1 ? (
    <Select value={selectedSourceName} onValueChange={setSelectedSourceName}>
      <SelectTrigger className="w-[160px] md:w-[200px] h-9 text-sm">
        <SelectValue placeholder={t('data:filterSource', 'Filter source...')} />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="__all__">{t('data:allSources', 'All Sources')}</SelectItem>
        {sourceOptions.map(([name, displayName]) => (
          <SelectItem key={name} value={name}>{displayName}</SelectItem>
        ))}
      </SelectContent>
    </Select>
  ) : null

  return (
    <>
      <PageLayout
        title={t('data:title', 'Data Explorer')}
        subtitle={t('data:subtitle', 'Browse all data sources across devices, extensions, and transforms')}
        hideFooterOnMobile
        headerContent={
          <PageTabsBar
            tabs={tabs}
            activeTab={activeType}
            onTabChange={(v) => setActiveType(v)}
            actions={[
              {
                label: t('common:refresh', 'Refresh'),
                icon: <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />,
                variant: 'outline',
                onClick: fetchDataSources,
                disabled: loading,
              },
            ]}
            actionsExtra={
              <div className="flex items-center gap-2">
                {sourceFilter}
                <div className="relative">
                  <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    placeholder={t('data:search', 'Search data sources...')}
                    value={search}
                    onChange={e => setSearch(e.target.value)}
                    className="pl-9 w-[180px] md:w-[240px] h-9"
                  />
                </div>
              </div>
            }
          />
        }
        footer={
          filteredSources.length > pageSize ? (
            <Pagination
              total={filteredSources.length}
              pageSize={pageSize}
              currentPage={page}
              onPageChange={setPage}
            />
          ) : undefined
        }
      >
        <PageTabsContent value={activeType} activeTab={activeType}>
          {dataTable}
        </PageTabsContent>
      </PageLayout>

      {/* Mobile: Bottom navigation bar */}
      <PageTabsBottomNav
        tabs={tabs}
        activeTab={activeType}
        onTabChange={(v) => setActiveType(v)}
      />

      {/* Detail dialog */}
      <Dialog open={!!selectedSource} onOpenChange={(open) => !open && setSelectedSource(null)}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              {selectedSource && <SourceTypeBadge type={selectedSource.source_type} />}
              {selectedSource?.source_display_name}
            </DialogTitle>
            <DialogDescription>
              {selectedSource?.id}
            </DialogDescription>
          </DialogHeader>
          {selectedSource && (
            <div className="space-y-4 py-2">
              <div className="grid grid-cols-2 gap-3">
                <div className="space-y-1">
                  <p className="text-xs text-muted-foreground">Field</p>
                  <p className="text-sm font-medium">{selectedSource.field_display_name}</p>
                </div>
                <div className="space-y-1">
                  <p className="text-xs text-muted-foreground">Data Type</p>
                  <Badge variant="secondary">{selectedSource.data_type}</Badge>
                </div>
                <div className="space-y-1">
                  <p className="text-xs text-muted-foreground">Unit</p>
                  <p className="text-sm">{selectedSource.unit || '-'}</p>
                </div>
                <div className="space-y-1">
                  <p className="text-xs text-muted-foreground">Last Update</p>
                  <p className="text-sm">{formatTime(selectedSource.last_update)}</p>
                </div>
              </div>
              {selectedSource.description && (
                <div className="space-y-1">
                  <p className="text-xs text-muted-foreground">Description</p>
                  <p className="text-sm">{selectedSource.description}</p>
                </div>
              )}
              {selectedSource.current_value !== undefined && selectedSource.current_value !== null && (
                <div className="space-y-1">
                  <p className="text-xs text-muted-foreground">Current Value</p>
                  <p className="text-sm font-mono bg-muted p-2 rounded break-all overflow-hidden max-h-32">
                    {typeof selectedSource.current_value === 'object'
                      ? JSON.stringify(selectedSource.current_value, null, 2)
                      : String(selectedSource.current_value)}
                  </p>
                </div>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>
    </>
  )
}
