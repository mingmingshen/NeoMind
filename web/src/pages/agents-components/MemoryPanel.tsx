/**
 * Memory Panel
 *
 * Displays Markdown-based memory files organized by category.
 * Supports viewing with ReactMarkdown and editing with CodeMirror.
 */

import { useState, useEffect, useCallback } from "react"
import { useTranslation } from "react-i18next"
import ReactMarkdown from "react-markdown"
import CodeMirror from "@uiw/react-codemirror"
import { Edit3, Save, X, RefreshCw, Minimize2, Download, Loader2 } from "lucide-react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Skeleton } from "@/components/ui/skeleton"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"

// Memory categories with labels
const categories = [
  { id: "user_profile", labelKey: "memory.categories.userProfile", defaultLabel: "User Profile" },
  { id: "domain_knowledge", labelKey: "memory.categories.domainKnowledge", defaultLabel: "Domain Knowledge" },
  { id: "task_patterns", labelKey: "memory.categories.taskPatterns", defaultLabel: "Task Patterns" },
  { id: "system_evolution", labelKey: "memory.categories.systemEvolution", defaultLabel: "System Evolution" },
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

interface MemoryPanelProps {
  refreshKey?: number
}

export function MemoryPanel({ refreshKey }: MemoryPanelProps) {
  const { t } = useTranslation("agents")
  const { handleError } = useErrorHandler()

  const [category, setCategory] = useState("user_profile")
  const [content, setContent] = useState("")
  const [loading, setLoading] = useState(true)
  const [editing, setEditing] = useState(false)
  const [editContent, setEditContent] = useState("")
  const [saving, setSaving] = useState(false)
  const [stats, setStats] = useState<Record<string, CategoryStats>>({})
  const [operating, setOperating] = useState<string | null>(null)

  const loadContent = useCallback(async () => {
    setLoading(true)
    try {
      const response: MemoryContentResponse = await api.getMemoryCategory(category)
      setContent(response.content || "")
      setEditContent(response.content || "")
    } catch (error) {
      handleError(error, { operation: "Load memory", showToast: false })
      setContent("")
      setEditContent("")
    } finally {
      setLoading(false)
    }
  }, [category, handleError])

  const loadStats = useCallback(async () => {
    try {
      const response: MemoryStatsResponse = await api.getMemoryStats()
      setStats(response.categories || {})
    } catch (error) {
      handleError(error, { operation: "Load stats", showToast: false })
    }
  }, [handleError])

  useEffect(() => {
    loadContent()
  }, [loadContent, refreshKey])

  useEffect(() => {
    loadStats()
  }, [loadStats])

  const handleSave = async () => {
    setSaving(true)
    try {
      await api.updateMemoryCategory(category, editContent)
      setContent(editContent)
      setEditing(false)
      loadStats()
    } catch (error) {
      handleError(error, { operation: "Save memory" })
    } finally {
      setSaving(false)
    }
  }

  const handleCancel = () => {
    setEditContent(content)
    setEditing(false)
  }

  const handleExtract = async () => {
    setOperating("extract")
    try {
      await api.triggerMemoryExtract()
      await loadContent()
      await loadStats()
    } catch (error) {
      handleError(error, { operation: "Extract memory" })
    } finally {
      setOperating(null)
    }
  }

  const handleCompress = async () => {
    setOperating("compress")
    try {
      await api.triggerMemoryCompress()
      await loadContent()
      await loadStats()
    } catch (error) {
      handleError(error, { operation: "Compress memory" })
    } finally {
      setOperating(null)
    }
  }

  const handleExport = async () => {
    setOperating("export")
    try {
      const markdown = await api.exportAllMemory()
      const blob = new Blob([markdown], { type: "text/markdown" })
      const url = URL.createObjectURL(blob)
      const a = document.createElement("a")
      a.href = url
      a.download = `memory_${new Date().toISOString().split("T")[0]}.md`
      a.click()
      URL.revokeObjectURL(url)
    } catch (error) {
      handleError(error, { operation: "Export memory" })
    } finally {
      setOperating(null)
    }
  }

  // Check for dark mode
  const isDark =
    typeof document !== "undefined" &&
    (document.documentElement.getAttribute("data-theme") === "dark" ||
      document.documentElement.classList.contains("dark"))

  return (
    <div className="space-y-4">
      {/* Category Tabs */}
      <Tabs value={category} onValueChange={setCategory}>
        <TabsList>
          {categories.map((c) => {
            const label = t(c.labelKey, c.defaultLabel)
            const entryCount = stats[c.id]?.entry_count ?? 0
            return (
              <TabsTrigger key={c.id} value={c.id} className="gap-2">
                {label}
                {entryCount > 0 && (
                  <Badge variant="secondary" className="text-xs">
                    {entryCount}
                  </Badge>
                )}
              </TabsTrigger>
            )
          })}
        </TabsList>
      </Tabs>

      {/* Action Buttons */}
      <div className="flex flex-wrap gap-2">
        {editing ? (
          <>
            <Button size="sm" onClick={handleSave} disabled={saving}>
              {saving ? (
                <Loader2 className="h-4 w-4 mr-1 animate-spin" />
              ) : (
                <Save className="h-4 w-4 mr-1" />
              )}
              {saving ? t("memory.saving", "Saving...") : t("memory.save", "Save")}
            </Button>
            <Button size="sm" variant="outline" onClick={handleCancel} disabled={saving}>
              <X className="h-4 w-4 mr-1" />
              {t("memory.cancel", "Cancel")}
            </Button>
          </>
        ) : (
          <>
            <Button
              size="sm"
              variant="outline"
              onClick={() => {
                setEditContent(content)
                setEditing(true)
              }}
            >
              <Edit3 className="h-4 w-4 mr-1" />
              {t("memory.edit", "Edit")}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={handleExtract}
              disabled={operating !== null}
            >
              {operating === "extract" ? (
                <Loader2 className="h-4 w-4 mr-1 animate-spin" />
              ) : (
                <RefreshCw className="h-4 w-4 mr-1" />
              )}
              {t("memory.extract", "Extract")}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={handleCompress}
              disabled={operating !== null}
            >
              {operating === "compress" ? (
                <Loader2 className="h-4 w-4 mr-1 animate-spin" />
              ) : (
                <Minimize2 className="h-4 w-4 mr-1" />
              )}
              {t("memory.compress", "Compress")}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={handleExport}
              disabled={operating !== null}
            >
              {operating === "export" ? (
                <Loader2 className="h-4 w-4 mr-1 animate-spin" />
              ) : (
                <Download className="h-4 w-4 mr-1" />
              )}
              {t("memory.export", "Export")}
            </Button>
          </>
        )}
      </div>

      {/* Content Area */}
      {loading ? (
        <div className="space-y-2">
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-8 w-3/4" />
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-8 w-1/2" />
        </div>
      ) : editing ? (
        <div className="border rounded-lg overflow-hidden">
          <CodeMirror
            value={editContent}
            height="400px"
            onChange={(value) => setEditContent(value)}
            theme={isDark ? "dark" : "light"}
            className="text-sm"
            style={{
              fontSize: "14px",
              fontFamily:
                'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace',
            }}
          />
        </div>
      ) : (
        <div className="border rounded-lg p-6 prose prose-sm dark:prose-invert max-w-none overflow-auto max-h-[500px]">
          {content ? (
            <ReactMarkdown>{content}</ReactMarkdown>
          ) : (
            <p className="text-muted-foreground italic">
              {t("memory.empty", "No content yet")}
            </p>
          )}
        </div>
      )}
    </div>
  )
}
