/**
 * Extension Tool Selector
 * Allows users to select tools (commands) provided by extensions for AI Agents
 * Uses the unified Extension system
 */

import React, { useState, useEffect, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  Puzzle,
  Search,
  Loader2,
  Check,
  Zap,
  Info,
  ChevronDown,
  ChevronUp,
} from "lucide-react"
import type {
  Extension,
  ExtensionCommandDescriptor,
} from "@/types"

interface ExtensionToolSelectorProps {
  /** Currently selected tools in format "extension_id:command_id" */
  selectedTools: string[]
  /** Callback when selection changes */
  onSelectionChange: (tools: string[]) => void
  /** Whether to show the header */
  showHeader?: boolean
  /** CSS class name */
  className?: string
}

interface GroupedTools {
  [extensionId: string]: {
    extensionName: string
    tools: ExtensionCommandDescriptor[]
  }
}

export function ExtensionToolSelector({
  selectedTools,
  onSelectionChange,
  showHeader = true,
  className,
}: ExtensionToolSelectorProps) {
  const { t } = useTranslation('extensions')
  const [extensions, setExtensions] = useState<Extension[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")
  const [expandedExtensions, setExpandedExtensions] = useState<Set<string>>(new Set())

  // Fetch extensions
  useEffect(() => {
    const fetchTools = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = await api.listExtensions()
        setExtensions(data)

        // Auto-expand extensions that have selected tools
        const initiallyExpanded = new Set<string>()
        data.forEach((ext: Extension) => {
          if (ext.commands && ext.commands.length > 0) {
            const hasSelected = ext.commands.some((cmd: ExtensionCommandDescriptor) =>
              selectedTools.includes(`${ext.id}:${cmd.id}`)
            )
            if (hasSelected) {
              initiallyExpanded.add(ext.id)
            }
          }
        })

        setExpandedExtensions(initiallyExpanded)
      } catch (err) {
        setError((err as Error).message)
      } finally {
        setLoading(false)
      }
    }

    fetchTools()
  }, [])

  // Group tools by extension
  const groupedTools = useCallback((): GroupedTools => {
    const grouped: GroupedTools = {}

    // Filter extensions by search query
    const filteredExtensions = searchQuery
      ? extensions.filter(ext => {
          const query = searchQuery.toLowerCase()
          const searchableFields = [
            ext.name,
            ext.id,
            ext.description || '',
            ext.commands?.map(c => c.display_name).join(' ') || '',
            ext.commands?.map(c => c.description).join(' ') || '',
          ].join(' ')
          return searchableFields.toLowerCase().includes(query)
        })
      : extensions

    // Group by extension
    filteredExtensions.forEach(ext => {
      if (ext.commands && ext.commands.length > 0) {
        grouped[ext.id] = {
          extensionName: ext.name,
          tools: ext.commands,
        }
      }
    })

    return grouped
  }, [extensions, searchQuery])

  // Check if a tool is selected
  const isToolSelected = (extensionId: string, commandId: string): boolean => {
    return selectedTools.includes(`${extensionId}:${commandId}`)
  }

  // Toggle tool selection
  const toggleTool = (extensionId: string, commandId: string) => {
    const key = `${extensionId}:${commandId}`
    if (selectedTools.includes(key)) {
      onSelectionChange(selectedTools.filter(t => t !== key))
    } else {
      onSelectionChange([...selectedTools, key])
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

  // Select all tools from an extension
  const selectAllFromExtension = (extensionId: string) => {
    const extension = groupedTools()[extensionId]
    if (!extension) return

    const newKeys = extension.tools.map(cmd => `${extensionId}:${cmd.id}`)
    const filtered = newKeys.filter(key => !selectedTools.includes(key))
    onSelectionChange([...selectedTools, ...filtered])
  }

  // Clear all tools from an extension
  const clearExtension = (extensionId: string) => {
    const newSelection = selectedTools.filter(t => !t.startsWith(`${extensionId}:`))
    onSelectionChange(newSelection)
  }

  // Get count of selected tools per extension
  const getSelectedCount = (extensionId: string): number => {
    return selectedTools.filter(t => t.startsWith(`${extensionId}:`)).length
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
        <Button
          variant="outline"
          size="sm"
          onClick={() => window.location.reload()}
          className="mt-3"
        >
          {t('retry')}
        </Button>
      </div>
    )
  }

  const grouped = groupedTools()
  const extensionIds = Object.keys(grouped)

  if (extensionIds.length === 0) {
    return (
      <div className={cn("text-center py-8 border-2 border-dashed rounded-lg", className)}>
        <Puzzle className="h-10 w-10 mx-auto mb-3 text-muted-foreground/50" />
        <p className="text-sm text-muted-foreground">
          {searchQuery ? t('noToolsFound') : t('noExtensionTools')}
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
              <Puzzle className="h-4 w-4" />
              {t('extensionTools')}
            </h3>
            {selectedTools.length > 0 && (
              <Badge variant="secondary">
                {t('selectedCount', { count: selectedTools.length })}
              </Badge>
            )}
          </div>

          {/* Search */}
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('searchTools')}
              className="pl-9 h-9"
            />
          </div>
        </div>
      )}

      {/* Extension Tool Groups */}
      <div className="space-y-2">
        {extensionIds.map((extensionId) => {
          const { extensionName, tools } = grouped[extensionId]
          const isExpanded = expandedExtensions.has(extensionId)
          const selectedCount = getSelectedCount(extensionId)
          const allSelected = selectedCount === tools.length

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
                    selectedCount > 0 ? "bg-primary text-primary-foreground" : "bg-muted"
                  )}>
                    <Puzzle className="h-3.5 w-3.5" />
                  </div>
                  <span className="font-medium text-sm">{extensionName}</span>
                  <span className="text-xs text-muted-foreground">
                    ({selectedCount}/{tools.length})
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

              {/* Tools List */}
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

                  {/* Tool Grid */}
                  <div className="grid grid-cols-1 gap-2">
                    {tools.map((cmd) => {
                      const isSelected = isToolSelected(extensionId, cmd.id)
                      return (
                        <button
                          key={cmd.id}
                          type="button"
                          onClick={() => toggleTool(extensionId, cmd.id)}
                          className={cn(
                            "p-2.5 rounded-lg text-left transition-all",
                            isSelected
                              ? "bg-purple-600 dark:bg-purple-700 text-white"
                              : "hover:bg-purple-50 dark:hover:bg-purple-950/30 border border-purple-200 dark:border-purple-800"
                          )}
                        >
                          <div className="flex items-start gap-2">
                            <div className="mt-0.5">
                              {isSelected ? (
                                <Check className="h-3.5 w-3.5 flex-shrink-0" />
                              ) : (
                                <div className="w-3.5 h-3.5 rounded border border-purple-300 dark:border-purple-700 flex-shrink-0" />
                              )}
                            </div>
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-1.5 mb-0.5">
                                <Zap className={cn(
                                  "h-3 w-3",
                                  isSelected ? "text-white" : "text-purple-500"
                                )} />
                                <span className="font-medium text-sm truncate">
                                  {cmd.display_name}
                                </span>
                              </div>
                              <p className={cn(
                                "text-xs truncate",
                                isSelected ? "text-purple-100" : "text-muted-foreground"
                              )}>
                                {cmd.description}
                              </p>
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

/**
 * Compact version of ExtensionToolSelector for use in dialogs/popovers
 */
export function ExtensionToolSelectorCompact({
  selectedTools,
  onSelectionChange,
  maxDisplay = 3,
}: {
  selectedTools: string[]
  onSelectionChange: (tools: string[]) => void
  maxDisplay?: number
}) {
  const { t } = useTranslation('extensions')
  const [showAll, setShowAll] = useState(false)

  if (selectedTools.length === 0) {
    return (
      <div className="text-xs text-muted-foreground italic">
        {t('noToolsSelected')}
      </div>
    )
  }

  const displayedTools = showAll ? selectedTools : selectedTools.slice(0, maxDisplay)

  return (
    <div className="space-y-1.5">
      {displayedTools.map((toolKey) => {
        const [extensionId, commandId] = toolKey.split(':')
        return (
          <div
            key={toolKey}
            className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-purple-50 dark:bg-purple-950/30 border border-purple-200 dark:border-purple-800 text-purple-700 dark:text-purple-400 text-xs"
          >
            <Zap className="h-3 w-3" />
            <span className="truncate max-w-[200px]">{commandId}</span>
            <button
              type="button"
              onClick={() => onSelectionChange(selectedTools.filter(t => t !== toolKey))}
              className="p-0.5 rounded-sm hover:bg-purple-200 dark:hover:bg-purple-800 transition-colors"
            >
              ×
            </button>
          </div>
        )
      })}
      {selectedTools.length > maxDisplay && !showAll && (
        <button
          type="button"
          onClick={() => setShowAll(true)}
          className="text-xs text-primary hover:underline"
        >
          +{selectedTools.length - maxDisplay} {t('more')}
        </button>
      )}
    </div>
  )
}
