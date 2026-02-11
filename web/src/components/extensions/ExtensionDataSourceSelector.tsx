/**
 * Extension Data Source Selector Component
 * Allows users to select extension-provided metrics as data sources for Dashboard widgets
 * Uses the unified Extension system
 */

import React, { useState, useEffect, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Puzzle,
  Search,
  Loader2,
  BarChart3,
  Info,
  ChevronDown,
  ChevronUp,
  Activity,
  Zap,
} from "lucide-react"
import type {
  Extension,
  ExtensionDataSource,
  ExtensionDataSourceInfo,
  ExtensionV2DataSourceInfo,
} from "@/types"

interface ExtensionDataSourceSelectorProps {
  /** Currently selected data source */
  dataSource: ExtensionDataSource | null
  /** Callback when selection changes */
  onChange: (dataSource: ExtensionDataSource | null) => void
  /** Filter by data type (optional) */
  dataTypeFilter?: string
  /** CSS class name */
  className?: string
}

/** Extension metric from the unified system */
interface ExtensionMetric {
  extension_id: string
  extension_name: string
  command: string
  field: string
  display_name: string
  data_type: string
  unit?: string
  description?: string
  /** Full data source ID */
  full_id?: string
}

interface GroupedMetrics {
  [extensionId: string]: {
    extensionName: string
    metrics: ExtensionMetric[]
  }
}

export function ExtensionDataSourceSelector({
  dataSource,
  onChange,
  dataTypeFilter,
  className,
}: ExtensionDataSourceSelectorProps) {
  const { t } = useTranslation('extensions')
  const [dataSources, setDataSources] = useState<ExtensionV2DataSourceInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")
  const [expandedExtensions, setExpandedExtensions] = useState<Set<string>>(new Set())
  const [selectedExtensionId, setSelectedExtensionId] = useState<string>("")
  const [selectedMetric, setSelectedMetric] = useState<string>("")

  // Fetch extension data sources
  useEffect(() => {
    const fetchDataSources = async () => {
      setLoading(true)
      setError(null)
      try {
        // Fetch unified extension data sources
        const allData = await api.listAllDataSources()
        // Filter only extension data sources (exclude transform data sources)
        const data = allData.filter((source): source is ExtensionDataSourceInfo => 'extension_id' in source)
        setDataSources(data)

        // Auto-expand if we have a selected data source
        if (dataSource) {
          setSelectedExtensionId(dataSource.extension_id)
          setSelectedMetric(dataSource.metric_name)
          setExpandedExtensions(new Set([dataSource.extension_id]))
        }
      } catch (err) {
        setError((err as Error).message)
      } finally {
        setLoading(false)
      }
    }

    fetchDataSources()
  }, [])

  // Extract metrics from data sources
  const extensionMetrics = useCallback((): ExtensionMetric[] => {
    return dataSources
      .filter(ds => !dataTypeFilter || ds.data_type === dataTypeFilter)
      .map(ds => ({
        extension_id: ds.extension_id,
        extension_name: ds.extension_id, // Use extension_id as name
        command: ds.command,
        field: ds.field,
        display_name: ds.display_name,
        data_type: ds.data_type,
        unit: ds.unit,
        description: ds.description,
        full_id: ds.id,
      }))
  }, [dataSources, dataTypeFilter])

  // Group metrics by extension
  const groupedMetrics = useCallback((): GroupedMetrics => {
    const metrics = extensionMetrics()
    const grouped: GroupedMetrics = {}

    // Filter by search query
    const filteredMetrics = searchQuery
      ? metrics.filter(metric => {
          const query = searchQuery.toLowerCase()
          const searchableFields = [
            metric.display_name,
            metric.description || '',
            metric.extension_name,
            metric.command,
            metric.field,
          ].join(' ')
          return searchableFields.toLowerCase().includes(query)
        })
      : metrics

    // Group by extension
    filteredMetrics.forEach(metric => {
      if (!grouped[metric.extension_id]) {
        grouped[metric.extension_id] = {
          extensionName: metric.extension_name,
          metrics: [],
        }
      }
      grouped[metric.extension_id].metrics.push(metric)
    })

    return grouped
  }, [extensionMetrics, searchQuery])

  // Handle extension selection
  const handleExtensionChange = (extensionId: string) => {
    setSelectedExtensionId(extensionId)
    setSelectedMetric("")
    setExpandedExtensions(new Set([extensionId]))
  }

  // Handle metric selection
  const handleMetricChange = (metricIdentifier: string) => {
    setSelectedMetric(metricIdentifier)

    if (!selectedExtensionId) return

    const metric = extensionMetrics().find(
      m => m.extension_id === selectedExtensionId &&
        `${m.command}:${m.field}` === metricIdentifier
    )

    if (metric) {
      onChange({
        extension_id: metric.extension_id,
        metric_name: `${metric.command}:${metric.field}`,
        display_name: metric.display_name,
        data_type: metric.data_type,
        unit: metric.unit,
      })
    }
  }

  // Get available metrics for selected extension
  const getMetricsForExtension = (extensionId: string): ExtensionMetric[] => {
    const grouped = groupedMetrics()
    return grouped[extensionId]?.metrics || []
  }

  // Get the unique identifier for a metric
  const getMetricIdentifier = (metric: ExtensionMetric): string => {
    return `${metric.command}:${metric.field}`
  }

  // Toggle extension expanded state
  const toggleExpanded = (extensionId: string) => {
    setExpandedExtensions(prev => {
      const newSet = new Set(prev)
      if (newSet.has(extensionId)) {
        newSet.delete(extensionId)
      } else {
        newSet.add(extensionId)
      }
      return newSet
    })
  }

  if (loading) {
    return (
      <div className={cn("flex items-center justify-center py-8", className)}>
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground mr-2" />
        <span className="text-sm text-muted-foreground">{t('loadingCapabilities')}</span>
      </div>
    )
  }

  if (error) {
    return (
      <div className={cn("text-center py-8", className)}>
        <Info className="h-8 w-8 mx-auto mb-2 text-destructive/50" />
        <p className="text-sm text-destructive">{t('errorLoadingCapabilities', { error })}</p>
      </div>
    )
  }

  const grouped = groupedMetrics()
  const extensionIds = Object.keys(grouped)

  return (
    <div className={cn("space-y-4", className)}>
      {/* Compact Mode - Single Select */}
      <div className="space-y-3">
        {/* Extension Select */}
        <div className="space-y-2">
          <Label className="text-sm">{t('dataSource.extensionSource')}</Label>
          <Select value={selectedExtensionId} onValueChange={handleExtensionChange}>
            <SelectTrigger className="w-full">
              <SelectValue placeholder={t('dataSource.selectExtension')} />
            </SelectTrigger>
            <SelectContent>
              {extensionIds.length === 0 ? (
                <div className="p-2 text-sm text-muted-foreground">
                  {t('dataSource.noProviderExtensions')}
                </div>
              ) : (
                extensionIds.map(extId => (
                  <SelectItem key={extId} value={extId}>
                    <div className="flex items-center gap-2">
                      <Puzzle className="h-4 w-4" />
                      <span>{grouped[extId].extensionName}</span>
                      <Badge variant="outline" className="text-xs">
                        {grouped[extId].metrics.length}
                      </Badge>
                    </div>
                  </SelectItem>
                ))
              )}
            </SelectContent>
          </Select>
        </div>

        {/* Metric Select */}
        {selectedExtensionId && (
          <div className="space-y-2">
            <Label className="text-sm">{t('dataSource.selectMetric')}</Label>
            <Select value={selectedMetric} onValueChange={handleMetricChange}>
              <SelectTrigger className="w-full">
                <SelectValue placeholder={t('dataSource.selectMetricPlaceholder')} />
              </SelectTrigger>
              <SelectContent>
                {getMetricsForExtension(selectedExtensionId).length === 0 ? (
                  <div className="p-2 text-sm text-muted-foreground">
                    {t('dataSource.noMetrics')}
                  </div>
                ) : (
                  getMetricsForExtension(selectedExtensionId).map(metric => {
                    const identifier = getMetricIdentifier(metric)
                    return (
                      <SelectItem key={identifier} value={identifier}>
                        <div className="flex items-center gap-2">
                          <BarChart3 className="h-4 w-4" />
                          <Zap className="h-3 w-3 text-yellow-500" />
                          <div className="flex-1">
                            <div className="font-medium">{metric.display_name}</div>
                            {metric.description && (
                              <div className="text-xs text-muted-foreground">
                                {metric.description}
                              </div>
                            )}
                          </div>
                          <div className="flex items-center gap-2">
                            {metric.unit && (
                              <span className="text-xs text-muted-foreground">
                                {metric.unit}
                              </span>
                            )}
                            <Badge variant="outline" className="text-xs">
                              {metric.data_type}
                            </Badge>
                          </div>
                        </div>
                      </SelectItem>
                    )
                  })
                )}
              </SelectContent>
            </Select>
          </div>
        )}
      </div>

      {/* Selected Summary */}
      {dataSource && (
        <div className="p-3 bg-green-50 dark:bg-green-950/30 border border-green-200 dark:border-green-800 rounded-lg">
          <div className="flex items-center gap-2 mb-2">
            <Activity className="h-4 w-4 text-green-600 dark:text-green-400" />
            <span className="font-medium text-sm text-green-700 dark:text-green-300">
              {t('dataSource.selectedSource')}
            </span>
          </div>
          <div className="text-xs text-green-600 dark:text-green-400 space-y-1">
            <div><span className="font-medium">{t('extension')}:</span> {dataSource.extension_id}</div>
            <div><span className="font-medium">{t('dataSource.metric')}:</span> {dataSource.display_name}</div>
            {dataSource.unit && (
              <div><span className="font-medium">{t('dataSource.unit')}:</span> {dataSource.unit}</div>
            )}
            <div><span className="font-medium">{t('dataSource.type')}:</span> {dataSource.data_type}</div>
          </div>
        </div>
      )}
    </div>
  )
}

/**
 * Extended view with all metrics visible (for browsing)
 */
export function ExtensionDataSourceBrowser({
  onSelect,
  className,
}: {
  onSelect?: (dataSource: ExtensionDataSource) => void
  className?: string
}) {
  const { t } = useTranslation('extensions')
  const [dataSources, setDataSources] = useState<ExtensionV2DataSourceInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [searchQuery, setSearchQuery] = useState("")
  const [expandedExtensions, setExpandedExtensions] = useState<Set<string>>(new Set())

  useEffect(() => {
    const fetchDataSources = async () => {
      setLoading(true)
      try {
        const allData = await api.listAllDataSources()
        // Filter only extension data sources (exclude transform data sources)
        const data = allData.filter((source): source is ExtensionDataSourceInfo => 'extension_id' in source)
        setDataSources(data)

        // Auto-expand all by default
        const allIds = [...new Set(data.map(ds => ds.extension_id))]
        setExpandedExtensions(new Set(allIds))
      } catch (err) {
        console.error('Error loading data sources:', err)
      } finally {
        setLoading(false)
      }
    }

    fetchDataSources()
  }, [])

  // Group metrics by extension
  const groupedMetrics = useCallback((): GroupedMetrics => {
    const grouped: GroupedMetrics = {}

    dataSources.forEach(ds => {
      const metricName = `${ds.command}:${ds.field}`
      if (!grouped[ds.extension_id]) {
        grouped[ds.extension_id] = {
          extensionName: ds.extension_id,
          metrics: [],
        }
      }
      grouped[ds.extension_id].metrics.push({
        extension_id: ds.extension_id,
        extension_name: ds.extension_id, // Use extension_id as name
        command: ds.command,
        field: ds.field,
        display_name: ds.display_name,
        data_type: ds.data_type,
        unit: ds.unit,
        description: ds.description,
        full_id: ds.id,
      })
    })

    return grouped
  }, [dataSources])

  const handleSelectMetric = (metric: ExtensionMetric) => {
    onSelect?.({
      extension_id: metric.extension_id,
      metric_name: `${metric.command}:${metric.field}`,
      display_name: metric.display_name,
      data_type: metric.data_type,
      unit: metric.unit,
    })
  }

  // Get the unique identifier for a metric
  const getMetricIdentifier = (metric: ExtensionMetric): string => {
    return `${metric.command}:${metric.field}`
  }

  // Toggle extension expanded state
  const toggleExpanded = (extensionId: string) => {
    setExpandedExtensions(prev => {
      const newSet = new Set(prev)
      if (newSet.has(extensionId)) {
        newSet.delete(extensionId)
      } else {
        newSet.add(extensionId)
      }
      return newSet
    })
  }

  if (loading) {
    return (
      <div className={cn("flex items-center justify-center py-8", className)}>
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground mr-2" />
        <span className="text-sm text-muted-foreground">{t('loadingCapabilities')}</span>
      </div>
    )
  }

  const grouped = groupedMetrics()
  const extensionIds = Object.keys(grouped)

  if (extensionIds.length === 0) {
    return (
      <div className={cn("text-center py-8 border-2 border-dashed rounded-lg", className)}>
        <Activity className="h-10 w-10 mx-auto mb-3 text-muted-foreground/50" />
        <p className="text-sm text-muted-foreground">
          {t('dataSource.noProviderExtensions')}
        </p>
      </div>
    )
  }

  return (
    <div className={cn("space-y-4", className)}>
      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder={t('dataSource.searchPlaceholder')}
          className="pl-9 h-9"
        />
      </div>

      {/* Extension Metric Groups */}
      <div className="space-y-2">
        {extensionIds.map((extensionId) => {
          const { extensionName, metrics } = grouped[extensionId]
          const isExpanded = expandedExtensions.has(extensionId)

          // Filter metrics by search
          const filteredMetrics = searchQuery
            ? metrics.filter(metric => {
                const query = searchQuery.toLowerCase()
                const searchableFields = [
                  metric.display_name,
                  metric.description || '',
                  metric.extension_name,
                  metric.command,
                  metric.field,
                ].join(' ')
                return searchableFields.toLowerCase().includes(query)
              })
            : metrics

          if (filteredMetrics.length === 0) return null

          return (
            <div key={extensionId} className="border rounded-lg overflow-hidden">
              {/* Extension Header */}
              <button
                type="button"
                onClick={() => toggleExpanded(extensionId)}
                className="w-full px-3 py-2.5 bg-muted/30 flex items-center justify-between hover:bg-muted/50 transition-colors"
              >
                <div className="flex items-center gap-2">
                  <div className="p-1.5 rounded-md bg-yellow-100 dark:bg-yellow-900 text-yellow-700 dark:text-yellow-300">
                    <Zap className="h-3.5 w-3.5" />
                  </div>
                  <span className="font-medium text-sm">{extensionName}</span>
                  <span className="text-xs text-muted-foreground">
                    ({filteredMetrics.length})
                  </span>
                </div>
                {isExpanded ? (
                  <ChevronUp className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <ChevronDown className="h-4 w-4 text-muted-foreground" />
                )}
              </button>

              {/* Metrics List */}
              {isExpanded && (
                <div className="p-3 border-t bg-background">
                  <div className="grid grid-cols-1 gap-2">
                    {filteredMetrics.map((metric) => {
                      const identifier = getMetricIdentifier(metric)
                      return (
                        <button
                          key={identifier}
                          type="button"
                          onClick={() => handleSelectMetric(metric)}
                          className="p-2.5 rounded-lg text-left transition-all text-sm hover:bg-yellow-50 dark:hover:bg-yellow-950/30 border border-transparent hover:border-yellow-200 dark:hover:border-yellow-800"
                        >
                          <div className="flex items-center gap-2">
                            <BarChart3 className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
                            <Zap className="h-3 w-3 text-yellow-500" />
                            <div className="flex-1 min-w-0">
                              <div className="font-medium truncate">{metric.display_name}</div>
                              {metric.description && (
                                <p className="text-xs text-muted-foreground truncate">
                                  {metric.description}
                                </p>
                              )}
                            </div>
                            <div className="flex items-center gap-2 flex-shrink-0">
                              {metric.unit && (
                                <span className="text-xs text-muted-foreground">
                                  {metric.unit}
                                </span>
                              )}
                              <Badge variant="outline" className="text-xs">
                                {metric.data_type}
                              </Badge>
                            </div>
                          </div>
                        </button>
                      )
                    })}
                  </div>
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
