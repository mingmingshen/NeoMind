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
  MoreVertical,
} from "lucide-react"
import CodeMirror from "@uiw/react-codemirror"
import { Card } from "@/components/ui/card"
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from "@/components/ui/dropdown-menu"
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
import { fontMonoStack, textMini } from "@/design-system/tokens/typography"
import { useIsMobile } from "@/hooks/useMobile"
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
  device: { label: "Device", icon: Zap, color: "bg-info-light text-info border-info" },
  rule: { label: "Rule", icon: Target, color: "bg-accent-purple-light text-accent-purple border-accent-purple-light" },
  agent: { label: "Agent", icon: Wrench, color: "bg-success-light text-success border-success-light" },
  message: { label: "Message", icon: BookOpen, color: "bg-accent-orange-light text-accent-orange border-accent-orange-light" },
  extension: { label: "Extension", icon: Wrench, color: "bg-accent-cyan-light text-accent-cyan border-accent-cyan-light" },
  general: { label: "General", icon: BookOpen, color: "bg-muted text-muted-foreground border-border" },
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
  const isMobile = useIsMobile()

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
      const res = await api.listSkills(currentPage, pageSize)
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
      const detail = await api.getSkill(id)
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
      const detail = await api.getSkill(id)
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
        await api.createSkill(dialogContent)
      } else if (dialogSkill) {
        await api.updateSkill(dialogSkill.id, dialogContent)
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
      await api.deleteSkill(id)
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
      {isMobile ? (
        <div className="space-y-2">
          {tableData.map((row) => {
            const catConf = categoryConfig[row.category] || categoryConfig.general
            const Icon = catConf.icon
            return (
              <Card
                key={row.id}
                className="overflow-hidden border-border shadow-sm cursor-pointer active:scale-[0.99] transition-all"
                onClick={() => handleView(row.id)}
              >
                <div className="px-3 py-2.5">
                  {/* Row 1: icon + name + actions */}
                  <div className="flex items-center gap-2.5">
                    <div className={cn("w-8 h-8 rounded-lg flex items-center justify-center border shrink-0", catConf.color)}>
                      <Icon className="h-4 w-4" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="font-medium text-sm truncate">{row.name}</div>
                    </div>
                    <Badge variant="secondary" className={cn(textMini, "font-mono h-5 px-1.5")}>
                      {row.priority}
                    </Badge>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                        <button className="p-1 rounded-md hover:bg-muted">
                          <MoreVertical className="h-4 w-4 text-muted-foreground" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={(e) => { e.stopPropagation(); handleView(row.id) }}>
                          <Eye className="h-4 w-4 mr-2" />
                          View
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={(e) => { e.stopPropagation(); handleEdit(row.id) }}>
                          <Pencil className="h-4 w-4 mr-2" />
                          Edit
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          className="text-error"
                          onClick={(e) => { e.stopPropagation(); handleDelete(row.id, row.name) }}
                        >
                          <Trash2 className="h-4 w-4 mr-2" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                  {/* Row 2: keywords + size */}
                  <div className="flex items-center gap-1.5 mt-1.5 ml-[42px]">
                    <span className={cn(textMini, "text-muted-foreground")}>{row.category}</span>
                    <span className={cn(textMini, "text-muted-foreground ml-auto")}>
                      {row.body_length > 1024
                        ? `${(row.body_length / 1024).toFixed(1)} KB`
                        : `${row.body_length} B`}
                    </span>
                  </div>
                </div>
              </Card>
            )
          })}
        </div>
      ) : (
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
                <Hash className="h-4 w-4" />
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
      )}

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
              : "bg-success-light dark:bg-success-light"
          }
          iconColor={dialogMode === "create" ? "text-success" : undefined}
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
                  fontFamily: fontMonoStack,
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
