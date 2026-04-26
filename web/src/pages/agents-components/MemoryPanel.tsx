/**
 * Memory Panel
 *
 * Displays Markdown-based memory files organized by category.
 * Uses table layout similar to device list for consistency.
 * Includes configuration UI for extraction, compression, and LLM settings.
 */

import { useState, useEffect, useCallback, forwardRef, useImperativeHandle } from "react"
import { useTranslation } from "react-i18next"
import ReactMarkdown from "react-markdown"
import CodeMirror from "@uiw/react-codemirror"
import {
  Eye,
  Pencil,
  Download,
  Loader2,
  User,
  BookOpen,
  Repeat2,
  Cpu,
  Clock,
  Hash,
  Save,
  X,
  Settings,
  Sparkles,
  Archive,
  Zap,
  Play,
  Brain,
  Wrench,
} from "lucide-react"
import { ResponsiveTable } from "@/components/shared"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Skeleton } from "@/components/ui/skeleton"
import { Switch } from "@/components/ui/switch"
import { Slider } from "@/components/ui/slider"
import { Label } from "@/components/ui/label"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  FullScreenDialog,
  FullScreenDialogHeader,
  FullScreenDialogContent,
  FullScreenDialogFooter,
  FullScreenDialogMain,
} from "@/components/automation/dialog"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { useToast } from "@/hooks/use-toast"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import { formatTimestamp } from "@/lib/utils/format"
import type { LlmBackendInstance, MemorySystemConfig } from "@/types"

// Default config for initialization
const defaultConfig: MemorySystemConfig = {
  enabled: true,
  storage_path: "data/memory",
  extraction: {
    similarity_threshold: 0.85,
    min_messages: 3,
    max_messages: 50,
    min_importance: 30,
    dedup_enabled: true,
  },
  compression: {
    decay_period_days: 30,
    min_importance: 20,
    max_entries: {
      user_profile: 50,
      domain_knowledge: 100,
      task_patterns: 80,
      system_evolution: 30,
    },
  },
  llm: {
    extraction_backend_id: undefined,
    compression_backend_id: undefined,
  },
  schedule: {
    extraction_enabled: true,
    extraction_interval_secs: 3600,
    compression_enabled: true,
    compression_interval_secs: 86400,
  },
}

// Memory categories configuration
const categoryConfig = [
  {
    id: "user_profile",
    labelKey: "systemMemory.categories.userProfile",
    defaultLabel: "User Profile",
    icon: User,
    description: "User preferences, habits and settings",
    color: "bg-blue-500/10 text-blue-600 border-blue-500/20",
  },
  {
    id: "domain_knowledge",
    labelKey: "systemMemory.categories.domainKnowledge",
    defaultLabel: "Domain Knowledge",
    icon: BookOpen,
    description: "Devices, protocols and system knowledge",
    color: "bg-green-500/10 text-green-600 border-green-500/20",
  },
  {
    id: "task_patterns",
    labelKey: "memory.categories.taskPatterns",
    defaultLabel: "Task Patterns",
    icon: Repeat2,
    description: "Common tasks and operation patterns",
    color: "bg-purple-500/10 text-purple-600 border-purple-500/20",
  },
  {
    id: "system_evolution",
    labelKey: "memory.categories.systemEvolution",
    defaultLabel: "System Evolution",
    icon: Cpu,
    description: "System changes and optimization records",
    color: "bg-orange-500/10 text-orange-600 border-orange-500/20",
  },
]

// Category stats from API
interface CategoryStats {
  entry_count: number
  file_size: number
  modified_at: number
}

// Memory content response from API
interface MemoryContentResponse {
  category: string
  content: string
  stats: CategoryStats
}

// Memory stats response from API
interface MemoryStatsResponse {
  categories: Record<string, CategoryStats>
}

// Table row data type
interface MemoryCategoryRow {
  id: string
  name: string
  description: string
  icon: React.ElementType
  color: string
  entry_count: number
  file_size: number
  modified_at: number
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
}

interface MemoryPanelProps {
  refreshKey?: number
}

export interface MemoryPanelRef {
  openConfig: () => void
  triggerExtract: () => void
  isExtracting: boolean
}

export const MemoryPanel = forwardRef<MemoryPanelRef, MemoryPanelProps>(function MemoryPanel({ refreshKey }, ref) {
  const { t } = useTranslation("agents")
  const { handleError } = useErrorHandler()
  const { toast } = useToast()

  // State
  const [stats, setStats] = useState<Record<string, CategoryStats>>({})
  const [loading, setLoading] = useState(true)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null)
  const [content, setContent] = useState("")
  const [contentLoading, setContentLoading] = useState(false)
  const [editing, setEditing] = useState(false)
  const [editContent, setEditContent] = useState("")
  const [saving, setSaving] = useState(false)
  const [exporting, setExporting] = useState<string | null>(null)

  // Configuration state
  const [configOpen, setConfigOpen] = useState(false)
  const [config, setConfig] = useState<MemorySystemConfig>(defaultConfig)
  const [configLoading, setConfigLoading] = useState(false)
  const [configSaving, setConfigSaving] = useState(false)
  const [llmBackends, setLlmBackends] = useState<LlmBackendInstance[]>([])
  const [extracting, setExtracting] = useState(false)

  // Load configuration
  const loadConfig = useCallback(async () => {
    setConfigLoading(true)
    try {
      const response = await api.getMemoryConfig()
      // Merge with defaults to ensure all fields exist
      setConfig({ ...defaultConfig, ...response })
    } catch (error) {
      handleError(error, { operation: "Load memory config", showToast: false })
    } finally {
      setConfigLoading(false)
    }
  }, [handleError])

  // Load LLM backends for dropdown
  const loadLlmBackends = useCallback(async () => {
    try {
      // Load all backends (not just active) so user can select any configured model
      const response = await api.listLlmBackends({})
      setLlmBackends(response.backends || [])
    } catch (error) {
      handleError(error, { operation: "Load LLM backends", showToast: false })
    }
  }, [handleError])

  // Load config on mount
  useEffect(() => {
    loadConfig()
    loadLlmBackends()
  }, [loadConfig, loadLlmBackends])

  // Save configuration
  const handleSaveConfig = async () => {
    setConfigSaving(true)
    try {
      await api.updateMemoryConfig(config)
      setConfigOpen(false)
    } catch (error) {
      handleError(error, { operation: "Save memory config" })
    } finally {
      setConfigSaving(false)
    }
  }

  // Trigger manual extraction
  const handleExtract = async () => {
    setExtracting(true)
    toast({
      title: t("systemMemory.extractStarted", "Extraction Started"),
      description: t("systemMemory.extractStartedDesc", "Extracting memories from conversations..."),
    })
    try {
      const result = await api.triggerMemoryExtract()
      toast({
        title: t("systemMemory.extractStarted", "Extraction Started"),
        description: result.message || t("systemMemory.extractStartedDesc", "Extraction is running in the background. Check stats after a moment."),
      })
      loadStats() // Refresh stats immediately
      // Schedule a delayed refresh for when background extraction finishes
      setTimeout(() => loadStats(), 30000)
    } catch (error) {
      console.error('[MemoryPanel] Extraction error:', error)
      // Check for specific error types
      if (error instanceof Error) {
        if (error.name === 'AbortError' || error.message.includes('abort')) {
          handleError(error, {
            operation: "Trigger memory extraction",
            userMessage: "Request timed out. The extraction is taking too long - please try again later."
          })
        } else if (error.message.includes('Load failed') || error.message.includes('Failed to fetch')) {
          handleError(error, {
            operation: "Trigger memory extraction",
            userMessage: "Could not connect to the server. Please ensure the backend is running."
          })
        } else {
          handleError(error, { operation: "Trigger memory extraction" })
        }
      } else {
        handleError(error, { operation: "Trigger memory extraction" })
      }
    } finally {
      setExtracting(false)
    }
  }

  // Load stats
  const loadStats = useCallback(async () => {
    setLoading(true)
    try {
      const response: MemoryStatsResponse = await api.getMemoryStats()
      setStats(response.categories || {})
    } catch (error) {
      handleError(error, { operation: "Load stats", showToast: false })
    } finally {
      setLoading(false)
    }
  }, [handleError])

  useEffect(() => {
    loadStats()
  }, [loadStats, refreshKey])

  // Load content for a category
  const loadContent = async (categoryId: string) => {
    setContentLoading(true)
    try {
      const response: MemoryContentResponse = await api.getMemoryCategory(categoryId)
      setContent(response.content || "")
      setEditContent(response.content || "")
    } catch (error) {
      handleError(error, { operation: "Load memory content" })
      setContent("")
      setEditContent("")
    } finally {
      setContentLoading(false)
    }
  }

  // Handle view/edit action
  const handleViewEdit = (categoryId: string) => {
    setSelectedCategory(categoryId)
    setEditing(false)
    setDialogOpen(true)
    loadContent(categoryId)
  }

  // Handle save
  const handleSave = async () => {
    if (!selectedCategory) return
    setSaving(true)
    try {
      await api.updateMemoryCategory(selectedCategory, editContent)
      setContent(editContent)
      setEditing(false)
      loadStats()
    } catch (error) {
      handleError(error, { operation: "Save memory" })
    } finally {
      setSaving(false)
    }
  }

  // Handle export
  const handleExport = async (categoryId: string) => {
    setExporting(categoryId)
    try {
      const markdown = await api.exportAllMemory()
      const blob = new Blob([markdown], { type: "text/markdown" })
      const url = URL.createObjectURL(blob)
      const a = document.createElement("a")
      a.href = url
      a.download = `memory_${categoryId}_${new Date().toISOString().split("T")[0]}.md`
      a.click()
      URL.revokeObjectURL(url)
    } catch (error) {
      handleError(error, { operation: "Export memory" })
    } finally {
      setExporting(null)
    }
  }

  // Handle dialog close
  const handleDialogClose = (open: boolean) => {
    setDialogOpen(open)
    if (!open) {
      setEditing(false)
    }
  }

  // Expose methods via ref
  useImperativeHandle(ref, () => ({
    openConfig: () => {
      setConfigOpen(true)
      loadConfig() // Refresh config when opening
      loadLlmBackends() // Refresh backends when opening config
    },
    triggerExtract: handleExtract,
    isExtracting: extracting,
  }), [handleExtract, loadConfig, loadLlmBackends, extracting])

  // Prepare table data
  const tableData: MemoryCategoryRow[] = categoryConfig.map((cat) => ({
    id: cat.id,
    name: t(cat.labelKey, cat.defaultLabel),
    description: cat.description,
    icon: cat.icon,
    color: cat.color,
    entry_count: stats[cat.id]?.entry_count ?? 0,
    file_size: stats[cat.id]?.file_size ?? 0,
    modified_at: stats[cat.id]?.modified_at ?? 0,
  }))

  // Get category config by id
  const getSelectedCategoryConfig = () => {
    return categoryConfig.find((c) => c.id === selectedCategory)
  }

  // Check for dark mode
  const isDark =
    typeof document !== "undefined" &&
    (document.documentElement.getAttribute("data-theme") === "dark" ||
      document.documentElement.classList.contains("dark"))

  return (
    <div className="space-y-4">
      {/* Category table */}
      <ResponsiveTable
        columns={[
          {
            key: "name",
            label: t("systemMemory.headers.category", "Category"),
          },
          {
            key: "entry_count",
            label: (
              <div className="flex items-center gap-1">
                <Hash className="h-4 w-4" />
                {t("systemMemory.headers.entries", "Entries")}
              </div>
            ),
            align: "center",
            width: "w-24",
          },
          {
            key: "file_size",
            label: t("systemMemory.headers.size", "Size"),
            align: "right",
            width: "w-20",
          },
          {
            key: "modified_at",
            label: (
              <div className="flex items-center gap-1">
                <Clock className="h-4 w-4" />
                {t("systemMemory.headers.modified", "Modified")}
              </div>
            ),
            align: "right",
            width: "w-32",
          },
        ]}
        data={tableData as unknown as Record<string, unknown>[]}
        rowKey={(row) => (row as unknown as MemoryCategoryRow).id}
        loading={loading}
        renderCell={(columnKey, rowData) => {
          const row = rowData as unknown as MemoryCategoryRow
          const Icon = row.icon

          switch (columnKey) {
            case "name":
              return (
                <div className="flex items-center gap-3">
                  <div
                    className={cn(
                      "w-9 h-9 rounded-lg flex items-center justify-center border",
                      row.color
                    )}
                  >
                    <Icon className="h-4 w-4" />
                  </div>
                  <div>
                    <div className="font-medium text-sm">{row.name}</div>
                    <div className="text-xs text-muted-foreground">
                      {row.description}
                    </div>
                  </div>
                </div>
              )

            case "entry_count":
              return (
                <Badge variant="secondary" className="font-mono">
                  {row.entry_count}
                </Badge>
              )

            case "file_size":
              return (
                <span className="text-xs text-muted-foreground font-mono">
                  {formatBytes(row.file_size)}
                </span>
              )

            case "modified_at":
              return (
                <span className="text-xs text-muted-foreground">
                  {row.modified_at > 0
                    ? formatTimestamp(row.modified_at, false)
                    : "-"}
                </span>
              )

            default:
              return null
          }
        }}
        actions={[
          {
            label: t("systemMemory.viewEdit", "View/Edit"),
            icon: <Eye className="h-4 w-4" />,
            onClick: (rowData) => {
              const row = rowData as unknown as MemoryCategoryRow
              handleViewEdit(row.id)
            },
          },
          {
            label: t("systemMemory.export", "Export"),
            icon: exporting ? <Loader2 className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4" />,
            onClick: (rowData) => {
              const row = rowData as unknown as MemoryCategoryRow
              handleExport(row.id)
            },
          },
        ]}
      />

      {/* Full Screen Dialog for View/Edit */}
      <FullScreenDialog open={dialogOpen} onOpenChange={handleDialogClose}>
        <FullScreenDialogHeader
          icon={
            (() => {
              const config = getSelectedCategoryConfig()
              if (!config) return null
              const Icon = config.icon
              return <Icon className="h-5 w-5" />
            })()
          }
          iconBg={getSelectedCategoryConfig()?.color || "bg-muted"}
          title={getSelectedCategoryConfig() ? t(getSelectedCategoryConfig()!.labelKey, getSelectedCategoryConfig()!.defaultLabel) : ""}
          onClose={() => handleDialogClose(false)}
        />

        <FullScreenDialogContent className="flex-col">
          {contentLoading ? (
            <div className="flex items-center justify-center h-full">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : editing ? (
            <div className="w-full h-full overflow-hidden">
              <CodeMirror
                value={editContent}
                height="100%"
                onChange={(value) => setEditContent(value)}
                theme={isDark ? "dark" : "light"}
                style={{
                  fontSize: "14px",
                  fontFamily:
                    'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace',
                  height: "100%",
                  width: "100%",
                }}
              />
            </div>
          ) : (
            <div className="prose prose-sm dark:prose-invert max-w-none p-6 overflow-auto h-full w-full">
              {content ? (
                <ReactMarkdown>{content}</ReactMarkdown>
              ) : (
                <p className="text-muted-foreground italic">
                  {t("systemMemory.empty", "No content yet")}
                </p>
              )}
            </div>
          )}
        </FullScreenDialogContent>

        <FullScreenDialogFooter>
          <div className="flex items-center justify-between w-full">
            <div className="text-xs text-muted-foreground">
              {content.split("\n").filter(l => l.trim()).length} {t("systemMemory.lines", "lines")}
            </div>
            <div className="flex items-center gap-2">
              {editing ? (
                <>
                  <Button
                    variant="outline"
                    onClick={() => {
                      setEditContent(content)
                      setEditing(false)
                    }}
                    disabled={saving}
                  >
                    <X className="h-4 w-4 mr-1" />
                    {t("systemMemory.cancel", "Cancel")}
                  </Button>
                  <Button onClick={handleSave} disabled={saving}>
                    {saving ? (
                      <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                    ) : (
                      <Save className="h-4 w-4 mr-1" />
                    )}
                    {saving ? t("systemMemory.saving", "Saving...") : t("systemMemory.save", "Save")}
                  </Button>
                </>
              ) : (
                <Button onClick={() => setEditing(true)}>
                  <Pencil className="h-4 w-4 mr-1" />
                  {t("systemMemory.edit", "Edit")}
                </Button>
              )}
            </div>
          </div>
        </FullScreenDialogFooter>
      </FullScreenDialog>

      {/* Configuration Dialog */}
      <FullScreenDialog open={configOpen} onOpenChange={setConfigOpen}>
        <FullScreenDialogHeader
          icon={<Settings className="h-5 w-5" />}
          iconBg="bg-purple-500/10 dark:bg-purple-500/20"
          iconColor="text-purple-500"
          title={t("systemMemory.config.title", "Memory Configuration")}
          subtitle={t("systemMemory.config.description", "Configure memory extraction, compression, and LLM settings")}
          onClose={() => setConfigOpen(false)}
        />

        <FullScreenDialogContent>
          {configLoading ? (
            <div className="flex items-center justify-center h-full">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : (
            <FullScreenDialogMain className="p-6">
              <div className="space-y-8 max-w-4xl mx-auto">
                {/* Extraction Settings */}
                <div className="space-y-4">
                  <h3 className="text-sm font-semibold flex items-center gap-2">
                    <Sparkles className="h-4 w-4 text-purple-500" />
                    {t("systemMemory.config.extraction", "Extraction Settings")}
                  </h3>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.minMessages", "Min Messages")}</Label>
                      <Input
                        type="number"
                        min={1}
                        max={20}
                        value={config.extraction?.min_messages ?? 3}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            extraction: {
                              ...config.extraction,
                              min_messages: parseInt(e.target.value) || 3,
                            },
                          })
                        }
                      />
                      <p className="text-xs text-muted-foreground">
                        {t("systemMemory.config.minMessagesHint", "Minimum messages to trigger extraction")}
                      </p>
                    </div>
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.minImportance", "Min Importance")}</Label>
                      <Input
                        type="number"
                        min={0}
                        max={100}
                        value={config.extraction?.min_importance ?? 30}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            extraction: {
                              ...config.extraction,
                              min_importance: parseInt(e.target.value) || 30,
                            },
                          })
                        }
                      />
                      <p className="text-xs text-muted-foreground">
                        {t("systemMemory.config.minImportanceHint", "Minimum importance threshold (0-100)")}
                      </p>
                    </div>
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.similarityThreshold", "Similarity Threshold")}</Label>
                      <Input
                        type="number"
                        min={0}
                        max={1}
                        step={0.05}
                        value={config.extraction?.similarity_threshold ?? 0.85}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            extraction: {
                              ...config.extraction,
                              similarity_threshold: parseFloat(e.target.value) || 0.85,
                            },
                          })
                        }
                      />
                      <p className="text-xs text-muted-foreground">
                        {t("systemMemory.config.similarityThresholdHint", "Dedup similarity (0-1)")}
                      </p>
                    </div>
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.dedupEnabled", "Deduplication")}</Label>
                      <div className="flex items-center">
                        <Switch
                          checked={config.extraction?.dedup_enabled ?? true}
                          onCheckedChange={(checked) =>
                            setConfig({
                              ...config,
                              extraction: {
                                ...config.extraction,
                                dedup_enabled: checked,
                              },
                            })
                          }
                        />
                      </div>
                    </div>
                  </div>
                </div>

                {/* Compression Settings */}
                <div className="space-y-4">
                  <h3 className="text-sm font-semibold flex items-center gap-2">
                    <Archive className="h-4 w-4 text-blue-500" />
                    {t("systemMemory.config.compression", "Compression Settings")}
                  </h3>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.decayPeriodDays", "Decay Period (Days)")}</Label>
                      <Input
                        type="number"
                        min={1}
                        max={365}
                        value={config.compression?.decay_period_days ?? 30}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            compression: {
                              ...config.compression,
                              decay_period_days: parseInt(e.target.value) || 30,
                            },
                          })
                        }
                      />
                    </div>
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.compressionMinImportance", "Min Importance")}</Label>
                      <Input
                        type="number"
                        min={0}
                        max={100}
                        value={config.compression?.min_importance ?? 20}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            compression: {
                              ...config.compression,
                              min_importance: parseInt(e.target.value) || 20,
                            },
                          })
                        }
                      />
                      <p className="text-xs text-muted-foreground">
                        {t("systemMemory.config.compressionMinImportanceHint", "Entries below this will be deleted")}
                      </p>
                    </div>
                  </div>
                </div>

                {/* LLM Backend Settings */}
                <div className="space-y-4">
                  <h3 className="text-sm font-semibold flex items-center gap-2">
                    <Cpu className="h-4 w-4 text-green-500" />
                    {t("systemMemory.config.llmBackends", "LLM Backends")}
                  </h3>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.extractionBackend", "Extraction Model")}</Label>
                      <Select
                        value={config.llm?.extraction_backend_id || "__default__"}
                        onValueChange={(value) =>
                          setConfig({
                            ...config,
                            llm: {
                              ...config.llm,
                              extraction_backend_id: value === "__default__" ? undefined : value,
                            },
                          })
                        }
                      >
                        <SelectTrigger className="h-10">
                          <SelectValue placeholder={t("systemMemory.config.defaultBackend", "Use default")} />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="__default__">
                            <div className="flex items-center gap-2">
                              <span>{t("systemMemory.config.defaultBackend", "Use default")}</span>
                            </div>
                          </SelectItem>
                          {llmBackends.map((backend) => (
                            <SelectItem key={backend.id} value={backend.id}>
                              <div className="flex items-center gap-2">
                                <span>{backend.name}</span>
                                <div className="flex items-center gap-0.5 text-muted-foreground">
                                  {backend.capabilities?.supports_multimodal && (
                                    <span title={t("systemMemory.config.supportsVision", "Supports vision")}>
                                      <Eye className="h-4 w-4" />
                                    </span>
                                  )}
                                  {backend.capabilities?.supports_tools && (
                                    <span title={t("systemMemory.config.supportsTools", "Supports tools")}>
                                      <Wrench className="h-4 w-4" />
                                    </span>
                                  )}
                                  {backend.capabilities?.supports_thinking && (
                                    <span title={t("systemMemory.config.supportsThinking", "Supports thinking")}>
                                      <Brain className="h-4 w-4" />
                                    </span>
                                  )}
                                </div>
                                <span className="text-xs text-muted-foreground ml-auto">{backend.model}</span>
                              </div>
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <p className="text-xs text-muted-foreground">
                        {t("systemMemory.config.extractionBackendHint", "Lightweight model for extraction")}
                      </p>
                    </div>
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.compressionBackend", "Compression Model")}</Label>
                      <Select
                        value={config.llm?.compression_backend_id || "__default__"}
                        onValueChange={(value) =>
                          setConfig({
                            ...config,
                            llm: {
                              ...config.llm,
                              compression_backend_id: value === "__default__" ? undefined : value,
                            },
                          })
                        }
                      >
                        <SelectTrigger className="h-10">
                          <SelectValue placeholder={t("systemMemory.config.defaultBackend", "Use default")} />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="__default__">
                            <div className="flex items-center gap-2">
                              <span>{t("systemMemory.config.defaultBackend", "Use default")}</span>
                            </div>
                          </SelectItem>
                          {llmBackends.map((backend) => (
                            <SelectItem key={backend.id} value={backend.id}>
                              <div className="flex items-center gap-2">
                                <span>{backend.name}</span>
                                <div className="flex items-center gap-0.5 text-muted-foreground">
                                  {backend.capabilities?.supports_multimodal && (
                                    <span title={t("systemMemory.config.supportsVision", "Supports vision")}>
                                      <Eye className="h-4 w-4" />
                                    </span>
                                  )}
                                  {backend.capabilities?.supports_tools && (
                                    <span title={t("systemMemory.config.supportsTools", "Supports tools")}>
                                      <Wrench className="h-4 w-4" />
                                    </span>
                                  )}
                                  {backend.capabilities?.supports_thinking && (
                                    <span title={t("systemMemory.config.supportsThinking", "Supports thinking")}>
                                      <Brain className="h-4 w-4" />
                                    </span>
                                  )}
                                </div>
                                <span className="text-xs text-muted-foreground ml-auto">{backend.model}</span>
                              </div>
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <p className="text-xs text-muted-foreground">
                        {t("systemMemory.config.compressionBackendHint", "Powerful model for summarization")}
                      </p>
                    </div>
                  </div>
                </div>

                {/* Schedule Settings */}
                <div className="space-y-4">
                  <h3 className="text-sm font-semibold flex items-center gap-2">
                    <Clock className="h-4 w-4 text-orange-500" />
                    {t("systemMemory.config.schedule", "Schedule")}
                  </h3>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.autoExtraction", "Auto Extraction")}</Label>
                      <div className="flex items-center">
                        <Switch
                          checked={config.schedule?.extraction_enabled ?? true}
                          onCheckedChange={(checked) =>
                            setConfig({
                              ...config,
                              schedule: {
                                ...config.schedule,
                                extraction_enabled: checked,
                              },
                            })
                          }
                        />
                      </div>
                    </div>
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.extractionInterval", "Extraction Interval")}</Label>
                      <Select
                        value={String(config.schedule?.extraction_interval_secs ?? 3600)}
                        onValueChange={(value) =>
                          setConfig({
                            ...config,
                            schedule: {
                              ...config.schedule,
                              extraction_interval_secs: parseInt(value),
                            },
                          })
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="1800">30 {t("common.minutes", "minutes")}</SelectItem>
                          <SelectItem value="3600">1 {t("common.hour", "hour")}</SelectItem>
                          <SelectItem value="7200">2 {t("common.hours", "hours")}</SelectItem>
                          <SelectItem value="14400">4 {t("common.hours", "hours")}</SelectItem>
                          <SelectItem value="28800">8 {t("common.hours", "hours")}</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.autoCompression", "Auto Compression")}</Label>
                      <div className="flex items-center">
                        <Switch
                          checked={config.schedule?.compression_enabled ?? true}
                          onCheckedChange={(checked) =>
                            setConfig({
                              ...config,
                              schedule: {
                                ...config.schedule,
                                compression_enabled: checked,
                              },
                            })
                          }
                        />
                      </div>
                    </div>
                    <div className="space-y-2">
                      <Label>{t("systemMemory.config.compressionInterval", "Compression Interval")}</Label>
                      <Select
                        value={String(config.schedule?.compression_interval_secs ?? 86400)}
                        onValueChange={(value) =>
                          setConfig({
                            ...config,
                            schedule: {
                              ...config.schedule,
                              compression_interval_secs: parseInt(value),
                            },
                          })
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="43200">12 {t("common.hours", "hours")}</SelectItem>
                          <SelectItem value="86400">1 {t("common.day", "day")}</SelectItem>
                          <SelectItem value="172800">2 {t("common.days", "days")}</SelectItem>
                          <SelectItem value="604800">1 {t("common.week", "week")}</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </div>
                </div>
              </div>
            </FullScreenDialogMain>
          )}
        </FullScreenDialogContent>

        <FullScreenDialogFooter>
          <Button variant="outline" onClick={() => setConfigOpen(false)}>
            {t("common.cancel", "Cancel")}
          </Button>
          <Button onClick={handleSaveConfig} disabled={configSaving}>
            {configSaving ? (
              <Loader2 className="h-4 w-4 mr-1 animate-spin" />
            ) : (
              <Save className="h-4 w-4 mr-1" />
            )}
            {configSaving ? t("common.saving", "Saving...") : t("common.save", "Save")}
          </Button>
        </FullScreenDialogFooter>
      </FullScreenDialog>
    </div>
  )
})
