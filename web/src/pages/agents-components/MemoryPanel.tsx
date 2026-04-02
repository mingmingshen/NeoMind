/**
 * Memory Panel
 *
 * Displays Markdown-based memory files organized by category.
 * Uses table layout similar to device list for consistency.
 */

import { useState, useEffect, useCallback } from "react"
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
} from "lucide-react"
import { ResponsiveTable } from "@/components/shared"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Skeleton } from "@/components/ui/skeleton"
import {
  FullScreenDialog,
  FullScreenDialogHeader,
  FullScreenDialogContent,
  FullScreenDialogFooter,
} from "@/components/automation/dialog"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import { formatTimestamp } from "@/lib/utils/format"

// Memory categories configuration
const categoryConfig = [
  {
    id: "user_profile",
    labelKey: "memory.categories.userProfile",
    defaultLabel: "User Profile",
    icon: User,
    description: "用户偏好、习惯和设置",
    color: "bg-blue-500/10 text-blue-600 border-blue-500/20",
  },
  {
    id: "domain_knowledge",
    labelKey: "memory.categories.domainKnowledge",
    defaultLabel: "Domain Knowledge",
    icon: BookOpen,
    description: "设备、协议和系统知识",
    color: "bg-green-500/10 text-green-600 border-green-500/20",
  },
  {
    id: "task_patterns",
    labelKey: "memory.categories.taskPatterns",
    defaultLabel: "Task Patterns",
    icon: Repeat2,
    description: "常见任务和操作模式",
    color: "bg-purple-500/10 text-purple-600 border-purple-500/20",
  },
  {
    id: "system_evolution",
    labelKey: "memory.categories.systemEvolution",
    defaultLabel: "System Evolution",
    icon: Cpu,
    description: "系统变更和优化记录",
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

export function MemoryPanel({ refreshKey }: MemoryPanelProps) {
  const { t } = useTranslation("agents")
  const { handleError } = useErrorHandler()

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
            label: t("memory.headers.category", "Category"),
          },
          {
            key: "entry_count",
            label: (
              <div className="flex items-center gap-1">
                <Hash className="h-3.5 w-3.5" />
                {t("memory.headers.entries", "Entries")}
              </div>
            ),
            align: "center",
            width: "w-24",
          },
          {
            key: "file_size",
            label: t("memory.headers.size", "Size"),
            align: "right",
            width: "w-20",
          },
          {
            key: "modified_at",
            label: (
              <div className="flex items-center gap-1">
                <Clock className="h-3.5 w-3.5" />
                {t("memory.headers.modified", "Modified")}
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
            label: t("memory.viewEdit", "View/Edit"),
            icon: <Eye className="h-4 w-4" />,
            onClick: (rowData) => {
              const row = rowData as unknown as MemoryCategoryRow
              handleViewEdit(row.id)
            },
          },
          {
            label: t("memory.export", "Export"),
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

        <FullScreenDialogContent>
          {contentLoading ? (
            <div className="flex items-center justify-center h-full">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : editing ? (
            <div className="h-full flex flex-col">
              <CodeMirror
                value={editContent}
                height="100%"
                onChange={(value) => setEditContent(value)}
                theme={isDark ? "dark" : "light"}
                className="flex-1 text-sm"
                style={{
                  fontSize: "14px",
                  fontFamily:
                    'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace',
                }}
              />
            </div>
          ) : (
            <div className="prose prose-sm dark:prose-invert max-w-none p-6 overflow-auto h-full">
              {content ? (
                <ReactMarkdown>{content}</ReactMarkdown>
              ) : (
                <p className="text-muted-foreground italic">
                  {t("memory.empty", "No content yet")}
                </p>
              )}
            </div>
          )}
        </FullScreenDialogContent>

        <FullScreenDialogFooter>
          <div className="flex items-center justify-between w-full">
            <div className="text-xs text-muted-foreground">
              {content.split("\n").filter(l => l.trim()).length} {t("memory.lines", "lines")}
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
                    {t("memory.cancel", "Cancel")}
                  </Button>
                  <Button onClick={handleSave} disabled={saving}>
                    {saving ? (
                      <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                    ) : (
                      <Save className="h-4 w-4 mr-1" />
                    )}
                    {saving ? t("memory.saving", "Saving...") : t("memory.save", "Save")}
                  </Button>
                </>
              ) : (
                <Button onClick={() => setEditing(true)}>
                  <Pencil className="h-4 w-4 mr-1" />
                  {t("memory.edit", "Edit")}
                </Button>
              )}
            </div>
          </div>
        </FullScreenDialogFooter>
      </FullScreenDialog>
    </div>
  )
}
