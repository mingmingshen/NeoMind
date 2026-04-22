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
  DialogContentBody,
} from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Search, Database, RefreshCw, Cpu, Puzzle, Workflow, Brain, History, Loader2, Eye } from 'lucide-react'
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
  const ms = timestamp < 1e12 ? timestamp * 1000 : timestamp
  const d = new Date(ms)
  const now = new Date()
  const isToday = d.toDateString() === now.toDateString()
  const pad = (n: number) => String(n).padStart(2, '0')
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  if (isToday) return time
  return `${d.getMonth() + 1}/${d.getDate()} ${time}`
}

function formatDateTime(timestamp: number): string {
  const ms = timestamp < 1e12 ? timestamp * 1000 : timestamp
  const d = new Date(ms)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

export function DataExplorerPage() {
  const { t } = useTranslation(['common', 'data'])
  const isMobile = useIsMobile()

  // Server-side paginated state
  const [pageData, setPageData] = useState<UnifiedDataSourceInfo[]>([])
  const [totalCount, setTotalCount] = useState(0)
  const [sourceOptions, setSourceOptions] = useState<[string, string][]>([])
  const [loading, setLoading] = useState(true)

  // Filters
  const [search, setSearch] = useState('')
  const [activeType, setActiveType] = useState<SourceType>('all')
  const [selectedSourceName, setSelectedSourceName] = useState<string>('__all__')
  const [page, setPage] = useState(1)
  const pageSize = 10

  // Debounced search value
  const [debouncedSearch, setDebouncedSearch] = useState('')
  const searchTimerRef = useRef<ReturnType<typeof setTimeout>>()

  // Detail dialog
  const [selectedSource, setSelectedSource] = useState<UnifiedDataSourceInfo | null>(null)
  const [historyRange, setHistoryRange] = useState<string>('1h')
  const [historyData, setHistoryData] = useState<Array<{ timestamp: number; value: unknown; quality: number | null }>>([])
  const [historyLoading, setHistoryLoading] = useState(false)

  // Fetch page from server
  const fetchDataSources = useCallback(async () => {
    setLoading(true)
    try {
      const params: Record<string, string | number> = {
        offset: (page - 1) * pageSize,
        limit: pageSize,
      }
      if (activeType !== 'all') params.source_type = activeType
      if (selectedSourceName !== '__all__') params.source = selectedSourceName
      if (debouncedSearch.trim()) params.search = debouncedSearch.trim()

      const res = await api.listUnifiedDataSources(params)
      setPageData(res.data)
      setTotalCount(res.total)
      setSourceOptions(res.source_options)
    } catch (err) {
      console.error('[DataExplorer] Failed to fetch data sources:', err)
    } finally {
      setLoading(false)
    }
  }, [page, activeType, selectedSourceName, debouncedSearch, pageSize])

  // Fetch on mount and when filters/page change
  useEffect(() => {
    fetchDataSources()
  }, [fetchDataSources])

  // Reset page when filters change
  useEffect(() => { setPage(1) }, [debouncedSearch, activeType, selectedSourceName])
  useEffect(() => { setSelectedSourceName('__all__') }, [activeType])

  // Debounce search input
  useEffect(() => {
    if (searchTimerRef.current) clearTimeout(searchTimerRef.current)
    searchTimerRef.current = setTimeout(() => setDebouncedSearch(search), 300)
    return () => { if (searchTimerRef.current) clearTimeout(searchTimerRef.current) }
  }, [search])

  // Refresh on device events
  useEvents({
    enabled: true,
    category: 'device',
    onEvent: () => {
      fetchDataSources()
    },
  })

  // Fetch historical telemetry when a source is selected or range changes
  useEffect(() => {
    if (!selectedSource) {
      setHistoryData([])
      return
    }
    const rangeSeconds: Record<string, number> = {
      '1h': 3600, '6h': 21600, '24h': 86400, '7d': 604800,
    }
    const now = Math.floor(Date.now() / 1000)
    const start = now - (rangeSeconds[historyRange] || 3600)

    const parts = selectedSource.id.split(':')
    if (parts.length < 3) return
    const source = `${parts[0]}:${parts[1]}`
    const metric = parts.slice(2).join(':')

    let cancelled = false
    setHistoryLoading(true)
    api.queryTelemetry(source, metric, start, now, 500).then(res => {
      if (cancelled) return
      setHistoryData(res.data.map(p => ({
        timestamp: p.timestamp,
        value: p.value,
        quality: p.quality,
      })))
    }).catch(err => {
      console.error('[DataExplorer] Failed to fetch history:', err)
      if (!cancelled) setHistoryData([])
    }).finally(() => {
      if (!cancelled) setHistoryLoading(false)
    })
    return () => { cancelled = true }
  }, [selectedSource, historyRange])

  const tabs = useMemo(() => [
    { value: 'all', label: t('data:tabs.all', 'All'), icon: <Database className="h-4 w-4" /> },
    { value: 'device', label: t('data:tabs.device', 'Device'), icon: <Cpu className="h-4 w-4" /> },
    { value: 'extension', label: t('data:tabs.extension', 'Extension'), icon: <Puzzle className="h-4 w-4" /> },
    { value: 'transform', label: t('data:tabs.transform', 'Transform'), icon: <Workflow className="h-4 w-4" /> },
    { value: 'ai', label: t('data:tabs.ai', 'AI Metrics'), icon: <Brain className="h-4 w-4" /> },
  ], [t])

  const columns: TableColumn[] = [
    { key: 'source_display_name', label: t('data:columns.source', 'Source'), width: '20%' },
    { key: 'field_display_name', label: t('data:columns.field', 'Field'), width: '22%' },
    { key: 'id', label: 'ID', width: '18%' },
    { key: 'source_type', label: t('data:columns.type', 'Type'), width: '10%' },
    { key: 'data_type', label: t('data:columns.dataType', 'Data Type'), width: '10%' },
    { key: 'last_update', label: t('data:columns.updated', 'Updated'), width: '12%' },
    { key: 'actions', label: '', width: '8%' },
  ]

  const renderCell = (columnKey: string, rowData: Record<string, unknown>) => {
    const source = rowData as unknown as UnifiedDataSourceInfo
    switch (columnKey) {
      case 'source_display_name':
        return (
          <span className="text-sm font-medium text-foreground/80">{source.source_display_name}</span>
        )
      case 'field_display_name':
        return (
          <div className="flex flex-col">
            <span className="text-sm font-medium">{source.field_display_name}</span>
            {source.unit && <span className="text-xs text-muted-foreground">{source.unit}</span>}
          </div>
        )
      case 'id':
        return (
          <div className="font-mono text-xs truncate max-w-[200px]" title={source.id}>
            {source.id}
          </div>
        )
      case 'source_type':
        return <SourceTypeBadge type={source.source_type} />
      case 'data_type':
        return <Badge variant="secondary" className="text-[10px]">{source.data_type}</Badge>
      case 'last_update':
        return <span className="text-xs text-muted-foreground">{formatTime(source.last_update)}</span>
      case 'actions':
        return (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2"
            onClick={(e) => { e.stopPropagation(); setSelectedSource(source) }}
          >
            <Eye className="h-3.5 w-3.5" />
          </Button>
        )
      default:
        return String(rowData[columnKey] ?? '')
    }
  }

  const dataTable = (
    <ResponsiveTable
      columns={columns}
      data={pageData as unknown as Record<string, unknown>[]}
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

  const formatHistoryValue = (val: unknown): string => {
    if (val === null || val === undefined) return '-'
    if (typeof val === 'object') return JSON.stringify(val)
    return String(val)
  }

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
          totalCount > pageSize ? (
            <Pagination
              total={totalCount}
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

      <PageTabsBottomNav
        tabs={tabs}
        activeTab={activeType}
        onTabChange={(v) => setActiveType(v)}
      />

      <Dialog open={!!selectedSource} onOpenChange={(open) => !open && setSelectedSource(null)}>
        <DialogContent className="sm:max-w-2xl">
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
            <DialogContentBody className="space-y-4 py-4">
              {/* Current Value */}
              {selectedSource.current_value !== undefined && selectedSource.current_value !== null && (
                <div className="bg-muted/30 rounded-lg p-3">
                  <p className="text-xs text-muted-foreground mb-1">Current Value</p>
                  <p className="text-sm font-medium font-mono break-all overflow-hidden max-h-40">
                    {typeof selectedSource.current_value === 'object'
                      ? JSON.stringify(selectedSource.current_value, null, 2)
                      : String(selectedSource.current_value)}
                    {selectedSource.unit && <span className="font-normal text-muted-foreground ml-1">{selectedSource.unit}</span>}
                  </p>
                </div>
              )}

              {/* Metadata Grid */}
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <p className="text-xs text-muted-foreground">Field</p>
                  <p className="text-sm font-medium">{selectedSource.field_display_name}</p>
                </div>
                <div>
                  <p className="text-xs text-muted-foreground">Data Type</p>
                  <Badge variant="secondary" className="text-[10px] mt-0.5">{selectedSource.data_type}</Badge>
                </div>
                <div>
                  <p className="text-xs text-muted-foreground">Unit</p>
                  <p className="text-sm">{selectedSource.unit || '-'}</p>
                </div>
                <div>
                  <p className="text-xs text-muted-foreground">Last Update</p>
                  <p className="text-sm">{formatTime(selectedSource.last_update)}</p>
                </div>
              </div>

              {selectedSource.description && (
                <div>
                  <p className="text-xs text-muted-foreground">Description</p>
                  <p className="text-sm">{selectedSource.description}</p>
                </div>
              )}

              {/* History Section */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-1.5 text-sm font-medium">
                    <History className="h-3.5 w-3.5" />
                    History
                  </div>
                  <Select value={historyRange} onValueChange={setHistoryRange}>
                    <SelectTrigger className="w-[100px] h-7 text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="1h">1 Hour</SelectItem>
                      <SelectItem value="6h">6 Hours</SelectItem>
                      <SelectItem value="24h">24 Hours</SelectItem>
                      <SelectItem value="7d">7 Days</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                {historyLoading ? (
                  <div className="flex items-center justify-center h-24 text-muted-foreground">
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                    <span className="text-xs">Loading...</span>
                  </div>
                ) : historyData.length > 0 ? (
                  <ScrollArea className="h-[240px] rounded border">
                    <table className="w-full text-sm">
                      <thead className="sticky top-0 bg-background">
                        <tr className="border-b">
                          <th className="text-left text-xs font-medium text-muted-foreground px-3 py-1.5">Timestamp</th>
                          <th className="text-left text-xs font-medium text-muted-foreground px-3 py-1.5">Value</th>
                          <th className="text-left text-xs font-medium text-muted-foreground px-3 py-1.5">Quality</th>
                        </tr>
                      </thead>
                      <tbody>
                        {historyData.map((point, i) => (
                          <tr key={i} className="border-b last:border-0 hover:bg-muted/50">
                            <td className="px-3 py-1.5 font-mono text-xs">{formatDateTime(point.timestamp)}</td>
                            <td className="px-3 py-1.5 font-mono text-xs">{formatHistoryValue(point.value)}</td>
                            <td className="px-3 py-1.5 text-xs text-muted-foreground">
                              {point.quality !== null ? (point.quality * 100).toFixed(0) + '%' : '-'}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </ScrollArea>
                ) : (
                  <p className="text-xs text-muted-foreground text-center py-6">
                    No historical data available for this period
                  </p>
                )}
              </div>
            </DialogContentBody>
          )}
        </DialogContent>
      </Dialog>
    </>
  )
}
