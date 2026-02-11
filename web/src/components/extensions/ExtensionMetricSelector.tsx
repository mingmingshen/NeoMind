/**
 * Extension Metric Selector for AI Agents
 * Allows users to select extension output fields as data sources for Agent queries
 * Uses the unified Extension system
 */

import React, { useState, useEffect, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import {
  Puzzle,
  Search,
  Loader2,
  Check,
  Info,
  ChevronDown,
  ChevronUp,
  Database,
} from "lucide-react"
import type {
  Extension,
  ExtensionDataSourceInfo,
  ExtensionV2DataSourceInfo,
  TransformDataSourceInfo,
} from "@/types"

interface ExtensionMetricSelectorProps {
  /** Currently selected metrics in format "extension_id:command:field" */
  selectedMetrics: string[]
  /** Callback when selection changes */
  onSelectionChange: (metrics: string[]) => void
  /** Whether to show the header */
  showHeader?: boolean
  /** CSS class name */
  className?: string
}

interface ExtensionMetric {
  extension_id: string
  extension_name: string
  command: string
  command_display_name: string
  field: string
  display_name: string
  data_type: string
  unit?: string
  is_primary?: boolean
}

interface GroupedMetrics {
  [extensionId: string]: {
    extensionName: string
    metrics: ExtensionMetric[]
  }
}

export function ExtensionMetricSelector({
  selectedMetrics,
  onSelectionChange,
  showHeader = true,
  className,
}: ExtensionMetricSelectorProps) {
  const { t } = useTranslation('extensions')
  const [extensions, setExtensions] = useState<Extension[]>([])
  const [dataSources, setDataSources] = useState<ExtensionV2DataSourceInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")
  const [expandedExtensions, setExpandedExtensions] = useState<Set<string>>(new Set())

  // Fetch extensions and data sources
  useEffect(() => {
    const fetchData = async () => {
      setLoading(true)
      setError(null)
      try {
        // Fetch both extensions (for command info) and data sources
        const [extData, dsData] = await Promise.all([
          api.listExtensions(),
          api.listAllDataSources().catch((): (ExtensionDataSourceInfo | TransformDataSourceInfo)[] => []), // May fail if no extensions
        ])
        setExtensions(extData)
        // Filter only extension data sources (exclude transform data sources)
        setDataSources(dsData.filter((source): source is ExtensionDataSourceInfo => 'extension_id' in source))

        // Auto-expand extensions that have selected metrics
        const initiallyExpanded = new Set<string>()
        selectedMetrics.forEach(metricKey => {
          const [extensionId] = metricKey.split(':')
          initiallyExpanded.add(extensionId)
        })
        setExpandedExtensions(initiallyExpanded)
      } catch (err) {
        setError((err as Error).message)
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [])

  // Extract metrics from extensions (commands) and data sources
  const extensionMetrics = useCallback((): ExtensionMetric[] => {
    const metrics: ExtensionMetric[] = []

    // Process each extension
    extensions.forEach(ext => {
      if (!ext.commands || ext.commands.length === 0) return

      // Get command display names map
      const commandNames = new Map<string, string>()
      ext.commands.forEach(cmd => {
        commandNames.set(cmd.id, cmd.display_name)
      })

      // Find all data sources for this extension
      const extensionDataSources = dataSources.filter(ds => ds.extension_id === ext.id)

      // Group data sources by command
      const dataSourcesByCommand = new Map<string, ExtensionV2DataSourceInfo[]>()
      extensionDataSources.forEach(ds => {
        if (!dataSourcesByCommand.has(ds.command)) {
          dataSourcesByCommand.set(ds.command, [])
        }
        dataSourcesByCommand.get(ds.command)!.push(ds)
      })

      // For each command, create metric entries from its output fields
      ext.commands.forEach(cmd => {
        const cmdDataSources = dataSourcesByCommand.get(cmd.id) || []

        // If we have data sources from the API, use those
        if (cmdDataSources.length > 0) {
          cmdDataSources.forEach(ds => {
            metrics.push({
              extension_id: ext.id,
              extension_name: ext.name,
              command: ds.command,
              command_display_name: commandNames.get(ds.command) || ds.command,
              field: ds.field,
              display_name: ds.display_name,
              data_type: ds.data_type,
              unit: ds.unit,
              is_primary: ds.aggregatable,
            })
          })
        }
      })
    })

    return metrics
  }, [extensions, dataSources])

  // Format data type from enum to string
  const formatDataType = (dataType: string): string => {
    return dataType.toLowerCase()
  }

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
            metric.command_display_name,
            metric.field,
            metric.extension_name,
            metric.command,
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

  // Check if a metric is selected
  const isMetricSelected = (extensionId: string, command: string, field: string): boolean => {
    const key = `${extensionId}:${command}:${field}`
    return selectedMetrics.includes(key)
  }

  // Toggle metric selection
  const toggleMetric = (extensionId: string, command: string, field: string) => {
    const key = `${extensionId}:${command}:${field}`
    if (selectedMetrics.includes(key)) {
      onSelectionChange(selectedMetrics.filter(m => m !== key))
    } else {
      onSelectionChange([...selectedMetrics, key])
    }
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

  // Select all metrics from an extension
  const selectAllFromExtension = (extensionId: string) => {
    const extension = groupedMetrics()[extensionId]
    if (!extension) return

    const newKeys = extension.metrics.map(m => `${extensionId}:${m.command}:${m.field}`)
    const filtered = newKeys.filter(key => !selectedMetrics.includes(key))
    onSelectionChange([...selectedMetrics, ...filtered])
  }

  // Clear all metrics from an extension
  const clearExtension = (extensionId: string) => {
    const newSelection = selectedMetrics.filter(m => !m.startsWith(`${extensionId}:`))
    onSelectionChange(newSelection)
  }

  // Get count of selected metrics per extension
  const getSelectedCount = (extensionId: string): number => {
    return selectedMetrics.filter(m => m.startsWith(`${extensionId}:`)).length
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

  if (extensionIds.length === 0) {
    return (
      <div className={cn("text-center py-8 border-2 border-dashed rounded-lg", className)}>
        <Database className="h-10 w-10 mx-auto mb-3 text-muted-foreground/50" />
        <p className="text-sm text-muted-foreground">
          {searchQuery ? t('noToolsFound') : t('noExtensionMetrics', { defaultValue: 'No extension metrics available' })}
        </p>
      </div>
    )
  }

  return (
    <div className={cn("space-y-4", className)}>
      {showHeader && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-medium flex items-center gap-2">
              <Database className="h-4 w-4" />
              {t('extensionMetrics', { defaultValue: 'Extension Metrics' })}
            </h3>
            {selectedMetrics.length > 0 && (
              <Badge variant="secondary">
                {t('selectedCount', { count: selectedMetrics.length })}
              </Badge>
            )}
          </div>

          {/* Search */}
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('searchMetrics', { defaultValue: 'Search metrics...' })}
              className="pl-9 h-9"
            />
          </div>
        </div>
      )}

      {/* Extension Metric Groups */}
      <div className="space-y-2">
        {extensionIds.map((extensionId) => {
          const { extensionName, metrics } = grouped[extensionId]
          const isExpanded = expandedExtensions.has(extensionId)
          const selectedCount = getSelectedCount(extensionId)
          const allSelected = selectedCount === metrics.length

          return (
            <div
              key={extensionId}
              className="border rounded-lg overflow-hidden"
            >
              {/* Extension Header */}
              <button
                type="button"
                onClick={() => toggleExpanded(extensionId)}
                className="w-full px-3 py-2.5 bg-muted/30 flex items-center justify-between hover:bg-muted/50 transition-colors"
              >
                <div className="flex items-center gap-2">
                  <div className={cn(
                    "p-1.5 rounded-md transition-colors",
                    selectedCount > 0 ? "bg-green-600 text-white" : "bg-muted"
                  )}>
                    <Database className="h-3.5 w-3.5" />
                  </div>
                  <span className="font-medium text-sm">{extensionName}</span>
                  <span className="text-xs text-muted-foreground">
                    ({selectedCount}/{metrics.length})
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  {selectedCount > 0 && (
                    <Badge variant="secondary" className="text-xs h-5">
                      {selectedCount}
                    </Badge>
                  )}
                  {isExpanded ? (
                    <ChevronUp className="h-4 w-4 text-muted-foreground" />
                  ) : (
                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                  )}
                </div>
              </button>

              {/* Metrics List */}
              {isExpanded && (
                <div className="p-3 border-t bg-background">
                  {/* Action Buttons */}
                  <div className="flex items-center gap-2 mb-3 pb-2 border-b">
                    <button
                      type="button"
                      onClick={() => selectAllFromExtension(extensionId)}
                      disabled={allSelected}
                      className="text-xs px-2 py-1 rounded hover:bg-muted transition-colors disabled:opacity-50 text-primary"
                    >
                      {t('selectAll')}
                    </button>
                    <button
                      type="button"
                      onClick={() => clearExtension(extensionId)}
                      disabled={selectedCount === 0}
                      className="text-xs px-2 py-1 rounded hover:bg-muted transition-colors disabled:opacity-50 text-muted-foreground"
                    >
                      {t('clear')}
                    </button>
                  </div>

                  {/* Metric Grid - grouped by command */}
                  <div className="space-y-3">
                    {(() => {
                      // Group metrics by command
                      const byCommand = new Map<string, ExtensionMetric[]>()
                      metrics.forEach(m => {
                        if (!byCommand.has(m.command)) {
                          byCommand.set(m.command, [])
                        }
                        byCommand.get(m.command)!.push(m)
                      })

                      return Array.from(byCommand.entries()).map(([command, cmdMetrics]) => (
                        <div key={command} className="space-y-1">
                          <div className="text-xs font-medium text-muted-foreground px-1">
                            {cmdMetrics[0].command_display_name}
                          </div>
                          <div className="grid grid-cols-1 gap-1.5">
                            {cmdMetrics.map((metric) => {
                              const isSelected = isMetricSelected(extensionId, metric.command, metric.field)
                              return (
                                <button
                                  key={`${metric.command}:${metric.field}`}
                                  type="button"
                                  onClick={() => toggleMetric(extensionId, metric.command, metric.field)}
                                  className={cn(
                                    "p-2 rounded-lg text-left transition-all text-sm",
                                    isSelected
                                      ? "bg-green-600 dark:bg-green-700 text-white"
                                      : "hover:bg-green-50 dark:hover:bg-green-950/30 border border-green-200 dark:border-green-800"
                                  )}
                                >
                                  <div className="flex items-center gap-2">
                                    <div className="flex-shrink-0">
                                      {isSelected ? (
                                        <Check className="h-3 w-3" />
                                      ) : (
                                        <div className="w-3 h-3 rounded border border-green-300 dark:border-green-700" />
                                      )}
                                    </div>
                                    <div className="flex-1 min-w-0">
                                      <div className="font-medium truncate">{metric.field}</div>
                                      <div className={cn(
                                        "text-xs truncate",
                                        isSelected ? "text-green-100" : "text-muted-foreground"
                                      )}>
                                        {metric.display_name}
                                      </div>
                                    </div>
                                    <div className="flex items-center gap-1 flex-shrink-0">
                                      {metric.is_primary && (
                                        <span className="text-xs">★</span>
                                      )}
                                      {metric.unit && (
                                        <span className="text-xs opacity-70">{metric.unit}</span>
                                      )}
                                      <Badge variant="outline" className={cn(
                                        "text-xs",
                                        isSelected ? "border-green-400" : ""
                                      )}>
                                        {metric.data_type}
                                      </Badge>
                                    </div>
                                  </div>
                                </button>
                              )
                            })}
                          </div>
                        </div>
                      ))
                    })()}
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

/**
 * Compact version of ExtensionMetricSelector for use in summaries
 */
export function ExtensionMetricSelectorCompact({
  selectedMetrics,
  maxDisplay = 3,
}: {
  selectedMetrics: string[]
  maxDisplay?: number
}) {
  const { t } = useTranslation('extensions')
  const [showAll, setShowAll] = useState(false)

  if (selectedMetrics.length === 0) {
    return (
      <div className="text-xs text-muted-foreground italic">
        {t('noMetricsSelected', { defaultValue: 'No metrics selected' })}
      </div>
    )
  }

  const displayedMetrics = showAll ? selectedMetrics : selectedMetrics.slice(0, maxDisplay)

  return (
    <div className="space-y-1">
      {displayedMetrics.map((metricKey) => {
        const [extensionId, command, field] = metricKey.split(':')
        return (
          <div
            key={metricKey}
            className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-green-50 dark:bg-green-950/30 border border-green-200 dark:border-green-800 text-green-700 dark:text-green-400 text-xs mr-1 mb-1"
          >
            <Database className="h-3 w-3" />
            <span className="truncate max-w-[150px]">{field}</span>
            <span className="text-xs opacity-70">({command})</span>
          </div>
        )
      })}
      {selectedMetrics.length > maxDisplay && !showAll && (
        <button
          type="button"
          onClick={() => setShowAll(true)}
          className="text-xs text-primary hover:underline"
        >
          +{selectedMetrics.length - maxDisplay} {t('more')}
        </button>
      )}
    </div>
  )
}
