/**
 * SkillsPanel - Skill management panel for the AI Agent page.
 *
 * Displays user-defined skills using ResponsiveTable.
 * Unified fullscreen dialog for view/edit/create.
 */

import { useState, useEffect, useCallback, forwardRef, useImperativeHandle } from "react"
import {
  BookOpen,
  Loader2,
  Plus,
  Trash2,
  Wrench,
  Save,
  X,
  Pencil,
  Eye,
  Hash,
  Zap,
  Target,
} from "lucide-react"
import CodeMirror from "@uiw/react-codemirror"
import { ResponsiveTable, Pagination } from "@/components/shared"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import {
  FullScreenDialog,
  FullScreenDialogHeader,
  FullScreenDialogContent,
  FullScreenDialogFooter,
} from "@/components/automation/dialog"
import { useErrorHandler } from "@/hooks/useErrorHandler"
import { useToast } from "@/hooks/use-toast"
import { confirm } from "@/hooks/use-confirm"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import type { SkillSummary, SkillDetail } from "@/types/skill"

// Category configuration with colors matching MemoryPanel style
const categoryConfig: Record<
  string,
  {
    label: string
    icon: React.ElementType
    color: string
  }
> = {
  device: { label: "Device", icon: Zap, color: "bg-blue-500/10 text-blue-600 border-blue-500/20" },
  rule: { label: "Rule", icon: Target, color: "bg-purple-500/10 text-purple-600 border-purple-500/20" },
  agent: { label: "Agent", icon: Wrench, color: "bg-green-500/10 text-green-600 border-green-500/20" },
  message: { label: "Message", icon: BookOpen, color: "bg-orange-500/10 text-orange-600 border-orange-500/20" },
  extension: { label: "Extension", icon: Wrench, color: "bg-teal-500/10 text-teal-600 border-teal-500/20" },
  general: { label: "General", icon: BookOpen, color: "bg-gray-500/10 text-gray-600 border-gray-500/20" },
}

// Table row type
interface SkillRow {
  id: string
  name: string
  category: string
  priority: number
  keywords: string[]
  body_length: number
}

// Default new skill template
const defaultTemplate = `---
id: my-skill
name: My Skill
category: general
priority: 50
token_budget: 500
triggers:
  keywords: [example keyword]
  tool_target:
    tool: example
    actions: [action1]
anti_triggers:
  keywords: []
---

# My Skill

Step-by-step guide here.
`

export interface SkillsPanelHandle {
  openCreate: () => void
  reload: () => void
}

export const SkillsPanel = forwardRef<SkillsPanelHandle>(function SkillsPanel(
  _props,
  ref
) {
  const { handleError } = useErrorHandler()
  const { toast } = useToast()

  // Data state
  const [skills, setSkills] = useState<SkillSummary[]>([])
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)
  const [currentPage, setCurrentPage] = useState(1)
  const [pageSize, setPageSize] = useState(10)

  // Unified dialog state
  const [dialogOpen, setDialogOpen] = useState(false)
  const [dialogMode, setDialogMode] = useState<"view" | "create" | "edit">("view")
  const [dialogContent, setDialogContent] = useState(defaultTemplate)
  const [dialogSkill, setDialogSkill] = useState<SkillDetail | null>(null)
  const [dialogLoading, setDialogLoading] = useState(false)
  const [saving, setSaving] = useState(false)

  // Check dark mode
  const isDark =
    typeof document !== "undefined" &&
    (document.documentElement.getAttribute("data-theme") === "dark" ||
      document.documentElement.classList.contains("dark"))

  // Load skills list
  const loadSkills = useCallback(async () => {
    setLoading(true)
    try {
      const res = await api.get<{ skills: SkillSummary[]; total: number; page: number; page_size: number; total_pages: number }>(
        `/skills?page=${currentPage}&page_size=${pageSize}`
      )
      setSkills(res.skills)
      setTotal(res.total)
    } catch (e) {
      handleError(e, { operation: "Load skills", showToast: false })
    } finally {
      setLoading(false)
    }
  }, [handleError, currentPage, pageSize])

  useEffect(() => {
    loadSkills()
  }, [loadSkills])

  // Helper: reconstruct full .md file content from detail
  const reconstructContent = (detail: SkillDetail): string => {
    const yamlSafeStr = (s: string) => (/[,:{}[\]&*?|>!%@"`]/.test(s) ? `"${s.replace(/"/g, '\\"')}"` : s)
    const yamlSafeArr = (arr: string[]) => arr.length === 0
      ? "[]"
      : `[${arr.map((k) => yamlSafeStr(k)).join(", ")}]`

    const frontmatter = `---
id: ${detail.id}
name: ${yamlSafeStr(detail.name)}
category: ${detail.category}
priority: ${detail.priority}
token_budget: ${detail.token_budget}
triggers:
  keywords: ${yamlSafeArr(detail.keywords)}
  tool_target:
${detail.tool_targets.map((t) => `    tool: ${t.tool}\n    actions: ${yamlSafeArr(t.actions)}`).join("\n")}
anti_triggers:
  keywords: ${yamlSafeArr(detail.anti_trigger_keywords)}
---`
    return `${frontmatter}\n\n${detail.body}`
  }

  // Open dialog in view mode
  const handleView = async (id: string) => {
    setDialogLoading(true)
    setDialogOpen(true)
    setDialogMode("view")
    try {
      const detail = await api.get<SkillDetail>(`/skills/${id}`)
      setDialogSkill(detail)
      setDialogContent(reconstructContent(detail))
    } catch (e) {
      handleError(e, { operation: "Load skill detail" })
      setDialogOpen(false)
    } finally {
      setDialogLoading(false)
    }
  }

  // Open dialog in edit mode
  const handleEdit = async (id: string) => {
    setDialogLoading(true)
    setDialogOpen(true)
    setDialogMode("edit")
    try {
      const detail = await api.get<SkillDetail>(`/skills/${id}`)
      setDialogSkill(detail)
      setDialogContent(reconstructContent(detail))
    } catch (e) {
      handleError(e, { operation: "Load skill for editing" })
      setDialogOpen(false)
    } finally {
      setDialogLoading(false)
    }
  }

  // Open dialog in create mode
  const handleCreate = () => {
    setDialogSkill(null)
    setDialogContent(defaultTemplate)
    setDialogMode("create")
    setDialogOpen(true)
  }

  // Expose methods to parent via ref
  useImperativeHandle(ref, () => ({
    openCreate: handleCreate,
    reload: () => {
      loadSkills()
    },
  }), [loadSkills])

  // Close dialog
  const closeDialog = () => {
    setDialogOpen(false)
  }

  // Switch from view to edit
  const switchToEdit = () => {
    if (!dialogSkill) return
    setDialogMode("edit")
  }

  // Save skill (create or update)
  const handleSave = async () => {
    setSaving(true)
    try {
      if (dialogMode === "create") {
        await api.post("/skills", { content: dialogContent })
      } else if (dialogSkill) {
        await api.put(`/skills/${dialogSkill.id}`, { content: dialogContent })
      }
      toast({ title: dialogMode === "create" ? "Skill created" : "Skill updated" })
      closeDialog()
      loadSkills()
    } catch (e) {
      handleError(e, { operation: "Save skill" })
    } finally {
      setSaving(false)
    }
  }

  // Delete skill
  const handleDelete = async (id: string, name: string) => {
    const confirmed = await confirm({
      title: "Delete Skill",
      description: `Are you sure you want to delete "${name}"? This action cannot be undone.`,
      confirmText: "Delete",
      cancelText: "Cancel",
      variant: "destructive",
    })
    if (!confirmed) return

    try {
      await api.delete(`/skills/${id}`)
      toast({ title: `Skill "${name}" deleted` })
      loadSkills()
    } catch (e) {
      handleError(e, { operation: "Delete skill" })
    }
  }

  // Table data
  const tableData: SkillRow[] = skills.map((skill) => ({
    id: skill.id,
    name: skill.name,
    category: skill.category,
    priority: skill.priority,
    keywords: skill.keywords,
    body_length: skill.body_length,
  }))

  // Dialog title
  const dialogTitle =
    dialogMode === "create"
      ? "Create New Skill"
      : dialogSkill?.name || ""

  const lineCount = dialogContent.split("\n").filter((l) => l.trim()).length

  return (
    <div className="space-y-4">
      {/* Skills table */}
      <ResponsiveTable
        columns={[
          {
            key: "name",
            label: "Skill",
          },
          {
            key: "priority",
            label: (
              <div className="flex items-center gap-1">
                <Hash className="h-3.5 w-3.5" />
                Priority
              </div>
            ),
            align: "center",
            width: "w-20",
          },
          {
            key: "body_length",
            label: "Size",
            align: "right",
            width: "w-20",
          },
        ]}
        data={tableData as unknown as Record<string, unknown>[]}
        rowKey={(row) => (row as unknown as SkillRow).id}
        loading={loading}
        renderCell={(columnKey, rowData) => {
          const row = rowData as unknown as SkillRow
          const catConf = categoryConfig[row.category] || categoryConfig.general
          const Icon = catConf.icon

          switch (columnKey) {
            case "name":
              return (
                <div className="flex items-center gap-3">
                  <div
                    className={cn(
                      "w-9 h-9 rounded-lg flex items-center justify-center border",
                      catConf.color
                    )}
                  >
                    <Icon className="h-4 w-4" />
                  </div>
                  <div className="min-w-0">
                    <div className="font-medium text-sm truncate">{row.name}</div>
                    <div className="text-xs text-muted-foreground">
                      <span>{row.category}</span>
                    </div>
                    <div className="flex flex-wrap gap-1 mt-0.5">
                      {row.keywords.slice(0, 3).map((kw, i) => (
                        <span key={i} className="text-xs bg-muted px-1 py-0 rounded">
                          {kw}
                        </span>
                      ))}
                      {row.keywords.length > 3 && (
                        <span className="text-xs text-muted-foreground">
                          +{row.keywords.length - 3}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              )

            case "priority":
              return (
                <Badge variant="secondary" className="font-mono">
                  {row.priority}
                </Badge>
              )

            case "body_length":
              return (
                <span className="text-xs text-muted-foreground font-mono">
                  {row.body_length > 1024
                    ? `${(row.body_length / 1024).toFixed(1)} KB`
                    : `${row.body_length} B`}
                </span>
              )

            default:
              return null
          }
        }}
        actions={[
          {
            label: "View",
            icon: <Eye className="h-4 w-4" />,
            onClick: (rowData) => {
              const row = rowData as unknown as SkillRow
              handleView(row.id)
            },
          },
          {
            label: "Edit",
            icon: <Pencil className="h-4 w-4" />,
            onClick: (rowData) => {
              const row = rowData as unknown as SkillRow
              handleEdit(row.id)
            },
          },
          {
            label: "Delete",
            icon: <Trash2 className="h-4 w-4" />,
            onClick: (rowData) => {
              const row = rowData as unknown as SkillRow
              handleDelete(row.id, row.name)
            },
          },
        ]}
      />

      {/* Pagination */}
      <Pagination
        total={total}
        pageSize={pageSize}
        currentPage={currentPage}
        onPageChange={setCurrentPage}
        showSizeChanger
        onPageSizeChange={(size) => {
          setPageSize(size)
          setCurrentPage(1)
        }}
      />

      {/* Unified Dialog (view / edit / create) */}
      <FullScreenDialog open={dialogOpen} onOpenChange={(open) => { if (!open) closeDialog() }}>
        <FullScreenDialogHeader
          icon={
            dialogMode === "create" ? (
              <Plus className="h-5 w-5" />
            ) : (
              <BookOpen className="h-5 w-5" />
            )
          }
          iconBg={
            dialogSkill
              ? categoryConfig[dialogSkill.category]?.color || "bg-muted"
              : "bg-green-500/10 dark:bg-green-500/20"
          }
          iconColor={dialogMode === "create" ? "text-green-500" : undefined}
          title={dialogTitle}
          onClose={closeDialog}
        />

        <FullScreenDialogContent>
          {dialogLoading ? (
            <div className="flex items-center justify-center w-full h-full">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : (
            <div className="w-full h-full overflow-hidden">
              <CodeMirror
                value={dialogContent}
                height="100%"
                onChange={dialogMode !== "view" ? (val) => setDialogContent(val) : undefined}
                editable={dialogMode !== "view"}
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
          )}
        </FullScreenDialogContent>

        <FullScreenDialogFooter>
          <div className="flex items-center justify-between w-full">
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground">
                {lineCount} lines
              </span>
            </div>
            <div className="flex items-center gap-2">
              {dialogMode === "view" && (
                <Button onClick={switchToEdit}>
                  <Pencil className="h-4 w-4 mr-1" />
                  Edit
                </Button>
              )}
              {dialogMode !== "view" && (
                <>
                  <Button variant="outline" onClick={closeDialog} disabled={saving}>
                    <X className="h-4 w-4 mr-1" />
                    Cancel
                  </Button>
                  <Button onClick={handleSave} disabled={saving}>
                    {saving ? (
                      <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                    ) : (
                      <Save className="h-4 w-4 mr-1" />
                    )}
                    {saving
                      ? "Saving..."
                      : dialogMode === "create"
                        ? "Create"
                        : "Save"}
                  </Button>
                </>
              )}
            </div>
          </div>
        </FullScreenDialogFooter>
      </FullScreenDialog>
    </div>
  )
})
