/**
 * DashboardListSidebar Component
 *
 * Left sidebar for managing multiple dashboards.
 * - Desktop: Fixed sidebar with collapse toggle
 * - Mobile: Slide-out drawer with backdrop
 */

import { useState } from 'react'
import {
  LayoutDashboard,
  Plus,
  Trash2,
  Edit2,
  Check,
  X,
  ChevronLeft,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
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
  const [editingName, setEditingName] = useState('')
  const [showCreateInput, setShowCreateInput] = useState(false)
  const [newDashboardName, setNewDashboardName] = useState('')

  const handleStartEdit = (dashboard: Dashboard) => {
    setEditingId(dashboard.id)
    setEditingName(dashboard.name)
  }

  const handleSaveEdit = () => {
    if (editingId && editingName.trim()) {
      onRename(editingId, editingName.trim())
    }
    setEditingId(null)
    setEditingName('')
  }

  const handleCancelEdit = () => {
    setEditingId(null)
    setEditingName('')
  }

  const handleCreate = () => {
    if (newDashboardName.trim()) {
      onCreate(newDashboardName.trim())
      setNewDashboardName('')
      setShowCreateInput(false)
    }
  }

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
    // Close drawer on mobile after switching
    if (!isDesktop) {
      onOpenChange?.(false)
    }
  }

  return (
    <>
      {/* Header */}
      <div className="flex items-center justify-between px-4 h-12 border-b border-border">
        <div className="flex items-center gap-2">
          <LayoutDashboard className="w-4 h-4 text-primary" />
          <h2 className="font-semibold text-sm">{t('sidebar.title')}</h2>
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => onOpenChange?.(false)}
        >
          {isDesktop ? <ChevronLeft className="h-4 w-4" /> : <X className="h-4 w-4" />}
        </Button>
      </div>

      {/* Dashboard List */}
      <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
        {dashboards.map((dashboard) => {
          const isEditing = editingId === dashboard.id
          const isActive = dashboard.id === currentDashboardId
          const componentCount = dashboard.components?.length ?? 0

          return (
            <div
              key={dashboard.id}
              className={cn(
                'group relative rounded-lg transition-all',
                isActive
                  ? 'bg-primary/8 ring-1 ring-primary/15'
                  : 'hover:bg-muted-50'
              )}
            >
              {isActive && (
                <div className="absolute left-0 top-2 bottom-2 w-0.5 rounded-full bg-primary" />
              )}
              {isEditing ? (
                <div className="flex items-center gap-1 p-2 pl-3">
                  <Input
                    value={editingName}
                    onChange={(e) => setEditingName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') handleSaveEdit()
                      if (e.key === 'Escape') handleCancelEdit()
                    }}
                    className="h-7 text-sm flex-1"
                    autoFocus
                  />
                  <Button variant="ghost" size="icon" className="h-6 w-6" onClick={handleSaveEdit}>
                    <Check className="h-3.5 w-3.5 text-success" />
                  </Button>
                  <Button variant="ghost" size="icon" className="h-6 w-6" onClick={handleCancelEdit}>
                    <X className="h-3.5 w-3.5" />
                  </Button>
                </div>
              ) : (
                <div
                  role="button"
                  tabIndex={0}
                  onClick={() => handleSwitch(dashboard.id)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault()
                      handleSwitch(dashboard.id)
                    }
                  }}
                  className="w-full text-left px-3 py-2 cursor-pointer rounded-lg"
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <p className={cn(
                        "text-sm truncate",
                        isActive ? "font-medium text-foreground" : "text-muted-foreground"
                      )}>
                        {dashboard.name}
                      </p>
                      <p className="text-[11px] text-muted-foreground mt-0.5">
                        {t('sidebar.componentCount', { count: componentCount })}
                      </p>
                    </div>
                    <div
                      className={cn(
                        'flex items-center gap-0.5 shrink-0',
                        isDesktop ? 'opacity-0 group-hover:opacity-100 transition-opacity' : ''
                      )}
                      onClick={(e) => e.stopPropagation()}
                    >
                      <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => handleStartEdit(dashboard)} title={t('sidebar.rename')}>
                        <Edit2 className="h-3.5 w-3.5" />
                      </Button>
                      {dashboards.length > 1 && (
                        <Button variant="ghost" size="icon" className="h-6 w-6 text-muted-foreground hover:text-error" onClick={() => handleDelete(dashboard.id)} title={t('sidebar.delete')}>
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      )}
                    </div>
                  </div>
                </div>
              )}
            </div>
          )
        })}

        {showCreateInput ? (
          <div className="rounded-lg border border-dashed border-border bg-muted-30 p-2">
            <Input
              value={newDashboardName}
              onChange={(e) => setNewDashboardName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleCreate()
                if (e.key === 'Escape') {
                  setShowCreateInput(false)
                  setNewDashboardName('')
                }
              }}
              placeholder={t('sidebar.namePlaceholder')}
              className="h-8 text-sm mb-1"
              autoFocus
            />
            <div className="flex gap-1">
              <Button size="sm" variant="ghost" className="h-7 px-2 text-xs flex-1" onClick={handleCreate}>
                <Check className="h-3.5 w-3.5 mr-1 text-success" />
                {t('sidebar.create')}
              </Button>
              <Button size="sm" variant="ghost" className="h-6 w-6 p-0" onClick={() => { setShowCreateInput(false); setNewDashboardName('') }}>
                <X className="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
        ) : (
          <Button variant="ghost" className="w-full justify-start text-muted-foreground hover:text-foreground" onClick={() => setShowCreateInput(true)}>
            <Plus className="h-4 w-4 mr-2" />
            {t('sidebar.newDashboard')}
          </Button>
        )}
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
  // Position below the fixed TopNav using --topnav-height CSS variable,
  // and z-[60] to render above TopNav (z-20) and its dropdowns.
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
