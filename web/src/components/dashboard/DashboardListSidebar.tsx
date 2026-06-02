/**
 * DashboardListSidebar Component
 *
 * Left sidebar for managing multiple dashboards.
 * - Desktop: Fixed sidebar with collapse toggle
 * - Mobile: Slide-out drawer with backdrop
 */

import { useState, useRef, useEffect } from 'react'
import {
  LayoutDashboard,
  Plus,
  Trash2,
  Pencil,
  Check,
  X,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { textBody, textNano } from '@/design-system/tokens/typography'
import type { Dashboard } from '@/types/dashboard'
import { confirm } from '@/hooks/use-confirm'

export interface DashboardListSidebarProps {
  dashboards: Dashboard[]
  currentDashboardId: string | null
  onSwitch: (id: string) => void
  onCreate: (name: string) => void
  onRename: (id: string, name: string) => void
  onDelete: (id: string) => void
  /** Open state: false = collapsed (desktop) or closed (mobile), true = expanded (desktop) or open drawer (mobile) */
  open?: boolean
  /** Open/close handler */
  onOpenChange?: (open: boolean) => void
  /** Is desktop mode (fixed sidebar) vs mobile (drawer) */
  isDesktop?: boolean
  className?: string
}

/** Inline rename input with auto-focus and select-all */
function InlineEdit({
  value,
  onSave,
  onCancel,
}: {
  value: string
  onSave: (v: string) => void
  onCancel: () => void
}) {
  const [draft, setDraft] = useState(value)
  const ref = useRef<HTMLInputElement>(null)

  useEffect(() => {
    const el = ref.current
    if (el) {
      el.focus()
      el.select()
    }
  }, [])

  return (
    <div className="flex items-center gap-1 h-full" onClick={(e) => e.stopPropagation()}>
      <Input
        ref={ref}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && draft.trim()) onSave(draft.trim())
          if (e.key === 'Escape') onCancel()
        }}
        className="h-7 text-sm flex-1 px-1.5 rounded-md"
        autoFocus
      />
      <button
        className="h-6 w-6 shrink-0 flex items-center justify-center rounded-md text-success hover:bg-success-light transition-colors"
        onClick={() => draft.trim() && onSave(draft.trim())}
      >
        <Check className="h-3.5 w-3.5" />
      </button>
      <button
        className="h-6 w-6 shrink-0 flex items-center justify-center rounded-md hover:bg-muted transition-colors"
        onClick={onCancel}
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  )
}

/** Shared content for both desktop and mobile sidebar layouts */
function DashboardSidebarContent({
  dashboards,
  currentDashboardId,
  onSwitch,
  onCreate,
  onRename,
  onDelete,
  onOpenChange,
  isDesktop,
}: Omit<DashboardListSidebarProps, 'open' | 'className'>) {
  const { t } = useTranslation('dashboardComponents')
  const [editingId, setEditingId] = useState<string | null>(null)
  const [showCreateInput, setShowCreateInput] = useState(false)
  const [newDashboardName, setNewDashboardName] = useState('')
  const createInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (showCreateInput && createInputRef.current) {
      createInputRef.current.focus()
    }
  }, [showCreateInput])

  const handleDelete = async (id: string) => {
    const dashboard = dashboards.find(d => d.id === id)
    const name = dashboard?.name || ''
    const confirmed = await confirm({
      title: t('sidebar.deleteTitle'),
      description: t('sidebar.deleteDescription', { name }),
      confirmText: t('sidebar.delete'),
      cancelText: t('common.cancel'),
      variant: 'destructive'
    })
    if (confirmed) {
      onDelete(id)
    }
  }

  const handleSwitch = (id: string) => {
    onSwitch(id)
    if (!isDesktop) {
      onOpenChange?.(false)
    }
  }

  return (
    <>
      {/* Mobile-only header with close button */}
      {!isDesktop && (
        <div className="flex items-center justify-between px-3 h-11 border-b border-border">
          <span className="font-medium text-xs text-muted-foreground uppercase tracking-wider">{t('sidebar.title')}</span>
          <button
            className="h-6 w-6 flex items-center justify-center rounded-md hover:bg-muted transition-colors"
            onClick={() => onOpenChange?.(false)}
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      )}

      {/* Create new dashboard — top area */}
      <div className="px-3 pt-3 pb-1">
        {showCreateInput ? (
          <div className="flex items-center gap-1">
            <Input
              ref={createInputRef}
              value={newDashboardName}
              onChange={(e) => setNewDashboardName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && newDashboardName.trim()) {
                  onCreate(newDashboardName.trim())
                  setNewDashboardName('')
                  setShowCreateInput(false)
                }
                if (e.key === 'Escape') {
                  setShowCreateInput(false)
                  setNewDashboardName('')
                }
              }}
              placeholder={t('sidebar.namePlaceholder')}
              className="h-7 text-sm flex-1 rounded-md"
              autoFocus
            />
            <button
              className="h-7 w-7 shrink-0 flex items-center justify-center rounded-md text-success hover:bg-success-light transition-colors"
              onClick={() => {
                if (newDashboardName.trim()) {
                  onCreate(newDashboardName.trim())
                  setNewDashboardName('')
                  setShowCreateInput(false)
                }
              }}
            >
              <Check className="h-3.5 w-3.5" />
            </button>
            <button
              className="h-7 w-7 shrink-0 flex items-center justify-center rounded-md hover:bg-muted transition-colors"
              onClick={() => { setShowCreateInput(false); setNewDashboardName('') }}
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        ) : (
          <button
            onClick={() => setShowCreateInput(true)}
            className="flex items-center justify-center gap-1.5 h-7 rounded-md text-xs font-medium bg-muted text-muted-foreground hover:text-foreground hover:bg-muted-50 transition-colors w-full"
          >
            <Plus className="h-3.5 w-3.5" />
            {t('sidebar.newDashboard')}
          </button>
        )}
      </div>

      {/* Dashboard List */}
      <div className="flex-1 overflow-y-auto px-2 pb-2 space-y-1">
        {dashboards.map((dashboard) => {
          const isActive = dashboard.id === currentDashboardId
          const isEditing = editingId === dashboard.id

          return (
            <div
              key={dashboard.id}
              role="button"
              tabIndex={0}
              onClick={() => !isEditing && handleSwitch(dashboard.id)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault()
                  if (!isEditing) handleSwitch(dashboard.id)
                }
              }}
              onDoubleClick={() => !isEditing && setEditingId(dashboard.id)}
              className={cn(
                'group relative flex items-center gap-2.5 px-3 py-2 rounded-md cursor-pointer transition-all',
                isActive
                  ? 'bg-primary/8 text-foreground'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground'
              )}
            >
              {/* Active indicator */}
              {isActive && (
                <div className="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-4 rounded-full bg-primary" />
              )}

              {isEditing ? (
                <InlineEdit
                  value={dashboard.name}
                  onSave={(name) => { onRename(dashboard.id, name); setEditingId(null) }}
                  onCancel={() => setEditingId(null)}
                />
              ) : (
                <>
                  <LayoutDashboard className="h-4 w-4 shrink-0" />

                  <span className="text-sm flex-1 truncate">
                    {dashboard.name}
                  </span>

                  {/* Hover actions */}
                  <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      className="h-6 w-6 flex items-center justify-center rounded hover:bg-muted transition-colors"
                      onClick={(e) => { e.stopPropagation(); setEditingId(dashboard.id) }}
                      title={t('sidebar.rename')}
                    >
                      <Pencil className="h-3 w-3" />
                    </button>
                    {dashboards.length > 1 && (
                      <button
                        className="h-6 w-6 flex items-center justify-center rounded hover:bg-error-light text-muted-foreground hover:text-destructive transition-colors"
                        onClick={(e) => { e.stopPropagation(); handleDelete(dashboard.id) }}
                        title={t('sidebar.delete')}
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    )}
                  </div>
                </>
              )}
            </div>
          )
        })}
      </div>
    </>
  )
}

export function DashboardListSidebar({
  dashboards,
  currentDashboardId,
  onSwitch,
  onCreate,
  onRename,
  onDelete,
  open = true,
  onOpenChange,
  isDesktop = true,
  className,
}: DashboardListSidebarProps) {
  const contentProps = {
    dashboards,
    currentDashboardId,
    onSwitch,
    onCreate,
    onRename,
    onDelete,
    onOpenChange,
    isDesktop,
  }

  // Desktop mode: fixed sidebar with collapse
  if (isDesktop) {
    return (
      <div
        className={cn(
          'flex-shrink-0 flex flex-col bg-card transition-[width] duration-300 ease-out',
          !open ? 'w-0 overflow-hidden border-r-0' : 'w-64 border-r border-border',
          className
        )}
      >
        <DashboardSidebarContent {...contentProps} />
      </div>
    )
  }

  // Mobile mode: drawer with backdrop
  return (
    <>
      {/* Backdrop */}
      {open && (
        <div
          className="fixed inset-0 bg-black/30 backdrop-blur-sm z-[55] transition-opacity lg:hidden"
          style={{ top: 'var(--topnav-height, 56px)' }}
          onClick={() => onOpenChange?.(false)}
        />
      )}

      {/* Sidebar Drawer */}
      <div
        className={cn(
          'fixed left-0 bottom-0 w-72 z-[60] lg:hidden',
          'bg-background shadow-xl flex flex-col',
          'transform transition-transform duration-300 ease-out',
          open ? 'translate-x-0' : '-translate-x-full',
          className
        )}
        style={{ top: 'var(--topnav-height, 56px)' }}
      >
        <DashboardSidebarContent {...contentProps} />
      </div>
    </>
  )
}
